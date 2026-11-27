use super::*;
use std::collections::BTreeSet;

type Anchors = BTreeSet<(usize, usize)>;

#[derive(Default)]
pub(super) struct PathSelection {
    component: Option<(usize, usize)>,
    anchors: Anchors,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handle {
    Anchor,
    Incoming,
    Outgoing,
}

enum Action {
    Component(usize),
    Anchors,
    Control(usize, usize, Handle),
    Marquee(Anchors),
}

pub(super) struct Drag {
    view: egui::Id,
    layer: usize,
    start: Pos2,
    action: Action,
}

impl Vector {
    fn contours(&self) -> Vec<Vec<Knot>> {
        match &self.geometry {
            Geometry::Compound(parts) => parts.iter().map(|p| p.vector.knots()).collect(),
            _ => vec![self.knots()],
        }
    }

    fn component_mut(&mut self, index: usize) -> &mut Self {
        if let Geometry::Compound(ref mut parts) = self.geometry {
            &mut parts[index].vector
        } else {
            self
        }
    }
}

fn anchor_hit(
    contours: &[Vec<Knot>],
    point: Pos2,
    tolerance: f32,
    selected: &Anchors,
) -> Option<(usize, usize, Handle)> {
    let mut best = None;
    let mut distance = tolerance * tolerance;
    for (component, knots) in contours.iter().enumerate().rev() {
        for (index, knot) in knots.iter().enumerate() {
            for (handle, at) in [
                (Handle::Anchor, knot.point),
                (Handle::Incoming, knot.incoming),
                (Handle::Outgoing, knot.outgoing),
            ] {
                if handle != Handle::Anchor
                    && (at == knot.point || !selected.contains(&(component, index)))
                {
                    continue;
                }
                let d = point.distance_sq(at.into());
                if d < distance {
                    best = Some((component, index, handle));
                    distance = d;
                }
            }
        }
    }
    best
}

fn component_hit(contours: &[Vec<Knot>], point: Pos2, tolerance: f32) -> Option<usize> {
    // Handles and outlines remain selectable even inside another component's fill.
    let mut fill = None;
    for (component, knots) in contours.iter().enumerate().rev() {
        let mut inside = false;
        for (a, b) in knots.iter().zip(knots.iter().cycle().skip(1)) {
            let curve = egui::epaint::CubicBezierShape::from_points_stroke(
                [
                    a.point.into(),
                    a.outgoing.into(),
                    b.incoming.into(),
                    b.point.into(),
                ],
                false,
                Color32::TRANSPARENT,
                egui::Stroke::NONE,
            );
            let points = curve.flatten(Some(tolerance * 0.1));
            for segment in points.windows(2) {
                let (a, b) = (segment[0], segment[1]);
                let edge = b - a;
                let t =
                    ((point - a).dot(edge) / edge.length_sq().max(f32::MIN_POSITIVE)).clamp(0., 1.);
                if point.distance_sq(a + edge * t) <= tolerance * tolerance {
                    return Some(component);
                }
                if (a.y > point.y) != (b.y > point.y)
                    && point.x < a.x + (point.y - a.y) * edge.x / edge.y
                {
                    inside = !inside;
                }
            }
        }
        if inside && fill.is_none() {
            fill = Some(component);
        }
    }
    fill
}

fn move_points(vector: &mut Vector, action: &Action, selected: &Anchors, delta: Vec2) -> bool {
    let translate = |shape: &mut Vector| {
        *shape = shape.transformed(|p| p + delta, Vec2::splat(1.), 0.);
    };
    if let Action::Component(component) = action {
        translate(vector.component_mut(*component));
        return false;
    }
    let shift = |p: &mut [f32; 2]| {
        p[0] += delta.x;
        p[1] += delta.y;
    };
    let mut convert = false;
    for (component, mut knots) in vector.contours().into_iter().enumerate() {
        let target = vector.component_mut(component);
        match action {
            Action::Anchors => {
                let mut indices = selected
                    .range((component, 0)..=(component, usize::MAX))
                    .peekable();
                if indices.peek().is_none() {
                    continue;
                }
                if indices.clone().count() == knots.len() {
                    translate(target);
                    continue;
                }
                for &(_, index) in indices {
                    let Some(knot) = knots.get_mut(index) else {
                        continue;
                    };
                    shift(&mut knot.point);
                    shift(&mut knot.incoming);
                    shift(&mut knot.outgoing);
                }
            }
            Action::Control(owner, index, handle) if *owner == component => {
                let knot = &mut knots[*index];
                shift(if *handle == Handle::Incoming {
                    &mut knot.incoming
                } else {
                    &mut knot.outgoing
                });
                knot.linked = false;
            }
            _ => continue,
        }
        convert |= !matches!(target.geometry, Geometry::Path(_));
        target.geometry = Geometry::Path(knots);
    }
    convert
}

impl PathTool {
    pub(in crate::asset_viewer::graphic_editor) fn clear_selection(&mut self) {
        self.selection = Default::default();
        self.drag = None;
    }

    pub(super) fn component(&self, layer: usize) -> Option<usize> {
        self.selection
            .component
            .filter(|&(owner, _)| owner == layer)
            .map(|(_, i)| i)
    }

    pub(super) fn select_component(&mut self, layer: usize, index: usize) {
        self.selection.component = Some((layer, index));
        self.selection.anchors.clear();
    }
}

impl Editor {
    pub(in crate::asset_viewer::graphic_editor) fn selected_shape_to_layer(
        &mut self,
        cut: bool,
    ) -> bool {
        let index = self.active_layer;
        if self.tool != Tool::Path
            || self.path_tool.path_mode != 0
            || self.selected_layers.len() > 1
        {
            return false;
        }
        let Some(component) = self.path_tool.component(index) else {
            return false;
        };
        let Some(source) = self.layers.get(index) else {
            return false;
        };
        let Some(vector) = source.settings.vector.as_ref().filter(|v| v.fill.is_some()) else {
            return false;
        };
        let (selected, parts) = match &vector.geometry {
            Geometry::Compound(parts) => {
                let Some(part) = parts.get(component) else {
                    return false;
                };
                (&part.vector, Some(parts))
            }
            _ if component == 0 => (vector, None),
            _ => return false,
        };
        if self.mask_editing
            || self.path_tool.drag.is_some()
            || self.path_tool.conversion.is_some()
            || !source.visible
            || !self.ancestors_visible(index)
            || (cut && (parts.is_none() || !self.vector_unlocked(index)))
        {
            return true;
        }
        if !self.can_add_layers(&[index], 0) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return true;
        }
        let extracted = selected.clone();
        let origin = self.vector_origin(source.settings.vector_origin);
        let Some(tiles) = self.render_vector_tiles(&extracted, origin) else {
            return true;
        };
        let remaining = if let Some(parts) = parts.filter(|_| cut) {
            let mut rest = parts.clone();
            rest.remove(component);
            let remaining = if rest.len() == 1 {
                rest.remove(0).vector
            } else {
                // Each color run needs a filled base when its original base is cut.
                for run in rest.chunk_by_mut(|a, b| a.vector.fill == b.vector.fill) {
                    run[0].operation = PathOp::Combine;
                }
                Vector {
                    fill: rest[0].vector.fill,
                    geometry: Geometry::Compound(rest),
                    inverted: false,
                }
            };
            let Some(images) = self.render_vector_tiles(&remaining, origin) else {
                return true;
            };
            Some((remaining, images))
        } else {
            None
        };
        let mut settings = source.settings.clone();
        settings.vector = Some(extracted);
        settings.vector_origin = origin;
        settings.psd_layer_id = None;
        settings.lock_alpha = false;
        settings.lock_pixels = false;
        settings.lock_position = false;
        let layer = Layer {
            name: format!("{} {}", source.name, tr("副本")),
            tiles,
            settings,
            visible: true,
            locked: false,
            material: false,
        };
        self.record_layer_change();
        self.layers.insert(index + 1, layer);
        self.select_layer(index + 1);
        if let Some((vector, images)) = remaining {
            self.layers[index].settings.vector = Some(vector);
            self.layers[index].settings.vector_origin = origin;
            self.layers[index].tiles = images;
        }
        self.path_tool.select_component(index + 1, 0);
        true
    }

    fn path_image(&self, layer: usize, image: Rect, scale: f32) -> Rect {
        let current = self.vector_origin(None).unwrap_or_default();
        let [x, y] = self
            .vector_origin(self.layers[layer].settings.vector_origin)
            .unwrap_or(current);
        image.translate(
            egui::vec2(x as f32 - current[0] as f32, y as f32 - current[1] as f32) * scale,
        )
    }

    fn hit_path(
        &self,
        press: Pos2,
        image: Rect,
        scale: f32,
        direct: bool,
        dragging: bool,
    ) -> Option<(usize, usize, Option<(usize, Handle)>)> {
        let tolerance = 6. / scale;
        let active = self.active_layer;
        let visible_vector = |layer: usize| {
            self.layers
                .get(layer)
                .filter(|l| l.visible && self.ancestors_visible(layer))
                .and_then(|l| l.settings.vector.as_ref())
        };
        let point = |layer| ((press - self.path_image(layer, image, scale).min) / scale).to_pos2();
        let empty = Anchors::new();
        if let Some(source) = visible_vector(active) {
            let contours = source.contours();
            // Visible anchors stay reachable even if a different layer covers them.
            let anchors = if direct {
                &self.path_tool.selection.anchors
            } else {
                &empty
            };
            if let Some((part, knot, handle)) =
                anchor_hit(&contours, point(active), tolerance, anchors)
            {
                return Some((active, part, direct.then_some((knot, handle))));
            }
            // Starting a direct-selection box keeps its original layer.
            if direct && dragging {
                return None;
            }
        }
        for layer in (0..self.layers.len()).rev() {
            let Some(source) = visible_vector(layer) else {
                continue;
            };
            let contours = source.contours();
            if direct
                && layer != active
                && let Some((part, knot, handle)) =
                    anchor_hit(&contours, point(layer), tolerance, &empty)
            {
                return Some((layer, part, Some((knot, handle))));
            }
            if let Some(part) = component_hit(&contours, point(layer), tolerance) {
                return Some((layer, part, None));
            }
        }
        None
    }

    pub(super) fn path_canvas(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
        image: Rect,
        scale: f32,
        blocked: bool,
        outline: egui::Stroke,
    ) {
        if self.active_layer >= self.layers.len() {
            return;
        }
        let button = egui::PointerButton::Primary;
        let direct = self.path_tool.path_mode == 1;
        let dragging = response.drag_started_by(button);
        if !blocked
            && self.path_tool.conversion.is_none()
            && (dragging || response.clicked_by(button))
            && let Some(press) = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| response.interact_pointer_pos())
        {
            let shift = ui.input(|i| i.modifiers.shift);
            let hit = self.hit_path(press, image, scale, direct, dragging);
            if let Some((layer, _, _)) = hit
                && self.active_layer != layer
            {
                self.select_layer(layer);
            }
            let layer = self.active_layer;
            let start = ((press - self.path_image(layer, image, scale).min) / scale).to_pos2();
            let selection = &mut self.path_tool.selection;
            let action = match hit {
                Some((_, part, Some((knot, handle)))) => {
                    selection.component = Some((layer, part));
                    let id = (part, knot);
                    if handle != Handle::Anchor {
                        Some(Action::Control(part, knot, handle))
                    } else if shift && !dragging && selection.anchors.remove(&id) {
                        None
                    } else {
                        if !shift && !(dragging && selection.anchors.contains(&id)) {
                            selection.anchors.clear();
                        }
                        selection.anchors.insert(id);
                        Some(Action::Anchors)
                    }
                }
                Some((_, part, None)) if !direct => {
                    selection.component = Some((layer, part));
                    selection.anchors.clear();
                    Some(Action::Component(part))
                }
                _ => {
                    selection.component = hit.map(|(_, part, _)| (layer, part));
                    if !shift || !direct {
                        selection.anchors.clear();
                    }
                    direct.then(|| Action::Marquee(selection.anchors.clone()))
                }
            };
            if dragging && self.vector_unlocked(layer) {
                self.path_tool.drag = action.map(|action| Drag {
                    view: response.id,
                    layer,
                    start,
                    action,
                });
            }
        }
        let layer = self.active_layer;
        let Some(mut vector) = self
            .layers
            .get(layer)
            .filter(|l| l.visible && self.ancestors_visible(layer))
            .and_then(|l| l.settings.vector.clone())
        else {
            return;
        };
        let origin = self.path_image(layer, image, scale).min;
        let screen = |p: [f32; 2]| origin + egui::vec2(p[0], p[1]) * scale;
        let mut marquee = None;
        if let Some(drag) = self
            .path_tool
            .drag
            .as_ref()
            .filter(|d| d.view == response.id && d.layer == layer)
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let point = ((pointer - origin) / scale).to_pos2();
            let delta = point - drag.start;
            let mut convert = false;
            if let Action::Marquee(base) = &drag.action {
                let rect = Rect::from_two_pos(drag.start, point);
                self.path_tool.selection.anchors.clone_from(base);
                for (part, knots) in vector.contours().iter().enumerate() {
                    for (knot, _) in knots
                        .iter()
                        .enumerate()
                        .filter(|(_, k)| rect.contains(k.point.into()))
                    {
                        self.path_tool.selection.anchors.insert((part, knot));
                    }
                }
                let anchors = &self.path_tool.selection.anchors;
                self.path_tool.selection.component = anchors
                    .first()
                    .zip(anchors.last())
                    .filter(|(first, last)| first.0 == last.0)
                    .map(|(first, _)| (layer, first.0));
                marquee = Some(Rect::from_min_max(
                    screen(rect.min.into()),
                    screen(rect.max.into()),
                ));
            } else if delta != Vec2::ZERO {
                convert = move_points(
                    &mut vector,
                    &drag.action,
                    &self.path_tool.selection.anchors,
                    delta,
                );
            }
            if response.drag_stopped_by(button) {
                self.path_tool.drag = None;
                if marquee.is_none()
                    && delta != Vec2::ZERO
                    && self.vector_unlocked(layer)
                    && vector.valid()
                {
                    if convert {
                        self.path_tool.conversion = Some((layer, vector.clone()));
                    } else {
                        self.apply_vector(layer, vector.clone());
                    }
                }
            }
        }
        let contours = vector.contours();
        for knots in &contours {
            draw_path(painter, knots, screen, outline, true);
        }
        for (part, knots) in contours.iter().enumerate() {
            let whole = !direct && self.path_tool.component(layer) == Some(part);
            if !direct && !whole {
                continue;
            }
            for (index, k) in knots.iter().enumerate() {
                let selected = whole || self.path_tool.selection.anchors.contains(&(part, index));
                if direct && selected {
                    for handle in [k.incoming, k.outgoing]
                        .into_iter()
                        .filter(|p| *p != k.point)
                    {
                        painter.line_segment([screen(k.point), screen(handle)], outline);
                        painter.circle_filled(screen(handle), 3., outline.color);
                    }
                }
                painter.rect(
                    Rect::from_center_size(screen(k.point), Vec2::splat(6.)),
                    0.,
                    if selected {
                        outline.color
                    } else {
                        Color32::WHITE
                    },
                    egui::Stroke::new(1_f32, outline.color),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if let Some(rect) = marquee {
            painter.rect(
                rect,
                0.,
                outline.color.gamma_multiply(0.12),
                egui::Stroke::new(1_f32, outline.color),
                egui::StrokeKind::Inside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(min: [f32; 2], max: [f32; 2]) -> Vector {
        Vector {
            geometry: Geometry::Rectangle {
                min,
                max,
                radii: [0.; 4],
                angle: 0.,
            },
            fill: Some([25, 80, 140]),
            inverted: false,
        }
    }

    fn scene() -> (Editor, Vector, Vector) {
        let first = rectangle([28., 26.], [68., 70.]);
        let mut second = rectangle([54., 38.], [100., 82.]);
        if let Geometry::Rectangle { radii, .. } = &mut second.geometry {
            *radii = [4.; 4];
        }
        let mut editor = Editor::default();
        editor.init_image(&RgbaImage::new(128, 96));
        editor.add_vector(Vector::combine([(&first, None), (&second, None)], None).unwrap());
        editor.tool = Tool::Path;
        (editor, first, second)
    }

    #[test]
    fn component_copy_and_cut_keep_live_shapes_and_undo_together() {
        for cut in [false, true] {
            let (mut editor, first, second) = scene();
            let source = editor.active_layer;
            let original = editor.layers[source].settings.vector.clone();
            assert!(!editor.selected_shape_to_layer(cut));
            editor.path_tool.select_component(source, 1);
            editor.selection.mask = Some([(0, 0, 0)].into_iter().collect());
            let steps = editor.undo.len();
            assert!(editor.selected_shape_to_layer(cut));
            assert_eq!(editor.active_layer, source + 1);
            assert!(editor.layers[source + 1].settings.vector.as_ref() == Some(&second));
            assert_eq!(editor.pixels, second.raster(128, 96).unwrap());
            if cut {
                assert!(editor.layers[source].settings.vector.as_ref() == Some(&first));
                assert_eq!(
                    editor.layers[source].tiles[0],
                    first.raster(128, 96).unwrap()
                );
            } else {
                assert!(editor.layers[source].settings.vector == original);
            }
            assert_eq!(editor.undo.len(), steps + 1);
            editor.history(false);
            assert_eq!(editor.layers.len(), source + 1);
            assert!(editor.layers[source].settings.vector == original);
        }
    }

    fn pointer_frame(
        editor: &mut Editor,
        ctx: &egui::Context,
        position: [f32; 2],
        button: Option<bool>,
    ) {
        let pos = position.into();
        let mut events = vec![egui::Event::PointerMoved(pos)];
        if let Some(pressed) = button {
            events.push(egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = Rect::from_min_max(Pos2::ZERO, egui::pos2(128., 96.));
                    let response =
                        ui.interact(rect, ui.id().with("canvas"), egui::Sense::click_and_drag());
                    editor.vector_canvas(ui, &response, ui.painter(), rect, 1., false);
                });
            },
        );
    }

    fn gesture(editor: &mut Editor, ctx: &egui::Context, from: [f32; 2], to: [f32; 2]) {
        pointer_frame(editor, ctx, from, None);
        pointer_frame(editor, ctx, from, Some(true));
        pointer_frame(editor, ctx, to, None);
        pointer_frame(editor, ctx, to, Some(false));
    }

    #[test]
    fn direct_selection_click_marquee_and_drag_only_move_selected_points() {
        let (mut editor, first, second) = scene();
        editor.path_tool.set_mode(Tool::Path, 1);
        let ctx = egui::Context::default();
        let steps = editor.undo.len();
        gesture(&mut editor, &ctx, [28., 26.], [28., 26.]);
        assert_eq!(editor.path_tool.selection.anchors, [(0, 0)].into());
        gesture(&mut editor, &ctx, [12., 12.], [36., 75.]);
        assert_eq!(editor.path_tool.selection.anchors, [(0, 0), (0, 3)].into());
        assert_eq!(editor.undo.len(), steps);
        gesture(&mut editor, &ctx, [28., 26.], [36., 30.]);
        let (_, preview) = editor.path_tool.conversion.as_ref().unwrap();
        let Geometry::Compound(parts) = &preview.geometry else {
            panic!()
        };
        assert!(parts[1].vector == second);
        let anchors: Vec<_> = parts[0]
            .vector
            .knots()
            .into_iter()
            .map(|k| k.point)
            .collect();
        assert_eq!(anchors, [[36., 30.], [68., 26.], [68., 70.], [36., 74.]]);
        let Geometry::Compound(original) = &editor.layers[editor.active_layer]
            .settings
            .vector
            .as_ref()
            .unwrap()
            .geometry
        else {
            panic!()
        };
        assert!(original[0].vector == first);
        assert_eq!(editor.undo.len(), steps);
    }

    #[test]
    fn path_selection_drag_from_anchor_translates_only_its_rectangle() {
        let (mut editor, first, second) = scene();
        let ctx = egui::Context::default();
        let steps = editor.undo.len();
        gesture(&mut editor, &ctx, [58., 38.], [65., 43.]);
        let vector = editor.layers[editor.active_layer]
            .settings
            .vector
            .as_ref()
            .unwrap();
        let Geometry::Compound(parts) = &vector.geometry else {
            panic!()
        };
        assert!(parts[0].vector == first);
        assert!(
            parts[1].vector == second.transformed(|p| p + egui::vec2(7., 5.), Vec2::splat(1.), 0.)
        );
        assert!(editor.path_tool.conversion.is_none());
        assert_eq!(editor.undo.len(), steps + 1);
    }
}
