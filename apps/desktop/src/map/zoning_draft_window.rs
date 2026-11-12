//! Plan değişikliği taslağı (docs/adr/5040; `cad.zoning.draft`), opened from İmar planı denetimi. A draft of a change
//! to a plan in force: its id, the plan, the reason (the plan report's justification) and the changes, each a
//! decision area of the plan, a field and its new value. The form's plan is said as it is filled: one table at a time
//! shows the changes (before and after), the plan's areas by use before and after, or what Mekânsal Planlar Yapım
//! Yönetmeliği m. 26 asks of the change. Taslağı kaydet keeps the draft in the project in one undo step and Taslağı
//! sil takes a kept one out; the decision areas in force are never written. A draft is not an approved change.

use crate::map;
use iced::{Element, Length, Task};
use kentos_commands::{CommandResult, DRAFT_FIELDS, ZONING_PROFILE, ZoningDraft, ZoningDraftChange, ZoningDrafted, builtin_profile};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, zoning_draft};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet_chosen};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Plan değişikliği taslağı";
/// İmar planı denetimi's button that opens it.
pub const OPEN: &str = "Plan değişikliği taslağı…";
const CLOSE: &str = "Kapat";
pub const SAVE: &str = "Taslağı kaydet";
pub const REMOVE: &str = "Taslağı sil";
pub const ADD: &str = "Değişikliğe ekle";
pub const DROP: &str = "Satırı çıkar";
pub const DRAFT: &str = "Taslak";
pub const ID: &str = "Kimlik";
pub const PLAN: &str = "Plan";
pub const REASON: &str = "Gerekçe";
pub const AREA: &str = "Karar alanı";
pub const FIELD: &str = "Alan";
pub const VALUE: &str = "Yeni değer";
/// The draft list's choice for a new draft.
const NEW: &str = "";

/// Which rows the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Changes,
    Uses,
    Findings,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Changes => "Değişiklikler",
            Show::Uses => "Kullanımlar",
            Show::Findings => "Mevzuatın sordukları",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub id: String,
    pub plan: String,
    pub reason: String,
    pub changes: Vec<ZoningDraftChange>,
    /// The change being written: an area of the plan, a field and its value.
    pub area: String,
    pub field: String,
    pub value: String,
    pub chosen: Option<usize>,
    pub show: Show,
    /// The command's plan for the form.
    pub answer: Option<Result<ZoningDrafted, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    /// A kept draft by its id, or a new one.
    Draft(String),
    Id(String),
    Plan(String),
    Reason(String),
    Area(String),
    Field(String),
    Value(String),
    Add,
    Choose(usize),
    Drop,
    Show(Show),
    Save,
    Remove,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Map(map::Event::ZoningDraft(e))
}

/// A field's words.
pub fn field_word(field: &str) -> &'static str {
    match field {
        "Kullanim" => "Kullanım",
        "Nizam" => "Yapı nizamı",
        "TAKS" => "TAKS",
        "KAKS" => "KAKS",
        "KatAdedi" => "Kat adedi",
        "Hmax" => "Hmax",
        "OnCekme" => "Ön çekme",
        "YanCekme" => "Yan çekme",
        "ArkaCekme" => "Arka çekme",
        _ => "?",
    }
}

fn domain_of(field: &str) -> Option<&'static str> {
    match field {
        "Kullanim" => Some("plan-kullanim"),
        "Nizam" => Some("yapi-nizami"),
        _ => None,
    }
}

fn choices(domain: &str) -> Vec<(String, String)> {
    builtin_profile(ZONING_PROFILE)
        .and_then(|p| p.domains.into_iter().find(|d| d.id == domain))
        .and_then(|d| d.values)
        .map(|v| v.into_iter().map(|c| (c.code, c.label)).collect())
        .unwrap_or_default()
}

/// A value in words: a listed value's label, else the value; an empty one a dash.
fn value_word(field: &str, value: &str) -> String {
    if value.is_empty() {
        return "—".to_owned();
    }
    domain_of(field)
        .and_then(|d| choices(d).into_iter().find(|(c, _)| c == value).map(|(_, l)| l))
        .unwrap_or_else(|| value.to_owned())
}

fn kind_word(kind: &str) -> &'static str {
    match kind {
        "infrastructureLess" => "Altyapı alanı küçülüyor",
        "densityUp" => "Yoğunluk artıyor",
        "heightUp" => "Yükseklik artıyor",
        _ => "Bulgu",
    }
}

/// The article and, shortly, what it asks; the whole sentence is the command's message.
fn asks(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "infrastructureLess" => ("m. 26(3)", "Zorunluluk, ilgili kurumun görüşü, hizmet etki alanında eşdeğer alan"),
        "densityUp" => ("m. 26(5)(a)", "Artan nüfusun sosyal ve teknik altyapı alanları"),
        "heightUp" => ("m. 26(4), (5)(b)", "Doku ve silüet; cepheler arası K = (Y1 + Y2)/2 + 7,00 m"),
        _ => ("", ""),
    }
}

impl App {
    fn zoning_draft_input(&self) -> ZoningDraft {
        let s = &self.map.zoning_draft;
        ZoningDraft {
            id: s.id.trim().to_owned(),
            plan: s.plan.clone(),
            reason: s.reason.clone(),
            changes: s.changes.clone(),
            remove: false,
            expected_revision: None,
        }
    }

    fn zoning_draft_run(&mut self) {
        let input = self.zoning_draft_input();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.map.zoning_draft.answer = Some(
            match zoning_draft::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Önizleme alınamadı.".to_owned()),
            },
        );
    }

    /// The areas of the form's plan: id, name and use.
    fn zoning_draft_areas(&self) -> Vec<(String, String, String)> {
        self.document
            .as_ref()
            .map(|d| zoning_draft::areas(&d.model, &self.map.zoning_draft.plan))
            .unwrap_or_default()
    }

    fn zoning_draft_name(&self, uid: &str) -> String {
        self.zoning_draft_areas()
            .into_iter()
            .find(|(u, _, _)| u == uid)
            .map(|(u, name, _)| if name.is_empty() { u } else { name })
            .unwrap_or_else(|| uid.to_owned())
    }

    /// Plan değişikliği taslağı from İmar planı denetimi: a new draft of the drawing's first plan, its first area's use.
    pub(crate) fn open_zoning_draft(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let plans = zoning_draft::plans(&doc.model);
        let taken: Vec<String> = zoning_draft::drafts(&doc.model).into_iter().map(|d| d.id).collect();
        let id = (1..).map(|n| format!("T-{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
        self.map.zoning_draft = State {
            id,
            plan: plans.first().map(|(id, _)| id.clone()).unwrap_or_default(),
            field: "Kullanim".to_owned(),
            ..State::default()
        };
        self.map.zoning_draft.area = self
            .zoning_draft_areas()
            .first()
            .map(|(u, _, _)| u.clone())
            .unwrap_or_default();
        self.zoning_draft_run();
        self.dialog = Some(Dialog::Map(map::MapDialogKind::ZoningDraft));
        Task::none()
    }

    pub(crate) fn zoning_draft_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_zoning_draft(),
            Event::Draft(id) => {
                if id == NEW {
                    return self.open_zoning_draft();
                }
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let Some(d) = zoning_draft::drafts(&doc.model).into_iter().find(|d| d.id == id) else { return Task::none() };
                let s = &mut self.map.zoning_draft;
                s.id = d.id;
                s.plan = d.plan;
                s.reason = d.reason;
                s.changes = d
                    .changes
                    .into_iter()
                    .map(|c| ZoningDraftChange { uid: c.area, field: c.field, value: c.after })
                    .collect();
                s.chosen = None;
            }
            Event::Id(v) => self.map.zoning_draft.id = v,
            Event::Plan(v) => {
                if v != self.map.zoning_draft.plan {
                    self.map.zoning_draft.plan = v;
                    self.map.zoning_draft.changes.clear();
                    self.map.zoning_draft.chosen = None;
                    self.map.zoning_draft.area = self
                        .zoning_draft_areas()
                        .first()
                        .map(|(u, _, _)| u.clone())
                        .unwrap_or_default();
                }
            }
            Event::Reason(v) => self.map.zoning_draft.reason = v,
            Event::Area(v) => self.map.zoning_draft.area = v,
            Event::Field(v) => {
                self.map.zoning_draft.field = v;
                self.map.zoning_draft.value.clear();
            }
            Event::Value(v) => self.map.zoning_draft.value = v,
            Event::Add => {
                let s = &mut self.map.zoning_draft;
                if s.area.is_empty() || s.value.trim().is_empty() {
                    return Task::none();
                }
                let change = ZoningDraftChange { uid: s.area.clone(), field: s.field.clone(), value: s.value.trim().to_owned() };
                match s.changes.iter().position(|c| c.uid == change.uid && c.field == change.field) {
                    Some(at) => s.changes[at] = change,
                    None => s.changes.push(change),
                }
                s.value.clear();
                s.show = Show::Changes;
            }
            Event::Choose(i) => self.map.zoning_draft.chosen = Some(i),
            Event::Drop => {
                let s = &mut self.map.zoning_draft;
                if let Some(i) = s.chosen.take().filter(|i| *i < s.changes.len()) {
                    s.changes.remove(i);
                }
            }
            Event::Show(v) => {
                self.map.zoning_draft.show = v;
                return Task::none();
            }
            Event::Save | Event::Remove => {
                let remove = matches!(event, Event::Remove);
                let mut input = self.zoning_draft_input();
                if remove {
                    input = ZoningDraft { id: input.id, remove: true, ..ZoningDraft::default() };
                }
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match zoning_draft::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        let id = self.map.zoning_draft.id.trim().to_owned();
                        self.say(
                            Level::Success,
                            if remove {
                                format!("{TITLE}: “{id}” projeden silindi (tek adımda geri alınır).")
                            } else {
                                format!(
                                    "{TITLE}: “{id}” projede saklandı (tek adımda geri alınır); yürürlükteki karar alanları değişmedi. Projede {} taslak var.",
                                    output.drafts
                                )
                            },
                        );
                        if remove {
                            return self.open_zoning_draft();
                        }
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
            }
            Event::Close => {
                self.map.zoning_draft = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.zoning_draft_run();
        Task::none()
    }

    pub(crate) fn zoning_draft_view(&self) -> Element<'_, Message> {
        let s = &self.map.zoning_draft;
        let f = self.format();
        let Some(doc) = self.document.as_ref() else { return iced::widget::Space::new().into() };
        let kept = zoning_draft::drafts(&doc.model);
        let is_kept = kept.iter().any(|d| d.id == s.id.trim());
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Taslak yürürlükteki planı değiştirmez; karar alanları olduğu gibi kalır. Taslak onaylı değişiklik değildir: plan raporu, müellif imzası, plan işlem numarası, onay ve askı KentOS dışındadır.",
        )];
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let plan_line = match &s.answer {
            Some(Ok(r)) => {
                let less: f64 = r.findings.iter().filter(|x| x.kind == "infrastructureLess").filter_map(|x| x.area).sum();
                let asks = r.findings.len();
                lines.push(words::text_line(
                    if asks > 0 { Kind::Warn } else { Kind::Ok },
                    if asks > 0 {
                        format!(
                            "{} değişiklik; mevzuatın sorduğu {asks} konu var (sosyal ve teknik altyapı alanı {} küçülüyor). Plan raporunda açıklayın.",
                            s.changes.len(),
                            f.area(less)
                        )
                    } else {
                        format!("{} değişiklik; kullanım alanlarının toplamı korunuyor, mevzuatın sorduğu bir konu yok.", s.changes.len())
                    },
                ));
                words::text_line(Kind::Ok, format!("Kaydedilebilir: “{}”, {} değişiklik.", s.id.trim(), s.changes.len()))
            }
            Some(Err(why)) => words::text_line(Kind::Info, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let ready = matches!(s.answer, Some(Ok(_)));
        let cols = match s.show {
            Show::Changes => {
                for c in &s.changes {
                    let before = zoning_draft::value(&doc.model, &c.uid, &c.field);
                    table_rows.push(vec![
                        self.zoning_draft_name(&c.uid),
                        field_word(&c.field).to_owned(),
                        value_word(&c.field, &before),
                        value_word(&c.field, &c.value),
                    ]);
                }
                vec![col("Karar", 140.0), col("Alan", 140.0), col("Önce", 200.0), col("Sonra", 200.0)]
            }
            Show::Uses => {
                if let Some(Ok(r)) = &s.answer {
                    for u in &r.uses {
                        table_rows.push(vec![
                            if u.usage.is_empty() { "Kullanımı yok".to_owned() } else { value_word("Kullanim", &u.usage) },
                            f.area(u.before),
                            f.area(u.after),
                            f.area(u.difference),
                        ]);
                    }
                }
                vec![col("Kullanım", 220.0), num("Önce", 150.0), num("Sonra", 150.0), num("Fark", 150.0)]
            }
            Show::Findings => {
                if let Some(Ok(r)) = &s.answer {
                    for x in &r.findings {
                        let (article, what) = asks(&x.kind);
                        table_rows.push(vec![
                            kind_word(&x.kind).to_owned(),
                            x.uid.as_deref().map(|u| self.zoning_draft_name(u)).or_else(|| x.usage.as_deref().map(|u| value_word("Kullanim", u))).unwrap_or_default(),
                            x.area.map(|a| f.area(a)).unwrap_or_default(),
                            article.to_owned(),
                            what.to_owned(),
                        ]);
                    }
                }
                vec![col("Konu", 170.0), col("Karar", 90.0), num("Alan", 110.0), col("Madde", 120.0), col("Ne ister", 420.0)]
            }
        };
        let empty = match s.show {
            Show::Changes => "Değişiklik yok: karar alanını, alanı ve yeni değeri seçip Değişikliğe ekle'ye basın.",
            Show::Uses => "Planın karar alanı yok.",
            Show::Findings => "Mevzuatın sorduğu bir konu yok.",
        };
        let chosen = if s.show == Show::Changes { s.chosen } else { None };
        let table = sheet_chosen(cols, table_rows, empty, 220.0, chosen, |i| msg(Event::Choose(i)));
        let choose = Segmented::new([Show::Changes, Show::Uses, Show::Findings], s.show, |v| msg(Event::Show(v)));
        let text = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |v| msg(on(v))).width(Length::Fill)
        };
        let mut draft_choices = vec![(NEW.to_owned(), "Yeni taslak".to_owned())];
        draft_choices.extend(kept.iter().map(|d| (d.id.clone(), format!("{} · {}", d.id, d.plan))));
        let current = if is_kept { s.id.trim().to_owned() } else { NEW.to_owned() };
        let plans: Vec<(String, String)> = zoning_draft::plans(&doc.model)
            .into_iter()
            .map(|(id, level)| (id.clone(), if level.is_empty() { id } else { format!("{id} ({})", level.to_uppercase()) }))
            .collect();
        let areas: Vec<(String, String)> = self
            .zoning_draft_areas()
            .into_iter()
            .map(|(u, name, usage)| {
                let shown = if name.is_empty() { u.clone() } else { name };
                (u, format!("{shown} · {}", value_word("Kullanim", &usage)))
            })
            .collect();
        let field_choices: Vec<(String, String)> = DRAFT_FIELDS.iter().map(|f| ((*f).to_owned(), field_word(f).to_owned())).collect();
        let now = value_word(&s.field, &zoning_draft::value(&doc.model, &s.area, &s.field));
        let value: Element<'_, Message> = match domain_of(&s.field) {
            Some(d) => {
                let mut c = vec![(String::new(), "— seçin".to_owned())];
                c.extend(choices(d));
                fields::select_owned(c, &s.value, |v| msg(Event::Value(v)))
            }
            None => text("Sayı (ör. 1.50)", &s.value, Event::Value).into(),
        };
        let can_add = !s.area.is_empty() && !s.value.trim().is_empty();
        let form = Form::new()
            .label_width(150.0)
            .section("Taslak")
            .field(DRAFT, fields::select_owned(draft_choices, &current, |v| msg(Event::Draft(v))))
            .field(ID, text("T-1", &s.id, Event::Id))
            .field(PLAN, fields::select_owned(plans, &s.plan, |v| msg(Event::Plan(v))))
            .field(REASON, text("Değişikliğin gerekçesi (plan raporu)", &s.reason, Event::Reason))
            .help("Gerekçe zorunludur: plan değişikliğinin raporu gerekçeyi ve gereklilik analizlerini açıklar.")
            .section("Değişiklik")
            .field(AREA, fields::select_owned(areas, &s.area, |v| msg(Event::Area(v))))
            .field(FIELD, fields::select_owned(field_choices, &s.field, |v| msg(Event::Field(v))))
            .field(VALUE, value)
            .help(format!("Yürürlükteki değeri: {now}."))
            .row(
                iced::widget::row![
                    words::secondary(ADD, can_add.then(|| msg(Event::Add))),
                    words::secondary(DROP, (s.show == Show::Changes && s.chosen.is_some()).then(|| msg(Event::Drop))),
                ]
                .spacing(8),
            )
            .section("Kaydet")
            .row(plan_line)
            .row(
                iced::widget::row![
                    words::primary(SAVE, ready.then(|| msg(Event::Save))),
                    words::secondary(REMOVE, is_kept.then(|| msg(Event::Remove))),
                ]
                .spacing(8),
            );
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(choose)
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0))
            .push(form);
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(980.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn zoning_draft_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let s = &self.map.zoning_draft;
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => matches!(s.answer, Some(Ok(_))).then(|| msg(Event::Save)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(DROP) => Some(msg(Event::Drop)),
            Control::Fill(ID, v) => Some(msg(Event::Id(v.to_owned()))),
            Control::Fill(REASON, v) => Some(msg(Event::Reason(v.to_owned()))),
            Control::Fill(VALUE, v) => Some(msg(Event::Value(v.to_owned()))),
            Control::Row(n) if n > 0 => Some(msg(Event::Choose(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
