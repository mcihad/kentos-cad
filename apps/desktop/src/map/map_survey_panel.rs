//! GNSS arşivi (plan/map-cad.md MAP-03, docs/adr/1340 §6): a map project's arazi ölçümleri in the dock, as
//! `map.survey.query` reads them, drawn by KentOS UI's status tree (Harita işi's pattern): the arazi ölçümleri with
//! their ölçüm oturumları (mark, antenna height, file, the replay of a recorded log), each mark with its solutions
//! (last measured, accepted, in the adjustment, excluded) under it and its accepted point's state, and the objects made
//! from accepted points. One toolbar row of icon buttons (their words in their tips): a new arazi ölçümü, a session
//! from a RINEX, NMEA or GPX file, reading the files' SHA-256, and one text box for the mark a file does not say or the
//! reason a solution is excluded. A row's button excludes or takes back a solution, solves a recorded log, accepts a
//! mark's point or makes it again (`map.survey.edit`). Under the marks, the baselines, the known points and the accepted
//! network adjustments (map_survey_network_rows.rs, docs/adr/2920). Harita only: GIS reads the accepted point in
//! Öznitelikler.

use crate::map;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    AcceptMethod, MapSurveyEdit, MapSurveyOperation as Op, MapSurveyTree, SolutionKind, SurveyFileState as F,
    SurveyMarkRow, SurveyPointRow, SurveySessionRow, SurveySolutionRow, SurveyState as S, SurveyUseRow,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_survey, map_survey_edit};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::widget::{Status, StatusItem, StatusTree};

use crate::app::{App, Message, Panel};

/// Docs/adr/4920: the button that writes the limits typed into the project.
pub(crate) const SAVE_LIMITS: &str = "Projeye yaz";

#[derive(Clone, Debug)]
pub enum Event {
    /// A new arazi ölçümü.
    Campaign,
    /// Ask for a session's file; the file chosen (none: the dialog was closed).
    Import,
    Imported(Option<PathBuf>),
    /// The toolbar's text: a mark, or a reason.
    Text(String),
    /// Docs/adr/4520: the user's limits of the known points' places against their accepted points, as typed.
    LimitHorizontal(String),
    LimitVertical(String),
    /// Docs/adr/4920: the limits' source, and the limits written into the project with it.
    LimitSource(String),
    LimitSave,
    Deep(bool),
    /// A solution left out with the toolbar's reason, or taken back.
    Exclude(String),
    Include(String),
    /// A recorded log's receiver solution (its RTK fixed positions).
    Solve(String),
    /// A session's antenna: the toolbar's number as its height straight up to the ARP (m).
    Antenna(String),
    /// A mark's point accepted, or an accepted point made again.
    Accept(String),
    Regenerate(String),
    /// An accepted point's or a use's row: its objects selected and shown.
    Objects(Vec<String>),
    /// A baseline's row (docs/adr/3320): its ends' accepted points selected, the baseline drawn between them.
    Baseline(String),
    /// Ağı göster (docs/adr/3720): every baseline whose ends have accepted points drawn at once.
    ShowNetwork,
    /// Ağ dengelemesi, from an adjustment's row (docs/adr/2920).
    Network,
    Fold(String),
    Filter(Option<Status>),
}

pub(crate) fn msg(e: Event) -> Message {
    Message::Map(map::Event::MapSurvey(e))
}

type Read = (String, bool, Option<kentos_commands::SurveyPlaceTolerance>, Result<MapSurveyTree, String>);

/// The panel's state: deep or quick, the toolbar's text, the user's folds, the filter, and the last tree.
#[derive(Default)]
pub struct PanelState {
    pub deep: bool,
    pub text: String,
    toggled: HashSet<String>,
    filter: Option<Status>,
    read: RefCell<Option<Read>>,
    /// Docs/adr/4520: the user's limits of a known point's place against its mark's accepted point, as typed (m;
    /// empty: none).
    pub limit_horizontal: String,
    pub limit_vertical: String,
    /// Docs/adr/4920: where the limits come from (a regulation's article, an administration's specification), as typed.
    pub limit_source: String,
}

impl std::fmt::Debug for PanelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelState").field("deep", &self.deep).finish()
    }
}

/// The icons of the archive (catalog/v1/additions.json).
pub(crate) fn glyph(name: &str) -> Icon {
    crate::icons::from_web(Some(name))
}

/// A solution's kind in words.
pub(crate) fn kind_words(k: SolutionKind) -> &'static str {
    match k {
        SolutionKind::Receiver => "Alıcı (RTK)",
        SolutionKind::Processed => "İşlenmiş",
        SolutionKind::Ppp => "PPP",
    }
}

/// A point's or a use's state in words.
pub(crate) fn state_words(s: S) -> &'static str {
    match s {
        S::Current => "Güncel",
        S::Stale => "Güncel değil",
        S::Missing => "Nesnesi yok",
    }
}

/// σ in millimetres: north, east, up.
fn sigma_mm(g: [f64; 3]) -> String {
    format!("σ {:.0}/{:.0}/{:.0} mm", g[0] * 1000.0, g[1] * 1000.0, g[2] * 1000.0)
}

/// A baseline's end on the map (docs/adr/3320, 4120): its mark's accepted point; else the own place in the project of
/// a known point on it that is not left out; none with neither.
fn end_of(tree: &MapSurveyTree, mark: &str) -> Option<crate::survey_baseline_marks::End> {
    use crate::survey_baseline_marks::End;
    if let Some(uid) = crate::map_survey_network_rows::objects_of(tree, &[mark]).into_iter().next() {
        return Some(End::Object(uid));
    }
    tree.known
        .iter()
        .filter(|k| k.mark == mark && k.excluded.is_none())
        .find_map(|k| tree.known_places.iter().find(|p| p.id == k.id).and_then(|p| Some(End::Known([p.x?, p.y?], mark.to_owned()))))
}

impl App {
    /// Docs/adr/4520: the limits typed, each a length above 0 (another text: not a limit); none when neither is.
    pub(crate) fn map_survey_limits(&self) -> Option<kentos_commands::SurveyPlaceTolerance> {
        let read = |t: &str| crate::calc::read::read_number(t).filter(|v| v.is_finite() && *v > 0.0);
        let t = kentos_commands::SurveyPlaceTolerance {
            horizontal: read(&self.map.map_survey.limit_horizontal),
            vertical: read(&self.map.map_survey.limit_vertical),
        };
        (t.horizontal.is_some() || t.vertical.is_some()).then_some(t)
    }

    /// The tree as the drawing is now, read again when its revision, the depth or the limits changed.
    pub(crate) fn map_survey_read(&self) -> Option<Result<MapSurveyTree, String>> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let deep = self.map.map_survey.deep;
        let limits = self.map_survey_limits();
        let mut kept = self.map.map_survey.read.borrow_mut();
        if let Some((r, d, l, answer)) = kept.as_ref()
            && *r == revision
            && *d == deep
            && *l == limits
        {
            return Some(answer.clone());
        }
        // Docs/adr/4120: with the known points' own places, for their rows and the baselines' ends; docs/adr/4520:
        // judged against the limits typed.
        let answer = map_survey::preview_judged(&doc.model, deep, limits.clone());
        *kept = Some((revision, deep, limits, answer.clone()));
        Some(answer)
    }

    /// Docs/adr/4920: the limits typed written into the project with their source (`map.survey.edit tolerance`); none
    /// typed takes the kept ones away.
    pub(crate) fn map_survey_save_limits(&mut self) {
        let source = self.map.map_survey.limit_source.trim().to_owned();
        let Some(limits) = self.map_survey_limits() else {
            let kept = self.document.as_ref().and_then(|d| d.model.map_survey()).is_some_and(|s| s.tolerance.is_some());
            if !kept {
                self.say(Level::Warn, "Yatay ya da düşey kabul farkı sınırını yazın (m), sonra Projeye yaz.");
            } else if self.map_survey_edit(Op::Tolerance { horizontal: None, vertical: None, source: None }).is_some() {
                self.say(Level::Success, "Projedeki kabul farkı sınırı kaldırıldı.");
            }
            return;
        };
        if source.is_empty() {
            self.say(Level::Warn, "Sınırın kaynağını yazın: yönetmeliğin maddesi ya da idarenin şartnamesi.");
            return;
        }
        let op = Op::Tolerance { horizontal: limits.horizontal, vertical: limits.vertical, source: Some(source.clone()) };
        if self.map_survey_edit(op).is_some() {
            self.say(Level::Success, format!("Kabul farkı sınırı projeye yazıldı; kaynağı: {source}."));
        }
    }

    pub(crate) fn map_survey_edit(&mut self, operation: Op) -> Option<kentos_commands::MapSurveyEdited> {
        let doc = self.document.as_mut()?;
        let input = MapSurveyEdit { operation, expected_revision: None };
        let result = map_survey_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.answered(result)
    }

    /// `map.survey`: the panel in front.
    pub(crate) fn show_map_survey(&mut self) {
        if !self.open_panel(Panel::MapSurvey) {
            return;
        }
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        self.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapSurvey));
    }

    /// `map.survey.select`: every accepted point's object selected and shown (Harita and GIS).
    pub(crate) fn select_accepted_points(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let uids: Vec<String> = match map_survey::preview(&doc.model, false, None, false) {
            Ok(tree) => tree.points.iter().map(|p| p.uid.clone()).collect(),
            Err(why) => {
                self.say(Level::Warn, why);
                return;
            }
        };
        if uids.is_empty() {
            self.say(Level::Info, "Projede kabul edilen GNSS noktası yok.");
            return;
        }
        self.select_uids(&uids);
    }

    /// A baseline's row (docs/adr/3320): its two ends' accepted points selected and shown, and the baseline drawn
    /// between them on the map with its id and length, until İşareti kaldır. An end without an accepted point is its
    /// known point's own place in the project (docs/adr/4120), said; a mark with neither says so.
    fn show_baseline(&mut self, id: &str) {
        use crate::survey_baseline_marks::End;
        let Some(Ok(tree)) = self.map_survey_read() else { return };
        let Some(b) = tree.baselines.iter().find(|b| b.id == id).cloned() else { return };
        let (from, to) = (end_of(&tree, &b.from), end_of(&tree, &b.to));
        let uids: Vec<String> = [&from, &to]
            .into_iter()
            .filter_map(|e| match e {
                Some(End::Object(uid)) => Some(uid.clone()),
                _ => None,
            })
            .collect();
        self.select_uids(&uids);
        match (from, to, self.document.as_ref().map(|d| d.session)) {
            (Some(from), Some(to), Some(session)) => {
                let own: Vec<&str> = [(&from, b.from.as_str()), (&to, b.to.as_str())]
                    .into_iter()
                    .filter(|(e, _)| matches!(e, End::Known(..)))
                    .map(|(_, m)| m)
                    .collect();
                let words = format!("{} · {:.3} m", b.id, b.length);
                let line = crate::survey_baseline_marks::ShownLine {
                    from,
                    to,
                    words,
                    excluded: b.excluded.is_some(),
                };
                self.map.baseline_shown = Some(crate::survey_baseline_marks::Shown {
                    session,
                    lines: vec![line],
                });
                if !own.is_empty() {
                    self.fit_baselines();
                    self.say(
                        kentos_interaction::Level::Info,
                        format!(
                            "{}: {} ucu kabul noktası olmadığından bilinen noktanın projedeki kendi yerinden çizildi (üçgen).",
                            b.id,
                            own.join(" ve ")
                        ),
                    );
                }
            }
            _ => {
                self.map.baseline_shown = None;
                self.say(
                    kentos_interaction::Level::Info,
                    format!(
                        "{} çizilemedi: {} → {} uçlarından birinin ne kabul noktası ne de projede yeri olan bilinen noktası var.",
                        b.id, b.from, b.to
                    ),
                );
            }
        }
    }

    /// Ağı göster (docs/adr/3720): every baseline whose two ends can be placed (an accepted point, else a known point's
    /// own place, docs/adr/4120) drawn on the map at once, a left out one in its own colour; the accepted ends
    /// selected and shown; the ends at known points' own places named, those that cannot be drawn counted.
    fn show_network(&mut self) {
        use crate::survey_baseline_marks::End;
        let Some(Ok(tree)) = self.map_survey_read() else { return };
        let Some(session) = self.document.as_ref().map(|d| d.session) else { return };
        let mut lines = Vec::new();
        let mut uids: Vec<String> = Vec::new();
        let mut own: Vec<String> = Vec::new();
        let mut missing = 0usize;
        for b in &tree.baselines {
            match (end_of(&tree, &b.from), end_of(&tree, &b.to)) {
                (Some(from), Some(to)) => {
                    for (end, mark) in [(&from, &b.from), (&to, &b.to)] {
                        match end {
                            End::Object(u) if !uids.contains(u) => uids.push(u.clone()),
                            End::Known(..) if !own.contains(mark) => own.push(mark.clone()),
                            _ => {}
                        }
                    }
                    let words = format!("{} · {:.3} m", b.id, b.length);
                    lines.push(crate::survey_baseline_marks::ShownLine { from, to, words, excluded: b.excluded.is_some() });
                }
                _ => missing += 1,
            }
        }
        let drawn = lines.len();
        let out = lines.iter().filter(|l| l.excluded).count();
        self.select_uids(&uids);
        self.map.baseline_shown =
            (drawn > 0).then_some(crate::survey_baseline_marks::Shown { session, lines });
        if !own.is_empty() {
            self.fit_baselines();
        }
        let mut said = match (drawn, out) {
            (0, _) => "Ağ çizilemedi: hiçbir bazın iki ucu da yerleştirilemiyor (kabul noktası ya da projede yeri olan bilinen nokta yok).".to_owned(),
            (n, 0) => format!("Ağ: {n} baz çizildi."),
            (n, k) => format!("Ağ: {n} baz çizildi; {k}'i dışlandı, kırmızı noktalı."),
        };
        if drawn > 0 && !own.is_empty() {
            said.push_str(&format!(" {} bilinen noktanın projedeki kendi yerinde (üçgen).", own.join(", ")));
        }
        if missing > 0 && drawn > 0 {
            said.push_str(&format!(
                " {missing} baz çizilemedi: uçlarından birinin ne kabul noktası ne de projede yeri olan bilinen noktası var."
            ));
        }
        self.say(kentos_interaction::Level::Info, said);
    }

    /// The view on every drawn baseline's two ends, 96 px in from the edges (docs/adr/4120: a known point's own place
    /// is no object of the drawing, so the selection's box would leave it out).
    fn fit_baselines(&mut self) {
        let Some(marks) = self.baseline_marks() else { return };
        let ends = marks.lines.iter().flat_map(|l| [l.from, l.to]);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for [x, y] in ends {
            (min_x, min_y, max_x, max_y) = (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y));
        }
        let bounds = kentos_render_wgpu::Bounds { min_x, min_y, max_x, max_y };
        self.navigating(|app| app.viewport.change(kentos_interaction::ViewChange::Fit { bounds, padding: 96.0 }));
    }

    fn select_uids(&mut self, uids: &[String]) {
        let Some(doc) = self.document.as_ref() else { return };
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

    /// The arazi ölçümü a new session (or a baseline file, docs/adr/1740) goes to: the last one, made when there is none.
    pub(crate) fn survey_campaign(&mut self) -> Option<String> {
        let last = self
            .document
            .as_ref()
            .and_then(|d| d.model.map_survey())
            .and_then(|s| s.campaigns.last())
            .map(|c| c.id.clone());
        if last.is_some() {
            return last;
        }
        self.add_survey_campaign()
    }

    fn add_survey_campaign(&mut self) -> Option<String> {
        let n = self.document.as_ref().and_then(|d| d.model.map_survey()).map_or(0, |s| s.campaigns.len()) + 1;
        let name = format!("Arazi ölçümü {n}");
        let out = self.map_survey_edit(Op::Campaign { name: name.clone(), work: None, operator: None, epoch: None, velocity: None })?;
        self.say(Level::Success, format!("“{name}” eklendi ({}).", out.id.clone().unwrap_or_default()));
        out.id
    }

    fn import_session(&mut self, path: PathBuf) {
        let Some(campaign) = self.survey_campaign() else { return };
        let mark = Some(self.map.map_survey.text.trim().to_owned()).filter(|m| !m.is_empty());
        let operation = Op::Session {
            campaign,
            mark,
            file: Some(path.to_string_lossy().into_owned()),
            start: None,
            end: None,
            time_system: None,
            receiver: None,
            antenna: None,
            operator: None,
        };
        if let Some(out) = self.map_survey_edit(operation) {
            let words = match out.observations {
                0 => format!("Ölçüm oturumu {} eklendi.", out.id.unwrap_or_default()),
                n => format!("Ölçüm oturumu {} eklendi: {n} ham ölçüm.", out.id.unwrap_or_default()),
            };
            self.say(Level::Success, words);
        }
    }

    /// A mark's point accepted from its solutions not excluded that share the last measured one's frame and epoch:
    /// their mean, or the one alone.
    fn accept_mark(&mut self, mark: &str) {
        let Some(Ok(tree)) = self.map_survey_read() else { return };
        let Some(m) = tree.marks.iter().find(|m| m.mark == mark) else { return };
        let open: Vec<&SurveySolutionRow> = m.solutions.iter().filter(|s| s.excluded.is_none()).collect();
        let Some(lead) = open.iter().find(|s| s.latest).or(open.last()).copied() else {
            self.say(Level::Warn, format!("“{mark}” işaretinin dışlanmamış çözümü yok."));
            return;
        };
        let ids: Vec<String> = open
            .iter()
            .filter(|s| s.srid == lead.srid && s.epoch == lead.epoch)
            .map(|s| s.id.clone())
            .collect();
        let method = if ids.len() == 1 { AcceptMethod::Single } else { AcceptMethod::Mean };
        if let Some(out) = self.map_survey_edit(Op::Accept { mark: mark.to_owned(), solutions: ids, method: Some(method), layer: None }) {
            self.say(Level::Success, format!("“{mark}” kabul edildi ({}, {}).", out.id.unwrap_or_default(), method.title()));
        }
    }

    pub(crate) fn map_survey_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Campaign => {
                self.add_survey_campaign();
            }
            Event::Import => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Ölçüm oturumunun dosyası (RINEX, NMEA, GPX)")
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| msg(Event::Imported(path)),
                );
            }
            Event::Imported(Some(path)) => self.import_session(path),
            Event::Imported(None) => {}
            Event::Text(t) => self.map.map_survey.text = t,
            Event::LimitHorizontal(t) => self.map.map_survey.limit_horizontal = t,
            Event::LimitVertical(t) => self.map.map_survey.limit_vertical = t,
            Event::LimitSource(t) => self.map.map_survey.limit_source = t,
            Event::LimitSave => self.map_survey_save_limits(),
            Event::Deep(on) => self.map.map_survey.deep = on,
            Event::Exclude(id) => {
                let reason = self.map.map_survey.text.trim().to_owned();
                if reason.is_empty() {
                    self.say(Level::Warn, "Dışlama nedenini araç çubuğundaki kutuya yazın; çözüm silinmez, nedeniyle dışlanır.");
                } else if self.map_survey_edit(Op::Exclude { solution: id.clone(), reason }).is_some() {
                    self.map.map_survey.text.clear();
                    self.say(Level::Info, format!("“{id}” dışlandı; kayıtta kalır."));
                }
            }
            Event::Include(id) => {
                if self.map_survey_edit(Op::Include { solution: id.clone() }).is_some() {
                    self.say(Level::Info, format!("“{id}” yeniden kullanılabilir."));
                }
            }
            Event::Solve(session) => {
                // NMEA 0183's positions are WGS 84 and GGA's height the antenna's: the receiver took no antenna off.
                let op = Op::Solve {
                    session: session.clone(),
                    srid: 4326,
                    antenna_applied: 0.0,
                    epoch: None,
                    from: None,
                    to: None,
                    fixes: vec!["RTK sabit".into()],
                };
                if let Some(out) = self.map_survey_edit(op) {
                    self.say(Level::Success, format!("Alıcı çözümü {}: “{session}” oturumunun RTK sabit ölçümleri.", out.id.unwrap_or_default()));
                }
            }
            Event::Antenna(session) => {
                let text = self.map.map_survey.text.trim().to_owned();
                let Some(height) = text.parse::<f64>().ok().filter(|h| h.is_finite()) else {
                    self.say(Level::Warn, "Anten yüksekliğini metre olarak araç çubuğundaki kutuya yazın (1.800 gibi; ARP'ye düşey).");
                    return Task::none();
                };
                let antenna = kentos_commands::SurveyAntenna {
                    kind: None,
                    serial: None,
                    height,
                    reference: kentos_commands::AntennaReference::Arp,
                    radius: None,
                    offset: None,
                    east: None,
                    north: None,
                };
                let op = Op::SetSession {
                    session: session.clone(),
                    mark: None,
                    start: None,
                    end: None,
                    time_system: None,
                    receiver: None,
                    antenna: Some(antenna),
                    operator: None,
                };
                if self.map_survey_edit(op).is_some() {
                    self.map.map_survey.text.clear();
                    self.say(Level::Info, format!("“{session}” oturumunun anten yüksekliği {height:.3} m (ARP'ye düşey)."));
                }
            }
            Event::Accept(mark) => self.accept_mark(&mark),
            Event::Regenerate(point) => {
                if let Some(out) = self.map_survey_edit(Op::Regenerate { point: point.clone(), solutions: None, method: None }) {
                    self.say(
                        Level::Success,
                        format!("“{point}” yeniden üretildi: {}. revizyon; eski sonuç geçmişte.", out.point_revision.unwrap_or(0)),
                    );
                }
            }
            Event::Objects(uids) => self.select_uids(&uids),
            Event::Baseline(id) => self.show_baseline(&id),
            Event::ShowNetwork => self.show_network(),
            Event::Network => return self.open_survey_network(),
            Event::Fold(key) => {
                if !self.map.map_survey.toggled.remove(&key) {
                    self.map.map_survey.toggled.insert(key);
                }
            }
            Event::Filter(f) => self.map.map_survey.filter = f,
        }
        Task::none()
    }

    pub(crate) fn map_survey_view(&self) -> Element<'_, Message> {
        let Some(read) = self.map_survey_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let tree = match read {
            Ok(t) => t,
            Err(why) => return container(label::caption(why)).padding([8, 12]).into(),
        };
        use crate::style::strip::{icon_button, icon_toggle};
        let bar = row![
            icon_button(glyph("gnssCampaign"), "Arazi ölçümü ekle", Some(msg(Event::Campaign))),
            icon_button(
                glyph("gnssSession"),
                "Ölçüm oturumu ekle: RINEX gözlem dosyası (başlığından işaret, anten, zaman), NMEA ya da GPX (ham ölçümler). İşareti dosya söylemiyorsa kutudaki ad.",
                Some(msg(Event::Import))
            ),
            icon_toggle(
                glyph("gnssFiles"),
                "Dosyaları oku: oturum dosyalarının SHA-256'sını kayıtla karşılaştır",
                self.map.map_survey.deep,
                msg(Event::Deep(!self.map.map_survey.deep))
            ),
            crate::style::fields::input(None, "İşaret ya da dışlama nedeni", &self.map.map_survey.text, false, false)
                .on_input(|t| msg(Event::Text(t))),
        ]
        .spacing(4)
        .align_y(Center);
        // Docs/adr/4520: the user's limits of the known points' places against their accepted points (a horizontal and
        // a vertical tolerance), shown with known points; docs/adr/4920: written into the project with their source,
        // the kept ones read when none is typed.
        let head: Element<'_, Message> = if tree.known.is_empty() {
            container(bar).padding([4, 12]).width(Fill).into()
        } else {
            let bad = |t: &str| !t.trim().is_empty() && crate::calc::read::read_number(t).is_none_or(|v| !(v.is_finite() && v > 0.0));
            let limits = row![
                label::caption("Kabul farkı sınırı (m)"),
                container(
                    crate::style::fields::input(
                        None,
                        "yatay",
                        &self.map.map_survey.limit_horizontal,
                        bad(&self.map.map_survey.limit_horizontal),
                        false
                    )
                    .on_input(|t| msg(Event::LimitHorizontal(t)))
                )
                .width(72),
                container(
                    crate::style::fields::input(
                        None,
                        "düşey",
                        &self.map.map_survey.limit_vertical,
                        bad(&self.map.map_survey.limit_vertical),
                        false
                    )
                    .on_input(|t| msg(Event::LimitVertical(t)))
                )
                .width(72),
            ]
            .spacing(6)
            .align_y(Center);
            let kept = tree.tolerance.as_ref().map_or_else(
                || "Projede sınır yok.".to_owned(),
                |t| {
                    let m = |v: Option<f64>| v.map_or("—".to_owned(), |v| format!("{v}").replace('.', ","));
                    format!("Projede: yatay {}, düşey {} m · {}", m(t.horizontal), m(t.vertical), t.source)
                },
            );
            let source = row![
                crate::style::fields::input(
                    None,
                    "Kaynak: yönetmelik maddesi, idare şartnamesi",
                    &self.map.map_survey.limit_source,
                    false,
                    false
                )
                .on_input(|t| msg(Event::LimitSource(t))),
                crate::exchange::words::secondary(SAVE_LIMITS, Some(msg(Event::LimitSave))),
            ]
            .spacing(6)
            .align_y(Center);
            container(iced::widget::column![bar, limits, source, label::caption(kept)].spacing(4)).padding([4, 12]).width(Fill).into()
        };
        let mut items = Vec::new();
        if !tree.campaigns.is_empty() {
            let mut group = StatusItem::new("campaigns", "Arazi ölçümleri")
                .icon(icon(glyph("gnssCampaign")).size(14.0))
                .meta(format!("{}", tree.campaigns.len()));
            for c in &tree.campaigns {
                let mut item = StatusItem::new(format!("kp:{}", c.id), c.name.clone())
                    .meta(format!("{} oturum", c.sessions))
                    .detail(campaign_detail(c));
                if c.work_missing {
                    item = item.status(Status::Warning, "Harita işi öğesi yok");
                }
                for s in tree.sessions.iter().filter(|s| s.campaign == c.id) {
                    item = item.push(session_item(s));
                }
                group = group.push(item);
            }
            items.push(group);
        }
        if !tree.marks.is_empty() {
            let mut group = StatusItem::new("marks", "İşaretler")
                .icon(icon(glyph("gnssAccepted")).size(14.0))
                .meta(format!("{}", tree.marks.len()));
            for m in &tree.marks {
                group = group.push(mark_item(m, tree.points.iter().find(|p| Some(&p.id) == m.point.as_ref())));
            }
            items.push(group);
        }
        // The baselines, the known points and the adjustments (docs/adr/2920).
        items.extend(crate::map_survey_network_rows::groups(&tree));
        if !tree.uses.is_empty() {
            let mut group = StatusItem::new("uses", "Dayananlar")
                .icon(icon(Icon::Layers).size(14.0))
                .meta(format!("{}", tree.uses.len()));
            for u in &tree.uses {
                group = group.push(use_item(u));
            }
            items.push(group);
        }
        let body = StatusTree::new(items)
            .toggled(&self.map.map_survey.toggled, |key| msg(Event::Fold(key)))
            .filter(self.map.map_survey.filter, |f| msg(Event::Filter(f)))
            .counted("Dikkat", "Sorun")
            .empty("Arşivde arazi ölçümü yok: araç çubuğundan ekleyin ya da bir ölçüm oturumu dosyası seçin.");
        column![head, kentos_ui::widget::horizontal_divider(), body.view()].spacing(4).padding([6, 0]).into()
    }
}

fn campaign_detail(c: &kentos_commands::SurveyCampaignRow) -> String {
    let mut out = vec![format!("Kimlik {}.", c.id)];
    match c.epoch {
        Some(e) => out.push(format!("Epok kararı {e:.4}.")),
        None => out.push("Epok kararı projeninki.".into()),
    }
    if let Some(v) = &c.velocity {
        out.push(format!("Hız ızgarası: {v}."));
    }
    if let Some(w) = &c.work {
        out.push(format!("Harita işi öğesi {w}{}.", if c.work_missing { " (projede yok)" } else { "" }));
    }
    out.join(" ")
}

/// A session's row: its mark and id, its antenna's vertical height and its file faint, the file's state as a pill, the
/// receiver, times and replay in the tip; a recorded log without a solution can be solved.
fn session_item<'a>(s: &SurveySessionRow) -> StatusItem<'a, Message> {
    let format = s.file.as_ref().map_or("elle", |f| match f.format {
        kentos_commands::SurveyFileFormat::Rinex => "RINEX",
        kentos_commands::SurveyFileFormat::Nmea => "NMEA",
        kentos_commands::SurveyFileFormat::Gpx => "GPX",
        kentos_commands::SurveyFileFormat::Report => "rapor",
        kentos_commands::SurveyFileFormat::Gvx => "GVX",
        kentos_commands::SurveyFileFormat::Text => "baz metni",
    });
    let height = s.vertical.map_or("anten yok".to_owned(), |h| format!("{h:.3} m"));
    let mut detail = Vec::new();
    if let (Some(a), Some(b)) = (&s.start, &s.end) {
        detail.push(format!("{a} – {b}."));
    }
    if let Some(r) = &s.receiver {
        detail.push(format!("Alıcı {}.", r.kind));
    }
    if let Some(a) = &s.antenna {
        detail.push(format!(
            "Anten {}: {:.3} m {}.",
            a.kind.as_deref().unwrap_or("türü yok"),
            a.height,
            if a.reference == kentos_commands::AntennaReference::Slant { "eğik" } else { "ARP'ye düşey" }
        ));
    }
    if s.observations > 0 {
        detail.push(format!("{} ham ölçüm.", s.observations));
    }
    if let Some(r) = &s.replay {
        let fixes: Vec<String> = r.fixes.iter().map(|f| format!("{} {}", f.fix, f.count)).collect();
        detail.push(format!(
            "Oynatma: {}; RTK sabitten {} düşüş, {} zaman atlaması{}.",
            fixes.join(", "),
            r.float_drops,
            r.time_jumps,
            r.nominal.map_or(String::new(), |n| format!(" (olağan adım {n} sn)"))
        ));
    }
    if let Some(f) = &s.file {
        detail.push(format!("Dosya: {} (revizyon {}).", f.path, &f.revision[..12.min(f.revision.len())]));
    }
    let (tone, words) = match s.file.as_ref().map(|f| f.state) {
        Some(F::Missing) => (Status::Danger, "Dosyası yok"),
        Some(F::Changed) => (Status::Warning, "Dosyası değişti"),
        _ if s.replay.as_ref().is_some_and(|r| r.float_drops > 0 || r.time_jumps > 0) => (Status::Warning, "Kopma var"),
        _ => (Status::Ok, format),
    };
    let mut item = StatusItem::new(format!("os:{}", s.id), format!("{} · {}", s.mark, s.id))
        .icon(icon(glyph("gnssSession")).size(13.0))
        .meta(height)
        .status(tone, words)
        .detail(detail.join(" "));
    if s.antenna.is_none() {
        item = item.action(
            icon(glyph("gnssAntenna")).size(13.0),
            "Anten yüksekliği: kutudaki değer (m, ARP'ye düşey); işaret yüksekliği ve kabul onu ister",
            msg(Event::Antenna(s.id.clone())),
        );
    } else if s.observations > 0 && s.solutions == 0 {
        item = item.action(
            icon(glyph("gnssSolve")).size(13.0),
            "Alıcı çözümü: RTK sabit ölçümlerin ortalaması (NMEA'da WGS 84, anten çıkarılmamış)",
            msg(Event::Solve(s.id.clone())),
        );
    }
    item
}

/// A mark's row: its accepted point's state as a pill (or none yet) and the button that accepts or makes it again;
/// its solutions under it.
fn mark_item<'a>(m: &SurveyMarkRow, point: Option<&SurveyPointRow>) -> StatusItem<'a, Message> {
    let meta = format!("{} oturum · {} çözüm", m.sessions.len(), m.solutions.len());
    let mut item = StatusItem::new(format!("mark:{}", m.mark), m.mark.clone()).meta(meta);
    item = match point {
        None => item.status(Status::Neutral, "Kabul yok").action(
            icon(glyph("gnssAccepted")).size(13.0),
            "Kabul et: dışlanmamış, aynı çerçevedeki çözümlerin ortalaması",
            msg(Event::Accept(m.mark.clone())),
        ),
        Some(p) => {
            let detail = point_detail(p);
            let tone = match p.state {
                S::Current => Status::Info,
                S::Stale => Status::Warning,
                S::Missing => Status::Danger,
            };
            let mut it = item
                .status(tone, format!("{} · {}", p.id, state_words(p.state)))
                .detail(detail)
                .on_press(msg(Event::Objects(vec![p.uid.clone()])));
            if p.state == S::Stale {
                it = it.action(
                    icon(glyph("gnssRegenerate")).size(13.0),
                    "Yeniden üret: bugünkü anten ve epok kararıyla; eski sonuç geçmişte kalır",
                    msg(Event::Regenerate(p.id.clone())),
                );
            }
            it
        }
    };
    for s in &m.solutions {
        item = item.push(solution_item(s));
    }
    item
}

fn point_detail(p: &SurveyPointRow) -> String {
    let r = &p.result;
    let mut out = p.why.clone();
    out.push(format!(
        "{}. revizyon, {} ({}); işaret yüksekliği {:.3} m; {}.",
        r.revision,
        r.method.title(),
        r.solutions.join(", "),
        r.height,
        if r.chain == "complete" { "zincir tam" } else { "zincir sınırlı: hassas dönüşüm sayılmaz" }
    ));
    if let Some(g) = r.sigma {
        out.push(format!("{}.", sigma_mm(g)));
    }
    if p.history > 0 {
        out.push(format!("{} eski sonuç geçmişte.", p.history));
    }
    out.join(" ")
}

/// A solution's row: its kind and id, the mark's height faint, its roles as a pill; the button excludes it with the
/// toolbar's reason, or takes it back.
fn solution_item<'a>(s: &SurveySolutionRow) -> StatusItem<'a, Message> {
    let mut roles = Vec::new();
    if s.latest {
        roles.push("son ölçülen");
    }
    if s.accepted {
        roles.push("kabul edilen");
    }
    if s.adjusted {
        roles.push("dengelemede");
    }
    // The roles in one pill (an accepted, last measured solution says both); a quiet one says none.
    let (tone, words) = if s.excluded.is_some() {
        (Status::Neutral, "Dışlandı".to_owned())
    } else if s.accepted && s.latest {
        (Status::Info, "Kabul · son ölçülen".to_owned())
    } else if s.accepted {
        (Status::Info, "Kabul edilen".to_owned())
    } else if s.latest {
        (Status::Neutral, "Son ölçülen".to_owned())
    } else if s.adjusted {
        (Status::Neutral, "Dengelemede".to_owned())
    } else {
        (Status::Ok, String::new())
    };
    let mut detail = Vec::new();
    if !roles.is_empty() {
        detail.push(format!("Rolü: {}.", roles.join(", ")));
    }
    if let Some(why) = &s.excluded {
        detail.push(format!("Dışlama nedeni: {why}. Kayıtta kalır."));
    }
    detail.push(format!("Çerçeve EPSG:{}{}.", s.srid, s.epoch.map_or(String::new(), |e| format!(", epok {e:.2}"))));
    if let Some(f) = &s.fix {
        detail.push(format!("Fix {f} (doğruluk garantisi değildir)."));
    }
    if let Some(n) = s.epochs {
        detail.push(format!("{n} epok."));
    }
    let meta = match (s.mark_height, s.sigma) {
        (Some(h), Some(g)) => format!("h {h:.3} · {}", sigma_mm(g)),
        (Some(h), None) => format!("h {h:.3}"),
        _ => "anten yok".into(),
    };
    let item = StatusItem::new(format!("cz:{}", s.id), format!("{} · {}", kind_words(s.kind), s.id))
        .meta(meta)
        .status(tone, words)
        .detail(detail.join(" "));
    if s.excluded.is_some() {
        item.action(icon(Icon::Undo).size(13.0), "Dışlamayı kaldır", msg(Event::Include(s.id.clone())))
    } else {
        item.action(
            icon(glyph("gnssExclude")).size(13.0),
            "Dışla: kutudaki nedenle; çözüm silinmez",
            msg(Event::Exclude(s.id.clone())),
        )
    }
}

fn use_item<'a>(u: &SurveyUseRow) -> StatusItem<'a, Message> {
    let tone = match u.state {
        S::Current => Status::Ok,
        S::Stale | S::Missing => Status::Warning,
    };
    let meta = if u.objects == u.recorded { format!("{} nesne", u.objects) } else { format!("{}/{} nesne", u.objects, u.recorded) };
    StatusItem::new(format!("kb:{}", u.id), u.operation.clone())
        .meta(meta)
        .status(tone, state_words(u.state))
        .detail(u.why.join(" "))
}
