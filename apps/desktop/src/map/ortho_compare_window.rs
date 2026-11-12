//! Ortofoto karşılaştırması (docs/adr/1780; `raster.compare`): two orthophotos of the drawing compared by the same
//! points read on each, by `cad.orthophoto.compare`, read only. The rasters and their reading layers on two lines (the
//! selection's rasters in the drawing's order, a layer named with a raster's year goes with it), the ground's points
//! (none: the images against each other only) and the scale on the third, the name attribute on the fourth. The
//! summary says the largest shift against the limit (2·GSD of the coarser image, BÖHHBÜY 67(6)), the systematic shift
//! and each image's own check; the table lists every point's shift and, with the ground's points, each image's error
//! (centimetres). Yer değiştir swaps the two; CSV kaydet writes the shifts. Birleşme çizgisi (docs/adr/2860) measures along
//! the one line or polyline chosen in the drawing. Kayıklık ölçümü (docs/adr/2460) measures the
//! shift in the images themselves where they overlap: its step, window, search and least correlation on one line, its
//! places in their own table (Noktalar | Kayıklık).

use std::path::PathBuf;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, Entity, OrthoSeamAsk, OrthoSubpixel, OrthophotoChecked, OrthophotoCompare, OrthophotoCompared,
    RasterFields,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, orthophoto_compare};
use kentos_ui::label;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Ortofoto karşılaştırması";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
const SWAP: &str = "Yer değiştir";
/// The seam line taken from the drawing's selection, and let go (docs/adr/2860).
const TAKE_LINE: &str = "Seçili çizgiyi al";
const TAKE_POLYGONS: &str = "Seçili mozaik elemanlarını al";
const DROP_LINE: &str = "Çizgiyi kaldır";
/// docs/adr/3660: the second resampled onto the first's grid.
const RESAMPLE: &str = "İkinciyi birincinin ızgarasına örnekle";
/// The ground's layer left out: the images are compared with each other only.
const NONE: &str = "— yok —";

/// Which table shows: the points read by hand, or the seam's places (docs/adr/2460).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Points,
    Seam,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Points => "Noktalar",
            Show::Seam => "Kayıklık",
        })
    }
}

/// How a place's peak is refined to a fraction of a pixel (docs/adr/3260): a parabola on its neighbours, or the second
/// image resampled by cubic convolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Refine {
    #[default]
    Parabola,
    Bicubic,
}

impl std::fmt::Display for Refine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Refine::Parabola => "Parabol",
            Refine::Bicubic => "Çift kübik",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's rasters: persistent id and name.
    pub rasters: Vec<(String, String)>,
    pub first: String,
    pub second: String,
    /// The drawing's layers: id and name.
    pub layers: Vec<(String, String)>,
    pub first_points: String,
    pub second_points: String,
    /// The ground's points; empty: none.
    pub known: String,
    pub field: String,
    pub scale: f64,
    pub answer: Option<Result<OrthophotoCompared, String>>,
    /// Kayıklık ölçümü (docs/adr/2460): on, its step (metres), window and search (pixels), least correlation (empty:
    /// every place counts), and which table shows.
    pub seam: bool,
    pub step: f64,
    pub window: f64,
    pub search: f64,
    pub correlation: String,
    pub show: Show,
    /// The seam line measured along (docs/adr/2860): its persistent id and its words; empty: the overlap's grid.
    pub seam_line: String,
    pub seam_line_name: String,
    /// The two images' seamline polygons measured along their shared boundary instead (docs/adr/4460): their
    /// persistent ids in the order chosen (`seam_line_name` then says so).
    pub seam_polygons: Option<[String; 2]>,
    /// How the peak is refined (docs/adr/3260).
    pub refine: Refine,
    /// The second resampled onto the first's grid when their pixels differ in size or the second is turned
    /// (docs/adr/3660).
    pub resample: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    First(String),
    Second(String),
    FirstPoints(String),
    SecondPoints(String),
    Known(String),
    Field(String),
    Scale(f64),
    Seam(bool),
    Step(f64),
    Window(f64),
    Search(f64),
    Correlation(String),
    /// The seam line: the one line or polyline chosen in the drawing taken, or let go (docs/adr/2860); the two
    /// seamline polygons chosen taken instead (docs/adr/4460).
    TakeLine,
    TakePolygons,
    DropLine,
    Refine(Refine),
    /// The second resampled onto the first's grid, or not (docs/adr/3660).
    Resample(bool),
    Show(Show),
    Swap,
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::OrthoCompare(e)
}

/// The summary's word on the resampling (docs/adr/3660): the second resampled onto the first's grid; docs/adr/4060: a
/// turned first is the grid itself. None when nothing was resampled.
pub(crate) fn resample_words(m: &kentos_commands::OrthoSeam) -> Option<&'static str> {
    m.resampled.then_some(if m.turned {
        "Birinci görüntü dönük: ikincisi onun ızgarasına kübik evrişimle yeniden örneklendi; yerler ve eşleme onun pikselleriyle, kayma doğu ve kuzey olarak."
    } else {
        "İkinci görüntü birincinin ızgarasına kübik evrişimle yeniden örneklendi; kayma birincinin pikselleriyle ölçüldü."
    })
}

fn cm(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

/// A raster's name: its file's, else its asset's or address.
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
    format!("{source} · {} cm", cm(gsd))
}

/// The first year (four digits, 19xx or 20xx, not part of a longer number) in a name.
fn year(text: &str) -> Option<&str> {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(3)).find_map(|i| {
        let digits = b[i..i + 4].iter().all(u8::is_ascii_digit);
        let alone = (i == 0 || !b[i - 1].is_ascii_digit()) && (i + 4 == b.len() || !b[i + 4].is_ascii_digit());
        (digits && alone && (b[i..].starts_with(b"19") || b[i..].starts_with(b"20"))).then(|| &text[i..i + 4])
    })
}

/// An image's own error at a point (its check's horizontal error), when the ground's points were given.
fn error_at(check: Option<&OrthophotoChecked>, name: &str) -> String {
    check.and_then(|c| c.points.iter().find(|p| p.name == name)).map(|p| cm(p.d)).unwrap_or_default()
}

/// The shifts as CSV (metres as computed).
pub fn csv(r: &OrthophotoCompared) -> String {
    let mut out = format!(
        "Kural paketi;{};Sınır (m);{};YÖA birinci (m);{};YÖA ikinci (m);{}\n\nNokta;dx (m);dy (m);Kayma (m);Sınırı aşar\n",
        r.rules, r.limit, r.gsd[0], r.gsd[1]
    );
    for p in &r.pairs {
        out.push_str(&format!("{};{};{};{};{}\n", p.name, p.dx, p.dy, p.d, if p.over { "Evet" } else { "Hayır" }));
    }
    out
}

impl App {
    fn ortho_compare_run(&mut self) {
        let s = &self.ortho_compare;
        let correlation = s.correlation.trim().replace(',', ".");
        let correlation = match correlation.as_str() {
            "" => None,
            t => match t.parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    let why = format!("En az korelasyon “{}” bir sayı değil.", s.correlation);
                    self.ortho_compare.answer = Some(Err(why));
                    return;
                }
            },
        };
        let folder = self.document.as_ref().and_then(|d| d.path.as_ref()).and_then(|p| p.parent()).map(|p| p.display().to_string());
        let seam = s.seam.then(|| OrthoSeamAsk {
            step: s.step,
            window: s.window.max(0.0) as u32,
            search: s.search.max(0.0) as u32,
            correlation,
            folder,
            line: (!s.seam_line.is_empty()).then(|| s.seam_line.clone()),
            polygons: s.seam_polygons.clone(),
            subpixel: (s.refine == Refine::Bicubic).then_some(OrthoSubpixel::Bicubic),
            resample: s.resample,
        });
        let input = OrthophotoCompare {
            first: s.first.clone(),
            second: s.second.clone(),
            first_points: s.first_points.clone(),
            second_points: s.second_points.clone(),
            known: (!s.known.is_empty()).then(|| s.known.clone()),
            field: Some(s.field.clone()),
            scale: Some(s.scale.max(1.0) as u32),
            rules: None,
            seam,
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.ortho_compare.answer = Some(match orthophoto_compare::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Karşılaştırılamadı.".to_owned()),
        });
    }

    /// Ortofoto karşılaştırması (`raster.compare`): the drawing's rasters, the selected two first (in the drawing's
    /// order); the ground's layer guessed from its name (GNSS), each image's reading layer by the year in its file's
    /// name, else the other point layers in order.
    pub(crate) fn open_ortho_compare(&mut self) -> Task<Message> {
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
        if rasters.len() < 2 {
            self.warn("Ortofoto karşılaştırması: çizimde karşılaştırılacak iki ortofoto (raster) yok; ikincisini ekleyin.");
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
        let (first, second) = (picked[0].clone(), picked[1].clone());
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let with_points: Vec<&(String, String)> = layers
            .iter()
            .filter(|(id, _)| doc.model.entities().any(|e| e.base().layer_id == *id && matches!(e, Entity::Point(_))))
            .collect();
        let known = with_points
            .iter()
            .find(|(_, n)| ["gnss", "arazi", "denetim"].iter().any(|w| n.to_lowercase().contains(w)))
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        // The layers with points, else (a seam measured alone, docs/adr/2460) every layer but the rasters'.
        let raster_layers: Vec<String> = doc
            .model
            .entities()
            .filter(|e| matches!(e, Entity::Raster(_)))
            .map(|e| e.base().layer_id.clone())
            .collect();
        let mut readings: Vec<&String> = with_points.iter().map(|(id, _)| id).filter(|id| **id != known).collect();
        if readings.is_empty() {
            readings = layers.iter().map(|(id, _)| id).filter(|id| **id != known && !raster_layers.contains(id) && id.as_str() != "0").collect();
        }
        let name_of = |uid: &str| rasters.iter().find(|(u, _)| u == uid).map(|(_, n)| n.clone()).unwrap_or_default();
        let reading_for = |raster: &str, taken: &str| -> String {
            let free: Vec<&String> = readings.iter().copied().filter(|id| id.as_str() != taken).collect();
            let by_year = year(&name_of(raster)).and_then(|y| {
                free.iter().copied().find(|id| {
                    id.contains(y) || layers.iter().any(|(l, n)| l.as_str() == id.as_str() && n.contains(y))
                })
            });
            by_year.or_else(|| free.first().copied()).cloned().unwrap_or_default()
        };
        let first_points = reading_for(&first, "");
        let second_points = reading_for(&second, &first_points);
        self.ortho_compare = State {
            rasters,
            first,
            second,
            layers,
            first_points,
            second_points,
            known,
            field: "Ad".into(),
            scale: doc.model.settings().plot_scale.round().max(1.0),
            answer: None,
            seam: false,
            step: 10.0,
            window: 21.0,
            search: 5.0,
            correlation: String::new(),
            show: Show::Points,
            seam_line: String::new(),
            seam_line_name: String::new(),
            seam_polygons: None,
            refine: Refine::Parabola,
            resample: false,
        };
        self.ortho_compare_run();
        self.dialog = Some(Dialog::OrthoCompare);
        Task::none()
    }

    /// The shifts written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn ortho_compare_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.ortho_compare.answer else { return };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} nokta {} dosyasına yazıldı.", r.pairs.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}.", path.display())),
        }
    }

    pub(crate) fn ortho_compare_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.ortho_compare;
        match event {
            Event::First(v) => s.first = v,
            Event::Second(v) => s.second = v,
            Event::FirstPoints(v) => s.first_points = v,
            Event::SecondPoints(v) => s.second_points = v,
            Event::Known(v) => s.known = if v == NONE { String::new() } else { v },
            Event::Field(v) => s.field = v,
            Event::Scale(v) => s.scale = v,
            Event::Seam(on) => {
                s.seam = on;
                s.show = if on { Show::Seam } else { Show::Points };
            }
            Event::Step(v) => s.step = v,
            Event::Window(v) => s.window = v,
            Event::Search(v) => s.search = v,
            Event::Correlation(v) => s.correlation = v,
            Event::Refine(v) => s.refine = v,
            Event::Resample(on) => s.resample = on,
            Event::TakeLine => {
                // The one line or polyline chosen (docs/adr/2860): its persistent id and words.
                let chosen = self.selection.ids().to_vec();
                let picked = self.document.as_ref().and_then(|doc| match chosen.as_slice() {
                    [slot] => match doc.model.get(*slot) {
                        Some(kentos_commands::Entity::Line(_)) => Some(("Çizgi", *slot)),
                        Some(kentos_commands::Entity::Polyline(_)) => Some(("Çoklu çizgi", *slot)),
                        _ => None,
                    }
                    .and_then(|(kind, slot)| doc.model.uid(slot).map(|u| (kind, u.to_string()))),
                    _ => None,
                });
                match picked {
                    Some((kind, uid)) => {
                        let s = &mut self.ortho_compare;
                        s.seam_line = uid;
                        s.seam_line_name = kind.to_owned();
                        s.seam_polygons = None;
                    }
                    None => {
                        self.warn("Birleşme çizgisi için çizimde tek bir çizgi ya da çoklu çizgi seçin.".to_owned());
                        return Task::none();
                    }
                }
            }
            Event::TakePolygons => {
                // The two areas or closed polylines chosen (docs/adr/4460): their persistent ids in the order chosen.
                let chosen = self.selection.ids().to_vec();
                let picked = self.document.as_ref().and_then(|doc| {
                    let seamline = |slot: kentos_domain::Slot| match doc.model.get(slot) {
                        Some(kentos_commands::Entity::Polygon(_)) => doc.model.uid(slot).map(|u| u.to_string()),
                        Some(kentos_commands::Entity::Polyline(p)) if p.closed => doc.model.uid(slot).map(|u| u.to_string()),
                        _ => None,
                    };
                    match chosen.as_slice() {
                        [a, b] => Some([seamline(*a)?, seamline(*b)?]),
                        _ => None,
                    }
                });
                match picked {
                    Some(pair) => {
                        let s = &mut self.ortho_compare;
                        s.seam_polygons = Some(pair);
                        s.seam_line.clear();
                        s.seam_line_name = "İki mozaik elemanı".to_owned();
                    }
                    None => {
                        self.warn(
                            "Mozaik elemanları için çizimde iki alan ya da kapalı çoklu çizgi seçin: iki görüntünün mozaikte kullanılan bölümleri."
                                .to_owned(),
                        );
                        return Task::none();
                    }
                }
            }
            Event::DropLine => {
                s.seam_line.clear();
                s.seam_line_name.clear();
                s.seam_polygons = None;
            }
            Event::Show(v) => {
                s.show = v;
                return Task::none();
            }
            Event::Swap => {
                std::mem::swap(&mut s.first, &mut s.second);
                std::mem::swap(&mut s.first_points, &mut s.second_points);
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("ortofoto-karsilastirmasi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.ortho_compare_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Close => {
                self.ortho_compare = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.ortho_compare_run();
        Task::none()
    }

    pub(crate) fn ortho_compare_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let Some(answer) = &self.ortho_compare.answer else { return (lines, rows) };
        let r = match answer {
            Ok(r) => r,
            Err(why) => {
                lines.push(words::text_line(Kind::Error, why.clone()));
                return (lines, rows);
            }
        };
        let coarse = r.gsd[0].max(r.gsd[1]);
        let times = if coarse > 0.0 { format!("{}", (r.limit / coarse * 10.0).round() / 10.0) } else { "?".into() };
        let over = r.pairs.iter().filter(|p| p.over).count();
        lines.push(match r.passed {
            Some(true) => words::text_line(
                Kind::Ok,
                format!(
                    "{} ortak nokta: en büyük kayma {} cm, sınır {} cm ({times}·YÖA, iri olanınki {} cm; BÖHHBÜY m. 67(6)): sınırın içinde.",
                    r.pairs.len(),
                    cm(r.shift.max_d),
                    cm(r.limit),
                    cm(coarse)
                ),
            ),
            Some(false) => words::text_line(
                Kind::Error,
                format!(
                    "{} ortak nokta: {over} nokta sınırı ({} cm, {times}·YÖA; BÖHHBÜY m. 67(6)) aşıyor; en büyük kayma {} cm.",
                    r.pairs.len(),
                    cm(r.limit),
                    cm(r.shift.max_d)
                ),
            ),
            None => words::text_line(Kind::Warn, "Eşleşen nokta yok: iki görüntüde aynı adla okunmuş nokta bulunamadı."),
        });
        if !r.pairs.is_empty() {
            lines.push(words::text_line(
                Kind::Info,
                format!(
                    "Ortalama kayma doğu {} cm, kuzey {} cm (görüntüler arasındaki sistematik fark); kaymaların KOH_X,Y'si {} cm.",
                    cm(r.shift.mean_x),
                    cm(r.shift.mean_y),
                    cm(r.shift.koh_xy)
                ),
            ));
        }
        lines.push(words::text_line(
            Kind::Info,
            format!("Yer örnekleme aralığı: birinci {} cm, ikinci {} cm.", cm(r.gsd[0]), cm(r.gsd[1])),
        ));
        for (which, check) in [("Birinci", &r.first_check), ("İkinci", &r.second_check)] {
            let Some(c) = check else { continue };
            lines.push(match c.passed_xy {
                Some(ok) => words::text_line(
                    if ok { Kind::Ok } else { Kind::Error },
                    format!(
                        "{which} görüntü arazi noktalarıyla: KOH_X,Y {} cm, sınır {} cm (BÖHHBÜY m. 91, 1/{}): {}.",
                        cm(c.check.koh_xy),
                        cm(c.limit_xy),
                        c.scale,
                        if ok { "sınırın içinde" } else { "sınırın dışında" }
                    ),
                ),
                None => words::text_line(Kind::Warn, format!("{which} görüntü: bağımsız denetim noktası yok, doğruluğu söylenemez.")),
            });
        }
        if r.first_check.is_none() {
            lines.push(words::text_line(Kind::Info, "Arazi noktaları verilmedi: görüntülerin kendi doğruluğu denetlenmedi."));
        }
        if !r.unmatched.is_empty() {
            lines.push(words::text_line(Kind::Warn, format!("Eşi bulunmayan noktalar: {}.", r.unmatched.join(", "))));
        }
        if let Some(m) = &r.seam {
            // Along a seam line (docs/adr/2860): its places in the overlap and those outside it.
            let where_ = match &m.line {
                Some(l) => format!(
                    "birleşme çizgisi boyunca ({}) {} yer, {} yer örtüşmenin dışında",
                    self.format().length(l.length),
                    m.patches.len(),
                    l.outside
                ),
                // docs/adr/4460: along the seamline polygons' shared boundary.
                None => match &m.shared {
                    Some(sh) => format!(
                        "mozaik elemanlarının ortak sınırı boyunca ({}, {} parça) {} yer, {} yer örtüşmenin dışında",
                        self.format().length(sh.length),
                        sh.parts.len(),
                        m.patches.len(),
                        sh.outside
                    ),
                    None => format!("{} yer", m.patches.len()),
                },
            };
            let counted = format!(
                "Kayıklık ölçümü: {where_}; {} sayıldı, {} korelasyonu düşük, {} tepe arama sınırında, {} dokusuz.",
                m.ok,
                m.low,
                m.edge,
                m.flat
            );
            lines.push(match (&m.shift, m.passed) {
                (Some(st), Some(ok)) => words::text_line(
                    if ok { Kind::Ok } else { Kind::Error },
                    format!(
                        "{counted} Ortalama kayma doğu {} cm, kuzey {} cm; en büyük {} cm, sınır {} cm: {}.",
                        cm(st.mean_x),
                        cm(st.mean_y),
                        cm(st.max_d),
                        cm(r.limit),
                        if ok { "sınırın içinde" } else { "sınırı aşan yer var" }
                    ),
                ),
                _ => words::text_line(Kind::Warn, format!("{counted} Sayılan yer yok: pencereyi, aramayı ya da korelasyon sınırını değiştirin.")),
            });
            if let Some(said) = resample_words(m) {
                lines.push(words::text_line(Kind::Info, said));
            }
        }
        lines.push(words::text_line(
            Kind::Info,
            "İki zamanın farkı kendiliğinden deformasyon değildir; görüntülerin kendi doğruluğuyla birlikte okunur.",
        ));
        if self.ortho_compare.show == Show::Seam {
            let f = self.format();
            // Docs/adr/4460: with more than one part of the shared boundary, each distance after its part's number.
            let parts = r.seam.as_ref().and_then(|m| m.shared.as_ref()).map_or(0, |sh| sh.parts.len());
            for p in r.seam.iter().flat_map(|m| &m.patches) {
                let o = |v: Option<f64>| v.map(cm).unwrap_or_default();
                let station = p.station.map(|v| f.length_bare(v)).unwrap_or_default();
                rows.push(vec![
                    // Docs/adr/2860: the distance along the seam line (the grid's places: none).
                    match p.part {
                        Some(n) if parts > 1 => format!("{} · {station}", n + 1),
                        _ => station,
                    },
                    f.coord(p.at[0]),
                    f.coord(p.at[1]),
                    o(p.dx),
                    o(p.dy),
                    o(p.d),
                    p.correlation.map(|c| format!("{c:.3}")).unwrap_or_default(),
                    match (p.state.as_str(), p.over) {
                        ("ok", true) => "Sınırı aşar",
                        ("ok", false) => "İçinde",
                        ("low", _) => "Korelasyon düşük",
                        ("edge", _) => "Tepe sınırda",
                        _ => "Dokusuz",
                    }
                    .to_owned(),
                ]);
            }
            return (lines, rows);
        }
        for p in &r.pairs {
            rows.push(vec![
                p.name.clone(),
                cm(p.dx),
                cm(p.dy),
                cm(p.d),
                error_at(r.first_check.as_ref(), &p.name),
                error_at(r.second_check.as_ref(), &p.name),
                if p.over { "Sınırı aşar".to_owned() } else { "İçinde".to_owned() },
            ]);
        }
        (lines, rows)
    }

    pub(crate) fn ortho_compare_view(&self) -> Element<'_, Message> {
        let s = &self.ortho_compare;
        let caption = |text: &'static str| container(label::caption(text)).width(120);
        let mut grounds = vec![(NONE.to_owned(), NONE.to_owned())];
        grounds.extend(s.layers.iter().cloned());
        let known = if s.known.is_empty() { NONE } else { s.known.as_str() };
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Birinci ortofoto",
                row![
                    container(fields::select_owned(s.rasters.clone(), &s.first, |v| msg(Event::First(v)))).width(260),
                    caption("Okunan noktalar"),
                    container(fields::select_owned(s.layers.clone(), &s.first_points, |v| msg(Event::FirstPoints(v)))).width(220)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "İkinci ortofoto",
                row![
                    container(fields::select_owned(s.rasters.clone(), &s.second, |v| msg(Event::Second(v)))).width(260),
                    caption("Okunan noktalar"),
                    container(fields::select_owned(s.layers.clone(), &s.second_points, |v| msg(Event::SecondPoints(v)))).width(220)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Arazi noktaları",
                row![
                    container(fields::select_owned(grounds, known, |v| msg(Event::Known(v)))).width(260),
                    caption("Ölçek 1/"),
                    container(NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(500.0).decimals(0)).width(220)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Nokta adı alanı",
                container(fields::input(None, "Ad", &s.field, false, false).on_input(|v| msg(Event::Field(v)))).width(260),
            )
            .field(
                "Kayıklık ölçümü",
                row![
                    fields::check(
                        if s.seam { Check::Checked } else { Check::Unchecked },
                        "Görüntülerden ölç".to_owned(),
                        Some(msg(Event::Seam(!s.seam))),
                    ),
                    label::caption("adım (m)"),
                    container(NumberInput::new(s.step, |v| msg(Event::Step(v))).step(1.0).decimals(1)).width(80),
                    label::caption("pencere / arama (px)"),
                    container(NumberInput::new(s.window, |v| msg(Event::Window(v))).step(2.0).decimals(0)).width(70),
                    container(NumberInput::new(s.search, |v| msg(Event::Search(v))).step(1.0).decimals(0)).width(70),
                    label::caption("en az korelasyon"),
                    container(fields::input(None, "—", &s.correlation, false, false).on_input(|v| msg(Event::Correlation(v)))).width(70)
                ]
                .spacing(8)
                .align_y(Center),
            )
            // Docs/adr/2860: along a drawn seam line, else the overlap's grid.
            .field(
                "Birleşme çizgisi",
                row![
                    container(fields::input(
                        None,
                        "Örtüşmenin bütünü",
                        &s.seam_line_name,
                        false,
                        false
                    ))
                    .width(160),
                    words::secondary(TAKE_LINE, s.seam.then(|| msg(Event::TakeLine))),
                    words::secondary(TAKE_POLYGONS, s.seam.then(|| msg(Event::TakePolygons))),
                    words::secondary(
                        DROP_LINE,
                        (s.seam && (!s.seam_line.is_empty() || s.seam_polygons.is_some())).then(|| msg(Event::DropLine))
                    )
                ]
                .spacing(8)
                .align_y(Center),
            )
            .help("Yerler seçili çizgi boyunca ya da iki görüntünün mozaik elemanlarının ortak sınırı (birleşme çizgisi) boyunca; ikisi de yoksa örtüşmenin bütününde.")
            // Docs/adr/3260: the peak refined by a parabola, or by the second image resampled (cubic convolution).
            .field("Piksel altı", Segmented::new([Refine::Parabola, Refine::Bicubic], s.refine, |v| msg(Event::Refine(v))))
            // Docs/adr/3660: a second of other pixels or turned, resampled onto the first's grid.
            .field(
                "Yeniden örnekleme",
                fields::check(
                    if s.resample { Check::Checked } else { Check::Unchecked },
                    RESAMPLE.to_owned(),
                    s.seam.then(|| msg(Event::Resample(!s.resample))),
                ),
            )
            .help("Pikselleri farklı boyda ya da dönük ikinci görüntü kübik evrişimle birincinin ızgarasına örneklenir; birinci dönükse ızgara onun pikselleridir.");
        let (lines, rows) = self.ortho_compare_lines();
        let f = self.format();
        let table = if s.show == Show::Seam {
            sheet(
                vec![
                    num("Hat (m)", 70.0),
                    num(f.east_label(), 110.0),
                    num(f.north_label(), 110.0),
                    num("dx (cm)", 70.0),
                    num("dy (cm)", 70.0),
                    num("Kayma (cm)", 85.0),
                    num("Korelasyon", 85.0),
                    col("Durum", 130.0),
                ],
                rows,
                "Kayıklık ölçümü yok.",
                240.0,
            )
        } else {
            sheet(
                vec![
                    col("Nokta", 90.0),
                    num("dx (cm)", 80.0),
                    num("dy (cm)", 80.0),
                    num("Kayma (cm)", 95.0),
                    num("1. hata (cm)", 95.0),
                    num("2. hata (cm)", 95.0),
                    col("Durum", 110.0),
                ],
                rows,
                "Eşleşen nokta yok.",
                240.0,
            )
        };
        let mut body = Column::new().spacing(12).push(form).push(words::summary(lines));
        if s.seam {
            body = body.push(Segmented::new([Show::Points, Show::Seam], s.show, |v| msg(Event::Show(v))));
        }
        let body = body.push(table);
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.pairs.is_empty());
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SWAP, Some(msg(Event::Swap))))
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn ortho_compare_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(SWAP) => Some(msg(Event::Swap)),
            Control::Press(TAKE_LINE) => Some(msg(Event::TakeLine)),
            Control::Press(TAKE_POLYGONS) => Some(msg(Event::TakePolygons)),
            Control::Press(DROP_LINE) => Some(msg(Event::DropLine)),
            Control::Check(RESAMPLE, on) => Some(msg(Event::Resample(on))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::year;

    #[test]
    fn a_year_is_four_digits_alone() {
        assert_eq!(year("ortofoto-2024.tif"), Some("2024"));
        assert_eq!(year("okuma-2026"), Some("2026"));
        assert_eq!(year("pafta-120245"), None);
        assert_eq!(year("1/1000"), None);
        assert_eq!(year("1999"), Some("1999"));
        assert_eq!(year(""), None);
    }
}
