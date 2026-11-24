//! Read-only project tab, asynchronously loaded and rasterized for display.
use super::*;
use std::sync::mpsc;
enum ResultEvent { Loaded(Box<Editor>,RgbaImage), Rendered(RgbaImage), Failed(String) }
pub(crate) struct ProjectPreview {
    editor: Option<Box<Editor>>,
    receiver: Option<mpsc::Receiver<ResultEvent>>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
    rendered_revision: u64,
    last_refresh:Option<std::time::Instant>,
    pending_change:Option<(u64,std::time::Instant)>,
}
impl ProjectPreview {
    pub fn new(path:&Path,ctx:&egui::Context)->Self {
        let (send,receive)=mpsc::channel();let path=path.to_owned();let ctx=ctx.clone();
        std::thread::spawn(move||{
            let result=(||{let mut editor=Box::new(Editor::default());editor.load_project(&path)?;let image=render(&editor);Ok::<_,String>((editor,image))})();
            let event=match result {Ok((editor,image))=>ResultEvent::Loaded(editor,image),Err(error)=>ResultEvent::Failed(error)};
            let _=send.send(event);ctx.request_repaint();
        });
        Self{editor:None,receiver:Some(receive),texture:None,error:None,rendered_revision:0,last_refresh:None,pending_change:None}
    }
    pub fn active(&self)->bool {self.editor.as_ref().is_some_and(|e|e.active())}
    fn poll(&mut self,ctx:&egui::Context,path:&Path){
        let event=self.receiver.as_ref().and_then(|r|r.try_recv().ok());
        if let Some(event)=event {
            self.receiver=None;
            let image=match event {
                ResultEvent::Loaded(editor,image)=>{self.rendered_revision=editor.revision;self.last_refresh=Some(std::time::Instant::now());self.editor=Some(editor);Some(image)},
                ResultEvent::Rendered(image)=>Some(image),
                ResultEvent::Failed(error)=>{self.error=Some(error);None},
            };
            if let Some(image)=image {let data=tile_preview_image(&image);if let Some(texture)=self.texture.as_mut(){texture.set(data,egui::TextureOptions::LINEAR);}else{self.texture=Some(ctx.load_texture(format!("tile-project-preview-{}",path.display()),data,egui::TextureOptions::LINEAR));}}
        }
    }
    pub fn show_editor(&mut self,ctx:&egui::Context,path:&Path){
        self.poll(ctx,path);
        if let Some(editor)=&mut self.editor {
            editor.show(ctx,path,None);
            if editor.discard_requested {
                // Replacing the receiver drops any in-flight rendering of discarded edits.
                // Reload the saved file before allowing a new preview or edit session.
                *self=Self::new(path,ctx);
                return;
            }
            let now=std::time::Instant::now();
            if editor.revision!=self.rendered_revision{
                if self.pending_change.is_none_or(|(revision,_)|revision!=editor.revision){self.pending_change=Some((editor.revision,now));}
                if self.receiver.is_none()&&editor.before.is_none(){
                    let changed_at=self.pending_change.unwrap().1;
                    let quiet_until=changed_at+std::time::Duration::from_millis(250);
                    let deadline=self.last_refresh.map_or(quiet_until,|last|(last+std::time::Duration::from_secs(1)).max(quiet_until));
                    if now<deadline{ctx.request_repaint_after(deadline-now);}else{
                        let state=editor.snapshot();self.rendered_revision=editor.revision;self.last_refresh=Some(now);self.pending_change=None;
                        let(send,receive)=mpsc::channel();self.receiver=Some(receive);let ctx=ctx.clone();
                        std::thread::spawn(move||{let editor=from_snapshot(state);let _=send.send(ResultEvent::Rendered(render(&editor)));ctx.request_repaint();});
                    }
                }
            }else{self.pending_change=None;}

        }
    }
    pub fn show(&mut self,ui:&mut egui::Ui,path:&Path){
        self.poll(ui.ctx(),path);
        ui.horizontal_wrapped(|ui|{
            ui.heading(path.file_name().unwrap_or_default().to_string_lossy());
            if ui.add_enabled(self.editor.as_ref().is_some_and(|e|!e.elevated),egui::Button::new(tr("绘Tile"))).clicked(){
                if let Some(editor)=&mut self.editor {editor.open=true;crate::native_window::wake(ui.ctx(),egui::Id::new(("pixel_tile",path)));}
            }
        });
        if self.editor.as_ref().is_none_or(|e|!e.elevated) {ui.label(tr("工程预览；请进入绘Tile进行编辑。"));}
        if let Some(error)=&self.error {ui.colored_label(ui.visuals().error_fg_color,error);return;}
        let Some(editor)=&self.editor else{ui.spinner();return;};
        let (cols,rows)=if editor.multiple{(editor.columns,editor.rows)}else{(1,1)};
        let shape=if editor.hex==1{"尖顶六边形"}else if editor.hex==2{"平顶六边形"}else if editor.diamond{"菱形"}else{"矩形"};
        if editor.elevated {ui.label(tr("高度Tile本版暂不支持编辑，仅提供只读预览。"));}
        ui.label(format!("{cols}×{rows} {} Tile{}",tr(shape),if editor.elevated{format!(" · {}",tr("带高度"))}else{String::new()}));
        ui.label(format!("{}: {}×{} px · {}: {} · {}: {}",tr("单格尺寸"),editor.pixels.width(),editor.pixels.height(),tr("图层"),editor.layers.len(),tr("素材"),editor.materials.len()));
        let Some(texture)=&self.texture else{return;};
        let available=ui.available_size().max(Vec2::splat(1.));
        let (area,_)=ui.allocate_exact_size(available,egui::Sense::hover());
        let size=texture.size_vec2();let scale=((area.width()-24.).max(1.)/size.x).min((area.height()-24.).max(1.)/size.y);
        let rect=Rect::from_center_size(area.center(),size*scale);
        let painter=ui.painter_at(area);
        editor.paint_background(&painter,rect,rect.min,Vec2::splat(1.));
        painter.image(texture.id(),rect,Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),Color32::WHITE);
    }
}
fn from_snapshot(state:Snapshot)->Editor {
    let mut e=Editor::default();e.fx_light=state.fx_light;e.layers=state.layers;e.active_layer=state.active_layer;e.tiles=state.tiles;e.selected=state.selected;e.pixels=e.tiles[e.selected].clone();e.layout=state.layout;e.columns=state.columns;e.rows=state.rows;e.multiple=state.multiple;e.diamond=state.diamond;e.hex=state.hex;e.elevated=state.elevated;e.footprint=state.footprint;e
}
fn render(editor:&Editor)->RgbaImage {
    let (w,h)=editor.pixels.dimensions();let (cols,rows)=if editor.multiple{(editor.columns,editor.rows)}else{(1,1)};
    let mut instances=Vec::new();let mut bounds=Rect::NOTHING;
    for row in 0..rows {for col in 0..cols {
        let center=editor.mapped_center(col as i32,row as i32,if editor.elevated{editor.footprint[0]}else{w},if editor.elevated{editor.footprint[1]}else{h});
        let ground=Rect::from_center_size(center.to_pos2(),egui::vec2(if editor.elevated{editor.footprint[0]}else{w} as f32,if editor.elevated{editor.footprint[1]}else{h} as f32));
        let rect=if editor.elevated{Rect::from_min_size(egui::pos2(center.x-w as f32/2.,center.y+editor.footprint[1] as f32/2.-h as f32),egui::vec2(w as f32,h as f32))}else{ground};
        bounds=bounds.union(rect);instances.push((col,row,rect,ground));
    }}
    let scale=(1600./bounds.width().max(bounds.height()).max(1.)).min(4.);
    let width=(bounds.width()*scale).ceil().max(1.) as u32;let height=(bounds.height()*scale).ceil().max(1.) as u32;
    let mut out=RgbaImage::new(width,height);
    if editor.elevated{instances.sort_by_key(|(col,row,_,_)|col+row);}
    for &(col,row,rect,_) in &instances {
        let mut image=editor.composite(editor.tile_at(col as i32,row as i32));
        for (x,y,pixel) in image.enumerate_pixels_mut(){if !editor.inside_tile(x as i32,y as i32,w,h){*pixel=Rgba([0;4]);}}
        let scaled=image::imageops::resize(&image,(rect.width()*scale).round().max(1.) as u32,(rect.height()*scale).round().max(1.) as u32,image::imageops::FilterType::Nearest);
        image::imageops::overlay(&mut out,&scaled,((rect.left()-bounds.left())*scale).round() as i64,((rect.top()-bounds.top())*scale).round() as i64);
    }
    for (_,_,_,ground) in instances {
        let rect=Rect::from_min_max(((ground.min-bounds.min)*scale).to_pos2(),((ground.max-bounds.min)*scale).to_pos2());
        let points=if editor.hex!=0{hex_outline(rect,editor.hex==2)}else if editor.diamond{vec![rect.center_top(),rect.right_center(),rect.center_bottom(),rect.left_center()]}else{vec![rect.left_top(),rect.right_top(),rect.right_bottom(),rect.left_bottom()]};
        for i in 0..points.len(){let a=points[i];let b=points[(i+1)%points.len()];line((a.x.round() as i32,a.y.round() as i32),(b.x.round() as i32,b.y.round() as i32),|x,y|{if x>=0&&y>=0&&x<width as i32&&y<height as i32{out.put_pixel(x as u32,y as u32,Rgba([255,255,255,200]));}});}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_and_live_snapshot_preview_use_the_same_shadow_direction() {
        let source=RgbaImage::from_fn(8,8,|x,y| if x==3 && y==3 {Rgba([170,180,190,255])} else {Rgba([0;4])});
        let project=serde_json::json!({
            "version":10,"width":8,"height":8,"selected":0,"tiles":[source.as_raw()],"layout":vec![0usize;64],
            "columns":1,"rows":1,"multiple":false,"diamond":false,"elevated":false,"footprint":[8,8],
            "fx_light":{"angle":0,"altitude":30},
            "layers":[{"name":"fx","visible":true,"locked":false,"tiles":[source.as_raw()],
                "settings":{"fill":0,"shadow":true,"shadow_radius":0,"shadow_opacity":100,
                    "shadow_shape":{"direction":[90,2],"global":true}}}]
        });
        let path=std::env::temp_dir().join(format!("tile-fx-light-{}-{}.uytile",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&path,serde_json::to_vec(&project).unwrap()).unwrap();
        let mut editor=Editor::default();
        let loaded=editor.load_project(&path);
        std::fs::remove_file(path).unwrap();
        loaded.unwrap();
        let image=editor.composite(0);
        assert_eq!(image.get_pixel(1,3)[3],255);
        assert_eq!(image.get_pixel(3,3)[3],0);
        assert_eq!(from_snapshot(editor.snapshot()).composite(0),image);
        assert_eq!(editor.export_cell(0),image);
    }
    #[test]
    fn discard_reloads_saved_project_and_rejects_stale_render(){
        let image=RgbaImage::from_pixel(8,4,Rgba([55,100,150,255]));
        let project=serde_json::json!({"version":1,"width":8,"height":4,"tiles":[image.as_raw()],"selected":0,"layout":vec![0usize;64],"columns":1,"rows":1,"multiple":false,"diamond":false,"elevated":false,"footprint":[8,4]});
        let path=std::env::temp_dir().join(format!("tile-discard-{}-{}.uytile",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&path,serde_json::to_vec(&project).unwrap()).unwrap();
        let (old_sender,old_receiver)=mpsc::channel();let mut editor=Box::new(Editor::default());editor.discard_requested=true;
        let mut preview=ProjectPreview{editor:Some(editor),receiver:Some(old_receiver),texture:None,error:None,rendered_revision:42,last_refresh:None,pending_change:None};
        preview.show_editor(&egui::Context::default(),&path);
        assert!(preview.editor.is_none());assert_eq!(preview.rendered_revision,0);
        assert!(old_sender.send(ResultEvent::Rendered(RgbaImage::new(1,1))).is_err());
        let event=preview.receiver.take().unwrap().recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        std::fs::remove_file(path).unwrap();
        match event{ResultEvent::Loaded(editor,_)=>assert_eq!(editor.pixels,image),ResultEvent::Failed(error)=>panic!("{error}"),_=>panic!("expected saved project reload")}
    }
    #[test]
    fn rectangular_project_preview_covers_all_cells_with_preserved_aspect_ratio() {
        let mut editor=Editor::default();editor.diamond=false;editor.multiple=true;editor.columns=2;editor.rows=3;
        editor.pixels=RgbaImage::from_pixel(8,4,Rgba([50,100,150,255]));editor.tiles=vec![editor.pixels.clone();6];
        for row in 0..3{for col in 0..2{editor.layout[row*8+col]=row*2+col;}}
        let image=render(&editor);assert_eq!(image.dimensions(),(64,48));
        assert_eq!(image.get_pixel(16,8).0,[50,100,150,255]);
        assert_eq!(image.get_pixel(48,40).0,[50,100,150,255]);
    }
}
