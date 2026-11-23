//! Project-local immutable material samples and a dedicated per-cell material layer.
use super::*;
#[derive(Clone, PartialEq)]
pub(super) struct Material { pub name: String, pub pixels: RgbaImage }
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct StoredMaterial { pub name: String, pub pixels: Vec<u8> }
impl Editor {
    pub(super) fn ensure_material_base(&mut self) -> Option<usize> {
        self.ensure_layers();
        if let Some(i) = self.layers.iter().position(|l| l.material) { return Some(i); }
        if self.layers.first().is_some_and(|l| l.name == "BaseLayer" && l.locked) {
            self.finish_stroke(); self.sync_layer(); self.invalidate_textures(); self.changed=true; self.layers[0].material = true; self.layers[0].name = "BaseLayer".into(); return Some(0);
        }
        let added: usize = self.tiles.iter().map(|p| p.as_raw().len()).sum();
        let total: usize = self.layers.iter().flat_map(|l| &l.tiles).map(|p| p.as_raw().len()).sum();
        if self.layers.len() >= 32 || total + added > LAYER_BYTES { self.message = tr("图层数量或内存已达上限，无法导入新图层。").into(); return None; }
        self.finish_stroke(); self.sync_layer(); self.invalidate_textures(); self.changed=true;
        let tiles = self.tiles.iter().map(|p| RgbaImage::new(p.width(),p.height())).collect();
        self.layers.insert(0,Layer { material:true, name:"BaseLayer".into(), visible:true, locked:true, tiles });
        self.active_layer += 1;
        Some(0)
    }
    pub(super) fn add_material(&mut self, name: String, pixels: RgbaImage) {
        let bytes: usize = self.materials.iter().map(|m| m.pixels.as_raw().len()).sum();
        if pixels.dimensions() != self.pixels.dimensions() { self.message = tr("Tile 尺寸必须与当前画布一致。").into(); return; }
        if self.materials.len() >= 64 || bytes + pixels.as_raw().len() > 16*1024*1024 {
            self.message = tr("素材库最多64项、总像素16 MiB。").into(); return;
        }
        self.record_layer_change();
        self.materials.push(Material { name: name.chars().take(64).collect(), pixels });
    }
    pub(super) fn assign_material(&mut self, material: Option<usize>) {
        let Some(base) = self.ensure_material_base() else { return; };
        let pixels = match material {
            Some(i) => match self.materials.get(i) { Some(m) => m.pixels.clone(), None => return },
            None => RgbaImage::new(self.pixels.width(),self.pixels.height()),
        };
        if self.layers[base].tiles[self.selected] == pixels { return; }
        self.record_layer_change();
        self.layers[base].tiles[self.selected] = pixels;
        if self.active_layer == base {
            self.tiles = self.layers[base].tiles.clone(); self.pixels = self.tiles[self.selected].clone();
        }
    }
    fn import_material_path(&mut self, path: &std::path::Path) -> Result<(),String> {
        let mut reader=image::ImageReader::open(path).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?;
        let mut limits=image::Limits::default(); limits.max_image_width=Some(MAX_SIDE); limits.max_image_height=Some(MAX_SIDE); limits.max_alloc=Some(32*1024*1024); reader.limits(limits);
        let image=reader.decode().map_err(|e|e.to_string())?.to_rgba8();
        let before=self.materials.len();
        self.add_material(path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),image);
        if self.materials.len()==before { return Err(self.message.clone()); }
        Ok(())
    }
    pub(super) fn material_controls(&mut self, ui: &mut egui::Ui) {
        let Some(base) = self.ensure_material_base() else { return; };
        ui.separator();
        egui::CollapsingHeader::new(egui::RichText::new(tr("工程素材库")).heading().strong())
            .id_salt("pixel_tile_materials_group").default_open(true).show(ui,|ui| {
        ui.small(tr("选择素材填入当前格；素材层与上方绘画层独立。"));
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("导入素材")).clicked() {
                self.material_drop_status=None;
                if let Some(path) = rfd::FileDialog::new().add_filter("Image", &["png","jpg","jpeg","webp","bmp","tga"]).pick_file() {
                    if let Err(error)=self.import_material_path(&path) { self.message=error; }
                }
            }
            if ui.button(tr("当前合成存为素材")).clicked() {
                self.material_drop_status=None;
                let image = self.export_cell(self.selected);
                self.add_material(format!("{} {}",tr("素材"),self.materials.len()+1),image);
            }
        });
        if ui.button(tr("清空当前格素材")).clicked() { self.assign_material(None); }
        let mut apply = None; let mut remove = None;
        let library = egui::Frame::new().inner_margin(4).stroke(egui::Stroke::new(1_f32,ui.visuals().widgets.noninteractive.bg_stroke.color)).show(ui,|ui| {
            egui::ScrollArea::vertical().id_salt("project_materials").max_height(200.).show(ui,|ui| {
                ui.set_min_height(112.);
                ui.horizontal_wrapped(|ui| {
                    for i in 0..self.materials.len() {
                        let thumb=image::imageops::thumbnail(&self.materials[i].pixels,40,40);
                        let data=egui::ColorImage::from_rgba_unmultiplied([thumb.width() as usize,thumb.height() as usize],thumb.as_raw());
                        while self.material_thumbnails.len()<=i { self.material_thumbnails.push(ui.ctx().load_texture(format!("material-{i}"),data.clone(),egui::TextureOptions::NEAREST)); }
                        self.material_thumbnails[i].set(data,egui::TextureOptions::NEAREST);
                        let selected=self.layers[base].tiles[self.selected]==self.materials[i].pixels;
                        let response=ui.add_sized([48.,48.],egui::Button::new("").selected(selected));
                        let rect=response.rect.shrink(4.);
                        super::super::seamless::checker_background(ui.painter(),rect,rect.min,Vec2::splat(1.),&super::super::ImageBackground::Checkerboard,None);
                        ui.painter().image(self.material_thumbnails[i].id(),Rect::from_center_size(rect.center(),egui::vec2(thumb.width() as f32,thumb.height() as f32)),Rect::from_min_max(Pos2::ZERO,egui::pos2(1.,1.)),Color32::WHITE);
                        if response.clicked() { apply=Some(i); }
                        response.on_hover_text(&self.materials[i].name).context_menu(|ui| {
                            if ui.button(tr("移除素材库")).on_hover_text(tr("从素材库移除，已填入格子的内容保留。")).clicked() { remove=Some(i); ui.close_menu(); }
                        });
                    }
                });
                if self.materials.is_empty() { ui.label(tr("拖入图片加入素材库")); }
            });
        });
        let region=library.response.rect.intersect(ui.clip_rect());
        let pointer=if cfg!(windows) {crate::native_window::file_drop_position(ui.ctx())}
            else {ui.input(|i|i.pointer.latest_pos())};
        let over=pointer.is_some_and(|p|region.contains(p));
        if over && ui.input(|i|!i.raw.hovered_files.is_empty()) {
            ui.painter().rect_stroke(region,2.,egui::Stroke::new(2_f32,ui.visuals().selection.stroke.color),egui::StrokeKind::Inside);
        }
        if over && ui.input(|i|!i.raw.dropped_files.is_empty()) {
            // These editor windows run before the root file opener; claim only drops in this region.
            let files=ui.input_mut(|i|std::mem::take(&mut i.raw.dropped_files));
            let mut added=0; let mut errors=Vec::new();
            for file in files {
                if let Some(path)=file.path {
                    let before=self.materials.len();
                    if let Err(error)=self.import_material_path(&path) { errors.push(format!("{}: {error}",path.display())); }
                    if self.materials.len()>before { added+=1; }
                } else { errors.push(tr("仅支持拖入本地图片文件。").to_owned()); }
            }
            self.message=format!("{}: {added}",tr("已加入素材库"));
            if !errors.is_empty() { self.message.push_str(&format!("\n{}",errors.join("\n"))); }
            self.material_drop_status=Some((!errors.is_empty(),self.message.clone()));
        }
        if !over && ui.input(|i|!i.raw.dropped_files.is_empty()) {
            self.material_drop_status=Some((true,tr("请将图片拖入素材库边框内。").to_owned()));
        }
        if let Some((failed,status))=&self.material_drop_status {
            let color=if *failed {ui.visuals().error_fg_color}else{ui.visuals().text_color()};
            ui.add(egui::Label::new(egui::RichText::new(status).color(color)).wrap());
        }
        if let Some(i)=apply { self.assign_material(Some(i)); }
        if let Some(i)=remove { self.record_layer_change(); self.materials.remove(i); }
        ui.small(tr("导出合成可见素材与绘画层；素材库随工程保存。"));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_and_multi_modes_share_one_base_and_new_imports_keep_a_drawing_layer() {
        let mut e=Editor::default();
        assert!(!e.multiple);
        assert_eq!(e.ensure_material_base(),Some(0));
        assert_eq!(e.layers.len(),2);
        e.assign_material(None);
        for multiple in [true,false,true,false] {
            e.multiple=multiple;
            assert_eq!(e.ensure_material_base(),Some(0));
            assert_eq!(e.layers.iter().filter(|l|l.material).count(),1);
            assert_eq!(e.layers.len(),2);
        }
        let image=RgbaImage::from_pixel(128,64,Rgba([20,80,160,255]));
        e.reset_document(image.clone());
        assert!(e.layers[0].material && e.layers[0].locked);
        assert_eq!(e.layers[0].tiles[0],image);
        assert!(!e.layers[e.active_layer].material && !e.layers[e.active_layer].locked);
        assert!(e.pixels.pixels().all(|p|p[3]==0));
        assert_eq!(e.materials.len(),1);
        e.reset_document(RgbaImage::new(128,64));
        assert!(e.layers[0].material);
        assert_eq!(e.layers.len(),2);
        assert!(e.materials.is_empty());
    }
    #[test]
    fn cell_material_assignment_preserves_overlays_and_base_cannot_be_deleted() {
        let mut e=Editor::default();
        e.ensure_layers(); e.multiple=true; e.columns=2; e.rows=1;
        e.independent_slots().unwrap();
        let base=e.ensure_material_base().unwrap();
        assert_eq!(base,0);
        assert!(e.layers[base].material && e.layers[base].locked);
        e.write_pixel(0,64,32,Rgba([200,10,20,255]));
        let overlay=e.pixels.clone();
        e.add_material("sample".into(),RgbaImage::from_pixel(128,64,Rgba([20,180,40,255])));
        e.assign_material(Some(0));
        assert_eq!(e.pixels,overlay);
        assert_eq!(e.layers[base].tiles[0].get_pixel(64,32).0,[20,180,40,255]);
        assert!(e.layers[base].tiles[1].pixels().all(|p|p[3]==0));
        let count=e.layers.len(); e.delete_layer(base); assert_eq!(e.layers.len(),count);
        e.assign_material(None);
        assert!(e.layers[base].tiles[0].pixels().all(|p|p[3]==0));
        assert_eq!(e.pixels,overlay);
        e.history(false);
        assert_eq!(e.layers[base].tiles[0].get_pixel(64,32).0,[20,180,40,255]);
        e.delete_layer(e.active_layer);
        assert_eq!(e.layers.len(),2);
        assert!(e.layers[0].material);
        assert!(!e.layers[1].locked && !e.layers[1].material);
    }
}
