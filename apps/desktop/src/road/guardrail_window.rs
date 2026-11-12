//! Otokorkuluk (plan/road.md Y11.1, docs/adr/0909 §1, `guardrail.editor`): a road project's guardrails along a
//! corridor. The rule is the project's own table, its source required; KentOS fills in no number. The runs are read as
//! the rule is typed (`road.guardrails.edit`'s plan on a reading copy): each side's runs as bars along the stations, the
//! warranted stretch dark within the lengthened run, and their table. Kaydet writes the guardrail (one undo step
//! “Otokorkuluk”), Planda çiz draws its runs with their km on a layer (one step “Otokorkuluk çiz”).
//!
//! ```text
//! ┌ Otokorkuluk ─────────────────────────────────────────────────────────────────────────────────┐
//! │ Korkuluk [▾ Yeni] Ad [ ] Koridor [▾] Kaynak [                    ] Okuma (m) [ ] Dışa (m) [ ]    │
//! │ Kural: Dolgu ≥ (m) [ ]  Şev 1: ≤ [ ]  …  [Satır ekle] [Satırı sil]                            │
//! │ Yaklaşma (m) [ ] Ayrılma (m) [ ] En az aralık (m) [ ]                                           │
//! │ ┌ left and right runs along the stations ┐  Yan  Km  Boy  Gerekli  Dolgu  Şev                  │
//! │ └────────────────────────────────────────┘                                                      │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]       │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::road;
use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, GuardrailReport, GuardrailRuleDef, GuardrailWarrantDef, GuardrailsDraw, GuardrailsEdit,
    RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_guardrails_draw, road_guardrails_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Otokorkuluk";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
const ADD: &str = "Satır ekle";
const REMOVE_ROW: &str = "Satırı sil";
/// The typed fields, in order.
const FIELDS: [&str; 7] = ["Ad", "Kaynak", "Okuma aralığı (m)", "Dışa (m)", "Yaklaşma (m)", "Ayrılma (m)", "En az aralık (m)"];

/// The runs as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<GuardrailReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The corridor's first and last station.
    pub(crate) range: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The guardrail edited: its id, none for a new one.
    pub(crate) chosen: Option<String>,
    pub(crate) corridor: Option<String>,
    pub(crate) fields: [String; 7],
    /// The rule's rows: the least fill height and the slope (1:n, empty for any), as typed.
    pub(crate) warrants: Vec<(String, String)>,
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A guardrail of the project, none for a new one.
    Choose(Option<String>),
    Corridor(String),
    Field(usize, String),
    /// A row's height (false) or slope (true).
    Warrant(usize, bool, String),
    AddRow,
    RemoveRow,
    Layer(String),
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Road(road::Event::Guardrails(event))
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
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The edit as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<GuardrailsEdit, String> {
        let need = |k: usize| -> Result<f64, String> {
            number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]))
        };
        let mut warrants = Vec::new();
        for (i, (h, s)) in self.warrants.iter().enumerate() {
            let height = number(h, &format!("{}. satırın dolgusu", i + 1))?
                .ok_or_else(|| format!("{}. satırın en az dolgu yüksekliğini yazın.", i + 1))?;
            warrants.push(GuardrailWarrantDef { height, slope: number(s, &format!("{}. satırın şevi", i + 1))? });
        }
        Ok(GuardrailsEdit {
            operation: RoadEditOperation::Set,
            guardrail: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            corridor: Some(self.corridor.clone().ok_or("Bir koridor seçin.")?),
            rule: Some(GuardrailRuleDef {
                source: self.fields[1].trim().to_owned(),
                warrants,
                approach: need(4)?,
                leaving: need(5)?,
                gap: need(6)?,
            }),
            step: Some(need(2)?),
            offset: number(&self.fields[3], FIELDS[3])?,
            expected_revision: None,
        })
    }
}

impl App {
    /// Otokorkuluk: the window on the road project's first guardrail, or a new one on its first corridor.
    pub(crate) fn open_guardrails(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Otokorkuluk yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first() else {
            self.warn("Otokorkuluk bir koridorun kesitlerinden çıkar: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        let corridor = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_guardrails().first().map(|g| g.id.clone());
        self.road.guardrail_window = Some(Box::new(Window {
            chosen: None,
            corridor: Some(corridor),
            fields: ["Otokorkuluk".into(), String::new(), "10".into(), String::new(), String::new(), String::new(), String::new()],
            warrants: vec![(String::new(), String::new())],
            layer,
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::Guardrails));
        let _ = self.guardrail_event(Event::Choose(chosen));
    }

    fn refresh_guardrail(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road.guardrail_window.as_mut() else {
            return;
        };
        let range = w
            .corridor
            .as_ref()
            .and_then(|c| doc.model.road_corridors().iter().find(|x| &x.id == c))
            .map_or([0.0, 0.0], |c| {
                [
                    c.regions.iter().map(|r| r.from).fold(f64::INFINITY, f64::min),
                    c.regions.iter().map(|r| r.to).fold(f64::NEG_INFINITY, f64::max),
                ]
            });
        w.preview = match w.input().and_then(|q| road_guardrails_edit::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), range, ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None, range },
        };
    }

    pub(crate) fn guardrail_event(&mut self, event: Event) -> Task<Message> {
        let kept = self
            .document
            .as_ref()
            .map(|d| d.model.road_guardrails().to_vec())
            .unwrap_or_default();
        let Some(w) = self.road.guardrail_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                if let Some(g) = id.as_ref().and_then(|id| kept.iter().find(|g| &g.id == id)) {
                    w.corridor = Some(g.corridor.clone());
                    w.fields = [
                        g.name.clone(),
                        g.rule.source.clone(),
                        trimmed(g.step),
                        if g.offset == 0.0 { String::new() } else { trimmed(g.offset) },
                        trimmed(g.rule.approach),
                        trimmed(g.rule.leaving),
                        trimmed(g.rule.gap),
                    ];
                    w.warrants =
                        g.rule.warrants.iter().map(|x| (trimmed(x.height), x.slope.map(trimmed).unwrap_or_default())).collect();
                } else {
                    // A new one: the rule is the user's, nothing filled in but the step it is read at.
                    let n = kept.len() + 1;
                    w.fields = [
                        if n == 1 { "Otokorkuluk".into() } else { format!("Otokorkuluk {n}") },
                        String::new(),
                        "10".into(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ];
                    w.warrants = vec![(String::new(), String::new())];
                }
            }
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Field(k, t) => w.fields[k.min(6)] = t,
            Event::Warrant(i, slope, t) => {
                if let Some(r) = w.warrants.get_mut(i) {
                    if slope { r.1 = t } else { r.0 = t }
                }
            }
            Event::AddRow => w.warrants.push((String::new(), String::new())),
            Event::RemoveRow => {
                if w.warrants.len() > 1 {
                    w.warrants.pop();
                }
            }
            Event::Layer(l) => w.layer = Some(l),
            Event::Save => return self.save_guardrail(),
            Event::Draw => return self.draw_guardrail(),
            Event::Remove => return self.remove_guardrail(),
            Event::Close => {
                self.road.guardrail_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_guardrail();
        Task::none()
    }

    /// Kaydet: `road.guardrails.edit`, one undo step; the window stays on it.
    fn save_guardrail(&mut self) -> Task<Message> {
        let Some(w) = self.road.guardrail_window.as_ref() else {
            return Task::none();
        };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Otokorkuluk yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_guardrails_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(g) = &out.guardrail {
            self.say(
                Level::Success,
                format!("“{}”: solda {:.1} m, sağda {:.1} m otokorkuluk ({} korkuluk).", g.name, g.left, g.right, g.runs.len()),
            );
            let id = g.id.clone();
            return self.guardrail_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    /// Planda çiz: the saved guardrail's runs with their km, `road.guardrails.draw`, one undo step.
    fn draw_guardrail(&mut self) -> Task<Message> {
        let Some(w) = self.road.guardrail_window.as_ref() else {
            return Task::none();
        };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce otokorkuluğu kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = GuardrailsDraw { guardrail: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_guardrails_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("{} korkuluk planda, toplam {:.1} m.", out.runs, out.length));
            self.road.guardrail_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_guardrail(&mut self) -> Task<Message> {
        let Some(id) = self
            .road
            .guardrail_window
            .as_ref()
            .and_then(|w| w.chosen.clone())
        else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = GuardrailsEdit {
            operation: RoadEditOperation::Remove,
            guardrail: Some(id),
            name: None,
            corridor: None,
            rule: None,
            step: None,
            offset: None,
            expected_revision: None,
        };
        let result = road_guardrails_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_guardrails().first().map(|g| g.id.clone()));
            return self.guardrail_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn guardrail_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road.guardrail_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni otokorkuluk".into())];
        listed.extend(model.map_or_else(Vec::new, |m| {
            m.road_guardrails().iter().map(|g| (Some(g.id.clone()), g.name.clone())).collect()
        }));
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_guardrail = Select::new(
            listed.iter().map(|x| Choice::new(x.1.clone())),
            ids.iter().position(|i| *i == w.chosen),
            move |i| msg(Event::Choose(ids[i].clone())),
        );
        let corridors: Vec<(String, String)> =
            model.map_or_else(Vec::new, |m| m.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect());
        let cids: Vec<String> = corridors.iter().map(|c| c.0.clone()).collect();
        let on_corridor = Select::new(
            corridors.iter().map(|c| Choice::new(c.1.clone())),
            w.corridor.as_ref().and_then(|c| cids.iter().position(|i| i == c)),
            move |i| msg(Event::Corridor(cids[i].clone())),
        )
        .placeholder("Koridor");
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Korkuluk", container(on_guardrail).width(170), None),
            field("Otokorkuluk", 0, 150.0),
            words::field("Koridor", container(on_corridor).width(150), None),
            field("standart ve tablosu", 1, 260.0),
            field("10", 2, 90.0),
            field("0", 3, 70.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut rule = row![label::caption("Kural:").style(muted)].spacing(8).align_y(iced::alignment::Vertical::Center);
        for (i, (h, s)) in w.warrants.iter().enumerate() {
            rule = rule
                .push(label::caption(format!("{}. dolgu ≥", i + 1)).style(muted))
                .push(kentos_ui::widget::text_field("m", h).on_input(move |t| msg(Event::Warrant(i, false, t))).width(56))
                .push(label::caption("şev 1:").style(muted))
                .push(kentos_ui::widget::text_field("her", s).on_input(move |t| msg(Event::Warrant(i, true, t))).width(56));
        }
        rule = rule
            .push(words::secondary(ADD, (w.warrants.len() < 32).then(|| msg(Event::AddRow))))
            .push(words::secondary(REMOVE_ROW, (w.warrants.len() > 1).then(|| msg(Event::RemoveRow))));
        let lengths = row![field("m", 4, 100.0), field("m", 5, 100.0), field("m", 6, 110.0)]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom);
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top).push(rule).push(lengths);
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "{} kesit okundu · solda {:.1} m, sağda {:.1} m otokorkuluk · kaynak: {}",
                    r.crosses, r.left, r.right, r.source
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let picture = crate::picture::picture(iced::widget::canvas(Strip { report: r, range: w.preview.range }).width(Fill).height(Fill), TITLE);
                use crate::road_dialog::{col, grouped, num};
                let list = crate::road_dialog::sheet_fill(
                    vec![col("Yan", 52.0), col("Km", 190.0), num("Boy (m)", 80.0), col("Gerekli", 110.0), num("Dolgu (m)", 84.0), num("Şev 1:", 64.0)],
                    r.runs
                        .iter()
                        .map(|run| {
                            vec![
                                if run.side == "right" { "Sağ" } else { "Sol" }.to_owned(),
                                format!("{} – {}", run.from_text, run.to_text),
                                grouped(run.length, 1),
                                format!("{:.0}–{:.0} m", run.warranted[0], run.warranted[1]),
                                grouped(run.height, 2),
                                grouped(run.slope, 2),
                            ]
                        })
                        .collect(),
                    "Kural bu koridorda hiçbir yerde tutmuyor: otokorkuluk yok.",
                );
                body = body.push_grow(
                    row![picture, list]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(200.0),
                );
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
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
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
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1060.0).modal(msg(Event::Close))
    }

    pub(crate) fn guardrail_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road.guardrail_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD) => Some(msg(Event::AddRow)),
            Control::Press(REMOVE_ROW) => Some(msg(Event::RemoveRow)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The runs along the stations: the left side above, the right below; each run light, its warranted stretch dark.
struct Strip<'a> {
    report: &'a GuardrailReport,
    range: [f64; 2],
}

impl canvas::Program<Message> for Strip<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let [s0, s1] = self.range;
        if s1.partial_cmp(&s0) != Some(std::cmp::Ordering::Greater) {
            return vec![frame.into_geometry()];
        }
        let (pad, w) = (40.0_f32, bounds.width - 56.0);
        let x = |s: f64| pad + ((s - s0) / (s1 - s0)) as f32 * w;
        let lanes = [("Sol", bounds.height * 0.3), ("Sağ", bounds.height * 0.62)];
        for (name, y) in lanes {
            frame.stroke(
                &Path::line(Point::new(pad, y), Point::new(pad + w, y)),
                Stroke::default().with_width(1.0).with_color(tk.muted),
            );
            frame.fill_text(Text {
                content: name.into(),
                position: Point::new(6.0, y),
                color: tk.muted,
                size: Pixels(11.0),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
        for run in &self.report.runs {
            let y = if run.side == "right" { lanes[1].1 } else { lanes[0].1 };
            let bar = |a: f64, b: f64, h: f32| Path::rectangle(Point::new(x(a), y - h / 2.0), iced::Size::new((x(b) - x(a)).max(1.0), h));
            frame.fill(&bar(run.from, run.to, 14.0), iced::Color { a: 0.25, ..tk.accent });
            frame.fill(&bar(run.warranted[0], run.warranted[1], 14.0), tk.accent);
        }
        // The stations at the ends and every tenth of the way.
        let base = bounds.height * 0.86;
        for k in 0..=4 {
            let s = s0 + (s1 - s0) * f64::from(k) / 4.0;
            frame.fill_text(Text {
                content: format!("{s:.0}"),
                position: Point::new(x(s), base),
                color: tk.muted,
                size: Pixels(10.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}
