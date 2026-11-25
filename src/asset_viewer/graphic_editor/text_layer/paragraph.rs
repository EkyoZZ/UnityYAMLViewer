use super::*;
use ab_glyph::{Font, FontRef, ScaleFont};
use resvg::usvg::fontdb;
#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) enum Align {
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
    pub(super) align: Align,
    pub(super) width: u16,
    pub(super) left: i16,
    pub(super) right: i16,
    pub(super) first: i16,
    pub(super) before: u16,
    pub(super) after: u16,
}

pub(super) struct CaretLine {
    pub(super) start: usize,
    pub(super) columns: Vec<f32>,
    pub(super) baseline: f32,
    pub(super) ascent: f32,
    pub(super) descent: f32,
}
impl Paragraph {
    pub(super) fn editor_alignment(&self) -> egui::Align {
        match self.align {
            Align::Center | Align::JustifyCenter => egui::Align::Center,
            Align::Right | Align::JustifyRight => egui::Align::Max,
            _ => egui::Align::Min,
        }
    }

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
        ui.horizontal(|ui| {
            ui.label(tr("段落宽度"));
            ui.add(
                egui::DragValue::new(&mut self.width)
                    .range(0..=2048)
                    .suffix(" px"),
            )
            .on_hover_text(tr("0 表示按文字自然宽度排版"));
        });
        egui::Grid::new("paragraph_lengths")
            .num_columns(5)
            .spacing(egui::vec2(3., 4.))
            .show(ui, |ui| {
                for (label, icon, v) in [
                    ("左缩进", Metric::Left, &mut self.left),
                    ("右缩进", Metric::Right, &mut self.right),
                    ("首行缩进", Metric::First, &mut self.first),
                ] {
                    icon.show(ui, label);
                    ui.add_sized(
                        [54., 18.],
                        egui::DragValue::new(v).range(-256..=256).suffix(" px"),
                    )
                    .on_hover_text(tr(label));
                    if matches!(icon, Metric::Left) {
                        ui.allocate_space(egui::vec2(20., 0.));
                    } else {
                        ui.end_row();
                    }
                }
                for (label, icon, v) in [
                    ("段前间距", Metric::Before, &mut self.before),
                    ("段后间距", Metric::After, &mut self.after),
                ] {
                    icon.show(ui, label);
                    ui.add_sized(
                        [54., 18.],
                        egui::DragValue::new(v).range(0..=256).suffix(" px"),
                    )
                    .on_hover_text(tr(label));
                    if matches!(icon, Metric::Before) {
                        ui.allocate_space(egui::vec2(20., 0.));
                    }
                }
                ui.end_row();
            });
    }
    pub(super) fn write_lines(
        &self,
        t: &Text,
        db: &fontdb::Database,
        svg: &mut String,
    ) -> Vec<CaretLine> {
        let query = fontdb::Query {
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
        let fallback = [
            fontdb::Family::SansSerif,
            fontdb::Family::Name("Segoe UI"),
            fontdb::Family::Name("Microsoft YaHei UI"),
            fontdb::Family::Name("Microsoft YaHei"),
            fontdb::Family::Name("Segoe UI Symbol"),
        ]
        .into_iter()
        .filter_map(|family| {
            db.query(&fontdb::Query {
                families: &[family],
                ..query
            })
        })
        .collect::<Vec<_>>();
        let glyph_width = |face, c, size| {
            db.with_face_data(face, |data, index| {
                let font = FontRef::try_from_slice_and_index(data, index).ok()?;
                let font = font.as_scaled(font.pt_to_px_scale(f32::from(size) * 0.75)?);
                let glyph = font.glyph_id(c);
                (glyph.0 != 0).then(|| font.h_advance(glyph))
            })
            .flatten()
        };
        let mut glyphs = std::collections::HashMap::new();
        let mut resolved = std::collections::HashMap::new();
        for (index, (byte, c)) in t.content.char_indices().enumerate() {
            let style = super::character::get(&t.character_runs, index);
            let family = style.and_then(|s| s.family.as_deref()).unwrap_or(&t.family);
            let size = style.and_then(|s| s.size).unwrap_or(t.size);
            let color = style.and_then(|s| s.color).unwrap_or(t.color);
            let (advance, family) = *resolved.entry((family, size, c)).or_insert_with(|| {
                let ps_face = db.faces().find(|face| face.post_script_name == family);
                let family = ps_face
                    .and_then(|face| face.families.first())
                    .map_or(family, |(family, _)| family.as_str());
                let primary = ps_face.map(|face| face.id).or_else(|| {
                    db.query(&fontdb::Query {
                        families: &[fontdb::Family::Name(family)],
                        ..query
                    })
                });
                if let Some(advance) = primary.and_then(|id| glyph_width(id, c, size)) {
                    return (advance, family);
                }
                let substitute = fallback
                    .iter()
                    .copied()
                    .chain(db.faces().map(|face| face.id))
                    .filter(|id| Some(*id) != primary)
                    .find_map(|id| {
                        let name = db.face(id)?.families.first()?.0.as_str();
                        let id = db.query(&fontdb::Query {
                            families: &[fontdb::Family::Name(name)],
                            ..query
                        })?;
                        Some((glyph_width(id, c, size)?, name))
                    });
                substitute.unwrap_or((f32::from(size), family))
            });
            glyphs.insert(byte, (advance, family, size, color));
        }
        let measure = |byte: usize| {
            (glyphs.get(&byte).map_or(f32::from(t.size), |entry| entry.0) + t.tracking as f32)
                .max(0.)
        };
        let mut offset = 0;
        let natural = t
            .content
            .split('\n')
            .map(|line| {
                let width = line
                    .char_indices()
                    .map(|(i, _)| measure(offset + i))
                    .sum::<f32>();
                offset += line.len() + 1;
                width
            })
            .fold(1., f32::max);
        let width = if self.width == 0 {
            natural
        } else {
            self.width as f32
        };
        let right = width / 2. - self.right as f32;
        let first_left = -width / 2. + self.left as f32 + self.first as f32;
        let rest_left = -width / 2. + self.left as f32;
        let line_step = glyphs.values().map(|entry| entry.2).max().unwrap_or(t.size) as f32
            * t.line_height as f32
            / 100.;
        let mut lines = Vec::new();
        let mut y = 0.;
        let mut byte = 0;
        for p in t.content.split('\n') {
            let mut left = first_left;
            let mut from = 0;
            let mut taken = 0.;
            let mut last_space = None;
            y += self.before as f32;
            for (i, c) in p.char_indices() {
                if !t.point_text
                    && self.width > 0
                    && i > from
                    && taken + measure(byte + i) > (right - left).max(1.)
                {
                    let end = last_space.take().filter(|&v| v > from).unwrap_or(i);
                    lines.push((&p[from..end], byte + from, left, y, false));
                    y += line_step;
                    left = rest_left;
                    from = end;
                    taken = p[from..i]
                        .char_indices()
                        .map(|(i, _)| measure(byte + from + i))
                        .sum();
                }
                taken += measure(byte + i);
                if c.is_whitespace() {
                    last_space = Some(i + c.len_utf8());
                }
            }
            lines.push((&p[from..], byte + from, left, y, true));
            byte += p.len() + 1;
            y += line_step + self.after as f32;
        }
        // The paragraph box is centred on the watermark position, including indents.
        let baseline = if t.point_text {
            t.size as f32 * 0.8
        } else {
            t.size as f32 * 0.35 - (y - line_step) * 0.5
        };
        let mut carets = Vec::with_capacity(lines.len());
        let mut synthetic_bold = std::collections::HashMap::new();
        for (line, byte, left, y, last) in lines {
            let justify = (!t.point_text || t.content.contains('\n'))
                && (matches!(self.align, Align::JustifyAll)
                    || (!last
                        && matches!(
                            self.align,
                            Align::JustifyLeft | Align::JustifyCenter | Align::JustifyRight
                        )));
            let (anchor, x) = if t.point_text && !t.content.contains('\n') {
                ("start", left + width / 2.)
            } else if justify {
                ("start", left)
            } else {
                match self.align {
                    Align::Left | Align::JustifyLeft => ("start", left),
                    Align::Right | Align::JustifyRight => ("end", right),
                    _ => ("middle", (left + right) / 2.),
                }
            };
            let x = if t.point_text && t.content.contains('\n') {
                x + width / 2.
            } else {
                x
            };
            let y = y + baseline;
            let width = line
                .char_indices()
                .map(|(offset, _)| measure(byte + offset))
                .sum::<f32>();
            let column_scale = if justify && line.chars().count() > 1 && width > 0. {
                (right - left).max(1.) / width
            } else {
                1.
            };
            let mut x_cursor = match anchor {
                "end" => x - width,
                "middle" => x - width * 0.5,
                _ => x,
            };
            let mut columns = vec![x_cursor];
            let mut height = if line.is_empty() {
                f32::from(t.size)
            } else {
                0.
            };
            for (offset, _) in line.char_indices() {
                let at = byte + offset;
                x_cursor += measure(at) * column_scale;
                columns.push(x_cursor);
                height = height.max(
                    glyphs
                        .get(&at)
                        .map_or(f32::from(t.size), |entry| f32::from(entry.2)),
                );
            }
            carets.push(CaretLine {
                start: t.content[..byte].chars().count(),
                columns,
                baseline: y,
                ascent: height * 0.8,
                descent: height * 0.2,
            });
            write!(svg, r#"<text x="{x}" y="{y}" text-anchor="{anchor}""#).unwrap();
            if justify && line.chars().count() > 1 {
                write!(
                    svg,
                    r#" textLength="{}" lengthAdjust="spacing""#,
                    (right - left).max(1.)
                )
                .unwrap();
            }
            svg.push('>');
            let mut start = 0;
            let mut style = None;
            let mut write_run = |text: &str, style: Option<(&str, u16, [u8; 3])>| {
                let text = quick_xml::escape::escape(text);
                if let Some((family, size, [r, g, b])) = style {
                    let synthesize = t.bold
                        && *synthetic_bold.entry(family.to_owned()).or_insert_with(|| {
                            let families = [fontdb::Family::Name(family)];
                            let regular = db.query(&fontdb::Query {
                                families: &families,
                                weight: fontdb::Weight::NORMAL,
                                ..query
                            });
                            let bold = db.query(&fontdb::Query {
                                families: &families,
                                ..query
                            });
                            bold == regular
                                || bold
                                    .and_then(|id| db.face(id))
                                    .is_none_or(|face| face.weight < fontdb::Weight::SEMIBOLD)
                        });
                    let family = quick_xml::escape::escape(family);
                    write!(svg,r##"<tspan font-family="{family}" font-size="{size}" fill="#{r:02x}{g:02x}{b:02x}""##).unwrap();
                    if synthesize {
                        let weight = f32::from(size) * 0.045;
                        write!(svg, r##" stroke="#{r:02x}{g:02x}{b:02x}" stroke-width="{weight}" stroke-linejoin="round""##).unwrap();
                    }
                    write!(svg, ">{text}</tspan>").unwrap();
                } else {
                    write!(svg, "{text}").unwrap();
                }
            };
            for (offset, _) in line.char_indices() {
                let next = glyphs
                    .get(&(byte + offset))
                    .map(|&(_, family, size, color)| (family, size, color));
                if next != style {
                    if offset > start {
                        write_run(&line[start..offset], style);
                    }
                    start = offset;
                    style = next;
                }
            }
            write_run(&line[start..], style);
            svg.push_str("</text>");
        }
        carets
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
