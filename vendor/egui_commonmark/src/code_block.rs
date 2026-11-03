// Adapted from egui_commonmark_backend 0.20.0 elements.rs (MIT OR Apache-2.0).
// Preserve the copy icon and selection behavior while painting only one frame.
use egui::{self, epaint, Ui};

pub fn code_block<'t>(
    ui: &mut Ui,
    max_width: f32,
    text: &str,
    layouter: &'t mut dyn FnMut(&Ui, &str, f32) -> std::sync::Arc<egui::Galley>,
) {
    let mut text = text.strip_suffix('\n').unwrap_or(text);

    // To manually add background color to the code block, we imitate what
    // TextEdit does internally
    let where_to_put_background = ui.painter().add(egui::Shape::Noop);

    // We use a `TextEdit` to make the text selectable.
    // Note that we take a `&mut` to a non-`mut` `&str`, which is
    // the how to tell `egui` that the text is not editable.
    let margin = egui::Margin { left:4, right:28, top:4, bottom:4 };
    let output = egui::TextEdit::multiline(&mut text)
        .frame(false)
        .margin(margin)
        .layouter(layouter)
        .desired_width(max_width)
        // prevent trailing lines
        .desired_rows(1)
        .show(ui);

    // Background color + frame (This is lost when TextEdit it not editable)
    // egui 0.31 returns the inner text rectangle here, not the allocated outer
    // rectangle. Restore the margins before drawing the single background frame.
    let frame_rect = output.response.rect + margin;
    ui.painter().set(
        where_to_put_background,
        epaint::RectShape::new(
            frame_rect,
            ui.style().noninteractive().corner_radius,
            ui.visuals().extreme_bg_color,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Outside,
        ),
    );

    // Copy icon
    let spacing = &ui.style().spacing;
    let position = egui::pos2(
        frame_rect.right_top().x - spacing.icon_width * 0.5 - spacing.button_padding.x,
        frame_rect.right_top().y + spacing.button_padding.y * 2.0,
    );

    // Check if we should show ✔ instead of 🗐 if the text was copied and the mouse is hovered
    let persistent_id = ui.make_persistent_id(output.response.id);
    let copied_icon = ui.memory_mut(|m| *m.data.get_temp_mut_or_default::<bool>(persistent_id));

    let copy_button = ui
        .put(
            egui::Rect {
                min: position,
                max: position,
            },
            egui::Button::new(if copied_icon { "✔" } else { "🗐" })
                .small()
                .frame(false)
                .fill(egui::Color32::TRANSPARENT),
        )
        // workaround for a regression after egui 0.27 where the edit cursor was shown even when
        // hovering over the button. We try interact_cursor first to allow the cursor to be
        // overriden
        .on_hover_cursor(
            ui.visuals()
                .interact_cursor
                .unwrap_or(egui::CursorIcon::Default),
        );

    // Update icon state in persistent memory
    if copied_icon && !copy_button.hovered() {
        ui.memory_mut(|m| *m.data.get_temp_mut_or_default(persistent_id) = false);
    }
    if !copied_icon && copy_button.clicked() {
        ui.memory_mut(|m| *m.data.get_temp_mut_or_default(persistent_id) = true);
    }

    if copy_button.clicked() {
        use egui::TextBuffer as _;
        let copy_text = if let Some(cursor) = output.cursor_range {
            let selected_chars = cursor.as_sorted_char_range();
            let selected_text = text.char_range(selected_chars);
            if selected_text.is_empty() {
                text.to_owned()
            } else {
                selected_text.to_owned()
            }
        } else {
            text.to_owned()
        };
        ui.ctx().copy_text(copy_text);
    }
}

