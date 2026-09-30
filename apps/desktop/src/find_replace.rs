//! Bul ve değiştir (the web's `ui/text/FindReplaceDialog.ts`, docs/adr/0145
//! §6): Bul and Değiştir, Joker (*), Büyük küçük harf eşleşsin, Tam sözcük
//! and Yalnız seçimde; the matches as rows (layer, the text, what it
//! becomes), each with its box, found again at every change; a row's words
//! take the view to its text and select it. Seçilenleri değiştir writes the
//! checked rows, Hepsini değiştir every row, in one step “Bul ve değiştir”; a
//! text on a locked layer, or one that would become empty, is listed but not
//! written, and that is said. The window stays open for the next search.
//! The matching and the write are `kentos_interaction::find_replace`.

use std::collections::HashSet;

use iced::widget::{Column, Id, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_domain::Slot;
use kentos_geometry_core::text::edit::Find;
use kentos_interaction::Level;
use kentos_interaction::find_replace::{Blocked, Match, Query, matches, write};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const FIND_TITLE: &str = "Bul ve değiştir";
const FIND: &str = "Bul";
const REPLACE: &str = "Değiştir";
const WILDCARD: &str = "Joker (*)";
const MATCH_CASE: &str = "Büyük küçük harf eşleşsin";
const WHOLE_WORD: &str = "Tam sözcük";
const SELECTION_ONLY: &str = "Yalnız seçimde";
const CHOSEN: &str = "Seçilenleri değiştir";
const ALL: &str = "Hepsini değiştir";
const CLOSE: &str = "Kapat";

/// The find field, which takes the keyboard when the window opens (the
/// trace player follows it: traces/command_line.rs).
pub(crate) const FIND_FIELD: &str = "find-replace-find";

/// A row's height in the list of matches, logical pixels.
const ROW: f32 = 26.0;

/// The window: what is looked for, and what it found.
#[derive(Debug, Clone)]
pub struct Window {
    find: String,
    replace: String,
    wildcard: bool,
    match_case: bool,
    whole_word: bool,
    selection_only: bool,
    matches: Vec<Match>,
    /// The rows left unchecked: every other row is checked.
    unchecked: HashSet<Slot>,
    /// The window opened: its find field takes the keyboard.
    focus: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Find(String),
    Replace(String),
    Wildcard(bool),
    MatchCase(bool),
    WholeWord(bool),
    SelectionOnly(bool),
    /// A row's box.
    Check(Slot, bool),
    /// A row's words: to its text on the drawing, selected.
    Go(Slot),
    Chosen,
    All,
    Close,
}

fn msg(event: Event) -> Message {
    Message::FindReplace(event)
}

impl Window {
    fn query(&self) -> Query {
        Query {
            find: self.find.clone(),
            replace: self.replace.clone(),
            how: Find {
                wildcard: self.wildcard,
                caseless: !self.match_case,
                whole_word: self.whole_word && !self.wildcard,
            },
        }
    }

    fn writable(&self) -> impl Iterator<Item = &Match> {
        self.matches.iter().filter(|m| m.blocked.is_none())
    }
}

impl App {
    /// Bul ve değiştir (`text.findReplace`): Yalnız seçimde when something is selected.
    pub(crate) fn open_find_replace(&mut self) {
        self.find_replace = Some(Window {
            find: String::new(),
            replace: String::new(),
            wildcard: false,
            match_case: false,
            whole_word: false,
            selection_only: !self.selection.is_empty(),
            matches: Vec::new(),
            unchecked: HashSet::new(),
            focus: true,
        });
        self.dialog = Some(Dialog::FindReplace);
    }

    /// The matches found again for the window's words and boxes.
    fn find_replace_refresh(&mut self) {
        let (Some(w), Some(doc)) = (self.find_replace.as_mut(), self.document.as_ref()) else {
            return;
        };
        let scope: Option<Vec<Slot>> = w
            .selection_only
            .then(|| self.selection.ids().to_vec());
        w.matches = matches(&doc.model, scope.as_deref(), &w.query());
        let present: HashSet<Slot> = w.matches.iter().map(|m| m.slot).collect();
        w.unchecked.retain(|s| present.contains(s));
    }

    pub(crate) fn find_replace_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.find_replace.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Find(t) => w.find = t,
            Event::Replace(t) => w.replace = t,
            Event::Wildcard(on) => w.wildcard = on,
            Event::MatchCase(on) => w.match_case = on,
            Event::WholeWord(on) => w.whole_word = on,
            Event::SelectionOnly(on) => w.selection_only = on,
            Event::Check(slot, on) => {
                if on {
                    w.unchecked.remove(&slot);
                } else {
                    w.unchecked.insert(slot);
                }
                return Task::none();
            }
            Event::Go(slot) => {
                self.selection.set([slot]);
                return self.run("view.zoomSelection");
            }
            Event::Chosen => self.find_replace_write(true),
            Event::All => self.find_replace_write(false),
            Event::Close => {
                self.find_replace = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.find_replace_refresh();
        Task::none()
    }

    /// Writes the checked rows (`chosen`) or every row, one step; says how many.
    fn find_replace_write(&mut self, chosen: bool) {
        let (Some(w), Some(doc)) = (self.find_replace.as_mut(), self.document.as_mut()) else {
            return;
        };
        let picked: Vec<Match> = w
            .writable()
            .filter(|m| !chosen || !w.unchecked.contains(&m.slot))
            .cloned()
            .collect();
        let blocked = w.matches.iter().filter(|m| m.blocked.is_some()).count();
        let (n, said) = write(&mut doc.model, &picked);
        w.unchecked.clear();
        for text in said {
            self.warn(text);
        }
        if n > 0 {
            self.say(Level::Success, format!("{n} yazı değiştirildi."));
        }
        if blocked > 0 && !chosen {
            self.warn(format!(
                "{blocked} yazı değiştirilmedi: kilitli katmanda ya da boş kalacaktı."
            ));
        }
    }

    /// The window opened: the keyboard to its find field.
    pub(crate) fn find_replace_tasks(&mut self) -> Task<Message> {
        let Some(w) = self.find_replace.as_mut() else {
            return Task::none();
        };
        if !std::mem::take(&mut w.focus) {
            return Task::none();
        }
        iced::widget::operation::focus(Id::new(FIND_FIELD))
    }

    pub(crate) fn find_replace_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.find_replace else {
            return text("").into();
        };
        let find = text_input(
            "Aranacak yazı; * herhangi bir dizi (Joker açıkken)",
            &w.find,
        )
        .id(Id::new(FIND_FIELD))
        .on_input(|t| msg(Event::Find(t)))
        .padding([5, 8])
        .style(style::field::input);
        let replace = text_input("Yeni yazı; boş bırakılırsa silinir", &w.replace)
            .on_input(|t| msg(Event::Replace(t)))
            .padding([5, 8])
            .style(style::field::input);
        let fields = row![
            container(words::field(FIND, focus_ring(find), None)).width(Fill),
            container(words::field(REPLACE, focus_ring(replace), None)).width(Fill),
        ]
        .spacing(18);
        let nothing_selected = self.selection.is_empty() && !w.selection_only;
        let options = row![
            words::check(
                w.wildcard,
                WILDCARD,
                Some(msg(Event::Wildcard(!w.wildcard)))
            ),
            words::check(
                w.match_case,
                MATCH_CASE,
                Some(msg(Event::MatchCase(!w.match_case)))
            ),
            words::check(
                w.whole_word && !w.wildcard,
                WHOLE_WORD,
                (!w.wildcard).then(|| msg(Event::WholeWord(!w.whole_word))),
            ),
            words::check(
                w.selection_only,
                SELECTION_ONLY,
                (!nothing_selected).then(|| msg(Event::SelectionOnly(!w.selection_only))),
            ),
        ]
        .spacing(18)
        .align_y(Center);

        let locked = w
            .matches
            .iter()
            .filter(|m| m.blocked == Some(Blocked::Locked))
            .count();
        let empty = w
            .matches
            .iter()
            .filter(|m| m.blocked == Some(Blocked::Empty))
            .count();
        let mut lines = vec![if w.find.is_empty() {
            words::text_line(
                Kind::Info,
                "Aranacak yazıyı yazın. Joker açıkken * herhangi bir dizidir ve kalıp bütün yazıya uyar; değiştirmedeki * sırasıyla onların yerine geçer.",
            )
        } else if w.matches.is_empty() {
            words::text_line(
                Kind::Warn,
                if w.selection_only {
                    "Eşleşen yazı yok (yalnız seçimde arandı)."
                } else {
                    "Eşleşen yazı yok."
                },
            )
        } else {
            words::text_line(Kind::Info, format!("{} eşleşme.", w.matches.len()))
        }];
        if locked > 0 {
            lines.push(words::text_line(
                Kind::Warn,
                format!("{locked} yazı kilitli katmanda: listelenir, değiştirilmez."),
            ));
        }
        if empty > 0 {
            lines.push(words::text_line(
                Kind::Warn,
                format!("{empty} yazı boş kalacağı için değiştirilmez."),
            ));
        }

        let head = row![
            container(text("")).width(typography::scaled(28.0)),
            container(label::caption("Katman")).width(Fill),
            container(label::caption("Metin")).width(Fill),
            container(label::caption("Yeni metin")).width(Fill),
        ]
        .spacing(8)
        .padding([4, 8]);
        let list = VirtualList::new(w.matches.len(), typography::scaled(ROW), move |i| {
            let m = &w.matches[i];
            let checked = m.blocked.is_none() && !w.unchecked.contains(&m.slot);
            let boxed = check_box(
                if checked {
                    Check::Checked
                } else {
                    Check::Unchecked
                },
                m.blocked
                    .is_none()
                    .then(|| msg(Event::Check(m.slot, !checked))),
            );
            let quiet = m.blocked.is_some();
            let cell = move |words: String| {
                Elided::new(words)
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(move |theme: &iced::Theme| {
                        let t = Tokens::of(theme);
                        iced::widget::text::Style {
                            color: Some(if quiet { t.muted } else { t.text }),
                        }
                    })
                    .width(Fill)
            };
            let layer = match m.blocked {
                Some(Blocked::Locked) => row![
                    icon(Icon::Lock).size(12.0).tone(Tone::Muted),
                    cell(m.layer.clone())
                ]
                .spacing(4)
                .align_y(Center)
                .width(Fill),
                _ => row![cell(m.layer.clone())].width(Fill),
            };
            let new = if m.blocked == Some(Blocked::Empty) {
                "(boş)".to_owned()
            } else {
                m.new.clone()
            };
            let new = Elided::new(new)
                .size(typography::caption())
                .font(typography::ui())
                .style(move |theme: &iced::Theme| {
                    let t = Tokens::of(theme);
                    iced::widget::text::Style {
                        color: Some(if quiet { t.muted } else { t.accent_hover }),
                    }
                })
                .width(Fill);
            let words = button(
                row![layer, cell(m.old.clone()), new]
                    .spacing(8)
                    .align_y(Center),
            )
            .on_press(msg(Event::Go(m.slot)))
            .padding(0)
            .style(style::button::ghost)
            .width(Fill);
            row![
                container(boxed)
                    .width(typography::scaled(28.0))
                    .center_x(typography::scaled(28.0)),
                words,
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center)
            .into()
        })
        .height(typography::scaled(240.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);

        let any = w.writable().next().is_some();
        let some_checked = w.writable().any(|m| !w.unchecked.contains(&m.slot));
        let body = Column::new()
            .spacing(12)
            .push(fields)
            .push(options)
            .push(words::summary(lines))
            .push(table);
        overlay::modal(
            Frame::new(FIND_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .action(words::secondary(
                    CHOSEN,
                    some_checked.then(|| msg(Event::Chosen)),
                ))
                .action(words::primary(ALL, any.then(|| msg(Event::All))))
                .width(680.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): Bul
    /// and Değiştir, the four boxes and the three buttons.
    pub(crate) fn find_replace_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.find_replace else {
            return Err(format!("{FIND_TITLE} penceresi açık değil"));
        };
        let set = |now: bool, on: bool, event: Event| (now != on).then(|| msg(event));
        Ok(match control {
            Control::Fill(FIND, t) => Some(msg(Event::Find(t.to_owned()))),
            Control::Fill(REPLACE, t) => Some(msg(Event::Replace(t.to_owned()))),
            Control::Check(WILDCARD, on) => set(w.wildcard, on, Event::Wildcard(on)),
            Control::Check(MATCH_CASE, on) => set(w.match_case, on, Event::MatchCase(on)),
            Control::Check(WHOLE_WORD, on) => set(w.whole_word, on, Event::WholeWord(on)),
            Control::Check(SELECTION_ONLY, on) => {
                set(w.selection_only, on, Event::SelectionOnly(on))
            }
            Control::Press(CHOSEN) => w
                .writable()
                .any(|m| !w.unchecked.contains(&m.slot))
                .then(|| msg(Event::Chosen)),
            Control::Press(ALL) => w.writable().next().is_some().then(|| msg(Event::All)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{FIND_TITLE}” penceresinde {other} yok")),
        })
    }
}
