use super::*;
use crate::localization::tr;
use eframe::egui::{self, Ui};
use std::ops::RangeInclusive;

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Bevel => "斜面和浮雕",
            Self::Stroke => "描边",
            Self::InnerShadow => "内阴影",
            Self::InnerGlow => "内发光",
            Self::Overlay => "颜色叠加",
            Self::Gradient => "渐变叠加",
            Self::OuterGlow => "外发光",
            Self::Shadow => "投影",
        }
    }
}
fn slider<T: egui::emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    v: &mut T,
    range: RangeInclusive<T>,
    unit: &str,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(tr(label));
        ui.add(egui::Slider::new(v, range).suffix(unit)).changed()
    })
    .inner
}
fn percent_ui(ui: &mut Ui, label: &str, v: &mut u8) {
    slider(ui, label, v, 0..=100, "%");
}
fn pick<T: Copy + PartialEq>(ui: &mut Ui, label: &str, v: &mut T, options: &[(T, &str)]) {
    ui.horizontal(|ui| {
        ui.label(tr(label));
        let selected = options.iter().find(|(k, _)| k == v).map_or("", |(_, s)| *s);
        egui::ComboBox::from_id_salt(label)
            .selected_text(tr(selected))
            .show_ui(ui, |ui| {
                for &(key, text) in options {
                    ui.selectable_value(v, key, tr(text));
                }
            });
    });
}
fn mode(ui: &mut Ui, label: &str, v: &mut Blend) {
    let choices = Blend::ALL.map(|m| (m, m.label()));
    pick(ui, label, v, &choices);
}
fn color(ui: &mut Ui, label: &str, rgb: &mut [u8; 3]) {
    ui.horizontal(|ui| {
        ui.label(tr(label));
        crate::color_property::edit_rgb(ui, tr(label), rgb);
    });
}
fn contour(ui: &mut Ui, v: &mut Contour, aa: &mut bool) {
    pick(
        ui,
        "等高线",
        v,
        &[
            (Contour::Linear, "线性"),
            (Contour::Smooth, "平滑"),
            (Contour::Cone, "锥形"),
            (Contour::Ring, "环形"),
        ],
    );
    ui.checkbox(aa, tr("消除锯齿"));
}
fn quality(ui: &mut Ui, q: &mut Quality) {
    ui.separator();
    ui.strong(tr("品质"));
    contour(ui, &mut q.contour, &mut q.antialias);
    percent_ui(ui, "杂色", &mut q.noise);
}
fn gradient_colors(ui: &mut Ui, g: &mut Gradient) {
    let mut stops: Vec<_> = g.color_stops().collect();
    crate::gradient_property::edit_rgb_stops(ui, &mut stops, g.reverse, |position| g.sample(position));
    g.set_color_stops(&stops);
}

fn gradient(ui: &mut Ui, g: &mut Gradient) {
    gradient_colors(ui, g);
    pick(
        ui,
        "渐变样式",
        &mut g.style,
        &[
            (GradientStyle::Linear, "线性"),
            (GradientStyle::Radial, "径向"),
            (GradientStyle::Angle, "角度渐变"),
            (GradientStyle::Reflected, "对称渐变"),
            (GradientStyle::Diamond, "菱形"),
        ],
    );
    angle_ui(ui, &mut g.angle);
    ui.add_enabled_ui(g.style != GradientStyle::Angle, |ui| {
        slider(ui, "缩放", &mut g.scale, 1..=400, "%");
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut g.reverse, tr("反向"));
        ui.checkbox(&mut g.align_layer, tr("与图层对齐"));
        ui.checkbox(&mut g.dither, tr("仿色"));
    });
}
fn angle_ui(ui: &mut Ui, value: &mut i16) -> bool {
    ui.horizontal(|ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(36.), egui::Sense::click_and_drag());
        let center = rect.center();
        let old = *value;
        if (response.clicked() || response.dragged())
            && let Some(pos) = response.interact_pointer_pos()
        {
            let v = pos - center;
            if v.length_sq() > 1. {
                *value = (-v.y).atan2(v.x).to_degrees().round() as i16;
            }
        }
        let angle = f32::from(*value).to_radians();
        let stroke = ui.style().interact(&response).fg_stroke;
        ui.painter().circle_stroke(center, 15., stroke);
        ui.painter().line_segment(
            [center, center + egui::vec2(angle.cos(), -angle.sin()) * 14.],
            stroke,
        );
        slider(ui, "角度", value, -180..=180, "°") || *value != old
    })
    .inner
}
fn shadow(
    ui: &mut Ui,
    inner: bool,
    shape: &mut ShadowShape,
    size: &mut u8,
    legacy: [i16; 2],
    light: &mut Light,
) {
    ui.checkbox(&mut shape.global, tr("使用全局光"));
    let (mut angle, mut distance) = shape.polar(legacy);
    let angle_changed = angle_ui(
        ui,
        if shape.global {
            &mut light.angle
        } else {
            &mut angle
        },
    );
    let moved = slider(ui, "距离", &mut distance, 0..=128, " px");
    if (angle_changed && !shape.global) || moved {
        shape.direction = Some((angle, distance));
    }
    percent_ui(ui, if inner { "阻塞" } else { "扩展" }, &mut shape.spread);
    slider(ui, "大小", size, 0..=64, " px");
    quality(ui, &mut shape.quality);
}
fn glow(ui: &mut Ui, g: &mut Glow, inner: bool) {
    mode(ui, "混合模式", &mut g.ink.mode);
    percent_ui(ui, "不透明度", &mut g.ink.opacity);
    pick(
        ui,
        "填充类型",
        &mut g.gradient,
        &[(false, "颜色"), (true, "渐变")],
    );
    if g.gradient {
        let mut ramp = g.color_ramp();
        gradient_colors(ui, &mut ramp);
        [g.ink.color, g.end_color] = ramp.colors;
        g.stops = ramp.stops;
    } else {
        color(ui, "颜色", &mut g.ink.color);
    }
    ui.separator();
    pick(
        ui,
        "方法",
        &mut g.precise,
        &[(false, "柔和"), (true, "精确")],
    );
    if inner {
        pick(
            ui,
            "光源位置",
            &mut g.center,
            &[(false, "边缘"), (true, "居中")],
        );
    }
    percent_ui(ui, if inner { "阻塞" } else { "扩展" }, &mut g.spread);
    slider(ui, "大小", &mut g.size, 1..=64, " px");
    quality(ui, &mut g.quality);
    slider(ui, "范围", &mut g.range, 1..=100, "%");
    ui.add_enabled_ui(g.gradient, |ui| percent_ui(ui, "抖动", &mut g.jitter));
}
fn bevel(ui: &mut Ui, b: &mut Bevel, global: &mut Light) {
    pick(
        ui,
        "斜面样式",
        &mut b.style,
        &[
            (Relief::Inner, "内斜面"),
            (Relief::Outer, "外斜面"),
            (Relief::Emboss, "浮雕"),
            (Relief::Pillow, "枕状浮雕"),
        ],
    );
    pick(
        ui,
        "方法",
        &mut b.technique,
        &[
            (Carve::Smooth, "平滑"),
            (Carve::Hard, "雕刻清晰"),
            (Carve::Soft, "雕刻柔和"),
        ],
    );
    slider(ui, "深度", &mut b.depth, 1..=1000, "%");
    pick(ui, "方向", &mut b.down, &[(false, "向上"), (true, "向下")]);
    slider(ui, "大小", &mut b.size, 1..=64, " px");
    slider(ui, "软化", &mut b.soften, 0..=16, " px");
    ui.separator();
    ui.strong(tr("光照"));
    ui.checkbox(&mut b.global, tr("使用全局光"));
    let l = if b.global { global } else { &mut b.light };
    angle_ui(ui, &mut l.angle);
    slider(ui, "光照高度", &mut l.altitude, 0..=90, "°");
    contour(ui, &mut b.contour, &mut b.antialias);
    for (label, ink) in [("高光模式", &mut b.highlight), ("阴影模式", &mut b.shade)] {
        ui.push_id(label, |ui| {
            mode(ui, label, &mut ink.mode);
            color(ui, "颜色", &mut ink.color);
            percent_ui(ui, "不透明度", &mut ink.opacity);
        });
    }
}
fn controls(ui: &mut Ui, entry: EntryMut<'_>, light: &mut Light) {
    match entry {
        EntryMut::Overlay(s) => {
            mode(ui, "混合模式", &mut s.mode);
            color(ui, "颜色", &mut s.color);
            percent_ui(ui, "不透明度", &mut s.opacity);
        }
        EntryMut::Gradient(s) => {
            mode(ui, "混合模式", &mut s.mode);
            percent_ui(ui, "不透明度", &mut s.opacity);
            gradient(ui, &mut s.ramp);
        }
        EntryMut::Stroke(s) => {
            slider(ui, "大小", &mut s.size, 1..=64, " px");
            pick(
                ui,
                "位置",
                &mut s.position,
                &[
                    (Position::Outside, "外部"),
                    (Position::Inside, "内部"),
                    (Position::Center, "居中"),
                ],
            );
            mode(ui, "混合模式", &mut s.mode);
            percent_ui(ui, "不透明度", &mut s.opacity);
            pick(
                ui,
                "填充类型",
                &mut s.gradient,
                &[(false, "颜色"), (true, "渐变")],
            );
            if s.gradient {
                gradient(ui, &mut s.ramp);
            } else {
                color(ui, "颜色", &mut s.color);
            }
        }
        EntryMut::Shadow(s) => {
            mode(ui, "混合模式", &mut s.mode);
            color(ui, "颜色", &mut s.color);
            percent_ui(ui, "不透明度", &mut s.opacity);
            shadow(ui, false, &mut s.shape, &mut s.radius, s.offset, light);
            ui.checkbox(&mut s.knockout, tr("图层挖空投影"));
        }
        EntryMut::InnerShadow(s) => {
            mode(ui, "混合模式", &mut s.ink.mode);
            color(ui, "颜色", &mut s.ink.color);
            percent_ui(ui, "不透明度", &mut s.ink.opacity);
            shadow(ui, true, &mut s.shape, &mut s.size, [0, 0], light);
        }
        EntryMut::InnerGlow(s) => glow(ui, s, true),
        EntryMut::OuterGlow(s) => glow(ui, s, false),
        EntryMut::Bevel(s) => bevel(ui, s, light),
    }
}
impl Page {
    fn label(self) -> String {
        let name = tr(self.kind.label());
        if self.index == 0 {
            name.to_owned()
        } else {
            format!("{name} {}", self.index + 1)
        }
    }
}
fn parameter_page(ui: &mut Ui, fx: &mut Effects, page: Page, light: &mut Light, editable: bool) {
    ui.push_id(page, |ui| {
        ui.add_enabled_ui(editable && fx.enabled(page), |ui| {
            controls(ui, fx.edit(page), light);
            ui.separator();
            if ui.button(tr("重置当前效果")).clicked() {
                fx.edit(page).reset();
            }
        });
    });
}

fn instance_button(ui: &mut Ui, fx: &Effects, row: Page, editable: bool) -> bool {
    if !row.kind.repeatable() {
        return false;
    }
    let add = row.index == 0;
    let full = add && fx.count(row.kind) >= MAX_INSTANCES;
    let tip = if full {
        "每种效果最多 10 项"
    } else if add {
        "添加同类效果"
    } else {
        "移除此效果"
    };
    let response = ui
        .add_enabled_ui(editable && !full, |ui| {
            ui.add_sized([22., 20.], egui::Button::new("").frame(false))
        })
        .inner;
    let rect = egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(16.));
    let color = ui.style().interact(&response).fg_stroke.color;
    if add {
        crate::icons::paint(ui, crate::icons::Icon::AddLayer, rect, color);
    } else {
        crate::icons::paint_layer_button(ui, rect, color, false);
    }
    response
        .on_hover_text(tr(tip))
        .on_disabled_hover_text(tr(tip))
        .clicked()
}
fn apply_instance_action(fx: &mut Effects, row: Page, selected: &mut Page) {
    if row.index == 0 {
        if let Some(added) = fx.add(row.kind) {
            *selected = added;
        }
    } else {
        fx.remove(row, selected);
    }
}
fn effect_list(ui: &mut Ui, fx: &mut Effects, selected: &mut Page, editable: bool) {
    let mut enabled = !fx.master_disabled;
    if ui
        .add_enabled(editable, egui::Checkbox::new(&mut enabled, tr("图层样式")))
        .changed()
    {
        fx.master_disabled = !enabled;
    }
    let mut action = None;
    for kind in Kind::ALL {
        for index in 0..fx.count(kind) {
            let row = Page { kind, index };
            ui.push_id(row, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.;
                    if ui
                        .add_enabled(
                            editable,
                            egui::Checkbox::without_text(fx.edit(row).switch()),
                        )
                        .on_hover_text(tr(kind.label()))
                        .changed()
                    {
                        *selected = row;
                    }
                    let label = row.label();
                    let name_width = (ui.available_width() - 26.).max(20.);
                    if ui
                        .add_sized(
                            [name_width, 20.],
                            egui::Button::new(&label)
                                .selected(*selected == row)
                                .frame(false)
                                .truncate(),
                        )
                        .on_hover_text(&label)
                        .clicked()
                    {
                        *selected = row;
                    }
                    if instance_button(ui, fx, row, editable) {
                        action = Some(row);
                    }
                })
            });
        }
    }
    if let Some(row) = action {
        apply_instance_action(fx, row, selected);
    }
}
pub(in super::super::super) fn detail_ui(
    ui: &mut Ui,
    fx: &mut Effects,
    page: &mut Page,
    light: &mut Light,
    editable: bool,
    periodic: bool,
) {
    // Undo, paste or clear can remove the previously selected instance.
    page.index = page.index.min(fx.count(page.kind) - 1);
    let max_height = (ui.available_height().min(ui.ctx().screen_rect().height()) - 32.).max(80.);
    if ui.available_width() >= 455. {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(172.);
                egui::ScrollArea::vertical()
                    .id_salt("fx_list")
                    .max_height(max_height)
                    .show(ui, |ui| effect_list(ui, fx, page, editable));
            });
            ui.add(egui::Separator::default().vertical().spacing(8.));
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.strong(page.label());
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt(("fx_controls", *page))
                    .max_height((max_height - 28.).max(72.))
                    .show(ui, |ui| parameter_page(ui, fx, *page, light, editable));
            });
        });
    } else {
        egui::ComboBox::from_id_salt("fx_page")
            .selected_text(page.label())
            .show_ui(ui, |ui| {
                for row in fx.pages() {
                    ui.selectable_value(page, row, row.label());
                }
            });
        ui.horizontal(|ui| {
            ui.add_enabled(
                editable,
                egui::Checkbox::new(fx.edit(*page).switch(), tr("启用")),
            );
            if instance_button(ui, fx, *page, editable) {
                apply_instance_action(fx, *page, page);
            }
        });
        egui::ScrollArea::vertical()
            .id_salt(("fx_controls", *page))
            .max_height((max_height - 58.).max(70.))
            .show(ui, |ui| parameter_page(ui, fx, *page, light, editable));
    }
    if !periodic {
        ui.separator();
        ui.small(tr("效果按单格计算，边缘会裁切。"));
    }
}
