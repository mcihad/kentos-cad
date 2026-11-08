//! İmar planı denetimi (docs/adr/1420; `zoning.check`): the plan as the profile `imar-plani`'s typed objects, checked
//! by `cad.zoning.check`, read only. The tolerance on one line; a choice over the findings by group (plans, decisions
//! and values, areas, notes); the table says each finding, its objects and the area it measures. Seç ve göster
//! selects the shown findings' objects and brings them into view; Profili uygula adds the profile's layers when the
//! drawing lacks them (`cad.profiles.apply`, one undo step).

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, ProfilesApply, ZONING_PROFILE, ZoningCheck, ZoningChecked, ZoningFinding};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, profiles, zoning_check};
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "İmar planı denetimi";
const CLOSE: &str = "Kapat";
const SHOW: &str = "Seç ve göster";
const APPLY_PROFILE: &str = "Profili uygula";

/// The findings a group shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Group {
    #[default]
    All,
    Plans,
    Decisions,
    Areas,
    Notes,
}

impl std::fmt::Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Group::All => "Tümü",
            Group::Plans => "Plan ve kademe",
            Group::Decisions => "Karar ve değer",
            Group::Areas => "Alan",
            Group::Notes => "Plan notu",
        })
    }
}

impl Group {
    pub fn holds(self, kind: &str) -> bool {
        match self {
            Group::All => true,
            Group::Plans => {
                matches!(kind, "missingLayer" | "noLevel" | "upperMissing" | "levelOrder" | "outsideUpper" | "noPlan")
            }
            Group::Decisions => {
                matches!(kind, "noUse" | "unknownUse" | "missingValue" | "declaredValued" | "kaksReview")
            }
            Group::Areas => {
                matches!(kind, "overlap" | "uncovered" | "outside" | "roadGap" | "roadWidth" | "roadParcel")
            }
            Group::Notes => matches!(kind, "noteMissing" | "noteUnused"),
        }
    }
}

/// A finding's kind in words.
pub fn kind_word(kind: &str) -> &'static str {
    match kind {
        "missingLayer" => "Katman yok",
        "noLevel" => "Kademe yok",
        "upperMissing" => "Üst plan yok",
        "levelOrder" => "Kademe sırası",
        "outsideUpper" => "Üst plan dışına taşma",
        "noPlan" => "Planı yok",
        "noUse" => "Kullanım yok",
        "unknownUse" => "Bilinmeyen kullanım",
        "missingValue" => "Değer verilmemiş",
        "declaredValued" => "Değer çelişkisi",
        "kaksReview" => "KAKS gözden geçir",
        "overlap" => "Karar çelişkisi",
        "uncovered" => "Kararsız kısım",
        "outside" => "Plan dışına taşma",
        "roadGap" => "Kopuk yol",
        "roadWidth" => "Yola giren karar",
        // docs/adr/2640.
        "roadParcel" => "Yola giren parsel",
        "noteMissing" => "Plan notu yok",
        "noteUnused" => "Anılmayan not",
        _ => "Bulgu",
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub tolerance: f64,
    pub group: Group,
    pub answer: Option<Result<ZoningChecked, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Tolerance(f64),
    Group(Group),
    Show,
    ApplyProfile,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Zoning(e)
}

impl App {
    fn zoning_run(&mut self) {
        let input = ZoningCheck { tolerance: Some(self.zoning.tolerance) };
        let Some(doc) = self.document.as_mut() else { return };
        self.zoning.answer = Some(match zoning_check::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Denetim yapılamadı.".to_owned()),
        });
    }

    /// İmar planı denetimi (`zoning.check`).
    pub(crate) fn open_zoning(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.zoning = State { tolerance: 0.01, ..State::default() };
        self.zoning_run();
        self.dialog = Some(Dialog::Zoning);
        Task::none()
    }

    fn zoning_shown(&self) -> Vec<&ZoningFinding> {
        match &self.zoning.answer {
            Some(Ok(r)) => r.findings.iter().filter(|f| self.zoning.group.holds(&f.kind)).collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn zoning_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tolerance(v) => {
                if v > 0.0 {
                    self.zoning.tolerance = v;
                    self.zoning_run();
                }
            }
            Event::Group(g) => self.zoning.group = g,
            Event::Show => {
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let mut slots: Vec<Slot> = Vec::new();
                for f in self.zoning_shown() {
                    for uid in &f.uids {
                        if let Some(s) = Uuid::parse_str(uid).ok().and_then(|u| doc.model.slot_of(u))
                            && !slots.contains(&s)
                        {
                            slots.push(s);
                        }
                    }
                }
                let n = slots.len();
                // The road overlaps hatched on the drawing until İşareti kaldır (docs/adr/3040).
                let hatched = self
                    .zoning_shown()
                    .iter()
                    .filter(|f| crate::zoning_marks::HATCHED.contains(&f.kind.as_str()))
                    .count();
                let input = ZoningCheck { tolerance: Some(self.zoning.tolerance) };
                self.zoning_shown_marks = (hatched > 0).then(|| crate::zoning_marks::Shown::new(doc.session, input));
                self.selection.set(slots);
                self.zoning = State::default();
                self.dialog = None;
                let words = match hatched {
                    0 => format!("{TITLE}: bulgulardaki {n} nesne seçildi."),
                    h => format!("{TITLE}: bulgulardaki {n} nesne seçildi; yola giren {h} kısım çizimde tarandı (İşareti kaldır siler)."),
                };
                self.say(Level::Info, words);
                return Task::done(Message::Run("view.zoomSelection"));
            }
            Event::ApplyProfile => {
                let input = ProfilesApply {
                    name: Some(ZONING_PROFILE.to_owned()),
                    profile: None,
                    mapping: None,
                    mapping_name: None,
                    drawing: None,
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match profiles::apply::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { .. } => self.say(
                        Level::Success,
                        format!("“{ZONING_PROFILE}” profili uygulandı: plan sınırı, karar alanı, plan notu ve yol katmanları eklendi."),
                    ),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
                self.zoning_run();
            }
            Event::Close => {
                self.zoning = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// The parcels that enter road widths, each with the sum of its parts in every road it enters (docs/adr/3040): its
    /// name and roads, largest first; none without such findings.
    pub(crate) fn zoning_parcel_totals(&self) -> Vec<(String, usize, f64)> {
        let (Some(doc), Some(Ok(r))) = (self.document.as_ref(), &self.zoning.answer) else { return Vec::new() };
        let mut by: Vec<(String, String, usize, f64)> = Vec::new();
        for f in r.findings.iter().filter(|f| f.kind == "roadParcel") {
            // The parcel is the finding's area object (the road is its axis).
            let Some((uid, e)) = f.uids.iter().find_map(|u| {
                let e = Uuid::parse_str(u).ok().and_then(|x| doc.model.slot_of(x)).and_then(|s| doc.model.get(s))?;
                matches!(e, kentos_commands::Entity::Polygon(_)).then(|| (u.clone(), e))
            }) else {
                continue;
            };
            let a = &e.base().attrs;
            let name: Vec<&str> = ["Ada", "Parsel"].iter().filter_map(|k| a.get(*k).map(|v| v.trim())).filter(|v| !v.is_empty()).collect();
            let name = if name.is_empty() { format!("#{}", e.base().id) } else { name.join("/") };
            match by.iter_mut().find(|x| x.0 == uid) {
                Some(x) => {
                    x.2 += 1;
                    x.3 += f.area.unwrap_or(0.0);
                }
                None => by.push((uid, name, 1, f.area.unwrap_or(0.0))),
            }
        }
        by.sort_by(|a, b| b.3.total_cmp(&a.3));
        by.into_iter().map(|(_, name, roads, area)| (name, roads, area)).collect()
    }

    /// A finding's objects as the table names them: a plan by its PlanKimlik, a decision by its KararKimlik, a note
    /// by its number, a road axis by its YolKimlik (docs/adr/2020), a parcel by its Ada/Parsel (docs/adr/2640).
    fn zoning_names(&self, f: &ZoningFinding) -> String {
        let Some(doc) = self.document.as_ref() else { return String::new() };
        let names: Vec<String> = f
            .uids
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok().and_then(|u| doc.model.slot_of(u)).and_then(|s| doc.model.get(s)))
            .map(|e| {
                let a = &e.base().attrs;
                let parcel: Vec<&str> =
                    ["Ada", "Parsel"].iter().filter_map(|k| a.get(*k).map(|v| v.trim())).filter(|v| !v.is_empty()).collect();
                ["PlanKimlik", "KararKimlik", "NotNo", "YolKimlik"]
                    .iter()
                    .find_map(|k| a.get(*k).filter(|v| !v.trim().is_empty()).map(|v| v.trim().to_owned()))
                    .map(|v| if a.contains_key("NotNo") { format!("Not {v}") } else { v })
                    .or_else(|| (!parcel.is_empty()).then(|| parcel.join("/")))
                    .unwrap_or_else(|| format!("#{}", e.base().id))
            })
            .collect();
        names.join(", ")
    }

    pub(crate) fn zoning_view(&self) -> Element<'_, Message> {
        let s = &self.zoning;
        let f = self.format();
        let form = Form::new().label_width(110.0).field(
            "Tolerans (m)",
            row![
                NumberInput::new(s.tolerance, |v| msg(Event::Tolerance(v))).step(0.01).decimals(3).point().width(140),
                kentos_ui::label::caption(format!("Profil: {ZONING_PROFILE} (KentOS çalışma profili)"))
            ]
            .spacing(14)
            .align_y(Center),
        );
        let mut lines = Vec::new();
        let mut missing_layer = false;
        match &s.answer {
            Some(Ok(r)) => {
                missing_layer = r.counts.contains_key("missingLayer");
                let total = r.findings.len();
                lines.push(words::text_line(
                    if total == 0 { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} plan sınırı, {} karar alanı, {} plan notu: {}.",
                        r.plans,
                        r.decisions,
                        r.notes,
                        if total == 0 { "bulgu yok".to_owned() } else { format!("{total} bulgu") }
                    ),
                ));
                // Each parcel's parts in road widths summed, those in more than one road said (docs/adr/3040).
                let totals = self.zoning_parcel_totals();
                if !totals.is_empty() {
                    let parts: Vec<String> = totals
                        .iter()
                        .take(6)
                        .map(|(name, roads, area)| match roads {
                            1 => format!("{name} {}", f.area(*area)),
                            k => format!("{name} {} ({k} yoldan)", f.area(*area)),
                        })
                        .collect();
                    let more = totals.len().saturating_sub(6);
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "Yola giren parseller, yol genişliğindeki toplam alanlarıyla: {}{}. Bulguları göster bu kısımları çizimde tarar.",
                            parts.join(", "),
                            if more > 0 { format!(" ve {more} parsel daha") } else { String::new() }
                        ),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Karar sembolden ya da renkten değil, alanların tipli değerlerinden okunur. Bakanlığın MPYY/PlanGML şemasına ya da mevzuata uygunluk iddiası değildir.",
                ));
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let rows: Vec<Vec<String>> = self
            .zoning_shown()
            .into_iter()
            .map(|x| {
                vec![
                    kind_word(&x.kind).to_owned(),
                    self.zoning_names(x),
                    x.area.map(|a| f.area(a)).unwrap_or_default(),
                    x.message.clone(),
                ]
            })
            .collect();
        let groups = [Group::All, Group::Plans, Group::Decisions, Group::Areas, Group::Notes];
        let table = sheet(
            vec![col("Bulgu", 140.0), col("Nesne", 100.0), num("Alan", 90.0), Col::rest("Açıklama")],
            rows,
            "Bu grupta bulgu yok.",
            300.0,
        );
        let body = Column::new()
            .spacing(12)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new(groups, s.group, |g| msg(Event::Group(g))))
            .push(table);
        let any = !self.zoning_shown().is_empty();
        let mut frame = Frame::new(TITLE).push(body).aside(words::secondary(SHOW, any.then(|| msg(Event::Show))));
        if missing_layer {
            frame = frame.aside(words::secondary(APPLY_PROFILE, Some(msg(Event::ApplyProfile))));
        }
        frame.action(words::primary(CLOSE, Some(msg(Event::Close)))).width(1000.0).modal(msg(Event::Close))
    }

    pub(crate) fn zoning_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SHOW) => Some(msg(Event::Show)),
            Control::Press(APPLY_PROFILE) => Some(msg(Event::ApplyProfile)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

use crate::road_dialog::Col;

impl Col<'_> {
    /// A column that takes the rest of the width.
    fn rest(title: &str) -> Col<'_> {
        Col { title, width: None, number: false }
    }
}
