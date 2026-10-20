use eframe::egui::{self, Pos2, Rect, Vec2};

#[derive(Clone, Debug)]
pub(crate) struct TextCharacter {
    pub text: char,
    /// Generated line breaks have text but no visible glyph bounds.
    pub bounds: Option<Rect>,
}

/// Character bounds use normalized bitmap coordinates, with the origin at the top left.
pub(crate) fn rotate_point(p: Pos2, rotation: i16) -> Pos2 {
    match rotation.rem_euclid(360) {
        90 => egui::pos2(1.0 - p.y, p.x),
        180 => egui::pos2(1.0 - p.x, 1.0 - p.y),
        270 => egui::pos2(p.y, 1.0 - p.x),
        _ => p,
    }
}

pub(crate) fn rotate_rect(rect: Rect, rotation: i16) -> Rect {
    Rect::from_two_pos(
        rotate_point(rect.min, rotation),
        rotate_point(rect.max, rotation),
    )
}

fn screen_rect(bounds: Rect, page: Rect) -> Rect {
    Rect::from_min_max(
        page.min + bounds.min.to_vec2() * page.size(),
        page.min + bounds.max.to_vec2() * page.size(),
    )
}

fn nearest_character(chars: &[TextCharacter], page: Rect, pos: Pos2) -> Option<usize> {
    chars
        .iter()
        .enumerate()
        .filter_map(|(i, ch)| ch.bounds.map(|bounds| (i, bounds)))
        .min_by(|(_, a), (_, b)| {
            let distance = |r: Rect| {
                let r = screen_rect(r, page);
                pos.distance_sq(pos.clamp(r.min, r.max))
            };
            distance(*a).total_cmp(&distance(*b))
        })
        .map(|(i, _)| i)
}

#[derive(Default)]
pub(crate) struct TextSelection {
    range: Option<(usize, usize, usize)>,
}

impl TextSelection {
    pub(crate) fn clear(&mut self) {
        self.range = None;
    }

    pub(crate) fn has_selection(&self) -> bool {
        self.range.is_some()
    }

    pub(crate) fn selected_text(&self, page: usize, chars: &[TextCharacter]) -> Option<String> {
        let (selected_page, anchor, end) = self.range?;
        if selected_page != page {
            return None;
        }
        let range = anchor.min(end)..=anchor.max(end);
        let text: String = chars
            .iter()
            .enumerate()
            .filter(|(i, _)| range.contains(i))
            .map(|(_, ch)| ch.text)
            .collect();
        Some(normalize_extracted_text(&text))
    }

    #[cfg(test)]
    pub(crate) fn text_at(
        &self,
        page: usize,
        chars: &[TextCharacter],
        page_rect: Rect,
        pos: Pos2,
    ) -> Option<String> {
        let (selected_page, anchor, end) = self.range?;
        if selected_page != page {
            return None;
        }
        let range = anchor.min(end)..=anchor.max(end);
        if !chars.iter().enumerate().any(|(i, ch)| {
            range.contains(&i)
                && ch
                    .bounds
                    .is_some_and(|r| screen_rect(r, page_rect).contains(pos))
        }) {
            return None;
        }
        self.selected_text(page, chars)
    }

    pub(crate) fn interact(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        page: usize,
        chars: &[TextCharacter],
    ) {
        let primary = egui::PointerButton::Primary;
        if response.clicked_by(primary) {
            self.clear();
        }
        if response.drag_started_by(primary) {
            self.range = ui
                .input(|i| i.pointer.press_origin())
                .and_then(|pos| nearest_character(chars, response.rect, pos))
                .map(|anchor| (page, anchor, anchor));
        }
        if (response.dragged_by(primary) || response.drag_stopped_by(primary))
            && let Some((selected_page, _, end)) = self.range.as_mut()
            && *selected_page == page
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(index) = nearest_character(chars, response.rect, pos)
        {
            *end = index;
        }
        if response.hovered() && !chars.is_empty() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        if let Some((selected_page, anchor, end)) = self.range
            && selected_page == page
        {
            for bounds in chars
                .iter()
                .take(anchor.max(end) + 1)
                .skip(anchor.min(end))
                .filter_map(|ch| ch.bounds)
            {
                ui.painter().rect_filled(
                    screen_rect(bounds, response.rect),
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(50, 130, 240, 85),
                );
            }
        }
    }
}

/// PDF text layers can encode visual word gaps as several consecutive spaces.
/// Keep explicit line breaks, but normalize horizontal whitespace for copy and translation.
fn normalize_extracted_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn secondary_pan(response: &egui::Response) -> Vec2 {
    if response.dragged_by(egui::PointerButton::Secondary) {
        response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        response.drag_delta()
    } else {
        Vec2::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, pos2, vec2};

    fn characters() -> Vec<TextCharacter> {
        vec![
            Rect::from_min_max(pos2(0.1, 0.1), pos2(0.2, 0.2)),
            Rect::from_min_max(pos2(0.3, 0.1), pos2(0.4, 0.2)),
            Rect::from_min_max(pos2(0.1, 0.4), pos2(0.2, 0.5)),
        ]
        .into_iter()
        .zip(['A', 'B', 'C'])
        .map(|(bounds, text)| TextCharacter {
            text,
            bounds: Some(bounds),
        })
        .collect()
    }

    #[test]
    fn hit_testing_follows_zoom_pan_and_all_rotations() {
        for rotation in [0, 90, 180, -90] {
            let chars: Vec<_> = characters()
                .into_iter()
                .map(|ch| TextCharacter {
                    bounds: ch.bounds.map(|r| rotate_rect(r, rotation)),
                    ..ch
                })
                .collect();
            for size in [vec2(200.0, 300.0), vec2(600.0, 900.0)] {
                let page = Rect::from_min_size(pos2(-35.0, 72.0), size);
                for (index, bounds) in chars.iter().enumerate() {
                    assert_eq!(
                        nearest_character(
                            &chars,
                            page,
                            screen_rect(bounds.bounds.unwrap(), page).center()
                        ),
                        Some(index)
                    );
                }
            }
        }
        assert_eq!(rotate_point(pos2(0.25, 0.75), 90), pos2(0.25, 0.25));
        assert_eq!(rotate_point(pos2(0.25, 0.75), -90), pos2(0.75, 0.75));
    }

    struct Harness {
        ctx: egui::Context,
        selection: TextSelection,
        pan: Vec2,
        menu: bool,
    }

    impl Harness {
        fn new() -> Self {
            let mut this = Self {
                ctx: egui::Context::default(),
                selection: TextSelection::default(),
                pan: Vec2::ZERO,
                menu: false,
            };
            this.frame(vec![]);
            this
        }

        fn frame(&mut self, events: Vec<Event>) {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
                events,
                ..Default::default()
            };
            let _ = self.ctx.run_ui(input, |ui| {
                for page in 0..2 {
                    let rect = Rect::from_min_size(
                        pos2(10.0 + page as f32 * 220.0, 10.0),
                        vec2(200.0, 300.0),
                    );
                    let response =
                        ui.interact(rect, ui.id().with(page), egui::Sense::click_and_drag());
                    self.selection.interact(ui, &response, page, &characters());
                    self.pan += secondary_pan(&response);
                    response.context_menu(|ui| {
                        ui.label("Rotate");
                    });
                    self.menu |= response.context_menu_opened();
                }
            });
        }

        fn button(&mut self, pos: Pos2, button: PointerButton, pressed: bool) {
            self.frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: Default::default(),
                },
            ]);
        }
    }

    #[test]
    fn primary_drag_selects_multiline_range_without_panning() {
        let mut ui = Harness::new();
        ui.button(pos2(260.0, 55.0), PointerButton::Primary, true);
        ui.frame(vec![Event::PointerMoved(pos2(260.0, 145.0))]);
        ui.button(pos2(260.0, 145.0), PointerButton::Primary, false);
        assert_eq!(ui.selection.range, Some((1, 0, 2)));
        assert_eq!(ui.pan, Vec2::ZERO);
        assert!(!ui.menu);
        ui.button(pos2(40.0, 55.0), PointerButton::Primary, true);
        ui.button(pos2(40.0, 55.0), PointerButton::Primary, false);
        assert!(ui.selection.range.is_none());
    }

    #[test]
    fn secondary_drag_pans_without_selection_or_menu() {
        let mut ui = Harness::new();
        ui.button(pos2(40.0, 55.0), PointerButton::Secondary, true);
        ui.frame(vec![Event::PointerMoved(pos2(80.0, 95.0))]);
        ui.button(pos2(80.0, 95.0), PointerButton::Secondary, false);
        assert_eq!(ui.pan, vec2(40.0, 40.0));
        assert!(ui.selection.range.is_none());
        assert!(!ui.menu);
    }

    #[test]
    fn context_actions_require_highlight_hit_and_preserve_unicode_and_line_breaks() {
        let mut chars = characters();
        chars[0].text = '日';
        chars[1].text = '本';
        chars.insert(
            2,
            TextCharacter {
                text: '\r',
                bounds: None,
            },
        );
        chars.insert(
            3,
            TextCharacter {
                text: '\n',
                bounds: None,
            },
        );
        let selection = TextSelection {
            range: Some((1, 4, 0)),
        };
        let page = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 300.0));
        assert_eq!(
            selection
                .text_at(1, &chars, page, pos2(30.0, 45.0))
                .as_deref(),
            Some("日本\nC")
        );
        assert!(
            selection
                .text_at(0, &chars, page, pos2(30.0, 45.0))
                .is_none()
        );
        assert!(
            selection
                .text_at(1, &chars, page, pos2(190.0, 290.0))
                .is_none()
        );
        let empty = TextSelection::default();
        assert!(empty.text_at(1, &chars, page, pos2(30.0, 45.0)).is_none());
    }

    #[test]
    fn secondary_click_opens_context_menu() {
        let mut ui = Harness::new();
        ui.button(pos2(40.0, 55.0), PointerButton::Secondary, true);
        ui.button(pos2(40.0, 55.0), PointerButton::Secondary, false);
        assert!(ui.menu);
        assert_eq!(ui.pan, Vec2::ZERO);
        assert!(ui.selection.range.is_none());
    }
}
