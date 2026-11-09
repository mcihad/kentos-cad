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

#[derive(Clone, Debug)]
pub enum Event {
    /// A new arazi ölçümü.
    Campaign,
    /// Ask for a session's file; the file chosen (none: the dialog was closed).
    Import,
    Imported(Option<PathBuf>),
    /// The toolbar's text: a mark, or a reason.
    Text(String),
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
    Message::MapSurvey(e)
}

type Read = (String, bool, Result<MapSurveyTree, String>);

/// The panel's state: deep or quick, the toolbar's text, the user's folds, the filter, and the last tree.
#[derive(Default)]
pub struct PanelState {
    pub deep: bool,
    pub text: String,
    toggled: HashSet<String>,
    filter: Option<Status>,
    read: RefCell<Option<Read>>,
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

impl App {
    /// The tree as the drawing is now, read again when its revision or the depth changed.
    pub(crate) fn map_survey_read(&self) -> Option<Result<MapSurveyTree, String>> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let deep = self.map_survey.deep;
        let mut kept = self.map_survey.read.borrow_mut();
        if let Some((r, d, answer)) = kept.as_ref()
            && *r == revision
            && *d == deep
        {
            return Some(answer.clone());
        }
        let answer = map_survey::preview(&doc.model, deep, None);
        *kept = Some((revision, deep, answer.clone()));
        Some(answer)
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
        let uids: Vec<String> = match map_survey::preview(&doc.model, false, None) {
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
    /// between them on the map with its id and length, until İşareti kaldır; a mark without an accepted point says so.
    fn show_baseline(&mut self, id: &str) {
        let Some(Ok(tree)) = self.map_survey_read() else { return };
        let Some(b) = tree.baselines.iter().find(|b| b.id == id).cloned() else { return };
        let ends = |mark: &str| crate::map_survey_network_rows::objects_of(&tree, &[mark]).into_iter().next();
        let (from, to) = (ends(&b.from), ends(&b.to));
        let uids: Vec<String> = from.iter().chain(to.iter()).cloned().collect();
        self.select_uids(&uids);
        match (from, to, self.document.as_ref().map(|d| d.session)) {
            (Some(from), Some(to), Some(session)) => {
                let words = format!("{} · {:.3} m", b.id, b.length);
                let line = crate::survey_baseline_marks::ShownLine { from, to, words, excluded: b.excluded.is_some() };
                self.baseline_shown = Some(crate::survey_baseline_marks::Shown { session, lines: vec![line] });
            }
            _ => {
                self.baseline_shown = None;
                self.say(
                    kentos_interaction::Level::Info,
                    format!("{} çizilemedi: {} → {} uçlarından birinin kabul noktası yok.", b.id, b.from, b.to),
                );
            }
        }
    }

    /// Ağı göster (docs/adr/3720): every baseline whose two ends have accepted points drawn on the map at once, a left
    /// out one in its own colour; their ends selected and shown; those that cannot be drawn counted.
    fn show_network(&mut self) {
        let Some(Ok(tree)) = self.map_survey_read() else { return };
        let Some(session) = self.document.as_ref().map(|d| d.session) else { return };
        let ends = |mark: &str| crate::map_survey_network_rows::objects_of(&tree, &[mark]).into_iter().next();
        let mut lines = Vec::new();
        let mut uids: Vec<String> = Vec::new();
        let mut missing = 0usize;
        for b in &tree.baselines {
            match (ends(&b.from), ends(&b.to)) {
                (Some(from), Some(to)) => {
                    for u in [&from, &to] {
                        if !uids.contains(u) {
                            uids.push(u.clone());
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
        self.baseline_shown = (drawn > 0).then_some(crate::survey_baseline_marks::Shown { session, lines });
        let mut said = match (drawn, out) {
            (0, _) => "Ağ çizilemedi: hiçbir bazın iki ucunun da kabul noktası yok.".to_owned(),
            (n, 0) => format!("Ağ: {n} baz çizildi."),
            (n, k) => format!("Ağ: {n} baz çizildi; {k}'i dışlandı, kırmızı noktalı."),
        };
        if missing > 0 && drawn > 0 {
            said.push_str(&format!(" {missing} baz çizilemedi: uçlarından birinin kabul noktası yok."));
        }
        self.say(kentos_interaction::Level::Info, said);
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
        let mark = Some(self.map_survey.text.trim().to_owned()).filter(|m| !m.is_empty());
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
            Event::Text(t) => self.map_survey.text = t,
            Event::Deep(on) => self.map_survey.deep = on,
            Event::Exclude(id) => {
                let reason = self.map_survey.text.trim().to_owned();
                if reason.is_empty() {
                    self.say(Level::Warn, "Dışlama nedenini araç çubuğundaki kutuya yazın; çözüm silinmez, nedeniyle dışlanır.");
                } else if self.map_survey_edit(Op::Exclude { solution: id.clone(), reason }).is_some() {
                    self.map_survey.text.clear();
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
                let text = self.map_survey.text.trim().to_owned();
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
                    self.map_survey.text.clear();
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
                if !self.map_survey.toggled.remove(&key) {
                    self.map_survey.toggled.insert(key);
                }
            }
            Event::Filter(f) => self.map_survey.filter = f,
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
                self.map_survey.deep,
                msg(Event::Deep(!self.map_survey.deep))
            ),
            crate::style::fields::input(None, "İşaret ya da dışlama nedeni", &self.map_survey.text, false, false)
                .on_input(|t| msg(Event::Text(t))),
        ]
        .spacing(4)
        .align_y(Center);
        let head = container(bar).padding([4, 12]).width(Fill);
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
            .toggled(&self.map_survey.toggled, |key| msg(Event::Fold(key)))
            .filter(self.map_survey.filter, |f| msg(Event::Filter(f)))
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
