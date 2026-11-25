use super::{Inline, Text, font_samples::Cache, tr};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Style {
    pub(super) family: Option<String>,
    pub(super) size: Option<u16>,
    pub(super) color: Option<[u8; 3]>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct Run {
    start: usize,
    end: usize,
    style: Style,
}

pub(super) fn get(runs: &[Run], index: usize) -> Option<&Style> {
    let at = runs.partition_point(|run| run.end <= index);
    runs.get(at)
        .filter(|run| run.start <= index)
        .map(|run| &run.style)
}

pub(super) fn valid(runs: &[Run], count: usize) -> bool {
    runs.len() <= count
        && runs
            .iter()
            .zip(std::iter::once(0).chain(runs.iter().map(|r| r.end)))
            .all(|(r, previous)| {
                previous <= r.start
                    && r.start < r.end
                    && r.end <= count
                    && r.style
                        .family
                        .as_ref()
                        .is_none_or(|s| !s.is_empty() && s.len() <= 512)
                    && r.style.size.is_none_or(|size| (1..=256).contains(&size))
            })
}

fn expand(runs: &[Run], count: usize) -> Vec<Style> {
    (0..count)
        .map(|i| get(runs, i).cloned().unwrap_or_default())
        .collect()
}

pub(super) fn compact(styles: Vec<Style>) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (index, style) in styles.into_iter().enumerate() {
        if style == Style::default() {
            continue;
        }
        if let Some(last) = runs.last_mut()
            && last.end == index
            && last.style == style
        {
            last.end += 1;
        } else {
            runs.push(Run {
                start: index,
                end: index + 1,
                style,
            });
        }
    }
    runs
}

pub(super) fn apply(runs: &mut Vec<Run>, count: usize, range: Range<usize>, change: Style) {
    let mut styles = expand(runs, count);
    for style in &mut styles[range] {
        if change.family.is_some() {
            style.family.clone_from(&change.family);
        }
        if change.size.is_some() {
            style.size = change.size;
        }
        if change.color.is_some() {
            style.color = change.color;
        }
    }
    *runs = compact(styles);
}

pub(super) fn edit(runs: &mut Vec<Run>, previous: &str, current: &str) {
    if runs.is_empty() {
        return;
    }
    let old: Vec<_> = previous.chars().collect();
    let new: Vec<_> = current.chars().collect();
    let start = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[start..]
        .iter()
        .rev()
        .zip(new[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let mut styles = expand(runs, old.len());
    let inherit = styles
        .get(start.saturating_sub(1))
        .cloned()
        .unwrap_or_default();
    styles.splice(
        start..old.len() - suffix,
        std::iter::repeat_n(inherit, new.len() - start - suffix),
    );
    *runs = compact(styles);
}

pub(super) fn uniform<T: PartialEq>(mut iter: impl Iterator<Item = T>) -> Option<T> {
    let first = iter.next()?;
    iter.all(|value| value == first).then_some(first)
}

pub(super) fn layer_style(text: &Text, content: &str, runs: &[Run]) -> Style {
    if content.is_empty() {
        return Style { family: Some(text.family.clone()), size: Some(text.size), color: Some(text.color) };
    }
    let count = content.chars().count();
    Style {
        family: uniform((0..count).map(|i| get(runs, i).and_then(|s| s.family.as_deref()).unwrap_or(&text.family))).map(str::to_owned),
        size: uniform((0..count).map(|i| get(runs, i).and_then(|s| s.size).unwrap_or(text.size))),
        color: uniform((0..count).map(|i| get(runs, i).and_then(|s| s.color).unwrap_or(text.color))),
    }
}

pub(super) fn apply_layer_style(text: &mut Text, change: Style) {
    let mut styles = expand(&text.character_runs, text.content.chars().count());
    for style in &mut styles {
        if change.family.is_some() { style.family = None; }
        if change.size.is_some() { style.size = None; }
        if change.color.is_some() { style.color = None; }
    }
    if let Some(family) = change.family { text.family = family; }
    if let Some(size) = change.size { text.size = size; }
    if let Some(color) = change.color { text.color = color; }
    text.character_runs = compact(styles);
}

pub(super) fn color_control(ui: &mut egui::Ui, color: Option<[u8; 3]>, fallback: [u8; 3]) -> Option<[u8; 3]> {
    let mut ink = color.unwrap_or(fallback);
    let changed = if color.is_some() {
        ui.color_edit_button_srgb(&mut ink).changed()
    } else {
        let label = egui::RichText::new("   ?   ")
            .strong().color(egui::Color32::RED).background_color(egui::Color32::WHITE);
        let draft_id = ui.id().with("mixed_text_color");
        let menu = ui.menu_button(label, |ui| {
            let width = (ui.ctx().screen_rect().width() - 24.).clamp(160., 220.);
            ui.set_width(width);
            ui.spacing_mut().slider_width = width;
            let mut draft = ui.ctx().data(|data| data.get_temp::<egui::Color32>(draft_id))
                .unwrap_or_else(|| egui::Color32::from_rgb(fallback[0], fallback[1], fallback[2]));
            egui::color_picker::color_picker_color32(ui, &mut draft, egui::color_picker::Alpha::Opaque);
            let mut apply = false;
            ui.add_space(4.);
            ui.vertical_centered(|ui| {
                let button = egui::vec2(76., 30.);
                let group_width = 2. * button.x + ui.spacing().item_spacing.x;
                ui.allocate_ui_with_layout(
                    egui::vec2(group_width, button.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if ui.add_sized(button, egui::Button::new(tr("取消"))).clicked() {
                            ui.close_menu();
                        }
                        if ui.add_sized(button, egui::Button::new(tr("确定"))).clicked() {
                            let [r, g, b, _] = draft.to_array();
                            ink = [r, g, b];
                            apply = true;
                            ui.close_menu();
                        }
                    },
                );
            });
            ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
            apply
        });
        if menu.inner.is_none() || menu.inner == Some(true) {
            ui.ctx().data_mut(|data| data.remove::<egui::Color32>(draft_id));
        }
        menu.inner.unwrap_or(false)
    };
    changed.then_some(ink)
}

pub(super) fn controls(
    ui: &mut egui::Ui,
    fonts: &mut Cache,
    edit: &mut Inline,
    range: Range<usize>,
) {
    let style_at = |index| get(&edit.runs, index);
    let mut family = uniform(range.clone().map(|index| {
        style_at(index)
            .and_then(|s| s.family.as_deref())
            .unwrap_or(&edit.original.family)
    }))
    .unwrap_or_default()
    .to_owned();
    let previous_family = family.clone();
    let mut size = uniform(range.clone().map(|index| {
        style_at(index)
            .and_then(|s| s.size)
            .unwrap_or(edit.original.size)
    }))
    .unwrap_or(0);
    let color = uniform(range.clone().map(|index| {
        style_at(index)
            .and_then(|s| s.color)
            .unwrap_or(edit.original.color)
    }));
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("字体"));
            fonts.picker(ui, &mut family);
            ui.label(tr("字号"));
            let size_changed = ui
                .add(
                    egui::DragValue::new(&mut size)
                        .range(1..=256)
                        .clamp_existing_to_range(false)
                        .custom_formatter(|value, _| {
                            if value == 0. {
                                String::new()
                            } else {
                                format!("{value:.0}")
                            }
                        }),
                )
                .changed()
                && size > 0;
            ui.label(tr("颜色"));
            let ink = color_control(ui, color, edit.original.color);
            let family_changed = family != previous_family;
            if family_changed || size_changed || ink.is_some() {
                apply(
                    &mut edit.runs,
                    edit.content.chars().count(),
                    range,
                    Style {
                        family: family_changed.then_some(family),
                        size: size_changed.then_some(size),
                        color: ink,
                    },
                );
                ui.ctx().request_repaint();
            }
        });
    });
}
