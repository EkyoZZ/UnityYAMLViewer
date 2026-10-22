use eframe::egui::{self, Color32, RichText, Stroke};

pub(crate) const BLUE: Color32 = Color32::from_rgb(0, 127, 255);

// Separate styles let System preference retain egui's live OS theme detection.
pub fn install(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            let dark = theme == egui::Theme::Dark;
            let (panel, surface, input, border, text, selected, hover) = if dark {
                (rgb(46,48,61), rgb(38,40,51), rgb(35,37,47), rgb(69,71,83), rgb(245,246,250), rgb(33,78,129), rgb(52,56,70))
            } else {
                (rgb(245,245,245), Color32::WHITE, Color32::WHITE, rgb(211,214,220), rgb(24,26,31), rgb(207,227,249), rgb(231,240,251))
            };
            let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
            visuals.panel_fill = panel;
            visuals.window_fill = surface;
            visuals.extreme_bg_color = input;
            visuals.faint_bg_color = if dark { rgb(42,44,56) } else { rgb(237,237,237) };
            visuals.override_text_color = Some(text);
            visuals.hyperlink_color = BLUE;
            visuals.selection.bg_fill = selected;
            visuals.selection.stroke = Stroke::new(1.0_f32, if dark { Color32::WHITE } else { text });
            visuals.window_stroke = Stroke::new(1.0_f32, border);
            visuals.window_corner_radius = egui::CornerRadius::same(8);
            for widget in [&mut visuals.widgets.noninteractive, &mut visuals.widgets.inactive, &mut visuals.widgets.hovered, &mut visuals.widgets.active, &mut visuals.widgets.open] {
                widget.bg_fill = surface;
                widget.weak_bg_fill = surface;
                widget.bg_stroke = Stroke::new(1.0_f32, border);
                widget.fg_stroke = Stroke::new(1.0_f32, text);
                widget.corner_radius = egui::CornerRadius::same(4);
            }
            visuals.widgets.noninteractive.bg_fill = panel;
            visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text);
            visuals.widgets.hovered.bg_fill = hover;
            visuals.widgets.hovered.weak_bg_fill = hover;
            visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BLUE);
            visuals.widgets.active.bg_fill = selected;
            visuals.widgets.active.weak_bg_fill = selected;
            visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, BLUE);
            visuals.widgets.open.bg_fill = selected;
            visuals.widgets.open.weak_bg_fill = selected;
            style.visuals = visuals;
        });
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 { Color32::from_rgb(r,g,b) }

/// Shared appearance for all standard sliders, including read-only property ranges.
/// Scoped styling avoids changing nearby buttons, inputs or custom color gradients.
pub fn slider(ui: &mut egui::Ui, enabled: bool, slider: egui::Slider<'_>) -> egui::Response {
    ui.scope(|ui| {
        let dark = ui.visuals().dark_mode;
        ui.spacing_mut().slider_rail_height = 5.0;
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.bg_fill = if dark { rgb(112,122,142) } else { rgb(161,175,194) };
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.5_f32, if dark { rgb(126,190,255) } else { rgb(0,91,190) });
        visuals.widgets.noninteractive.bg_fill = if dark { rgb(160,177,202) } else { rgb(86,108,139) };
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.5_f32, if dark { rgb(214,229,250) } else { rgb(39,63,96) });
        visuals.widgets.hovered.bg_fill = BLUE;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.widgets.active.bg_fill = BLUE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.selection.bg_fill = BLUE;
        ui.add_enabled(enabled, slider.trailing_fill(true))
    }).inner
}

pub fn zoom_slider(ui: &mut egui::Ui, value: &mut f32, maximum: f32) -> egui::Response {
    slider(ui, true, egui::Slider::new(value,1.0..=maximum).logarithmic(true).suffix("×"))
}

pub fn set_preference(ctx: &egui::Context, preference: &str) {
    ctx.set_theme(match preference {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    });
}

/// Joined tabs with a solid blue active segment, matching the reference.
pub fn selector(ui: &mut egui::Ui, preference: &mut String) {
    selector_inner(ui, preference, false, [("light", "浅色"), ("dark", "深色"), ("system", "系统")]);
}
pub fn hierarchy_depth_selector(ui: &mut egui::Ui, preference: &mut String) {
    selector_inner(ui, preference, false, [("1", "1"), ("2", "2"), ("3", "3"), ("all", "全部")]);
}
pub fn video_decoder_selector(ui:&mut egui::Ui,preference:&mut String) {
    selector_inner(ui,preference,true,[("auto","自动"),("none","软解"),("d3d11va","硬解")]);
}
pub fn video_scale_selector(ui:&mut egui::Ui,preference:&mut String) {
    selector_inner(ui,preference,true,[("fit","适应窗口"),("native","原尺寸"),("free","自由缩放")]);
}

pub fn toolbar_selector(ui: &mut egui::Ui, preference: &mut String) {
    selector_inner(ui, preference, true, [("light", "浅色"), ("dark", "深色"), ("system", "系统")]);
}

/// Compact joined segments, sharing the toolbar theme selector's implementation.
pub fn compact_selector<const N:usize>(ui:&mut egui::Ui,value:&mut String,labels:[(&str,&str);N])->bool {
    let previous=value.clone();selector_inner(ui,value,true,labels);*value!=previous
}

pub fn background_selector(ui: &mut egui::Ui, preference: &mut String) {
    selector_inner(ui, preference, true, [("", "棋盘格"), ("gray", "灰色"), ("custom", "自定义")]);
}

/// Filled toolbar actions use the same height and text padding as theme tabs.
pub fn toolbar_button(ui: &egui::Ui, label: &str, update: bool) -> egui::Button<'static> {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width = ui.painter().layout_no_wrap(label.to_owned(), font, Color32::WHITE).size().x.ceil() + 12.0;
    let fill = if update { rgb(210,40,40) } else { BLUE };
    egui::Button::new(RichText::new(label.to_owned()).color(Color32::WHITE))
        .fill(fill).stroke(Stroke::new(1.0_f32, fill))
        .corner_radius(egui::CornerRadius::same(4))
        .min_size(egui::vec2(width, 24.0))
}

pub fn preview_mode_selector(ui:&mut egui::Ui,dynamic:&mut bool) -> bool {
    let mut value=if *dynamic {"dynamic"} else {"static"}.to_owned();
    selector_inner(ui,&mut value,true,[("static","静态"),("dynamic","动态")]);
    let next=value=="dynamic";let changed=next!=*dynamic;*dynamic=next;changed
}

pub(crate) fn preview_navigation_button(label:&str)->egui::Button<'_> {
    egui::Button::new(RichText::new(label).color(Color32::WHITE))
        .fill(BLUE).stroke(Stroke::new(1.0_f32,BLUE)).corner_radius(4.0)
}
fn selector_inner<const N:usize>(ui: &mut egui::Ui, preference: &mut String, compact: bool, labels: [(&str, &str); N]) {
    selector_inner_scaled(ui, preference, compact, labels, 1.0);
}

// Font sizes are supplied by the caller's style; scale the remaining geometry
// explicitly for controls embedded in a zoomable node canvas.
fn selector_inner_scaled<const N:usize>(ui: &mut egui::Ui, preference: &mut String, compact: bool, labels: [(&str, &str); N], scale: f32) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widths = labels.map(|(_, label)| if compact {
        ui.painter().layout_no_wrap(crate::localization::tr(label).to_owned(), font.clone(), BLUE).size().x.ceil() + 12.0 * scale
    } else { 208.0 * scale / N as f32 });
    let height = (if compact { 24.0 } else { 30.0 }) * scale;
    ui.allocate_ui_with_layout(egui::vec2(widths.iter().sum(), height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if compact { ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0) * scale; }
        for (index, (value, label)) in labels.into_iter().enumerate() {
            let selected = preference == value;
            let radius = (4.0 * scale).round() as u8;
            let rounding = egui::CornerRadius {
                nw: if index == 0 { radius } else { 0 }, sw: if index == 0 { radius } else { 0 },
                ne: if index == N-1 { radius } else { 0 }, se: if index == N-1 { radius } else { 0 },
            };
            let button = egui::Button::new(RichText::new(crate::localization::tr(label)).color(if selected { Color32::WHITE } else { BLUE }))
                .fill(if selected { BLUE } else { ui.visuals().window_fill })
                .stroke(Stroke::new(scale, BLUE)).corner_radius(rounding);
            let response = ui.add_sized([widths[index], height], button.wrap());
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, crate::localization::tr(label)));
            if response.clicked() { *preference = value.into(); }
        }
    });
}

/// Settings button using the shared, theme-tinted SVG asset.
pub fn preview_warning_details(ui:&mut egui::Ui,key:&str,label:&str,issues:&[String]) {
    if issues.is_empty() {return;}
    let id=ui.make_persistent_id(key);
    let mut open=ui.ctx().data_mut(|data|data.get_temp::<bool>(id)).unwrap_or(false);
    let color=ui.visuals().warn_fg_color;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x=4.0;
        let label=ui.add(egui::Label::new(RichText::new(format!("{label}: {}",issues.len())).color(color)).sense(egui::Sense::click()));
        let (rect,arrow)=ui.allocate_exact_size(egui::vec2(20.0,20.0),egui::Sense::click());
        crate::icons::paint(ui,if open {crate::icons::Icon::TriangleDown}else{crate::icons::Icon::TriangleRight},rect.shrink(2.0),color);
        if label.clicked() || arrow.clicked() {open=!open;}
    });
    ui.ctx().data_mut(|data|data.insert_temp(id,open));
    if open {
        let height=(ui.available_height()*0.25).clamp(40.0,160.0);
        egui::Frame::group(ui.style()).show(ui,|ui| {
            if ui.button(crate::localization::tr("复制错误详情")).clicked() {ui.ctx().copy_text(issues.join("\n"));}
            egui::ScrollArea::vertical().id_salt((key,"details")).max_height(height).show(ui,|ui| {
                for issue in issues {ui.add(egui::Label::new(RichText::new(issue).color(color)).wrap().selectable(true));}
            });
        });
    }
}

pub fn settings_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, crate::localization::tr("设置")));
    let visuals = ui.style().interact(&response);
    if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);
    }
    crate::icons::paint(ui, crate::icons::Icon::Gear,
        egui::Rect::from_center_size(rect.center(), egui::vec2(22.0, 22.0)), visuals.fg_stroke.color);
    response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(crate::localization::tr("设置"))
}

/// Lightweight preview toggle, aligned with the Inspector heading.
pub fn preview_toggle(ui: &mut egui::Ui, expanded: bool) -> egui::Response {
    let label = crate::localization::tr("预览");
    let font = egui::FontId::proportional(13.0);
    let color = ui.visuals().text_color();
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(galley.size().x + 30.0, 26.0), egui::Sense::click());
    if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, ui.visuals().widgets.hovered.bg_fill);
    }
    let icon = egui::pos2(rect.left() + 10.0, rect.center().y);
    let direction = if expanded { -1.0_f32 } else { 1.0_f32 };
    let stroke = egui::Stroke::new(1.4_f32, if response.hovered() { color } else { ui.visuals().weak_text_color() });
    ui.painter().add(egui::Shape::line(vec![
        icon + egui::vec2(-direction * 2.0, -4.0),
        icon + egui::vec2(direction * 2.0, 0.0),
        icon + egui::vec2(-direction * 2.0, 4.0),
    ], stroke));
    ui.painter().galley(egui::pos2(rect.left() + 21.0, rect.center().y - galley.size().y * 0.5), galley, color);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Reusable labeled switch. Pass a localized label and the caller-owned state.
/// Returns a normal response with clicked/changed state; supports ui.add_enabled_ui.
pub fn compact_toggle_switch(ui:&mut egui::Ui,label:&str,value:&mut bool) -> egui::Response {
    let font=egui::TextStyle::Button.resolve(ui.style());
    let width=ui.painter().layout_no_wrap(label.to_owned(),font,ui.visuals().text_color()).size().x+48.0;
    toggle_switch_width(ui,label,value,width)
}

pub fn toggle_switch(ui:&mut egui::Ui,label:&str,value:&mut bool) -> egui::Response {
    let width=ui.available_width().max(48.0);
    toggle_switch_width(ui,label,value,width)
}

fn toggle_switch_width(ui:&mut egui::Ui,label:&str,value:&mut bool,width:f32) -> egui::Response {
    let color=ui.visuals().text_color();
    let font=egui::TextStyle::Button.resolve(ui.style());
    let galley=ui.painter().layout(label.to_owned(),font,color,(width-48.0).max(1.0));
    let height=galley.size().y.max(26.0);
    let (rect,mut response)=ui.allocate_exact_size(egui::vec2(width,height),egui::Sense::click());
    if response.clicked() {*value=!*value;response.mark_changed();}
    response.widget_info(||egui::WidgetInfo::selected(egui::WidgetType::Checkbox,ui.is_enabled(),*value,label));
    let rail=egui::Rect::from_center_size(egui::pos2(rect.right()-20.0,rect.center().y),egui::vec2(40.0,24.0));
    let fill=if *value {rgb(45,125,250)} else if ui.visuals().dark_mode {rgb(62,62,66)} else {rgb(175,175,180)};
    ui.painter().rect_filled(rail,12.0,fill);
    let t=ui.ctx().animate_bool(response.id,*value);
    let center=egui::pos2(rail.left()+12.0+16.0*t,rail.center().y);
    ui.painter().circle_filled(center,10.0,if *value {Color32::WHITE} else {rgb(220,220,220)});
    ui.painter().galley(egui::pos2(rect.left(),rect.center().y-galley.size().y*0.5),galley,color);
    if response.has_focus() {ui.painter().rect_stroke(rail.expand(2.0),14.0,Stroke::new(1.0_f32,BLUE),egui::StrokeKind::Outside);}
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn preview_space_selector(ui:&mut egui::Ui,three_dimensional:&mut bool) {
    preview_space_selector_scaled(ui, three_dimensional, 1.0);
}

pub fn preview_space_selector_scaled(ui:&mut egui::Ui,three_dimensional:&mut bool,scale:f32) {
    let mut value=if *three_dimensional {"3d"} else {"2d"}.to_owned();
    selector_inner_scaled(ui,&mut value,true,[("2d","2D"),("3d","3D")],scale);
    *three_dimensional=value=="3d";
}

#[derive(Clone,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
#[serde(default)]
struct PreviewBackgroundSetting { mode:String, rgb:[u8;3] }
impl Default for PreviewBackgroundSetting {
    fn default()->Self {Self {mode:String::new(),rgb:[0;3]}}
}

/// All preview surfaces share this persisted preference; output backgrounds are separate.
pub fn shared_preview_background(ui:&mut egui::Ui,mode:&mut String,rgb:&mut [u8;3])->bool {
    use std::sync::{Mutex,OnceLock};
    static SETTING:OnceLock<Mutex<(PreviewBackgroundSetting,Option<String>)>>=OnceLock::new();
    let path=crate::app_cache::root().map(|p|p.join("preview_background.json"));
    let state=SETTING.get_or_init(|| {
        let mut setting=path.as_ref().and_then(|p|std::fs::read(p).ok())
            .and_then(|bytes|serde_json::from_slice::<PreviewBackgroundSetting>(&bytes).ok()).unwrap_or_default();
        if !matches!(setting.mode.as_str(),""|"gray"|"custom") {setting.mode.clear();}
        Mutex::new((setting,None))
    });
    let mut state=state.lock().unwrap_or_else(|e|e.into_inner());
    let old=state.0.clone();
    let mut current=old.clone();
    background_selector(ui,&mut current.mode);
    if current.mode=="custom" {ui.color_edit_button_srgb(&mut current.rgb);}
    if current!=old {
        let saved=(||->Result<(),String>{
            let path=path.as_ref().ok_or_else(||"Cannot locate preview settings directory".to_owned())?;
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;
            let bytes=serde_json::to_vec_pretty(&current).map_err(|e|e.to_string())?;
            std::fs::write(path,bytes).map_err(|e|e.to_string())
        })();
        state.1=saved.err();
        state.0=current;
        let viewports=ui.ctx().input(|input|input.raw.viewports.keys().copied().collect::<Vec<_>>());
        for viewport in viewports {ui.ctx().request_repaint_of(viewport);}
    }
    let changed=*mode!=state.0.mode || *rgb!=state.0.rgb;
    *mode=state.0.mode.clone();*rgb=state.0.rgb;
    if let Some(error)=&state.1 {ui.colored_label(ui.visuals().warn_fg_color,crate::localization::tr("预览背景设置保存失败")).on_hover_text(error);}
    changed
}

/// Shared upward zoom menu, matching the translucent preview badges.
pub(crate) fn zoom_preset_menu(ui:&mut egui::Ui,badge:egui::Rect,zoom:f32,text_right:f32)->Option<f32> {
    let id=ui.id().with("preview_zoom_presets");
    let response=ui.interact(badge,id.with("button"),egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked(){ui.memory_mut(|memory|memory.toggle_popup(id));}
    let ratio_width=ui.painter().layout_no_wrap(format!("{:.0}%",zoom*100.0),egui::FontId::proportional(13.0),Color32::WHITE).size().x;
    let option_width=ui.painter().layout_no_wrap("300%".into(),egui::TextStyle::Button.resolve(ui.style()),Color32::WHITE).size().x;
    let menu_width=(option_width+24.0).ceil();
    let mut anchor=response.clone();
    anchor.rect=egui::Rect::from_center_size(egui::pos2(text_right-ratio_width*0.5,badge.center().y),egui::vec2(menu_width,badge.height()));
    let mut selected=None;
    ui.scope(|ui|{
        let visuals=&mut ui.style_mut().visuals;
        visuals.window_fill=Color32::from_black_alpha(128);
        visuals.window_stroke=egui::Stroke::NONE;
        visuals.window_shadow=egui::epaint::Shadow::NONE;
        egui::popup::popup_above_or_below_widget(ui,id,&anchor,egui::AboveOrBelow::Above,egui::popup::PopupCloseBehavior::CloseOnClickOutside,|ui|{
            egui::ScrollArea::vertical().max_height(280.0).show(ui,|ui|{
                for percent in [300,200,150,125,100,75,50,25,10] {
                    let value=percent as f32/100.0;
                    if ui.selectable_label((zoom-value).abs()<0.0001,egui::RichText::new(format!("{percent}%")).color(Color32::WHITE)).clicked(){
                        selected=Some(value);ui.memory_mut(|memory|memory.close_popup());
                    }
                }
            });
        });
    });
    if selected.is_some(){ui.ctx().request_repaint();}
    selected
}
