//! Güzergâh (plan/road.md Y3, `alignment.editor`): a road project's horizontal alignments, as OpenRoads' Complex by
//! PI table and Netcad's güzergâh dialog make them: the PIs in a table (their Y, X, the curve's radius and the
//! clothoids' lengths in and out), the km at the start and the station equations; a parallel axis by its source and
//! offset. The alignment being edited is built as it is typed (`kentos.alignment/1`), so its length, km and curves, or
//! why it does not build, show before it is saved.
//!
//! ```text
//! ┌ Güzergâh ──────────────────────────────────────────────────────────────────────────────┐
//! │ Akslar             │ Ad [Ana aks                ]        Başlangıç km [2000          ]   │
//! │ ● Ana aks   aks    │ PI   Y             X              R        L giriş   L çıkış        │
//! │   Sol kenar ∥ -7,5 │ a    485000,000    4420000,000                                      │
//! │ [Yeni aks]         │ p1   485600,000    4420000,000    400      80        80             │
//! │ [Paralel aks]      │ [Satır ekle] [Satırı sil] [Seçili çizgiden al]                      │
//! │                    │ Km eşitlikleri  geri [2200] ileri [2500] [x] artan  [Ekle] [Sil]   │
//! │                    │ 1 289,370 m · km 2+000,000–3+289,370 · 1 kurp                       │
//! │                    │ Noktanın km'si [485520, 4420030] → km 2+525,205 · sol 19,496 m      │
//! │ [Aksı sil] [Ters çevir]                                           [Vazgeç] [Kaydet]     │
//! └────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the definitions; Kaydet writes what changed through `road.alignments.edit` (removes,
//! sets) and `road.alignments.create` (the new ones), each one undo step, and closes. Ters çevir reverses a saved
//! alignment (`road.alignments.edit reverse`).

use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_commands::{
    AlignmentDef, AlignmentOffset, AlignmentPi, AlignmentStationing, AlignmentsCreate, AlignmentsEdit,
    AlignmentsEditOperation, AlignmentsQuery, StationEquationDef,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_alignments, road_alignments_create, road_alignments_edit, road_alignments_query,
};
use kentos_road::alignment::{self as core, km_text};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Güzergâh";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni aks";
const PARALLEL: &str = "Paralel aks";
const REMOVE: &str = "Aksı sil";
const REVERSE: &str = "Ters çevir";
const ADD_PI: &str = "Satır ekle";
const REMOVE_PI: &str = "Satırı sil";
const FROM_SELECTION: &str = "Seçili çizgiden al";
const LABEL: &str = "Km'yi çizime yaz";

/// A PI as typed: its id and its numbers' texts.
#[derive(Debug, Clone, PartialEq)]
pub struct PiRow {
    id: String,
    texts: [String; 5],
}

/// An equation as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct EquationRow {
    back: String,
    ahead: String,
    increasing: bool,
}

/// An alignment being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    id: String,
    name: String,
    revision: u32,
    pis: Vec<PiRow>,
    start: String,
    equations: Vec<EquationRow>,
    /// A parallel axis: its source's id and its offset's text.
    parallel: Option<(String, String)>,
}

fn text(v: f64) -> String {
    if v == 0.0 { String::new() } else { trimmed(v) }
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

/// A number the user typed, with a comma or a point; empty: none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or(())
}

impl Row {
    fn of(d: &AlignmentDef) -> Self {
        let st = d.stationing.clone().unwrap_or_default();
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            pis: d
                .pis
                .iter()
                .map(|p| PiRow {
                    id: p.id.clone(),
                    texts: [trimmed(p.x), trimmed(p.y), text(p.radius), text(p.spiral_in), text(p.spiral_out)],
                })
                .collect(),
            start: if d.stationing.is_some() { trimmed(st.start) } else { String::new() },
            equations: st
                .equations
                .iter()
                .map(|q| EquationRow {
                    back: trimmed(q.back),
                    ahead: trimmed(q.ahead),
                    increasing: q.increasing,
                })
                .collect(),
            parallel: d.offset_of.as_ref().map(|o| (o.alignment.clone(), trimmed(o.offset))),
        }
    }

    /// The definition the row says, or what is wrong with a number.
    fn def(&self) -> Result<AlignmentDef, String> {
        let mut pis = Vec::new();
        for (i, p) in self.pis.iter().enumerate() {
            let mut v = [0.0; 5];
            for (j, t) in p.texts.iter().enumerate() {
                v[j] = match number(t) {
                    Ok(Some(n)) => n,
                    Ok(None) if j >= 2 => 0.0,
                    _ => return Err(format!("{}. satırın {} değeri sayı olmalı.", i + 1, ["Y", "X", "R", "L giriş", "L çıkış"][j])),
                };
            }
            pis.push(AlignmentPi {
                id: p.id.clone(),
                x: v[0],
                y: v[1],
                radius: v[2],
                spiral_in: v[3],
                spiral_out: v[4],
            });
        }
        let stationing = if self.parallel.is_some() {
            None
        } else {
            let start = number(&self.start).map_err(|_| "Başlangıç km'si sayı olmalı.".to_owned())?;
            let mut equations = Vec::new();
            for (i, q) in self.equations.iter().enumerate() {
                let (Ok(Some(back)), Ok(Some(ahead))) = (number(&q.back), number(&q.ahead)) else {
                    return Err(format!("{}. km eşitliğinin geri ve ileri km'si sayı olmalı.", i + 1));
                };
                equations.push(StationEquationDef {
                    back,
                    ahead,
                    increasing: q.increasing,
                });
            }
            let st = AlignmentStationing {
                start: start.unwrap_or(0.0),
                equations,
            };
            (!st.is_empty()).then_some(st)
        };
        let offset_of = match &self.parallel {
            Some((src, off)) => Some(AlignmentOffset {
                alignment: src.clone(),
                offset: number(off).ok().flatten().ok_or("Öteleme sayı olmalı (sağa artı, sola eksi).")?,
            }),
            None => None,
        };
        Ok(AlignmentDef {
            id: self.id.clone(),
            name: self.name.clone(),
            revision: self.revision,
            pis: if offset_of.is_some() { Vec::new() } else { pis },
            offset_of,
            stationing,
        })
    }
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<AlignmentDef>,
    chosen: Option<usize>,
    /// The PI row chosen in the table.
    pi: Option<usize>,
    /// Noktanın km'si: the place typed, and its answer.
    point: String,
    pub(crate) answer: String,
    /// Km'yi çizime yaz: the layer the km goes on.
    layer: Option<String>,
    /// What the PIs typed would do to what hangs on the alignment's stations (docs/adr/0963).
    pub(crate) impact: Option<Result<(kentos_commands::AlignmentImpact, Vec<kentos_commands::CommandWarning>), String>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Parallel,
    Remove,
    Reverse,
    Name(String),
    Start(String),
    PiChoose(usize),
    Pi(usize, usize, String),
    PiAdd,
    PiRemove,
    FromSelection,
    EquationAdd,
    EquationRemove(usize),
    Back(usize, String),
    Ahead(usize, String),
    Increasing(usize, bool),
    Source(String),
    Offset(String),
    Point(String),
    LabelLayer(String),
    Label,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Alignments(event)
}

/// What the row builds to now: its alignment's line, or why it does not build.
/// The impact in words (docs/adr/0963): where the line leaves its place, how far the km after it slide, what moves and
/// what is left without a place.
fn impact_words(imp: &kentos_commands::AlignmentImpact) -> String {
    if imp.clear {
        return "Aks ve ona bağlı km'ler 1 mm'den çok değişmiyor.".into();
    }
    let comma = |v: f64| format!("{v:.3}").replace('.', ",");
    let mut parts: Vec<String> = Vec::new();
    for r in &imp.removed {
        parts.push(match r.most {
            Some(m) => format!("km {} – {} yerinden kayar (en çok {} m)", r.from_text, r.to_text, comma(m)),
            None => format!("km {} – {} kalkar", r.from_text, r.to_text),
        });
    }
    for r in &imp.added {
        parts.push(format!("yeni km {} – {}", r.from_text, r.to_text));
    }
    if let Some(run) = imp.runs.iter().rev().find(|r| r.shift_to.abs() > 1e-6) {
        parts.push(format!("km {}'den sonra km'ler {} m kayar", run.from_text, comma(run.shift_to)));
    }
    let mut said = format!("Kaydedilirse: {}.", parts.join("; "));
    let moved = imp.refs.iter().filter(|r| r.to.is_some()).count();
    let lost = imp.refs.len() - moved;
    if !imp.refs.is_empty() {
        said.push_str(&format!(" Bağlı km'lerden {moved} taşınır, {lost} yerini yitirir."));
    }
    let with = |d: &kentos_commands::ImpactDerived| if d.approved { format!("{} (onaylı)", d.name) } else { d.name.clone() };
    let stale: Vec<String> = imp.volumes.iter().map(with).chain(imp.hauls.iter().map(with)).collect();
    if !stale.is_empty() {
        said.push_str(&format!(" Eskiyecekler: {}.", stale.join(", ")));
    }
    if !imp.parallels.is_empty() {
        said.push_str(&format!(" İzleyen paralel akslar: {}.", imp.parallels.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")));
    }
    said
}

/// The chosen alignment's status line, as the window writes it (for the tests).
pub(crate) fn status_of(w: &Window) -> String {
    w.chosen.and_then(|i| w.rows.get(i)).map_or_else(String::new, |r| status(&w.rows, r))
}

fn status(rows: &[Row], r: &Row) -> String {
    let defs: Result<Vec<AlignmentDef>, String> = rows.iter().map(Row::def).collect();
    let (defs, def) = match (defs, r.def()) {
        (Ok(d), Ok(def)) => (d, def),
        (Err(e), _) | (_, Err(e)) => return e,
    };
    let source = match &def.offset_of {
        Some(o) => match defs.iter().find(|d| d.id == o.alignment) {
            Some(s) => s.clone(),
            None => return "Paralel aksın kaynağını seçin.".into(),
        },
        None => def.clone(),
    };
    let pis: Vec<core::PiInput> = source
        .pis
        .iter()
        .map(|p| core::PiInput {
            id: p.id.clone(),
            x: p.x,
            y: p.y,
            radius: p.radius,
            spiral_in: p.spiral_in,
            spiral_out: p.spiral_out,
        })
        .collect();
    let al = match core::build(&pis) {
        Ok(a) => a,
        Err(e) => return format!("Kurulamıyor: {}", chain_words(&e)),
    };
    let st = road_alignments::stationing_of(source.stationing.as_ref());
    let regions = match st.regions(al.length) {
        Ok(r) => r,
        Err(_) => return "Km eşitliği aksın üstünde, bir öncekinden sonra olmalı.".into(),
    };
    let comma = |v: f64| format!("{v:.3}").replace('.', ",");
    if let Some(o) = &def.offset_of
        && let Err(core::offset::OffsetError::Cusp { from, to } | core::offset::OffsetError::Crosses { a: from, b: to }) =
            core::offset::check(&al, o.offset)
    {
        return format!("Paralel aks kıvrılıyor: km {}–{} arası.", comma(from), comma(to));
    }
    let length = def.offset_of.as_ref().map_or(al.length, |o| core::offset::length(&al, o.offset));
    let joins = al.joins.iter().filter(|j| j.reverse).count();
    let mut words = format!(
        "{} m · km {}–{} · {} kurp",
        comma(length),
        km_text(core::Stationing::km(&regions, 0.0), 3).replace('.', ","),
        km_text(core::Stationing::km(&regions, al.length), 3).replace('.', ","),
        al.curves.len()
    );
    if joins > 0 {
        words.push_str(&format!(" · {joins} ters kurp"));
    }
    // Continuity (docs/adr/0967): kinks (a direction jump) and curvature jumps (a circular curve without a clothoid),
    // else G2 throughout.
    let seams = al.seams();
    let kinks: Vec<String> = seams
        .iter()
        .filter(|s| s.direction > 1e-9)
        .map(|s| format!("{} kırık {}°", s.after.split('/').next().unwrap_or(""), comma(s.direction.to_degrees())))
        .collect();
    let jumps = seams.iter().filter(|s| s.curvature > 1e-9).count();
    if !kinks.is_empty() {
        words.push_str(&format!(" · {}", kinks.join(", ")));
    }
    if jumps > 0 {
        words.push_str(&format!(" · {jumps} yerde eğrilik sıçrar (klotoidsiz kurp)"));
    }
    if kinks.is_empty() && jumps == 0 && !seams.is_empty() {
        words.push_str(" · konum, doğrultu ve eğrilik sürekli");
    }
    words
}

fn chain_words(e: &core::ChainError) -> String {
    use core::ChainError as E;
    match e {
        E::TooFewPoints => "en az iki nokta gerekir".into(),
        E::DuplicateId { pi } => format!("{pi} kimliği iki kez var"),
        E::NotFinite { pi } => format!("{pi} noktasının sayıları sonlu değil"),
        E::SamePlace { pi } => format!("{pi} bir öncekiyle aynı yerde"),
        E::BadCurve { pi } => format!("{pi} noktasının kurpu olamaz"),
        E::TurnTooSharp { pi } => format!("{pi} noktasında doğrultu geri dönüyor"),
        E::SpiralsLong { pi, .. } => format!("{pi} noktasında klotoidler sapmadan uzun"),
        E::CurvesOverlap { a, b, excess } => format!(
            "{a}–{b} aliymanına kurplar sığmıyor, {} m taşıyor",
            format!("{excess:.3}").replace('.', ",")
        ),
    }
}

impl App {
    /// Güzergâh: the window on the road project's alignments.
    pub(crate) fn open_alignments(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Aks yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_alignments().to_vec();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.alignment_window = Some(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            layer,
            ..Window::default()
        });
        self.dialog = Some(Dialog::Alignments);
    }

    /// Yeni aks from the ribbon: the window with a new alignment, its PIs from a selected line when there is one.
    pub(crate) fn new_alignment(&mut self) {
        self.open_alignments();
        if self.alignment_window.is_some() {
            let _ = self.alignment_event(Event::New);
            if !self.selection.ids().is_empty() {
                let _ = self.alignment_event(Event::FromSelection);
            }
        }
    }

    /// The vertices of the one selected line, polyline or area.
    fn selected_path(&self) -> Option<Vec<[f64; 2]>> {
        use kentos_commands::Entity;
        let doc = self.document.as_ref()?;
        let [slot] = self.selection.ids() else {
            return None;
        };
        match doc.model.get(*slot)? {
            Entity::Line(l) => Some(vec![[l.a.x, l.a.y], [l.b.x, l.b.y]]),
            Entity::Polyline(p) | Entity::Polygon(p) => Some(p.pts.iter().map(|v| [v.x, v.y]).collect()),
            _ => None,
        }
    }

    pub(crate) fn alignment_event(&mut self, event: Event) -> Task<Message> {
        let path = matches!(event, Event::FromSelection).then(|| self.selected_path()).flatten();
        let Some(w) = self.alignment_window.as_mut() else {
            return Task::none();
        };
        let mut warning = None;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.pi = None;
                w.answer.clear();
            }
            Event::New | Event::Parallel => {
                let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                let id = (1..).map(|n| format!("g-{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                let n = w.rows.len() + 1;
                let parallel = matches!(event, Event::Parallel);
                let source = w.chosen.and_then(|i| w.rows.get(i)).filter(|r| r.parallel.is_none()).map(|r| r.id.clone());
                if parallel && source.is_none() {
                    warning = Some("Paralel aks için önce PI'larıyla kurulan bir aks seçin.");
                } else {
                w.rows.push(Row {
                    id,
                    name: if parallel {
                        format!("Paralel {n}")
                    } else if n == 1 {
                        "Ana aks".to_owned()
                    } else {
                        format!("Aks {n}")
                    },
                    revision: 0,
                    pis: if parallel {
                        Vec::new()
                    } else {
                        ["a", "b"].iter().map(|id| PiRow { id: (*id).to_owned(), texts: Default::default() }).collect()
                    },
                    start: String::new(),
                    equations: Vec::new(),
                    parallel: source.filter(|_| parallel).map(|s| (s, "7,5".to_owned())),
                });
                w.chosen = Some(w.rows.len() - 1);
                w.pi = None;
                }
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    let id = w.rows[i].id.clone();
                    if w.rows.iter().any(|r| r.parallel.as_ref().is_some_and(|(s, _)| *s == id)) {
                        warning = Some("Bu aksı paralel aks izliyor; önce paralel aksı silin.");
                    } else {
                        w.rows.remove(i);
                        w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                    }
                }
            }
            Event::Reverse => return self.reverse_alignment(),
            Event::Name(t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.name = t;
                }
            }
            Event::Start(t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.start = t;
                }
            }
            Event::PiChoose(j) => w.pi = Some(j),
            Event::Pi(j, k, t) => {
                if let Some(p) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.pis.get_mut(j)) {
                    if k == 5 {
                        p.id = t;
                    } else {
                        p.texts[k] = t;
                    }
                }
                w.pi = Some(j);
            }
            Event::PiAdd => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    let taken: Vec<String> = r.pis.iter().map(|p| p.id.clone()).collect();
                    let id = (1..).map(|n| format!("p{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                    // Before the end, or after the chosen row.
                    let at = w.pi.map_or(r.pis.len().saturating_sub(1), |j| (j + 1).min(r.pis.len().saturating_sub(1))).max(1);
                    r.pis.insert(at.min(r.pis.len()), PiRow { id, texts: Default::default() });
                    w.pi = Some(at);
                }
            }
            Event::PiRemove => {
                if let (Some(r), Some(j)) = (w.chosen.and_then(|i| w.rows.get_mut(i)), w.pi)
                    && j < r.pis.len()
                {
                    r.pis.remove(j);
                    w.pi = None;
                }
            }
            Event::FromSelection => match (w.chosen.and_then(|i| w.rows.get_mut(i)), path) {
                (Some(r), Some(path)) if path.len() >= 2 && r.parallel.is_none() => {
                    let n = path.len();
                    r.pis = path
                        .iter()
                        .enumerate()
                        .map(|(i, p)| PiRow {
                            id: if i == 0 {
                                "a".into()
                            } else if i == n - 1 {
                                "b".into()
                            } else {
                                format!("p{i}")
                            },
                            texts: [trimmed(p[0]), trimmed(p[1]), String::new(), String::new(), String::new()],
                        })
                        .collect();
                }
                _ => warning = Some("Çizimde tek bir çizgi, çoklu çizgi ya da alan seçin: köşeleri PI olur."),
            },
            Event::EquationAdd => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.equations.push(EquationRow { back: String::new(), ahead: String::new(), increasing: true });
                }
            }
            Event::EquationRemove(j) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i))
                    && j < r.equations.len()
                {
                    r.equations.remove(j);
                }
            }
            Event::Back(j, t) => {
                if let Some(q) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.equations.get_mut(j)) {
                    q.back = t;
                }
            }
            Event::Ahead(j, t) => {
                if let Some(q) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.equations.get_mut(j)) {
                    q.ahead = t;
                }
            }
            Event::Increasing(j, on) => {
                if let Some(q) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.equations.get_mut(j)) {
                    q.increasing = on;
                }
            }
            Event::Source(id) => {
                if let Some((s, _)) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.parallel.as_mut()) {
                    *s = id;
                }
            }
            Event::Offset(t) => {
                if let Some((_, o)) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.parallel.as_mut()) {
                    *o = t;
                }
            }
            Event::Point(t) => {
                w.point = t;
                return self.ask_point();
            }
            Event::LabelLayer(id) => w.layer = Some(id),
            Event::Label => return self.label_alignment(),
            Event::Cancel => {
                self.alignment_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_alignments(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        self.refresh_alignment_impact();
        Task::none()
    }

    /// The impact of the chosen alignment's PIs as typed against the saved ones (`road.alignments.impact`, docs/adr/0963):
    /// none for a new alignment, a parallel axis, PIs that do not read or are the saved ones.
    fn refresh_alignment_impact(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.alignment_window.as_mut() else { return };
        w.impact = w.chosen.and_then(|i| w.rows.get(i)).and_then(|r| r.def().ok()).and_then(|d| {
            let kept = w.kept.iter().find(|k| k.id == d.id && k.offset_of.is_none())?;
            (kept.pis != d.pis && d.offset_of.is_none()).then(|| {
                let input = kentos_commands::AlignmentsImpact { alignment: d.id.clone(), pis: d.pis.clone(), reverse: false, tolerance: None };
                kentos_native_application::road_alignments_impact::preview(&doc.model, &input)
            })
        });
    }

    /// Noktanın km'si: the typed place's km and offset on the chosen saved alignment (`road.alignments.query`).
    fn ask_point(&mut self) -> Task<Message> {
        let Some(w) = self.alignment_window.as_mut() else {
            return Task::none();
        };
        let parts: Vec<Option<f64>> = w.point.split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
        let [Some(x), Some(y)] = parts[..] else {
            w.answer = if w.point.trim().is_empty() { String::new() } else { "Yeri “Y; X” olarak yazın.".into() };
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !w.kept.iter().any(|k| k.id == r.id) {
            w.answer = "Önce aksı kaydedin.".into();
            return Task::none();
        }
        let id = r.id.clone();
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let mut copy = doc.model.reading_copy();
        let got = road_alignments_query::plan(
            &ExecutionContext::new(&mut copy),
            &AlignmentsQuery { alignment: Some(id), points: vec![[x, y]], ..AlignmentsQuery::default() },
        );
        let answer = match got {
            kentos_commands::CommandResult::Completed { output, .. } => match output.points.first() {
                Some(p) => match (&p.nearest, &p.beyond) {
                    (Some(f), _) => format!(
                        "km {} · {} {} m{}",
                        f.at.km_text.replace('.', ","),
                        if f.offset >= 0.0 { "sağ" } else { "sol" },
                        format!("{:.3}", f.offset.abs()).replace('.', ","),
                        if p.feet.len() > 1 { format!(" · {} ayak", p.feet.len()) } else { String::new() }
                    ),
                    (None, Some(end)) => {
                        format!("Nokta aksın {} ucunun ötesinde.", if end == "start" { "başlangıç" } else { "bitiş" })
                    }
                    _ => String::new(),
                },
                None => String::new(),
            },
            _ => "Sorulamadı.".into(),
        };
        if let Some(w) = self.alignment_window.as_mut() {
            w.answer = answer;
        }
        Task::none()
    }

    /// Km'yi çizime yaz: the chosen saved alignment's km on the chosen layer (`road.alignments.label`), one undo step.
    fn label_alignment(&mut self) -> Task<Message> {
        let Some(w) = &self.alignment_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)) {
            self.warn("Önce aksı kaydedin; km kaydedilmiş akstan yazılır.".to_owned());
            return Task::none();
        }
        let input = kentos_commands::AlignmentsLabel {
            alignment: r.id.clone(),
            layer_id: w.layer.clone().unwrap_or_default(),
            every: None,
            major: None,
            height: None,
            curves: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = kentos_native_application::road_alignments_label::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let words = out.labels.iter().filter(|l| l.text.is_some()).count();
            self.say(Level::Success, format!("{} işaret ve {words} km yazısı çizildi.", out.labels.len()));
        }
        Task::none()
    }

    /// Ters çevir: the chosen saved alignment reversed (`road.alignments.edit reverse`), one undo step.
    fn reverse_alignment(&mut self) -> Task<Message> {
        let Some(w) = &self.alignment_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)) {
            self.warn("Önce aksı kaydedin; kaydedilmiş aks ters çevrilir.".to_owned());
            return Task::none();
        }
        let edit = AlignmentsEdit {
            alignment: r.id.clone(),
            operation: AlignmentsEditOperation::Reverse,
            name: None,
            pis: None,
            stationing: None,
            offset: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        // What does not turn with it, read before (docs/adr/0965): told after it is turned.
        let stale: Vec<String> = kentos_native_application::road_alignments_impact::preview(
            &doc.model,
            &kentos_commands::AlignmentsImpact { alignment: edit.alignment.clone(), pis: Vec::new(), reverse: true, tolerance: None },
        )
        .map(|(_, w)| w.into_iter().filter(|w| w.code == "alignment_reverse_stale").map(|w| w.message).collect())
        .unwrap_or_default();
        let result = road_alignments_edit::execute(&mut ExecutionContext::new(&mut doc.model), edit);
        if let Some(out) = self.answered(result) {
            // Its profiles and superelevation schedules turned round with it (docs/adr/0964, 0965): said by name.
            let names: Vec<String> = self.document.as_ref().map_or_else(Vec::new, |d| {
                let profiles = out.profiles.iter().filter_map(|id| {
                    d.model.road_profiles().iter().find(|p| p.id == *id).map(|p| format!("“{}” boykesiti", p.name))
                });
                let supers = out.superelevations.iter().filter_map(|id| {
                    d.model.road_superelevations().iter().find(|s| s.id == *id).map(|s| format!("“{}” dever çizelgesi", s.name))
                });
                profiles.chain(supers).collect()
            });
            self.reload_alignments();
            if names.is_empty() {
                self.say(Level::Success, "Aks ters çevrildi.");
            } else {
                self.say(Level::Success, format!("Aks ters çevrildi; {} de km'leriyle döndü.", names.join(", ")));
            }
            for words in stale {
                self.warn(words);
            }
        }
        Task::none()
    }

    fn reload_alignments(&mut self) {
        let Some(doc) = &self.document else { return };
        let kept = doc.model.road_alignments().to_vec();
        if let Some(w) = self.alignment_window.as_mut() {
            w.rows = kept.iter().map(Row::of).collect();
            w.kept = kept;
            w.chosen = w.chosen.filter(|i| *i < w.rows.len());
        }
    }

    /// Kaydet: what changed, the removed first (parallel axes before their sources), then the changed, then the new.
    fn save_alignments(&mut self) -> Task<Message> {
        let Some(w) = self.alignment_window.clone() else {
            return Task::none();
        };
        let mut defs = Vec::new();
        for r in &w.rows {
            match r.def() {
                Ok(d) => defs.push(d),
                Err(e) => {
                    self.warn(format!("“{}”: {e}", r.name));
                    return Task::none();
                }
            }
        }
        let mut removed: Vec<&AlignmentDef> = w.kept.iter().filter(|k| defs.iter().all(|d| d.id != k.id)).collect();
        removed.sort_by_key(|k| k.offset_of.is_none());
        for k in removed {
            let edit = AlignmentsEdit {
                alignment: k.id.clone(),
                operation: AlignmentsEditOperation::Remove,
                name: None,
                pis: None,
                stationing: None,
                offset: None,
                expected_revision: None,
            };
            if !self.alignment_write(|cx| road_alignments_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        // Sources before their parallel axes.
        let mut order: Vec<&AlignmentDef> = defs.iter().collect();
        order.sort_by_key(|d| d.offset_of.is_some());
        for d in order {
            match w.kept.iter().find(|k| k.id == d.id) {
                Some(k) if k == d => {}
                Some(k) => {
                    let edit = AlignmentsEdit {
                        alignment: d.id.clone(),
                        operation: AlignmentsEditOperation::Set,
                        name: (k.name != d.name).then(|| d.name.clone()),
                        pis: (k.pis != d.pis && d.offset_of.is_none()).then(|| d.pis.clone()),
                        stationing: (k.stationing != d.stationing).then(|| d.stationing.clone().unwrap_or_default()),
                        offset: d.offset_of.as_ref().filter(|_| k.offset_of != d.offset_of).map(|o| o.offset),
                        expected_revision: None,
                    };
                    if !self.alignment_write(|cx| road_alignments_edit::execute(cx, edit)) {
                        return Task::none();
                    }
                }
                None => {
                    let create = AlignmentsCreate {
                        name: d.name.clone(),
                        id: Some(d.id.clone()),
                        pis: d.pis.clone(),
                        offset_of: d.offset_of.clone(),
                        stationing: d.stationing.clone(),
                        expected_revision: None,
                    };
                    if !self.alignment_write(|cx| road_alignments_create::execute(cx, create)) {
                        return Task::none();
                    }
                }
            }
        }
        self.alignment_window = None;
        self.dialog = None;
        self.say(Level::Success, "Akslar kaydedildi.");
        Task::none()
    }

    /// One write; on a refusal the window stays.
    fn alignment_write<T: serde::Serialize>(
        &mut self,
        write: impl FnOnce(&mut ExecutionContext<'_>) -> kentos_commands::CommandResult<T>,
    ) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = write(&mut ExecutionContext::new(&mut doc.model));
        self.answered(result).is_some()
    }

    pub(crate) fn alignment_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.alignment_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let kind = match &r.parallel {
                Some((src, off)) => {
                    let name = w.rows.iter().find(|x| x.id == *src).map_or(src.clone(), |x| x.name.clone());
                    format!("paralel · {name} {off} m")
                }
                None => format!("{} PI", r.pis.len()),
            };
            let face = column![
                Elided::new(r.name.clone()).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(kind).style(muted),
            ]
            .spacing(1);
            list = list.push(
                iced::widget::button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.rows.is_empty() {
            list = list.push(
                label::caption("Projede aks yok. Yeni aks ile PI'lardan bir güzergâh kurun; seçili bir çoklu çizginin köşeleri PI olur.")
                    .style(muted),
            );
        }
        let left = column![
            label::strong("Akslar"),
            container(scrollable(list).height(typography::scaled(300.0))).style(style::container::field_box).width(Fill),
            row![
                words::secondary(NEW, Some(msg(Event::New))),
                words::secondary(PARALLEL, Some(msg(Event::Parallel))),
            ]
            .spacing(8),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir aks seçin ya da yeni bir aks oluşturun.")).padding(12).into(),
            Some(r) => self.alignment_editor(w, r),
        };
        let saved = chosen.is_some_and(|r| w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)));
        let ids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let names: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let layer = kentos_ui::widget::select::Select::new(
            names.iter().map(|n| kentos_ui::widget::select::Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| ids.iter().position(|id| id == l)),
            move |i| msg(Event::LabelLayer(ids[i].clone())),
        )
        .placeholder("Katman");
        let km = row![
            label::strong("Km yazısı"),
            words::field("Katman", layer, None),
            words::secondary(LABEL, saved.then(|| msg(Event::Label))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        // The editor scrolls with the body (resized or full screen it grows); the drawing row stays above the footer.
        let body = row![container(left).width(230), container(right).width(Fill)].spacing(16);
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer = row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(
                    REVERSE,
                    (saved && chosen.is_some_and(|r| r.parallel.is_none())).then(|| msg(Event::Reverse))
                ),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center);
        Frame::new(TITLE).scroll(body).push(km).footer(footer).width(900.0).modal(msg(Event::Cancel))
    }

    fn alignment_editor<'a>(&'a self, w: &'a Window, r: &'a Row) -> Element<'a, Message> {
        let name = kentos_ui::widget::text_field("Aksın adı", &r.name).on_input(|t| msg(Event::Name(t)));
        let mut col = Column::new().spacing(10).padding(iced::Padding::default().right(14));
        match &r.parallel {
            Some((src, off)) => {
                let sources: Vec<&Row> = w.rows.iter().filter(|x| x.parallel.is_none()).collect();
                let ids: Vec<String> = sources.iter().map(|x| x.id.clone()).collect();
                let pick = kentos_ui::widget::select::Select::new(
                    sources.iter().map(|x| kentos_ui::widget::select::Choice::new(x.name.clone())),
                    ids.iter().position(|id| id == src),
                    move |i| msg(Event::Source(ids[i].clone())),
                )
                .placeholder("Kaynak aks");
                col = col.push(words::field("Ad", name, None)).push(
                    row![
                        words::field("Kaynak aks", pick, None),
                        words::field(
                            "Öteleme (m, sağa artı)",
                            kentos_ui::widget::text_field("7,5", off).on_input(|t| msg(Event::Offset(t))),
                            None
                        ),
                    ]
                    .spacing(12),
                );
            }
            None => {
                let start = kentos_ui::widget::text_field("0", &r.start).on_input(|t| msg(Event::Start(t)));
                col = col.push(row![words::field("Ad", name, None), words::field("Başlangıç km (m)", start, None)].spacing(12));
                let pis = &r.pis;
                let fill = |title| road_dialog::Col { title, width: None, number: true };
                let table = road_dialog::edit_table(
                    vec![road_dialog::col("PI", 80.0), fill("Y (m)"), fill("X (m)"), fill("R (m)"), fill("L giriş (m)"), fill("L çıkış (m)")],
                    pis.len(),
                    move |j, k| {
                        let p = &pis[j];
                        let end = j == 0 || j + 1 == pis.len();
                        match k {
                            0 => road_dialog::edit(&p.id, false, move |t| msg(Event::Pi(j, 5, t))),
                            // The ends have no curve: their R and spirals are not typed.
                            k if end && k >= 3 => kentos_ui::widget::table::text("—").into(),
                            k => road_dialog::edit(&p.texts[k - 1], true, move |t| msg(Event::Pi(j, k - 1, t))),
                        }
                    },
                    w.pi,
                    |j| msg(Event::PiChoose(j)),
                    "PI yok",
                );
                col = col.push(words::field("PI'lar", table, None)).push(
                    row![
                        words::secondary(ADD_PI, Some(msg(Event::PiAdd))),
                        words::ghost(REMOVE_PI, w.pi.filter(|j| *j < r.pis.len()).map(|_| msg(Event::PiRemove))),
                        words::secondary(FROM_SELECTION, Some(msg(Event::FromSelection))),
                    ]
                    .spacing(8),
                );
                let equations = &r.equations;
                let eqs = column![
                    road_dialog::grid_table(
                        vec![
                            road_dialog::Col { title: "Geri km", width: None, number: true },
                            road_dialog::Col { title: "İleri km", width: None, number: true },
                            road_dialog::col("Yön", 110.0),
                            road_dialog::col("", 64.0),
                        ],
                        equations.len(),
                        move |j, k| {
                            let q = &equations[j];
                            match k {
                                0 => road_dialog::edit(&q.back, true, move |t| msg(Event::Back(j, t))),
                                1 => road_dialog::edit(&q.ahead, true, move |t| msg(Event::Ahead(j, t))),
                                2 => road_dialog::pick(["Artan", "Azalan"], Some(usize::from(!q.increasing)), move |i| {
                                    msg(Event::Increasing(j, i == 0))
                                }),
                                _ => kentos_ui::widget::Cell::padded(words::ghost("Sil", Some(msg(Event::EquationRemove(j))))).into(),
                            }
                        },
                        "Km eşitliği yok",
                    ),
                    words::secondary("Eşitlik ekle", Some(msg(Event::EquationAdd))),
                ]
                .spacing(6);
                col = col.push(words::field("Km eşitlikleri", eqs, None));
            }
        }
        let point = kentos_ui::widget::text_field("Y; X", &w.point).on_input(|t| msg(Event::Point(t)));
        let said = status(&w.rows, r);
        let bad = said.starts_with("Kurulamıyor") || said.starts_with("Paralel aks kıvrılıyor") || said.ends_with("olmalı.");
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let mut col = col.push(if bad { label::caption(said).style(danger) } else { label::caption(said) });
        // What saving the PIs typed would do (docs/adr/0963).
        if let Some(Ok((imp, warnings))) = &w.impact {
            let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
            let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
            let said = impact_words(imp);
            col = col.push(if warnings.is_empty() { label::caption(said).style(muted) } else { label::caption(said).style(warn) });
            for x in warnings {
                col = col.push(label::caption(x.message.clone()).style(warn));
            }
        }
        col.push(row![words::field("Noktanın km'si", point.width(260), None), label::caption(w.answer.clone())]
                .spacing(12)
                .align_y(iced::alignment::Vertical::Bottom))
            .into()
    }

    pub(crate) fn alignment_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.alignment_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(PARALLEL) => Some(msg(Event::Parallel)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REVERSE) => Some(msg(Event::Reverse)),
            Control::Press(ADD_PI) => Some(msg(Event::PiAdd)),
            Control::Press(REMOVE_PI) => Some(msg(Event::PiRemove)),
            Control::Press(FROM_SELECTION) => Some(msg(Event::FromSelection)),
            Control::Press(LABEL) => Some(msg(Event::Label)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Başlangıç km (m)", text) => Some(msg(Event::Start(text.to_owned()))),
            Control::Fill("Noktanın km'si", text) => Some(msg(Event::Point(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
