//! Tarama ağı dengelemesi (docs/adr/4720; `pointcloud.network`): three or more scans registered together by
//! `cad.pointclouds.network`, read only, as a GNSS network adjusts its baselines. The scans are the selected point
//! clouds (Seçili bulutları al; at first the selection, else every cloud of the drawing); one is held (Sabit tarama,
//! the first at first); every pair whose plan boxes overlap is a link, registered as Tarama kaydı registers two (the
//! voxel, the reach, the steps; Huber's weights at first, since the edges of roofs pull an unweighted registration a
//! few centimetres; the pairs at a scan's edge left out on request). The summary is the command's notes; the tables
//! give each scan's correction and plan strength, each link's pairs, overlap, RMS and the shift the network gives it,
//! and each loop's misclosure. Dönüşüm uygula… selects the chosen scan and opens Dönüşüm uygula (`pointcloud.transform`)
//! with its matrix: a scan is moved only when that tool runs, into a new file beside its source.
//!
//! ```text
//! ┌ Tarama ağı dengelemesi ─────────────────────────────────────────────────────┐
//! │ Taramalar        4 tarama: ag-a, ag-b, ag-c, ag-d     [Seçili bulutları al]  │
//! │ Sabit tarama     [ag-a.xyz ▾]                                                │
//! │ Voksel (m)       [0,50]  Erişim (m) [1,00]  En çok adım [30]                 │
//! │ Ağırlık          [Eşit | Huber]   [ ] Kenardaki eşleşmeleri ayıkla           │
//! │ ┌ summary ─────────────────────────────────────────────────────────────────┐│
//! │ Uygulanacak tarama [ag-b.xyz ▾]                       [Dönüşüm uygula…]      │
//! │ [Taramalar | Bağlar | Döngüler]                                              │
//! │ ┌ the chosen table ───────────────────────────────────────────────────────┐ │
//! │                                                                     [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{container, row, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, Entity, PointCloudsNetwork, PointCloudsNetworked, RegisterReject, RegisterRobust};
use kentos_native_application::{ExecutionContext, pointcloud_network, pointcloud_register};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::register_window::matrix_text;
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Tarama ağı dengelemesi";
const CLOSE: &str = "Kapat";
pub const TAKE: &str = "Seçili bulutları al";
pub const APPLY: &str = "Dönüşüm uygula…";
/// docs/adr/5100: the chosen scan's matrix kept on its object, its file not written.
pub const KEEP: &str = "Nesnede tut";
const EDGES: &str = "Kenardaki eşleşmeleri ayıkla";
/// The layer Dönüşüm uygula writes the moved scan on.
const MOVED_LAYER: &str = "Ağda dengelenen taramalar";

/// The pairs' weights, in the window's words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Weights {
    Plain,
    #[default]
    Huber,
}

impl std::fmt::Display for Weights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Weights::Plain => "Eşit",
            Weights::Huber => "Huber",
        })
    }
}

/// Which table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Scans,
    Links,
    Loops,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Scans => "Taramalar",
            Show::Links => "Bağlar",
            Show::Loops => "Döngüler",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The network's scans (id, name), in order.
    pub scans: Vec<(String, String)>,
    pub fixed: String,
    pub spacing: f64,
    pub reach: f64,
    pub iterations: f64,
    pub weights: Weights,
    pub edges: bool,
    /// The scan Dönüşüm uygula takes.
    pub chosen: String,
    pub show: Show,
    pub answer: Option<Result<PointCloudsNetworked, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Take,
    Fixed(String),
    Spacing(f64),
    Reach(f64),
    Iterations(f64),
    Weights(Weights),
    Edges(bool),
    Chosen(String),
    Show(Show),
    Apply,
    Keep,
    Close,
}

fn msg(e: Event) -> Message {
    Message::ScanNetwork(e)
}

/// A cloud's name: its first file's, else its number.
fn cloud_name(e: &Entity) -> Option<String> {
    let Entity::PointCloud(c) = e else { return None };
    Some(c.cloud.sources.first().and_then(|s| s.file.as_deref()).map_or_else(
        || format!("Bulut #{}", e.base().id),
        |f| f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned(),
    ))
}

impl App {
    /// The point clouds selected, in the selection's order; none selected: every cloud of the drawing.
    fn scan_network_clouds(&self) -> Vec<(String, String)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let selected: Vec<(String, String)> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| {
                let e = doc.model.get(*s)?;
                let name = cloud_name(e)?;
                Some((doc.model.uid(*s)?.to_string(), name))
            })
            .collect();
        if !selected.is_empty() {
            return selected;
        }
        doc.model.entities_with_uids().filter_map(|(u, e)| cloud_name(e).map(|n| (u.to_string(), n))).collect()
    }

    fn scan_network_run(&mut self) {
        let s = &self.scan_network;
        let input = PointCloudsNetwork {
            uids: s.scans.iter().map(|(u, _)| u.clone()).collect(),
            fixed: vec![s.fixed.clone()],
            spacing: Some(s.spacing),
            reach: Some(s.reach),
            iterations: Some(s.iterations.max(1.0) as u32),
            robust: (s.weights == Weights::Huber)
                .then(|| RegisterRobust { kind: "huber".into(), k: Some(pointcloud_register::HUBER_K), ratio: None }),
            reject: s.edges.then_some(RegisterReject { edges: true, angle: None }),
            drawing: self.document.as_ref().and_then(|d| d.path.as_ref()).map(|p| p.display().to_string()),
            ..Default::default()
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.scan_network.answer = Some(match pointcloud_network::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Tarama ağı dengelenemedi.".to_owned()),
        });
    }

    /// Tarama ağı dengelemesi (`pointcloud.network`): the selected clouds (else every cloud), the first held.
    pub(crate) fn open_scan_network(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        let scans = self.scan_network_clouds();
        if scans.len() < 3 {
            self.warn("Tarama ağı dengelemesi: en az üç nokta bulutu gerekir; iki tarama için Tarama kaydı'nı kullanın.");
            return Task::none();
        }
        let fixed = scans[0].0.clone();
        let chosen = scans[1].0.clone();
        self.scan_network = State {
            scans,
            fixed,
            spacing: 0.5,
            reach: 1.0,
            iterations: 30.0,
            weights: Weights::Huber,
            chosen,
            ..Default::default()
        };
        self.scan_network_run();
        self.dialog = Some(Dialog::ScanNetwork);
        Task::none()
    }

    /// Dönüşüm uygula… (docs/adr/3100): the chosen scan selected (the tool's input), the window closed and Dönüşüm
    /// uygula opened with its matrix and a layer of its own. Nothing is written until that tool runs.
    fn scan_network_apply(&mut self) {
        let Some(Ok(r)) = &self.scan_network.answer else { return };
        let Some(scan) = r.scans.iter().find(|s| s.uid == self.scan_network.chosen) else { return };
        if scan.fixed {
            self.warn(format!("{TITLE}: sabit tarama yerinde kalır; başka bir tarama seçin."));
            return;
        }
        let matrix = matrix_text(&scan.matrix);
        let slot = self.document.as_ref().and_then(|d| kentos_domain::Uuid::parse_str(&scan.uid).ok().and_then(|u| d.model.slot_of(u)));
        let Some(slot) = slot else {
            self.warn(format!("{TITLE}: tarama çizimde bulunamadı."));
            return;
        };
        self.selection.set([slot]);
        self.dialog = None;
        let values = serde_json::from_value(serde_json::json!({
            "input": { "scope": "selection" },
            "matrix": matrix,
            "layer": { "newName": MOVED_LAYER },
        }))
        .ok();
        self.open_processing("pointcloud.transform", values);
    }

    /// Nesnede tut (docs/adr/5100): the chosen scan's matrix taken after its place by `cad.pointclouds.place`, one
    /// undo step; its file is not written. The network is adjusted again from there: the scan's correction is then
    /// what is left.
    fn scan_network_keep(&mut self) {
        let Some(Ok(r)) = &self.scan_network.answer else { return };
        let Some(scan) = r.scans.iter().find(|s| s.uid == self.scan_network.chosen) else { return };
        if scan.fixed {
            self.warn(format!("{TITLE}: sabit tarama yerinde kalır; başka bir tarama seçin."));
            return;
        }
        let name = self.scan_network.scans.iter().find(|(u, _)| *u == scan.uid).map(|(_, n)| n.clone()).unwrap_or_default();
        let input = kentos_commands::PointCloudsPlace {
            uid: scan.uid.clone(),
            place: kentos_commands::CloudPlaceBy::Matrix { matrix: scan.matrix.to_vec() },
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return };
        match kentos_native_application::pointcloud_place::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { .. } => {
                self.say(
                    kentos_interaction::Level::Success,
                    format!("{TITLE}: “{name}” taramasının konumu nesnede tutuldu; dosyası değişmedi. Tek adımda geri alınır."),
                );
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
        self.scan_network_run();
    }

    pub(crate) fn scan_network_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.scan_network;
        match event {
            Event::Take => {
                let scans = self.scan_network_clouds();
                if scans.len() < 3 {
                    self.warn(format!("{TITLE}: en az üç nokta bulutu seçin; seçimde {} var.", scans.len()));
                    return Task::none();
                }
                let s = &mut self.scan_network;
                if !scans.iter().any(|(u, _)| *u == s.fixed) {
                    s.fixed = scans[0].0.clone();
                }
                if !scans.iter().any(|(u, _)| *u == s.chosen) {
                    s.chosen = scans[1].0.clone();
                }
                s.scans = scans;
            }
            Event::Fixed(v) => s.fixed = v,
            Event::Spacing(v) => s.spacing = v,
            Event::Reach(v) => s.reach = v,
            Event::Iterations(v) => s.iterations = v,
            Event::Weights(v) => s.weights = v,
            Event::Edges(v) => s.edges = v,
            Event::Chosen(v) => {
                s.chosen = v;
                return Task::none();
            }
            Event::Show(v) => {
                s.show = v;
                return Task::none();
            }
            Event::Apply => {
                self.scan_network_apply();
                return Task::none();
            }
            Event::Keep => {
                self.scan_network_keep();
                return Task::none();
            }
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.scan_network_run();
        Task::none()
    }

    /// The summary's lines and the chosen table's rows.
    pub(crate) fn scan_network_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let s = &self.scan_network;
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let r = match &s.answer {
            None => return (lines, rows),
            Some(Err(why)) => {
                lines.push(words::text_line(Kind::Error, why.clone()));
                return (lines, rows);
            }
            Some(Ok(r)) => r,
        };
        for (k, n) in r.notes.iter().enumerate() {
            lines.push(words::text_line(if k == 0 { Kind::Ok } else { Kind::Info }, n.clone()));
        }
        let format = self.format();
        let degrees = format.angle_unit_label() == "°";
        // Small angles in the project's small unit: cc in a gon project, seconds in a degree one.
        let angle = |rad: f64| {
            if degrees {
                format!("{:.1}″", rad.to_degrees() * 3600.0)
            } else {
                format!("{:.1} cc", rad * 200.0 / std::f64::consts::PI * 10_000.0)
            }
        };
        let name = |u: &str| s.scans.iter().find(|(id, _)| id == u).map_or_else(|| u.to_owned(), |(_, n)| n.clone());
        let mm = |m: f64| format!("{:.1}", m * 1000.0);
        match s.show {
            Show::Scans => {
                for sc in &r.scans {
                    rows.push(vec![
                        name(&sc.uid),
                        if sc.fixed { "Sabit".to_owned() } else { String::new() },
                        angle(sc.rotation[0]),
                        angle(sc.rotation[1]),
                        angle(sc.rotation[2]),
                        format!("{} / {} / {}", mm(sc.translation[0]), mm(sc.translation[1]), mm(sc.translation[2])),
                        sc.horizontal.map(|h| format!("{h:.2}")).unwrap_or_default(),
                    ]);
                }
            }
            Show::Links => {
                for l in &r.links {
                    rows.push(vec![
                        format!("{} – {}", name(&l.scans[0]), name(&l.scans[1])),
                        l.pairs.to_string(),
                        format!("{:.0}", l.overlap * 100.0),
                        mm(l.rms_after),
                        format!("{:.2}", l.strength),
                        mm(l.shift),
                    ]);
                }
            }
            Show::Loops => {
                for lp in &r.loops {
                    rows.push(vec![
                        lp.scans.iter().map(|u| name(u)).collect::<Vec<_>>().join(" → "),
                        angle(lp.rotation),
                        mm(lp.translation),
                    ]);
                }
            }
        }
        (lines, rows)
    }

    pub(crate) fn scan_network_view(&self) -> Element<'_, Message> {
        let s = &self.scan_network;
        let names: Vec<String> = s.scans.iter().map(|(_, n)| n.clone()).collect();
        let free: Vec<(String, String)> = s.scans.iter().filter(|(u, _)| *u != s.fixed).cloned().collect();
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Taramalar",
                row![
                    label::body(format!("{} tarama: {}", s.scans.len(), names.join(", "))).width(Fill),
                    words::secondary(TAKE, Some(msg(Event::Take))),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field("Sabit tarama", container(fields::select_owned(s.scans.clone(), &s.fixed, |v| msg(Event::Fixed(v)))).width(260))
            .field(
                "Voksel (m)",
                row![
                    NumberInput::new(s.spacing, |v| msg(Event::Spacing(v))).step(0.25).decimals(2).point().width(100),
                    label::caption("Erişim (m)"),
                    NumberInput::new(s.reach, |v| msg(Event::Reach(v))).step(0.5).decimals(2).point().width(80),
                    label::caption("En çok adım"),
                    NumberInput::new(s.iterations, |v| msg(Event::Iterations(v))).step(5.0).decimals(0).width(80)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Ağırlık",
                row![
                    Segmented::new([Weights::Plain, Weights::Huber], s.weights, |v| msg(Event::Weights(v))),
                    fields::check(
                        if s.edges { Check::Checked } else { Check::Unchecked },
                        EDGES.to_owned(),
                        Some(msg(Event::Edges(!s.edges))),
                    ),
                ]
                .spacing(14)
                .align_y(Center),
            )
            .help("Her bağ Tarama kaydı gibi kaydedilir; çatı kenarındaki eşleşmeler ağırlıksız kaydı birkaç santimetre saptırır, Huber ağırlığı onları bastırır.");
        let (lines, rows) = self.scan_network_lines();
        let table = match s.show {
            Show::Scans => sheet(
                vec![col("Tarama", 140.0), col("", 60.0), num("ω", 100.0), num("φ", 100.0), num("κ", 100.0), num("Öteleme D / K / Kot (mm)", 210.0), num("Yatay güç", 90.0)],
                rows,
                "Tarama yok.",
                220.0,
            ),
            Show::Links => sheet(
                vec![col("Bağ", 220.0), num("Eşleşme", 80.0), num("Örtüşme %", 90.0), num("Kayıt RMS (mm)", 120.0), num("Güç", 70.0), num("Ağdaki kayma (mm)", 140.0)],
                rows,
                "Bağ yok.",
                220.0,
            ),
            Show::Loops => sheet(vec![col("Döngü", 330.0), num("Dönme kapanması", 170.0), num("Öteleme kapanması (mm)", 170.0)], rows, "Döngü yok: bağlar ağaç gibi.", 220.0),
        };
        let ready = matches!(&s.answer, Some(Ok(_))) && s.chosen != s.fixed;
        let apply = row![
            label::caption("Uygulanacak tarama"),
            container(fields::select_owned(free, &s.chosen, |v| msg(Event::Chosen(v)))).width(240),
            space::horizontal().width(Fill),
            words::secondary(KEEP, ready.then(|| msg(Event::Keep))),
            words::primary(APPLY, ready.then(|| msg(Event::Apply))),
        ]
        .spacing(10)
        .align_y(Center);
        let body = Grows::new()
            .spacing(12.0)
            .push(form)
            .push(words::summary(lines))
            .push(apply)
            .push(Segmented::new([Show::Scans, Show::Links, Show::Loops], s.show, |v| msg(Event::Show(v))))
            .push_grow(table, 220.0);
        Frame::new(TITLE).scroll(body).action(words::secondary(CLOSE, Some(msg(Event::Close)))).width(960.0).modal(msg(Event::Close))
    }

    pub(crate) fn scan_network_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(TAKE) => Some(msg(Event::Take)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(KEEP) => Some(msg(Event::Keep)),
            Control::Check(EDGES, on) => Some(msg(Event::Edges(on))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
