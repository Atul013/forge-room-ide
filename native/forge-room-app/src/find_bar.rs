//! The Find / Replace bar floating over the editor, plus its match bookkeeping.

use crate::{find, theme};
use eframe::egui::{self, Align2, Key, Modifiers, RichText};
use std::ops::Range;

#[derive(Default)]
pub struct FindState {
    pub open: bool,
    pub replace: bool,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub matches: Vec<Range<usize>>,
    pub current: Option<usize>,
    focus_query: bool,
    key: Option<(u64, u64, String, bool)>,
}

pub enum Intent {
    Next,
    Previous,
    ReplaceOne,
    ReplaceAll,
    Close,
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

impl FindState {
    pub fn show_bar(&mut self, replace: bool) {
        self.open = true;
        self.replace = replace || self.replace;
        self.focus_query = true;
    }

    /// Recompute matches when the tab, its text, the query or the case option changed.
    /// `cursor` is the caret's character index, used to pick the first match to show.
    pub fn refresh(&mut self, tab_id: u64, text: &str, cursor: usize) {
        let key = (tab_id, hash(text), self.query.clone(), self.case_sensitive);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let query_changed = self.key.as_ref().map(|old| (&old.2, old.3)) != Some((&key.2, key.3));
        self.matches = find::find_all(text, &self.query, self.case_sensitive);
        self.key = Some(key);
        if self.matches.is_empty() {
            self.current = None;
        } else if query_changed || self.current.is_none_or(|index| index >= self.matches.len()) {
            self.current = find::next_match(&self.matches, cursor);
        }
    }

    pub fn step(&mut self, forward: bool) -> Option<Range<usize>> {
        let count = self.matches.len();
        if count == 0 {
            return None;
        }
        let next = match (self.current, forward) {
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
            (None, _) => 0,
        };
        self.current = Some(next);
        self.matches.get(next).cloned()
    }
}

/// Draw the bar; returns what the user asked for.
pub fn show(context: &egui::Context, state: &mut FindState) -> Vec<Intent> {
    let mut intents = Vec::new();
    if !state.open {
        return intents;
    }
    let escape = context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    if escape {
        intents.push(Intent::Close);
    }
    egui::Area::new(egui::Id::unique("find-bar"))
        .anchor(Align2::RIGHT_TOP, [-34.0, 118.0])
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            egui::Frame::new()
                .fill(theme::bg_sidebar())
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .corner_radius(8.0)
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(90),
                })
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut state.query)
                                .hint_text("Find")
                                .desired_width(210.0),
                        );
                        if state.focus_query {
                            input.request_focus();
                            state.focus_query = false;
                        }
                        if input.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            intents.push(if ui.input(|i| i.modifiers.shift) {
                                Intent::Previous
                            } else {
                                Intent::Next
                            });
                            input.request_focus();
                        }
                        let summary = if state.query.is_empty() {
                            String::new()
                        } else if state.matches.is_empty() {
                            "No results".to_string()
                        } else {
                            format!(
                                "{} of {}",
                                state.current.map(|index| index + 1).unwrap_or(0),
                                state.matches.len()
                            )
                        };
                        let color = if !state.query.is_empty() && state.matches.is_empty() {
                            egui::Color32::from_rgb(240, 113, 120)
                        } else {
                            theme::text_dim()
                        };
                        ui.label(RichText::new(summary).size(12.0).color(color));
                        if ui
                            .small_button("‹")
                            .on_hover_text("Previous (Shift+Enter)")
                            .clicked()
                        {
                            intents.push(Intent::Previous);
                        }
                        if ui.small_button("›").on_hover_text("Next (Enter)").clicked() {
                            intents.push(Intent::Next);
                        }
                        ui.toggle_value(&mut state.case_sensitive, "Aa")
                            .on_hover_text("Match case");
                        if ui.small_button("×").on_hover_text("Close (Esc)").clicked() {
                            intents.push(Intent::Close);
                        }
                    });
                    if state.replace {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut state.replacement)
                                    .hint_text("Replace")
                                    .desired_width(210.0),
                            );
                            let any = !state.matches.is_empty();
                            if ui.add_enabled(any, egui::Button::new("Replace")).clicked() {
                                intents.push(Intent::ReplaceOne);
                            }
                            if ui.add_enabled(any, egui::Button::new("All")).clicked() {
                                intents.push(Intent::ReplaceAll);
                            }
                        });
                    }
                });
        });
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_finds_matches_and_picks_one_near_the_cursor() {
        let mut state = FindState {
            query: "ab".into(),
            ..FindState::default()
        };
        state.refresh(1, "ab xx ab yy ab", 6);
        assert_eq!(state.matches.len(), 3);
        assert_eq!(state.current, Some(1));
        // Unchanged inputs do not disturb the chosen match.
        state.current = Some(2);
        state.refresh(1, "ab xx ab yy ab", 0);
        assert_eq!(state.current, Some(2));
    }

    #[test]
    fn stepping_wraps_both_ways() {
        let mut state = FindState {
            query: "a".into(),
            ..FindState::default()
        };
        state.refresh(1, "a.a.a", 0);
        assert_eq!(state.step(true), Some(2..3));
        assert_eq!(state.step(true), Some(4..5));
        assert_eq!(state.step(true), Some(0..1));
        assert_eq!(state.step(false), Some(4..5));
    }

    #[test]
    fn no_query_means_no_matches() {
        let mut state = FindState::default();
        state.refresh(1, "anything", 0);
        assert!(state.matches.is_empty() && state.current.is_none());
        assert!(state.step(true).is_none());
    }
}
