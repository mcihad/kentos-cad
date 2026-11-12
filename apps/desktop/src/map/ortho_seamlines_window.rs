//! Mozaik birleşme çizgisi (docs/adr/4700; `cad.orthophoto.seamlines`), in the raster tab beside Ortofoto
//! karşılaştırması. Two orthophotos' seamline (birleşme çizgisi) and mosaic elements (mozaik elemanı, TUCBS
//! Ortogörüntü): the images (the selected two first), their footprints (ayak izi: the rasters' extents, or two areas
//! taken from the selection), the method (eşit uzaklık / Voronoi, or radyometrik fark with its cell, search band and the
//! areas not to cross) and the layer the elements go on. The command's plan runs on every change and is drawn in the
//! window: the two elements, the overlap and the line; Noktalar lists the line's points. Mozaik elemanlarını çiz writes
//! them in one undo step and selects them, ready for Ortofoto karşılaştırması's Seçili mozaik elemanlarını al
//! (docs/adr/4460).
//!
//! Docs/adr/5260: the kind above (İki görüntü | Ağ | Ayak izi) shows instead the network of the rasters ticked or a
//! raster's footprint from its pixels (ortho_network_section.rs).
//!
//! ```text
//! ┌ Mozaik birleşme çizgisi ────────────────────────────────────────────────────┐
//! │ Ortofotolar        [dikis-a.tif · 10.0 cm ▾] [Yer değiştir] [dikis-b.tif ▾]  │
//! │ Ayak izleri   [Raster sınırları | Seçili alanlar]  [Seçili alanları al] │
//! │ Yöntem             [Eşit uzaklık (Voronoi) | Radyometrik fark]              │
//! │ Eğri kesimde aralık [1,0] m   (radyometrik fark: hücre, arama bandı, alanlar)│
//! │ ┌ summary ───────────────────────────────────────────────────────────────┐ │
//! │ Yazılacak katman [Mozaik elemanları (yeni) ▾]  [Mozaik elemanlarını çiz]   │
//! │ [Plan | Noktalar]                                                           │
//! │ ┌ the elements, the overlap and the line, or the line's points ──────────┐ │
//! │                                                                    [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{container, row, space};
use iced::{Center, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandResult, Entity, OrthoSeamMethod, OrthophotoSeamlines, OrthophotoSeamlinesBuilt, RasterFields, SEAMLINE_LAYER,
    SEAMLINE_LAYER_NAME,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, orthophoto_seamlines};
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Mozaik birleşme çizgisi";
const CLOSE: &str = "Kapat";
const SWAP: &str = "Yer değiştir";
pub const WRITE: &str = "Mozaik elemanlarını çiz";
pub const TAKE_AREAS: &str = "Seçili alanları al";
/// No layer to keep out of; the seamline layer made new.
const NONE: &str = "— yok —";

/// Where the footprints come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Prints {
    #[default]
    Bounds,
    Areas,
}

impl std::fmt::Display for Prints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Prints::Bounds => "Raster sınırları",
            Prints::Areas => "Seçili alanlar",
        })
    }
}

/// The method, in the window's words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Voronoi,
    Radiometry,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Method::Voronoi => "Eşit uzaklık (Voronoi)",
            Method::Radiometry => "Radyometrik fark",
        })
    }
}

/// Which shows under the summary: the drawing, or the seam's points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Plan,
    Points,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Plan => "Plan",
            Show::Points => "Noktalar",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's rasters (id, name) and the two chosen.
    pub rasters: Vec<(String, String)>,
    pub first: String,
    pub second: String,
    pub prints: Prints,
    /// The two areas taken from the selection, and how they are named.
    pub areas: Option<[String; 2]>,
    pub areas_name: String,
    pub method: Method,
    pub step: f64,
    pub cell: f64,
    /// Metres, typed; empty: the whole overlap.
    pub corridor: String,
    /// The layer to keep out of; empty: none.
    pub avoid: String,
    /// The drawing's layers (id, name); the one written on (empty: the seamline layer, new).
    pub layers: Vec<(String, String)>,
    pub layer: String,
    pub show: Show,
    pub answer: Option<Result<OrthophotoSeamlinesBuilt, String>>,
    /// Docs/adr/5260: the network's and the footprint's choices and plans.
    pub net: crate::ortho_network_section::NetState,
}

#[derive(Clone, Debug)]
pub enum Event {
    First(String),
    Second(String),
    Swap,
    Prints(Prints),
    TakeAreas,
    Method(Method),
    Step(f64),
    Cell(f64),
    Corridor(String),
    Avoid(String),
    Layer(String),
    Show(Show),
    Write,
    /// Docs/adr/5260: the kind, the network and the footprint.
    Net(crate::ortho_network_section::NetEvent),
    Close,
}

fn msg(e: Event) -> Message {
    Message::OrthoSeamlines(e)
}

/// A raster's name: its file's, else its asset's or address, with its pixel.
fn raster_name(r: &RasterFields) -> String {
    let a = r.affine;
    let gsd = (a[1] * a[5] - a[2] * a[4]).abs().sqrt();
    let source = r
        .file
        .as_deref()
        .map(|f| f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned())
        .or_else(|| r.asset.clone())
        .or_else(|| r.url.clone())
        .unwrap_or_else(|| "Raster".to_owned());
    format!("{source} · {:.1} cm", gsd * 100.0)
}

impl App {
    /// The window's input as it stands.
    pub(crate) fn ortho_seamlines_input(&self) -> Result<OrthophotoSeamlines, String> {
        let s = &self.ortho_seamlines;
        let footprints = match s.prints {
            Prints::Bounds => None,
            Prints::Areas => Some(s.areas.clone().ok_or("Ayak izi olarak iki alan seçip Seçili alanları al'a basın.")?),
        };
        let radiometry = s.method == Method::Radiometry;
        let corridor = match s.corridor.trim().replace(',', ".").as_str() {
            "" => None,
            t => Some(t.parse::<f64>().map_err(|_| format!("Arama bandı “{}” bir sayı değil.", s.corridor))?),
        };
        let folder = self.document.as_ref().and_then(|d| d.path.as_ref()).and_then(|p| p.parent()).map(|p| p.display().to_string());
        Ok(OrthophotoSeamlines {
            first: s.first.clone(),
            second: s.second.clone(),
            footprints,
            method: if radiometry { OrthoSeamMethod::Radiometry } else { OrthoSeamMethod::Voronoi },
            step: (!radiometry).then_some(s.step),
            cell: radiometry.then_some(s.cell.max(0.0) as u32),
            corridor: if radiometry { corridor } else { None },
            avoid: (radiometry && !s.avoid.is_empty()).then(|| s.avoid.clone()),
            folder,
            layer: (!s.layer.is_empty()).then(|| s.layer.clone()),
            expected_revision: None,
        })
    }

    fn ortho_seamlines_run(&mut self) {
        let input = match self.ortho_seamlines_input() {
            Ok(i) => i,
            Err(why) => {
                self.ortho_seamlines.answer = Some(Err(why));
                return;
            }
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.ortho_seamlines.answer = Some(match orthophoto_seamlines::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Birleşme çizgisi kurulamadı.".to_owned()),
        });
    }

    /// Mozaik birleşme çizgisi (`raster.seamlines`): the drawing's rasters, the selected two first (in the drawing's order);
    /// written on the mosaic elements' layer when the drawing has it, else on it made new.
    pub(crate) fn open_ortho_seamlines(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let rasters: Vec<(String, String)> = doc
            .model
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                Entity::Raster(r) => Some((u.to_string(), raster_name(&r.raster))),
                _ => None,
            })
            .collect();
        if rasters.is_empty() {
            self.warn("Mozaik birleşme çizgisi: çizimde ortofoto (raster) yok; önce ekleyin.");
            return Task::none();
        }
        let selected: Vec<String> = self.selection.ids().iter().filter_map(|s| doc.model.uid(*s).map(|u| u.to_string())).collect();
        let mut picked: Vec<String> = rasters.iter().filter(|(u, _)| selected.contains(u)).map(|(u, _)| u.clone()).collect();
        for (u, _) in &rasters {
            if picked.len() >= 2 {
                break;
            }
            if !picked.contains(u) {
                picked.push(u.clone());
            }
        }
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let layer = if layers.iter().any(|(id, _)| id == SEAMLINE_LAYER) { SEAMLINE_LAYER.to_owned() } else { String::new() };
        // Docs/adr/5260: one raster only: its footprint.
        let lone = picked.len() < 2;
        self.ortho_seamlines = State {
            first: picked[0].clone(),
            second: picked.get(1).cloned().unwrap_or_default(),
            net: crate::ortho_network_section::NetState {
                mode: if lone { crate::ortho_network_section::Mode::Footprint } else { Default::default() },
                ticked: rasters.iter().map(|(u, _)| u.clone()).collect(),
                raster: picked[0].clone(),
                ..Default::default()
            },
            rasters,
            layers,
            layer,
            step: 1.0,
            cell: 1.0,
            ..Default::default()
        };
        if lone {
            self.ortho_net_run();
        } else {
            self.ortho_seamlines_run();
        }
        self.dialog = Some(Dialog::OrthoSeamlines);
        Task::none()
    }

    /// The two areas (or closed polylines) selected, in the selection's order.
    fn ortho_seamlines_take(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let areas: Vec<(String, String)> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| {
                let e = doc.model.get(*s)?;
                let fits = matches!(e, Entity::Polygon(_)) || matches!(e, Entity::Polyline(p) if p.closed);
                let uid = doc.model.uid(*s)?;
                fits.then(|| (uid.to_string(), format!("#{}", s.0)))
            })
            .collect();
        if areas.len() != 2 {
            self.warn(format!("{TITLE}: ayak izi olarak iki alan (ya da kapalı çoklu çizgi) seçin; seçimde {} var.", areas.len()));
            return;
        }
        self.ortho_seamlines.areas = Some([areas[0].0.clone(), areas[1].0.clone()]);
        self.ortho_seamlines.areas_name = format!("{} ve {}", areas[0].1, areas[1].1);
        self.ortho_seamlines.prints = Prints::Areas;
    }

    fn ortho_seamlines_write(&mut self) -> Task<Message> {
        let input = match self.ortho_seamlines_input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("{TITLE}: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = orthophoto_seamlines::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let slots: Vec<kentos_domain::Slot> = self.document.as_ref().map_or_else(Vec::new, |d| {
                out.polygons
                    .iter()
                    .filter_map(|p| p.uid.as_deref().and_then(|u| kentos_domain::Uuid::parse_str(u).ok()).and_then(|u| d.model.slot_of(u)))
                    .collect()
            });
            let layer = self.ortho_seamlines.layers.iter().find(|(id, _)| *id == out.layer).map_or(SEAMLINE_LAYER_NAME, |(_, n)| n.as_str()).to_owned();
            self.selection.set(slots);
            self.say(Level::Success, format!("{TITLE}: iki mozaik elemanı “{layer}” katmanına yazıldı ve seçildi; Ortofoto karşılaştırması'nın Seçili mozaik elemanlarını al'ı onları ölçer."));
            self.dialog = None;
        }
        Task::none()
    }

    pub(crate) fn ortho_seamlines_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.ortho_seamlines;
        match event {
            Event::First(v) => s.first = v,
            Event::Second(v) => s.second = v,
            Event::Swap => {
                std::mem::swap(&mut s.first, &mut s.second);
                if let Some([a, b]) = &mut s.areas {
                    std::mem::swap(a, b);
                }
            }
            Event::Prints(p) => s.prints = p,
            Event::TakeAreas => self.ortho_seamlines_take(),
            Event::Method(m) => s.method = m,
            Event::Step(v) => s.step = v,
            Event::Cell(v) => s.cell = v,
            Event::Corridor(v) => s.corridor = v,
            Event::Avoid(v) => s.avoid = if v == NONE { String::new() } else { v },
            Event::Layer(v) => s.layer = if v == NONE { String::new() } else { v },
            Event::Show(v) => {
                s.show = v;
                return Task::none();
            }
            Event::Write => return self.ortho_seamlines_write(),
            Event::Net(e) => {
                let back = matches!(e, crate::ortho_network_section::NetEvent::Mode(crate::ortho_network_section::Mode::Pair));
                self.ortho_net_event(e);
                if !back {
                    return Task::none();
                }
            }
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.ortho_seamlines_run();
        Task::none()
    }

    /// The summary's lines and the seam's points (number, east, north, distance along the seam).
    pub(crate) fn ortho_seamlines_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &self.ortho_seamlines.answer {
            None => {}
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            Some(Ok(r)) => {
                // The colour-matching and keep-out notes are the radiometry rows' help already.
                let said = ["BÖHHBÜY m. 67(6): ortak alanda", "Geçilmeyecek alanların piksellerini"];
                for (k, n) in r.notes.iter().enumerate().filter(|(_, n)| !said.iter().any(|w| n.starts_with(w))) {
                    lines.push(words::text_line(if k == 0 { Kind::Ok } else { Kind::Info }, n.clone()));
                }
                let f = self.format();
                let mut along = 0.0;
                for (k, p) in r.seam.iter().enumerate() {
                    if k > 0 {
                        along += (p[0] - r.seam[k - 1][0]).hypot(p[1] - r.seam[k - 1][1]);
                    }
                    rows.push(vec![(k + 1).to_string(), f.coord(p[0]), f.coord(p[1]), f.length_bare(along)]);
                }
            }
        }
        (lines, rows)
    }

    pub(crate) fn ortho_seamlines_view(&self) -> Element<'_, Message> {
        let s = &self.ortho_seamlines;
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let mut avoids = vec![(NONE.to_owned(), NONE.to_owned())];
        avoids.extend(s.layers.iter().cloned());
        let mut targets = vec![(NONE.to_owned(), format!("{SEAMLINE_LAYER_NAME} (yeni)"))];
        targets.extend(s.layers.iter().cloned());
        let avoid = if s.avoid.is_empty() { NONE } else { s.avoid.as_str() };
        let target = if s.layer.is_empty() { NONE } else { s.layer.as_str() };
        let mut form = Form::new()
            .label_width(150.0)
            // The two images on one row, the swap between them (related small controls share a row).
            .field(
                "Ortofotolar",
                row![
                    container(fields::select_owned(s.rasters.clone(), &s.first, |v| msg(Event::First(v)))).width(250),
                    words::secondary(SWAP, Some(msg(Event::Swap))),
                    container(fields::select_owned(s.rasters.clone(), &s.second, |v| msg(Event::Second(v)))).width(250),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Ayak izleri",
                row![
                    Segmented::new([Prints::Bounds, Prints::Areas], s.prints, |v| msg(Event::Prints(v))),
                    words::secondary(TAKE_AREAS, Some(msg(Event::TakeAreas))),
                    label::caption(if s.prints == Prints::Areas && s.areas.is_some() { s.areas_name.clone() } else { String::new() }).style(muted),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .help("Ayak izi (TUCBS Ortogörüntü: ayakizi) görüntünün veri taşıyan bölümüdür; dışbükey ve düz kenarlı olmalı. Verilmezse rasterin sınırı alınır.")
            .field("Yöntem", Segmented::new([Method::Voronoi, Method::Radiometry], s.method, |v| msg(Event::Method(v))));
        form = if s.method == Method::Voronoi {
            form.field(
                "Eğri kesimde aralık",
                row![
                    container(NumberInput::new(s.step, |v| msg(Event::Step(v))).step(0.5).decimals(2)).width(100),
                    label::caption("m").style(muted)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .help("Birleşme çizgisi iki görüntünün kendi bölgelerinden eşit uzaktır (Pan ve ark. 2009); düz kesimleri kesin, köşe ile kenar arasındaki eğri kesimi bu aralıkla.")
        } else {
            form.field(
                "Hücre ve arama bandı",
                row![
                    container(NumberInput::new(s.cell, |v| msg(Event::Cell(v))).step(1.0).decimals(0)).width(80),
                    label::caption("piksel").style(muted),
                    container(fields::input(None, "bütün örtüşme", &s.corridor, false, false).on_input(|v| msg(Event::Corridor(v)))).width(130),
                    label::caption("m, eşit uzaklık çizgisinden en çok").style(muted),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field("Geçilmeyecek alanlar", container(fields::select_owned(avoids, avoid, |v| msg(Event::Avoid(v)))).width(300))
            .help("Birleşme çizgisi iki görüntünün radyometrik farkının en küçük olduğu yoldan geçer; seçilen katmanın alanlarını (bina, köprü) kesmez. BÖHHBÜY m. 67(6) ortak alanda radyometrik düzeltme ister; çizgiyi düzeltmeden sonra kurun.")
        };
        let (lines, rows) = self.ortho_seamlines_lines();
        let f = self.format();
        let shown: Element<'_, Message> = match (&s.answer, s.show) {
            (Some(Ok(r)), Show::Plan) => {
                let avoid = self.ortho_seamlines_avoided();
                crate::picture::picture(iced::widget::canvas(Plan { r, avoid }).width(Fill).height(Fill), TITLE)
            }
            _ => sheet(
                vec![num("No", 50.0), num(f.east_label(), 130.0), num(f.north_label(), 130.0), num("Çizgi boyunca (m)", 130.0)],
                rows,
                "Birleşme çizgisi yok.",
                240.0,
            ),
        };
        let ready = matches!(&s.answer, Some(Ok(_)));
        let write = row![
            label::caption("Yazılacak katman").style(muted),
            container(fields::select_owned(targets, target, |v| msg(Event::Layer(v)))).width(240),
            space::horizontal().width(Fill),
            words::primary(WRITE, ready.then(|| msg(Event::Write))),
        ]
        .spacing(10)
        .align_y(Center);
        // The plan takes the height the rest leaves (a resized or full screen window), at least a readable one; the
        // body scrolls in a short window and the footer keeps Kapat on the dialog's bottom edge.
        let modes = Segmented::new(
            [crate::ortho_network_section::Mode::Pair, crate::ortho_network_section::Mode::Network, crate::ortho_network_section::Mode::Footprint],
            s.net.mode,
            |v| msg(Event::Net(crate::ortho_network_section::NetEvent::Mode(v))),
        );
        if s.net.mode != crate::ortho_network_section::Mode::Pair {
            return Frame::new(TITLE)
                .scroll(Grows::new().spacing(12.0).push(modes).push(self.ortho_net_view()))
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .width(880.0)
                .modal(msg(Event::Close));
        }
        let body = Grows::new()
            .spacing(12.0)
            .push(modes)
            .push(form)
            .push(words::summary(lines))
            // Writing is its own section, above the views, so a short window never hides it.
            .push(write)
            .push(Segmented::new([Show::Plan, Show::Points], s.show, |v| msg(Event::Show(v))))
            .push_grow(shown, typography::scaled(260.0));
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(880.0)
            .modal(msg(Event::Close))
    }

    /// The rings of the layer the radiometric line keeps out of (geçilmeyecek alanlar), drawn in the plan.
    fn ortho_seamlines_avoided(&self) -> Vec<Vec<[f64; 2]>> {
        let s = &self.ortho_seamlines;
        let Some(doc) = self.document.as_ref().filter(|_| s.method == Method::Radiometry && !s.avoid.is_empty()) else {
            return Vec::new();
        };
        doc.model
            .entities()
            .filter(|e| e.base().layer_id == s.avoid)
            .filter_map(|e| match e {
                Entity::Polygon(p) => Some(p.pts.iter().map(|v| [v.x, v.y]).collect()),
                Entity::Polyline(p) if p.closed => Some(p.pts.iter().map(|v| [v.x, v.y]).collect()),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn ortho_seamlines_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SWAP) => Some(msg(Event::Swap)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(TAKE_AREAS) => Some(msg(Event::TakeAreas)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The two mosaic elements (the first's in the accent, the second's in green), the overlap dashed, the areas not to
/// cross in red and the seamline over them.
struct Plan<'a> {
    r: &'a OrthophotoSeamlinesBuilt,
    avoid: Vec<Vec<[f64; 2]>>,
}

/// A ring's area centroid (its vertices' mean when it has no area).
fn centroid(r: &[[f64; 2]]) -> [f64; 2] {
    let o = r[0];
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for k in 1..r.len().saturating_sub(1) {
        let (p, q) = ([r[k][0] - o[0], r[k][1] - o[1]], [r[k + 1][0] - o[0], r[k + 1][1] - o[1]]);
        let w = p[0] * q[1] - q[0] * p[1];
        a += w;
        cx += (p[0] + q[0]) * w;
        cy += (p[1] + q[1]) * w;
    }
    if a.abs() < 1e-12 {
        let n = r.len() as f64;
        let (sx, sy) = r.iter().fold((0.0, 0.0), |s, q| (s.0 + q[0], s.1 + q[1]));
        return [sx / n, sy / n];
    }
    [o[0] + cx / (3.0 * a), o[1] + cy / (3.0 * a)]
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let pts: Vec<[f64; 2]> = self.r.polygons.iter().flat_map(|p| p.ring.iter().copied()).collect();
        if pts.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &pts {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        let pad = 24.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let scale = (w / (x1 - x0).max(1e-9)).min(h / (y1 - y0).max(1e-9));
        let (ox, oy) = (pad + (w - (x1 - x0) * scale) / 2.0, pad + (h - (y1 - y0) * scale) / 2.0);
        let at = |p: [f64; 2]| Point::new((ox + (p[0] - x0) * scale) as f32, (oy + (y1 - p[1]) * scale) as f32);
        let ring = |r: &[[f64; 2]]| {
            Path::new(|b| {
                b.move_to(at(r[0]));
                for q in &r[1..] {
                    b.line_to(at(*q));
                }
                b.close();
            })
        };
        for (k, p) in self.r.polygons.iter().enumerate() {
            if p.ring.len() < 3 {
                continue;
            }
            let color = if k == 0 { tk.accent } else { tk.success };
            let shape = ring(&p.ring);
            frame.fill(&shape, iced::Color { a: 0.2, ..color });
            frame.stroke(&shape, Stroke::default().with_width(1.4).with_color(color));
            frame.fill_text(Text {
                content: (k + 1).to_string(),
                position: at(centroid(&p.ring)),
                color,
                size: Pixels(18.0),
                align_x: iced::widget::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
        if self.r.overlap.len() >= 3 {
            let dashed = Stroke { line_dash: canvas::LineDash { segments: &[5.0, 4.0], offset: 0 }, ..Stroke::default() };
            frame.stroke(&ring(&self.r.overlap), dashed.with_width(1.0).with_color(tk.muted));
        }
        for a in self.avoid.iter().filter(|a| a.len() >= 3) {
            let shape = ring(a);
            frame.fill(&shape, iced::Color { a: 0.3, ..tk.danger });
            frame.stroke(&shape, Stroke::default().with_width(1.2).with_color(tk.danger));
        }
        if self.r.seam.len() >= 2 {
            let seam = Path::new(|b| {
                b.move_to(at(self.r.seam[0]));
                for q in &self.r.seam[1..] {
                    b.line_to(at(*q));
                }
            });
            frame.stroke(&seam, Stroke::default().with_width(2.6).with_color(tk.warning));
            for e in &self.r.ends {
                frame.fill(&Path::circle(at(e.at), 4.0), tk.warning);
            }
        }
        vec![frame.into_geometry()]
    }
}
