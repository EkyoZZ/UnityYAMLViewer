//! Input/output levels controls and adjustment slider artwork.
use super::*;
fn marker(ui: &egui::Ui, center: Pos2, color: Color32) {
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            center + egui::vec2(0., -4.),
            center + egui::vec2(-4., 4.),
            center + egui::vec2(4., 4.),
        ],
        color,
        egui::Stroke::new(1_f32, ui.visuals().text_color()),
    ));
}
pub(super) fn show(
    ui: &mut egui::Ui,
    black: &mut u8,
    gamma: &mut f32,
    white: &mut u8,
    output_black: &mut u8,
    output_white: &mut u8,
    histogram: Option<&[f32; 256]>,
) {
    ui.label("RGB");
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 115.), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0., ui.visuals().extreme_bg_color);
    ui.painter().rect_stroke(
        rect,
        0.,
        ui.visuals().widgets.inactive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    if let Some(bins) = histogram {
        let max = bins.iter().copied().fold(0_f32, f32::max);
        if max > 0. {
            for (i, value) in bins.iter().enumerate() {
                let h = (value / max).sqrt() * (rect.height() - 3.);
                let x = rect.left() + i as f32 / 256. * rect.width();
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        egui::pos2(x, rect.bottom() - h),
                        egui::pos2(x + rect.width() / 256. + 0.5, rect.bottom()),
                    ),
                    0.,
                    ui.visuals().text_color().gamma_multiply(0.7),
                );
            }
        }
    }
    let (track, response) =
        ui.allocate_exact_size(egui::vec2(rect.width(), 16.), egui::Sense::click_and_drag());
    let range = white.saturating_sub(*black).max(1) as f32;
    let xs = [
        *black as f32 / 255.,
        (*black as f32 + range * 0.5_f32.powf(*gamma)) / 255.,
        *white as f32 / 255.,
    ];
    let drag = ui.id().with("levels_input_drag");
    if response.drag_started() || response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let i = (0..3)
                .min_by(|a, b| {
                    ((track.left() + xs[*a] * track.width()) - p.x)
                        .abs()
                        .total_cmp(&((track.left() + xs[*b] * track.width()) - p.x).abs())
                })
                .unwrap();
            ui.ctx().data_mut(|d| d.insert_temp(drag, i));
        }
    }
    if response.dragged() || response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let value = ((pointer.x - track.left()) / track.width() * 255.).clamp(0., 255.);
            match ui.ctx().data(|d| d.get_temp::<usize>(drag)).unwrap_or(0) {
                0 => *black = (value.round() as u8).min(white.saturating_sub(1)),
                2 => *white = (value.round() as u8).max(black.saturating_add(1)),
                _ => {
                    let t = ((value - *black as f32) / range).clamp(0.001, 0.999);
                    *gamma = (t.ln() / 0.5_f32.ln()).clamp(0.1, 10.);
                }
            }
        }
    }
    for (i, x) in xs.iter().enumerate() {
        marker(
            ui,
            egui::pos2(track.left() + x * track.width(), track.center().y),
            [Color32::BLACK, Color32::GRAY, Color32::WHITE][i],
        );
    }
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(black).range(0..=white.saturating_sub(1)));
        ui.add_space((ui.available_width() - 110.).max(0.) / 2.);
        ui.add(
            egui::DragValue::new(gamma)
                .range(0.1..=10.)
                .speed(0.01)
                .fixed_decimals(2),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::DragValue::new(white).range(black.saturating_add(1)..=255));
        });
    });
    ui.add_space(6.);
    let (bar, response) =
        ui.allocate_exact_size(egui::vec2(rect.width(), 16.), egui::Sense::click_and_drag());
    for i in 0..128 {
        let gray = (i * 255 / 127) as u8;
        ui.painter().rect_filled(
            Rect::from_min_max(
                egui::pos2(bar.left() + i as f32 / 128. * bar.width(), bar.top()),
                egui::pos2(
                    bar.left() + (i + 1) as f32 / 128. * bar.width() + 0.5,
                    bar.bottom() - 5.,
                ),
            ),
            0.,
            Color32::from_gray(gray),
        );
    }
    let output_drag = ui.id().with("levels_output_drag");
    if response.drag_started() || response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let x = (p.x - bar.left()) / bar.width() * 255.;
            let left = (x - *output_black as f32).abs() <= (x - *output_white as f32).abs();
            ui.ctx().data_mut(|d| d.insert_temp(output_drag, left));
        }
    }
    if response.dragged() || response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let value = ((p.x - bar.left()) / bar.width() * 255.)
                .clamp(0., 255.)
                .round() as u8;
            if ui
                .ctx()
                .data(|d| d.get_temp::<bool>(output_drag))
                .unwrap_or(true)
            {
                *output_black = value;
            } else {
                *output_white = value;
            }
        }
    }
    marker(
        ui,
        egui::pos2(
            bar.left() + *output_black as f32 / 255. * bar.width(),
            bar.bottom() - 2.,
        ),
        Color32::BLACK,
    );
    marker(
        ui,
        egui::pos2(
            bar.left() + *output_white as f32 / 255. * bar.width(),
            bar.bottom() - 2.,
        ),
        Color32::WHITE,
    );
    ui.horizontal(|ui| {
        ui.label(tr("输出色阶"));
        ui.add(egui::DragValue::new(output_black).range(0..=255));
        ui.add(egui::DragValue::new(output_white).range(0..=255));
    });
}
pub(super) fn colored_slider(
    ui: &mut egui::Ui,
    value: &mut i16,
    range: std::ops::RangeInclusive<i16>,
    colors: &[Color32],
) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 20.),
        egui::Sense::click_and_drag(),
    );
    for i in 0..128 {
        let t = i as f32 / 127. * (colors.len() - 1) as f32;
        let index = (t.floor() as usize).min(colors.len() - 2);
        let k = t - index as f32;
        let a = colors[index];
        let b = colors[index + 1];
        let c = Color32::from_rgb(
            (a.r() as f32 * (1. - k) + b.r() as f32 * k).round() as u8,
            (a.g() as f32 * (1. - k) + b.g() as f32 * k).round() as u8,
            (a.b() as f32 * (1. - k) + b.b() as f32 * k).round() as u8,
        );
        ui.painter().rect_filled(
            Rect::from_min_max(
                egui::pos2(
                    rect.left() + i as f32 / 128. * rect.width(),
                    rect.top() + 3.,
                ),
                egui::pos2(
                    rect.left() + (i + 1) as f32 / 128. * rect.width() + 0.5,
                    rect.top() + 7.,
                ),
            ),
            0.,
            c,
        );
    }
    let min = *range.start() as f32;
    let max = *range.end() as f32;
    if response.clicked() || response.dragged() {
        if let Some(p) = response.interact_pointer_pos() {
            *value = (min + ((p.x - rect.left()) / rect.width()).clamp(0., 1.) * (max - min))
                .round() as i16;
        }
    }
    let t = ((*value as f32 - min) / (max - min)).clamp(0., 1.);
    marker(
        ui,
        egui::pos2(rect.left() + t * rect.width(), rect.top() + 13.),
        Color32::LIGHT_GRAY,
    );
}
