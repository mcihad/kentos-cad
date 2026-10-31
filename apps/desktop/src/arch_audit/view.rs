//! The Denetim tab drawn (arch_audit.rs): its bar (Denetle, Grup, Kapsam, İstisnalar ▾, Eşikler ▾; the count, Kata
//! ver ▾, İstisna yap…, Düzelt), the note when the check is old, İstisna yap's question and the table with each row's
//! menu (docs/adr/0703, 0717, 0732 §5).

use iced::widget::{Column, button, container, row, space};
use iced::{Background, Center, Element, Fill, Length};
use kentos_commands::ArchAuditCheck;
use kentos_ui::icon::{Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Elided, Tip, horizontal_divider, tip};

use super::{
    ALL, AREAS, AuditScope, CLEARS, BUTTONS, CHECK, CHECK_HINT, CLEAN, COLUMNS, EXCEPT, EXCEPT_HINT, Event,
    FIX, GAPS, Ink, LENGTHS, LEVELS, Line, NONE_SHOWN, NOT_CHECKED, NOTE_FIELD, RAMP_PROFILES,
    SCOPES, STAIR_PROFILES, STALE, START, TEXT_ADVANCE, UNEXCEPT, VIEWS, WIDTHS, bar_button, cell,
    group_icon, msg,
};
use crate::app::{App, Message};
use crate::icons::from_web;

/// One of Eşikler ▾'s submenus: its name, the value chosen, and each choice's words, whether it is the one chosen, and
/// its message.
type Limit = (&'static str, String, Vec<(String, bool, Message)>);

impl App {
    /// The Denetim tab: its bar, a note when the check is old, the table.
    pub(crate) fn arch_audit_tab(&self) -> Element<'_, Message> {
        let bar = iced::widget::responsive(move |room| self.arch_audit_bar(room.width))
            .height(Length::Shrink);
        let stale = self.arch_audit_stale();
        let audited = self.arch_audit.audited.as_ref();
        let shown = self.arch_audit_shown();
        let columns = COLUMNS.iter().enumerate().map(|(i, title)| {
            let width = match i {
                0 => Length::Fixed(typography::scaled(44.0)),
                1 => Length::FillPortion(2),
                2 => Length::FillPortion(6),
                3 => Length::FillPortion(5),
                _ => Length::FillPortion(2),
            };
            let column = TableColumn::new(*title).width(width);
            if i == 0 { column.align_right() } else { column }
        });
        let empty = match audited {
            None => NOT_CHECKED,
            Some(a) if a.findings.is_empty() => CLEAN,
            Some(_) => NONE_SHOWN,
        };
        let lines: Vec<Line> = audited.map_or_else(Vec::new, |a| {
            shown
                .iter()
                .map(|&i| {
                    let f = &a.findings[i];
                    let objects = match f.uids.len() {
                        1 => "1 nesne".to_owned(),
                        n => format!("{n} nesne"),
                    };
                    // An exception's row says so and its note; a lapsed one's says it lapsed (docs/adr/0732 §5).
                    let (message, ink) = if f.exception {
                        let note = f
                            .note
                            .as_deref()
                            .map_or_else(String::new, |n| format!(" — {n}"));
                        (format!("İstisna · {}{note}", f.message), Ink::Quiet)
                    } else if f.lapsed {
                        (format!("İstisna düştü · {}", f.message), Ink::Warning)
                    } else {
                        (
                            f.message.clone(),
                            if stale { Ink::Quiet } else { Ink::Plain },
                        )
                    };
                    Line {
                        check: f.check,
                        cells: [
                            f.check.label().to_owned(),
                            message,
                            f.repair.clone(),
                            objects,
                        ],
                        ink,
                        chosen: self.arch_audit.selected == Some(i)
                            || self.arch_audit.chosen.contains(&i),
                        current: self.arch_audit.selected == Some(i),
                        fix: f.fix.as_ref().map(|x| x.label.clone()),
                        exception: f.exception || f.lapsed,
                    }
                })
                .collect()
        });
        let first = lines.iter().position(|l| l.current);
        let count = lines.len();
        let many = self.arch_audit.chosen.len() > 1;
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let line = &lines[i];
                let group = row![
                    icon(from_web(Some(group_icon(line.check))))
                        .size(13.0)
                        .tone(Tone::Muted),
                    cell(
                        line.cells[0].clone(),
                        if line.ink == Ink::Quiet {
                            Ink::Quiet
                        } else {
                            Ink::Plain
                        }
                    )
                ]
                .spacing(6)
                .align_y(Center);
                let menu_fix = line.fix.clone();
                let exception = line.exception;
                let chosen = line.chosen;
                TableLine::new([
                    label::muted((i + 1).to_string()).into(),
                    group.into(),
                    cell(line.cells[1].clone(), line.ink),
                    cell(line.cells[2].clone(), Ink::Quiet),
                    cell(line.cells[3].clone(), Ink::Quiet),
                ])
                .selected(line.chosen)
                .current(many && line.current)
                .on_press(msg(Event::Press(i)))
                .menu(move |_| {
                    // On a row among those chosen the menu speaks for them all.
                    let all = chosen && many;
                    let mut menu = kentos_ui::widget::Menu::new()
                        .item("Seç ve göster", msg(Event::Press(i)))
                        .icon(from_web(Some("zoomSelection")));
                    if let Some(label) = &menu_fix {
                        let words = if all {
                            format!("{FIX} (seçililer)")
                        } else {
                            format!("{FIX}: {label}")
                        };
                        menu = menu
                            .item(words, (!stale).then(|| msg(Event::FixRow(i))))
                            .icon(from_web(Some("topologyFix")));
                    }
                    menu = menu.separator();
                    if exception {
                        menu = menu.item(UNEXCEPT, msg(Event::Unexcept(Some(i))));
                    } else {
                        menu = menu.item(
                            format!("{EXCEPT}…"),
                            (!stale).then(|| msg(Event::Except(Some(i)))),
                        );
                    }
                    menu
                })
            })
            .reveal(first)
            .empty(empty);
        let mut out = Column::new().push(bar);
        if stale {
            out = out.push(
                container(
                    row![
                        icon(from_web(Some("warning")))
                            .size(14.0)
                            .tone(Tone::Warning),
                        label::body(STALE),
                        space::horizontal(),
                        bar_button("archAudit", CHECK, CHECK_HINT, Some(msg(Event::Check))),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .padding([4, 10])
                .width(Fill)
                .style(|theme: &iced::Theme| container::Style {
                    background: Some(Background::Color(
                        Tokens::of(theme).warning.scale_alpha(0.14),
                    )),
                    ..container::Style::default()
                }),
            );
        }
        if let Some(note) = &self.arch_audit.note {
            out = out.push(self.arch_audit_note_row(note));
        }
        out.push(horizontal_divider())
            .push(table)
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// İstisna yap's question: the note (optional), Tamam and Vazgeç; Enter gives it, Esc leaves it.
    fn arch_audit_note_row<'a>(&'a self, note: &'a str) -> Element<'a, Message> {
        let n = self
            .arch_audit_chosen()
            .iter()
            .filter(|f| !f.exception)
            .count();
        let field = kentos_ui::widget::text_field("Neden bırakıldı? (isteğe bağlı)", note)
            .on_input(|t| msg(Event::Note(t)))
            .on_submit(msg(Event::ExceptConfirm))
            .id(iced::widget::Id::new(NOTE_FIELD));
        container(
            row![
                icon(from_web(Some("topologyException"))).size(14.0),
                label::body(match n {
                    1 => "1 bulguyu istisna yap:".to_owned(),
                    n => format!("{n} bulguyu istisna yap:"),
                }),
                container(field).width(Length::Fixed(typography::scaled(420.0))),
                button(label::body("Tamam"))
                    .style(style::button::primary)
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(msg(Event::ExceptConfirm)),
                button(label::body("Vazgeç"))
                    .style(style::button::secondary)
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(msg(Event::ExceptCancel)),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([4, 10])
        .width(Fill)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(Background::Color(
                Tokens::of(theme).accent.scale_alpha(0.10),
            )),
            ..container::Style::default()
        })
        .into()
    }

    /// The bar at `width`: Denetle, the group, Kapsam, İstisnalar ▾ and Eşikler ▾; the count, Kata ver ▾, İstisna yap…
    /// and Düzelt at its end. Narrow: the count and the buttons on a second line.
    fn arch_audit_bar(&self, width: f32) -> Element<'_, Message> {
        let audited = self.arch_audit.audited.as_ref();
        let count_of = |g: Option<ArchAuditCheck>| {
            audited.map(|a| {
                a.findings
                    .iter()
                    .filter(|f| g.is_none_or(|g| f.check == g))
                    .count()
            })
        };
        let mut choices = vec![match count_of(None) {
            Some(n) => Choice::new(ALL).detail(n.to_string()),
            None => Choice::new(ALL),
        }];
        choices.extend(ArchAuditCheck::ALL.iter().map(|g| {
            let c = Choice::new(g.label()).icon(from_web(Some(group_icon(*g))));
            match count_of(Some(*g)) {
                Some(n) => c.detail(n.to_string()),
                None => c,
            }
        }));
        let picked = self
            .arch_audit
            .group
            .and_then(|g| ArchAuditCheck::ALL.iter().position(|x| *x == g))
            .map_or(0, |i| i + 1);
        let group = Select::new(choices, Some(picked), |i| {
            msg(Event::Group(if i == 0 {
                None
            } else {
                Some(ArchAuditCheck::ALL[i - 1])
            }))
        });
        let scope_now = match self.arch_audit.scope {
            AuditScope::All => 0,
            AuditScope::ActiveLevel => 1,
            AuditScope::Selection(_) => 2,
        };
        let scope = Select::new(
            SCOPES
                .iter()
                .enumerate()
                .map(|(i, w)| match (&self.arch_audit.scope, i) {
                    (AuditScope::Selection(uids), 2) => {
                        Choice::new(format!("{w} ({})", uids.len()))
                    }
                    _ => Choice::new(*w),
                })
                .collect::<Vec<_>>(),
            Some(scope_now),
            |i| msg(Event::Scope(i)),
        );
        let scope = tip(
            container(scope).width(Length::Fixed(typography::scaled(150.0))),
            Tip::new("Kapsam").body(
                "Bütün çizim, etkin katın nesneleri ya da seçildiği andaki seçili nesneler. Nesnelerinden biri kapsamdaki bulgu listelenir; dışarıda kalanlar sayılır.",
            ),
            iced::widget::tooltip::Position::Top,
        );
        let start = row![
            bar_button("archAudit", CHECK, CHECK_HINT, Some(msg(Event::Check))),
            container(group).width(Length::Fixed(typography::scaled(160.0))),
            scope,
            self.arch_audit_exceptions_button(),
            self.arch_audit_limits(),
        ]
        .spacing(10)
        .align_y(Center);
        let count = audited.map_or_else(String::new, |a| {
            let n = a.findings.len();
            let fixable = a.findings.iter().filter(|f| f.fix.is_some()).count();
            let mut words = match n {
                0 => "Bulgu yok".to_owned(),
                n => format!("{n} bulgu · {fixable} düzeltilebilir"),
            };
            if a.outside > 0 {
                words.push_str(&format!(" · {} kapsam dışında", a.outside));
            }
            words
        });
        let stale = self.arch_audit_stale();
        let chosen = self.arch_audit_chosen();
        let fixes: Vec<&str> = chosen
            .iter()
            .filter_map(|f| f.fix.as_ref())
            .map(|f| f.label.as_str())
            .collect();
        let levels = self.arch_audit_levels_button();
        // One line when the count and the buttons fit beside the controls (their words measured roughly); else the
        // count and the buttons on a line of their own. Düzelt says its one fix when there is room for it.
        let words = |w: &str| w.chars().count() as f32 * TEXT_ADVANCE;
        let end_of = |fix: f32| BUTTONS + fix + if levels.is_some() { LEVELS } else { 0.0 };
        let long_fix = match fixes.as_slice() {
            [one] => words(one) + words("Düzelt: ") + 40.0,
            _ => 110.0,
        };
        let room = |fix: f32| typography::from_default(START + end_of(fix) + words(&count) + 40.0);
        let wide = width >= room(110.0);
        let long = width >= room(long_fix)
            || (!wide
                && width >= typography::from_default(end_of(long_fix) + words(&count) + 40.0));
        let count = container(
            Elided::new(count)
                .size(typography::body())
                .font(typography::ui())
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).muted),
                })
                .width(Fill),
        )
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right);
        let fix_hint = match fixes.as_slice() {
            [] => "Seçili bulgunun açık bir düzeltmesi yok; Ne yapılır sütununa göre elle düzeltin. Ctrl/⌘ ile birden çok, Shift ile aralık seçilir.".to_owned(),
            [one] => format!("{one}. Çizimde kesikli önizlenir. Tek geri alma adımıdır; denetim kendiliğinden yinelenir."),
            many => format!(
                "Seçili {} bulgunun düzeltmeleri tek adımda (“Denetim düzeltmeleri”); biri yazılamazsa hiçbiri yazılmaz. Çizimde kesikli önizlenir.",
                many.len()
            ),
        };
        let fix_words = match fixes.as_slice() {
            [] => FIX.to_owned(),
            [one] if long => format!("{FIX}: {one}"),
            list => format!("{FIX} ({})", list.len()),
        };
        let fix = tip(
            button(
                row![
                    icon(from_web(Some("topologyFix"))).size(14.0),
                    label::body(fix_words)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .style(style::button::primary)
            .padding(kentos_ui::theme::metrics::action())
            .on_press_maybe((!fixes.is_empty() && !stale).then(|| msg(Event::Fix))),
            Tip::new(FIX).body(fix_hint),
            iced::widget::tooltip::Position::Top,
        );
        // İstisna yap… on open findings, İstisnayı kaldır on exceptions.
        let open = chosen.iter().filter(|f| !f.exception).count();
        let marked = chosen.iter().filter(|f| f.exception || f.lapsed).count();
        let except: Element<'_, Message> = if marked > 0 && open == 0 {
            bar_button(
                "topologyException",
                UNEXCEPT,
                "Seçili bulguların istisnasını kaldırır; bulgular yeniden açık listelenir. Tek geri alma adımıdır.",
                Some(msg(Event::Unexcept(None))),
            )
        } else {
            bar_button(
                "topologyException",
                "İstisna yap…",
                EXCEPT_HINT,
                (open > 0 && !stale).then(|| msg(Event::Except(None))),
            )
        };
        let mut end = row![].spacing(10).align_y(Center);
        if let Some(levels) = levels {
            end = end.push(levels);
        }
        let end = end.push(except).push(fix);
        let bar: Element<'_, Message> = if wide {
            row![start, count, end].spacing(12).align_y(Center).into()
        } else {
            Column::new()
                .spacing(6)
                .push(start)
                .push(row![count, end].spacing(12).align_y(Center))
                .into()
        };
        container(bar).padding([6, 10]).width(Fill).into()
    }

    /// İstisnalar ▾ (docs/adr/0732 §3): how the exceptions are listed, and Eski istisnaları temizle; how many there are
    /// on its face.
    fn arch_audit_exceptions_button(&self) -> Element<'_, Message> {
        let audited = self.arch_audit.audited.as_ref();
        let excepted = audited.map_or(0, |a| a.excepted);
        let stale = audited.map_or(0, |a| a.stale.len());
        let now = self.arch_audit.view;
        let menu = move || {
            let mut menu = kentos_ui::widget::Menu::new().header("Listede");
            for (i, (view, words)) in VIEWS.iter().enumerate() {
                menu = menu.radio(*words, *view == now, msg(Event::View(i)));
            }
            menu.separator()
                .item(
                    format!("Eski istisnaları temizle ({stale})"),
                    (stale > 0).then(|| msg(Event::Prune)),
                )
                .detail("Bulgusu artık bulunmayan istisnaları siler. Tek geri alma adımıdır.")
        };
        let mut face = row![
            icon(from_web(Some("topologyException"))).size(14.0),
            label::body(format!("İstisnalar ({excepted})")),
        ]
        .spacing(6)
        .align_y(Center);
        if stale > 0 {
            face = face.push(
                icon(from_web(Some("warning")))
                    .size(12.0)
                    .tone(Tone::Warning),
            );
        }
        let face = face.push(
            icon(from_web(Some("chevronDown")))
                .size(12.0)
                .tone(Tone::Muted),
        );
        tip(
            kentos_ui::widget::MenuButton::new(container(face).padding([4, 10]), menu),
            Tip::new("İstisnalar").body(
                "Bilerek bırakılmış bulgular: Gizle onları listelemez, Göster soluk listeler, Yalnız istisnalar yalnız onları. Bulgusu değişen istisna düşer ve uyarı renginde açık listelenir.",
            ),
            iced::widget::tooltip::Position::Top,
        )
    }

    /// Eşikler ▾ (docs/adr/0703 §1, 0723 §4, 0724 §2, 0732 §4): each threshold and profile a submenu, its value beside
    /// it; the values in its tip.
    fn arch_audit_limits(&self) -> Element<'_, Message> {
        let format = self.format();
        let a = &self.arch_audit;
        let length = |v: f64| format.length(v);
        let area = |v: f64| format!("{v:.2} m²");
        let clear = |v: f64| if v > 0.0 { format.length(v) } else { "Yok".to_owned() };
        let values =
            |list: &[f64], now: f64, show: &dyn Fn(f64) -> String, pick: fn(usize) -> Event| {
                list.iter()
                    .enumerate()
                    .map(|(i, v)| (show(*v), (v - now).abs() < 1e-12, msg(pick(i))))
                    .collect::<Vec<_>>()
            };
        let profiles = |list: &[(Option<&'static str>, &'static str)],
                        now: Option<&'static str>,
                        pick: fn(usize) -> Event| {
            list.iter()
                .enumerate()
                .map(|(i, (id, words))| ((*words).to_owned(), *id == now, msg(pick(i))))
                .collect::<Vec<_>>()
        };
        let named = |list: &[(Option<&'static str>, &'static str)], now: Option<&'static str>| {
            list.iter()
                .find(|(id, _)| *id == now)
                .map_or("Yok", |(_, w)| *w)
                .to_owned()
        };
        let groups: Vec<Limit> = vec![
            (
                "Uç boşluğu ≤",
                length(a.max_gap),
                values(&GAPS, a.max_gap, &length, Event::Gap),
            ),
            (
                "Kısa duvar <",
                length(a.min_length),
                values(&LENGTHS, a.min_length, &length, Event::Length),
            ),
            (
                "Küçük mahal <",
                area(a.min_area),
                values(&AREAS, a.min_area, &area, Event::Area),
            ),
            (
                "Dar mahal <",
                length(a.min_width),
                values(&WIDTHS, a.min_width, &length, Event::Width),
            ),
            (
                "Net geçiş <",
                clear(a.min_clear),
                values(&CLEARS, a.min_clear, &clear, Event::Clear),
            ),
        ];
        let kinds: Vec<Limit> = vec![
            (
                "Rampa",
                named(&RAMP_PROFILES, a.ramp_profile),
                profiles(&RAMP_PROFILES, a.ramp_profile, Event::RampProfile),
            ),
            (
                "Merdiven",
                named(&STAIR_PROFILES, a.stair_profile),
                profiles(&STAIR_PROFILES, a.stair_profile, Event::StairProfile),
            ),
        ];
        let summary = groups
            .iter()
            .chain(kinds.iter())
            .map(|(name, now, _)| format!("{name} {now}"))
            .collect::<Vec<_>>()
            .join(" · ");
        let notes: Vec<String> = [a.ramp_profile, a.stair_profile]
            .into_iter()
            .flatten()
            .filter_map(kentos_commands::arch_profile)
            .map(|p| format!("{}. {}", p.name, p.note))
            .collect();
        let menu = move || {
            let mut menu = kentos_ui::widget::Menu::new().header("Eşikler");
            for (name, now, items) in &groups {
                let mut sub = kentos_ui::widget::Menu::new();
                for (words, on, message) in items {
                    sub = sub.radio(words.clone(), *on, message.clone());
                }
                menu = menu.submenu(*name, sub).hint(now.clone());
            }
            menu = menu.separator().header("Profiller");
            for (name, now, items) in &kinds {
                let mut sub = kentos_ui::widget::Menu::new();
                for (words, on, message) in items {
                    sub = sub.radio(words.clone(), *on, message.clone());
                }
                menu = menu.submenu(*name, sub).hint(now.clone());
            }
            menu
        };
        let face = row![
            icon(from_web(Some("settings"))).size(14.0),
            label::body("Eşikler"),
            icon(from_web(Some("chevronDown")))
                .size(12.0)
                .tone(Tone::Muted),
        ]
        .spacing(6)
        .align_y(Center);
        let mut body = summary;
        for n in notes {
            body.push('\n');
            body.push_str(&n);
        }
        tip(
            kentos_ui::widget::MenuButton::new(container(face).padding([4, 10]), menu),
            Tip::new("Eşikler ve profiller").body(body),
            iced::widget::tooltip::Position::Top,
        )
    }

    /// Kata ver ▾ while the check found objects without a level (docs/adr/0717 §2): Seç ve göster, then the project's
    /// levels, Etkin kat marked; none otherwise.
    fn arch_audit_levels_button(&self) -> Option<Element<'_, Message>> {
        let count = self.arch_audit_levelless().len();
        if count == 0 {
            return None;
        }
        let levels: Vec<(String, String)> = self
            .document
            .as_ref()?
            .model
            .settings()
            .arch
            .as_ref()?
            .levels
            .iter()
            .map(|l| (l.id.clone(), l.name.clone()))
            .collect();
        let active = self.active_level();
        let stale = self.arch_audit_stale();
        let face = row![
            icon(from_web(Some("archSettings"))).size(14.0),
            label::body(format!("Kata ver ({count})")),
            icon(from_web(Some("chevronDown")))
                .size(12.0)
                .tone(Tone::Muted),
        ]
        .spacing(6)
        .align_y(Center);
        let menu = move || {
            let mut menu = kentos_ui::widget::Menu::new()
                .header(format!("Katsız {count} nesne"))
                .item("Seç ve göster", msg(Event::ShowLevelless))
                .icon(from_web(Some("zoomSelection")))
                .separator()
                .header("Kata ver: hepsi tek adımda");
            for (id, name) in &levels {
                let words = if active.as_deref() == Some(id.as_str()) {
                    format!("{name} · etkin kat")
                } else {
                    name.clone()
                };
                menu = menu.item(words, (!stale).then(|| msg(Event::AssignLevel(id.clone()))));
            }
            menu
        };
        Some(tip(
            kentos_ui::widget::MenuButton::new(container(face).padding([4, 10]), menu),
            Tip::new("Kata ver").body(
                "Katsız ya da projede olmayan bir kattaki duvar, mahal, kolon, merdiven, rampa ve mahal ayırıcılara bir kat verir; yerleri değişmez. Seç ve göster önce onları seçer. Tek geri alma adımıdır.",
            ),
            iced::widget::tooltip::Position::Top,
        ))
    }
}
