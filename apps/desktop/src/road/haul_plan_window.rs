//! Taşıma planı (plan/road.md Y23 and Y24, docs/adr/0924 §3 and 0925, `haul.plan`): a mass haul diagram's earthwork moved
//! at the least haul moment or the least cost (the diagram's free haul and prices), its borrow and waste sites taking up
//! to their volume. The window reads `road.hauls.plan` (nothing written) whenever a value changes (docs/adr/0975 §4: no
//! Önizle): the flow along the road (above the line forward, below it back), the sites marked where they stand; under
//! it four figures and one list at a time: the sites against their capacity, or the costs. The layout is the road
//! windows' one (road_dialog.rs).
//!
//! ```text
//! ┌ Taşıma planı ─────────────────────────────────────────────────────────────────────── × ┐
//! │ Girdi ───────────────   ┌ the flow along the road, sites ────────────────────────────┐ │
//! │  Brükner [Yan ▾]        └────────────────────────────────────────────────────────────┘ │
//! │  Amaç [Moment|Maliyet]  [durum] [taşınan] [moment] [ortalama / toplam maliyet]        │
//! │                         (Alanlar | Maliyet)  table                                    │
//! │                                                                              [Kapat] │
//! └──────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::road;
use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{HaulObjective, HaulPlanned, HaulSiteKind, HaulsPlan};
use kentos_interaction::Level;
use kentos_commands::{HaulNetworkIssue, HaulNetworkRead, HaulsNetwork};
use kentos_native_application::road_hauls_network::{self, Drawn};
use kentos_native_application::{ExecutionContext, road_hauls_approve, road_hauls_plan};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Taşıma planı";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
pub(crate) const FLEET: &str = "Kamyon filosu…";
pub(crate) const MOMENT: &str = "En küçük moment";
pub(crate) const COST: &str = "En düşük maliyet";
/// Planı yaz (ROAD-HAUL-12, docs/adr/0953): the plan as a workbook or a CSV.
pub(crate) const WRITE: &str = "Planı yaz…";
/// The plan's approval (ROAD-HAUL-08, docs/adr/0954): approve it as planned now, or take the approval off, with a note.
pub(crate) const APPROVE: &str = "Onayla";
pub(crate) const REVOKE: &str = "Onayı kaldır";
pub(crate) const APPROVAL_NOTE: &str = "Onay notu";
/// The haul road network in plan (ROAD-HAUL-06, docs/adr/0955).
pub(crate) const FLOW_VIEW: &str = "Akış";
pub(crate) const NETWORK_VIEW: &str = "Ağ";

/// What the picture shows: the flow along the road, or the haul road network in plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shown {
    #[default]
    Flow,
    Network,
}

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if *self == Shown::Flow { FLOW_VIEW } else { NETWORK_VIEW })
    }
}

/// Which list the window shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Listed {
    #[default]
    Sites,
    Costs,
    /// The approved plan's sites against today's (docs/adr/0954).
    Approval,
}

impl std::fmt::Display for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Listed::Sites => "Alanlar",
            Listed::Costs => "Maliyet",
            Listed::Approval => "Onay",
        })
    }
}

/// What the plan makes least, as the window shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Aim {
    #[default]
    Moment,
    Cost,
}

impl std::fmt::Display for Aim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Aim::Moment => MOMENT,
            Aim::Cost => COST,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) haul: Option<String>,
    pub(crate) aim: Aim,
    pub(crate) preview: Option<Result<HaulPlanned, String>>,
    pub(crate) warnings: Vec<String>,
    pub(crate) list: Listed,
    /// Kamyon filosu over the window (haul_fleet_window.rs, docs/adr/0936 §3), and its values kept when closed.
    pub(crate) fleet: Option<Box<crate::haul_fleet_window::Fleet>>,
    pub(crate) fleet_kept: Option<crate::haul_fleet_window::Fleet>,
    /// The format Planı yaz writes (docs/adr/0953); none: the workbook.
    pub(crate) format: Option<kentos_commands::WorkbookFormat>,
    /// The note the next approval or revocation is written with (docs/adr/0954).
    pub(crate) approval_note: String,
    /// The picture shown, and the diagram's haul road network as read and drawn (docs/adr/0955); none without one.
    pub(crate) shown: Shown,
    pub(crate) network: Option<Result<(HaulNetworkRead, Vec<String>), String>>,
    pub(crate) drawn: Option<Drawn>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Kamyon filosu…: the fleet window on the diagram and objective chosen.
    OpenFleet,
    Fleet(crate::haul_fleet_window::Event),
    List(Listed),
    Haul(String),
    Aim(Aim),
    Preview,
    /// Planı yaz's format, and the writing.
    Format(kentos_commands::WorkbookFormat),
    Write,
    /// Approve the plan as planned now with the chosen objective (true), or take its approval off (false).
    Approve(bool),
    ApprovalNote(String),
    Show(Shown),
    Close,
}

fn msg(event: Event) -> Message {
    Message::Road(road::Event::HaulPlan(event))
}

/// Thousands parted by a thin space, for the large volumes and moments.
fn grouped(v: f64, places: usize) -> String {
    let text = format!("{:.places$}", v.abs());
    let (whole, part) = text.split_once('.').map_or((text.as_str(), None), |(a, b)| (a, Some(b)));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if let Some(p) = part {
        out.push(',');
        out.push_str(p);
    }
    if v < 0.0 && out.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.insert(0, '−');
    }
    out
}

impl App {
    /// Taşıma planı: the window on the road project's first mass haul diagram.
    pub(crate) fn open_haul_plan(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Taşıma planı yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_hauls().first().map(|h| h.id.clone()) else {
            self.warn("Taşıma planı bir Brüknerde kurulur: önce Brükner oluştur ile bir Brükner yazın.".to_owned());
            return;
        };
        self.road.haul_plan_window = Some(Box::new(Window {
            haul: Some(first),
            ..Window::default()
        }));
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::HaulPlan));
        let _ = self.haul_plan_event(Event::Preview);
    }

    pub(crate) fn haul_plan_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenFleet => {
                self.open_haul_fleet();
                return Task::none();
            }
            Event::Fleet(e) => {
                self.haul_fleet_event(e);
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.road.haul_plan_window.as_mut() else {
            return Task::none();
        };
        match event {
            // Answered before the window is borrowed.
            Event::OpenFleet | Event::Fleet(_) => return Task::none(),
            Event::Haul(h) => w.haul = Some(h),
            Event::Aim(a) => w.aim = a,
            Event::List(l) => {
                w.list = l;
                return Task::none();
            }
            Event::ApprovalNote(t) => {
                w.approval_note = t;
                return Task::none();
            }
            Event::Show(v) => {
                w.shown = v;
                return Task::none();
            }
            Event::Approve(on) => return self.approve_haul(on),
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let read = match w.haul.clone() {
                    Some(haul) => {
                        let objective = Some(if w.aim == Aim::Cost { HaulObjective::Cost } else { HaulObjective::Moment });
                        road_hauls_plan::preview(&doc.model, &HaulsPlan { haul, objective })
                    }
                    None => Err("Bir Brükner seçin.".to_owned()),
                };
                match read {
                    Ok((r, warnings)) => {
                        w.preview = Some(Ok(r));
                        w.warnings = warnings.into_iter().map(|x| x.message).collect();
                    }
                    Err(why) => {
                        w.preview = Some(Err(why));
                        w.warnings.clear();
                    }
                }
                // The haul road network, where the diagram has one (docs/adr/0955).
                let has = w.haul.as_ref().and_then(|h| doc.model.road_hauls().iter().find(|d| &d.id == h)).is_some_and(|d| d.network.is_some());
                match (has, w.haul.clone()) {
                    (true, Some(haul)) => {
                        w.network = Some(
                            road_hauls_network::preview(&doc.model, &HaulsNetwork { haul: haul.clone() })
                                .map(|(r, warnings)| (r, warnings.into_iter().map(|x| x.message).collect())),
                        );
                        w.drawn = road_hauls_network::drawn(&doc.model, &haul);
                    }
                    _ => {
                        w.network = None;
                        w.drawn = None;
                        w.shown = Shown::Flow;
                    }
                }
                return Task::none();
            }
            Event::Format(f) => {
                if let Some(w) = self.road.haul_plan_window.as_mut() {
                    w.format = Some(f);
                }
                return Task::none();
            }
            Event::Write => {
                let Some(w) = &self.road.haul_plan_window else {
                    return Task::none();
                };
                let (Some(doc), Some(Ok(planned))) = (&self.document, &w.preview) else {
                    self.warn("Plan yazılamaz: önce bir Brükner seçin; plan kurulmadı.".to_owned());
                    return Task::none();
                };
                let name = doc
                    .model
                    .road_hauls()
                    .iter()
                    .find(|h| h.id == planned.haul)
                    .map_or_else(|| "taşıma planı".to_owned(), |h| format!("{} taşıma planı", h.name));
                let sheets = road_hauls_plan::sheets(&doc.model, planned);
                return crate::table_write::save(sheets, w.format.unwrap_or(kentos_commands::WorkbookFormat::Xlsx), name);
            }
            Event::Close => {
                self.road.haul_plan_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        // The window only reads: it reads again with the new value (docs/adr/0975 §4).
        self.haul_plan_event(Event::Preview)
    }

    /// The plan approved today as planned now with the window's objective, or its approval taken off (docs/adr/0954);
    /// then the window reads it again.
    fn approve_haul(&mut self, on: bool) -> Task<Message> {
        let Some(w) = &self.road.haul_plan_window else {
            return Task::none();
        };
        let Some(haul) = w.haul.clone() else {
            return Task::none();
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let zone = crate::cloud::local_time::Zone::system();
        let day: String = crate::cloud::local_time::iso_local(i64::try_from(now).unwrap_or(0), zone).chars().take(10).collect();
        let input = kentos_commands::HaulsApprove {
            haul,
            revoke: !on,
            at: Some(day),
            note: (!w.approval_note.trim().is_empty()).then(|| w.approval_note.trim().to_owned()),
            objective: on.then_some(if w.aim == Aim::Cost { HaulObjective::Cost } else { HaulObjective::Moment }),
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_hauls_approve::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            if let Some(w) = self.road.haul_plan_window.as_mut() {
                w.approval_note.clear();
            }
            self.say(
                Level::Success,
                if on { "Taşıma planı onaylandı; önceki onayı varsa geçmişte." } else { "Taşıma planının onayı kaldırıldı; onay geçmişte." },
            );
        }
        self.haul_plan_event(Event::Preview)
    }

    pub(crate) fn haul_plan_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road.haul_plan_window else {
            return iced::widget::text("").into();
        };
        if let Some(x) = &w.fleet {
            return self.haul_fleet_view(x);
        }
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_hauls().iter().map(|h| (h.id.clone(), h.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.haul.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Haul(ids[i].clone())));
        let aim = Segmented::new([Aim::Moment, Aim::Cost], w.aim, |a| msg(Event::Aim(a)));
        let form = Form::new()
            .label_width(90.0)
            .section("Girdi")
            .field("Brükner", pick)
            .field("Amaç", aim)
            .help("En küçük moment taşıma uzaklığını, en düşük maliyet Brüknerin serbest taşımasını ve fiyatlarını okur.")
            // The note of the next approval or revocation (docs/adr/0954).
            .section("Onay")
            .field(
                APPROVAL_NOTE,
                kentos_ui::widget::text_field("Şantiye şefiyle görüşüldü, döküm alanı değişti…", &w.approval_note)
                    .on_input(|t| msg(Event::ApprovalNote(t))),
            )
            .help("Onayla planı seçili amaçla bugünkü hâliyle, bugünün günüyle yazar. Brükner, kübaj ya da alanlar değişince plan onaya göre okunur ve neyin değiştiği söylenir.");
        let form = if w.network.is_some() {
            form.section("Görünüş")
                .field("Resim", Segmented::new([Shown::Flow, Shown::Network], w.shown, |v| msg(Event::Show(v))))
                .help("Ağ, taşıma yollarını düzlemde gösterir: aksa bağlandıkları km'ler, alanların erişimi, birleşmeyen yerler.")
        } else {
            form
        };
        let feasible = matches!(&w.preview, Some(Ok(r)) if r.feasible);
        let approved = matches!(&w.preview, Some(Ok(r)) if r.approval.is_some());
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .ghost(FLEET, feasible.then(|| msg(Event::OpenFleet)))
            .ghost(APPROVE, feasible.then(|| msg(Event::Approve(true))))
            .ghost(REVOKE, approved.then(|| msg(Event::Approve(false))))
            .form_width(380.0)
            .empty("Her yarma yerleşir, her dolgu karşılanır; ödünç ve döküm alanları hacimleri kadar alır.");
        match &w.preview {
            None => {}
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(r)) => {
                let network = w.network.as_ref().filter(|_| w.shown == Shown::Network);
                d = match (network, &w.drawn) {
                    (Some(Ok((n, _))), Some(drawn)) => d
                        .picture(iced::widget::canvas(NetworkMap { read: n, drawn }).width(Fill).height(Fill), 320.0)
                        .legend("Gri aks; mavi taşıma yolu, koyu mavi alanların aksa yolu; nokta aksa bağlantı; yeşil ödünç, kırmızı döküm alanı, içi boş bağlanmayan; turuncu × birleşmeyen kesişme, halka boşta uç"),
                    (Some(Err(why)), _) => d.note(Banner::error(why.clone())),
                    _ => d
                        .picture(iced::widget::canvas(Flow { read: r }).width(Fill).height(Fill), 320.0)
                        .legend(if r.unsuitable.is_some() {
                            "Mavi ileri, turuncu geri taşınan hacim, mor uygunsuz kısmı; yeşil ödünç, kırmızı döküm alanı; gri çizgi engel"
                        } else {
                            "Mavi ileri, turuncu geri taşınan hacim; yeşil ödünç, kırmızı döküm alanı"
                        }),
                };
                d = d
                    .figure("Durum", if r.feasible { "Kurulur".to_owned() } else { "Kurulamaz".to_owned() })
                    .figure("Yarma / dolgu", format!("{} / {} m³", grouped(r.cut, 0), grouped(r.fill, 0)));
                if r.feasible {
                    d = d.figure("Taşıma momenti", format!("{} m³·m", grouped(r.moment, 0)));
                    // Docs/adr/0955: the part of the moment on the sites' access roads.
                    if let Some(a) = r.access_moment {
                        d = d.figure("Erişim yolu", format!("{} m³·m", grouped(a, 0)));
                    }
                    d = match &r.cost {
                        Some(c) => d.figure("Toplam maliyet", format!("{} {}", grouped(c.total, 2), c.currency)),
                        None => d.figure("Ortalama uzaklık", r.average.map_or_else(|| "—".into(), |a| format!("{} m", grouped(a, 1)))),
                    };
                } else {
                    d = d
                        .figure("Karşılanmayan dolgu", format!("{} m³", grouped(r.shortage, 1)))
                        .figure("Yerleşmeyen yarma", format!("{} m³", grouped(r.surplus, 1)));
                }
                // The approval (docs/adr/0954): stale with why, or still the plan approved.
                if let Some(a) = &r.approval {
                    let x = &a.approved;
                    let note = x.note.as_ref().map_or_else(String::new, |n| format!(" · {n}"));
                    let aim = if x.cost { "en düşük maliyetle" } else { "en küçük momentle" };
                    d = d.note(if a.stale {
                        Banner::warning(format!(
                            "{}. onaylı plan ({}{note}, {aim}) eskidi: {}. Yeniden onaylayın ya da onayı kaldırın.",
                            x.number,
                            x.at,
                            a.reasons.join("; ")
                        ))
                    } else {
                        Banner::info(format!(
                            "{}. onaylı plan ({}{note}, {aim}): bugün de aynı; moment {} m³·m, taşınan {} m³.",
                            x.number,
                            x.at,
                            grouped(x.moment, 0),
                            grouped(x.moved, 0)
                        ))
                    });
                }
                // The unsuitable cut and the barriers (docs/adr/0944): each said only where there is one.
                if let Some(u) = r.unsuitable.filter(|u| *u > 0.05) {
                    let stranded = r.stranded.unwrap_or(0.0);
                    d = d.figure("Uygunsuz yarma", format!("{} m³", grouped(u, 0)));
                    if stranded > 0.05 {
                        d = d.figure("Dökülemeyen uygunsuz", format!("{} m³", grouped(stranded, 1)));
                    }
                    d = d.note(Banner::info(format!(
                        "Uygunsuz yarma {} m³ yalnız döküm alanlarına gider; momentin {} m³·m'si onun.",
                        grouped(u, 0),
                        grouped(r.unsuitable_moment.unwrap_or(0.0), 0),
                    )));
                }
                if !r.barriers.is_empty() {
                    let crossings: Vec<String> = r
                        .barriers
                        .iter()
                        .map(|b| match b.detour {
                            Some(dt) => format!("{} (km {}, {} m dolaşma): {} m³ geçiyor", b.name, grouped(b.station, 0), grouped(dt, 0), grouped(b.crossed, 0)),
                            None => format!("{} (km {}) kapalı: taşıma geçmiyor", b.name, grouped(b.station, 0)),
                        })
                        .collect();
                    d = d.note(Banner::info(format!("Engeller: {}.", crossings.join("; "))));
                }
                let sorted = r.unsuitable.is_some();
                let reached = r.access_moment.is_some();
                // With the access column the others give room (the detail is ~730 px wide at 1440).
                let (wide, mid, kind, km) = if reached { (120.0, 105.0, 60.0, 80.0) } else { (180.0, 130.0, 70.0, 90.0) };
                let mut columns = vec![col("Alan", wide), col("Tür", kind), num("Km (m)", km), num("Kullanılan (m³)", mid)];
                if sorted {
                    columns.push(num("Uygunsuz (m³)", 110.0));
                }
                columns.push(num("Kapasite (m³)", mid));
                if reached {
                    columns.push(num("Erişim (m)", 90.0));
                }
                let sites = road_dialog::table(
                    columns,
                    r.sites
                        .iter()
                        .map(|s| {
                            let mut row = vec![
                                s.name.clone(),
                                if s.kind == HaulSiteKind::Borrow { "ödünç" } else { "döküm" }.to_owned(),
                                grouped(s.station, 1),
                                grouped(s.used, 1),
                            ];
                            if sorted {
                                row.push(s.used_unsuitable.map_or_else(|| "—".into(), |u| grouped(u, 1)));
                            }
                            row.push(grouped(s.capacity, 1));
                            if reached {
                                row.push(if s.unconnected { "bağlanmıyor".to_owned() } else { s.access.map_or_else(|| "—".into(), |a| grouped(a, 1)) });
                            }
                            row
                        })
                        .collect(),
                    "Ödünç ya da döküm alanı yok",
                );
                // The approved plan's sites against today's: each approved site, then the ones added since.
                let approval = r.approval.as_ref().map(|a| {
                    let mut rows: Vec<Vec<String>> = Vec::new();
                    let m3 = |v: Option<f64>| v.map_or_else(|| "—".into(), |v| grouped(v, 1));
                    for s in &a.approved.sites {
                        let ch = a.sites.iter().find(|c| c.name == s.name && c.approved_capacity.is_some());
                        let (cap, used) = ch.map_or((Some(s.capacity), Some(s.used)), |c| (c.capacity, c.used));
                        let state = match ch {
                            None => "aynı",
                            Some(c) if c.capacity.is_none() => "kalktı",
                            Some(_) => "değişti",
                        };
                        rows.push(vec![s.name.clone(), grouped(s.capacity, 1), m3(cap), grouped(s.used, 1), m3(used), state.to_owned()]);
                    }
                    for c in a.sites.iter().filter(|c| c.approved_capacity.is_none()) {
                        rows.push(vec![c.name.clone(), "—".into(), m3(c.capacity), "—".into(), m3(c.used), "eklendi".to_owned()]);
                    }
                    rows.push(vec![
                        "Taşıma momenti (m³·m)".into(),
                        String::new(),
                        String::new(),
                        grouped(a.approved.moment, 0),
                        a.moment_change.map_or_else(|| "—".into(), |c| grouped(a.approved.moment + c, 0)),
                        if a.moment_change.is_some_and(|c| c.abs() > 1e-6 * a.approved.moment.abs().max(1.0)) { "değişti" } else { "aynı" }.to_owned(),
                    ]);
                    road_dialog::table(
                        vec![
                            col("Alan", 150.0),
                            num("Onaylı kapasite", 115.0),
                            num("Kapasite", 95.0),
                            num("Onaylı kullanım", 120.0),
                            num("Bugün", 95.0),
                            col("Durum", 70.0),
                        ],
                        rows,
                        "",
                    )
                });
                let mut options = vec![Listed::Sites];
                if r.cost.is_some() {
                    options.push(Listed::Costs);
                }
                if approval.is_some() {
                    options.push(Listed::Approval);
                }
                let list = if options.contains(&w.list) { w.list } else { Listed::Sites };
                // The network's view (docs/adr/0955): what does not join, then each site's access.
                if let Some(Some(Ok((n, _)))) = (w.shown == Shown::Network).then_some(&w.network) {
                    let loose = n.problems.iter().filter(|p| p.kind == HaulNetworkIssue::Loose).count();
                    let crossing = n.problems.len() - loose;
                    if !n.problems.is_empty() {
                        d = d.note(Banner::warning(format!(
                            "Ağda birleşmeyen {} yer: {crossing} kesişme, {loose} boşta uç. {}",
                            n.problems.len(),
                            n.problems.iter().take(3).map(|p| p.message.clone()).collect::<Vec<_>>().join(" ")
                        )));
                    }
                    let rows: Vec<Vec<String>> = n
                        .sites
                        .iter()
                        .map(|s| {
                            vec![
                                s.name.clone(),
                                s.length.map_or_else(|| "—".into(), |l| grouped(l, 1)),
                                s.km.clone().unwrap_or_else(|| "—".into()),
                                s.offset.map_or_else(|| "—".into(), |o| grouped(o, 1)),
                                if s.roads.is_empty() { "—".into() } else { s.roads.iter().map(|x| (x + 1).to_string()).collect::<Vec<_>>().join(" → ") },
                                s.why.clone().unwrap_or_else(|| "aksa bağlı".into()),
                            ]
                        })
                        .collect();
                    let table = road_dialog::table(
                        vec![
                            col("Alan", 130.0),
                            num("Erişim (m)", 80.0),
                            col("Aksa km", 85.0),
                            num("Kayma (m)", 75.0),
                            col("Yollar", 70.0),
                            road_dialog::Col { title: "Durum", width: None, number: false },
                        ],
                        rows,
                        "Konumu verilen alan yok: alanlar yol üstünde",
                    );
                    d = d.detail(table);
                }
                let shows_network = matches!(&w.network, Some(Ok(_))) && w.shown == Shown::Network;
                d = match &r.cost {
                    _ if shows_network => d,
                    _ if list == Listed::Approval => {
                        let Some(t) = approval else { unreachable!() };
                        d.detail(road_dialog::detail_choice(options, list, |l| msg(Event::List(l)), t))
                    }
                    Some(c) => {
                        let money = |v: f64| format!("{} {}", grouped(v, 2), c.currency);
                        let mut rows = vec![
                            vec!["Serbest taşıma".to_owned(), format!("{} m", grouped(r.free.unwrap_or(0.0), 1))],
                            vec!["Fazla taşıma momenti".to_owned(), format!("{} m³·m", grouped(c.over_moment, 0))],
                            vec!["Fazla taşıma".to_owned(), money(c.overhaul)],
                            vec!["Döküm".to_owned(), money(c.waste)],
                            vec!["Ödünç".to_owned(), money(c.borrow)],
                            vec!["Toplam".to_owned(), money(c.total)],
                        ];
                        if let Some(e) = c.economic {
                            rows.insert(1, vec!["Ekonomik taşıma sınırı".to_owned(), format!("{} m", grouped(e, 1))]);
                        }
                        let costs = road_dialog::table(vec![col("Kalem", 220.0), num("Değer", 200.0)], rows, "");
                        let shown = if list == Listed::Costs { costs } else { sites };
                        d.detail(road_dialog::detail_choice(options, list, |l| msg(Event::List(l)), shown))
                    }
                    None if options.len() > 1 => d.detail(road_dialog::detail_choice(options, list, |l| msg(Event::List(l)), sites)),
                    None => d.detail(sites),
                };
                // Planı yaz (ROAD-HAUL-12): the summary with its units and the volume's revision and approval, the
                // flows, the sites and the barriers, as a workbook or a CSV.
                let formats: Vec<kentos_commands::WorkbookFormat> = crate::table_write::FORMATS.iter().map(|c| c.format).collect();
                let chosen = w.format.unwrap_or(kentos_commands::WorkbookFormat::Xlsx);
                let format = Select::new(
                    crate::table_write::FORMATS.iter().map(|c| Choice::new(c.label.to_owned())),
                    formats.iter().position(|f| *f == chosen),
                    move |i| msg(Event::Format(formats[i])),
                );
                d = d.tail(
                    iced::widget::row![
                        kentos_ui::label::caption("Taşıma planı")
                            .style(|t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) }),
                        iced::widget::container(format).width(240),
                        crate::exchange::words::secondary(WRITE, Some(msg(Event::Write))),
                    ]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
        }
        d.view()
    }

    pub(crate) fn haul_plan_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.road.haul_plan_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        if w.fleet.is_some() {
            use crate::haul_fleet_window::{BACK, Event as X, FIELDS as FF, TITLE as FT};
            let x = |e: X| msg(Event::Fleet(e));
            return Ok(match control {
                Control::Press(BACK) => Some(x(X::Back)),
                Control::Fill(name, text) => match FF.iter().position(|f| *f == name) {
                    Some(k) => Some(x(X::Field(k, text.to_owned()))),
                    None => return Err(format!("“{FT}” penceresinde “{name}” alanı yok")),
                },
                other => return Err(format!("“{FT}” penceresinde {other} yok")),
            });
        }
        Ok(match control {
            Control::Press(FLEET) => Some(msg(Event::OpenFleet)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(APPROVE) => Some(msg(Event::Approve(true))),
            Control::Press(FLOW_VIEW) => Some(msg(Event::Show(Shown::Flow))),
            Control::Press(NETWORK_VIEW) => Some(msg(Event::Show(Shown::Network))),
            Control::Press(REVOKE) => Some(msg(Event::Approve(false))),
            Control::Fill(APPROVAL_NOTE, text) => Some(msg(Event::ApprovalNote(text.to_owned()))),
            Control::Press(MOMENT) => Some(msg(Event::Aim(Aim::Moment))),
            Control::Press(COST) => Some(msg(Event::Aim(Aim::Cost))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

struct Flow<'a> {
    read: &'a HaulPlanned,
}

impl canvas::Program<Message> for Flow<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.read;
        let caption = |frame: &mut canvas::Frame, content: String, at: Point, color: Color| {
            frame.fill_text(Text { content, position: at, color, size: Pixels(11.0), ..Text::default() });
        };
        if r.segments.is_empty() {
            let why = if r.feasible { "Taşınacak bir şey yok." } else { "Plan kurulamadı: alanlar yetmiyor." };
            caption(&mut frame, why.into(), Point::new(10.0, 10.0), tk.muted);
            return vec![frame.into_geometry()];
        }
        let x0 = r.segments.first().map_or(0.0, |s| s.from).min(r.sites.iter().map(|s| s.station).fold(f64::INFINITY, f64::min));
        let x1 = r.segments.last().map_or(1.0, |s| s.to).max(r.sites.iter().map(|s| s.station).fold(f64::NEG_INFINITY, f64::max));
        let most = r.segments.iter().map(|s| s.flow.abs()).fold(1e-9, f64::max);
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let (left, right, top, bottom) = (56.0, 16.0, 26.0, 40.0);
        let mid = top + (h - top - bottom) / 2.0;
        let half = (h - top - bottom) / 2.0 - 6.0;
        let x = |s: f64| (left + (s - x0) / (x1 - x0).max(1e-9) * (w - left - right)) as f32;
        let y = |f: f64| (mid - f / most * half) as f32;
        let ahead = Color { a: 0.55, ..tk.accent };
        let back = Color { a: 0.55, ..tk.warning };
        for s in &r.segments {
            if s.flow == 0.0 {
                continue;
            }
            let band = Path::rectangle(
                Point::new(x(s.from), y(s.flow.max(0.0))),
                iced::Size::new(x(s.to) - x(s.from), (y(0.0) - y(s.flow.abs())).abs()),
            );
            frame.fill(&band, if s.flow > 0.0 { ahead } else { back });
            // The unsuitable cut's part, over the band from the axis (docs/adr/0944).
            if let Some(u) = s.unsuitable_flow.filter(|u| u.abs() > 1e-9) {
                let part = Path::rectangle(
                    Point::new(x(s.from), y(u.max(0.0))),
                    iced::Size::new(x(s.to) - x(s.from), (y(0.0) - y(u.abs())).abs()),
                );
                frame.fill(&part, Color { a: 0.75, ..Color::from_rgb8(0x93, 0x33, 0xea) });
            }
        }
        for b in &r.barriers {
            let at = x(b.station);
            let dash = Stroke { line_dash: canvas::LineDash { segments: &[5.0, 4.0], offset: 0 }, ..Stroke::default().with_width(1.5).with_color(tk.muted) };
            frame.stroke(&Path::line(Point::new(at, top as f32), Point::new(at, (h - bottom) as f32)), dash);
            let words = match b.detour {
                Some(dt) => format!("{} · +{} m", b.name, grouped(dt, 0)),
                None => format!("{} · kapalı", b.name),
            };
            caption(&mut frame, words, Point::new(at + 4.0, 6.0), tk.muted);
        }
        frame.stroke(&Path::line(Point::new(x(x0), y(0.0)), Point::new(x(x1), y(0.0))), Stroke::default().with_width(1.0).with_color(tk.muted));
        caption(&mut frame, format!("{} m³", grouped(most, 0)), Point::new(4.0, y(most) - 6.0), tk.muted);
        caption(&mut frame, format!("{} m³", grouped(-most, 0)), Point::new(4.0, y(-most) - 6.0), tk.muted);
        // A site the network does not reach is not in the plan (docs/adr/0955): not drawn on the road.
        for s in r.sites.iter().filter(|s| !s.unconnected) {
            let at = x(s.station);
            let (color, up) = if s.kind == HaulSiteKind::Borrow { (tk.success, true) } else { (tk.danger, false) };
            let base = y(0.0);
            let tip = if up { base - 12.0 } else { base + 12.0 };
            let mark = Path::new(|p| {
                p.move_to(Point::new(at, tip));
                p.line_to(Point::new(at - 6.0, if up { base - 2.0 } else { base + 2.0 }));
                p.line_to(Point::new(at + 6.0, if up { base - 2.0 } else { base + 2.0 }));
                p.close();
            });
            frame.fill(&mark, color);
            let room = (w - right) as f32 - 7.0 * s.name.chars().count() as f32;
            caption(&mut frame, s.name.clone(), Point::new((at - 18.0).clamp(4.0, room.max(4.0)), if up { tip - 14.0 } else { tip + 2.0 }), color);
        }
        caption(&mut frame, format!("{} m", grouped(x0, 0)), Point::new(x(x0) - 8.0, (h - bottom + 8.0) as f32), tk.muted);
        caption(&mut frame, format!("{} m", grouped(x1, 0)), Point::new(x(x1) - 30.0, (h - bottom + 8.0) as f32), tk.muted);
        vec![frame.into_geometry()]
    }
}

/// The haul road network in plan (docs/adr/0955): one scale both ways, north up.
struct NetworkMap<'a> {
    read: &'a HaulNetworkRead,
    drawn: &'a Drawn,
}

impl canvas::Program<Message> for NetworkMap<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let (n, dr) = (self.read, self.drawn);
        let caption = |frame: &mut canvas::Frame, content: String, at: Point, color: Color| {
            frame.fill_text(Text { content, position: at, color, size: Pixels(11.0), ..Text::default() });
        };
        // The extent: the roads, the sites and the alignment near them (the whole alignment where there are none).
        let mut pts: Vec<[f64; 2]> = dr.roads.iter().flatten().copied().collect();
        pts.extend(n.sites.iter().map(|s| [s.x, s.y]));
        let near: Vec<[f64; 2]> = if pts.is_empty() {
            dr.axis.clone()
        } else {
            let (lo, hi) = extent(&pts);
            let pad = (hi[0] - lo[0]).max(hi[1] - lo[1]) * 0.25 + 20.0;
            dr.axis.iter().copied().filter(|p| p[0] > lo[0] - pad && p[0] < hi[0] + pad && p[1] > lo[1] - pad && p[1] < hi[1] + pad).collect()
        };
        pts.extend(near.iter().copied());
        if pts.is_empty() {
            caption(&mut frame, "Ağın katmanlarında yol yok.".into(), Point::new(10.0, 10.0), tk.muted);
            return vec![frame.into_geometry()];
        }
        let (lo, hi) = extent(&pts);
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let m = 24.0;
        let scale = ((w - 2.0 * m) / (hi[0] - lo[0]).max(1e-6)).min((h - 2.0 * m) / (hi[1] - lo[1]).max(1e-6));
        let (ox, oy) = (m + ((w - 2.0 * m) - (hi[0] - lo[0]) * scale) / 2.0, m + ((h - 2.0 * m) - (hi[1] - lo[1]) * scale) / 2.0);
        let at = |p: [f64; 2]| Point::new((ox + (p[0] - lo[0]) * scale) as f32, (h - oy - (p[1] - lo[1]) * scale) as f32);
        let line = |frame: &mut canvas::Frame, ps: &[[f64; 2]], stroke: Stroke| {
            if ps.len() >= 2 {
                let path = Path::new(|b| {
                    b.move_to(at(ps[0]));
                    for p in &ps[1..] {
                        b.line_to(at(*p));
                    }
                });
                frame.stroke(&path, stroke);
            }
        };
        // The alignment: the visible stretch, thick and quiet.
        let mut run: Vec<[f64; 2]> = Vec::new();
        for p in &dr.axis {
            let inside = p[0] >= lo[0] - 1.0 && p[0] <= hi[0] + 1.0 && p[1] >= lo[1] - 1.0 && p[1] <= hi[1] + 1.0;
            if inside {
                run.push(*p);
            } else if !run.is_empty() {
                line(&mut frame, &run, Stroke::default().with_width(5.0).with_color(Color { a: 0.35, ..tk.muted }));
                run.clear();
            }
        }
        line(&mut frame, &run, Stroke::default().with_width(5.0).with_color(Color { a: 0.35, ..tk.muted }));
        // The roads; those on a site's way to the alignment stronger.
        let used: Vec<usize> = n.sites.iter().flat_map(|s| s.roads.iter().map(|r| *r as usize)).collect();
        for (k, road) in dr.roads.iter().enumerate() {
            let on = used.contains(&k);
            let color = if on { tk.accent } else { Color { a: 0.55, ..tk.accent } };
            line(&mut frame, road, Stroke::default().with_width(if on { 3.0 } else { 1.5 }).with_color(color));
            // Its number a third of the way along: the middle is where crossings and joins often fall.
            if let Some(p) = along(road, 0.35) {
                caption(&mut frame, format!("{}", k + 1), Point::new(at(p).x + 5.0, at(p).y - 13.0), tk.muted);
            }
        }
        // Where the roads meet the alignment, with the km. The labels' boxes are kept: the sites' avoid them.
        let wide = |t: &str| 6.3 * t.chars().count() as f32;
        let mut taken: Vec<Rectangle> = Vec::new();
        for j in &n.junctions {
            let Some(road) = dr.roads.get(j.road as usize) else { continue };
            let Some(p) = (if j.end { road.last() } else { road.first() }) else { continue };
            frame.fill(&Path::circle(at(*p), 4.0), tk.accent);
            let place = Point::new(at(*p).x + 6.0, at(*p).y + 2.0);
            taken.push(Rectangle::new(place, iced::Size::new(wide(&j.km), 13.0)));
            caption(&mut frame, j.km.clone(), place, tk.muted);
        }
        // What does not join.
        for pr in &n.problems {
            let c = at([pr.x, pr.y]);
            match pr.kind {
                HaulNetworkIssue::Loose => frame.stroke(&Path::circle(c, 6.0), Stroke::default().with_width(2.0).with_color(tk.warning)),
                _ => {
                    let s = 6.0;
                    frame.stroke(&Path::line(Point::new(c.x - s, c.y - s), Point::new(c.x + s, c.y + s)), Stroke::default().with_width(2.5).with_color(tk.warning));
                    frame.stroke(&Path::line(Point::new(c.x - s, c.y + s), Point::new(c.x + s, c.y - s)), Stroke::default().with_width(2.5).with_color(tk.warning));
                }
            }
        }
        // The sites: borrow up, waste down; hollow where the network does not reach them.
        for s in &n.sites {
            let c = at([s.x, s.y]);
            let borrow = s.kind == HaulSiteKind::Borrow;
            let color = if borrow { tk.success } else { tk.danger };
            let tip = if borrow { c.y - 9.0 } else { c.y + 9.0 };
            let base = if borrow { c.y + 5.0 } else { c.y - 5.0 };
            let mark = Path::new(|p| {
                p.move_to(Point::new(c.x, tip));
                p.line_to(Point::new(c.x - 7.0, base));
                p.line_to(Point::new(c.x + 7.0, base));
                p.close();
            });
            if s.joined {
                frame.fill(&mark, color);
            } else {
                frame.stroke(&mark, Stroke::default().with_width(2.0).with_color(color));
            }
            let words = match s.length {
                Some(l) => format!("{} · {} m", s.name, grouped(l, 0)),
                None => format!("{} · bağlanmıyor", s.name),
            };
            // Right of the mark, else left of it, else under it: wherever it does not cover another label.
            let size = iced::Size::new(wide(&words), 13.0);
            let tries = [
                Point::new(c.x + 9.0, c.y - 6.0),
                Point::new(c.x - 9.0 - size.width, c.y - 6.0),
                Point::new(c.x - size.width / 2.0, c.y + 12.0),
                Point::new(c.x - size.width / 2.0, c.y - 24.0),
            ];
            let place = tries
                .iter()
                .copied()
                .find(|q| !taken.iter().any(|r| r.intersects(&Rectangle::new(*q, size))))
                .unwrap_or(tries[0]);
            taken.push(Rectangle::new(place, size));
            caption(&mut frame, words, place, color);
        }
        // The scale: a round length about a fifth of the width.
        let want = (w / 5.0) / scale;
        let step = [1.0, 2.0, 5.0].iter().flat_map(|m| (0..6).map(move |e| m * 10f64.powi(e))).filter(|v| *v <= want).fold(1.0, f64::max);
        let (sx, sy) = (m as f32, (h - 10.0) as f32);
        frame.stroke(&Path::line(Point::new(sx, sy), Point::new(sx + (step * scale) as f32, sy)), Stroke::default().with_width(2.0).with_color(tk.muted));
        caption(&mut frame, format!("{} m", grouped(step, 0)), Point::new(sx + (step * scale) as f32 + 6.0, sy - 7.0), tk.muted);
        vec![frame.into_geometry()]
    }
}

/// The point `share` of the way along a polyline.
fn along(pts: &[[f64; 2]], share: f64) -> Option<[f64; 2]> {
    let total: f64 = pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
    let mut left = total * share;
    for w in pts.windows(2) {
        let l = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if l >= left && l > 0.0 {
            let t = left / l;
            return Some([w[0][0] + t * (w[1][0] - w[0][0]), w[0][1] + t * (w[1][1] - w[0][1])]);
        }
        left -= l;
    }
    pts.last().copied()
}

fn extent(pts: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    pts.iter().fold(([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]), |(lo, hi), p| {
        ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
    })
}
