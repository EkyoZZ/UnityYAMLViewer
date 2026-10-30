//! Read-only spreadsheet data, independent of LibreOffice and PDFium.
use std::{path::Path,sync::{mpsc,Arc,atomic::{AtomicBool,Ordering}}};
use calamine::{Reader,Data};
use eframe::egui;
use crate::localization::tr;
mod freeze;
const FILE_LIMIT:u64=128*1024*1024;
const TEXT_LIMIT:usize=16*1024*1024;
const CELL_LIMIT:usize=250_000;
pub(super) fn supports(path:&Path)->bool{matches!(super::extension(path).as_str(),"xls"|"xlsx"|"ods")}
struct SheetData{values:String,formulas:String,row_start:usize,column_start:usize,missing:usize,formula_count:usize,formula_error:Option<String>,freeze:Option<crate::delimited_table::FreezeSettings>}
struct Loaded{names:Vec<String>,selected:usize,sheet:Result<SheetData,String>}
fn preflight(path:&Path)->Result<(),String>{
    let file=std::fs::File::open(path).map_err(|e|e.to_string())?;
    if file.metadata().map_err(|e|e.to_string())?.len()>FILE_LIMIT{return Err(tr("表格文件超过 128 MiB，无法预览").into());}
    if super::extension(path)!="xls"{
        let mut zip=zip::ZipArchive::new(file).map_err(|e|e.to_string())?;
        if zip.len()>10_000{return Err(tr("表格内容过大，无法预览").into());}
        let mut total=0u64;for i in 0..zip.len(){total=total.saturating_add(zip.by_index(i).map_err(|e|e.to_string())?.size());if total>256*1024*1024{return Err(tr("表格内容过大，无法预览").into());}}
    }Ok(())
}
fn append_cell(text:&mut String,value:&str,last:bool)->Result<(),String>{
    // Reuse the TSV encoder so embedded tabs/newlines/quotes remain a single cell.
    if value.len()>TEXT_LIMIT.saturating_sub(text.len()){return Err(tr("工作表数据过大，无法预览").into());}
    let encoded=crate::delimited_table::encoded(value,'\t');
    if encoded.len().saturating_add(1)>TEXT_LIMIT.saturating_sub(text.len()){return Err(tr("工作表数据过大，无法预览").into());}
    text.push_str(&encoded);text.push(if last{'\n'}else{'\t'});
    if text.len()>TEXT_LIMIT{return Err(tr("工作表数据过大，无法预览").into());}Ok(())
}
fn grid(values:calamine::Range<Data>,formulas:Result<calamine::Range<String>,String>,cancel:&AtomicBool)->Result<SheetData,String>{
    let formula_error=formulas.as_ref().err().cloned();let formulas=formulas.ok();
    let start=match (values.start(),formulas.as_ref().and_then(|f|f.start())){(Some(a),Some(b))=>Some((a.0.min(b.0),a.1.min(b.1))),(a,b)=>a.or(b)};
    let end=match (values.end(),formulas.as_ref().and_then(|f|f.end())){(Some(a),Some(b))=>Some((a.0.max(b.0),a.1.max(b.1))),(a,b)=>a.or(b)};
    let mut sheet=SheetData{values:String::new(),formulas:String::new(),row_start:1,column_start:0,missing:0,formula_count:0,formula_error,freeze:None};
    let (Some(start),Some(end))=(start,end)else{return Ok(sheet);};
    let rows=(end.0-start.0) as usize+1;let columns=(end.1-start.1) as usize+1;
    if rows>100_000||columns>1024||rows.saturating_mul(columns)>CELL_LIMIT{return Err(tr("工作表范围过大，最多预览 250000 个单元格、100000 行和 1024 列。").into());}
    sheet.row_start=start.0 as usize+1;sheet.column_start=start.1 as usize;
    for row in start.0..=end.0{
        if cancel.load(Ordering::Relaxed){return Err(tr("表格读取已取消").into());}
        for col in start.1..=end.1{
            let value=values.get_value((row,col)).unwrap_or(&Data::Empty);
            let formula=formulas.as_ref().and_then(|f|f.get_value((row,col))).filter(|s|!s.is_empty());
            let expression=formula.map(|s|if s.starts_with('='){s.clone()}else{format!("={s}")});
            let text=if expression.is_some()&&matches!(value,Data::Empty){sheet.missing+=1;format!("{} {}",tr("公式结果未缓存"),expression.as_deref().unwrap())}else{value.to_string()};
            if expression.is_some(){sheet.formula_count+=1;}
            append_cell(&mut sheet.values,&text,col==end.1)?;append_cell(&mut sheet.formulas,expression.as_deref().unwrap_or(&text),col==end.1)?;
        }
    }Ok(sheet)
}
fn load(path:&Path,index:usize,cancel:&AtomicBool)->Result<Loaded,String>{
    preflight(path)?;
    if cancel.load(Ordering::Relaxed){return Err(tr("表格读取已取消").into());}
    let mut book=calamine::open_workbook_auto(path).map_err(|e|e.to_string())?;
    let names=book.sheet_names().to_vec();if names.is_empty(){return Err(tr("没有可预览的工作表").into());}
    let selected=index.min(names.len()-1);let name=&names[selected];
    let sheet=book.worksheet_range(name).map_err(|e|e.to_string()).and_then(|values|{
        let formulas=book.worksheet_formula(name).map_err(|e|e.to_string());grid(values,formulas,cancel).map(|mut sheet|{
            sheet.freeze=freeze::read(path,name).map(|settings|crate::delimited_table::FreezeSettings{
                rows:settings.rows.saturating_sub(sheet.row_start.saturating_sub(1)),
                columns:settings.columns.saturating_sub(sheet.column_start),
            });sheet
        })
    });Ok(Loaded{names,selected,sheet})
}
#[derive(Default)]
pub(super) struct Preview{
    names:Vec<String>,selected:usize,sheet:Option<SheetData>,table:crate::delimited_table::Preview,
    pending:Option<mpsc::Receiver<Result<Loaded,String>>>,cancel:Arc<AtomicBool>,error:Option<String>,
    show_formulas:bool,reveal_sheet:bool,editor:super::office_preview::Editor,started:bool,
}
impl Drop for Preview{fn drop(&mut self){self.cancel.store(true,Ordering::Relaxed);}}
impl Preview{
    fn request(&mut self,path:&Path,ctx:&egui::Context){
        self.cancel.store(true,Ordering::Relaxed);self.cancel=Arc::new(AtomicBool::new(false));let cancel=self.cancel.clone();
        let path=path.to_owned();let selected=self.selected;let ctx=ctx.clone();let (send,receive)=mpsc::channel();self.pending=Some(receive);self.error=None;self.started=true;
        std::thread::spawn(move||{let result=load(&path,selected,&cancel);let _=send.send(result);ctx.request_repaint();});
    }
    pub(super) fn show(&mut self,ui:&mut egui::Ui,path:&Path){
        if let Some(receiver)=&self.pending{match receiver.try_recv(){
            Ok(result)=>{self.pending=None;match result{
                Ok(loaded)=>{self.names=loaded.names;self.selected=loaded.selected;self.reveal_sheet=true;self.table=Default::default();match loaded.sheet{Ok(sheet)=>{self.sheet=Some(sheet);self.error=None;},Err(e)=>{self.sheet=None;self.error=Some(e);}}},
                Err(e)=>{self.sheet=None;self.error=Some(e);}
            }},
            Err(mpsc::TryRecvError::Disconnected)=>{self.pending=None;self.error=Some(tr("表格读取任务中断").into());},_=>{}
        }}
        let mut reload=false;
        ui.horizontal_wrapped(|ui|{
            super::file_context_menu(&ui.heading(path.file_name().unwrap_or_default().to_string_lossy()),path);
            self.editor.button(ui,path);
            reload=ui.add_enabled(self.pending.is_none(),egui::Button::new(tr("重载"))).clicked();
            ui.checkbox(&mut self.show_formulas,tr("显示公式"));
        });
        self.editor.status(ui,path);
        ui.small(tr("只读数据表格，可选择复制和搜索；公式显示文件保存的结果，不在本软件中重算。"));
        ui.small(tr("不还原单元格样式、合并、图表或打印布局；日期和自定义格式可能显示底层数值。"));

        // Reserve a separate footer below the table viewport and its horizontal bar.
        // Single-sheet/unknown workbooks consume no footer space at all.
        // Blend into the surrounding preview and keep tabs flush with its bottom.
        let available=ui.available_rect_before_wrap();
        let (region,_)=ui.allocate_exact_size(available.size().max(egui::Vec2::ZERO),egui::Sense::hover());
        let mut footer_height=0.0_f32;let mut changed=false;
        if self.names.len()>1{
            let font=egui::TextStyle::Button.resolve(ui.style());
            let widths:Vec<f32>=self.names.iter().map(|name|{
                (ui.painter().layout_no_wrap(name.clone(),font.clone(),ui.visuals().text_color()).size().x+ui.spacing().button_padding.x*2.0).clamp(48.0,260.0)
            }).collect();
            let status=format!("{} {}/{}",tr("工作表"),self.selected+1,self.names.len());
            let status_width=ui.painter().layout_no_wrap(status.clone(),egui::TextStyle::Body.resolve(ui.style()),ui.visuals().text_color()).size().x;
            let tabs_width=widths.iter().sum::<f32>()+ui.spacing().item_spacing.x*(widths.len()-1) as f32;
            let overflow=tabs_width>(region.width()-status_width-ui.spacing().item_spacing.x-8.0).max(0.0);
            let bar=egui::style::ScrollStyle::solid();
            footer_height=(ui.spacing().interact_size.y.max(24.0)+if overflow{bar.bar_width+bar.bar_inner_margin+bar.bar_outer_margin}else{0.0}).min(region.height().max(0.0));
            let footer=egui::Rect::from_min_max(egui::pos2(region.left(),region.bottom()-footer_height),region.max);
            let mut tabs_ui=ui.new_child(egui::UiBuilder::new().id_salt((path,"worksheet_footer")).max_rect(footer).layout(egui::Layout::top_down(egui::Align::Min)));
            tabs_ui.set_clip_rect(footer.intersect(ui.clip_rect()));
            tabs_ui.style_mut().spacing.scroll=bar;
            tabs_ui.style_mut().spacing.scroll.foreground_color=true;
            let reveal=std::mem::take(&mut self.reveal_sheet);
            let height=ui.spacing().interact_size.y.max(24.0);
            tabs_ui.horizontal(|ui|{
                ui.add(egui::Label::new(status).wrap_mode(egui::TextWrapMode::Extend));
                egui::ScrollArea::horizontal().id_salt((path,"worksheet_tabs"))
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .auto_shrink([false,true]).max_height(footer_height.max(0.0)).show(ui,|ui|{
                    ui.add_enabled_ui(self.pending.is_none(),|ui|{ui.horizontal(|ui|{
                        for (index,name) in self.names.iter().enumerate(){
                            let active=index==self.selected;
                            let text=if active{egui::RichText::new(name).strong()}else{egui::RichText::new(name)};
                            let response=ui.add_sized([widths[index],height],egui::Button::new(text).selected(active).truncate()).on_hover_text(name);
                            if reveal&&active{response.scroll_to_me(Some(egui::Align::Center));}
                            if response.clicked()&&!active{self.selected=index;changed=true;}
                        }
                    });});
                });
            });
        }
        if !self.started||reload||changed{
            self.sheet=None;self.table=Default::default();self.reveal_sheet=true;self.request(path,ui.ctx());
        }
        // Keep the table's horizontal scrollbar clear of the bottom tabs without lifting the tabs.
        let footer_gap=if footer_height>0.0{6.0_f32.min((region.height()-footer_height).max(0.0))}else{0.0};
        let body_rect=egui::Rect::from_min_max(region.min,egui::pos2(region.right(),region.bottom()-footer_height-footer_gap));
        let mut body=ui.new_child(egui::UiBuilder::new().id_salt((path,"worksheet_body")).max_rect(body_rect).layout(egui::Layout::top_down(egui::Align::Min)));
        body.set_clip_rect(body_rect.intersect(ui.clip_rect()));
        if self.pending.is_some(){body.spinner();body.label(tr("正在读取工作表…"));body.ctx().request_repaint_after(std::time::Duration::from_millis(100));return;}
        if let Some(error)=&self.error{body.colored_label(body.visuals().error_fg_color,error);if body.button(tr("重试")).clicked(){self.request(path,body.ctx());}return;}
        if let Some(sheet)=&self.sheet{
            if sheet.missing>0{body.colored_label(body.visuals().warn_fg_color,format!("{}: {}",tr("未缓存结果的公式"),sheet.missing));}
            if sheet.formula_count>0{body.small(tr("缓存结果可能不是最新值；需要重算时请在 LibreOffice 或 Excel 保存后重载。"));}
            if let Some(error)=&sheet.formula_error{body.colored_label(body.visuals().warn_fg_color,format!("{}: {error}",tr("部分公式无法读取，当前显示可读取的缓存值")));}
            let text=if self.show_formulas{&sheet.formulas}else{&sheet.values};
            self.table.set_source_freeze(sheet.freeze);
            body.push_id((path,self.selected,self.show_formulas),|ui|self.table.show_readonly(ui,text,sheet.row_start,sheet.column_start));
        }
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn formulas_preserve_cached_zero_and_mark_missing_values_with_actual_coordinates(){
        let mut values=calamine::Range::new((3,2),(4,3));values.set_value((3,2),Data::Float(0.));values.set_value((4,2),Data::String("a\tb\n\"c\"".into()));
        let mut formulas=calamine::Range::new((3,2),(3,3));formulas.set_value((3,2),"SUM(A1:A2)".into());formulas.set_value((3,3),"A1*2".into());
        let result=grid(values,Ok(formulas),&AtomicBool::new(false)).unwrap();
        assert_eq!((result.row_start,result.column_start),(4,2));assert_eq!(result.missing,1);assert_eq!(result.formula_count,2);
        assert!(result.values.starts_with("0\t"));assert!(result.values.contains("=A1*2"));assert!(result.formulas.starts_with("=SUM(A1:A2)\t=A1*2"));
        assert!(result.values.contains("\"a\tb\n\"\"c\"\"\""));
    }
    #[test] fn formula_only_sheet_is_not_lost_and_extreme_ranges_are_rejected(){
        let mut formulas=calamine::Range::new((1,1),(1,1));formulas.set_value((1,1),"1+1".into());
        let result=grid(calamine::Range::empty(),Ok(formulas),&AtomicBool::new(false)).unwrap();assert_eq!(result.missing,1);assert_eq!((result.row_start,result.column_start),(2,1));
        let values=calamine::Range::new((0,0),(0,1024));assert!(grid(values,Ok(calamine::Range::empty()),&AtomicBool::new(false)).is_err());
    }
    #[test]
    #[ignore="requires SPREADSHEET_SAMPLE and SPREADSHEET_ORACLE, generated independently from workbook XML"]
    fn supplied_workbook_matches_independent_xml_oracle(){
        let path=std::env::var("SPREADSHEET_SAMPLE").expect("SPREADSHEET_SAMPLE");
        let oracle=std::fs::read(std::env::var("SPREADSHEET_ORACLE").expect("SPREADSHEET_ORACLE")).unwrap();
        let oracle:serde_json::Value=serde_json::from_slice(&oracle).unwrap();
        for (index,expected) in oracle["sheets"].as_array().unwrap().iter().enumerate(){
            let loaded=load(Path::new(&path),index,&AtomicBool::new(false)).unwrap();
            assert_eq!(loaded.names[index],expected["name"].as_str().unwrap());
            let sheet=loaded.sheet.unwrap();
            if let Some(expected_freeze)=expected.get("freeze"){
                let freeze=if expected_freeze.is_null(){None}else{Some(crate::delimited_table::FreezeSettings{rows:expected_freeze["rows"].as_u64().unwrap() as usize,columns:expected_freeze["columns"].as_u64().unwrap() as usize})};
                assert_eq!(sheet.freeze,freeze,"source frozen panes in worksheet {index}");
            }
            let values=crate::delimited_table::parse(&sheet.values,'\t').unwrap();
            let formulas=crate::delimited_table::parse(&sheet.formulas,'\t').unwrap();
            assert_eq!(sheet.formula_count,expected["formulas"].as_u64().unwrap() as usize);
            assert_eq!(sheet.missing,expected["missing"].as_u64().unwrap() as usize);
            for cell in expected["cells"].as_array().unwrap(){
                let row=cell["row"].as_u64().unwrap() as usize+1-sheet.row_start;
                let col=cell["column"].as_u64().unwrap() as usize-sheet.column_start;
                let actual=&values[row][col];let expected_value=cell["value"].as_str().unwrap();
                if cell["kind"]=="n"{assert_eq!(actual.parse::<f64>().unwrap(),expected_value.parse::<f64>().unwrap(),"sheet {index} row {row} col {col}");}
                else{assert_eq!(actual,expected_value,"sheet {index} row {row} col {col}");}
                if let Some(formula)=cell["formula"].as_str(){assert_eq!(formulas[row][col],format!("={}",formula.trim_start_matches('=')));}
            }
            println!("Verified worksheet {index}: {} cells, {} formulas, {} missing caches",expected["cells"].as_array().unwrap().len(),sheet.formula_count,sheet.missing);
        }
    }

}
