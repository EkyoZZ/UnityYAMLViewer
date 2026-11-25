//! Hierarchical Tile groups. Pass-through coverage interpolates premultiplied results.
//! Reference: PhotoCraft crates/compose/src/composite.rs (group isolation semantics).
//! Independent implementation for this editor's per-cell storage.
use super::*;
pub(super) fn valid_tree<'a>(
    items: impl Iterator<Item = (&'a layer_properties::Properties, bool)>,
) -> bool {
    let nodes: Vec<_> = items.collect();
    let mut ids = std::collections::HashSet::new();
    for (p, base) in &nodes {
        if *base && (p.parent != 0 || p.group_id != 0) {
            return false;
        }
        if p.group_id != 0 && (!ids.insert(p.group_id) || p.adjustment.is_some()) {
            return false;
        }
    }
    for (p, _) in &nodes {
        let mut parent = p.parent;
        let mut seen = std::collections::HashSet::new();
        if p.group_id != 0 {
            seen.insert(p.group_id);
        }
        while parent != 0 {
            if !seen.insert(parent) {
                return false;
            }
            let Some((q, _)) = nodes.iter().find(|(q, _)| q.group_id == parent) else {
                return false;
            };
            parent = q.parent;
        }
    }
    true
}
impl Editor {
    fn adjustment_backdrop(&self, parent: u64, tile: usize, depth: usize) -> RgbaImage {
        let mut out = RgbaImage::new(self.pixels.width(), self.pixels.height());
        if depth > 32 {
            return out;
        }
        if let Some(group) = self.group_index(parent) {
            let p = &self.layers[group].settings;
            if p.pass_through && !p.clipped && p.fill == 100 && !p.has_effects() {
                out = self.adjustment_backdrop(p.parent, tile, depth + 1);
                self.composite_sibling_prefix(p.parent, tile, &mut out, depth, group);
            }
        }
        out
    }
    pub(super) fn adjustment_input(&self, index: usize, tile: usize) -> RgbaImage {
        let p = &self.layers[index].settings;
        if p.clipped {
            if let Some(base) = self.clipping_base(index) {
                let mut settings = self.layers[base].settings.clone();
                settings.blend = layer_properties::Blend::Normal;
                settings.opacity = 100;
                settings.pass_through = false;
                let mut out = RgbaImage::new(self.pixels.width(), self.pixels.height());
                self.composite_layer_entry(base, tile, &mut out, 0, Some(&settings));
                let alpha: Vec<_> = out.pixels().map(|p| p[3]).collect();
                for i in base + 1..index {
                    if self.layers[i].settings.parent != p.parent || !self.layers[i].visible {
                        continue;
                    }
                    for (p, a) in out.pixels_mut().zip(&alpha) {
                        p[3] = if *a == 0 { 0 } else { 255 };
                    }
                    self.composite_layer_entry(i, tile, &mut out, 0, None);
                    for (p, a) in out.pixels_mut().zip(&alpha) {
                        p[3] = *a;
                    }
                }
                return out;
            }
        }
        let mut out = self.adjustment_backdrop(p.parent, tile, 0);
        self.composite_sibling_prefix(p.parent, tile, &mut out, 0, index);
        out
    }

    pub(super) fn request_layer_action(&mut self, clear: bool, index: usize) {
        if index >= self.layers.len() {
            return;
        }
        self.finish_stroke();
        let selected = if index == self.active_layer && self.selected_layers.contains(&index) {
            self.selected_layers.clone()
        } else {
            vec![index]
        };
        if self.layers[index].material || self.layers[index].locked || self.ancestor_locked(index) {
            return;
        }
        if clear
            && !self.mask_editing
            && selected
                .iter()
                .any(|i| self.layers[*i].settings.text.is_some())
        {
            return;
        }
        if !clear {
            let revision = self.revision;
            self.delete_selected_layers(index);
            if self.revision != revision {
                self.layer_detail = None;
                self.invalidate_textures();
            }
            return;
        }
        let mask = self.mask_editing && index == self.active_layer;
        if index != self.active_layer {
            self.select_layer(index);
        }
        self.mask_editing = mask;
        self.clear_selected_layer();
        self.invalidate_textures();
    }
    pub(super) fn layer_insert_position(&self) -> (u64, usize) {
        let layer = &self.layers[self.active_layer];
        if layer.settings.group_id != 0 {
            let parent = layer.settings.group_id;
            let index = self
                .layers
                .iter()
                .enumerate()
                .filter(|(_, l)| l.settings.parent == parent)
                .map(|(i, _)| i + 1)
                .max()
                .unwrap_or(self.active_layer + 1);
            (parent, index)
        } else {
            (layer.settings.parent, self.active_layer + 1)
        }
    }
    fn drag_layer_roots(&self, from: usize) -> Vec<usize> {
        if self.selected_layers.contains(&from) {
            self.selected_roots()
        } else {
            vec![from]
        }
    }
    pub(super) fn valid_layer_drop(&self, from: usize, parent: u64, anchor: Option<usize>) -> bool {
        if from >= self.layers.len() || anchor.is_some_and(|i| i >= self.layers.len()) {
            return false;
        }
        let target = if parent == 0 {
            None
        } else {
            let Some(i) = self
                .layers
                .iter()
                .position(|l| l.settings.group_id == parent)
            else {
                return false;
            };
            Some(i)
        };
        if target.is_some_and(|i| {
            self.layers[i].locked
                || self.ancestor_locked(i)
                || self.layers[i].settings.lock_position
                || self.ancestor_move_locked(i)
        }) {
            return false;
        }
        if anchor.is_some_and(|i| self.layers[i].settings.parent != parent) {
            return false;
        }
        let roots = self.drag_layer_roots(from);
        for root in roots {
            if self.layers[root].material
                || self.layers[root].locked
                || self.ancestor_locked(root)
                || self.layers[root].settings.lock_position
                || self.ancestor_move_locked(root)
            {
                return false;
            }
            if target.is_some_and(|i| self.group_contains(root, i))
                || anchor.is_some_and(|i| self.group_contains(root, i))
            {
                return false;
            }
        }
        true
    }
    pub(super) fn drop_layer_objects(
        &mut self,
        from: usize,
        parent: u64,
        anchor: Option<usize>,
        above: bool,
    ) {
        if !self.valid_layer_drop(from, parent, anchor) {
            return;
        }
        let mut roots = self.drag_layer_roots(from);
        roots.sort_unstable();
        let mut order: Vec<_> = (0..self.layers.len())
            .filter(|i| !roots.contains(i))
            .collect();
        let insert = if let Some(anchor) = anchor {
            order.iter().position(|i| *i == anchor).unwrap() + usize::from(above)
        } else {
            order
                .iter()
                .rposition(|i| self.layers[*i].settings.parent == parent)
                .map(|i| i + 1)
                .unwrap_or(order.len())
        };
        let insert = if parent == 0 && self.layers.iter().any(|l| l.material) {
            insert.max(1)
        } else {
            insert
        };
        order.splice(insert..insert, roots.iter().copied());
        let was_active = self.active_layer;
        let old_active = if self.selected_layers.contains(&from) {
            self.active_layer
        } else {
            from
        };
        let old_selected = if self.selected_layers.contains(&from) {
            self.selected_layers.clone()
        } else {
            vec![from]
        };
        let old_anchor = self.layer_anchor;
        self.record_layer_change();
        for &i in &roots {
            if self.layers[i].settings.parent != parent {
                self.layers[i].settings.clipped = false;
            }
            self.layers[i].settings.parent = parent;
        }
        if let Some(target) = self
            .layers
            .iter_mut()
            .find(|l| l.settings.group_id == parent && parent != 0)
        {
            target.settings.collapsed = false;
        }
        let mut slots: Vec<_> = self.layers.drain(..).map(Some).collect();
        self.layers = order.iter().map(|i| slots[*i].take().unwrap()).collect();
        self.active_layer = order.iter().position(|i| *i == old_active).unwrap();
        self.selected_layers = old_selected
            .iter()
            .filter_map(|old| order.iter().position(|i| i == old))
            .collect();
        self.layer_anchor = order
            .iter()
            .position(|i| *i == old_anchor)
            .unwrap_or(self.active_layer);
        self.cleanup_single_links();
        self.mask_editing = self.mask_editing
            && old_active == was_active
            && self.layers[self.active_layer].settings.mask.is_some();
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
        self.invalidate_textures();
    }

    pub(super) fn clipping_base(&self, index: usize) -> Option<usize> {
        let parent = self.layers[index].settings.parent;
        self.layers[..index]
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| l.settings.parent == parent)
            .find(|(_, l)| !l.settings.clipped)
            .map(|(i, _)| i)
    }
    pub(super) fn toggle_clipping(&mut self, index: usize) {
        if index >= self.layers.len()
            || self.layers[index].material
            || self.layers[index].locked
            || self.ancestor_locked(index)
        {
            return;
        }
        if !self.layers[index].settings.clipped
            && self
                .clipping_base(index)
                .is_none_or(|base| self.layers[base].settings.adjustment.is_some())
        {
            return;
        }
        self.record_layer_change();
        self.layers[index].settings.clipped = !self.layers[index].settings.clipped;
        self.cleanup_single_links();
    }

    pub(super) fn move_hit_target(
        &self,
        point: Pos2,
        layout: bool,
        infinite: bool,
    ) -> Option<usize> {
        let (tile, x, y) = if infinite {
            self.plane_hit(point.x.floor() as i32, point.y.floor() as i32, layout)
        } else {
            self.finite_plane_hit(point.x.floor() as i32, point.y.floor() as i32, layout)
        }?;
        self.move_hit_stack(0, tile, x, y, 0)
    }
    fn move_hit_stack(
        &self,
        parent: u64,
        tile: usize,
        x: u32,
        y: u32,
        depth: usize,
    ) -> Option<usize> {
        if depth > 32 {
            return None;
        }
        let offset = (y * self.pixels.width() + x) as usize;
        for (index, layer) in self
            .layers
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, l)| l.settings.parent == parent)
        {
            let p = &layer.settings;
            if !layer.visible
                || layer.locked
                || layer.material
                || p.opacity == 0
                || p.mask_value(tile, offset) == 0.
            {
                continue;
            }
            if p.clipped {
                if let Some(base) = self.clipping_base(index) {
                    if !self.layers[base].visible {
                        continue;
                    }
                    let mut coverage = RgbaImage::new(self.pixels.width(), self.pixels.height());
                    self.composite_layer_entry(base, tile, &mut coverage, depth, None);
                    if coverage.get_pixel(x, y)[3] == 0 {
                        continue;
                    }
                }
            }
            if p.group_id != 0 {
                if p.fill == 0 && !p.has_effects() {
                    continue;
                }
                if let Some(hit) = self.move_hit_stack(p.group_id, tile, x, y, depth + 1) {
                    return Some(if self.layer_move_group { index } else { hit });
                }
                continue;
            }
            if p.adjustment.is_some() {
                continue;
            }
            let source = if index == self.active_layer {
                if tile == self.selected {
                    &self.pixels
                } else {
                    &self.tiles[tile]
                }
            } else {
                &layer.tiles[tile]
            };
            let alpha = if p.has_effects() {
                let mut image = RgbaImage::new(source.width(), source.height());
                p.composite(&mut image, source, tile, self.fx_light);
                image.get_pixel(x, y)[3]
            } else if p.fill > 0 {
                source.get_pixel(x, y)[3]
            } else {
                0
            };
            if alpha > 0 {
                return Some(index);
            }
        }
        None
    }

    pub(super) fn cleanup_single_links(&mut self) {
        for i in 0..self.layers.len() {
            if self.layers[i].settings.clipped
                && self
                    .clipping_base(i)
                    .is_none_or(|base| self.layers[base].settings.adjustment.is_some())
            {
                self.layers[i].settings.clipped = false;
            }
        }
        let mut counts = std::collections::HashMap::new();
        for layer in &self.layers {
            if layer.settings.link_id != 0 {
                *counts.entry(layer.settings.link_id).or_insert(0usize) += 1;
            }
        }
        for layer in &mut self.layers {
            if layer.settings.link_id != 0
                && counts.get(&layer.settings.link_id).copied().unwrap_or(0) < 2
            {
                layer.settings.link_id = 0;
            }
        }
    }
    fn group_index(&self, id: u64) -> Option<usize> {
        if id == 0 {
            None
        } else {
            self.layers.iter().position(|l| l.settings.group_id == id)
        }
    }
    fn ancestors(&self, index: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut id = self.layers[index].settings.parent;
        while let Some(i) = self.group_index(id) {
            if out.contains(&i) || out.len() >= 32 {
                break;
            }
            out.push(i);
            id = self.layers[i].settings.parent;
        }
        out
    }
    pub(super) fn ancestor_paint_locked(&self, index: usize) -> bool {
        self.ancestors(index)
            .iter()
            .any(|i| self.layers[*i].settings.lock_pixels)
    }
    pub(super) fn ancestor_move_locked(&self, index: usize) -> bool {
        self.ancestors(index)
            .iter()
            .any(|i| self.layers[*i].settings.lock_position)
    }
    pub(super) fn ancestor_locked(&self, index: usize) -> bool {
        self.ancestors(index).iter().any(|i| self.layers[*i].locked)
    }
    pub(super) fn ancestors_visible(&self, index: usize) -> bool {
        self.ancestors(index)
            .iter()
            .all(|i| self.layers[*i].visible)
    }
    pub(super) fn group_depth(&self, index: usize) -> usize {
        self.ancestors(index).len()
    }
    fn is_descendant(&self, index: usize, group: usize) -> bool {
        self.ancestors(index).contains(&group)
    }
    pub(super) fn group_display_order(&self) -> Vec<usize> {
        fn visit(e: &Editor, id: u64, out: &mut Vec<usize>, depth: usize) {
            if depth > 32 {
                return;
            }
            for (i, l) in e
                .layers
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, l)| l.settings.parent == id)
            {
                out.push(i);
                if l.settings.group_id != 0 && !l.settings.collapsed {
                    visit(e, l.settings.group_id, out, depth + 1);
                }
            }
        }
        let mut out = Vec::new();
        visit(self, 0, &mut out, 0);
        out
    }
    fn direct_move_members(&self, index: usize, group_mode: bool) -> Vec<usize> {
        let root = if self.layers[index].settings.group_id != 0 {
            index
        } else if group_mode {
            self.group_index(self.layers[index].settings.parent)
                .unwrap_or(index)
        } else {
            index
        };
        (0..self.layers.len())
            .filter(|i| *i == root || self.is_descendant(*i, root))
            .collect()
    }
    pub(super) fn selected_roots(&self) -> Vec<usize> {
        let selected: Vec<_> = if self.selected_layers.contains(&self.active_layer) {
            self.selected_layers
                .iter()
                .copied()
                .filter(|i| *i < self.layers.len())
                .collect()
        } else {
            vec![self.active_layer]
        };
        selected
            .iter()
            .copied()
            .filter(|i| {
                !selected
                    .iter()
                    .any(|j| i != j && self.is_descendant(*i, *j))
            })
            .collect()
    }
    pub(super) fn move_members(&self, index: usize, group_mode: bool) -> Vec<usize> {
        let roots = if index == self.active_layer {
            self.selected_roots()
        } else {
            vec![index]
        };
        let mut members = std::collections::BTreeSet::new();
        for root in roots {
            members.extend(self.direct_move_members(root, group_mode));
        }
        loop {
            let old = members.len();
            let links: Vec<_> = members
                .iter()
                .map(|i| self.layers[*i].settings.link_id)
                .filter(|id| *id != 0)
                .collect();
            for (i, l) in self.layers.iter().enumerate() {
                if links.contains(&l.settings.link_id) {
                    members.extend(self.direct_move_members(i, false));
                }
            }
            if members.len() == old {
                break;
            }
        }
        members.into_iter().collect()
    }
    pub(super) fn select_layer_click(&mut self, index: usize, modifiers: egui::Modifiers) {
        let old = self.selected_layers.clone();
        let anchor = self.layer_anchor.min(self.layers.len() - 1);
        self.select_layer(index);
        if modifiers.shift {
            let order = self.group_display_order();
            if let (Some(a), Some(b)) = (
                order.iter().position(|i| *i == anchor),
                order.iter().position(|i| *i == index),
            ) {
                self.selected_layers = order[a.min(b)..=a.max(b)].to_vec();
            }
            self.layer_anchor = anchor;
        } else if modifiers.ctrl || modifiers.command {
            let mut selected = old;
            if selected.contains(&index) {
                selected.retain(|i| *i != index);
                if let Some(&active) = selected.last() {
                    self.select_layer(active);
                } else {
                    selected.push(index);
                }
            } else {
                selected.push(index);
            }
            self.selected_layers = selected;
            self.layer_anchor = anchor;
        }
    }
    fn link_selection(&mut self, unlink: bool) {
        let members = self.selected_roots();
        if members
            .iter()
            .any(|i| self.layers[*i].material || self.layers[*i].locked || self.ancestor_locked(*i))
        {
            return;
        }
        self.record_layer_change();
        if unlink {
            let links: Vec<_> = members
                .iter()
                .map(|i| self.layers[*i].settings.link_id)
                .filter(|id| *id != 0)
                .collect();
            for l in &mut self.layers {
                if links.contains(&l.settings.link_id) {
                    l.settings.link_id = 0;
                }
            }
        } else if members.len() > 1 {
            let previous: Vec<_> = members
                .iter()
                .map(|i| self.layers[*i].settings.link_id)
                .filter(|id| *id != 0)
                .collect();
            let id = (1..)
                .find(|id| !self.layers.iter().any(|l| l.settings.link_id == *id))
                .unwrap();
            for (i, l) in self.layers.iter_mut().enumerate() {
                if members.contains(&i) || previous.contains(&l.settings.link_id) {
                    l.settings.link_id = id;
                }
            }
        }
    }
    pub(super) fn composite_group_stack(
        &self,
        parent: u64,
        tile: usize,
        out: &mut RgbaImage,
        depth: usize,
    ) {
        self.composite_sibling_prefix(parent, tile, out, depth, usize::MAX);
    }
    fn composite_sibling_prefix(
        &self,
        parent: u64,
        tile: usize,
        out: &mut RgbaImage,
        depth: usize,
        end: usize,
    ) {
        if depth > 32 {
            return;
        }
        let siblings: Vec<_> = self
            .layers
            .iter()
            .enumerate()
            .filter(|(i, l)| *i < end && l.settings.parent == parent)
            .map(|(i, _)| i)
            .collect();
        let mut cursor = 0;
        while cursor < siblings.len() {
            let base = siblings[cursor];
            let mut end = cursor + 1;
            while end < siblings.len() && self.layers[siblings[end]].settings.clipped {
                end += 1;
            }
            if self.layers[base].visible {
                if end == cursor + 1 || self.layers[base].settings.adjustment.is_some() {
                    self.composite_layer_entry(base, tile, out, depth, None);
                } else {
                    let properties = &self
                        .transform
                        .as_ref()
                        .filter(|t| t.index == base)
                        .and_then(|t| t.preview.as_ref())
                        .unwrap_or(&self.layers[base])
                        .settings;
                    let mut settings = properties.clone();
                    settings.blend = layer_properties::Blend::Normal;
                    settings.opacity = 100;
                    settings.pass_through = false;
                    let mut isolated = RgbaImage::new(out.width(), out.height());
                    self.composite_layer_entry(base, tile, &mut isolated, depth, Some(&settings));
                    let coverage: Vec<_> = isolated.pixels().map(|p| p[3]).collect();
                    for &index in &siblings[cursor + 1..end] {
                        if !self.layers[index].visible {
                            continue;
                        }
                        for (p, a) in isolated.pixels_mut().zip(&coverage) {
                            p[3] = if *a == 0 { 0 } else { 255 };
                        }
                        self.composite_layer_entry(index, tile, &mut isolated, depth, None);
                        for (p, a) in isolated.pixels_mut().zip(&coverage) {
                            p[3] = *a;
                            if *a == 0 {
                                *p = Rgba([0; 4]);
                            }
                        }
                    }
                    let mut final_settings = layer_properties::Properties::default();
                    final_settings.blend = properties.blend;
                    final_settings.opacity = properties.opacity;
                    final_settings.composite(out, &isolated, tile, self.fx_light);
                }
            }
            cursor = end;
        }
    }
    fn composite_layer_entry(
        &self,
        index: usize,
        tile: usize,
        out: &mut RgbaImage,
        depth: usize,
        override_settings: Option<&layer_properties::Properties>,
    ) {
        let draft = self
            .transform
            .as_ref()
            .filter(|t| t.index == index)
            .and_then(|t| t.preview.as_ref());
        let layer = draft.unwrap_or(&self.layers[index]);
        let p = override_settings.unwrap_or(&layer.settings);
        if p.group_id != 0 {
            if p.pass_through && p.fill == 100 && !p.has_effects() {
                if p.opacity == 100 && (p.mask.is_none() || !p.mask_enabled) {
                    self.composite_group_stack(p.group_id, tile, out, depth + 1);
                    return;
                }
                let before = out.clone();
                self.composite_group_stack(p.group_id, tile, out, depth + 1);
                for (i, (dst, old)) in out.pixels_mut().zip(before.pixels()).enumerate() {
                    let k = p.opacity as f32 / 100. * p.mask_value(tile, i);
                    *dst = mix_group(*old, *dst, k);
                }
            } else {
                let mut group = RgbaImage::new(out.width(), out.height());
                self.composite_group_stack(p.group_id, tile, &mut group, depth + 1);
                if p.has_effects() {
                    let (w, h) = group.dimensions();
                    for (x, y, pixel) in group.enumerate_pixels_mut() {
                        if !self.inside_tile(x as i32, y as i32, w, h) {
                            *pixel = Rgba([0; 4]);
                        }
                    }
                }
                let mut settings = p.clone();
                if settings.pass_through {
                    settings.blend = layer_properties::Blend::Normal;
                }
                settings.composite(out, &group, tile, self.fx_light);
            }
            return;
        }
        let source = if draft.is_some() {
            &layer.tiles[tile]
        } else if index == self.active_layer {
            if tile == self.selected {
                &self.pixels
            } else {
                &self.tiles[tile]
            }
        } else {
            &layer.tiles[tile]
        };
        if p.has_effects() {
            let mut clipped = source.clone();
            let (w, h) = clipped.dimensions();
            for (x, y, pixel) in clipped.enumerate_pixels_mut() {
                if !self.inside_tile(x as i32, y as i32, w, h) {
                    *pixel = Rgba([0; 4]);
                }
            }
            p.composite(out, &clipped, tile, self.fx_light);
        } else {
            p.composite(out, source, tile, self.fx_light);
        }
    }
    fn next_group_id(&self) -> u64 {
        (1..).find(|id| self.group_index(*id).is_none()).unwrap()
    }
    pub(super) fn can_add_layers(&self, indices: &[usize], extra: usize) -> bool {
        let bytes = self
            .layers
            .iter()
            .map(|l| l.settings.bytes() + l.tiles.iter().map(|t| t.as_raw().len()).sum::<usize>())
            .sum::<usize>();
        let added = indices
            .iter()
            .map(|i| {
                let l = &self.layers[*i];
                l.settings.bytes() + l.tiles.iter().map(|t| t.as_raw().len()).sum::<usize>()
            })
            .sum::<usize>()
            + extra;
        self.layers.len() + indices.len() + usize::from(extra > 0) <= 32
            && bytes.saturating_add(added) <= self.layer_budget()
    }
    fn create_group(&mut self) {
        let i = self.active_layer;
        if self.layers[i].material || self.layers[i].locked || self.ancestor_locked(i) {
            return;
        }
        let bytes = self.tiles.iter().map(|t| t.as_raw().len()).sum();
        if !self.can_add_layers(&[], bytes) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        let selected = self.selected_roots();
        if selected
            .iter()
            .any(|j| self.layers[*j].material || self.layers[*j].locked || self.ancestor_locked(*j))
        {
            return;
        }
        self.record_layer_change();
        let id = self.next_group_id();
        let parent = if selected
            .iter()
            .all(|j| self.layers[*j].settings.parent == self.layers[i].settings.parent)
        {
            self.layers[i].settings.parent
        } else {
            0
        };
        let mut settings = layer_properties::Properties::default();
        settings.group_id = id;
        settings.parent = parent;
        for j in selected {
            self.layers[j].settings.parent = id;
        }
        let tiles = self
            .tiles
            .iter()
            .map(|t| RgbaImage::new(t.width(), t.height()))
            .collect();
        self.layers.insert(
            i + 1,
            Layer {
                settings,
                material: false,
                name: format!("{} {id}", tr("组")),
                visible: true,
                locked: false,
                tiles,
            },
        );
        self.active_layer = i + 1;
        self.selected_layers = vec![self.active_layer];
        self.mask_editing = false;
        self.tiles = self.layers[i + 1].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
    }
    fn ungroup(&mut self, index: usize) {
        if self.layers[index].locked || self.ancestor_locked(index) {
            return;
        }
        let id = self.layers[index].settings.group_id;
        if id == 0 {
            return;
        }
        self.record_layer_change();
        let parent = self.layers[index].settings.parent;
        for l in &mut self.layers {
            if l.settings.parent == id {
                l.settings.parent = parent;
            }
        }
        self.layers.remove(index);
        self.cleanup_single_links();
        self.ensure_layer_after_removal();
        self.active_layer = index.saturating_sub(1).min(self.layers.len() - 1);
        self.selected_layers = vec![self.active_layer];
        self.mask_editing = false;
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
    }
    pub(super) fn delete_selected_layers(&mut self, index: usize) {
        let roots = if index == self.active_layer {
            self.selected_roots()
        } else {
            vec![index]
        };
        if roots.len() == 1 {
            self.delete_layer(roots[0]);
            return;
        }
        let mut members = std::collections::BTreeSet::new();
        for root in roots {
            members.extend(self.direct_move_members(root, false));
        }
        if members
            .iter()
            .any(|i| self.layers[*i].material || self.layers[*i].locked || self.ancestor_locked(*i))
        {
            self.message = tr("选中对象包含受保护图层，无法删除。").into();
            return;
        }
        self.record_layer_change();
        for i in members.into_iter().rev() {
            self.layers.remove(i);
        }
        self.cleanup_single_links();
        self.ensure_layer_after_removal();
        self.active_layer = 0;
        self.selected_layers = vec![0];
        self.layer_anchor = 0;
        self.mask_editing = false;
        self.tiles = self.layers[0].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
    }
    pub(super) fn duplicate_selected_layers(&mut self, index: usize, all_cells: bool) {
        let mut roots = if index == self.active_layer {
            self.selected_roots()
        } else {
            vec![index]
        };
        if roots.len() == 1 {
            self.duplicate_layer(roots[0], all_cells);
            return;
        }
        let members: Vec<_> = roots
            .iter()
            .flat_map(|i| self.direct_move_members(*i, false))
            .collect();
        if !self.can_add_layers(&members, 0) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        self.finish_stroke();
        let before = self.snapshot();
        let undo_len = self.undo.len();
        roots.sort_unstable();
        for root in roots.into_iter().rev() {
            self.duplicate_layer(root, all_cells);
        }
        self.undo.truncate(undo_len);
        self.undo.push(before);
        self.redo.clear();
        self.trim_history();
    }
    pub(super) fn delete_group(&mut self, index: usize) {
        let members = self.direct_move_members(index, false);
        if members
            .iter()
            .any(|i| self.layers[*i].locked || self.layers[*i].material)
        {
            self.message = tr("组内存在锁定图层，无法删除组。").into();
            return;
        }
        self.record_layer_change();
        for i in members.into_iter().rev() {
            self.layers.remove(i);
        }
        self.cleanup_single_links();
        self.ensure_layer_after_removal();
        self.active_layer = 0;
        self.selected_layers = vec![0];
        self.mask_editing = false;
        self.tiles = self.layers[0].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
    }
    pub(super) fn duplicate_group(&mut self, index: usize) {
        self.finish_stroke();
        self.sync_layer();
        let members = self.direct_move_members(index, false);
        if !self.can_add_layers(&members, 0) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return;
        }
        self.record_layer_change();
        let mut next = self.next_group_id();
        let mut mapping = std::collections::HashMap::new();
        for i in &members {
            let id = self.layers[*i].settings.group_id;
            if id != 0 {
                while self.group_index(next).is_some() || mapping.values().any(|id| *id == next) {
                    next += 1;
                }
                mapping.insert(id, next);
                next += 1;
            }
        }
        let mut copies = Vec::new();
        let root_id = mapping[&self.layers[index].settings.group_id];
        let mut link_map = std::collections::HashMap::new();
        let mut next_link = 1;
        for i in &members {
            let id = self.layers[*i].settings.link_id;
            if id != 0 && !link_map.contains_key(&id) {
                while self.layers.iter().any(|l| l.settings.link_id == next_link)
                    || link_map.values().any(|id| *id == next_link)
                {
                    next_link += 1;
                }
                link_map.insert(id, next_link);
                next_link += 1;
            }
        }
        for i in members {
            let mut l = self.layers[i].clone();
            if let Some(id) = link_map.get(&l.settings.link_id) {
                l.settings.link_id = *id;
            }
            if let Some(id) = mapping.get(&l.settings.group_id) {
                l.settings.group_id = *id;
            }
            if let Some(id) = mapping.get(&l.settings.parent) {
                l.settings.parent = *id;
            }
            l.locked = false;
            l.settings.lock_alpha = false;
            l.settings.lock_pixels = false;
            l.settings.lock_position = false;
            if i == index {
                l.name = format!("{} {}", l.name, tr("副本"));
            }
            copies.push(l);
        }
        let at = index + 1;
        self.layers.splice(at..at, copies);
        self.active_layer = self.group_index(root_id).unwrap();
        self.selected_layers = vec![self.active_layer];
        self.mask_editing = false;
        self.tiles = self.layers[self.active_layer].tiles.clone();
        self.pixels = self.tiles[self.selected].clone();
    }
    pub(super) fn group_contains(&self, root: usize, index: usize) -> bool {
        root == index || self.is_descendant(index, root)
    }
    pub(super) fn insert_empty_layer(&mut self, record: bool) -> bool {
        let bytes = self.tiles.iter().map(|t| t.as_raw().len()).sum();
        if !self.can_add_layers(&[], bytes) {
            self.message = tr("图层数量或内存已达上限，无法导入新图层。").into();
            return false;
        }
        if record {
            self.record_layer_change();
        }
        let mut settings = layer_properties::Properties::default();
        let (parent, insertion) = self.layer_insert_position();
        settings.parent = parent;
        if let Some(group) = self
            .layers
            .iter_mut()
            .find(|l| parent != 0 && l.settings.group_id == parent)
        {
            group.settings.collapsed = false;
        }
        let tiles = self
            .tiles
            .iter()
            .map(|t| RgbaImage::new(t.width(), t.height()))
            .collect();
        self.layers.insert(
            insertion,
            Layer {
                settings,
                material: false,
                name: format!("{} {}", tr("图层"), self.layers.len()),
                visible: true,
                locked: false,
                tiles,
            },
        );
        if insertion <= self.active_layer {
            self.active_layer += 1;
        }
        self.select_layer(insertion);
        true
    }
    pub(super) fn group_controls(&mut self, ui: &mut egui::Ui) {
        let Some((action, target)) = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<(u8, u64)>(egui::Id::new("tile_layer_panel_action")))
        else {
            return;
        };
        let index = self.active_layer;
        match action {
            1 => self.link_selection(false),
            2 => self.link_selection(true),
            3 => self.create_group(),
            4 => self.ungroup(index),
            5 => {
                let roots = self.selected_roots();
                if roots.iter().all(|i| {
                    !self.layers[*i].material
                        && !self.layers[*i].locked
                        && !self.ancestor_locked(*i)
                }) && self
                    .group_index(target)
                    .is_none_or(|to| !roots.iter().any(|root| self.group_contains(*root, to)))
                {
                    self.record_layer_change();
                    for i in roots {
                        self.layers[i].settings.parent = target;
                    }
                }
            }
            6 => {
                self.insert_empty_layer(true);
            }
            7 => self.request_layer_action(false, index),
            8 => {
                if !self.mask_editing {
                    self.request_layer_action(true, index);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn mix_group(before: Rgba<u8>, after: Rgba<u8>, k: f32) -> Rgba<u8> {
    if k == 1. {
        return after;
    }
    if k == 0. {
        return before;
    }
    let wa = before[3] as f32 * (1. - k);
    let wb = after[3] as f32 * k;
    let a = wa + wb;
    if a <= 0. {
        return Rgba([0; 4]);
    }
    Rgba([
        ((before[0] as f32 * wa + after[0] as f32 * wb) / a).round() as u8,
        ((before[1] as f32 * wa + after[1] as f32 * wb) / a).round() as u8,
        ((before[2] as f32 * wa + after[2] as f32 * wb) / a).round() as u8,
        a.round() as u8,
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_into_out_of_group_preserves_tree_pixels_and_undo() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(4, 4));
        let child = e.active_layer;
        e.write_pixel(0, 1, 1, Rgba([80, 100, 120, 255]));
        e.create_group();
        let group_id = e.layers[e.active_layer].settings.group_id;
        let group = e.active_layer;
        e.select_layer(child);
        e.duplicate_layer(child, true);
        let copy = e.active_layer;
        e.drop_layer_objects(copy, 0, None, true);
        let copy = e.active_layer;
        assert_eq!(e.layers[copy].settings.parent, 0);
        let pixel = e.pixels.clone();
        e.drop_layer_objects(copy, group_id, None, true);
        assert_eq!(e.layers[e.active_layer].settings.parent, group_id);
        assert_eq!(e.pixels, pixel);
        e.history(false);
        assert_eq!(e.layers[e.active_layer].settings.parent, 0);
        let group = e
            .layers
            .iter()
            .position(|l| l.settings.group_id == group_id)
            .unwrap_or(group);
        assert!(!e.valid_layer_drop(group, group_id, None));
        assert!(valid_tree(
            e.layers.iter().map(|l| (&l.settings, l.material))
        ));
    }
    #[test]
    fn clipping_adjustment_chain_does_not_modify_backdrop_outside_base() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 255])));
        let base = e.active_layer;
        e.write_pixel(0, 1, 1, Rgba([100, 100, 100, 128]));
        e.sync_layer();
        for brightness in [10, 20] {
            let mut layer = e.layers[base].clone();
            layer.settings = layer_properties::Properties::default();
            layer.settings.clipped = true;
            layer.settings.adjustment = Some(layer_properties::Adjustment::Brightness {
                brightness,
                contrast: 0,
            });
            e.layers.push(layer);
        }
        let image = e.composite(0);
        assert_eq!(image.get_pixel(0, 0).0, [10, 20, 30, 255]);
        assert!(image.get_pixel(1, 1)[0] > 80);
        e.layers[base].visible = false;
        assert_eq!(e.composite(0).get_pixel(1, 1).0, [10, 20, 30, 255]);
    }
    #[test]
    fn auto_select_respects_masks_and_group_target() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(4, 4));
        let lower = e.active_layer;
        e.write_pixel(0, 1, 1, Rgba([200, 100, 50, 255]));
        e.duplicate_layer(lower, true);
        let upper = e.active_layer;
        assert_eq!(e.move_hit_stack(0, 0, 1, 1, 0), Some(upper));
        let mut mask = vec![255; 16];
        mask[5] = 0;
        e.layers[upper].settings.mask = Some(vec![mask]);
        assert_eq!(e.move_hit_stack(0, 0, 1, 1, 0), Some(lower));
        e.layers[upper].settings.mask = None;
        e.create_group();
        let group = e.active_layer;
        e.layer_move_group = false;
        assert_eq!(e.move_hit_stack(0, 0, 1, 1, 0), Some(upper));
        e.layer_move_group = true;
        assert_eq!(e.move_hit_stack(0, 0, 1, 1, 0), Some(group));
        e.layers[group].visible = false;
        assert_eq!(e.move_hit_stack(0, 0, 1, 1, 0), Some(lower));
    }
    #[test]
    fn deleting_last_linked_partner_clears_link_and_undo_restores_it() {
        let mut e = Editor::default();
        e.reset_document(RgbaImage::new(2, 2));
        let first = e.active_layer;
        e.duplicate_layer(first, true);
        let second = e.active_layer;
        e.layers[first].settings.link_id = 17;
        e.layers[second].settings.link_id = 17;
        e.delete_layer(second);
        assert_eq!(e.layers[first].settings.link_id, 0);
        e.history(false);
        assert_eq!(e.layers[first].settings.link_id, 17);
        assert_eq!(e.layers[second].settings.link_id, 17);
        e.history(true);
        assert_eq!(e.layers[first].settings.link_id, 0);
    }
    #[test]
    fn pass_through_adjustment_reaches_backdrop_but_isolated_does_not() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::from_pixel(2, 2, Rgba([100, 100, 100, 255])));
        e.create_group();
        let g = e.active_layer;
        let mut adjustment = e.layers[g].clone();
        adjustment.settings.group_id = 0;
        adjustment.settings.parent = e.layers[g].settings.group_id;
        adjustment.settings.adjustment = Some(layer_properties::Adjustment::Brightness {
            brightness: 20,
            contrast: 0,
        });
        e.layers.push(adjustment);
        assert!(e.composite(0).get_pixel(0, 0)[0] > 100);
        e.layers[g].settings.pass_through = false;
        assert_eq!(e.composite(0).get_pixel(0, 0).0, [100, 100, 100, 255]);
    }
    #[test]
    fn ctrl_shift_selection_and_links_expand_movement_once() {
        let mut e = Editor::default();
        e.reset_document(RgbaImage::new(2, 2));
        let first = e.active_layer;
        e.duplicate_layer(first, true);
        let second = e.active_layer;
        e.select_layer(first);
        e.select_layer_click(
            second,
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(e.selected_roots().len(), 2);
        e.link_selection(false);
        assert_ne!(e.layers[first].settings.link_id, 0);
        e.select_layer(second);
        let members = e.move_members(second, false);
        assert_eq!(members, vec![first, second]);
        e.link_selection(true);
        assert_eq!(e.move_members(second, false), vec![second]);
        e.select_layer(first);
        e.select_layer_click(
            second,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(e.selected_roots().len(), 2);
    }
    #[test]
    fn group_opacity_applies_once_and_mask_preserves_children() {
        let mut e = Editor::default();
        e.diamond = false;
        e.reset_document(RgbaImage::new(2, 2));
        let child = e.active_layer;
        e.write_pixel(0, 0, 0, Rgba([255, 0, 0, 255]));
        e.create_group();
        let group = e.active_layer;
        e.layers[group].settings.opacity = 50;
        assert_eq!(e.composite(0).get_pixel(0, 0).0, [255, 0, 0, 128]);
        e.layers[group].settings.mask = Some(vec![vec![0; 4]]);
        assert_eq!(e.composite(0).get_pixel(0, 0)[3], 0);
        assert_eq!(e.layers[child].tiles[0].get_pixel(0, 0)[3], 255);
    }
    #[test]
    fn nested_group_copy_remaps_ids_and_undo_restores_tree() {
        let mut e = Editor::default();
        e.reset_document(RgbaImage::new(2, 2));
        e.create_group();
        e.create_group();
        let root = e.active_layer;
        let count = e.layers.len();
        e.duplicate_group(root);
        assert_eq!(e.layers.len(), count + 3);
        assert!(valid_tree(
            e.layers.iter().map(|l| (&l.settings, l.material))
        ));
        e.history(false);
        assert_eq!(e.layers.len(), count);
        assert!(valid_tree(
            e.layers.iter().map(|l| (&l.settings, l.material))
        ));
    }
    #[test]
    fn malformed_trees_and_cycles_rejected() {
        let mut a = layer_properties::Properties::default();
        a.group_id = 1;
        a.parent = 2;
        let mut b = a.clone();
        b.group_id = 2;
        b.parent = 1;
        assert!(!valid_tree([(&a, false), (&b, false)].into_iter()));
        b.parent = 0;
        assert!(valid_tree([(&a, false), (&b, false)].into_iter()));
    }
    #[test]
    fn premultiplied_group_mix_has_no_black_fringe() {
        assert_eq!(
            mix_group(Rgba([0; 4]), Rgba([240, 100, 50, 128]), 0.5).0,
            [240, 100, 50, 64]
        );
    }
}
