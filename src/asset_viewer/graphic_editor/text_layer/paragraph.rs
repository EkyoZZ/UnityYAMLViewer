use super::*;
use ab_glyph::{Font, FontRef, ScaleFont};
use resvg::usvg::fontdb;
#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
enum Align {
    Left,
    #[default]
    Center,
    Right,
    JustifyLeft,
    JustifyCenter,
    JustifyRight,
    JustifyAll,
}
impl Align {
    const ALL: [Self; 7] = [
        Self::Left,
        Self::Center,
        Self::Right,
        Self::JustifyLeft,
        Self::JustifyCenter,
        Self::JustifyRight,
        Self::JustifyAll,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Left => "左对齐",
            Self::Center => "居中对齐",
            Self::Right => "右对齐",
            Self::JustifyLeft => "两端对齐，末行居左",
            Self::JustifyCenter => "两端对齐，末行居中",
            Self::JustifyRight => "两端对齐，末行居右",
            Self::JustifyAll => "全部两端对齐",
        }
    }
}
#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct Paragraph {
    align: Align,
    width: u16,
    left: i16,
    right: i16,
    first: i16,
    before: u16,
    after: u16,
}
impl Paragraph {
    pub(super) fn valid(&self) -> bool {
        self.width <= 2048
            && [self.left, self.right, self.first]
                .iter()
                .all(|v| (-256..=256).contains(v))
            && self.before <= 256
            && self.after <= 256
    }
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for align in Align::ALL {
                let label = tr(align.label());
                let button = ui
                    .add_sized(
                        [26., 24.],
                        egui::Button::new("").selected(self.align == align),
                    )
                    .on_hover_text(label);
                button.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        button.enabled(),
                        self.align == align,
                        label,
                    )
                });
                let stroke = ui.style().interact(&button).fg_stroke;
                for row in 0..4 {
                    let center = button.rect.center();
                    let full = row % 2 == 0
                        || (row < 3
                            && matches!(
                                align,
                                Align::JustifyLeft | Align::JustifyCenter | Align::JustifyRight
                            ))
                        || align == Align::JustifyAll;
                    let (left, right) = if full {
                        (-7., 7.)
                    } else {
                        match align {
                            Align::Left | Align::JustifyLeft => (-7., 2.),
                            Align::Right | Align::JustifyRight => (-2., 7.),
                            _ => (-4.5, 4.5),
                        }
                    };
                    let y = row as f32 * 3. - 4.5;
                    ui.painter().line_segment(
                        [center + egui::vec2(left, y), center + egui::vec2(right, y)],
                        stroke,
                    );
                }
                if button.clicked() {
                    self.align = align;
                }
            }
        });
        egui::Grid::new("paragraph_lengths").show(ui, |ui| {
            ui.label(tr("段落宽度"));
            ui.add(
                egui::DragValue::new(&mut self.width)
                    .range(0..=2048)
                    .suffix(" px"),
            )
            .on_hover_text(tr("0 表示按文字自然宽度排版"));
            ui.end_row();
            for (label, icon, v) in [
                ("左缩进", Metric::Left, &mut self.left),
                ("右缩进", Metric::Right, &mut self.right),
                ("首行缩进", Metric::First, &mut self.first),
            ] {
                icon.show(ui, label);
                ui.add(egui::DragValue::new(v).range(-256..=256).suffix(" px"))
                    .on_hover_text(tr(label));
                if !matches!(icon, Metric::Left) {
                    ui.end_row();
                }
            }
            for (label, icon, v) in [
                ("段前间距", Metric::Before, &mut self.before),
                ("段后间距", Metric::After, &mut self.after),
            ] {
                icon.show(ui, label);
                ui.add(egui::DragValue::new(v).range(0..=256).suffix(" px"))
                    .on_hover_text(tr(label));
            }
            ui.end_row();
        });
    }
    pub(super) fn write_lines(&self, t: &Text, db: &fontdb::Database, svg: &mut String) {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(&t.family)],
            weight: if t.bold {
                fontdb::Weight::BOLD
            } else {
                fontdb::Weight::NORMAL
            },
            style: if t.italic {
                fontdb::Style::Italic
            } else {
                fontdb::Style::Normal
            },
            ..Default::default()
        };
        let widths = db
            .query(&query)
            .and_then(|id| {
                db.with_face_data(id, |data, index| {
                    let face = FontRef::try_from_slice_and_index(data, index).ok()?;
                    let scaled = face.as_scaled(face.pt_to_px_scale(t.size as f32 * 0.75)?);
                    Some(
                        t.content
                            .chars()
                            .map(|c| {
                                let id = scaled.glyph_id(c);
                                (
                                    c,
                                    if id.0 != 0 {
                                        scaled.h_advance(id)
                                    } else {
                                        t.size as f32
                                    },
                                )
                            })
                            .collect::<std::collections::HashMap<_, _>>(),
                    )
                })
                .flatten()
            })
            .unwrap_or_default();
        let measure = |c: char| {
            (widths.get(&c).copied().unwrap_or(t.size as f32) + t.tracking as f32).max(0.)
        };
        let natural = t
            .content
            .split('\n')
            .map(|p| p.chars().map(measure).sum::<f32>())
            .fold(1., f32::max);
        let width = if self.width == 0 {
            natural
        } else {
            self.width as f32
        };
        let right = width / 2. - self.right as f32;
        let first_left = -width / 2. + self.left as f32 + self.first as f32;
        let rest_left = -width / 2. + self.left as f32;
        let line_step = t.size as f32 * t.line_height as f32 / 100.;
        let mut lines = Vec::new();
        let mut y = 0.;
        for p in t.content.split('\n') {
            let mut left = first_left;
            let mut from = 0;
            let mut taken = 0.;
            let mut last_space = None;
            y += self.before as f32;
            for (i, c) in p.char_indices() {
                if self.width > 0 && i > from && taken + measure(c) > (right - left).max(1.) {
                    let end = last_space.take().filter(|&v| v > from).unwrap_or(i);
                    lines.push((&p[from..end], left, y, false));
                    y += line_step;
                    left = rest_left;
                    from = end;
                    taken = p[from..i].chars().map(measure).sum();
                }
                taken += measure(c);
                if c.is_whitespace() {
                    last_space = Some(i + c.len_utf8());
                }
            }
            lines.push((&p[from..], left, y, true));
            y += line_step + self.after as f32;
        }
        // The paragraph box is centred on the watermark position, including indents.
        let baseline = t.size as f32 * 0.35 - (y - line_step) * 0.5;
        for (line, left, y, last) in lines {
            let justify = matches!(self.align, Align::JustifyAll)
                || (!last
                    && matches!(
                        self.align,
                        Align::JustifyLeft | Align::JustifyCenter | Align::JustifyRight
                    ));
            let (anchor, x) = if justify {
                ("start", left)
            } else {
                match self.align {
                    Align::Left | Align::JustifyLeft => ("start", left),
                    Align::Right | Align::JustifyRight => ("end", right),
                    _ => ("middle", (left + right) / 2.),
                }
            };
            let y = y + baseline;
            write!(svg, r#"<text x="{x}" y="{y}" text-anchor="{anchor}""#).unwrap();
            if justify && line.chars().count() > 1 {
                write!(
                    svg,
                    r#" textLength="{}" lengthAdjust="spacing""#,
                    (right - left).max(1.)
                )
                .unwrap();
            }
            write!(svg, ">{}</text>", quick_xml::escape::escape(line)).unwrap();
        }
    }
}

#[derive(Clone, Copy)]
enum Metric {
    Left,
    Right,
    First,
    Before,
    After,
}

impl Metric {
    fn show(self, ui: &mut egui::Ui, label: &str) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(20., 19.), egui::Sense::hover());
        response.on_hover_text(tr(label));
        let center = rect.center();
        let stroke = egui::Stroke::new(1_f32, ui.visuals().text_color());
        let (line_x, arrow_x, arrow_y, vector) = match self {
            Self::Left => (3., -9., 0., egui::vec2(7., 0.)),
            Self::Right => (-3., 9., 0., egui::vec2(-7., 0.)),
            Self::First => (3., -9., -4., egui::vec2(7., 0.)),
            Self::Before => (4., -6., -6., egui::vec2(0., 7.)),
            Self::After => (4., -6., 6., egui::vec2(0., -7.)),
        };
        for y in [-4., 0., 4.] {
            ui.painter().line_segment(
                [
                    center + egui::vec2(line_x - 4., y),
                    center + egui::vec2(line_x + 4., y),
                ],
                stroke,
            );
        }
        ui.painter()
            .arrow(center + egui::vec2(arrow_x, arrow_y), vector, stroke);
    }
}
