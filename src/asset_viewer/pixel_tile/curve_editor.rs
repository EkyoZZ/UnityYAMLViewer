//! Editable cubic Bezier tone curves in normalized input/output coordinates.
use super::*;
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Node {
    pub point: [f32; 2],
    pub left: [f32; 2],
    pub right: [f32; 2],
}
pub(super) fn identity() -> [Vec<Node>; 4] {
    std::array::from_fn(|_| {
        vec![
            Node {
                point: [0., 0.],
                left: [0., 0.],
                right: [1. / 3., 1. / 3.],
            },
            Node {
                point: [1., 1.],
                left: [2. / 3., 2. / 3.],
                right: [1., 1.],
            },
        ]
    })
}
pub(super) fn valid(nodes: &[Node]) -> bool {
    (2..=32).contains(&nodes.len())
        && nodes[0].point[0] == 0.
        && nodes.last().unwrap().point[0] == 1.
        && nodes.iter().all(|n| {
            n.point
                .iter()
                .chain(&n.left)
                .chain(&n.right)
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        })
        && nodes
            .windows(2)
            .all(|p| p[1].point[0] - p[0].point[0] >= 1. / 255. - 1e-6)
}
fn cubic(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let u = 1. - t;
    u * u * u * a + 3. * u * u * t * b + 3. * u * t * t * c + t * t * t * d
}
pub(super) fn evaluate(nodes: &[Node], x: f32) -> f32 {
    if nodes.len() < 2 {
        return x;
    }
    let i = nodes
        .partition_point(|n| n.point[0] < x)
        .saturating_sub(1)
        .min(nodes.len() - 2);
    let a = &nodes[i];
    let b = &nodes[i + 1];
    let x1 = a.right[0].clamp(a.point[0], b.point[0]);
    let x2 = b.left[0].clamp(x1, b.point[0]);
    let mut lo = 0.;
    let mut hi = 1.;
    for _ in 0..18 {
        let t = (lo + hi) * 0.5;
        if cubic(a.point[0], x1, x2, b.point[0], t) < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    cubic(
        a.point[1],
        a.right[1],
        b.left[1],
        b.point[1],
        (lo + hi) * 0.5,
    )
    .clamp(0., 1.)
}
pub(super) fn lookup(channels: &[Vec<Node>; 4]) -> [[u8; 256]; 3] {
    std::array::from_fn(|c| {
        std::array::from_fn(|v| {
            (evaluate(&channels[c + 1], evaluate(&channels[0], v as f32 / 255.)) * 255.).round()
                as u8
        })
    })
}
pub(super) fn from_legacy(values: [[u8; 5]; 4]) -> [Vec<Node>; 4] {
    std::array::from_fn(|c| {
        let xs = [0., 64. / 255., 128. / 255., 192. / 255., 1.];
        let mut nodes: Vec<_> = xs
            .iter()
            .enumerate()
            .map(|(i, x)| Node {
                point: [*x, values[c][i] as f32 / 255.],
                left: [*x, values[c][i] as f32 / 255.],
                right: [*x, values[c][i] as f32 / 255.],
            })
            .collect();
        for i in 0..4 {
            let a = nodes[i].point;
            let b = nodes[i + 1].point;
            nodes[i].right = [a[0] + (b[0] - a[0]) / 3., a[1] + (b[1] - a[1]) / 3.];
            nodes[i + 1].left = [b[0] - (b[0] - a[0]) / 3., b[1] - (b[1] - a[1]) / 3.];
        }
        nodes
    })
}
fn smooth(nodes: &mut [Node]) {
    let points: Vec<_> = nodes.iter().map(|n| n.point).collect();
    for (i, n) in nodes.iter_mut().enumerate() {
        let a = points[i.saturating_sub(1)];
        let b = points[(i + 1).min(points.len() - 1)];
        let slope = (b[1] - a[1]) / (b[0] - a[0]).max(1e-6);
        let left = (n.point[0] - a[0]) / 3.;
        let right = (b[0] - n.point[0]) / 3.;
        n.left = [n.point[0] - left, (n.point[1] - slope * left).clamp(0., 1.)];
        n.right = [
            n.point[0] + right,
            (n.point[1] + slope * right).clamp(0., 1.),
        ];
    }
}
fn insert(nodes: &mut Vec<Node>, point: [f32; 2]) -> Option<usize> {
    if nodes.len() >= 32
        || point[0] <= 0.
        || point[0] >= 1.
        || nodes
            .iter()
            .any(|n| (n.point[0] - point[0]).abs() < 1. / 255.)
    {
        return None;
    }
    let i = nodes.partition_point(|n| n.point[0] < point[0]);
    let a = nodes[i - 1].point;
    let b = nodes[i].point;
    let slope = (b[1] - a[1]) / (b[0] - a[0]).max(1e-6);
    let width = (point[0] - a[0]).min(b[0] - point[0]) / 3.;
    nodes.insert(
        i,
        Node {
            point,
            left: [point[0] - width, (point[1] - slope * width).clamp(0., 1.)],
            right: [point[0] + width, (point[1] + slope * width).clamp(0., 1.)],
        },
    );
    Some(i)
}
pub(super) fn show(ui: &mut egui::Ui, channels: &mut [Vec<Node>; 4]) {
    let base = ui.id().with("bezier_curve");
    let mut channel = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(base))
        .unwrap_or(0)
        .min(3);
    egui::ComboBox::from_id_salt(base)
        .selected_text(["RGB", "R", "G", "B"][channel])
        .show_ui(ui, |ui| {
            for (i, label) in ["RGB", "R", "G", "B"].iter().enumerate() {
                ui.selectable_value(&mut channel, i, *label);
            }
        });
    ui.ctx().data_mut(|d| d.insert_temp(base, channel));
    let nodes = &mut channels[channel];
    let id = base.with(channel);
    let mut selected = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(id))
        .unwrap_or(0)
        .min(nodes.len() - 1);
    let drag_id = id.with("drag");
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width()
                .clamp(160., super::layer_properties::DETAIL_WIDTH),
            180.,
        ),
        egui::Sense::click_and_drag(),
    );
    let to_screen = |p: [f32; 2]| {
        egui::pos2(
            rect.left() + p[0] * rect.width(),
            rect.bottom() - p[1] * rect.height(),
        )
    };
    let to_value = |p: Pos2| {
        [
            ((p.x - rect.left()) / rect.width()).clamp(0., 1.),
            ((rect.bottom() - p.y) / rect.height()).clamp(0., 1.),
        ]
    };
    let pick = |pointer: Pos2, nodes: &[Node], selected: usize| -> Option<(usize, u8)> {
        let n = &nodes[selected];
        for (kind, point) in [(1, n.left), (2, n.right)] {
            if to_screen(point).distance(pointer) <= 7. && point != n.point {
                return Some((selected, kind));
            }
        }
        nodes
            .iter()
            .enumerate()
            .find(|(_, n)| to_screen(n.point).distance(pointer) <= 7.)
            .map(|(i, _)| (i, 0))
    };
    if response.drag_started() {
        if let Some(pointer) = ui.input(|i| i.pointer.press_origin()) {
            let target = pick(pointer, nodes, selected)
                .or_else(|| insert(nodes, to_value(pointer)).map(|i| (i, 0)));
            if let Some(target) = target {
                selected = target.0;
                ui.ctx().data_mut(|d| d.insert_temp(drag_id, target));
            }
        }
    }
    if response.dragged() {
        if let (Some((i, kind)), Some(pointer)) = (
            ui.ctx().data(|d| d.get_temp::<(usize, u8)>(drag_id)),
            response.interact_pointer_pos(),
        ) {
            if i < nodes.len() {
                selected = i;
                let value = to_value(pointer);
                let prev = if i == 0 { 0. } else { nodes[i - 1].point[0] };
                let next = if i + 1 == nodes.len() {
                    1.
                } else {
                    nodes[i + 1].point[0]
                };
                if kind == 0 {
                    let old = nodes[i].point;
                    let x = if i == 0 {
                        0.
                    } else if i + 1 == nodes.len() {
                        1.
                    } else {
                        value[0].clamp(prev + 1. / 255., (next - 1. / 255.).max(prev + 1. / 255.))
                    };
                    let point = [x, value[1]];
                    let node = &mut nodes[i];
                    for handle in [&mut node.left, &mut node.right] {
                        for c in 0..2 {
                            handle[c] = (handle[c] + point[c] - old[c]).clamp(0., 1.);
                        }
                    }
                    node.point = point;
                    node.left[0] = node.left[0].clamp(prev, x);
                    node.right[0] = node.right[0].clamp(x, next);
                } else if kind == 1 {
                    nodes[i].left = [value[0].clamp(prev, nodes[i].point[0]), value[1]];
                } else {
                    nodes[i].right = [value[0].clamp(nodes[i].point[0], next), value[1]];
                }
            }
        }
    }
    if response.drag_stopped() {
        ui.ctx().data_mut(|d| d.remove_temp::<(usize, u8)>(drag_id));
    }
    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            if let Some(i) = nodes
                .iter()
                .position(|n| to_screen(n.point).distance(pointer) <= 7.)
            {
                if i > 0 && i + 1 < nodes.len() {
                    nodes.remove(i);
                    selected = selected.min(nodes.len() - 1);
                }
            }
        }
    } else if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            if let Some((i, _)) = pick(pointer, nodes, selected) {
                selected = i;
            } else if let Some(i) = insert(nodes, to_value(pointer)) {
                selected = i;
            }
        }
    }
    for i in 0..nodes.len() - 1 {
        let a = nodes[i].point[0];
        let b = nodes[i + 1].point[0];
        nodes[i].right[0] = nodes[i].right[0].clamp(a, b);
        nodes[i + 1].left[0] = nodes[i + 1].left[0].clamp(nodes[i].right[0], b);
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2., ui.visuals().extreme_bg_color);
    let grid = egui::Stroke::new(1_f32, ui.visuals().widgets.noninteractive.bg_stroke.color);
    for i in 1..4 {
        let t = i as f32 / 4.;
        painter.line_segment([to_screen([t, 0.]), to_screen([t, 1.])], grid);
        painter.line_segment([to_screen([0., t]), to_screen([1., t])], grid);
    }
    painter.line_segment([to_screen([0., 0.]), to_screen([1., 1.])], grid);
    let line = (0..=255)
        .map(|x| {
            let x = x as f32 / 255.;
            to_screen([x, evaluate(nodes, x)])
        })
        .collect();
    painter.add(egui::Shape::line(
        line,
        egui::Stroke::new(1.5_f32, ui.visuals().text_color()),
    ));
    let n = &nodes[selected];
    let handle_color = ui.visuals().selection.stroke.color;
    for point in [n.left, n.right] {
        if point != n.point {
            painter.line_segment(
                [to_screen(n.point), to_screen(point)],
                egui::Stroke::new(1_f32, handle_color),
            );
            painter.circle_stroke(to_screen(point), 3., egui::Stroke::new(1_f32, handle_color));
        }
    }
    for (i, n) in nodes.iter().enumerate() {
        painter.circle_filled(
            to_screen(n.point),
            if i == selected { 4. } else { 3. },
            if i == selected {
                handle_color
            } else {
                ui.visuals().text_color()
            },
        );
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, selected));
    let mut reset = false;
    ui.horizontal(|ui| {
        if ui.button(tr("平滑曲线")).clicked() {
            smooth(nodes);
        }
        if ui.button(tr("重置曲线")).clicked() {
            reset = true;
            ui.ctx().data_mut(|d| d.remove_temp::<usize>(id));
        }
    });
    if reset {
        *channels = identity();
    }
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr("点击添加控制点，拖动圆形手柄，双击内部点删除。")).small(),
        )
        .wrap(),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_lookup_and_alpha_preservation() {
        let channels = identity();
        assert!(channels.iter().all(|c| valid(c)));
        for table in lookup(&channels) {
            for (i, v) in table.iter().enumerate() {
                assert_eq!(*v, i as u8);
            }
        }
    }
    #[test]
    fn bezier_handle_changes_tone_and_roundtrips() {
        let mut channels = identity();
        channels[0][0].right = [0.2, 0.8];
        let tables = lookup(&channels);
        assert!(tables[0][64] > 64);
        let restored: [Vec<Node>; 4] =
            serde_json::from_slice(&serde_json::to_vec(&channels).unwrap()).unwrap();
        assert!(channels == restored);
        assert_eq!(lookup(&channels), lookup(&restored));
    }
    #[test]
    fn rejects_duplicate_x_and_nan() {
        let mut c = identity()[0].clone();
        c[1].point[0] = 0.;
        assert!(!valid(&c));
        c = identity()[0].clone();
        c[0].right[1] = f32::NAN;
        assert!(!valid(&c));
    }
    #[test]
    fn legacy_conversion_preserves_linear_segments() {
        let values = [[0, 40, 90, 210, 255]; 4];
        let channels = from_legacy(values);
        assert!(channels.iter().all(|c| valid(c)));
        assert!((evaluate(&channels[0], 96. / 255.) * 255. - 65.).abs() < 0.01);
    }
}
