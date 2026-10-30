//! Delimited text preview and transactional cell editing.
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};

#[derive(Default)]
pub struct Preview {
    source: String,
    delimiter: char,
    rows: Vec<Vec<String>>,
    columns: usize,
    raw: bool,
    freeze_header:bool,
    source_freeze:Option<FreezeSettings>,
    error: Option<String>,
    ranges: Vec<Vec<std::ops::Range<usize>>>,
    check: Check,
    search: Search,
    active_cell: Option<(usize,usize)>,
}

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub(crate) struct FreezeSettings{pub rows:usize,pub columns:usize}

#[derive(Clone,Copy)]
struct CellView{readonly:bool,editing:bool,delimiter:char,target:Option<(usize,usize)>}

fn column_name(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (index % 26) as u8) as char);
        if index < 26 { break; }
        index = index / 26 - 1;
    }
    letters.into_iter().rev().collect()
}
fn cell_address(row: usize, column: usize) -> String { format!("{}{row}",column_name(column)) }
fn decoded_cell(raw: &str, delimiter: char) -> String {
    parse(raw,delimiter).ok().and_then(|rows|rows.into_iter().next()).and_then(|row|row.into_iter().next()).unwrap_or_default()
}

#[derive(Default)]
struct Search {
    query: String, cached_query: String, source: Option<String>,
    matches: Vec<(usize,usize)>, selected: Option<(usize,usize)>, jump: bool,
    last_input: Option<Instant>, rerun: bool, select_after: bool,
    pending: Option<mpsc::Receiver<Vec<(usize,usize)>>>, cancel: Option<Arc<AtomicBool>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchMode { Live, Delayed, Submitted }
fn search_mode(cells: usize, bytes: usize) -> SearchMode {
    if cells <= 10_000 && bytes <= 1_000_000 { SearchMode::Live }
    else if cells <= 50_000 && bytes <= 5_000_000 { SearchMode::Delayed }
    else { SearchMode::Submitted }
}
impl Drop for Search {
    fn drop(&mut self) { if let Some(cancel) = &self.cancel { cancel.store(true,Ordering::Relaxed); } }
}
fn matches(rows: &[Vec<String>], query: &str) -> Vec<(usize,usize)> {
    if query.is_empty() { return Vec::new(); }
    let query = query.to_lowercase();
    rows.iter().enumerate().flat_map(|(row, values)| values.iter().enumerate().filter_map({
        let query = &query;
        move |(column,value)| value.to_lowercase().contains(query).then_some((row,column))
    })).collect()
}
fn navigate(items: &[(usize,usize)], selected: Option<(usize,usize)>, forward: bool) -> Option<(usize,usize)> {
    if items.is_empty() { return None; }
    let count = items.len();
    let current = selected.and_then(|key| items.iter().position(|item| *item == key));
    let next = current.map_or(if forward {0} else {count-1}, |i| if forward {(i+1)%count} else {(i+count-1)%count});
    Some(items[next])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { Integer, Number, Boolean, JsonObject, Array, Text }
impl Kind {
    fn label(self) -> &'static str { match self { Self::Integer=>"整数", Self::Number=>"数值", Self::Boolean=>"布尔值", Self::JsonObject=>"JSON 对象", Self::Array=>"数组", Self::Text=>"文本" } }
    fn accepts(self, value: &str) -> bool {
        let value = value.trim();
        value.is_empty() || match self {
            Self::Integer => integer(value),
            Self::Number => value.parse::<f64>().is_ok_and(f64::is_finite),
            Self::Boolean => value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false"),
            Self::JsonObject => value.starts_with('{') && serde_json::from_str::<serde_json::Value>(value).is_ok_and(|value|value.is_object()),
            Self::Array => value.starts_with('[') && serde_json::from_str::<serde_json::Value>(value).is_ok_and(|value|value.is_array()),
            Self::Text => true,
        }
    }
}
fn integer(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())
}
fn inspect(rows: &[Vec<String>], columns: usize, header: bool, samples: usize, required:bool) -> (Vec<Kind>, Vec<(usize, usize)>) {
    let start = usize::from(header);
    let kinds: Vec<_> = (0..columns).map(|column| {
        let values: Vec<_> = rows.iter().skip(start).take(samples).filter_map(|row| row.get(column)).map(|value| value.trim()).filter(|value| !value.is_empty()).collect();
        if values.len() >= 2 {
            if values.iter().all(|value| Kind::Integer.accepts(value)) { return Kind::Integer; }
            if values.iter().all(|value| Kind::Number.accepts(value)) { return Kind::Number; }
            if values.iter().all(|value| Kind::Boolean.accepts(value)) { return Kind::Boolean; }
            if values.iter().all(|value| Kind::JsonObject.accepts(value)) { return Kind::JsonObject; }
            if values.iter().all(|value| Kind::Array.accepts(value)) { return Kind::Array; }
        }
        // Ambiguous samples get two more records, never require a small table
        // to have five rows. A strong majority can expose a bad sample itself.
        let extended: Vec<_> = rows.iter().skip(start).take(samples+2).filter_map(|row| row.get(column)).map(|value| value.trim()).filter(|value| !value.is_empty()).collect();
        for kind in [Kind::Integer, Kind::Number, Kind::Boolean, Kind::JsonObject, Kind::Array] {
            let matches = extended.iter().filter(|value| kind.accepts(value)).count();
            if matches >= 2 && matches*5 >= extended.len()*4 { return kind; }
        }
        Kind::Text
    }).collect();
    let mut errors = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(start) {
        for (column, kind) in kinds.iter().enumerate() {
            let value=row.get(column).map(String::as_str).unwrap_or("");
            if (required&&value.trim().is_empty())||!kind.accepts(value){errors.push((index,column));}
        }
    }
    (kinds, errors)
}
struct Check {
    enabled: bool, header: bool, required:bool,
    source: Option<String>, kinds: Vec<Kind>, errors: Vec<(usize,usize)>,
    selected: Option<(usize,usize)>, jump: bool,
}
impl Default for Check {
    fn default() -> Self { Self { enabled:true, header:true, required:false, source:None, kinds:Vec::new(), errors:Vec::new(), selected:None, jump:false } }
}

fn cell_ranges(text: &str, delimiter: char) -> Vec<Vec<std::ops::Range<usize>>> {
    let mut rows = Vec::new(); let mut row = Vec::new();
    let mut start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let bom = start;
    let mut chars = text.char_indices().filter(|(i, _)| *i >= bom).peekable();
    let mut quoted = false;
    while let Some((i, c)) = chars.next() {
        if c == '"' {
            if quoted && chars.peek().is_some_and(|(_, next)| *next == '"') { chars.next(); }
            else if quoted { quoted = false; }
            else if i == start { quoted = true; }
        } else if !quoted && (c == delimiter || c == '\n' || c == '\r') {
            row.push(start..i); start = i + c.len_utf8();
            if c != delimiter {
                if c == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') { chars.next(); start += 1; }
                rows.push(std::mem::take(&mut row));
            }
        }
    }
    if start < text.len() || !row.is_empty() { row.push(start..text.len()); rows.push(row); }
    rows
}

pub(crate) fn encoded(value: &str, delimiter: char) -> String {
    if value.contains([delimiter, '"', '\r', '\n']) { format!("\"{}\"", value.replace('"', "\"\"")) }
    else { value.to_owned() }
}

pub(crate) fn parse(text: &str, delimiter: char) -> Result<Vec<Vec<String>>, &'static str> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut pending = false;
    while let Some(c) = chars.next() {
        pending = true;
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') { chars.next(); field.push('"'); }
                else { quoted = false; closed = true; }
            } else { field.push(c); }
        } else if c == delimiter {
            row.push(std::mem::take(&mut field)); closed = false;
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') { chars.next(); }
            row.push(std::mem::take(&mut field)); rows.push(std::mem::take(&mut row));
            closed = false; pending = false;
        } else if c == '"' && field.is_empty() && !closed { quoted = true; }
        else if closed { return Err("表格引号格式无效，请查看原文"); }
        else { field.push(c); }
    }
    if quoted { return Err("表格引号格式无效，请查看原文"); }
    if pending { row.push(field); rows.push(row); }
    Ok(rows)
}

impl Preview {
    pub(crate) fn set_source_freeze(&mut self,freeze:Option<FreezeSettings>){if self.source_freeze!=freeze{self.source_freeze=freeze;if freeze.is_some(){self.freeze_header=true;}}}
    pub fn show(&mut self, ui: &mut egui::Ui, text: &str, delimiter: char) {self.show_impl(ui,text,delimiter,false,0,0);}
    pub(crate) fn show_readonly(&mut self,ui:&mut egui::Ui,text:&str,row_offset:usize,column_offset:usize){self.show_impl(ui,text,'\t',true,row_offset,column_offset);}
    fn show_impl(&mut self, ui: &mut egui::Ui, text: &str, delimiter: char,readonly:bool,row_offset:usize,column_offset:usize) {
        use crate::localization::tr;
        // Reserve physical gutters instead of painting floating bars over cells.
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        // The theme uses the same fill for idle widgets and the panel. Use
        // foreground colors so idle handles remain visible against the track.
        ui.style_mut().spacing.scroll.foreground_color = true;
        ui.style_mut().spacing.scroll.bar_width = 10.0;
        ui.style_mut().spacing.scroll.handle_min_length = 24.0;
        if self.source != text || self.delimiter != delimiter {
            self.active_cell = None;
            self.check.source = None; self.check.selected = None;
            self.search.source = None; self.search.selected = None;
            if let Some(cancel) = &self.search.cancel { cancel.store(true,Ordering::Relaxed); }
            self.source = text.to_owned(); self.delimiter = delimiter;
            match parse(text, delimiter) {
                Ok(rows) => { self.ranges = cell_ranges(text, delimiter); self.columns = rows.iter().map(Vec::len).max().unwrap_or(0); self.rows = rows; self.error = None; }
                Err(error) => { self.rows.clear(); self.columns = 0; self.error = Some(tr(error).to_owned()); }
            }
        }
        let mut find = false;
        let mut input_changed = false;
        let mut recheck = false;
        let focus_find = ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.raw, false, tr("表格"));
            ui.selectable_value(&mut self.raw, true, tr(if readonly{"数据文本"}else{"原文"}));
            ui.small(format!("{} × {}", self.rows.len(), self.columns));
        });
        ui.horizontal(|ui| {
            recheck |= ui.checkbox(&mut self.check.header, tr("首行为表头")).changed();
            ui.add_enabled(!self.raw,egui::Checkbox::new(&mut self.freeze_header,tr("冻结表头")))
                .on_hover_text(tr("优先使用原文件冻结行列；无可用设置时固定第一行。行号、列字母始终固定，校验规则不变。"));
            recheck |= ui.checkbox(&mut self.check.enabled, tr("类型检查")).changed();
            if self.check.enabled{
                recheck|=ui.checkbox(&mut self.check.required,tr("必填"))
                    .on_hover_text(tr("检查所有非表头单元格；空白字符和短行缺少的列也视为空值。")).changed();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                find = ui.button(tr("查找")).clicked();
                let response = ui.add(egui::TextEdit::singleline(&mut self.search.query).id_source("table_find").desired_width(160.0));
                if focus_find { response.request_focus(); }
                input_changed = response.changed();
                if response.has_focus() && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE,egui::Key::Enter)) { find = true; }
            });
        });
        let rendered = if readonly{text.to_owned()}else{crate::editing::scope_value(ui).unwrap_or_else(|| text.to_owned())};
        let mode = search_mode(self.rows.len().saturating_mul(self.columns),rendered.len());
        if input_changed { self.search.last_input = Some(Instant::now()); }
        let auto = match mode {
            SearchMode::Live => input_changed,
            SearchMode::Delayed => self.search.last_input.is_some_and(|time|time.elapsed() >= Duration::from_millis(250)),
            SearchMode::Submitted => false,
        };
        if mode == SearchMode::Delayed && self.search.last_input.is_some() { ui.ctx().request_repaint_after(Duration::from_millis(50)); }
        let new_query = (find || auto) && self.search.cached_query != self.search.query;
        if find || auto { self.search.last_input = None; }
        if new_query {
            self.search.cached_query = self.search.query.clone();
            self.search.source = None;
            self.search.selected = None;
        }
        if self.search.source.as_deref() != Some(&rendered) {
            self.search.source = Some(rendered.clone());
            if let Some(cancel) = &self.search.cancel { cancel.store(true,Ordering::Relaxed); }
            self.search.matches.clear(); self.search.selected = None;
            self.search.rerun = !self.search.cached_query.is_empty();
            self.search.select_after = find;
        }
        if let Some(pending) = &self.search.pending {
            if find { self.search.select_after = true; }
            match pending.try_recv() {
                Ok(found) => {
                    let cancelled = self.search.cancel.as_ref().is_some_and(|cancel|cancel.load(Ordering::Relaxed));
                    self.search.pending = None; self.search.cancel = None;
                    if !cancelled {
                        self.search.matches = found;
                        if self.search.select_after { self.search.selected = self.search.matches.first().copied(); self.search.jump = self.search.selected.is_some(); }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.search.pending = None; self.search.cancel = None; }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.search.rerun && self.search.pending.is_none() {
            self.search.rerun = false;
            if mode == SearchMode::Submitted {
                let source = rendered.clone(); let query = self.search.cached_query.to_lowercase();
                let cancel = Arc::new(AtomicBool::new(false)); let worker_cancel = cancel.clone();
                let (tx,rx) = mpsc::channel(); let ctx = ui.ctx().clone();
                self.search.pending = Some(rx); self.search.cancel = Some(cancel);
                std::thread::spawn(move || {
                    let mut found = Vec::new();
                    if let Ok(rows) = parse(&source,delimiter) {
                        for (row,values) in rows.iter().enumerate() {
                            if worker_cancel.load(Ordering::Relaxed) { break; }
                            for (column,value) in values.iter().enumerate() {
                                if value.to_lowercase().contains(&query) { found.push((row,column)); }
                            }
                        }
                    }
                    let _ = tx.send(found); ctx.request_repaint();
                });
            } else {
                self.search.matches = parse(&rendered,delimiter).map(|rows|matches(&rows,&self.search.cached_query)).unwrap_or_default();
            }
        }
        if self.check.enabled && self.error.is_none() {
            if recheck || self.check.source.as_deref() != Some(&rendered) {
                if let Ok(rows) = parse(&rendered, delimiter) {
                    (self.check.kinds, self.check.errors) = inspect(&rows, self.columns, self.check.header, 3, self.check.required);
                    if self.check.selected.is_some_and(|key| !self.check.errors.contains(&key)) { self.check.selected = None; }
                }
                self.check.source = Some(rendered);
            }
        }
        ui.horizontal(|ui| {
            if self.check.enabled && self.error.is_none() {
                ui.label(format!("{}: {}", tr(if self.check.required{"校验错误"}else{"类型错误"}), self.check.errors.len()));
                for (label, forward) in [("上一个",false),("下一个",true)] {
                    if ui.add_enabled(!self.check.errors.is_empty(),egui::Button::new(tr(label))).clicked() {
                        self.check.selected = navigate(&self.check.errors,self.check.selected,forward); self.check.jump = true; self.raw = false;
                        self.search.selected = None;
                    }
                }
                if let Some((row,column)) = self.check.selected { ui.label(cell_address(row+row_offset,column+column_offset)); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (label,forward) in [("下一个",true),("上一个",false)] {
                if ui.add_enabled(!self.search.matches.is_empty(),egui::Button::new(tr(label))).clicked() {
                    self.search.selected = navigate(&self.search.matches,self.search.selected,forward); self.search.jump = true;
                }
            }
            ui.label(format!("{}: {}",tr("匹配"),self.search.matches.len()));
            if self.search.pending.is_some() || self.search.rerun { ui.spinner(); }
            if let Some(key) = self.search.selected {
                if let Some(index) = self.search.matches.iter().position(|item| *item == key) { ui.small(format!("{} / {}",index+1,self.search.matches.len())); }
            }
        });
        });
        if find && self.search.pending.is_none() && !self.search.jump { self.search.selected = navigate(&self.search.matches,self.search.selected,true); self.search.jump = self.search.selected.is_some(); }
        if self.search.jump { self.check.selected = None; self.check.jump = false; self.raw = false; }
        if let Some(error) = &self.error { ui.colored_label(ui.visuals().warn_fg_color, error); }
        if self.raw || self.error.is_some() {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            egui::ScrollArea::both().id_salt("delimited_raw").auto_shrink([false, false]).show(ui, |ui| {
                let rendered = if readonly{text.to_owned()}else{crate::editing::scope_value(ui).unwrap_or_else(|| text.to_owned())};
                let mut view = rendered.as_str();
                ui.add(egui::TextEdit::multiline(&mut view).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
            });
            return;
        }
        if self.columns == 0 { return; }
        let editing = !readonly&&crate::editing::active(ui);
        if editing {
            if self.active_cell.is_none() && self.ranges.first().is_some_and(|row| !row.is_empty()) { self.active_cell = Some((0,0)); }
            ui.horizontal(|ui| {
                if let Some((row,column)) = self.active_cell {
                    let height = ui.spacing().interact_size.y;
                    let address = cell_address(row+row_offset,column+column_offset);
                    let font = egui::TextStyle::Monospace.resolve(ui.style());
                    let color = ui.visuals().weak_text_color();
                    let width = ui.fonts(|fonts|fonts.layout_no_wrap(address.clone(),font.clone(),color).size().x)+8.0;
                    let (rect,_) = ui.allocate_exact_size(egui::vec2(width,height),egui::Sense::hover());
                    ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,address,font,color);
                    if let Some(bind) = self.ranges.get(row).and_then(|row|row.get(column)).cloned().and_then(|range|crate::editing::cell_binding(ui,range)) {
                        let mut value = decoded_cell(&bind.value(),delimiter);
                        let fill = if bind.dirty() { egui::Color32::from_rgba_unmultiplied(240,170,30,55) } else { ui.visuals().extreme_bg_color };
                        let response = ui.add_sized(egui::vec2(ui.available_width(),height),egui::TextEdit::singleline(&mut value).id_source(("cell_value_bar",row,column)).background_color(fill));
                        if response.changed() { bind.replace_text(encoded(&value,delimiter)); ui.ctx().request_repaint(); }
                    }
                }
            });
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        // One viewport owns both bars, so vertical scrolling cannot move the
        // bottom bar away, and wide columns cannot move the right bar offscreen.
        // Each solid bar remains visible only while its own axis overflows.
        let target = if self.search.jump { self.search.selected } else if self.check.jump { self.check.selected } else { None };
        self.check.jump = false;
        self.search.jump = false;
        let requested=if self.freeze_header{self.source_freeze.unwrap_or(FreezeSettings{rows:1,columns:0})}else{FreezeSettings::default()};
        let frozen_rows=requested.rows.min(self.rows.len());
        let frozen_columns=requested.columns.min(self.columns);
        let view=CellView{readonly,editing,delimiter,target};
        let mut scroll=egui::ScrollArea::both().id_salt("delimited_viewport")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).auto_shrink([false,false]);
        if let Some((row,_))=target{
            if row>=frozen_rows{scroll=scroll.vertical_scroll_offset(row.saturating_sub(frozen_rows) as f32*(24.0+ui.spacing().item_spacing.y));}
        }
        scroll.show_viewport(ui,|ui,viewport|{
            let viewport_rect=egui::Rect::from_min_size(ui.max_rect().min+viewport.min.to_vec2(),viewport.size());
            // TableBuilder also paints row stripes and resize guides outside cell UIs.
            // Clip the entire table before painting, not just its individual cells.
            ui.set_clip_rect(ui.clip_rect().intersect(viewport_rect));
            let content_clip=ui.clip_rect();let spacing=ui.spacing().item_spacing;
            let header_height=24.0+frozen_rows as f32*(24.0+spacing.y);
            let mut columns=Vec::with_capacity(self.columns+1);let mut number_rows=Vec::new();
            TableBuilder::new(ui).striped(true).resizable(true).vscroll(false)
                .column(Column::initial(48.0).at_least(40.0)).columns(Column::initial(160.0).at_least(40.0),self.columns)
                .header(header_height,|mut header|{
                    // Only the fixed overlay paints labels; a second scrolling copy
                    // would show through around the edge of the frozen region.
                    header.col(|ui|{columns.push(ui.max_rect());});
                    for _ in 0..self.columns{header.col(|ui|{columns.push(ui.max_rect());});}
                }).body(|body|{
                    let number_width=columns.first().map_or(48.0,|r|r.width());
                    let fixed_width=number_width+spacing.x*0.5+columns.iter().skip(1).take(frozen_columns).map(|r|r.width()+spacing.x).sum::<f32>();
                    let body_clip=egui::Rect::from_min_max(egui::pos2((viewport_rect.left()+fixed_width).min(viewport_rect.right()),(viewport_rect.top()+header_height).min(viewport_rect.bottom())),viewport_rect.max).intersect(content_clip);
                    body.rows(24.0,self.rows.len()-frozen_rows,|mut row|{
                        let index=row.index()+frozen_rows;
                        row.col(|ui|{number_rows.push((index,ui.max_rect()));});
                        for column in 0..self.columns{row.col(|ui|{
                            ui.set_clip_rect(ui.clip_rect().intersect(body_clip));
                            if column>=frozen_columns&&ui.max_rect().intersects(ui.clip_rect()){self.show_cell(ui,index,column,view,false);}
                        });}
                    });
                });
            let number_width=columns.first().map_or(48.0,|r|r.width());
            let number_right=(viewport_rect.left()+number_width+spacing.x*0.5).min(viewport_rect.right());
            let fixed_right=(number_right+columns.iter().skip(1).take(frozen_columns).map(|r|r.width()+spacing.x).sum::<f32>()).min(viewport_rect.right());
            let fixed_body=egui::Rect::from_min_max(egui::pos2(viewport_rect.left(),(viewport_rect.top()+header_height).min(viewport_rect.bottom())),egui::pos2(fixed_right,viewport_rect.bottom())).intersect(content_clip);
            let background=ui.visuals().window_fill;
            ui.painter().with_clip_rect(fixed_body).rect_filled(fixed_body,0.0,background);
            for (index,source_rect) in number_rows{
                let number_rect=egui::Rect::from_min_max(egui::pos2(viewport_rect.left(),source_rect.top()),egui::pos2(number_right,source_rect.bottom()));
                if (index-frozen_rows)%2==0{
                    let stripe=egui::Rect::from_min_max(egui::pos2(viewport_rect.left(),source_rect.top()-spacing.y*0.5),egui::pos2(fixed_right,source_rect.bottom()+spacing.y*0.5));
                    ui.painter().with_clip_rect(fixed_body).rect_filled(stripe,0.0,ui.visuals().faint_bg_color);
                }
                let mut row_ui=ui.new_child(egui::UiBuilder::new().id_salt(("fixed_row_number",index)).max_rect(number_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                row_ui.set_clip_rect(number_rect.intersect(fixed_body));row_ui.label((index+row_offset).to_string());
                for column in 0..frozen_columns{
                    let source=columns[column+1];
                    let rect=egui::Rect::from_min_max(egui::pos2(source.left()+viewport.min.x,source_rect.top()),egui::pos2(source.right()+viewport.min.x,source_rect.bottom()));
                    if !rect.intersects(fixed_body){continue;}
                    let mut cell_ui=ui.new_child(egui::UiBuilder::new().id_salt(("fixed_body_cell",index,column)).max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    cell_ui.set_clip_rect(rect.intersect(fixed_body));self.show_cell(&mut cell_ui,index,column,view,true);
                }
            }
            // The coordinate axes are always fixed; source data rows/columns are optional.
            let header_rect=egui::Rect::from_min_size(viewport_rect.min,egui::vec2(viewport_rect.width(),header_height));
            let header_clip=header_rect.intersect(viewport_rect).intersect(content_clip);
            ui.painter().with_clip_rect(header_clip).rect_filled(header_rect,0.0,background);
            for (index,source) in columns.iter().enumerate(){
                let fixed=index<=frozen_columns;
                let left=if index==0{viewport_rect.left()}else{source.left()+if fixed{viewport.min.x}else{0.0}};
                let right=if index==0{number_right}else{source.right()+if fixed{viewport.min.x}else{0.0}};
                let column_clip=egui::Rect::from_min_max(egui::pos2(if index==0{viewport_rect.left()}else if fixed{number_right}else{fixed_right},viewport_rect.top()),viewport_rect.max).intersect(header_clip);
                let letters_rect=egui::Rect::from_min_max(egui::pos2(left,header_rect.top()),egui::pos2(right,header_rect.top()+24.0));
                let mut label_ui=ui.new_child(egui::UiBuilder::new().id_salt(("fixed_column_letter",index)).max_rect(letters_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                label_ui.set_clip_rect(letters_rect.intersect(column_clip));
                label_ui.add(egui::Label::new(egui::RichText::new(if index==0{"#".to_owned()}else{self.column_label(index-1,column_offset)}).strong()).truncate());
                for row in 0..frozen_rows{
                    let top=header_rect.top()+24.0+spacing.y+row as f32*(24.0+spacing.y);
                    if top>=viewport_rect.bottom(){break;}
                    let rect=egui::Rect::from_min_max(egui::pos2(left,top),egui::pos2(right,top+24.0));
                    let mut cell_ui=ui.new_child(egui::UiBuilder::new().id_salt(("fixed_header_cell",row,index)).max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    cell_ui.set_clip_rect(rect.intersect(column_clip));
                    if index==0{cell_ui.label((row+row_offset).to_string());}else{self.show_cell(&mut cell_ui,row,index-1,view,true);}
                }
            }
            ui.painter().with_clip_rect(header_clip).hline(header_rect.x_range(),header_rect.bottom()-0.5,ui.visuals().widgets.noninteractive.bg_stroke);
            // The opaque overlays cover TableBuilder's original column guides.
            // Redraw them at the same half-spacing boundaries as the body.
            let stroke=ui.visuals().widgets.noninteractive.bg_stroke;
            let header_painter=ui.painter().with_clip_rect(header_clip);
            for (index,source) in columns.iter().enumerate(){
                let fixed=index<=frozen_columns;
                let x=if index==0{number_right}else{source.right()+spacing.x*0.5+if fixed{viewport.min.x}else{0.0}};
                if index==0||fixed||x>fixed_right{
                    header_painter.vline(x,header_clip.y_range(),stroke);
                }
            }
            let fixed_painter=ui.painter().with_clip_rect(fixed_body);
            fixed_painter.vline(number_right,fixed_body.y_range(),stroke);
            for source in columns.iter().skip(1).take(frozen_columns){
                fixed_painter.vline(source.right()+viewport.min.x+spacing.x*0.5,fixed_body.y_range(),stroke);
            }
        });
    }

    fn column_label(&self,column:usize,offset:usize)->String{
        let mut label=column_name(column+offset);
        if self.check.enabled{if let Some(kind)=self.check.kinds.get(column){label.push_str(&format!(" · {}",crate::localization::tr(kind.label())));}}
        label
    }
    fn show_cell(&mut self,ui:&mut egui::Ui,index:usize,column:usize,view:CellView,frozen:bool){
        use crate::localization::tr;
        let cell_rect=ui.max_rect();
        let invalid = self.check.enabled && self.check.errors.binary_search(&(index,column)).is_ok();
        let matched = self.search.matches.binary_search(&(index,column)).is_ok();
        let selected = self.check.enabled && self.check.selected == Some((index,column)) || self.search.selected == Some((index,column)) || view.editing && self.active_cell == Some((index,column));
        let background = ui.painter().add(egui::Shape::Noop);
        let value = self.rows[index].get(column).map(String::as_str).unwrap_or("");
        if let Some(bind) = (!view.readonly).then(||self.ranges.get(index).and_then(|row|row.get(column)).cloned().and_then(|range|crate::editing::cell_binding(ui,range))).flatten() {
            let raw = bind.value();
            let mut draft = decoded_cell(&raw, view.delimiter);
            let fill = if invalid { egui::Color32::from_rgba_unmultiplied(230,45,55,90) } else if matched { egui::Color32::from_rgba_unmultiplied(90,205,130,65) } else if bind.dirty() { egui::Color32::from_rgba_unmultiplied(240,170,30,55) } else { ui.visuals().extreme_bg_color };
            let response = ui.add(egui::TextEdit::singleline(&mut draft).id_source((index,column)).desired_width(ui.available_width()).background_color(fill));
            if (response.clicked() || response.has_focus()) && self.active_cell != Some((index,column)) { self.active_cell = Some((index,column)); ui.ctx().request_repaint(); }
            if response.changed() { bind.replace_text(encoded(&draft, view.delimiter)); ui.ctx().request_repaint(); }
            if bind.dirty() && matched { ui.painter().line_segment([response.rect.left_bottom(),response.rect.right_bottom()],egui::Stroke::new(2.0_f32,egui::Color32::from_rgb(240,170,30))); }
            response.on_hover_text(draft);
        } else {
            // Truncated labels already provide a full-text tooltip.
            let response=ui.add(egui::Label::new(value).truncate().selectable(true));
            if view.readonly{response.context_menu(|ui|{
                if ui.button(tr("复制单元格内容")).clicked(){ui.ctx().copy_text(value.to_owned());ui.close_menu();}
            });}
            if invalid { ui.painter().set(background,egui::Shape::rect_filled(cell_rect,0.0,egui::Color32::from_rgba_unmultiplied(230,45,55,90))); }
            else if matched { ui.painter().set(background,egui::Shape::rect_filled(cell_rect,0.0,egui::Color32::from_rgba_unmultiplied(90,205,130,65))); }
        }
        if matched && invalid { ui.painter().rect_stroke(cell_rect,0.0,egui::Stroke::new(1.0_f32,egui::Color32::from_rgb(70,180,110)),egui::StrokeKind::Inside); }
        if selected { ui.painter().rect_stroke(cell_rect,0.0,egui::Stroke::new(1.0_f32,ui.visuals().selection.stroke.color),egui::StrokeKind::Inside); }
        if view.target == Some((index,column)) { ui.scroll_to_rect(cell_rect,if frozen{None}else{Some(egui::Align::Center)}); }
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;
    use super::{parse, cell_ranges, encoded, inspect, Kind, matches, navigate, search_mode, SearchMode, column_name, cell_address};
    #[test]
    fn source_freeze_is_applied_once_without_overriding_manual_disable(){
        let mut preview=super::Preview::default();assert!(!preview.freeze_header);
        let freeze=Some(super::FreezeSettings{rows:2,columns:1});preview.set_source_freeze(freeze);assert!(preview.freeze_header);
        preview.freeze_header=false;preview.set_source_freeze(freeze);assert!(!preview.freeze_header);
        preview.set_source_freeze(None);assert!(!preview.freeze_header);
    }
    #[test]
    fn frozen_coordinates_and_data_header_keep_alignment_after_two_axis_scroll(){
        fn texts(shape:&egui::Shape,clip:egui::Rect,result:&mut Vec<(String,egui::Pos2,egui::Rect)>){
            match shape{egui::Shape::Text(text)=>result.push((text.galley.job.text.clone(),text.pos,clip)),egui::Shape::Vec(shapes)=>for shape in shapes{texts(shape,clip,result);},_=>{}}
        }
        let ctx=egui::Context::default();let mut preview=super::Preview::default();preview.check.enabled=false;
        preview.set_source_freeze(Some(super::FreezeSettings{rows:2,columns:1}));
        let mut source="FIRST_FIXED,second,THIRD_FIXED_HEADER,c,d,e\nSECOND_FIXED,b,c,d,e,f\n".to_owned();
        for row in 2..100{source.push_str(&format!("row{row},b,c,d,e,f\n"));}
        let mut before=None;let mut after=None;let mut fixed_before=None;let mut fixed_after=None;
        for frame in 0..5{
            let mut found=Vec::new();
            let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(700.0,400.0))),..Default::default()};
            let output=ctx.run(input,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
                let id=ui.make_persistent_id(egui::Id::new("delimited_viewport"));
                if frame>=2{if let Some(mut state)=egui::scroll_area::State::load(ctx,id){state.offset=egui::vec2(80.0,180.0);state.store(ctx,id);}}
                preview.show(ui,&source,',');
            });});
            for shape in &output.shapes{texts(&shape.shape,shape.clip_rect,&mut found);}
            let header=found.iter().find(|(s,p,c)|s=="THIRD_FIXED_HEADER"&&c.contains(*p)).map(|(_,p,_)|*p);
            let first=found.iter().find(|(s,p,c)|s=="FIRST_FIXED"&&c.contains(*p)).map(|(_,p,_)|*p);
            if frame==1{before=header;fixed_before=first;}
            if frame==4{after=header;fixed_after=first;}
        }
        let (before,after,fixed_before,fixed_after)=(before.unwrap(),after.unwrap(),fixed_before.unwrap(),fixed_after.unwrap());
        assert!((before.y-after.y).abs()<1.0,"frozen rows must not move vertically");
        assert!((fixed_before.x-fixed_after.x).abs()<1.0,"source frozen columns must not move horizontally");
        assert!((before.x-after.x-80.0).abs()<2.0,"unfrozen columns must track horizontal scrolling");
        // Coordinate axes stay fixed even when the optional data-header freeze is off.
        preview.freeze_header=false;
        let mut axes_before=None;let mut axes_after=None;
        for frame in 0..4{
            let mut found=Vec::new();
            let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(700.0,400.0))),..Default::default()};
            let output=ctx.run(input,|ctx|{egui::CentralPanel::default().show(ctx,|ui|{
                let id=ui.make_persistent_id(egui::Id::new("delimited_viewport"));
                if let Some(mut state)=egui::scroll_area::State::load(ctx,id){state.offset=if frame<2{egui::Vec2::ZERO}else{egui::vec2(80.0,80.0)};state.store(ctx,id);}
                preview.show(ui,&source,',');
            });});
            for shape in &output.shapes{texts(&shape.shape,shape.clip_rect,&mut found);}
            let points=["#","C","4"].map(|name|found.iter().find(|(s,p,c)|s==name&&c.contains(*p)).map(|(_,p,_)|*p).unwrap());
            if frame==1{axes_before=Some(points);}if frame==3{axes_after=Some(points);}
        }
        let (before,after)=(axes_before.unwrap(),axes_after.unwrap());
        assert!(before[0].distance(after[0])<1.0,"coordinate corner stays fixed");
        assert!((before[1].y-after[1].y).abs()<1.0,"column letters stay fixed vertically");
        assert!((before[1].x-after[1].x-80.0).abs()<2.0,"column letters track data columns");
        assert!((before[2].x-after[2].x).abs()<1.0,"row numbers stay fixed horizontally");
        assert!((before[2].y-after[2].y-80.0).abs()<2.0,"row numbers track data rows");

    }
    #[test]
    fn empty_required_cells_receive_visible_full_cell_background(){
        fn marker(shape:&egui::Shape)->bool{
            match shape{
                egui::Shape::Rect(rect)=>rect.fill==egui::Color32::from_rgba_unmultiplied(230,45,55,90)&&rect.rect.width()>=100.0&&rect.rect.height()>=20.0,
                egui::Shape::Vec(shapes)=>shapes.iter().any(marker),_=>false
            }
        }
        let ctx=egui::Context::default();let mut preview=super::Preview::default();preview.check.required=true;
        let mut visible=false;
        for _ in 0..2{
            let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(800.0,600.0))),..Default::default()};
            let output=ctx.run(input,|ctx|{egui::CentralPanel::default().show(ctx,|ui|preview.show(ui,"id,name\n1,\n2,word",','));});
            visible|=output.shapes.iter().any(|shape|marker(&shape.shape));
        }
        assert_eq!(preview.check.errors,vec![(1,1)]);assert!(visible,"empty cells must have a visible full-width error background");
    }
    #[test]
    fn required_checks_blank_whitespace_and_short_rows_without_treating_literals_as_empty(){
        let rows=parse("id,label\n1,A\n2, \n3\n0,NULL\n4,false\n5,0",',').unwrap();
        assert!(inspect(&rows,2,true,3,false).1.is_empty());
        assert_eq!(inspect(&rows,2,true,3,true).1,vec![(2,1),(3,1)]);
        let multiline=parse("id,label\n1,\" \t\n\"\n2,x",',').unwrap();
        assert_eq!(inspect(&multiline,2,true,3,true).1,vec![(1,1)]);
    }
    #[test]
    fn required_and_type_errors_combine_in_row_order_and_header_option_is_respected(){
        let rows=parse("id,enabled\n1,true\n2,false\n3,true\nbad,\n,maybe\n5,false",',').unwrap();
        assert_eq!(inspect(&rows,2,true,3,false).1,vec![(4,0),(5,1)]);
        assert_eq!(inspect(&rows,2,true,3,true).1,vec![(4,0),(4,1),(5,0),(5,1)]);
        let rows=parse(",\n1,true",',').unwrap();
        assert!(inspect(&rows,2,true,3,true).1.is_empty());
        assert_eq!(inspect(&rows,2,false,3,true).1,vec![(0,0),(0,1)]);
    }
    #[test]
    fn json_object_and_array_columns_validate_syntax_and_outer_type() {
        let rows = vec![
            vec!["object".into(),"array".into()],
            vec!["{\"item\":1}".into(),"[16,0,0,0]".into()],
            vec!["{}".into(),"[]".into()],
            vec!["{\"nested\":{\"v\":true}}".into(),"[\"a\",{\"x\":1}]".into()],
            vec!["{bad}".into(),"[1,]".into()],
            vec!["[]".into(),"{}".into()],
            vec!["".into(),"".into()],
        ];
        let (kinds,errors) = inspect(&rows,2,true,3, false);
        assert!(kinds == vec![Kind::JsonObject,Kind::Array]);
        assert_eq!(errors,vec![(4,0),(4,1),(5,0),(5,1)]);
    }
    #[test]
    fn spreadsheet_columns_and_zero_based_row_addresses() {
        for (index,name) in [(0,"A"),(4,"E"),(25,"Z"),(26,"AA"),(51,"AZ"),(52,"BA"),(701,"ZZ"),(702,"AAA")] {
            assert_eq!(column_name(index),name);
        }
        assert_eq!(cell_address(0,0),"A0");
        assert_eq!(cell_address(15,4),"E15");
    }
    #[test]
    fn search_modes_respect_both_cell_and_byte_boundaries() {
        assert!(search_mode(10_000,1_000_000) == SearchMode::Live);
        assert!(search_mode(10_001,1) == SearchMode::Delayed);
        assert!(search_mode(1,1_000_001) == SearchMode::Delayed);
        assert!(search_mode(50_000,5_000_000) == SearchMode::Delayed);
        assert!(search_mode(50_001,1) == SearchMode::Submitted);
        assert!(search_mode(1,5_000_001) == SearchMode::Submitted);
    }
    #[test]
    fn search_counts_cells_and_navigation_wraps_in_both_directions() {
        let rows = parse("Name|备注\nTree tree|树木\nTREE|other", '|').unwrap();
        let found = matches(&rows,"tree");
        assert_eq!(found,vec![(1,0),(2,0)]);
        assert_eq!(navigate(&found,None,true),Some((1,0)));
        assert_eq!(navigate(&found,None,false),Some((2,0)));
        assert_eq!(navigate(&found,Some((2,0)),true),Some((1,0)));
        assert_eq!(navigate(&found,Some((1,0)),false),Some((2,0)));
        assert_eq!(matches(&rows,"树"),vec![(1,1)]);
        assert!(matches(&rows,"").is_empty());
        assert_eq!(navigate(&[],None,true),None);
    }
    #[test]
    fn three_rows_establish_type_before_later_errors_and_five_resolve_ambiguity() {
        let rows = parse("id\n1\n2\n3\nbad\nwrong", ',').unwrap();
        assert_eq!(inspect(&rows,1,true,3, false).1,vec![(4,0),(5,0)]);
        let rows = parse("id\n1\nbad\n2\n3\n4", ',').unwrap();
        assert_eq!(inspect(&rows,1,true,3, false).1,vec![(2,0)]);
        let short = parse("id\n1\n2", ',').unwrap();
        assert!(inspect(&short,1,true,3, false).0 == vec![Kind::Integer]);
        let sparse = parse("id\n\n1\n\n2", ',').unwrap();
        assert!(inspect(&sparse,1,true,3, false).0 == vec![Kind::Integer]);
    }
    #[test]
    fn inferred_types_skip_header_widen_numbers_and_find_later_errors() {
        let rows = parse("id,value,enabled,label\n1,1.5,true,a\n2,2,FALSE,3\noops,NaN,yes,b\n4,,true,c", ',').unwrap();
        let (kinds, errors) = inspect(&rows, 4, true, 2, false);
        assert!(kinds == vec![Kind::Integer,Kind::Number,Kind::Boolean,Kind::Text]);
        assert_eq!(errors,vec![(3,0),(3,1),(3,2)]);
        let (kinds, _) = inspect(&rows, 4, false, 2, false);
        assert!(kinds == vec![Kind::Text;4]);
        let mut fixed = rows;
        fixed[3] = vec!["3".into(),"2e3".into(),"false".into(),"b".into()];
        assert!(inspect(&fixed,4,true,2, false).1.is_empty());
    }
    #[test]
    fn ambiguous_or_sparse_samples_do_not_invent_errors() {
        let rows = parse("a|b|c\n1||true\nword||\n2|bad|no", '|').unwrap();
        let (kinds,errors) = inspect(&rows,3,true,2, false);
        assert!(kinds == vec![Kind::Text;3]);
        assert!(errors.is_empty());
    }
    #[test]
    fn ranges_and_encoding_preserve_multiline_unicode_fields() {
        for delimiter in [',', '\t', '|'] {
            let text = format!("\u{feff}中文{delimiter}\"a{delimiter}b\r\nc\"{delimiter}\r\n");
            let rows = parse(&text, delimiter).unwrap();
            let ranges = cell_ranges(&text, delimiter);
            assert_eq!(ranges.len(), rows.len());
            for (row, spans) in rows.iter().zip(ranges) {
                assert_eq!(row.len(), spans.len());
                for (value, span) in row.iter().zip(spans) {
                    let raw = &text[span];
                    let decoded = parse(raw, delimiter).unwrap().into_iter().next().and_then(|row| row.into_iter().next()).unwrap_or_default();
                    assert_eq!(*value, decoded);
                }
            }
            for value in ["", "中文", "a\"b", "two\nlines", "a,b\t|c"] {
                let serialized = encoded(value, delimiter);
                let decoded = parse(&serialized, delimiter).unwrap().into_iter().next().and_then(|row| row.into_iter().next()).unwrap_or_default();
                assert_eq!(value, decoded);
            }
        }
    }
    #[test]
    fn quoted_delimiters_newlines_and_empty_cells() {
        assert_eq!(parse("\u{feff}a,\"b,c\",\"d\"\"e\"\r\n1,\"two\nlines\",\r\n", ',').unwrap(),
            vec![vec!["a", "b,c", "d\"e"], vec!["1", "two\nlines", ""]]);
        assert_eq!(parse("a\tb\n1\t\t", '\t').unwrap(), vec![vec!["a", "b"], vec!["1", "", ""]]);
        assert_eq!(parse("a|\"b|c\"|\n", '|').unwrap(), vec![vec!["a", "b|c", ""]]);
        assert!(parse("\"unfinished", ',').is_err());
        assert!(parse("\"closed\"extra", ',').is_err());
        assert!(parse("", ',').unwrap().is_empty());
    }
}
