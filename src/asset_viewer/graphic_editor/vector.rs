//! Editable image-space paths and parameterized rectangles.
use super::*;
use resvg::tiny_skia as sk;
use serde::{Deserialize, Serialize};
pub(super) mod psd;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct Knot {
    pub(super) point: [f32; 2],
    pub(super) incoming: [f32; 2],
    pub(super) outgoing: [f32; 2],
    pub(super) linked: bool,
}

impl Knot {
    fn corner(point: [f32; 2]) -> Self {
        Self {
            point,
            incoming: point,
            outgoing: point,
            linked: false,
        }
    }
}

fn rounded_polygon(points: &[[f32; 2]], radius: f32) -> Vec<Knot> {
    let vertices: Vec<_> = points.iter().map(|&[x, y]| egui::vec2(x, y)).collect();
    let count = vertices.len();
    let corners: Vec<_> = (0..count)
        .map(|i| {
            let previous = (vertices[(i + count - 1) % count] - vertices[i]).normalized();
            let next = (vertices[(i + 1) % count] - vertices[i]).normalized();
            let angle = previous.dot(next).clamp(-1., 1.).acos();
            (previous, next, angle)
        })
        .collect();
    if radius == 0.
        || corners
            .iter()
            .any(|(_, _, angle)| *angle < 0.001 || *angle >= std::f32::consts::PI - 0.001)
    {
        return points.iter().copied().map(Knot::corner).collect();
    }
    let tangents: Vec<_> = corners
        .iter()
        .map(|(_, _, angle)| radius / (angle * 0.5).tan())
        .collect();
    let scale = (0..count)
        .map(|i| {
            let next = (i + 1) % count;
            (vertices[next] - vertices[i]).length() / (tangents[i] + tangents[next])
        })
        .fold(1., f32::min);
    let point = |v: Vec2| [v.x, v.y];
    (0..count)
        .flat_map(|i| {
            let (previous, next, angle) = corners[i];
            let distance = tangents[i] * scale;
            let handle = (4. / 3.) * ((std::f32::consts::PI - angle) / 4.).tan() * radius * scale;
            let start = vertices[i] + previous * distance;
            let end = vertices[i] + next * distance;
            [
                Knot {
                    outgoing: point(start - previous * handle),
                    ..Knot::corner(point(start))
                },
                Knot {
                    incoming: point(end - next * handle),
                    ..Knot::corner(point(end))
                },
            ]
        })
        .collect()
}

fn shape_geometry(mode: usize, start: Pos2, end: Pos2, polygon_sides: u8) -> Option<Geometry> {
    let bounds = Rect::from_two_pos(start, end);
    if mode == 4 {
        let delta = end - start;
        if delta.length_sq() == 0. {
            return None;
        }
        let edge = egui::vec2(-delta.y, delta.x).normalized() * 0.5;
        return Some(Geometry::Path(
            [start + edge, end + edge, end - edge, start - edge]
                .into_iter()
                .map(|p| Knot::corner([p.x, p.y]))
                .collect(),
        ));
    }
    if !bounds.is_positive() {
        return None;
    }
    if mode == 0 {
        return Some(Geometry::Rectangle {
            min: [bounds.min.x, bounds.min.y],
            max: [bounds.max.x, bounds.max.y],
            radii: [0.; 4],
            angle: 0.,
        });
    }
    if mode == 2 {
        return Some(Geometry::Triangle {
            points: [
                [bounds.center().x, bounds.top()],
                [bounds.right(), bounds.bottom()],
                [bounds.left(), bounds.bottom()],
            ],
            radius: 0.,
        });
    }
    let center = bounds.center();
    let radius = bounds.size() * 0.5;
    if mode == 3 {
        return Some(Geometry::Polygon {
            center: [center.x, center.y],
            axes: [[radius.x, 0.], [0., radius.y]],
            sides: polygon_sides.clamp(3, 100),
            radius: 0.,
            inset: 100,
        });
    }
    if mode == 1 {
        let knots = [(0., -1.), (1., 0.), (0., 1.), (-1., 0.)].map(|(x, y)| {
            let point = center + egui::vec2(x * radius.x, y * radius.y);
            let tangent = egui::vec2(-y * radius.x, x * radius.y) * 0.552_284_8;
            Knot {
                point: [point.x, point.y],
                incoming: [point.x - tangent.x, point.y - tangent.y],
                outgoing: [point.x + tangent.x, point.y + tangent.y],
                linked: true,
            }
        });
        return Some(Geometry::Path(knots.into()));
    }
    None
}

fn resize_rectangle(min: [f32; 2], max: &mut [f32; 2], axis: usize, size: f32, linked: bool) {
    if linked {
        let other = 1 - axis;
        let ratio = (max[other] - min[other]) / (max[axis] - min[axis]);
        max[other] = min[other] + (size * ratio).clamp(1., 8192.);
    }
    max[axis] = min[axis] + size;
}

fn pen_knots(points: &[[f32; 2]], smooth: bool) -> Vec<Knot> {
    let mut knots: Vec<_> = points.iter().copied().map(Knot::corner).collect();
    if smooth {
        for index in 0..knots.len() {
            smooth_knot(&mut knots, index);
        }
    }
    knots
}

fn smooth_knot(knots: &mut [Knot], index: usize) {
    let previous = knots[(index + knots.len() - 1) % knots.len()].point;
    let next = knots[(index + 1) % knots.len()].point;
    let knot = &mut knots[index];
    for axis in 0..2 {
        let tangent = (next[axis] - previous[axis]) / 6.;
        knot.incoming[axis] = knot.point[axis] - tangent;
        knot.outgoing[axis] = knot.point[axis] + tangent;
    }
    knot.linked = true;
}

fn draw_path(
    painter: &egui::Painter,
    knots: &[Knot],
    to_screen: impl Fn([f32; 2]) -> Pos2,
    stroke: egui::Stroke,
    closed: bool,
) {
    let count = if closed {
        knots.len()
    } else {
        knots.len().saturating_sub(1)
    };
    for (a, b) in knots.iter().zip(knots.iter().cycle().skip(1)).take(count) {
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [
                to_screen(a.point),
                to_screen(a.outgoing),
                to_screen(b.incoming),
                to_screen(b.point),
            ],
            false,
            Color32::TRANSPARENT,
            stroke,
        ));
    }
}

fn edit_knot(knots: &mut Vec<Knot>, pointer: [f32; 2], threshold: f32, mode: usize) -> bool {
    let distance =
        |point: [f32; 2]| egui::vec2(point[0] - pointer[0], point[1] - pointer[1]).length_sq();
    if mode == 3 {
        if knots.len() >= 512 {
            return false;
        }
        let mix =
            |a: [f32; 2], b: [f32; 2], t| [egui::lerp(a[0]..=b[0], t), egui::lerp(a[1]..=b[1], t)];
        let split = |a: &Knot, b: &Knot, t| {
            let left = mix(a.point, a.outgoing, t);
            let middle = mix(a.outgoing, b.incoming, t);
            let right = mix(b.incoming, b.point, t);
            let incoming = mix(left, middle, t);
            let outgoing = mix(middle, right, t);
            (
                left,
                right,
                Knot {
                    point: mix(incoming, outgoing, t),
                    incoming,
                    outgoing,
                    linked: true,
                },
            )
        };
        let nearest = knots
            .iter()
            .zip(knots.iter().cycle().skip(1))
            .enumerate()
            .flat_map(|(index, (a, b))| {
                (1..32).map(move |step| {
                    let t = step as f32 / 32.;
                    (index, t, distance(split(a, b, t).2.point))
                })
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let Some((index, t, _)) = nearest.filter(|candidate| candidate.2 <= threshold * threshold)
        else {
            return false;
        };
        let next = (index + 1) % knots.len();
        let (left, right, knot) = split(&knots[index], &knots[next], t);
        knots[index].outgoing = left;
        knots[next].incoming = right;
        knots.insert(index + 1, knot);
        return true;
    }
    let Some((index, _)) = knots
        .iter()
        .enumerate()
        .map(|(index, knot)| (index, distance(knot.point)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|candidate| candidate.1 <= threshold * threshold)
    else {
        return false;
    };
    if mode == 4 {
        if knots.len() <= 3 {
            return false;
        }
        knots.remove(index);
    } else if knots[index].linked {
        knots[index] = Knot::corner(knots[index].point);
    } else {
        smooth_knot(knots, index);
    }
    true
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) enum Geometry {
    Rectangle {
        min: [f32; 2],
        max: [f32; 2],
        #[serde(alias = "radius", deserialize_with = "deserialize_radii")]
        radii: [f32; 4],
        #[serde(default)]
        angle: f32,
    },
    Triangle {
        points: [[f32; 2]; 3],
        radius: f32,
    },
    Polygon {
        center: [f32; 2],
        axes: [[f32; 2]; 2],
        sides: u8,
        radius: f32,
        inset: u8,
    },
    Path(Vec<Knot>),
}

fn deserialize_radii<'de, D: serde::Deserializer<'de>>(input: D) -> Result<[f32; 4], D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Radii {
        Independent([f32; 4]),
        Uniform(f32),
    }
    Ok(match Radii::deserialize(input)? {
        Radii::Independent(values) => values,
        Radii::Uniform(value) => [value; 4],
    })
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct Vector {
    pub(super) geometry: Geometry,
    pub(super) fill: Option<[u8; 3]>,
    pub(super) inverted: bool,
}

impl Vector {
    pub(super) fn property_label(&self) -> &'static str {
        match self.geometry {
            Geometry::Rectangle { .. } | Geometry::Triangle { .. } | Geometry::Polygon { .. } => {
                "形状属性"
            }
            Geometry::Path(_) => "路径属性",
        }
    }

    pub(super) fn layer_icon(&self) -> crate::icons::Icon {
        match self.geometry {
            Geometry::Rectangle { .. } => crate::icons::Icon::VectorShape,
            Geometry::Triangle { .. } => crate::icons::Icon::VectorTriangle,
            Geometry::Polygon { .. } => crate::icons::Icon::SelectionPolygon,
            Geometry::Path(_) => crate::icons::Icon::VectorPath,
        }
    }

    pub(super) fn valid(&self) -> bool {
        let point = |p: &[f32; 2]| p.iter().all(|v| v.abs() <= 131072.);
        match &self.geometry {
            Geometry::Rectangle {
                min,
                max,
                radii,
                angle,
            } => {
                point(min)
                    && point(max)
                    && min[0] < max[0]
                    && min[1] < max[1]
                    && radii.iter().all(|radius| (0. ..=8192.).contains(radius))
                    && (-180. ..=180.).contains(angle)
            }
            Geometry::Triangle { points, radius } => {
                points.iter().all(point) && (0. ..=8192.).contains(radius)
            }
            Geometry::Polygon {
                center,
                axes,
                sides,
                radius,
                inset,
            } => {
                point(center)
                    && axes.iter().all(point)
                    && (3..=100).contains(sides)
                    && (0. ..=8192.).contains(radius)
                    && (1..=100).contains(inset)
            }
            Geometry::Path(knots) => {
                (2..=512).contains(&knots.len())
                    && knots
                        .iter()
                        .all(|k| point(&k.point) && point(&k.incoming) && point(&k.outgoing))
            }
        }
    }

    pub(super) fn knots(&self) -> Vec<Knot> {
        match &self.geometry {
            Geometry::Path(knots) => knots.clone(),
            Geometry::Triangle { points, radius } => rounded_polygon(points, *radius),
            Geometry::Polygon {
                center,
                axes,
                sides,
                radius,
                inset,
            } => {
                let count = if *inset == 100 { *sides } else { *sides * 2 };
                let points: Vec<_> = (0..count)
                    .map(|index| {
                        let angle = std::f32::consts::TAU * f32::from(index) / f32::from(count)
                            - std::f32::consts::FRAC_PI_2;
                        let (sin, cos) = angle.sin_cos();
                        let ratio = if *inset < 100 && index % 2 != 0 {
                            f32::from(*inset) / 100.
                                * (std::f32::consts::PI / f32::from(*sides)).cos()
                        } else {
                            1.
                        };
                        std::array::from_fn(|axis| {
                            center[axis] + (axes[0][axis] * cos + axes[1][axis] * sin) * ratio
                        })
                    })
                    .collect();
                rounded_polygon(&points, *radius)
            }
            Geometry::Rectangle {
                min: [left, top],
                max: [right, bottom],
                radii,
                angle,
            } => {
                // Reduce all corners proportionally only when neighboring arcs overlap.
                let [tl, tr, br, bl] = *radii;
                let scale = [
                    (right - left, tl + tr),
                    (right - left, bl + br),
                    (bottom - top, tl + bl),
                    (bottom - top, tr + br),
                ]
                .into_iter()
                .filter(|(_, requested)| *requested > 0.)
                .map(|(length, requested)| length / requested)
                .fold(1., f32::min);
                let [tl, tr, br, bl] = radii.map(|radius| radius * scale);
                let points = [
                    [left + tl, *top],
                    [right - tr, *top],
                    [*right, top + tr],
                    [*right, bottom - br],
                    [right - br, *bottom],
                    [left + bl, *bottom],
                    [*left, bottom - bl],
                    [*left, top + tl],
                ];
                let tangents = [
                    [1., 0.],
                    [1., 0.],
                    [0., 1.],
                    [0., 1.],
                    [-1., 0.],
                    [-1., 0.],
                    [0., -1.],
                    [0., -1.],
                ];
                let radii = [tl, tr, tr, br, br, bl, bl, tl];
                let center = egui::vec2((left + right) * 0.5, (top + bottom) * 0.5);
                let spin = egui::emath::Rot2::from_angle(angle.to_radians());
                let rotate = |p: [f32; 2]| {
                    let p = center + spin * (egui::vec2(p[0], p[1]) - center);
                    [p.x, p.y]
                };
                points
                    .into_iter()
                    .zip(tangents)
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 0 || radii[*i] > 0.)
                    .map(|(i, (p, d))| {
                        let k = radii[i] * 0.55228475;
                        let mut knot = Knot::corner(p);
                        if i % 2 == 0 {
                            knot.incoming = [p[0] - k * d[0], p[1] - k * d[1]];
                        } else {
                            knot.outgoing = [p[0] + k * d[0], p[1] + k * d[1]];
                        }
                        Knot {
                            point: rotate(knot.point),
                            incoming: rotate(knot.incoming),
                            outgoing: rotate(knot.outgoing),
                            ..knot
                        }
                    })
                    .collect()
            }
        }
    }

    fn path(&self) -> Option<sk::Path> {
        let knots = self.knots();
        let first = knots.first()?;
        let mut path = sk::PathBuilder::new();
        path.move_to(first.point[0], first.point[1]);
        for (a, b) in knots.iter().zip(knots.iter().cycle().skip(1)) {
            path.cubic_to(
                a.outgoing[0],
                a.outgoing[1],
                b.incoming[0],
                b.incoming[1],
                b.point[0],
                b.point[1],
            );
        }
        path.close();
        path.finish()
    }

    pub(super) fn bounds(&self) -> Option<Rect> {
        let bounds = self.path()?.compute_tight_bounds()?;
        Some(Rect::from_min_max(
            egui::pos2(bounds.left(), bounds.top()),
            egui::pos2(bounds.right(), bounds.bottom()),
        ))
    }

    pub(super) fn transformed(&self, map: impl Fn(Pos2) -> Pos2, scale: Vec2, angle: f32) -> Self {
        let point = |p: [f32; 2]| {
            let p = map(egui::pos2(p[0], p[1]));
            [p.x, p.y]
        };
        let geometry = match &self.geometry {
            Geometry::Rectangle {
                min,
                max,
                radii,
                angle: original_angle,
            } if (scale.x - scale.y).abs() <= scale.x.max(scale.y) * 0.000_001
                || (*original_angle == 0. && *radii == [0.; 4]) =>
            {
                let center = point([(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5]);
                let half = [
                    (max[0] - min[0]) * scale.x * 0.5,
                    (max[1] - min[1]) * scale.y * 0.5,
                ];
                Geometry::Rectangle {
                    min: [center[0] - half[0], center[1] - half[1]],
                    max: [center[0] + half[0], center[1] + half[1]],
                    radii: radii.map(|radius| (radius * scale.x).min(8192.)),
                    angle: (*original_angle + angle + 180.).rem_euclid(360.) - 180.,
                }
            }
            Geometry::Triangle { points, radius } => Geometry::Triangle {
                points: points.map(point),
                radius: (radius * scale.x.min(scale.y)).min(8192.),
            },
            Geometry::Polygon {
                center,
                axes,
                sides,
                radius,
                inset,
            } => {
                let position = point(*center);
                let axes = axes.map(|axis| {
                    let end = point([center[0] + axis[0], center[1] + axis[1]]);
                    [end[0] - position[0], end[1] - position[1]]
                });
                Geometry::Polygon {
                    center: position,
                    axes,
                    sides: *sides,
                    radius: (radius * scale.x.min(scale.y)).min(8192.),
                    inset: *inset,
                }
            }
            _ => Geometry::Path(
                self.knots()
                    .into_iter()
                    .map(|knot| Knot {
                        point: point(knot.point),
                        incoming: point(knot.incoming),
                        outgoing: point(knot.outgoing),
                        ..knot
                    })
                    .collect(),
            ),
        };
        Self {
            geometry,
            ..self.clone()
        }
    }

    pub(super) fn raster(&self, w: u32, h: u32) -> Option<RgbaImage> {
        let mut image = sk::Pixmap::new(w, h)?;
        if let Some(path) = self.path() {
            let mut paint = sk::Paint::default();
            paint.set_color_rgba8(255, 255, 255, 255);
            image.fill_path(
                &path,
                &paint,
                sk::FillRule::EvenOdd,
                sk::Transform::identity(),
                None,
            );
        }
        let color = self.fill.unwrap_or([255; 3]);
        Some(RgbaImage::from_fn(w, h, |x, y| {
            let alpha = image.data()[((y * w + x) * 4 + 3) as usize];
            Rgba([
                color[0],
                color[1],
                color[2],
                if self.inverted { 255 - alpha } else { alpha },
            ])
        }))
    }
}

#[derive(Default)]
pub(super) struct PathTool {
    start: Option<Pos2>,
    path_mode: usize,
    shape_mode: usize,
    polygon_sides: Option<u8>,
    pen_mode: usize,
    pen: Vec<[f32; 2]>,
    drag: Option<(usize, usize, u8, Pos2, Vector)>,
    conversion: Option<(usize, Vector)>,
    pub(super) corner_drag: Option<CornerDrag>,
    pub(super) corner_unlinked: bool,
}

#[derive(Clone, Copy)]
pub(super) struct CornerDrag {
    pub(super) layer: usize,
    pub(super) corner: usize,
    pub(super) pointer: Pos2,
    pub(super) radius: f32,
    pub(super) changed: bool,
}
impl PathTool {
    pub(super) fn modes(tool: Tool) -> &'static [(&'static str, crate::icons::Icon)] {
        use crate::icons::Icon;
        match tool {
            Tool::Path => &[
                ("路径选择", Icon::PathSelectionFilled),
                ("直接选择", Icon::PathSelection),
            ],
            Tool::Shape => &[
                ("矩形", Icon::VectorShape),
                ("椭圆", Icon::SelectionEllipse),
                ("三角形", Icon::VectorTriangle),
                ("多边形", Icon::SelectionPolygon),
                ("直线", Icon::LineTool),
            ],
            Tool::Pen => &[
                ("钢笔", Icon::PenNib),
                ("自由钢笔", Icon::PenFree),
                ("弯度钢笔", Icon::PenCurve),
                ("添加锚点", Icon::PenAdd),
                ("删除锚点", Icon::PenDelete),
                ("转换点", Icon::PenConvert),
            ],
            _ => &[],
        }
    }

    pub(super) fn mode(&self, tool: Tool) -> usize {
        match tool {
            Tool::Path => self.path_mode,
            Tool::Shape => self.shape_mode,
            Tool::Pen => self.pen_mode,
            _ => 0,
        }
    }

    pub(super) fn set_mode(&mut self, tool: Tool, mode: usize) {
        if self.mode(tool) == mode || mode >= Self::modes(tool).len() {
            return;
        }
        match tool {
            Tool::Path => self.path_mode = mode,
            Tool::Shape => self.shape_mode = mode,
            Tool::Pen => self.pen_mode = mode,
            _ => return,
        }
        self.start = None;
        self.drag = None;
        self.clear_pen();
    }

    pub(super) fn confirming_conversion(&self) -> bool {
        self.conversion.is_some()
    }

    pub(super) fn clear_pen(&mut self) {
        self.pen.clear();
    }
}

impl Editor {
    pub(super) fn vector_available(&self) -> bool {
        !self.elevated
    }

    pub(super) fn vector_cell(&self, origin: Pos2, scale: f32, layout: bool) -> Rect {
        let (w, h) = self.pixels.dimensions();
        let (col, row) = if layout {
            (
                (self.selected_slot % 8) as i32,
                (self.selected_slot / 8) as i32,
            )
        } else {
            (0, 0)
        };
        Rect::from_center_size(
            origin + self.mapped_center(col, row, w, h) * scale,
            egui::vec2(w as f32, h as f32) * scale,
        )
    }

    fn apply_vector(&mut self, index: usize, vector: Vector) {
        self.update_vector(index, vector, true);
    }

    pub(super) fn vector_origin(&self, saved: Option<[i32; 2]>) -> Option<[i32; 2]> {
        if self.plain_image {
            return None;
        }
        saved.or_else(|| {
            let min = self.vector_cell(Pos2::ZERO, 1., self.whole_unit()).min;
            Some([min.x.round() as i32, min.y.round() as i32])
        })
    }

    pub(super) fn render_vector_tiles(
        &self,
        vector: &Vector,
        origin: Option<[i32; 2]>,
    ) -> Option<Vec<RgbaImage>> {
        let (w, h) = self.pixels.dimensions();
        let Some([ox, oy]) = origin else {
            return Some(vec![vector.raster(w, h)?; self.tiles.len()]);
        };
        let color = vector.fill.unwrap_or([255; 3]);
        let mut tiles = vec![
            RgbaImage::from_pixel(w, h, Rgba([color[0], color[1], color[2], 0]));
            self.tiles.len()
        ];
        if let Some(bounds) = vector.bounds() {
            let start = bounds.min.floor();
            let size = bounds.max.ceil() - start;
            if size.x * size.y > 4. * 1024. * 1024. {
                return None;
            }
            let mut stamp = vector.transformed(|p| p - start.to_vec2(), Vec2::splat(1.), 0.);
            stamp.inverted = false;
            let stamp = stamp.raster((size.x as u32).max(1), (size.y as u32).max(1))?;
            for (x, y, pixel) in stamp.enumerate_pixels().filter(|(_, _, p)| p[3] != 0) {
                let x = ox.saturating_add(start.x as i32).saturating_add(x as i32);
                let y = oy.saturating_add(start.y as i32).saturating_add(y as i32);
                if let Some((tile, x, y)) = self.geometry_hit(x, y, self.multiple, self.wrap) {
                    let target = tiles[tile].get_pixel_mut(x, y);
                    target[3] = target[3].max(pixel[3]);
                }
            }
        }
        if vector.inverted {
            for image in &mut tiles {
                for pixel in image.pixels_mut() {
                    pixel[3] = 255 - pixel[3];
                }
            }
        }
        Some(tiles)
    }

    pub(super) fn update_vector(&mut self, index: usize, vector: Vector, record: bool) {
        let origin = self.vector_origin(self.layers[index].settings.vector_origin);
        let Some(images) = self.render_vector_tiles(&vector, origin) else {
            return;
        };
        if record {
            self.record_layer_change();
        }
        let layer = &mut self.layers[index];
        if vector.fill.is_some() {
            layer.tiles = images;
        } else {
            layer.settings.mask = Some(
                images
                    .iter()
                    .map(|image| image.pixels().map(|p| p[3]).collect())
                    .collect(),
            );
        }
        layer.settings.vector = Some(vector);
        layer.settings.vector_origin = origin;
        self.tiles = layer.tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }

    fn add_vector(&mut self, vector: Vector) {
        let (w, h) = self.pixels.dimensions();
        let tile_count = self.tiles.len();
        if !self.can_add_layers(&[], w as usize * h as usize * 4 * tile_count) {
            return;
        }
        let origin = self.vector_origin(None);
        let Some(images) = self.render_vector_tiles(&vector, origin) else {
            return;
        };
        self.record_layer_change();
        let index = self.layers.len();
        self.layers.push(Layer {
            name: tr("矢量形状").into(),
            visible: true,
            locked: false,
            material: false,
            tiles: images,
            settings: layer_properties::Properties {
                vector: Some(vector),
                vector_origin: origin,
                ..Default::default()
            },
        });
        self.select_layer(index);
    }

    pub(super) fn vector_options(&mut self, ui: &mut egui::Ui) {
        if !self.vector_available() {
            return;
        }
        let modes = PathTool::modes(self.tool);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            for (index, &(label, _)) in modes.iter().enumerate() {
                let left = if index == 0 { 4 } else { 0 };
                let right = if index + 1 == modes.len() { 4 } else { 0 };
                let radius = egui::CornerRadius {
                    nw: left,
                    sw: left,
                    ne: right,
                    se: right,
                };
                if ui
                    .add(
                        egui::Button::new(tr(label))
                            .selected(self.path_tool.mode(self.tool) == index)
                            .corner_radius(radius),
                    )
                    .clicked()
                {
                    self.path_tool.set_mode(self.tool, index);
                }
            }
        });
        if self.tool == Tool::Shape && self.path_tool.shape_mode == 3 {
            ui.label(tr("边数"));
            ui.add(
                egui::DragValue::new(self.path_tool.polygon_sides.get_or_insert(6)).range(3..=100),
            );
        }
        let index = self.active_layer;
        let Some(mut vector) = self
            .layers
            .get(index)
            .and_then(|l| l.settings.vector.clone())
        else {
            return;
        };
        ui.add_enabled_ui(self.vector_unlocked(index), |ui| {
            let mut changed = false;
            if let Geometry::Rectangle { min, max, .. } = &mut vector.geometry {
                for (axis, label) in ["宽度", "高度"].into_iter().enumerate() {
                    ui.label(tr(label));
                    let mut size = max[axis] - min[axis];
                    if ui
                        .add(
                            egui::DragValue::new(&mut size)
                                .range(1. ..=8192.)
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        resize_rectangle(*min, max, axis, size, self.transform_linked);
                        changed = true;
                    }
                }
            }
            if let Some(fill) = &mut vector.fill {
                ui.label(tr("填充"));
                changed |= crate::color_property::edit_rgb(ui, tr("填充"), fill);
            }
            if changed {
                self.apply_vector(index, vector);
            }
        });
    }

    pub(super) fn vector_detail_ui(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        name: String,
        position: Pos2,
        mut vector: Vector,
    ) {
        let mut open = true;
        let mut changed = false;
        let mut edit_path = false;
        let enabled =
            self.vector_unlocked(index) && self.transform.is_none() && self.pending_open.is_none();
        egui::Window::new(name)
            .id(ui.id().with("vector_layer_detail"))
            .open(&mut open)
            .collapsible(false)
            .auto_sized()
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
                ui.set_width(230.);
                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(16.), egui::Sense::hover());
                        crate::icons::paint(
                            ui,
                            vector.layer_icon(),
                            rect,
                            ui.visuals().text_color(),
                        );
                        ui.strong(tr(vector.property_label()));
                    });
                    if let Geometry::Rectangle {
                        min,
                        max,
                        radii,
                        angle,
                    } = &mut vector.geometry
                    {
                        ui.separator();
                        ui.checkbox(&mut self.transform_linked, tr("锁定比例"));
                        egui::Grid::new("vector_rect_size")
                            .num_columns(4)
                            .show(ui, |ui| {
                                for (axis, size_label, position_label) in
                                    [(0, "宽度", "X"), (1, "高度", "Y")]
                                {
                                    let mut size = max[axis] - min[axis];
                                    ui.label(tr(size_label));
                                    let resized = ui
                                        .add(
                                            egui::DragValue::new(&mut size)
                                                .range(1. ..=8192.)
                                                .suffix(" px"),
                                        )
                                        .changed();
                                    ui.label(position_label);
                                    let mut position = min[axis];
                                    let moved = ui
                                        .add(
                                            egui::DragValue::new(&mut position)
                                                .range(-65536. ..=65536.)
                                                .suffix(" px"),
                                        )
                                        .changed();
                                    if resized {
                                        resize_rectangle(
                                            *min,
                                            max,
                                            axis,
                                            size,
                                            self.transform_linked,
                                        );
                                        changed = true;
                                    }
                                    if moved {
                                        max[axis] += position - min[axis];
                                        min[axis] = position;
                                        changed = true;
                                    }
                                    ui.end_row();
                                }
                            });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("角度"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(angle)
                                        .range(-180. ..=180.)
                                        .max_decimals(2)
                                        .suffix("°"),
                                )
                                .changed();
                            if ui.button(tr("左右翻转")).clicked() {
                                radii.swap(0, 1);
                                radii.swap(2, 3);
                                *angle = -*angle;
                                changed = true;
                            }
                            if ui.button(tr("上下翻转")).clicked() {
                                radii.swap(0, 3);
                                radii.swap(1, 2);
                                *angle = -*angle;
                                changed = true;
                            }
                        });
                        ui.separator();
                        let mut linked = !self.path_tool.corner_unlinked;
                        ui.checkbox(&mut linked, tr("联动圆角"));
                        self.path_tool.corner_unlinked = !linked;
                        egui::Grid::new("vector_corner_radii")
                            .num_columns(4)
                            .show(ui, |ui| {
                                for (axis, label) in
                                    [(0, "左上"), (1, "右上"), (3, "左下"), (2, "右下")]
                                {
                                    ui.label(tr(label));
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut radii[axis])
                                                .range(0. ..=8192.)
                                                .suffix(" px"),
                                        )
                                        .changed()
                                    {
                                        if linked {
                                            let radius = radii[axis];
                                            radii.fill(radius);
                                        }
                                        changed = true;
                                    }
                                    if axis == 1 || axis == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    } else if let Geometry::Polygon {
                        sides,
                        radius,
                        inset,
                        ..
                    } = &mut vector.geometry
                    {
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("边数"));
                            changed |= ui.add(egui::DragValue::new(sides).range(3..=100)).changed();
                            ui.label(tr("圆角"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(radius)
                                        .range(0. ..=8192.)
                                        .max_decimals(2)
                                        .suffix(" px"),
                                )
                                .changed();
                            ui.label(tr("星形比例"));
                            changed |= ui
                                .add(egui::DragValue::new(inset).range(1..=100).suffix("%"))
                                .changed();
                        });
                    } else if let Geometry::Triangle { radius, .. } = &mut vector.geometry {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(tr("圆角"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(radius)
                                        .range(0. ..=8192.)
                                        .max_decimals(2)
                                        .suffix(" px"),
                                )
                                .changed();
                        });
                    } else {
                        edit_path = ui.button(tr("编辑路径锚点")).clicked();
                    }
                    if let Some(fill) = &mut vector.fill {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(tr("填充"));
                            changed |= crate::color_property::edit_rgb(ui, tr("填充"), fill);
                        });
                    }
                });
            });
        if changed && vector.valid() {
            self.apply_vector(index, vector);
        }
        if edit_path && self.finish_transform(true) {
            self.finish_stroke();
            self.tool = Tool::Path;
            self.path_tool.set_mode(Tool::Path, 1);
        }
        if !open {
            self.layer_detail = None;
        }
    }

    fn vector_unlocked(&self, i: usize) -> bool {
        self.text_unlocked(i)
            && !self.layers[i].settings.lock_position
            && !self.ancestor_move_locked(i)
    }

    pub(super) fn vector_canvas(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
        image: Rect,
        scale: f32,
        blocked: bool,
    ) {
        if self.tool != Tool::Pen {
            self.path_tool.clear_pen();
        }
        if !self.vector_available() || !matches!(self.tool, Tool::Shape | Tool::Path | Tool::Pen) {
            self.path_tool.start = None;
            self.path_tool.drag = None;
            return;
        }
        let point = response
            .interact_pointer_pos()
            .map(|p| ((p - image.min) / scale).to_pos2());
        let to_screen = |p: [f32; 2]| image.min + egui::vec2(p[0], p[1]) * scale;
        let outline = egui::Stroke::new(1_f32, Color32::from_rgb(95, 175, 255));
        let pen_mode = self.path_tool.pen_mode;
        if self.tool == Tool::Pen && pen_mode < 3 {
            let mut close = !blocked
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            if pen_mode == 1 && !blocked {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    self.path_tool.clear_pen();
                    if let Some(start) = ui
                        .input(|i| i.pointer.press_origin())
                        .filter(|p| !self.plain_image || image.contains(*p))
                    {
                        let p = (start - image.min) / scale;
                        self.path_tool.pen.push([p.x, p.y]);
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary)
                    && let Some(p) = point.filter(|p| {
                        !self.path_tool.pen.is_empty()
                            && self.path_tool.pen.len() < 512
                            && to_screen(self.path_tool.pen.last().copied().unwrap())
                                .distance(to_screen([p.x, p.y]))
                                >= 3.
                    })
                {
                    self.path_tool.pen.push([p.x, p.y]);
                }
                close |= response.drag_stopped_by(egui::PointerButton::Primary);
            } else if !blocked
                && response.clicked_by(egui::PointerButton::Primary)
                && let Some(pointer) = response
                    .interact_pointer_pos()
                    .filter(|p| !self.plain_image || image.contains(*p))
            {
                close |= self.path_tool.pen.len() >= 3
                    && to_screen(self.path_tool.pen[0]).distance(pointer) <= 7.;
                if !close && self.path_tool.pen.len() < 512 {
                    let p = (pointer - image.min) / scale;
                    let anchor = [p.x.round(), p.y.round()];
                    if self.path_tool.pen.last() != Some(&anchor) {
                        self.path_tool.pen.push(anchor);
                    }
                }
            }
            if close && self.path_tool.pen.len() >= 3 {
                let points = std::mem::take(&mut self.path_tool.pen);
                self.add_vector(Vector {
                    geometry: Geometry::Path(pen_knots(&points, pen_mode == 2)),
                    fill: Some([self.color[0], self.color[1], self.color[2]]),
                    inverted: false,
                });
                return;
            }
            let mut points = self.path_tool.pen.clone();
            for point in points.iter().copied().map(to_screen) {
                painter.rect_filled(
                    Rect::from_center_size(point, Vec2::splat(6.)),
                    1.,
                    outline.color,
                );
            }
            if let Some(pointer) = response.hover_pos()
                && !points.is_empty()
                && pen_mode != 1
            {
                let p = (pointer - image.min) / scale;
                points.push([p.x, p.y]);
            }
            draw_path(
                painter,
                &pen_knots(&points, pen_mode == 2),
                to_screen,
                outline,
                false,
            );
            return;
        }
        if self.tool == Tool::Shape {
            if !blocked && response.drag_started_by(egui::PointerButton::Primary) {
                self.path_tool.start = ui
                    .input(|i| i.pointer.press_origin())
                    .filter(|p| !self.plain_image || image.contains(*p))
                    .map(|p| ((p - image.min) / scale).to_pos2().round());
            }
            if let (Some(a), Some(b)) = (self.path_tool.start, point) {
                let b = if self.plain_image {
                    b.round().clamp(Pos2::ZERO, (image.size() / scale).to_pos2())
                } else {
                    b.round()
                };
                let vector = shape_geometry(
                    self.path_tool.shape_mode,
                    a,
                    b,
                    self.path_tool.polygon_sides.unwrap_or(6),
                )
                .map(|geometry| Vector {
                    geometry,
                    fill: Some([self.color[0], self.color[1], self.color[2]]),
                    inverted: false,
                });
                if let Some(vector) = &vector {
                    draw_path(painter, &vector.knots(), to_screen, outline, true);
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    self.path_tool.start = None;
                    if let Some(vector) = vector {
                        self.add_vector(vector);
                    }
                }
            }
            return;
        }
        if !blocked
            && response.clicked_by(egui::PointerButton::Primary)
            && let Some(p) = point.filter(|p| p.x >= 0. && p.y >= 0.)
            && let Some(index) = self
                .layers
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, layer)| {
                    let x = p.x as u32;
                    let y = p.y as u32;
                    let image = &layer.tiles[self.selected];
                    (layer.settings.vector.is_some()
                        && layer.visible
                        && self.ancestors_visible(index)
                        && x < image.width()
                        && y < image.height()
                        && image.get_pixel(x, y)[3] != 0)
                        .then_some(index)
                })
        {
            self.select_layer(index);
        }
        let index = self.active_layer;
        let Some(vector) = self
            .layers
            .get(index)
            .and_then(|l| l.settings.vector.clone())
        else {
            return;
        };
        let cell = self.vector_origin(self.layers[index].settings.vector_origin);
        let image = if let Some([x, y]) = cell {
            let current = self.vector_origin(None).unwrap_or([x, y]);
            image.translate(egui::vec2(
                (x as f32 - current[0] as f32) * scale,
                (y as f32 - current[1] as f32) * scale,
            ))
        } else {
            image
        };
        let point = response
            .interact_pointer_pos()
            .map(|p| ((p - image.min) / scale).to_pos2());
        let to_screen = |p: [f32; 2]| image.min + egui::vec2(p[0], p[1]) * scale;
        let mut displayed = vector.clone();
        if self.tool == Tool::Pen
            && !blocked
            && self.path_tool.conversion.is_none()
            && self.vector_unlocked(index)
            && response.clicked_by(egui::PointerButton::Primary)
            && let Some(point) = point
        {
            let mut knots = displayed.knots();
            if edit_knot(&mut knots, [point.x, point.y], 7. / scale, pen_mode) {
                let convert = !matches!(displayed.geometry, Geometry::Path(_));
                displayed.geometry = Geometry::Path(knots);
                if convert {
                    self.path_tool.conversion = Some((index, displayed.clone()));
                } else {
                    self.apply_vector(index, displayed.clone());
                }
            }
        }
        if !blocked
            && self.tool == Tool::Path
            && self.path_tool.conversion.is_none()
            && self.vector_unlocked(index)
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(press) = ui.input(|i| i.pointer.press_origin())
        {
            let handles = |k: &Knot| [(0, k.point), (1, k.incoming), (2, k.outgoing)];
            let closest = vector
                .knots()
                .iter()
                .enumerate()
                .flat_map(|(i, k)| handles(k).into_iter().map(move |(part, p)| (i, part, p)))
                .filter(|(_, _, p)| to_screen(*p).distance(press) <= 7.)
                .min_by(|a, b| {
                    to_screen(a.2)
                        .distance(press)
                        .total_cmp(&to_screen(b.2).distance(press))
                });
            let candidate = if self.path_tool.path_mode == 0 {
                let local = (press - image.min) / scale;
                let inside = image.contains(press)
                    && self.layers[index].tiles[self.selected]
                        .get_pixel_checked(local.x as u32, local.y as u32)
                        .is_some_and(|p| p[3] != 0);
                (inside || closest.is_some()).then_some((0, 3, [0.; 2]))
            } else {
                closest
            };
            if let Some((knot, part, _)) = candidate {
                self.path_tool.drag = Some((
                    index,
                    knot,
                    part,
                    ((press - image.min) / scale).to_pos2(),
                    vector.clone(),
                ));
            }
        }
        if let (Some((owner, knot, part, start, original)), Some(point)) =
            (&self.path_tool.drag, point)
            && *owner == index
        {
            let delta = point - *start;
            let mut knots = original.knots();
            let k = &mut knots[*knot];
            let shift = |p: &mut [f32; 2]| {
                p[0] += delta.x;
                p[1] += delta.y;
            };
            match part {
                0 => {
                    shift(&mut k.point);
                    shift(&mut k.incoming);
                    shift(&mut k.outgoing);
                }
                1 => {
                    shift(&mut k.incoming);
                    k.linked = false;
                }
                2 => {
                    shift(&mut k.outgoing);
                    k.linked = false;
                }
                _ => {}
            }
            if *part == 3 {
                match &mut displayed.geometry {
                    Geometry::Rectangle { min, max, .. } => {
                        shift(min);
                        shift(max);
                    }
                    Geometry::Triangle { points, .. } => {
                        for point in points {
                            shift(point);
                        }
                    }
                    Geometry::Polygon { center, .. } => shift(center),
                    Geometry::Path(knots) => {
                        for knot in knots {
                            shift(&mut knot.point);
                            shift(&mut knot.incoming);
                            shift(&mut knot.outgoing);
                        }
                    }
                }
            } else {
                displayed.geometry = Geometry::Path(knots);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                let convert = *part != 3 && !matches!(original.geometry, Geometry::Path(_));
                self.path_tool.drag = None;
                if delta.length_sq() > 0. && displayed.valid() {
                    if convert {
                        self.path_tool.conversion = Some((index, displayed.clone()));
                    } else {
                        self.apply_vector(index, displayed.clone());
                    }
                }
            }
        }
        let knots = displayed.knots();
        for (a, b) in knots.iter().zip(knots.iter().cycle().skip(1)) {
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                [
                    to_screen(a.point),
                    to_screen(a.outgoing),
                    to_screen(b.incoming),
                    to_screen(b.point),
                ],
                false,
                Color32::TRANSPARENT,
                outline,
            ));
            if self.tool == Tool::Path && self.path_tool.path_mode == 0 {
                continue;
            }
            for p in [a.incoming, a.outgoing] {
                if p != a.point {
                    painter.line_segment([to_screen(a.point), to_screen(p)], outline);
                    painter.circle_filled(to_screen(p), 3., outline.color);
                }
            }
            painter.rect_filled(
                Rect::from_center_size(to_screen(a.point), Vec2::splat(6.)),
                1.,
                outline.color,
            );
        }
    }

    pub(super) fn vector_confirmation(&mut self, ui: &egui::Ui) {
        let Some((index, vector)) = self.path_tool.conversion.take() else {
            return;
        };
        let decision = editor_confirmation(
            ui,
            egui::Modal::new(ui.id().with("shape_to_path")),
            tr("确认转换为路径"),
            tr("编辑锚点会将形状转换为路径，圆角将不能再按数值调整。"),
            tr("取消"),
            tr("确定"),
            |_| {},
        );
        if decision == Some(true) && self.active_layer == index && self.vector_unlocked(index) {
            self.apply_vector(index, vector);
        } else if decision.is_none() {
            self.path_tool.conversion = Some((index, vector));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_tile_2x2_vector_and_mask_follow_group_edges() {
        let mut editor = Editor::default();
        if let Some(path) = std::env::var_os("UYV_TILE_2X2_FIXTURE") {
            editor.load_project(Path::new(&path)).unwrap();
        } else {
            editor.set_single_tile(RgbaImage::new(32, 16));
            editor.multiple = true;
            editor.columns = 2;
            editor.rows = 2;
            editor.independent_slots().unwrap();
        }
        assert!(editor.diamond && editor.multiple && editor.columns == 2 && editor.rows == 2);
        let (w, h) = editor.pixels.dimensions();
        let first = editor.tile_at(0, 0);
        let adjacent = editor.tile_at(1, 0);
        let mut vector = Vector {
            geometry: Geometry::Rectangle {
                min: [0., 0.],
                max: [w as f32 / 2., h as f32 / 2.],
                radii: [0.; 4],
                angle: 0.,
            },
            fill: Some([255; 3]),
            inverted: false,
        };
        editor.wrap = false;
        let images = editor.render_vector_tiles(&vector, Some([0, 0])).unwrap();
        assert!(images[first].pixels().any(|p| p[3] > 0));
        assert!(images[adjacent].pixels().any(|p| p[3] > 0));
        vector.geometry = Geometry::Path(vector.knots());
        vector.fill = None;
        let outside = [-(w as i32) / 2 - 1, -(h as i32) / 2 - 1];
        let mask = editor.render_vector_tiles(&vector, Some(outside)).unwrap();
        assert!(mask[adjacent].pixels().all(|p| p[3] == 0));
        editor.wrap = true;
        let mask = editor.render_vector_tiles(&vector, Some(outside)).unwrap();
        assert_eq!(mask[adjacent].get_pixel(w / 2, h / 2)[3], 255);
        assert_eq!(mask[first].get_pixel(w / 2, h / 2)[3], 0);
    }

    #[test]
    fn rotating_a_rounded_rectangle_keeps_its_editable_properties() {
        let shape = Vector {
            geometry: Geometry::Rectangle {
                min: [10., 5.],
                max: [30., 15.],
                radii: [1., 2., 3., 4.],
                angle: 15.,
            },
            fill: Some([255; 3]),
            inverted: false,
        };
        let center = egui::pos2(20., 10.);
        let rotate = egui::emath::Rot2::from_angle(30_f32.to_radians());
        let rotated = shape.transformed(|p| center + rotate * (p - center), Vec2::splat(1.), 30.);
        assert!(matches!(
            rotated.geometry,
            Geometry::Rectangle {
                min: [10., 5.],
                max: [30., 15.],
                radii: [1., 2., 3., 4.],
                angle: 45.
            }
        ));
    }

    #[test]
    fn pixel_paint_over_vector_uses_new_layer_and_one_undo() {
        let mut editor = Editor::default();
        editor.init_image(&RgbaImage::new(20, 20));
        editor.add_vector(Vector {
            geometry: Geometry::Rectangle {
                min: [3., 3.],
                max: [16., 16.],
                radii: [0., 1., 2., 3.],
                angle: 0.,
            },
            fill: Some([240, 240, 240]),
            inverted: false,
        });
        let index = editor.active_layer;
        let original = editor.layers[index].clone();
        let offset = editor.mapped_center(0, 0, 20, 20) - Vec2::splat(10.);
        let point = (offset.x as i32 + 5, offset.y as i32 + 5);
        editor.tool = Tool::Brush;
        editor.soft_brush = false;
        editor.brush_size = 1;
        editor.color = [10, 40, 100, 255];
        editor.plane_stroke(point, point, false);
        editor.finish_stroke();
        assert_eq!(editor.active_layer, index + 1);
        assert!(editor.layers[index] == original);
        assert_eq!(editor.pixels.get_pixel(5, 5).0, [10, 40, 100, 255]);
        editor.history(false);
        assert_eq!(editor.layers.len(), index + 1);
        assert!(editor.layers[index] == original);
    }
}
