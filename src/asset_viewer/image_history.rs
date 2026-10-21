use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};
static LIMIT: AtomicUsize = AtomicUsize::new(crate::editing::DEFAULT_UNDO_LIMIT);
pub fn set_limit(limit: usize) {
    LIMIT.store(
        limit.clamp(
            crate::editing::MIN_UNDO_LIMIT,
            crate::editing::MAX_UNDO_LIMIT,
        ),
        Ordering::Relaxed,
    );
}
pub fn limit() -> usize {
    LIMIT.load(Ordering::Relaxed)
}
pub struct History<T> {
    pub undo: VecDeque<T>,
    pub redo: Vec<T>,
}
impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
        }
    }
}
impl<T: Clone + PartialEq> History<T> {
    pub fn record(&mut self, before: T, after: &T) {
        if before == *after {
            return;
        }
        self.undo.push_back(before);
        self.redo.clear();
        self.trim();
    }
    pub fn trim(&mut self) {
        while self.undo.len() > limit() {
            self.undo.pop_front();
        }
        while self.redo.len() > limit() {
            self.redo.remove(0);
        }
    }
    pub fn undo(&mut self, current: &T) -> Option<T> {
        self.trim();
        let before = self.undo.pop_back()?;
        self.redo.push(current.clone());
        Some(before)
    }
    pub fn redo(&mut self, current: &T) -> Option<T> {
        self.trim();
        let after = self.redo.pop()?;
        self.undo.push_back(current.clone());
        Some(after)
    }
}
pub fn shortcut(ui: &mut eframe::egui::Ui) -> i8 {
    use eframe::egui::{Key, Modifiers};
    if ui.ctx().wants_keyboard_input() {
        return 0;
    }
    if ui.ctx().viewport_id() == eframe::egui::ViewportId::ROOT
        && !ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| ui.ctx().layer_id_at(p) == Some(ui.layer_id()))
    {
        return 0;
    }
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Z)
            || i.consume_key(Modifiers::CTRL, Key::Y)
        {
            1
        } else if i.consume_key(Modifiers::CTRL, Key::Z) {
            -1
        } else {
            0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batches_are_atomic_and_new_edits_invalidate_redo() {
        let mut history = History::default();
        let empty: Vec<String> = Vec::new();
        let batch = vec!["first.png".into(), "unrelated.png".into()];
        history.record(empty.clone(), &batch);
        assert_eq!(history.undo(&batch), Some(empty.clone()));
        assert_eq!(history.redo(&empty), Some(batch.clone()));
        assert_eq!(history.undo(&batch), Some(empty.clone()));
        history.record(empty.clone(), &empty);
        assert_eq!(history.redo.len(), 1);
        history.record(empty, &vec!["different.png".into()]);
        assert!(history.redo.is_empty());
    }
}
