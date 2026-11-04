//! Menfez (plan/road.md Y11.2, docs/adr/0909 §2, `culvert.editor`): a road project's culverts across a corridor. The
//! barrel's sizes are the project's; KentOS fills in none. The culvert is read as it is typed (`road.culverts.edit`'s
//! plan on a reading copy): its barrel along its line between the fill's toes, its flow line falling to the outlet, its
//! ends' elevations, the least cover and the quantities. Kaydet writes it (one undo step “Menfez”), Planda çiz draws its
//! barrel with its km on a layer (one step “Menfez çiz”).
//!
//! Hidrolik (plan/road.md Y41, docs/adr/0947) reads the saved culvert at a flow (`road.culverts.hydraulics`): the water
//! before its inlet by HDS-5's inlet and outlet control, the road as a weir, the water over the road and its hazard
//! class; as HY-8 shows a crossing, a performance curve (the water's level against the flow, from 0 to 1.5 × the flow)
//! beside its table. The inlet's row, ke, n and the weir's Cd and length are the user's; KentOS gives none.
//!
//! ```text
//! ┌ Menfez ───────────────────────────────────────────────────────────────────────────────────────┐
//! │ Menfez [▾] Ad [ ] Koridor [▾] Km (m) [ ] Açı (°) [ ] Akış kotu (m) [ ] Eğim (%) [ ] Akış [▾]     │
//! │ Kesit [Boru ▾]  Çap (m) [ ] Et (m) [ ] Sayı [ ] Aralık (m) [ ]  | Uzatma (m) [ ] En az örtü (m) [ ]│
//! │ L … m · genişlik … m · örtü en az … m · beton … m³ · boru … m · warnings                        │
//! │ ┌ the barrel along its line ┐  Uç  Öteleme  Y  X  Akış kotu                                      │
//! │ └───────────────────────────┘                                                                    │
//! │ Hidrolik: Debi [ ] Kuyruk suyu [ ] Giriş [▾] ke [ ] n [ ] Savak Cd [ ] Savak boyu [ ] Kret kotu [ ]   │
//! │ HW … m (kot …) · giriş/çıkış kontrolü · yoldan … m³/s, H…                                          │
//! │ ┌ level against flow: inlet, outlet, crest ┐  Toplam  Menfez  Yol  Su kotu  Giriş  Çıkış  Kontrol  T │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]        │
//! └───────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, CulvertFlow, CulvertHydraulicsRead, CulvertInlet, CulvertReport, CulvertShapeDef, CulvertWeir,
    CulvertsDraw, CulvertsEdit, CulvertsHydraulics, RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_culverts_draw, road_culverts_edit, road_culverts_hydraulics};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab, Tabs};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Menfez";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
/// The typed fields, in order: the culvert's, then the pipe's, then the box's.
pub(crate) const FIELDS: [&str; 16] = [
    "Ad",
    "Km (m)",
    "Açı (°)",
    "Akış kotu (m)",
    "Eğim (%)",
    "Uzatma (m)",
    "En az örtü (m)",
    "Malzeme",
    "Çap (m)",
    "Et (m)",
    "Boru sayısı",
    "Aralık (m)",
    "Göz genişliği (m)",
    "Göz yüksekliği (m)",
    "Duvar (m)",
    "Döşeme (m)",
];
const CELLS: &str = "Göz sayısı";
/// The pages under the culvert's sizes: its barrel, its hydraulics.
const PAGES: [&str; 2] = ["Gövde", "Hidrolik"];
/// The hydraulics' typed fields, in order.
pub(crate) const HYDRAULICS: [&str; 7] =
    ["Debi (m³/s)", "Kuyruk suyu (m)", "Giriş kaybı ke", "Manning n", "Savak Cd", "Savak boyu (m)", "Kret kotu (m)"];
/// The performance curve's points.
const CURVE: u32 = 13;
/// HDS-5's inlets KentOS names, the pipes' then the boxes', in words.
const INLETS: [(CulvertInlet, &str); 7] = [
    (CulvertInlet::PipeSquareHeadwall, "Düz ağız, başlık duvarı"),
    (CulvertInlet::PipeGrooveHeadwall, "Oluklu ağız, başlık duvarı"),
    (CulvertInlet::PipeGrooveProjecting, "Oluklu ağız, çıkıntılı"),
    (CulvertInlet::BoxWingwall30To75, "30°–75° kanat duvarı"),
    (CulvertInlet::BoxWingwall90And15, "90° ve 15° kanat duvarı"),
    (CulvertInlet::BoxWingwall0, "Kanatsız (0°)"),
    (CulvertInlet::BoxHeadwallChamfer, "90° başlık, pahlı kenar"),
];

/// The inlets of a pipe's or a box's.
fn inlets(pipe: bool) -> &'static [(CulvertInlet, &'static str)] {
    if pipe { &INLETS[..3] } else { &INLETS[3..] }
}

/// The culvert as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<CulvertReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The barrel's top above its flow line.
    pub(crate) top: f64,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) chosen: Option<String>,
    pub(crate) corridor: Option<String>,
    pub(crate) pipe: bool,
    pub(crate) flow: CulvertFlow,
    pub(crate) fields: [String; 16],
    pub(crate) cells: String,
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
    /// The page shown: 0 the barrel, 1 the hydraulics.
    pub(crate) page: usize,
    pub(crate) inlet: CulvertInlet,
    pub(crate) hydraulic: [String; 7],
    /// The saved culvert's hydraulics as last read: none until a flow is typed.
    pub(crate) hydraulics: Option<Result<(CulvertHydraulicsRead, Vec<CommandWarning>), String>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    Corridor(String),
    Pipe(bool),
    Flow(CulvertFlow),
    Field(usize, String),
    Cells(String),
    Layer(String),
    Page(usize),
    Inlet(CulvertInlet),
    Hydraulic(usize, String),
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Culverts(event)
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
    /// The hydraulics' question as typed: none without a flow, else it or why it is not one.
    pub(crate) fn hydraulics_input(&self) -> Option<Result<CulvertsHydraulics, String>> {
        let id = self.chosen.clone()?;
        let f = &self.hydraulic;
        let flow = match number(&f[0], HYDRAULICS[0]) {
            Ok(None) => return None,
            Ok(Some(q)) => q,
            Err(e) => return Some(Err(e)),
        };
        Some((|| {
            let ke = number(&f[2], HYDRAULICS[2])?.ok_or_else(|| format!("{} yazılmalı; KentOS vermez.", HYDRAULICS[2]))?;
            let (cd, length) = (number(&f[4], HYDRAULICS[4])?, number(&f[5], HYDRAULICS[5])?);
            let weir = match (cd, length) {
                (None, None) => None,
                (Some(cd), Some(length)) => Some(CulvertWeir { cd, length, crest: number(&f[6], HYDRAULICS[6])? }),
                _ => return Err("Yolu savak saymak için savak Cd ve savak boyu birlikte yazılmalı.".into()),
            };
            Ok(CulvertsHydraulics {
                culvert: id,
                flow,
                tailwater: number(&f[1], HYDRAULICS[1])?,
                inlet: self.inlet,
                entrance_loss: ke,
                roughness: number(&f[3], HYDRAULICS[3])?,
                weir,
                curve: Some(CURVE),
            })
        })())
    }

    /// The edit as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<CulvertsEdit, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        let count = |t: &str, what: &str| -> Result<u32, String> {
            t.trim().parse::<u32>().map_err(|_| format!("{what} bir tam sayı olmalı."))
        };
        let shape = if self.pipe {
            CulvertShapeDef::Pipe {
                diameter: need(8)?,
                thickness: need(9)?,
                count: count(&self.fields[10], FIELDS[10])?,
                spacing: number(&self.fields[11], FIELDS[11])?.unwrap_or(0.0),
            }
        } else {
            CulvertShapeDef::Box {
                width: need(12)?,
                height: need(13)?,
                thickness: need(14)?,
                slab: need(15)?,
                cells: count(&self.cells, CELLS)?,
            }
        };
        let material = self.fields[7].trim();
        Ok(CulvertsEdit {
            operation: RoadEditOperation::Set,
            culvert: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            corridor: Some(self.corridor.clone().ok_or("Bir koridor seçin.")?),
            station: Some(need(1)?),
            skew: Some(number(&self.fields[2], FIELDS[2])?.unwrap_or(0.0)),
            shape: Some(shape),
            invert: Some(need(3)?),
            slope: Some(need(4)?),
            flow: Some(self.flow),
            extend: Some(number(&self.fields[5], FIELDS[5])?.unwrap_or(0.0)),
            cover: Some(number(&self.fields[6], FIELDS[6])?),
            material: Some((!material.is_empty()).then(|| material.to_owned())),
            roughness: None,
            expected_revision: None,
        })
    }
}

impl App {
    /// Menfez: the window on the road project's first culvert, or a new one on its first corridor.
    pub(crate) fn open_culverts(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Menfez yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first() else {
            self.warn("Menfez bir koridorun altından geçer: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        let corridor = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_culverts().first().map(|c| c.id.clone());
        self.culvert_window = Some(Box::new(Window {
            chosen: None,
            corridor: Some(corridor),
            pipe: true,
            flow: CulvertFlow::Right,
            fields: Default::default(),
            cells: "1".into(),
            layer,
            preview: Preview::default(),
            page: 0,
            inlet: CulvertInlet::PipeSquareHeadwall,
            hydraulic: Default::default(),
            hydraulics: None,
        }));
        self.dialog = Some(Dialog::Culverts);
        let _ = self.culvert_event(Event::Choose(chosen));
    }

    fn refresh_culvert(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.culvert_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| {
            let top = match q.shape.as_ref() {
                Some(CulvertShapeDef::Pipe { diameter, thickness, .. }) => diameter + thickness,
                Some(CulvertShapeDef::Box { height, slab, .. }) => height + slab,
                None => 0.0,
            };
            road_culverts_edit::preview(&doc.model, &q).map(|r| (r, top))
        }) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok(((report, warnings), top)) => Preview { report: Some(report), warnings, why: None, top },
        };
        if !inlets(w.pipe).iter().any(|(i, _)| *i == w.inlet) {
            w.inlet = inlets(w.pipe)[0].0;
        }
        w.hydraulics = w
            .hydraulics_input()
            .map(|asked| asked.and_then(|q| road_culverts_hydraulics::preview(&doc.model, &q)));
    }

    pub(crate) fn culvert_event(&mut self, event: Event) -> Task<Message> {
        let kept = self.document.as_ref().map(|d| d.model.road_culverts().to_vec()).unwrap_or_default();
        let Some(w) = self.culvert_window.as_mut() else { return Task::none() };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                let mut f: [String; 16] = Default::default();
                match id.as_ref().and_then(|id| kept.iter().find(|c| &c.id == id)) {
                    Some(c) => {
                        w.corridor = Some(c.corridor.clone());
                        w.flow = c.flow;
                        f[0] = c.name.clone();
                        f[1] = trimmed(c.station);
                        f[2] = if c.skew == 0.0 { String::new() } else { trimmed(c.skew) };
                        f[3] = trimmed(c.invert);
                        f[4] = trimmed(c.slope);
                        f[5] = if c.extend == 0.0 { String::new() } else { trimmed(c.extend) };
                        f[6] = c.cover.map(trimmed).unwrap_or_default();
                        f[7] = c.material.clone().unwrap_or_default();
                        match c.shape {
                            CulvertShapeDef::Pipe { diameter, thickness, count, spacing } => {
                                w.pipe = true;
                                f[8] = trimmed(diameter);
                                f[9] = trimmed(thickness);
                                f[10] = count.to_string();
                                f[11] = if spacing == 0.0 { String::new() } else { trimmed(spacing) };
                            }
                            CulvertShapeDef::Box { width, height, thickness, slab, cells } => {
                                w.pipe = false;
                                f[12] = trimmed(width);
                                f[13] = trimmed(height);
                                f[14] = trimmed(thickness);
                                f[15] = trimmed(slab);
                                w.cells = cells.to_string();
                            }
                        }
                    }
                    None => {
                        // A new one: its sizes are the user's; only the name and one barrel are filled in.
                        let n = kept.len() + 1;
                        f[0] = if n == 1 { "Menfez".into() } else { format!("Menfez {n}") };
                        f[10] = "1".into();
                        w.cells = "1".into();
                    }
                }
                w.fields = f;
            }
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Pipe(p) => w.pipe = p,
            Event::Flow(f) => w.flow = f,
            Event::Field(k, t) => w.fields[k.min(15)] = t,
            Event::Cells(t) => w.cells = t,
            Event::Layer(l) => w.layer = Some(l),
            Event::Page(p) => w.page = p.min(1),
            Event::Inlet(i) => w.inlet = i,
            Event::Hydraulic(k, t) => w.hydraulic[k.min(6)] = t,
            Event::Save => return self.save_culvert(),
            Event::Draw => return self.draw_culvert(),
            Event::Remove => return self.remove_culvert(),
            Event::Close => {
                self.culvert_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_culvert();
        Task::none()
    }

    fn save_culvert(&mut self) -> Task<Message> {
        let Some(w) = self.culvert_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Menfez yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_culverts_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(c) = &out.culvert {
            self.say(Level::Success, format!("“{}”: {:.2} m gövde, en az örtü {:.2} m.", c.name, c.length, c.cover));
            let id = c.id.clone();
            return self.culvert_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    fn draw_culvert(&mut self) -> Task<Message> {
        let Some(w) = self.culvert_window.as_ref() else { return Task::none() };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce menfezi kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = CulvertsDraw { culvert: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_culverts_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("Menfez planda, {:.2} m.", out.length));
            self.culvert_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_culvert(&mut self) -> Task<Message> {
        let Some(id) = self.culvert_window.as_ref().and_then(|w| w.chosen.clone()) else { return Task::none() };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = CulvertsEdit {
            operation: RoadEditOperation::Remove,
            culvert: Some(id),
            name: None,
            corridor: None,
            station: None,
            skew: None,
            shape: None,
            invert: None,
            slope: None,
            flow: None,
            extend: None,
            cover: None,
            material: None,
            roughness: None,
            expected_revision: None,
        };
        let result = road_culverts_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_culverts().first().map(|c| c.id.clone()));
            return self.culvert_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn culvert_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.culvert_window else {
            return iced::widget::text("").into();
        };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni menfez".into())];
        listed.extend(
            model.map_or_else(Vec::new, |m| m.road_culverts().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()),
        );
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_culvert = Select::new(
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
        );
        let on_shape = Select::new(["Boru", "Kutu"].map(Choice::new), Some(usize::from(!w.pipe)), |i| msg(Event::Pipe(i == 0)));
        let flows = [CulvertFlow::Right, CulvertFlow::Left];
        let on_flow = Select::new(["Sağa", "Sola"].map(Choice::new), flows.iter().position(|f| *f == w.flow), move |i| {
            msg(Event::Flow(flows[i]))
        });
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Menfez", container(on_culvert).width(170), None),
            field("Menfez", 0, 150.0),
            words::field("Koridor", container(on_corridor).width(150), None),
            field("m", 1, 80.0),
            field("0", 2, 60.0),
            field("m", 3, 90.0),
            field("%", 4, 60.0),
            words::field("Akış", container(on_flow).width(80), None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut sizes = row![words::field("Kesit", container(on_shape).width(90), None)].spacing(10);
        if w.pipe {
            sizes = sizes.push(field("m", 8, 70.0)).push(field("m", 9, 60.0)).push(field("1", 10, 60.0)).push(field("0", 11, 60.0));
        } else {
            sizes = sizes
                .push(field("m", 12, 80.0))
                .push(field("m", 13, 80.0))
                .push(field("m", 14, 60.0))
                .push(field("m", 15, 60.0))
                .push(words::field(
                    CELLS,
                    kentos_ui::widget::text_field("1", &w.cells).on_input(|t| msg(Event::Cells(t))).width(60),
                    None,
                ));
        }
        sizes = sizes.push(field("0", 5, 70.0)).push(field("yok", 6, 80.0)).push(field("", 7, 110.0));
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top).push(sizes.align_y(iced::alignment::Vertical::Bottom));
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "km {} · gövde {:.2} m, dış genişlik {:.2} m · en az örtü {:.2} m ({:.2} m ötelemede) · beton {:.2} m³{}",
                    r.km_text,
                    r.length,
                    r.width,
                    r.cover,
                    r.cover_offset,
                    r.concrete,
                    if r.pipes > 0.0 { format!(" · boru {:.2} m", r.pipes) } else { String::new() }
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                body = body.push(Tabs::new(PAGES.map(|t| Tab::new(t).closable(false)), w.page, |i| msg(Event::Page(i))));
                if w.page == 1 {
                    body = self.hydraulics_view(w, body);
                    return self.culvert_frame(w, body);
                }
                let picture = crate::picture::picture(iced::widget::canvas(Barrel { report: r, top: w.preview.top }).width(Fill).height(Fill), TITLE);
                use crate::road_dialog::{col, grouped, num};
                let list = crate::road_dialog::sheet_fill(
                    vec![col("Uç", 110.0), num("Öteleme (m)", 96.0), num("Y (m)", 112.0), num("X (m)", 112.0), num("Akış kotu (m)", 104.0)],
                    r.ends
                        .iter()
                        .map(|e| {
                            let side = if e.side == "left" { "Sol" } else { "Sağ" };
                            let mark = if e.side == r.inlet { "giriş" } else { "çıkış" };
                            vec![format!("{side} ({mark})"), grouped(e.offset, 2), grouped(e.x, 3), grouped(e.y, 3), grouped(e.invert, 3)]
                        })
                        .collect(),
                    "Uç yok",
                );
                body = body.push_grow(
                    row![picture, list]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(180.0),
                );
            }
            _ => {}
        }
        self.culvert_frame(w, body)
    }

    /// The window around its body, the buttons its footer.
    fn culvert_frame<'a>(&'a self, w: &'a Window, body: kentos_ui::widget::grow::Grows<'a, Message>) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
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
        Frame::new(TITLE).scroll(body).footer(footer).width(1180.0).modal(msg(Event::Close))
    }

    /// Hidrolik: the saved culvert's water at a flow, its performance curve and HY-8's summary table.
    fn hydraulics_view<'a>(
        &'a self,
        w: &'a Window,
        mut body: kentos_ui::widget::grow::Grows<'a, Message>,
    ) -> kentos_ui::widget::grow::Grows<'a, Message> {
        use crate::road_dialog::{col, grouped, num};
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                HYDRAULICS[k],
                kentos_ui::widget::text_field(hint, &w.hydraulic[k]).on_input(move |t| msg(Event::Hydraulic(k, t))).width(width),
                None,
            )
        };
        let choices = inlets(w.pipe);
        let on_inlet = Select::new(
            choices.iter().map(|(_, words)| Choice::new(*words)),
            choices.iter().position(|(i, _)| *i == w.inlet),
            move |k| msg(Event::Inlet(choices[k].0)),
        );
        if w.chosen.is_none() {
            return body.push(label::caption("Hidrolik kayıtlı menfezi okur: önce menfezi kaydedin.").style(muted));
        }
        body = body.push(
            row![
                field("m³/s", 0, 90.0),
                field("0", 1, 90.0),
                words::field("Giriş", container(on_inlet).width(200), None),
                field("ke", 2, 90.0),
                field("menfezin", 3, 80.0),
                field("yok", 4, 70.0),
                field("yok", 5, 90.0),
                field("yol", 6, 100.0),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        let read = match &w.hydraulics {
            None => {
                return body.push(
                    label::caption(
                        "Debiyi, giriş kaybını (ke) ve gerekirse Manning n'yi yazın: kayıtlı menfezin önünde su HDS-5'in \
                         giriş ve çıkış kontrolüyle okunur. Savak Cd ve boyu yazılınca yol savak sayılır; kret boşsa \
                         gövdenin üstündeki yolun en yüksek kotudur. Hidrolik kayıtlı menfezi okur.",
                    )
                    .style(muted),
                );
            }
            Some(Err(why)) => return body.push(label::caption(why.clone()).style(danger)),
            // The culvert's own warnings are the edit's, shown above; the overtopping is said below in words.
            Some(Ok((read, _))) => read,
        };
        let at = &read.at;
        let control = if at.inlet_control { "giriş kontrolü" } else { "çıkış kontrolü" };
        let road = match (&read.crest, &at.over) {
            (_, Some(o)) => format!(
                " · yoldan {:.2} m³/s, kret üstünde {:.2} m, derinlik {:.2} m, hız {:.2} m/s → {}",
                at.overtopping, o.head, o.depth, o.speed, o.words
            ),
            (Some(c), None) => format!(" · yol aşılmıyor: krete {:.2} m var (kret {:.2})", c - at.level, c),
            (None, None) => String::new(),
        };
        body = body.push(label::caption(format!(
            "Q {} m³/s · giriş önünde su {:.2} m, kot {:.2} · {control} (giriş {:.2} m, çıkış {:.2} m) · menfezden {:.2} m³/s{road}",
            trimmed(at.flow),
            at.headwater,
            at.level,
            at.inlet_headwater,
            at.outlet_headwater,
            at.through
        )));
        if let Some(o) = &at.over {
            let style = if o.hazard >= 3 { danger } else { warn };
            body = body.push(
                label::caption(format!(
                    "Yol bu debide su altında: D·V = {:.2} m²/s, {}. Kapatma kararı tasarımcınındır.",
                    o.unit, o.words
                ))
                .style(style),
            );
        }
        let rows = read
            .curve
            .iter()
            .map(|p| {
                vec![
                    grouped(p.flow, 2),
                    grouped(p.through, 2),
                    grouped(p.overtopping, 2),
                    grouped(p.level, 3),
                    grouped(p.inlet_headwater, 2),
                    grouped(p.outlet_headwater, 2),
                    if p.inlet_control { "Giriş" } else { "Çıkış" }.into(),
                    p.over.as_ref().map_or_else(String::new, |o| format!("H{}", o.hazard)),
                ]
            })
            .collect();
        let table = crate::road_dialog::sheet_fill(
            vec![
                num("Toplam (m³/s)", 88.0),
                num("Menfez (m³/s)", 88.0),
                num("Yol (m³/s)", 72.0),
                num("Su kotu (m)", 80.0),
                num("Giriş HW (m)", 78.0),
                num("Çıkış HW (m)", 78.0),
                col("Kontrol", 58.0),
                col("Tehlike", 54.0),
            ],
            rows,
            "Eğri yok",
        );
        let picture = crate::picture::picture(
            iced::widget::canvas(Performance { read }).width(Fill).height(Fill),
            "Menfez performans eğrisi",
        );
        // The curve keeps the larger share; a narrow window scrolls the table inside itself.
        body.push_grow(
            row![
                container(picture).width(iced::FillPortion(5)).height(Fill),
                container(table).width(iced::FillPortion(4)).height(Fill)
            ]
            .spacing(12)
            .height(Fill),
            typography::scaled(220.0),
        )
    }

    pub(crate) fn culvert_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.culvert_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Fill(CELLS, t) => Some(msg(Event::Cells(t.to_owned()))),
            Control::Fill(name, t) if HYDRAULICS.contains(&name) => {
                let k = HYDRAULICS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Hydraulic(k, t.to_owned())))
            }
            Control::Press(page) if PAGES.contains(&page) => {
                Some(msg(Event::Page(PAGES.iter().position(|p| *p == page).unwrap_or(0))))
            }
            Control::Pick("Giriş", words) => {
                let found = INLETS.iter().find(|(_, w)| *w == words).ok_or_else(|| format!("“{words}” diye bir giriş yok"))?;
                Some(msg(Event::Inlet(found.0)))
            }
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The barrel along its line, left end to right: its flow line and top, the ends and the water's way.
struct Barrel<'a> {
    report: &'a CulvertReport,
    top: f64,
}

impl canvas::Program<Message> for Barrel<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let [a, b] = [&self.report.ends[0], &self.report.ends[1]];
        let (z0, z1) = (a.invert.min(b.invert), a.invert.max(b.invert) + self.top);
        let pad = 30.0_f32;
        let (w, h) = (f64::from(bounds.width - 2.0 * pad), f64::from(bounds.height - 2.0 * pad));
        let span = (b.distance - a.distance).max(1e-6);
        // Heights drawn at most five times the length's scale, so a fall and a top read without filling the picture.
        let sx = w / span;
        let sy = (h / (z1 - z0).max(1e-6)).min(5.0 * sx);
        let base = f64::from(pad) + 0.5 * (h + (z1 - z0) * sy);
        let at = |d: f64, z: f64| Point::new(pad + ((d - a.distance) * sx) as f32, (base - (z - z0) * sy) as f32);
        let shape = Path::new(|p| {
            p.move_to(at(a.distance, a.invert));
            p.line_to(at(b.distance, b.invert));
            p.line_to(at(b.distance, b.invert + self.top));
            p.line_to(at(a.distance, a.invert + self.top));
            p.close();
        });
        frame.fill(&shape, iced::Color { a: 0.2, ..tk.accent });
        frame.stroke(&shape, Stroke::default().with_width(1.4).with_color(tk.accent));
        for (e, align) in [(a, iced::alignment::Horizontal::Left), (b, iced::alignment::Horizontal::Right)] {
            let word = if e.side == self.report.inlet { "giriş" } else { "çıkış" };
            frame.fill_text(Text {
                content: format!("{word} {:.2}", e.invert),
                position: at(e.distance, e.invert) + iced::Vector::new(0.0, 12.0),
                color: tk.muted,
                size: Pixels(11.0),
                align_x: align.into(),
                ..Text::default()
            });
        }
        frame.fill_text(Text {
            content: format!("L = {:.2} m", self.report.length),
            position: Point::new(bounds.width / 2.0, 12.0),
            color: tk.text,
            size: Pixels(12.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            ..Text::default()
        });
        vec![frame.into_geometry()]
    }
}

/// A round step for about `n` ticks over `span`.
fn tick(span: f64, n: f64) -> f64 {
    let raw = (span / n).max(1e-9);
    let p = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].into_iter().map(|k| k * p).find(|s| *s >= raw).unwrap_or(10.0 * p)
}

/// The performance curve as HY-8 draws one: the water's level before the inlet against the total flow, the inlet's and
/// the outlet's control apart (dashed), the road's crest, the asked flow marked; overtopped points coloured by hazard.
struct Performance<'a> {
    read: &'a CulvertHydraulicsRead,
}

impl canvas::Program<Message> for Performance<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.read;
        let pts = &r.curve;
        if pts.len() < 2 {
            return vec![frame.into_geometry()];
        }
        let invert = r.inlet_invert;
        let q1 = pts.iter().map(|p| p.flow).fold(0.0, f64::max).max(1e-9);
        let mut z1 = pts.iter().map(|p| invert + p.inlet_headwater.max(p.outlet_headwater).max(p.headwater)).fold(invert, f64::max);
        if let Some(c) = r.crest {
            z1 = z1.max(c);
        }
        let (z0, z1) = (invert, z1 + 0.05 * (z1 - invert).max(0.1));
        let (left, right, top, bottom) = (52.0_f32, 14.0_f32, 26.0_f32, 30.0_f32);
        let (w, h) = (f64::from(bounds.width - left - right), f64::from(bounds.height - top - bottom));
        let at = |q: f64, z: f64| {
            Point::new(left + (q / q1 * w) as f32, top + ((z1 - z.clamp(z0, z1)) / (z1 - z0) * h) as f32)
        };
        let small = Pixels(11.0);
        // The grid and its numbers.
        let dz = tick(z1 - z0, 5.0);
        let mut z = (z0 / dz).ceil() * dz;
        while z <= z1 + 1e-9 {
            let p = at(0.0, z);
            frame.stroke(&Path::line(p, Point::new(left + w as f32, p.y)), Stroke::default().with_width(1.0).with_color(tk.faint));
            frame.fill_text(Text {
                content: format!("{z:.1}"),
                position: Point::new(left - 6.0, p.y - 6.0),
                color: tk.muted,
                size: small,
                align_x: iced::alignment::Horizontal::Right.into(),
                ..Text::default()
            });
            z += dz;
        }
        let dq = tick(q1, 6.0);
        let mut q = 0.0;
        while q <= q1 + 1e-9 {
            let p = at(q, z0);
            frame.stroke(&Path::line(at(q, z1), p), Stroke::default().with_width(1.0).with_color(tk.faint));
            frame.fill_text(Text {
                content: trimmed((q * 100.0).round() / 100.0),
                position: Point::new(p.x, p.y + 4.0),
                color: tk.muted,
                size: small,
                align_x: iced::alignment::Horizontal::Center.into(),
                ..Text::default()
            });
            q += dq;
        }
        frame.fill_text(Text {
            content: "Toplam debi (m³/s) →".into(),
            position: Point::new(left + w as f32, bounds.height - 13.0),
            color: tk.muted,
            size: small,
            align_x: iced::alignment::Horizontal::Right.into(),
            ..Text::default()
        });
        frame.fill_text(Text {
            content: "Giriş önünde su kotu (m)".into(),
            position: Point::new(left, 6.0),
            color: tk.muted,
            size: small,
            ..Text::default()
        });
        let line = |f: &dyn Fn(&kentos_commands::CulvertFlowRead) -> f64| {
            Path::new(|b| {
                for (i, p) in pts.iter().enumerate() {
                    let at = at(p.flow, invert + f(p));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            })
        };
        let dashed = |c: iced::Color| Stroke {
            line_dash: canvas::LineDash { segments: &[5.0, 4.0], offset: 0 },
            ..Stroke::default().with_width(1.3).with_color(c)
        };
        frame.stroke(&line(&|p| p.inlet_headwater), dashed(tk.success));
        frame.stroke(&line(&|p| p.outlet_headwater), dashed(tk.warning));
        if let Some(c) = r.crest {
            let (a, b) = (at(0.0, c), at(q1, c));
            frame.stroke(&Path::line(a, b), Stroke::default().with_width(1.6).with_color(tk.danger));
            frame.fill_text(Text {
                content: format!("yol kreti {c:.2}"),
                // Under the line at its left: the curve meets the crest far to the right, the axis' words sit above.
                position: Point::new(a.x + 4.0, a.y + 3.0),
                color: tk.danger,
                size: small,
                ..Text::default()
            });
        }
        frame.stroke(&line(&|p| p.headwater), Stroke::default().with_width(2.4).with_color(tk.accent));
        for p in pts.iter().filter(|p| p.over.is_some()) {
            let hazard = p.over.as_ref().map_or(0, |o| o.hazard);
            let c = if hazard >= 3 { tk.danger } else { tk.warning };
            frame.fill(&Path::circle(at(p.flow, p.level), 3.5), c);
        }
        let asked = at(r.at.flow, r.at.level);
        frame.stroke(&Path::line(Point::new(asked.x, top), at(r.at.flow, z0)), Stroke::default().with_width(1.0).with_color(tk.text));
        frame.fill(&Path::circle(asked, 5.0), tk.accent);
        frame.stroke(&Path::circle(asked, 5.0), Stroke::default().with_width(1.4).with_color(tk.text));
        // Under the curve, which rises to the right: the asked flow's words there stay clear of it.
        frame.fill_text(Text {
            content: format!("Q {} m³/s → {:.2}", trimmed(r.at.flow), r.at.level),
            position: Point::new(asked.x + 8.0, asked.y + 6.0),
            color: tk.text,
            size: Pixels(12.0),
            ..Text::default()
        });
        // The legend.
        let legend = [("su kotu", tk.accent, false), ("giriş kontrolü", tk.success, true), ("çıkış kontrolü", tk.warning, true)];
        let widths: Vec<f32> = legend.iter().map(|(word, _, _)| 18.0 + 6.2 * word.chars().count() as f32).collect();
        let whole: f32 = widths.iter().map(|w| w + 10.0).sum();
        let y = top + h as f32 - 10.0;
        frame.fill_rectangle(
            Point::new(left + w as f32 - whole - 6.0, y - 9.0),
            iced::Size::new(whole + 4.0, 18.0),
            iced::Color { a: 0.92, ..tk.surface },
        );
        let mut x = left + w as f32;
        for ((word, color, dash), width) in legend.iter().zip(&widths).rev() {
            x -= width + 10.0;
            let stroke = if *dash { dashed(*color) } else { Stroke::default().with_width(2.4).with_color(*color) };
            frame.stroke(&Path::line(Point::new(x, y), Point::new(x + 14.0, y)), stroke);
            frame.fill_text(Text {
                content: (*word).into(),
                position: Point::new(x + 18.0, y - 6.0),
                color: tk.muted,
                size: small,
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}
