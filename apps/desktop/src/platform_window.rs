//! Platform (plan/road.md Y14.3, docs/adr/0912 §3, `platform.editor`): a road project's graded pads. A pad is a plane
//! inside a closed boundary (its corners typed, or taken from the selected closed polyline or area), through a base
//! point and rising a grade towards a direction; its cut and fill slopes, the spacing of slope lines along an edge, a
//! corner's fan and the longest line are the project's: KentOS fills in none. The pad is graded as it is typed
//! (`road.platforms.edit`'s plan on a reading copy): in plan its boundary, its slope lines (cut, fill, open) and the
//! daylight line; its areas and cut and fill against the terrain. Dengele finds the base height that balances cut and
//! fill in the bracket typed (`road.platforms.query`'s balance, on the pad as typed); Uygula puts it in the base's
//! height, nothing written. Kaydet writes the pad (one undo step “Platform”), Planda çiz draws it on a layer (one step
//! “Platform çiz”).
//!
//! ```text
//! ┌ Platform ─────────────────────────────────────────────────────────────────────────────────────┐
//! │ Platform [▾] Ad [ ] Yüzey [▾] Köşeler [Y,X; Y,X; …                           ] [Seçimden al]    │
//! │ Taban Y [ ] Taban X [ ] Taban kotu [ ] Eğim [ ] Doğrultu [ ] Yarma şevi [ ] Dolgu şevi [ ] …    │
//! │ alan … m² · taban … m² · kazı … m³ · dolgu … m³ · warnings                                       │
//! │ ┌ the pad in plan ─────────────┐  Dengele: En alçak [ ] En yüksek [ ] Tolerans [ ] [Dengele]     │
//! │ └──────────────────────────────┘  Kot … m · fark … m³ · n adım [Uygula]                          │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]        │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, Entity, PlatformBalanceAsk, PlatformBalanced, PlatformReport, PlatformSide, PlatformsDraw, PlatformsEdit,
    PlatformsQuery, RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_platforms_draw, road_platforms_edit, road_platforms_query};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Platform";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
const BALANCE: &str = "Dengele";
const APPLY: &str = "Uygula";
const FROM_SELECTION: &str = "Seçimden al";
/// The typed fields, in order: the pad's, then the balance's bracket.
pub(crate) const FIELDS: [&str; 15] = [
    "Ad",
    "Köşeler (Y,X; …)",
    "Taban Y (m)",
    "Taban X (m)",
    "Taban kotu (m)",
    "Eğim (%)",
    "Doğrultu (°)",
    "Yarma şevi (y/d)",
    "Dolgu şevi (y/d)",
    "Aralık (m)",
    "Yelpaze (°)",
    "Erim (m)",
    "En alçak (m)",
    "En yüksek (m)",
    "Tolerans (m³)",
];
const BRACKET_AT: usize = 12;

/// The pad as last graded, and the balance last found.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<PlatformReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    pub(crate) balance: Option<Result<PlatformBalanced, String>>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) chosen: Option<String>,
    pub(crate) surface: Option<String>,
    pub(crate) fields: [String; 15],
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    Surface(String),
    Field(usize, String),
    FromSelection,
    Layer(String),
    Balance,
    Apply,
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Platforms(event)
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

/// Corners as typed, Y,X pairs (east, north) apart by `;` or a new line.
pub(crate) fn corners_of(text: &str) -> Result<Vec<[f64; 2]>, String> {
    text.split([';', '\n'])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|pair| {
            let mut it = pair.split(',').map(str::trim);
            match (it.next(), it.next(), it.next()) {
                (Some(y), Some(x), None) => match (y.parse::<f64>(), x.parse::<f64>()) {
                    (Ok(e), Ok(n)) if e.is_finite() && n.is_finite() => Ok([e, n]),
                    _ => Err(format!("Köşeler: “{pair}” iki sayı değil (Y,X).")),
                },
                _ => Err(format!("Köşeler: “{pair}” Y,X biçiminde değil.")),
            }
        })
        .collect()
}

pub(crate) fn corners_text(corners: &[[f64; 2]]) -> String {
    corners.iter().map(|c| format!("{},{}", trimmed(c[0]), trimmed(c[1]))).collect::<Vec<_>>().join("; ")
}

/// A closed boundary's corners: an area's ring, or a closed polyline's points (its arcs as their chords' ends).
fn boundary_of(e: &Entity) -> Option<Vec<[f64; 2]>> {
    match e {
        Entity::Polygon(p) => Some(p.pts.iter().map(|v| [v.x, v.y]).collect()),
        Entity::Polyline(p) if p.closed => Some(p.pts.iter().map(|v| [v.x, v.y]).collect()),
        _ => None,
    }
}

impl Window {
    /// The edit as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<PlatformsEdit, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        let may = |k: usize| number(&self.fields[k], FIELDS[k]).map(|v| v.unwrap_or(0.0));
        let corners = corners_of(&self.fields[1])?;
        if corners.is_empty() {
            return Err("Köşeler yazılmalı ya da seçimden alınmalı.".into());
        }
        Ok(PlatformsEdit {
            operation: RoadEditOperation::Set,
            platform: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            corners: Some(corners),
            base: Some([need(2)?, need(3)?, need(4)?]),
            grade: Some(may(5)?),
            direction: Some(may(6)?),
            cut: Some(need(7)?),
            fill: Some(need(8)?),
            step: Some(need(9)?),
            fan: Some(need(10)?),
            reach: Some(need(11)?),
            surface: Some(self.surface.clone().ok_or("Bir arazi yüzeyi seçin.")?),
            expected_revision: None,
        })
    }

    /// The balance's bracket as typed.
    fn bracket(&self) -> Result<PlatformBalanceAsk, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        Ok(PlatformBalanceAsk { low: need(BRACKET_AT)?, high: need(BRACKET_AT + 1)?, tolerance: need(BRACKET_AT + 2)? })
    }
}

impl App {
    /// Platform: the window on the road project's first pad, or a new one on its first terrain surface.
    pub(crate) fn open_platforms(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Platform yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let surfaces = doc.model.settings().road.as_ref().map(|r| r.surfaces.clone()).unwrap_or_default();
        let Some(first) = surfaces.first() else {
            self.warn("Platform bir arazi yüzeyine tesviye edilir: önce Arazi yüzeyleri'nde bir yüzey oluşturun.".to_owned());
            return;
        };
        let surface = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_platforms().first().map(|p| p.id.clone());
        self.platform_window = Some(Box::new(Window {
            chosen: None,
            surface: Some(surface),
            fields: Default::default(),
            layer,
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Platforms);
        let _ = self.platform_event(Event::Choose(chosen));
        // A closed boundary selected when the window opens is a new pad's.
        if chosen_boundary(self).is_some() && self.platform_window.as_ref().is_some_and(|w| w.chosen.is_none()) {
            let _ = self.platform_event(Event::FromSelection);
        }
    }

    fn refresh_platform(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.platform_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| road_platforms_edit::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None, balance: None },
        };
    }

    /// The balance of the pad as typed: written on a reading copy, then asked there.
    fn balance_platform(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.platform_window.as_mut() else { return };
        let found = (|| {
            let edit = w.input()?;
            let ask = w.bracket()?;
            let mut copy = doc.model.reading_copy();
            let written = match road_platforms_edit::execute(&mut ExecutionContext::new(&mut copy), edit) {
                kentos_commands::CommandResult::Completed { output, .. } => output,
                kentos_commands::CommandResult::NeedsInput { error }
                | kentos_commands::CommandResult::Conflict { error }
                | kentos_commands::CommandResult::Failed { error } => return Err(error.message),
                _ => return Err("Platform okunamadı.".into()),
            };
            let id = written.platform.map(|p| p.id).ok_or("Platform yok.")?;
            let (q, _) = road_platforms_query::preview(&copy, &PlatformsQuery { platform: Some(id), balance: Some(ask) })?;
            q.balance.ok_or_else(|| "Denge bulunamadı.".to_owned())
        })();
        w.preview.balance = Some(found);
    }

    pub(crate) fn platform_event(&mut self, event: Event) -> Task<Message> {
        let kept = self.document.as_ref().map(|d| d.model.road_platforms().to_vec()).unwrap_or_default();
        let picked = chosen_boundary(self);
        let Some(w) = self.platform_window.as_mut() else { return Task::none() };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                let mut f: [String; 15] = Default::default();
                match id.as_ref().and_then(|id| kept.iter().find(|c| &c.id == id)) {
                    Some(p) => {
                        w.surface = Some(p.surface.clone());
                        f[0] = p.name.clone();
                        f[1] = corners_text(&p.corners);
                        f[2] = trimmed(p.base[0]);
                        f[3] = trimmed(p.base[1]);
                        f[4] = trimmed(p.base[2]);
                        f[5] = trimmed(p.grade);
                        f[6] = trimmed(p.direction);
                        f[7] = trimmed(p.cut);
                        f[8] = trimmed(p.fill);
                        f[9] = trimmed(p.step);
                        f[10] = trimmed(p.fan);
                        f[11] = trimmed(p.reach);
                    }
                    None => {
                        // A new one: its slopes, spacing, fan and reach are the user's; only the name is filled in.
                        let n = kept.len() + 1;
                        f[0] = if n == 1 { "Platform".into() } else { format!("Platform {n}") };
                    }
                }
                // The bracket stays as typed between pads.
                f[BRACKET_AT..].clone_from_slice(&w.fields[BRACKET_AT..]);
                w.fields = f;
            }
            Event::Surface(s) => w.surface = Some(s),
            Event::Field(k, t) => w.fields[k.min(14)] = t,
            Event::FromSelection => match picked {
                Some(corners) => {
                    w.fields[1] = corners_text(&corners);
                    // A new pad's base is its first corner until the user types one.
                    if w.fields[2].trim().is_empty() && w.fields[3].trim().is_empty() {
                        w.fields[2] = trimmed(corners[0][0]);
                        w.fields[3] = trimmed(corners[0][1]);
                    }
                }
                None => {
                    self.warn("Köşeleri almak için kapalı bir çoklu çizgi ya da alan seçin.".to_owned());
                    return Task::none();
                }
            },
            Event::Layer(l) => w.layer = Some(l),
            Event::Balance => {
                self.balance_platform();
                return Task::none();
            }
            Event::Apply => {
                // The balance stays said beside the pad it gave.
                let found = w.preview.balance.clone();
                if let Some(Ok(b)) = &found {
                    w.fields[4] = trimmed((b.elevation * 1000.0).round() / 1000.0);
                }
                self.refresh_platform();
                if let Some(w) = self.platform_window.as_mut() {
                    w.preview.balance = found;
                }
                return Task::none();
            }
            Event::Save => return self.save_platform(),
            Event::Draw => return self.draw_platform(),
            Event::Remove => return self.remove_platform(),
            Event::Close => {
                self.platform_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_platform();
        Task::none()
    }

    fn save_platform(&mut self) -> Task<Message> {
        let Some(w) = self.platform_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Platform yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_platforms_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(r) = &out.platform {
            let said = match (r.cut, r.fill) {
                (Some(c), Some(f)) => format!("“{}”: kazı {c:.1} m³, dolgu {f:.1} m³.", r.name),
                _ => format!("“{}” kaydedildi; açık şev çizgisi var, hacim yok.", r.name),
            };
            self.say(Level::Success, said);
            let id = r.id.clone();
            return self.platform_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    fn draw_platform(&mut self) -> Task<Message> {
        let Some(w) = self.platform_window.as_ref() else { return Task::none() };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce platformu kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = PlatformsDraw { platform: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_platforms_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("Platform planda, {} nesne.", out.uids.len()));
            self.platform_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_platform(&mut self) -> Task<Message> {
        let Some(id) = self.platform_window.as_ref().and_then(|w| w.chosen.clone()) else { return Task::none() };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = PlatformsEdit {
            operation: RoadEditOperation::Remove,
            platform: Some(id),
            name: None,
            corners: None,
            base: None,
            grade: None,
            direction: None,
            cut: None,
            fill: None,
            step: None,
            fan: None,
            reach: None,
            surface: None,
            expected_revision: None,
        };
        let result = road_platforms_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_platforms().first().map(|c| c.id.clone()));
            return self.platform_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn platform_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.platform_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni platform".into())];
        listed.extend(
            model.map_or_else(Vec::new, |m| m.road_platforms().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()),
        );
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_pad = Select::new(
            listed.iter().map(|x| Choice::new(x.1.clone())),
            ids.iter().position(|i| *i == w.chosen),
            move |i| msg(Event::Choose(ids[i].clone())),
        );
        let surfaces: Vec<(String, String)> = model
            .and_then(|m| m.settings().road.as_ref())
            .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect());
        let sids: Vec<String> = surfaces.iter().map(|c| c.0.clone()).collect();
        let on_surface = Select::new(
            surfaces.iter().map(|c| Choice::new(c.1.clone())),
            w.surface.as_ref().and_then(|c| sids.iter().position(|i| i == c)),
            move |i| msg(Event::Surface(sids[i].clone())),
        );
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let wide = |hint: &'static str, k: usize| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(Fill),
                None,
            )
        };
        let picked = chosen_boundary(self).is_some();
        let top = row![
            words::field("Platform", container(on_pad).width(160), None),
            field("Platform", 0, 140.0),
            words::field("Arazi yüzeyi", container(on_surface).width(150), None),
            wide("Y,X; Y,X; Y,X", 1),
            words::secondary(FROM_SELECTION, picked.then(|| msg(Event::FromSelection))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let plane = row![
            field("m", 2, 96.0),
            field("m", 3, 96.0),
            field("m", 4, 96.0),
            field("0", 5, 64.0),
            field("0", 6, 80.0),
            field("yatay/düşey", 7, 108.0),
            field("yatay/düşey", 8, 108.0),
            field("m", 9, 76.0),
            field("°", 10, 76.0),
            field("m", 11, 76.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(10).push(top).push(plane);
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                let volumes = match (r.cut, r.fill, r.footprint) {
                    (Some(c), Some(f), Some(fp)) => {
                        format!(" · taban {fp:.1} m² · kazı {c:.1} m³ · dolgu {f:.1} m³ · fark {:.1} m³", c - f)
                    }
                    _ => String::new(),
                };
                let counted = |side: PlatformSide| r.lines.iter().filter(|l| l.side == side).count();
                body = body.push(label::caption(format!(
                    "Platform alanı {:.1} m²{volumes} · {} şev çizgisi: {} yarma, {} dolgu, {} açık",
                    r.top_area,
                    r.lines.len(),
                    counted(PlatformSide::Cut),
                    counted(PlatformSide::Fill),
                    r.open
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                if let Some(why) = &r.why {
                    body = body.push(label::caption(why.clone()).style(danger));
                }
                let picture = container(iced::widget::canvas(Plan { report: r }).width(Fill).height(typography::scaled(300.0)))
                    .style(style::container::field_box);
                let mut side = Column::new()
                    .spacing(8)
                    .width(360)
                    .push(label::caption("Kazı ile dolguyu dengele").style(muted))
                    .push(
                        row![field("m", BRACKET_AT, 100.0), field("m", BRACKET_AT + 1, 100.0), field("m³", BRACKET_AT + 2, 110.0)]
                            .spacing(10)
                            .align_y(iced::alignment::Vertical::Bottom),
                    );
                let found = matches!(&w.preview.balance, Some(Ok(_)));
                side = side.push(
                    row![
                        words::secondary(BALANCE, Some(msg(Event::Balance))),
                        words::secondary(APPLY, found.then(|| msg(Event::Apply))),
                    ]
                    .spacing(8),
                );
                match &w.preview.balance {
                    Some(Ok(b)) => {
                        side = side.push(label::caption(format!(
                            "Dengeli taban kotu {:.3} m · kalan fark {:.2} m³ · {} adım{}",
                            b.elevation,
                            b.net,
                            b.iterations,
                            if b.converged { "" } else { " (yakınsamadı)" }
                        )));
                    }
                    Some(Err(why)) => side = side.push(label::caption(why.clone()).style(danger)),
                    None => {}
                }
                side = side.push(
                    label::caption("Taban kotu, aralıkta kazı ile dolgunun farkı toleransa inene dek aranır; Uygula yalnız alana yazar.")
                        .style(muted),
                );
                body = body.push(row![picture, side].spacing(12));
            }
            _ => {}
        }
        let lids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let lnames: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let on_layer = Select::new(
            lnames.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        let saved = w.chosen.is_some();
        body = body.push(
            row![
                words::secondary(REMOVE, saved.then(|| msg(Event::Remove))),
                space::horizontal().width(Fill),
                label::caption("Katman").style(muted),
                container(on_layer).width(150),
                words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
                space::horizontal().width(24),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(SAVE, w.preview.report.as_ref().map(|_| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn platform_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.platform_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(BALANCE) => Some(msg(Event::Balance)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(FROM_SELECTION) => Some(msg(Event::FromSelection)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The one selected object's closed boundary, if it is one.
fn chosen_boundary(app: &App) -> Option<Vec<[f64; 2]>> {
    let doc = app.document.as_ref()?;
    match app.selection.ids() {
        [slot] => doc.model.get(*slot).and_then(boundary_of).filter(|c| c.len() >= 3),
        _ => None,
    }
}

/// The pad in plan: its slope lines by side, the daylight line, the pad's boundary over them.
struct Plan<'a> {
    report: &'a PlatformReport,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.report;
        let all = r
            .top
            .iter()
            .map(|p| [p[0], p[1]])
            .chain(r.lines.iter().filter_map(|l| l.to.map(|t| [t[0], t[1]])))
            .chain(r.lines.iter().map(|l| [l.from[0], l.from[1]]));
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in all {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        if !x0.is_finite() || r.top.len() < 3 {
            return vec![frame.into_geometry()];
        }
        let pad = 18.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let scale = (w / (x1 - x0).max(1e-6)).min(h / (y1 - y0).max(1e-6));
        let (ox, oy) = (pad + (w - (x1 - x0) * scale) / 2.0, pad + (h - (y1 - y0) * scale) / 2.0);
        let at = |x: f64, y: f64| Point::new((ox + (x - x0) * scale) as f32, (oy + (y1 - y) * scale) as f32);
        if r.daylight.len() >= 3 {
            let ring = Path::new(|p| {
                p.move_to(at(r.daylight[0][0], r.daylight[0][1]));
                for d in &r.daylight[1..] {
                    p.line_to(at(d[0], d[1]));
                }
                p.close();
            });
            frame.fill(&ring, iced::Color { a: 0.08, ..tk.text });
            frame.stroke(&ring, Stroke::default().with_width(1.2).with_color(tk.muted));
        }
        for l in &r.lines {
            let (color, end) = match (l.side, l.to) {
                (PlatformSide::Cut, Some(t)) => (tk.danger, t),
                (PlatformSide::Fill, Some(t)) => (tk.success, t),
                _ => (tk.warning, l.from),
            };
            if l.to.is_none() {
                let mark = Path::circle(at(l.from[0], l.from[1]), 3.0);
                frame.fill(&mark, tk.warning);
                continue;
            }
            let line = Path::line(at(l.from[0], l.from[1]), at(end[0], end[1]));
            frame.stroke(&line, Stroke::default().with_width(1.0).with_color(color));
        }
        let top = Path::new(|p| {
            p.move_to(at(r.top[0][0], r.top[0][1]));
            for c in &r.top[1..] {
                p.line_to(at(c[0], c[1]));
            }
            p.close();
        });
        frame.fill(&top, iced::Color { a: 0.22, ..tk.accent });
        frame.stroke(&top, Stroke::default().with_width(1.8).with_color(tk.accent));
        for (i, c) in r.top.iter().enumerate() {
            frame.fill_text(Text {
                content: format!("{:.2}", c[2]),
                position: at(c[0], c[1]) + iced::Vector::new(4.0, if i % 2 == 0 { 2.0 } else { -14.0 }),
                color: tk.text,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        let legend = [("yarma", tk.danger), ("dolgu", tk.success), ("açık", tk.warning), ("günışığı", tk.muted)];
        let mut x = 8.0_f32;
        for (word, color) in legend {
            frame.stroke(
                &Path::line(Point::new(x, 12.0), Point::new(x + 14.0, 12.0)),
                Stroke::default().with_width(2.0).with_color(color),
            );
            frame.fill_text(Text {
                content: word.into(),
                position: Point::new(x + 18.0, 5.0),
                color: tk.muted,
                size: Pixels(11.0),
                ..Text::default()
            });
            x += 30.0 + 7.0 * word.chars().count() as f32;
        }
        vec![frame.into_geometry()]
    }
}
