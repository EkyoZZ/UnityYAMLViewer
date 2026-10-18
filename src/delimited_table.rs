//! Delimited text preview and transactional cell editing.
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::{sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};

#[derive(Default)]
pub struct Preview {
    source: String,
    delimiter: char,
    rows: Vec<Vec<String>>,
    columns: usize,
    raw: bool,
    error: Option<String>,
    ranges: Vec<Vec<std::ops::Range<usize>>>,
    check: Check,
    search: Search,
    active_cell: Option<(usize,usize)>,
}

fn column_name(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (index % 26) as u8) as char);
        if index < 26 { break; }
        index = index / 26 - 1;
    }
    letters.into_iter().rev().collect()
}
fn cell_address(row: usize, column: usize) -> String { format!("{}{row}",column_name(column)) }
fn decoded_cell(raw: &str, delimiter: char) -> String {
    parse(raw,delimiter).ok().and_then(|rows|rows.into_iter().next()).and_then(|row|row.into_iter().next()).unwrap_or_default()
}

#[derive(Default)]
struct Search {
    query: String, cached_query: String, source: Option<String>,
    matches: Vec<(usize,usize)>, selected: Option<(usize,usize)>, jump: bool,
    last_input: Option<Instant>, rerun: bool, select_after: bool,
    pending: Option<mpsc::Receiver<Vec<(usize,usize)>>>, cancel: Option<Arc<AtomicBool>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchMode { Live, Delayed, Submitted }
fn search_mode(cells: usize, bytes: usize) -> SearchMode {
    if cells <= 10_000 && bytes <= 1_000_000 { SearchMode::Live }
    else if cells <= 50_000 && bytes <= 5_000_000 { SearchMode::Delayed }
    else { SearchMode::Submitted }
}
impl Drop for Search {
    fn drop(&mut self) { if let Some(cancel) = &self.cancel { cancel.store(true,Ordering::Relaxed); } }
}
fn matches(rows: &[Vec<String>], query: &str) -> Vec<(usize,usize)> {
    if query.is_empty() { return Vec::new(); }
    let query = query.to_lowercase();
    rows.iter().enumerate().flat_map(|(row, values)| values.iter().enumerate().filter_map({
        let query = &query;
        move |(column,value)| value.to_lowercase().contains(query).then_some((row,column))
    })).collect()
}
fn navigate(items: &[(usize,usize)], selected: Option<(usize,usize)>, forward: bool) -> Option<(usize,usize)> {
    if items.is_empty() { return None; }
    let count = items.len();
    let current = selected.and_then(|key| items.iter().position(|item| *item == key));
    let next = current.map_or(if forward {0} else {count-1}, |i| if forward {(i+1)%count} else {(i+count-1)%count});
    Some(items[next])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { Integer, Number, Boolean, JsonObject, Array, Text }
impl Kind {
    fn label(self) -> &'static str { match self { Self::Integer=>"整数", Self::Number=>"数值", Self::Boolean=>"布尔值", Self::JsonObject=>"JSON 对象", Self::Array=>"数组", Self::Text=>"文本" } }
    fn accepts(self, value: &str) -> bool {
        let value = value.trim();
        value.is_empty() || match self {
            Self::Integer => integer(value),
            Self::Number => value.parse::<f64>().is_ok_and(f64::is_finite),
            Self::Boolean => value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false"),
            Self::JsonObject => value.starts_with('{') && serde_json::from_str::<serde_json::Value>(value).is_ok_and(|value|value.is_object()),
            Self::Array => value.starts_with('[') && serde_json::from_str::<serde_json::Value>(value).is_ok_and(|value|value.is_array()),
            Self::Text => true,
        }
    }
}
fn integer(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())
}
fn inspect(rows: &[Vec<String>], columns: usize, header: bool, samples: usize) -> (Vec<Kind>, Vec<(usize, usize)>) {
    let start = usize::from(header);
    let kinds: Vec<_> = (0..columns).map(|column| {
        let values: Vec<_> = rows.iter().skip(start).take(samples).filter_map(|row| row.get(column)).map(|value| value.trim()).filter(|value| !value.is_empty()).collect();
        if values.len() >= 2 {
            if values.iter().all(|value| Kind::Integer.accepts(value)) { return Kind::Integer; }
            if values.iter().all(|value| Kind::Number.accepts(value)) { return Kind::Number; }
            if values.iter().all(|value| Kind::Boolean.accepts(value)) { return Kind::Boolean; }
            if values.iter().all(|value| Kind::JsonObject.accepts(value)) { return Kind::JsonObject; }
            if values.iter().all(|value| Kind::Array.accepts(value)) { return Kind::Array; }
        }
        // Ambiguous samples get two more records, never require a small table
        // to have five rows. A strong majority can expose a bad sample itself.
        let extended: Vec<_> = rows.iter().skip(start).take(samples+2).filter_map(|row| row.get(column)).map(|value| value.trim()).filter(|value| !value.is_empty()).collect();
        for kind in [Kind::Integer, Kind::Number, Kind::Boolean, Kind::JsonObject, Kind::Array] {
            let matches = extended.iter().filter(|value| kind.accepts(value)).count();
            if matches >= 2 && matches*5 >= extended.len()*4 { return kind; }
        }
        Kind::Text
    }).collect();
    let mut errors = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(start) {
        for (column, kind) in kinds.iter().enumerate() {
            if row.get(column).is_some_and(|value| !kind.accepts(value)) { errors.push((index, column)); }
        }
    }
    (kinds, errors)
}
struct Check {
    enabled: bool, header: bool,
    source: Option<String>, kinds: Vec<Kind>, errors: Vec<(usize,usize)>,
    selected: Option<(usize,usize)>, jump: bool,
}
impl Default for Check {
    fn default() -> Self { Self { enabled:true, header:true, source:None, kinds:Vec::new(), errors:Vec::new(), selected:None, jump:false } }
}

fn cell_ranges(text: &str, delimiter: char) -> Vec<Vec<std::ops::Range<usize>>> {
    let mut rows = Vec::new(); let mut row = Vec::new();
    let mut start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let bom = start;
    let mut chars = text.char_indices().filter(|(i, _)| *i >= bom).peekable();
    let mut quoted = false;
    while let Some((i, c)) = chars.next() {
        if c == '"' {
            if quoted && chars.peek().is_some_and(|(_, next)| *next == '"') { chars.next(); }
            else if quoted { quoted = false; }
            else if i == start { quoted = true; }
        } else if !quoted && (c == delimiter || c == '\n' || c == '\r') {
            row.push(start..i); start = i + c.len_utf8();
            if c != delimiter {
                if c == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') { chars.next(); start += 1; }
                rows.push(std::mem::take(&mut row));
            }
        }
    }
    if start < text.len() || !row.is_empty() { row.push(start..text.len()); rows.push(row); }
    rows
}

fn encoded(value: &str, delimiter: char) -> String {
    if value.contains([delimiter, '"', '\r', '\n']) { format!("\"{}\"", value.replace('"', "\"\"")) }
    else { value.to_owned() }
}

fn parse(text: &str, delimiter: char) -> Result<Vec<Vec<String>>, &'static str> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut pending = false;
    while let Some(c) = chars.next() {
        pending = true;
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') { chars.next(); field.push('"'); }
                else { quoted = false; closed = true; }
            } else { field.push(c); }
        } else if c == delimiter {
            row.push(std::mem::take(&mut field)); closed = false;
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') { chars.next(); }
            row.push(std::mem::take(&mut field)); rows.push(std::mem::take(&mut row));
            closed = false; pending = false;
        } else if c == '"' && field.is_empty() && !closed { quoted = true; }
        else if closed { return Err("表格引号格式无效，请查看原文"); }
        else { field.push(c); }
    }
    if quoted { return Err("表格引号格式无效，请查看原文"); }
    if pending { row.push(field); rows.push(row); }
    Ok(rows)
}

impl Preview {
    pub fn show(&mut self, ui: &mut egui::Ui, text: &str, delimiter: char) {
        use crate::localization::tr;
        // Reserve physical gutters instead of painting floating bars over cells.
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        // The theme uses the same fill for idle widgets and the panel. Use
        // foreground colors so idle handles remain visible against the track.
        ui.style_mut().spacing.scroll.foreground_color = true;
        ui.style_mut().spacing.scroll.bar_width = 10.0;
        ui.style_mut().spacing.scroll.handle_min_length = 24.0;
        if self.source != text || self.delimiter != delimiter {
            self.active_cell = None;
            self.check.source = None; self.check.selected = None;
            self.search.source = None; self.search.selected = None;
            if let Some(cancel) = &self.search.cancel { cancel.store(true,Ordering::Relaxed); }
            self.source = text.to_owned(); self.delimiter = delimiter;
            match parse(text, delimiter) {
                Ok(rows) => { self.ranges = cell_ranges(text, delimiter); self.columns = rows.iter().map(Vec::len).max().unwrap_or(0); self.rows = rows; self.error = None; }
                Err(error) => { self.rows.clear(); self.columns = 0; self.error = Some(tr(error).to_owned()); }
            }
        }
        let mut find = false;
        let mut input_changed = false;
        let mut recheck = false;
        let focus_find = ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.raw, false, tr("表格"));
            ui.selectable_value(&mut self.raw, true, tr("原文"));
            ui.small(format!("{} × {}", self.rows.len(), self.columns));
        });
        ui.horizontal(|ui| {
            recheck |= ui.checkbox(&mut self.check.enabled, tr("类型检查")).changed();
            recheck |= ui.checkbox(&mut self.check.header, tr("首行为表头")).changed();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                find = ui.button(tr("查找")).clicked();
                let response = ui.add(egui::TextEdit::singleline(&mut self.search.query).id_source("table_find").desired_width(160.0));
                if focus_find { response.request_focus(); }
                input_changed = response.changed();
                if response.has_focus() && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE,egui::Key::Enter)) { find = true; }
            });
        });
        let rendered = crate::editing::scope_value(ui).unwrap_or_else(|| text.to_owned());
        let mode = search_mode(self.rows.len().saturating_mul(self.columns),rendered.len());
        if input_changed { self.search.last_input = Some(Instant::now()); }
        let auto = match mode {
            SearchMode::Live => input_changed,
            SearchMode::Delayed => self.search.last_input.is_some_and(|time|time.elapsed() >= Duration::from_millis(250)),
            SearchMode::Submitted => false,
        };
        if mode == SearchMode::Delayed && self.search.last_input.is_some() { ui.ctx().request_repaint_after(Duration::from_millis(50)); }
        let new_query = (find || auto) && self.search.cached_query != self.search.query;
        if find || auto { self.search.last_input = None; }
        if new_query {
            self.search.cached_query = self.search.query.clone();
            self.search.source = None;
            self.search.selected = None;
        }
        if self.search.source.as_deref() != Some(&rendered) {
            self.search.source = Some(rendered.clone());
            if let Some(cancel) = &self.search.cancel { cancel.store(true,Ordering::Relaxed); }
            self.search.matches.clear(); self.search.selected = None;
            self.search.rerun = !self.search.cached_query.is_empty();
            self.search.select_after = find;
        }
        if let Some(pending) = &self.search.pending {
            if find { self.search.select_after = true; }
            match pending.try_recv() {
                Ok(found) => {
                    let cancelled = self.search.cancel.as_ref().is_some_and(|cancel|cancel.load(Ordering::Relaxed));
                    self.search.pending = None; self.search.cancel = None;
                    if !cancelled {
                        self.search.matches = found;
                        if self.search.select_after { self.search.selected = self.search.matches.first().copied(); self.search.jump = self.search.selected.is_some(); }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.search.pending = None; self.search.cancel = None; }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.search.rerun && self.search.pending.is_none() {
            self.search.rerun = false;
            if mode == SearchMode::Submitted {
                let source = rendered.clone(); let query = self.search.cached_query.to_lowercase();
                let cancel = Arc::new(AtomicBool::new(false)); let worker_cancel = cancel.clone();
                let (tx,rx) = mpsc::channel(); let ctx = ui.ctx().clone();
                self.search.pending = Some(rx); self.search.cancel = Some(cancel);
                std::thread::spawn(move || {
                    let mut found = Vec::new();
                    if let Ok(rows) = parse(&source,delimiter) {
                        for (row,values) in rows.iter().enumerate() {
                            if worker_cancel.load(Ordering::Relaxed) { break; }
                            for (column,value) in values.iter().enumerate() {
                                if value.to_lowercase().contains(&query) { found.push((row,column)); }
                            }
                        }
                    }
                    let _ = tx.send(found); ctx.request_repaint();
                });
            } else {
                self.search.matches = parse(&rendered,delimiter).map(|rows|matches(&rows,&self.search.cached_query)).unwrap_or_default();
            }
        }
        if self.check.enabled && self.error.is_none() {
            if recheck || self.check.source.as_deref() != Some(&rendered) {
                if let Ok(rows) = parse(&rendered, delimiter) {
                    (self.check.kinds, self.check.errors) = inspect(&rows, self.columns, self.check.header, 3);
                    if self.check.selected.is_some_and(|key| !self.check.errors.contains(&key)) { self.check.selected = None; }
                }
                self.check.source = Some(rendered);
            }
        }
        ui.horizontal(|ui| {
            if self.check.enabled && self.error.is_none() {
                ui.label(format!("{}: {}", tr("类型错误"), self.check.errors.len()));
                for (label, forward) in [("上一个",false),("下一个",true)] {
                    if ui.add_enabled(!self.check.errors.is_empty(),egui::Button::new(tr(label))).clicked() {
                        self.check.selected = navigate(&self.check.errors,self.check.selected,forward); self.check.jump = true; self.raw = false;
                        self.search.selected = None;
                    }
                }
                if let Some((row,column)) = self.check.selected { ui.label(cell_address(row,column)); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (label,forward) in [("下一个",true),("上一个",false)] {
                if ui.add_enabled(!self.search.matches.is_empty(),egui::Button::new(tr(label))).clicked() {
                    self.search.selected = navigate(&self.search.matches,self.search.selected,forward); self.search.jump = true;
                }
            }
            ui.label(format!("{}: {}",tr("匹配"),self.search.matches.len()));
            if self.search.pending.is_some() || self.search.rerun { ui.spinner(); }
            if let Some(key) = self.search.selected {
                if let Some(index) = self.search.matches.iter().position(|item| *item == key) { ui.small(format!("{} / {}",index+1,self.search.matches.len())); }
            }
        });
        });
        if find && self.search.pending.is_none() && !self.search.jump { self.search.selected = navigate(&self.search.matches,self.search.selected,true); self.search.jump = self.search.selected.is_some(); }
        if self.search.jump { self.check.selected = None; self.check.jump = false; self.raw = false; }
        if let Some(error) = &self.error { ui.colored_label(ui.visuals().warn_fg_color, error); }
        if self.raw || self.error.is_some() {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            egui::ScrollArea::both().id_salt("delimited_raw").auto_shrink([false, false]).show(ui, |ui| {
                let rendered = crate::editing::scope_value(ui).unwrap_or_else(|| text.to_owned());
                let mut view = rendered.as_str();
                ui.add(egui::TextEdit::multiline(&mut view).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
            });
            return;
        }
        if self.columns == 0 { return; }
        let editing = crate::editing::active(ui);
        if editing {
            if self.active_cell.is_none() && self.ranges.first().is_some_and(|row| !row.is_empty()) { self.active_cell = Some((0,0)); }
            ui.horizontal(|ui| {
                if let Some((row,column)) = self.active_cell {
                    let height = ui.spacing().interact_size.y;
                    let address = cell_address(row,column);
                    let font = egui::TextStyle::Monospace.resolve(ui.style());
                    let color = ui.visuals().weak_text_color();
                    let width = ui.fonts(|fonts|fonts.layout_no_wrap(address.clone(),font.clone(),color).size().x)+8.0;
                    let (rect,_) = ui.allocate_exact_size(egui::vec2(width,height),egui::Sense::hover());
                    ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,address,font,color);
                    if let Some(bind) = self.ranges.get(row).and_then(|row|row.get(column)).cloned().and_then(|range|crate::editing::cell_binding(ui,range)) {
                        let mut value = decoded_cell(&bind.value(),delimiter);
                        let fill = if bind.dirty() { egui::Color32::from_rgba_unmultiplied(240,170,30,55) } else { ui.visuals().extreme_bg_color };
                        let response = ui.add_sized(egui::vec2(ui.available_width(),height),egui::TextEdit::singleline(&mut value).id_source(("cell_value_bar",row,column)).background_color(fill));
                        if response.changed() { bind.replace_text(encoded(&value,delimiter)); ui.ctx().request_repaint(); }
                    }
                }
            });
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        // One viewport owns both bars, so vertical scrolling cannot move the
        // bottom bar away, and wide columns cannot move the right bar offscreen.
        let target = if self.search.jump { self.search.selected } else if self.check.jump { self.check.selected } else { None };
        self.check.jump = false;
        self.search.jump = false;
        let mut scroll = egui::ScrollArea::both().id_salt("delimited_viewport")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .auto_shrink([false, false]);
        if let Some((row,_)) = target { scroll = scroll.vertical_scroll_offset(row as f32 * (24.0 + ui.spacing().item_spacing.y)); }
        scroll.show(ui, |ui| {
            TableBuilder::new(ui).striped(true).resizable(true).vscroll(false)
                .column(Column::initial(48.0).at_least(40.0))
                .columns(Column::initial(160.0).at_least(40.0), self.columns)
                .header(24.0, |mut header| {
                    header.col(|ui| { ui.strong("#"); });
                    for column in 0..self.columns { header.col(|ui| {
                        let mut label = column_name(column);
                        if self.check.enabled { if let Some(kind) = self.check.kinds.get(column) { label.push_str(&format!(" · {}",tr(kind.label()))); } }
                        ui.strong(label);
                    }); }
                }).body(|body| {
                    body.rows(24.0, self.rows.len(), |mut row| {
                        let index = row.index();
                        row.col(|ui| { ui.label(index.to_string()); });
                        for column in 0..self.columns {
                            row.col(|ui| {
                                let invalid = self.check.enabled && self.check.errors.binary_search(&(index,column)).is_ok();
                                let matched = self.search.matches.binary_search(&(index,column)).is_ok();
                                let selected = self.check.enabled && self.check.selected == Some((index,column)) || self.search.selected == Some((index,column)) || editing && self.active_cell == Some((index,column));
                                let background = ui.painter().add(egui::Shape::Noop);
                                let value = self.rows[index].get(column).map(String::as_str).unwrap_or("");
                                if let Some(bind) = self.ranges.get(index).and_then(|row| row.get(column)).cloned().and_then(|range| crate::editing::cell_binding(ui, range)) {
                                    let raw = bind.value();
                                    let mut draft = decoded_cell(&raw, delimiter);
                                    let fill = if invalid { egui::Color32::from_rgba_unmultiplied(230,45,55,90) } else if matched { egui::Color32::from_rgba_unmultiplied(90,205,130,65) } else if bind.dirty() { egui::Color32::from_rgba_unmultiplied(240,170,30,55) } else { ui.visuals().extreme_bg_color };
                                    let response = ui.add(egui::TextEdit::singleline(&mut draft).id_source((index,column)).desired_width(ui.available_width()).background_color(fill));
                                    if (response.clicked() || response.has_focus()) && self.active_cell != Some((index,column)) { self.active_cell = Some((index,column)); ui.ctx().request_repaint(); }
                                    if response.changed() { bind.replace_text(encoded(&draft, delimiter)); ui.ctx().request_repaint(); }
                                    if bind.dirty() && matched { ui.painter().line_segment([response.rect.left_bottom(),response.rect.right_bottom()],egui::Stroke::new(2.0_f32,egui::Color32::from_rgb(240,170,30))); }
                                    response.on_hover_text(draft);
                                } else {
                                    // Truncated labels already provide a full-text tooltip.
                                    ui.add(egui::Label::new(value).truncate().selectable(true));
                                    if invalid { ui.painter().set(background,egui::Shape::rect_filled(ui.min_rect(),0.0,egui::Color32::from_rgba_unmultiplied(230,45,55,90))); }
                                    else if matched { ui.painter().set(background,egui::Shape::rect_filled(ui.min_rect(),0.0,egui::Color32::from_rgba_unmultiplied(90,205,130,65))); }
                                }
                                if matched && invalid { ui.painter().rect_stroke(ui.min_rect(),0.0,egui::Stroke::new(1.0_f32,egui::Color32::from_rgb(70,180,110)),egui::StrokeKind::Inside); }
                                if selected { ui.painter().rect_stroke(ui.min_rect(),0.0,egui::Stroke::new(1.0_f32,ui.visuals().selection.stroke.color),egui::StrokeKind::Inside); }
                                if target == Some((index,column)) { ui.scroll_to_rect(ui.min_rect(),Some(egui::Align::Center)); }
                            });
                        }
                    });
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, cell_ranges, encoded, inspect, Kind, matches, navigate, search_mode, SearchMode, column_name, cell_address};
    #[test]
    fn json_object_and_array_columns_validate_syntax_and_outer_type() {
        let rows = vec![
            vec!["object".into(),"array".into()],
            vec!["{\"item\":1}".into(),"[16,0,0,0]".into()],
            vec!["{}".into(),"[]".into()],
            vec!["{\"nested\":{\"v\":true}}".into(),"[\"a\",{\"x\":1}]".into()],
            vec!["{bad}".into(),"[1,]".into()],
            vec!["[]".into(),"{}".into()],
            vec!["".into(),"".into()],
        ];
        let (kinds,errors) = inspect(&rows,2,true,3);
        assert!(kinds == vec![Kind::JsonObject,Kind::Array]);
        assert_eq!(errors,vec![(4,0),(4,1),(5,0),(5,1)]);
    }
    #[test]
    fn spreadsheet_columns_and_zero_based_row_addresses() {
        for (index,name) in [(0,"A"),(4,"E"),(25,"Z"),(26,"AA"),(51,"AZ"),(52,"BA"),(701,"ZZ"),(702,"AAA")] {
            assert_eq!(column_name(index),name);
        }
        assert_eq!(cell_address(0,0),"A0");
        assert_eq!(cell_address(15,4),"E15");
    }
    #[test]
    fn search_modes_respect_both_cell_and_byte_boundaries() {
        assert!(search_mode(10_000,1_000_000) == SearchMode::Live);
        assert!(search_mode(10_001,1) == SearchMode::Delayed);
        assert!(search_mode(1,1_000_001) == SearchMode::Delayed);
        assert!(search_mode(50_000,5_000_000) == SearchMode::Delayed);
        assert!(search_mode(50_001,1) == SearchMode::Submitted);
        assert!(search_mode(1,5_000_001) == SearchMode::Submitted);
    }
    #[test]
    fn search_counts_cells_and_navigation_wraps_in_both_directions() {
        let rows = parse("Name|备注\nTree tree|树木\nTREE|other", '|').unwrap();
        let found = matches(&rows,"tree");
        assert_eq!(found,vec![(1,0),(2,0)]);
        assert_eq!(navigate(&found,None,true),Some((1,0)));
        assert_eq!(navigate(&found,None,false),Some((2,0)));
        assert_eq!(navigate(&found,Some((2,0)),true),Some((1,0)));
        assert_eq!(navigate(&found,Some((1,0)),false),Some((2,0)));
        assert_eq!(matches(&rows,"树"),vec![(1,1)]);
        assert!(matches(&rows,"").is_empty());
        assert_eq!(navigate(&[],None,true),None);
    }
    #[test]
    fn three_rows_establish_type_before_later_errors_and_five_resolve_ambiguity() {
        let rows = parse("id\n1\n2\n3\nbad\nwrong", ',').unwrap();
        assert_eq!(inspect(&rows,1,true,3).1,vec![(4,0),(5,0)]);
        let rows = parse("id\n1\nbad\n2\n3\n4", ',').unwrap();
        assert_eq!(inspect(&rows,1,true,3).1,vec![(2,0)]);
        let short = parse("id\n1\n2", ',').unwrap();
        assert!(inspect(&short,1,true,3).0 == vec![Kind::Integer]);
        let sparse = parse("id\n\n1\n\n2", ',').unwrap();
        assert!(inspect(&sparse,1,true,3).0 == vec![Kind::Integer]);
    }
    #[test]
    fn inferred_types_skip_header_widen_numbers_and_find_later_errors() {
        let rows = parse("id,value,enabled,label\n1,1.5,true,a\n2,2,FALSE,3\noops,NaN,yes,b\n4,,true,c", ',').unwrap();
        let (kinds, errors) = inspect(&rows, 4, true, 2);
        assert!(kinds == vec![Kind::Integer,Kind::Number,Kind::Boolean,Kind::Text]);
        assert_eq!(errors,vec![(3,0),(3,1),(3,2)]);
        let (kinds, _) = inspect(&rows, 4, false, 2);
        assert!(kinds == vec![Kind::Text;4]);
        let mut fixed = rows;
        fixed[3] = vec!["3".into(),"2e3".into(),"false".into(),"b".into()];
        assert!(inspect(&fixed,4,true,2).1.is_empty());
    }
    #[test]
    fn ambiguous_or_sparse_samples_do_not_invent_errors() {
        let rows = parse("a|b|c\n1||true\nword||\n2|bad|no", '|').unwrap();
        let (kinds,errors) = inspect(&rows,3,true,2);
        assert!(kinds == vec![Kind::Text;3]);
        assert!(errors.is_empty());
    }
    #[test]
    fn ranges_and_encoding_preserve_multiline_unicode_fields() {
        for delimiter in [',', '\t', '|'] {
            let text = format!("\u{feff}中文{delimiter}\"a{delimiter}b\r\nc\"{delimiter}\r\n");
            let rows = parse(&text, delimiter).unwrap();
            let ranges = cell_ranges(&text, delimiter);
            assert_eq!(ranges.len(), rows.len());
            for (row, spans) in rows.iter().zip(ranges) {
                assert_eq!(row.len(), spans.len());
                for (value, span) in row.iter().zip(spans) {
                    let raw = &text[span];
                    let decoded = parse(raw, delimiter).unwrap().into_iter().next().and_then(|row| row.into_iter().next()).unwrap_or_default();
                    assert_eq!(*value, decoded);
                }
            }
            for value in ["", "中文", "a\"b", "two\nlines", "a,b\t|c"] {
                let serialized = encoded(value, delimiter);
                let decoded = parse(&serialized, delimiter).unwrap().into_iter().next().and_then(|row| row.into_iter().next()).unwrap_or_default();
                assert_eq!(value, decoded);
            }
        }
    }
    #[test]
    fn quoted_delimiters_newlines_and_empty_cells() {
        assert_eq!(parse("\u{feff}a,\"b,c\",\"d\"\"e\"\r\n1,\"two\nlines\",\r\n", ',').unwrap(),
            vec![vec!["a", "b,c", "d\"e"], vec!["1", "two\nlines", ""]]);
        assert_eq!(parse("a\tb\n1\t\t", '\t').unwrap(), vec![vec!["a", "b"], vec!["1", "", ""]]);
        assert_eq!(parse("a|\"b|c\"|\n", '|').unwrap(), vec![vec!["a", "b|c", ""]]);
        assert!(parse("\"unfinished", ',').is_err());
        assert!(parse("\"closed\"extra", ',').is_err());
        assert!(parse("", ',').unwrap().is_empty());
    }
}
