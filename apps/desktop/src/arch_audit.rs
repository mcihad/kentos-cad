//! Denetim, the bottom panel's tab of the architectural audit (docs/adr/0703; `arch.audit`): `cad.arch.audit`'s
//! findings in a table, by group; a row selects its objects and fits the view on them (Archicad's “Seçili bölgelere
//! yakınlaştır”); Düzelt runs the chosen finding's fix — its command call, one undo step — and checks again. The
//! thresholds are the bar's choices. The drawing changes only by a fix.

use iced::widget::{Column, button, container, row, space};
use iced::{Background, Center, Element, Fill, Length, Task};
use kentos_commands::{
    ARCH_AUDIT_MAX_GAP, ARCH_AUDIT_MIN_AREA, ARCH_AUDIT_MIN_WIDTH, ArchAudit, ArchAuditCheck, ArchAuditFinding, CallOp,
    CommandResult,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, arch_audit, registry};
use kentos_ui::icon::{Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Elided, Tip, horizontal_divider, tip};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::icons::from_web;

/// The tab's title, which a trace names it by.
pub(crate) const TAB: &str = "Denetim";
const CHECK: &str = "Denetle";
const CHECK_HINT: &str = "Duvar birleşimlerini, uç boşluklarını, kopuk açıklık, mahal, ölçü, kolon ve işaretleri, yinelenen numaraları, katı olmayan nesneleri, seçilen profillere göre rampaları ve merdivenleri bulur. Çizimi değiştirmez.";
const FIX: &str = "Düzelt";
const STALE: &str = "Çizim denetimden sonra değişti; yeniden denetleyin.";
const NOT_CHECKED: &str = "Denetle ile çizimin mimari sorunlarını bulun.";
const CLEAN: &str = "Bulgu yok. Bu, ruhsat ya da standart uygunluğunu göstermez.";
const NONE_SHOWN: &str = "Bu grupta bulgu yok.";
const ALL: &str = "Bütün gruplar";
const COLUMNS: [&str; 5] = ["#", "Grup", "Bulgu", "Ne yapılır", "Nesne"];

/// The thresholds a bar offers (docs/adr/0703 §1), metres and square metres.
const GAPS: [f64; 5] = [0.02, 0.05, ARCH_AUDIT_MAX_GAP, 0.20, 0.50];
const AREAS: [f64; 5] = [0.05, ARCH_AUDIT_MIN_AREA, 0.25, 0.50, 1.00];
const WIDTHS: [f64; 4] = [0.10, ARCH_AUDIT_MIN_WIDTH, 0.20, 0.30];
/// The ramp profiles a bar offers (docs/adr/0723 §4): none, or one of the catalog's, by its words.
const RAMP_PROFILES: [(Option<&str>, &str); 2] = [(None, "Yok"), (Some("erisilebilirlik-rampa/1"), "TS 9111 özeti")];
/// The stair profiles a bar offers (docs/adr/0724 §2): none, or one of the catalog's, by its words.
const STAIR_PROFILES: [(Option<&str>, &str); 5] = [
    (None, "Yok"),
    (Some("imar-merdiven-konut/1"), "Konut, asansörlü"),
    (Some("imar-merdiven-konut-asansorsuz/1"), "Konut, asansörsüz"),
    (Some("imar-merdiven-diger/1"), "Diğer yapılar"),
    (Some("imar-merdiven-daire-ici/1"), "Daire içi"),
];
/// The widest the bar needs on one line (logical pixels at the default text size).
const BAR_WIDTH: f32 = 1640.0;

/// The last check: its findings and the drawing's state it saw.
#[derive(Clone, Debug)]
pub(crate) struct Audited {
    pub findings: Vec<ArchAuditFinding>,
    pub generation: u64,
    pub revision: u64,
}

/// The tab's state for as long as the app lives.
#[derive(Clone, Debug)]
pub(crate) struct ArchAuditPanel {
    pub audited: Option<Audited>,
    /// The chosen finding, by its place in the findings.
    pub selected: Option<usize>,
    /// One group; none: every group.
    pub group: Option<ArchAuditCheck>,
    pub max_gap: f64,
    pub min_area: f64,
    pub min_width: f64,
    /// The profile the ramps are measured against (docs/adr/0723 §4); none: not measured.
    pub ramp_profile: Option<&'static str>,
    /// The profile the stairs are measured against (docs/adr/0724 §2); none: not measured.
    pub stair_profile: Option<&'static str>,
}

impl Default for ArchAuditPanel {
    fn default() -> Self {
        Self {
            audited: None,
            selected: None,
            group: None,
            max_gap: ARCH_AUDIT_MAX_GAP,
            min_area: ARCH_AUDIT_MIN_AREA,
            min_width: ARCH_AUDIT_MIN_WIDTH,
            ramp_profile: RAMP_PROFILES[1].0,
            stair_profile: STAIR_PROFILES[1].0,
        }
    }
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Check,
    /// A group; none: every group.
    Group(Option<ArchAuditCheck>),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// Düzelt on the chosen finding.
    Fix,
    /// Kata ver ▾'s Seç ve göster (docs/adr/0717 §2): every object without a level selected, the view on them.
    ShowLevelless,
    /// Kata ver ▾'s level: every object without a level given it, one step.
    AssignLevel(String),
    Gap(usize),
    Area(usize),
    Width(usize),
    /// Rampa's choice, by its place in [`RAMP_PROFILES`].
    RampProfile(usize),
    /// Merdiven's choice, by its place in [`STAIR_PROFILES`].
    StairProfile(usize),
}

fn msg(e: Event) -> Message {
    Message::ArchAudit(e)
}

/// A group's icon.
fn group_icon(check: ArchAuditCheck) -> &'static str {
    match check {
        ArchAuditCheck::Walls => "wall",
        ArchAuditCheck::Openings => "door",
        ArchAuditCheck::Spaces => "space",
        ArchAuditCheck::Numbers => "openingNumber",
        ArchAuditCheck::Tags => "tag",
        ArchAuditCheck::Dimensions => "wallDimensions",
        ArchAuditCheck::Grids => "archGrid",
        ArchAuditCheck::Columns => "column",
        ArchAuditCheck::Viewmarks => "viewSection",
        ArchAuditCheck::Levels => "archSettings",
        ArchAuditCheck::Accessibility => "ramp",
        ArchAuditCheck::Stairs => "stair",
    }
}

/// A bar button: its icon and words, its hint above it.
fn bar_button<'a>(glyph: &str, words: &'a str, hint: &'a str, on: Option<Message>) -> Element<'a, Message> {
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
fn cell<'a>(words: String, quiet: bool) -> Element<'a, Message> {
    Elided::new(words)
        .size(typography::body())
        .font(typography::ui())
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(if quiet { t.muted } else { t.text }),
            }
        })
        .width(Fill)
        .into()
}

/// A threshold's choice: its label before it, its values in the project's words.
fn threshold<'a>(
    words: &'a str,
    values: &[f64],
    now: f64,
    show: impl Fn(f64) -> String,
    pick: fn(usize) -> Event,
) -> Element<'a, Message> {
    let picked = values.iter().position(|v| (v - now).abs() < 1e-12);
    let select = Select::new(
        values.iter().map(|v| Choice::new(show(*v))).collect::<Vec<_>>(),
        picked,
        move |i| msg(pick(i)),
    );
    row![
        label::muted(words),
        container(select).width(Length::Fixed(typography::scaled(96.0)))
    ]
    .spacing(6)
    .align_y(Center)
    .into()
}

impl App {
    /// Mimari denetim (`arch.audit`): the tab, checked.
    pub(crate) fn open_arch_audit(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.show_bottom(BottomTab::Audit);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < crate::search::PANEL_HEIGHT {
            self.bottom_dragged(
                Some((crate::search::PANEL_HEIGHT - bar) as f32),
                std::time::Instant::now(),
            );
        }
        self.arch_audit_check();
        Task::none()
    }

    /// The bar's thresholds as the command's input.
    fn arch_audit_input(&self) -> ArchAudit {
        ArchAudit {
            checks: Vec::new(),
            max_gap: Some(self.arch_audit.max_gap),
            min_area: Some(self.arch_audit.min_area),
            min_width: Some(self.arch_audit.min_width),
            // A level finding's Düzelt gives Etkin kat (docs/adr/0717 §3).
            level: self.active_level(),
            profiles: [self.arch_audit.ramp_profile, self.arch_audit.stair_profile]
                .into_iter()
                .flatten()
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Denetle: every group checked; the chosen finding kept when it is still there.
    pub(crate) fn arch_audit_check(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let input = self.arch_audit_input();
        let kept = self
            .arch_audit
            .selected
            .and_then(|i| self.arch_audit.audited.as_ref()?.findings.get(i))
            .map(|f| (f.code.clone(), f.uids.clone()));
        match arch_audit::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, .. } => {
                self.arch_audit.selected = kept.and_then(|(code, uids)| {
                    output
                        .findings
                        .iter()
                        .position(|f| f.code == code && f.uids == uids)
                });
                self.arch_audit.audited = Some(Audited {
                    findings: output.findings,
                    generation: doc.model.generation(),
                    revision: doc.model.revision(),
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Whether the drawing changed since the check.
    pub(crate) fn arch_audit_stale(&self) -> bool {
        match (&self.arch_audit.audited, &self.document) {
            (Some(a), Some(d)) => a.generation != d.model.generation() || a.revision != d.model.revision(),
            _ => false,
        }
    }

    /// The rows shown, as places in the findings.
    pub(crate) fn arch_audit_shown(&self) -> Vec<usize> {
        self.arch_audit.audited.as_ref().map_or_else(Vec::new, |a| {
            a.findings
                .iter()
                .enumerate()
                .filter(|(_, f)| self.arch_audit.group.is_none_or(|g| f.check == g))
                .map(|(i, _)| i)
                .collect()
        })
    }

    pub(crate) fn arch_audit_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check => self.arch_audit_check(),
            Event::Group(g) => {
                self.arch_audit.group = g;
                self.arch_audit.selected = None;
            }
            Event::Press(at) => {
                if let Some(&i) = self.arch_audit_shown().get(at) {
                    self.arch_audit.selected = Some(i);
                    self.arch_audit_show(i);
                }
            }
            Event::Fix => self.arch_audit_fix(),
            Event::ShowLevelless => self.arch_audit_show_levelless(),
            Event::AssignLevel(level) => self.arch_audit_assign_level(&level),
            Event::Gap(i) => {
                self.arch_audit.max_gap = GAPS[i.min(GAPS.len() - 1)];
                self.arch_audit_check();
            }
            Event::Area(i) => {
                self.arch_audit.min_area = AREAS[i.min(AREAS.len() - 1)];
                self.arch_audit_check();
            }
            Event::Width(i) => {
                self.arch_audit.min_width = WIDTHS[i.min(WIDTHS.len() - 1)];
                self.arch_audit_check();
            }
            Event::RampProfile(i) => {
                self.arch_audit.ramp_profile = RAMP_PROFILES[i.min(RAMP_PROFILES.len() - 1)].0;
                self.arch_audit_check();
            }
            Event::StairProfile(i) => {
                self.arch_audit.stair_profile = STAIR_PROFILES[i.min(STAIR_PROFILES.len() - 1)].0;
                self.arch_audit_check();
            }
        }
        Task::none()
    }

    /// A finding's objects selected and the view fitted on them.
    fn arch_audit_show(&mut self, finding: usize) {
        let Some(f) = self
            .arch_audit
            .audited
            .as_ref()
            .and_then(|a| a.findings.get(finding))
            .cloned()
        else {
            return;
        };
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let slots: Vec<Slot> = f
            .uids
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if ids.is_empty() {
            return;
        }
        self.navigating(|app| app.zoom_to_objects(&ids));
    }

    /// Düzelt: the chosen finding's fix, its command run as one undo step; the drawing checked again.
    fn arch_audit_fix(&mut self) {
        if self.arch_audit_stale() {
            self.warn(STALE);
            return;
        }
        let Some(f) = self
            .arch_audit
            .selected
            .and_then(|i| self.arch_audit.audited.as_ref()?.findings.get(i))
            .cloned()
        else {
            return;
        };
        let Some(fix) = f.fix else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            &fix.step.command,
            CallOp::Execute,
            fix.step.input.clone(),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.say(Level::Success, format!("Düzeltildi: {}. Geri al ile dönülür.", fix.label));
                for w in v["warnings"].as_array().into_iter().flatten() {
                    if let Some(m) = w["message"].as_str() {
                        self.warn(m.to_owned());
                    }
                }
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("Düzeltme yazılamadı.");
                self.warn(why.to_owned());
                return;
            }
            Err(e) => {
                self.warn(format!("Düzeltme çalıştırılamadı: {e:?}"));
                return;
            }
        }
        self.arch_audit_check();
    }

    /// The level findings' objects of the last check.
    fn arch_audit_levelless(&self) -> Vec<String> {
        self.arch_audit.audited.as_ref().map_or_else(Vec::new, |a| {
            a.findings
                .iter()
                .filter(|f| f.check == ArchAuditCheck::Levels)
                .flat_map(|f| f.uids.iter().cloned())
                .collect()
        })
    }

    /// Kata ver ▾'s Seç ve göster: the objects without a level selected and the view on them, before any level is
    /// given (docs/adr/0717 §2).
    fn arch_audit_show_levelless(&mut self) {
        let uids = self.arch_audit_levelless();
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let slots: Vec<Slot> = uids
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    /// Kata ver ▾'s level: every object without a level given it through `cad.levels.assign`, one undo step; the
    /// objects stay selected so the step shows; the drawing checked again.
    fn arch_audit_assign_level(&mut self, level: &str) {
        if self.arch_audit_stale() {
            self.warn(STALE);
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_LEVELS_ASSIGN,
            CallOp::Execute,
            serde_json::json!({ "level": level }),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                let given: Vec<Slot> = v["output"]["assigned"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["id"].as_u64())
                    .map(|id| Slot(id as u32))
                    .collect();
                let name = doc
                    .model
                    .settings()
                    .arch
                    .as_ref()
                    .and_then(|a| a.levels.iter().find(|l| l.id == level))
                    .map_or_else(|| level.to_owned(), |l| l.name.clone());
                let count = given.len();
                self.selection.set(given);
                self.say(
                    Level::Success,
                    format!("{count} nesnenin katı {name} oldu. Geri al ile dönülür."),
                );
                for w in v["warnings"].as_array().into_iter().flatten() {
                    if let Some(m) = w["message"].as_str() {
                        self.warn(m.to_owned());
                    }
                }
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("Kat verilemedi.");
                self.warn(why.to_owned());
                return;
            }
            Err(e) => {
                self.warn(format!("Kat verilemedi: {e:?}"));
                return;
            }
        }
        self.arch_audit_check();
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
            icon(from_web(Some("chevronDown"))).size(12.0).tone(Tone::Muted),
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

    /// The Denetim tab: its bar, a note when the check is old, the table.
    pub(crate) fn arch_audit_tab(&self) -> Element<'_, Message> {
        let bar = iced::widget::responsive(move |room| self.arch_audit_bar(room.width)).height(Length::Shrink);
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
        let lines: Vec<(ArchAuditCheck, [String; 4], bool)> = audited.map_or_else(Vec::new, |a| {
            shown
                .iter()
                .map(|&i| {
                    let f = &a.findings[i];
                    let objects = match f.uids.len() {
                        1 => "1 nesne".to_owned(),
                        n => format!("{n} nesne"),
                    };
                    (
                        f.check,
                        [f.check.label().to_owned(), f.message.clone(), f.repair.clone(), objects],
                        self.arch_audit.selected == Some(i),
                    )
                })
                .collect()
        });
        let first = lines.iter().position(|(_, _, on)| *on);
        let count = lines.len();
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let (check, cells, on) = &lines[i];
                let group = row![
                    icon(from_web(Some(group_icon(*check)))).size(13.0).tone(Tone::Muted),
                    cell(cells[0].clone(), false)
                ]
                .spacing(6)
                .align_y(Center);
                TableLine::new([
                    label::muted((i + 1).to_string()).into(),
                    group.into(),
                    cell(cells[1].clone(), stale),
                    cell(cells[2].clone(), true),
                    cell(cells[3].clone(), true),
                ])
                .selected(*on)
                .current(false)
                .on_press(msg(Event::Press(i)))
            })
            .reveal(first)
            .empty(empty);
        let mut out = Column::new().push(bar);
        if stale {
            out = out.push(
                container(
                    row![
                        icon(from_web(Some("warning"))).size(14.0).tone(Tone::Warning),
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
                    background: Some(Background::Color(Tokens::of(theme).warning.scale_alpha(0.14))),
                    ..container::Style::default()
                }),
            );
        }
        out.push(horizontal_divider()).push(table).width(Fill).height(Fill).into()
    }

    /// Rampa and Merdiven: the profiles the ramps (docs/adr/0723 §4) and the stairs (0724 §2) are measured against;
    /// the chosen profile's name and note in its tip.
    fn arch_audit_profiles(&self) -> Element<'_, Message> {
        let profile = |caption: &'static str,
                       list: &'static [(Option<&'static str>, &'static str)],
                       chosen: Option<&'static str>,
                       on: fn(usize) -> Event,
                       width: f32| {
            let picked = list.iter().position(|(id, _)| *id == chosen);
            let select = Select::new(
                list.iter().map(|(_, words)| Choice::new(*words)).collect::<Vec<_>>(),
                picked,
                move |i| msg(on(i)),
            );
            let note = chosen.and_then(kentos_commands::arch_profile).map_or_else(
                || format!("{caption} denetlenmez. Bir profil seçin."),
                |p| format!("{}. {}", p.name, p.note),
            );
            tip(
                row![label::muted(caption), container(select).width(Length::Fixed(typography::scaled(width)))]
                    .spacing(6)
                    .align_y(Center),
                Tip::new(caption).body(note),
                iced::widget::tooltip::Position::Top,
            )
        };
        row![
            profile("Rampa", &RAMP_PROFILES, self.arch_audit.ramp_profile, Event::RampProfile, 124.0),
            profile("Merdiven", &STAIR_PROFILES, self.arch_audit.stair_profile, Event::StairProfile, 150.0),
        ]
        .spacing(12)
        .align_y(Center)
        .into()
    }

    /// The bar at `width`: Denetle, the group, the thresholds; the count and Düzelt at its end, or under it when
    /// narrow.
    fn arch_audit_bar(&self, width: f32) -> Element<'_, Message> {
        let audited = self.arch_audit.audited.as_ref();
        let count_of = |g: Option<ArchAuditCheck>| {
            audited.map(|a| a.findings.iter().filter(|f| g.is_none_or(|g| f.check == g)).count())
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
            msg(Event::Group(if i == 0 { None } else { Some(ArchAuditCheck::ALL[i - 1]) }))
        });
        let format = self.format();
        let length = |v: f64| format.length(v);
        let start = row![
            bar_button("archAudit", CHECK, CHECK_HINT, Some(msg(Event::Check))),
            container(group).width(Length::Fixed(typography::scaled(210.0))),
        ]
        .spacing(12)
        .align_y(Center);
        let limits = row![
            threshold("Boşluk ≤", &GAPS, self.arch_audit.max_gap, length, Event::Gap),
            threshold("Küçük <", &AREAS, self.arch_audit.min_area, |v| format!("{v:.2} m²"), Event::Area),
            threshold("Dar <", &WIDTHS, self.arch_audit.min_width, length, Event::Width),
            self.arch_audit_profiles(),
        ]
        .spacing(12)
        .align_y(Center);
        let chosen = self
            .arch_audit
            .selected
            .and_then(|i| audited?.findings.get(i));
        let count = audited.map_or_else(String::new, |a| {
            let n = a.findings.len();
            let fixable = a.findings.iter().filter(|f| f.fix.is_some()).count();
            if n == 0 {
                "Bulgu yok".to_owned()
            } else {
                format!("{n} bulgu · {fixable} düzeltilebilir")
            }
        });
        let fix_hint = chosen
            .and_then(|f| f.fix.as_ref())
            .map_or_else(
                || "Seçili bulgunun açık bir düzeltmesi yok; Ne yapılır sütununa göre elle düzeltin.".to_owned(),
                |f| format!("{}. Tek geri alma adımıdır; denetim kendiliğinden yinelenir.", f.label),
            );
        let fixable = chosen.is_some_and(|f| f.fix.is_some()) && !self.arch_audit_stale();
        let fix_words = chosen
            .and_then(|f| f.fix.as_ref())
            .map_or(FIX.to_owned(), |f| format!("{FIX}: {}", f.label));
        let fix = tip(
            button(
                row![icon(from_web(Some("topologyFix"))).size(14.0), label::body(fix_words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::primary)
            .padding([4, 10])
            .on_press_maybe(fixable.then(|| msg(Event::Fix))),
            Tip::new(FIX).body(fix_hint),
            iced::widget::tooltip::Position::Top,
        );
        let mut end = row![label::muted(count)].spacing(10).align_y(Center);
        if let Some(levels) = self.arch_audit_levels_button() {
            end = end.push(levels);
        }
        let end = end.push(fix);
        // Wide: one line. Narrow: the check, the group, the count and Düzelt first; the thresholds under them.
        let bar: Element<'_, Message> = if width >= typography::from_default(BAR_WIDTH) {
            row![start, limits, space::horizontal(), end]
                .spacing(12)
                .align_y(Center)
                .into()
        } else {
            Column::new()
                .spacing(6)
                .push(row![start, space::horizontal(), end].spacing(10).align_y(Center))
                .push(limits)
                .into()
        };
        container(bar).padding([6, 10]).width(Fill).into()
    }
}
