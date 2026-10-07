//! Topoloji's view (docs/adr/0202 §5; the web's `TopologyPanel`): the bar
//! (Denetle, Kurallar…, Kural, Açık / İstisna / Hepsi; the count, Düzelt ▾
//! and İstisna yap), a note when the results may be stale or rules were
//! not checked, and the table of findings.

use iced::widget::{Column, button, container, row, space};
use iced::{Background, Center, Element, Fill, Length};
use kentos_geometry_core::ops::topology_rules as core;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Elided, Menu, MenuButton, Tip, horizontal_divider, tip};

use super::Event;
use super::plan::{self, COLUMNS, Filter, texts};
use crate::app::{App, Message};
use crate::icons::from_web;

/// The widest the bar needs on one line (logical pixels at the default text size):
/// its start, the count with İstisnayı kaldır at its end, and the gaps.
const BAR_WIDTH: f32 = 1060.0;

fn msg(e: Event) -> Message {
    Message::Topology(e)
}

/// The icon of each fix in Düzelt ▾ (the operations they are; the web's `FIX_ICONS`).
fn fix_icon(key: &str) -> &'static str {
    match key {
        "subtractFirst" => "areaSubtract",
        "subtractSecond" => "geoDifference",
        "mergeNeighbour" => "areaUnion",
        "snapEnd" => "extend",
        "repair" => "geoRepair",
        "addVertex" => "vertex",
        "clipOutside" => "geoClip",
        "snapToEnd" => "snapEndpoint",
        "deletePart" | "deleteDuplicate" | "removeVertex" | "deleteObject" => "erase",
        _ => "topologyFix",
    }
}

/// A bar button: its icon and words, its hint above it.
fn bar_button<'a>(
    glyph: &str,
    words: &'a str,
    hint: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    tip(
        button(
            row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                .spacing(6)
                .align_y(Center),
        )
        .style(style::button::secondary)
        .padding([4, 10])
        .on_press_maybe(on),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// A table cell's words: one line, cut with an ellipsis.
fn words<'a>(words: String, quiet: bool, strong: bool) -> Elided<'a> {
    Elided::new(words)
        .size(typography::body())
        .font(if strong {
            typography::ui_strong()
        } else {
            typography::ui()
        })
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(if quiet { t.muted } else { t.text }),
            }
        })
}

/// A table cell, its words at its start.
fn cell<'a>(text: String, quiet: bool, strong: bool) -> Element<'a, Message> {
    words(text, quiet, strong).width(Fill).into()
}

impl App {
    /// The Topoloji tab: its bar, a note, the table.
    pub(crate) fn topology_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted(texts::NO_RULES)).padding(12).into();
        };
        let rules = doc
            .model
            .settings()
            .topology
            .as_ref()
            .map_or(0, |t| t.rules.len());
        let checked = self.topology_checked();
        let stale = self.topology_stale();
        let shown = self.topology_shown();
        let chosen = &self.topology.selected;
        // The bar on one line where it fits, else its end on a second (the web's wrapping bar).
        let bar = iced::widget::responsive(move |room| self.topology_bar(room.width))
            .height(Length::Shrink);

        // A note: the drawing changed since the check, rules not checked.
        let notes: Vec<String> = [
            stale.then(|| texts::STALE.to_owned()),
            checked
                .filter(|c| c.missing > 0)
                .map(|c| plan::missing_layers(c.missing)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let banner = (!notes.is_empty()).then(|| {
            container(notes.into_iter().fold(Column::new().spacing(2), |col, n| {
                col.push(
                    row![
                        icon(from_web(Some("warning")))
                            .size(14.0)
                            .tone(Tone::Warning),
                        label::body(n)
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
            }))
            .padding([4, 10])
            .width(Fill)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(Background::Color(
                    Tokens::of(theme).warning.scale_alpha(0.14),
                )),
                ..container::Style::default()
            })
        });

        let columns = COLUMNS.iter().enumerate().map(|(i, title)| {
            let width = match i {
                0 => Length::Fixed(typography::scaled(56.0)),
                1 => Length::FillPortion(3),
                2 => Length::FillPortion(5),
                3 => Length::FillPortion(4),
                4 => Length::FillPortion(4),
                _ => Length::FillPortion(3),
            };
            let column = TableColumn::new(*title).width(width);
            if i == 0 || i == 5 {
                column.align_right()
            } else {
                column
            }
        });
        let empty = if rules == 0 {
            texts::NO_RULES.to_owned()
        } else {
            match checked {
                None => plan::not_checked(rules),
                Some(c) if c.findings.is_empty() => plan::no_findings(c.rules.len(), c.slots.len()),
                Some(_) => texts::NONE_SHOWN.to_owned(),
            }
        };
        let layers = doc.model.layers();
        let name = |id: &str| {
            layers
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let format = self.format();
        let lines: Vec<(Vec<String>, bool, bool)> = checked.map_or_else(Vec::new, |c| {
            let ids: Vec<u32> = c.slots.iter().map(|s| s.0).collect();
            shown
                .iter()
                .map(|&at| {
                    let f = &c.findings[at];
                    let r = &c.rules[f.rule];
                    (
                        vec![
                            name(&r.layer),
                            plan::rule_text(r, name),
                            core::problem_label(f.problem).to_owned(),
                            plan::objects_text(f, &ids),
                            plan::measure_text(f, &format),
                        ],
                        f.exception,
                        chosen.contains(&at),
                    )
                })
                .collect()
        });
        let first = lines.iter().position(|(_, _, on)| *on);
        let count = lines.len();
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let (cells, exception, on) = &lines[i];
                let quiet = *exception;
                let mut problem = row![].spacing(5).align_y(Center).width(Fill);
                if quiet {
                    problem = problem.push(
                        icon(from_web(Some("topologyException")))
                            .size(12.0)
                            .tone(Tone::Muted),
                    );
                }
                problem = problem.push(cell(cells[2].clone(), quiet, true));
                TableLine::new([
                    label::muted((i + 1).to_string()).into(),
                    cell(cells[0].clone(), quiet, false),
                    cell(cells[1].clone(), quiet, false),
                    problem.into(),
                    cell(cells[3].clone(), quiet, false),
                    container(words(cells[4].clone(), quiet, false))
                        .align_right(Fill)
                        .into(),
                ])
                .selected(*on)
                .current(false)
                .on_press(msg(Event::Press(i)))
            })
            .reveal(first)
            .empty(empty);
        let mut out = Column::new().push(bar);
        if let Some(banner) = banner {
            out = out.push(banner);
        }
        out.push(horizontal_divider())
            .push(table)
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The tab's bar at `width`: Denetle, Kurallar…, Kural, the filter; the
    /// count, Düzelt ▾ and İstisna yap at its end, or under it when narrow.
    fn topology_bar(&self, width: f32) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return space::horizontal().into();
        };
        let rules = doc
            .model
            .settings()
            .topology
            .as_ref()
            .map_or(0, |t| t.rules.len());
        let checked = self.topology_checked();
        let stale = self.topology_stale();
        let shown = self.topology_shown();
        let chosen = &self.topology.selected;

        // The bar's start: the check, the rules, one rule, the filter.
        let names = self.topology_rule_names();
        let mut choices = vec![Choice::new(texts::ALL_RULES)];
        choices.extend(names.iter().enumerate().map(|(i, n)| {
            let count = checked.map(|c| c.findings.iter().filter(|f| f.rule == i).count());
            match count {
                Some(k) => Choice::new(n.clone()).detail(k.to_string()),
                None => Choice::new(n.clone()),
            }
        }));
        let picked = self
            .topology
            .rule
            .filter(|&r| r < names.len())
            .map_or(0, |r| r + 1);
        let rule = Select::new(choices, Some(picked), |i| {
            msg(Event::Rule(if i == 0 { None } else { Some(i - 1) }))
        });
        let filter = Segmented::new(
            [Filter::Open, Filter::Exception, Filter::All],
            self.topology.filter,
            |f| msg(Event::Filter(f)),
        )
        .compact();

        // The bar's end: the count, Düzelt ▾, İstisna yap.
        let count = checked
            .filter(|c| !c.findings.is_empty())
            .map(|c| {
                let open = c.findings.iter().filter(|f| !f.exception).count();
                plan::count_text(shown.len(), open, c.findings.len() - open)
            })
            .unwrap_or_default();
        let one = (chosen.len() == 1 && !stale)
            .then(|| checked.and_then(|c| c.findings.get(chosen[0])))
            .flatten();
        let fixes: Vec<(&'static str, String)> = one
            .map(|f| {
                f.fixes
                    .iter()
                    .map(|k| (*k, self.topology_fix_label(f, k)))
                    .collect()
            })
            .unwrap_or_default();
        let face = container(
            row![
                icon(from_web(Some("topologyFix"))).size(14.0),
                label::body(texts::FIX),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let fix: Element<'_, Message> = if one.is_some() {
            tip(
                MenuButton::new(face, move || {
                    if fixes.is_empty() {
                        return Menu::new().item(texts::FIX_NONE, None);
                    }
                    fixes.iter().fold(Menu::new(), |menu, (key, words)| {
                        menu.item(words.clone(), Some(msg(Event::Fix(key))))
                            .icon(from_web(Some(fix_icon(key))))
                    })
                }),
                Tip::new(texts::FIX).body(texts::FIX_HINT),
                iced::widget::tooltip::Position::Top,
            )
        } else {
            container(face)
                .style(|_| container::Style::default())
                .into()
        };
        let marking = self.topology_marking();
        let mark = bar_button(
            "topologyException",
            if marking { texts::MARK } else { texts::UNMARK },
            if marking {
                texts::MARK_HINT
            } else {
                texts::UNMARK_HINT
            },
            (!chosen.is_empty() && !stale && checked.is_some())
                .then(|| msg(Event::Exception(marking))),
        );
        let start = row![
            bar_button(
                "topologyCheck",
                texts::CHECK,
                texts::CHECK_HINT,
                (rules > 0).then(|| msg(Event::Check))
            ),
            bar_button(
                "topologyRules",
                texts::RULES,
                texts::RULES_HINT,
                Some(msg(Event::Rules))
            ),
            container(rule).width(Length::Fixed(typography::scaled(230.0))),
            filter,
        ]
        .spacing(10)
        .align_y(Center);
        let end = row![label::muted(count), fix, mark]
            .spacing(10)
            .align_y(Center);
        let bar: Element<'_, Message> = if width >= typography::from_default(BAR_WIDTH) {
            row![start, space::horizontal(), end]
                .spacing(10)
                .align_y(Center)
                .into()
        } else {
            Column::new()
                .spacing(6)
                .push(start)
                .push(row![space::horizontal(), end].align_y(Center))
                .into()
        };
        container(bar).padding([6, 10]).width(Fill).into()
    }
}
