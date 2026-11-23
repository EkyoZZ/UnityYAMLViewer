//! Shared preset identity for the compact tip strip and stroke cards.
use super::*;
#[derive(Clone,Copy)]
pub(super) struct Preset { pub hardness:u8,pub size:bool,pub opacity:bool,pub flow:bool,pub label:&'static str }
impl Preset {
    pub fn matches(self,h:u8,s:bool,o:bool,f:bool)->bool{(self.hardness,self.size,self.opacity,self.flow)==(h,s,o,f)}
}
pub(super) const PRESETS:[Preset;8]=[
    Preset{hardness:0,size:false,opacity:false,flow:false,label:"柔边圆"},
    Preset{hardness:100,size:false,opacity:false,flow:false,label:"硬边圆"},
    Preset{hardness:0,size:true,opacity:false,flow:false,label:"柔边圆压力大小"},
    Preset{hardness:100,size:true,opacity:false,flow:false,label:"硬边圆压力大小"},
    Preset{hardness:0,size:false,opacity:true,flow:false,label:"柔边圆压力不透明度"},
    Preset{hardness:100,size:false,opacity:true,flow:false,label:"硬边圆压力不透明度"},
    Preset{hardness:0,size:false,opacity:true,flow:true,label:"柔边圆压力不透明度和流量"},
    Preset{hardness:100,size:false,opacity:true,flow:true,label:"硬边圆压力不透明度和流量"},
];
fn tip_image(preset:Preset)->RgbaImage {
    let mut image=RgbaImage::new(28,28);
    for (x,y,pixel) in image.enumerate_pixels_mut(){
        let mut alpha=0.;
        for sy in 0..2 {for sx in 0..2 {let dx=(x as f32+(sx as f32+0.5)/2.-14.)/11.;let dy=(y as f32+(sy as f32+0.5)/2.-14.)/11.;alpha+=round_tip_coverage((dx*dx+dy*dy).sqrt(),preset.hardness);}}
        *pixel=Rgba([255,255,255,(alpha/4.*255.).round() as u8]);
    }image
}
fn stroke_image(preset:Preset)->RgbaImage {
    let mut image=RgbaImage::new(224,36);
    for (x,y,pixel) in image.enumerate_pixels_mut(){
        let local=(x as f32+0.5).clamp(14.,210.);let t=(local-14.)/196.;
        let pressure=0.18+0.82*(std::f32::consts::PI*t).sin().max(0.);
        let radius=8.*if preset.size{pressure}else{1.};
        let center_y=18.+4.*(t*std::f32::consts::TAU).sin();
        let dx=(x as f32+0.5-local)/radius;let dy=(y as f32+0.5-center_y)/radius;
        let coverage=round_tip_coverage((dx*dx+dy*dy).sqrt(),preset.hardness);
        let opacity=if preset.opacity{pressure}else{1.};let flow=if preset.flow{pressure.sqrt()}else{1.};
        *pixel=Rgba([255,255,255,(coverage*opacity*flow*255.).round() as u8]);
    }image
}
pub(super) fn show(ui:&mut egui::Ui,hardness:u8,size:bool,opacity:bool,flow:bool)->Option<Preset> {
    let cache_id=egui::Id::new("tile_brush_preset_previews_v1");
    let mut textures=ui.ctx().data(|d|d.get_temp::<Vec<(egui::TextureHandle,egui::TextureHandle)>>(cache_id));
    if textures.is_none(){
        let cache=PRESETS.iter().enumerate().map(|(i,p)|(
            ui.ctx().load_texture(format!("tile-tip-preset-{i}"),tile_preview_image(&tip_image(*p)),egui::TextureOptions::LINEAR),
            ui.ctx().load_texture(format!("tile-stroke-preset-{i}"),tile_preview_image(&stroke_image(*p)),egui::TextureOptions::LINEAR),
        )).collect::<Vec<_>>();
        ui.ctx().data_mut(|d|d.insert_temp(cache_id,cache.clone()));textures=Some(cache);
    }
    let textures=textures.unwrap();let mut chosen=None;let mut selected=PRESETS.iter().position(|p|p.matches(hardness,size,opacity,flow));
    let mut scroll_target=None;
    ui.horizontal(|ui|{
        ui.spacing_mut().item_spacing.x=3.;
        for (i,preset) in PRESETS.iter().enumerate(){
            let response=ui.add_sized([28.,28.],egui::Button::new("").selected(selected==Some(i))).on_hover_text(tr(preset.label));
            ui.painter().image(textures[i].0.id(),response.rect.shrink(2.),Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),ui.visuals().text_color());
            if response.clicked(){selected=Some(i);chosen=Some(*preset);scroll_target=Some(i);}
        }
    });
    ui.separator();
    egui::CollapsingHeader::new(tr("常规画笔")).id_salt("tile_general_brushes").default_open(true).open(scroll_target.map(|_|true)).show_unindented(ui,|ui|{
    let height=(ui.ctx().screen_rect().height()-420.).clamp(90.,260.);
    egui::ScrollArea::vertical().id_salt("tile_brush_preset_cards").max_height(height).show(ui,|ui|{
        for (i,preset) in PRESETS.iter().enumerate(){
            let width=ui.available_width().max(245.);
            let label=ui.painter().layout(tr(preset.label).to_owned(),egui::FontId::proportional(12.),ui.visuals().text_color(),width-16.);
            let response=ui.add_sized([width,44.+label.size().y],egui::Button::new("").selected(selected==Some(i)));
            let preview=Rect::from_min_size(response.rect.min+egui::vec2(8.,3.),egui::vec2((response.rect.width()-16.).min(224.),36.));
            ui.painter().image(textures[i].1.id(),preview,Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),ui.visuals().text_color());
            ui.painter().galley(response.rect.min+egui::vec2(8.,40.),label,ui.visuals().text_color());
            if response.clicked(){chosen=Some(*preset);}
            if scroll_target==Some(i){response.scroll_to_me(Some(egui::Align::Center));}
        }
    });
    });
    chosen
}
#[cfg(test)]
mod tests {use super::*;#[test]fn preset_identity_is_unique_and_soft_tip_has_alpha_falloff(){for (i,p) in PRESETS.iter().enumerate(){assert_eq!(PRESETS.iter().filter(|q|q.matches(p.hardness,p.size,p.opacity,p.flow)).count(),1);assert!(PRESETS[i].matches(p.hardness,p.size,p.opacity,p.flow));}let soft=tip_image(PRESETS[0]);let hard=tip_image(PRESETS[1]);assert!(soft.get_pixel(20,14)[3]<hard.get_pixel(20,14)[3]);assert_eq!(stroke_image(PRESETS[0]).dimensions(),(224,36));}}

/// The compact tip uses the same radial coverage as the preset strip and painting engine.
pub(super) fn paint_current_tip(ui:&egui::Ui,rect:Rect,diameter:f32,hardness:u8){
    let id=egui::Id::new(("tile_current_tip",hardness));
    let texture=ui.ctx().data(|d|d.get_temp::<egui::TextureHandle>(id)).unwrap_or_else(||{
        let image=tip_image(Preset{hardness,size:false,opacity:false,flow:false,label:""});
        let texture=ui.ctx().load_texture(format!("tile-current-tip-{hardness}"),tile_preview_image(&image),egui::TextureOptions::LINEAR);
        ui.ctx().data_mut(|d|d.insert_temp(id,texture.clone()));texture
    });
    let size=(diameter*28./22.).min(rect.width().min(rect.height())-2.).max(2.);
    ui.painter().image(texture.id(),Rect::from_center_size(rect.center(),Vec2::splat(size)),Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),ui.visuals().text_color());
}
