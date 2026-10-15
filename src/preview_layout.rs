//! Shared proportional sizing for a preview beside the central Inspector.
use eframe::egui;

fn width_for(available: f32, previous: Option<(f32, f32)>) -> (f32, f32, f32) {
    let available = available.max(1.);
    let minimum = 180_f32.min(available * 0.4);
    let maximum = (available - 360_f32.min(available * 0.4)).max(minimum);
    let width = previous.filter(|(width, total)| width.is_finite() && *total > 0.)
        .map_or(available * 0.7, |(width, total)| width / total * available);
    (width.clamp(minimum, maximum), minimum, maximum)
}

pub(crate) fn panel(ctx: &egui::Context, name: &'static str) -> egui::SidePanel {
    let id = egui::Id::new(name);
    let available = ctx.available_rect().width();
    let previous_available = ctx.data(|data| data.get_temp::<f32>(id.with("available_width")));
    let mut state = egui::containers::panel::PanelState::load(ctx, id);
    let previous = state.as_ref().zip(previous_available).map(|(state, total)| (state.rect.width(), total));
    let (width, minimum, maximum) = width_for(available, previous);
    if let Some(state) = &mut state {
        // Keep egui's resize interaction and remember its width for the next frame.
        if previous_available.is_some() { state.rect.max.x = state.rect.min.x + width; }
    }
    ctx.data_mut(|data| {
        data.insert_temp(id.with("available_width"), available);
        if let Some(state) = state { data.insert_persisted(id, state); }
    });
    egui::SidePanel::left(name).resizable(true).default_width(width).min_width(minimum).max_width(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resize_preserves_user_split_until_minimum_is_reached() {
        assert_eq!(width_for(1600., Some((600., 1000.))).0, 960.);
        assert_eq!(width_for(1000., Some((960., 1600.))).0, 600.);
        let (width, minimum, maximum) = width_for(250., Some((600., 1000.)));
        assert!(width >= minimum && width <= maximum);
        assert!(250. - width >= 100.);
    }
}
