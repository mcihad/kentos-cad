//! PDF içe aktar (docs/adr/0352, plan/cad.md CAD-IO-21–26): the window asks for the page, the scale it was printed at,
//! where its lower left corner goes (typed or picked on the drawing), what is taken, what the curves become and their
//! tolerance, a region and the page's layers; it shows the command's plan as the choices change (the box in the
//! drawing, the counts, the layers, what is left out and what a PDF cannot carry) and takes the page with
//! `file.import.pdf`, so a script and the window do the same.

use std::fmt;
use std::path::PathBuf;

use iced::widget::{Column, Row, button, row, text_input};
use iced::{Center, Element, Task};
use kentos_commands::{CallOp, CommandResult, PdfCurveOutput, PdfImported};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_native_application::ExecutionContext;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::style;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};
use kentos_ui::label;
use serde_json::{Value, json};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Picker, Window, message};
use crate::app::{App, Dialog as Shown, Message};

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub file: String,
    pub pages: u32,
    pub page: u32,
    /// The page's layers: “PDF” for what is in no group, then its groups; whether the PDF hides it; whether it is taken.
    pub layers: Vec<(String, bool)>,
    pub take: Vec<bool>,
    pub scale: String,
    pub tolerance: String,
    pub curves: PdfCurveOutput,
    pub strokes: bool,
    pub fills: bool,
    pub texts: bool,
    pub at: Option<[f64; 2]>,
    pub region: bool,
    /// Sol, Alt, Sağ, Üst: millimetres from the page's lower left corner.
    pub bounds: [String; 4],
    /// The window is put away while the corner is picked on the drawing.
    pub picking: bool,
    /// The command's plan for the choices, or why there is none.
    pub plan: Option<Result<(PdfImported, Vec<String>), String>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Picked(Option<PathBuf>),
    Page(u32),
    Scale(String),
    Tolerance(String),
    Curves(PdfCurveOutput),
    Strokes,
    Fills,
    Texts,
    Layer(usize),
    Region,
    Bound(usize, String),
    Pick,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::PdfImport(e))
}

/// The curves' choice as the segmented control shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Curves(PdfCurveOutput);

impl fmt::Display for Curves {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            PdfCurveOutput::Polyline => "Çoklu çizgi",
            PdfCurveOutput::Nurbs => "NURBS eğrisi",
        })
    }
}

/// A number typed the Turkish way or the other.
fn number(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A page's layers, “PDF” first when something is in no group.
fn layers_of(page: &kentos_formats::pdf::PdfPage) -> Vec<(String, bool)> {
    let plain = page.paths.iter().any(|p| p.layer.is_none()) || page.texts.iter().any(|t| t.layer.is_none());
    let mut out = Vec::new();
    if plain || page.layers.is_empty() {
        out.push((kentos_native_application::file_import_pdf::PLAIN.to_owned(), false));
    }
    for (i, l) in page.layers.iter().enumerate() {
        if !out.iter().any(|(n, _)| n == l) {
            out.push((l.clone(), page.hidden.get(i).copied().unwrap_or(false)));
        }
    }
    out
}

/// The file's first page and its layers, or why it cannot be read.
pub fn describe(path: &std::path::Path, scale: f64) -> Result<State, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let page = kentos_formats::pdf::read_page(&bytes, 1).map_err(|e| e.to_string())?;
    let layers = layers_of(&page);
    Ok(State {
        file: path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned()),
        path: path.to_path_buf(),
        pages: page.count,
        page: 1,
        take: vec![true; layers.len()],
        layers,
        scale: kentos_interaction::fixed(scale, 0),
        tolerance: "0.01".to_owned(),
        curves: PdfCurveOutput::Polyline,
        strokes: true,
        fills: true,
        texts: true,
        at: None,
        region: false,
        bounds: [
            "0".to_owned(),
            "0".to_owned(),
            kentos_interaction::fixed(page.width * 25.4 / 72.0, 1),
            kentos_interaction::fixed(page.height * 25.4 / 72.0, 1),
        ],
        picking: false,
        plan: None,
        status: None,
    })
}

impl State {
    /// The command's input, or the field that is not a number.
    pub fn input(&self) -> Result<Value, String> {
        let scale = number(&self.scale).ok_or("Ölçek bir sayı olmalı: sayfanın basıldığı ölçeğin paydası (50 → 1:50).")?;
        let tolerance = number(&self.tolerance).ok_or("Tolerans bir sayı olmalı: kâğıtta milimetre.")?;
        let mut input = json!({ "path": self.path.display().to_string(), "page": self.page, "scale": scale,
                                "tolerance": tolerance, "strokes": self.strokes, "fills": self.fills, "texts": self.texts,
                                "curves": if self.curves == PdfCurveOutput::Nurbs { "nurbs" } else { "polyline" } });
        if let Some([x, y]) = self.at {
            input["at"] = json!({ "x": x, "y": y });
        }
        let skip: Vec<&str> = self
            .layers
            .iter()
            .zip(&self.take)
            .filter(|(_, t)| !**t)
            .map(|((n, _), _)| n.as_str())
            .collect();
        if !skip.is_empty() {
            input["skipLayers"] = json!(skip);
        }
        if self.region {
            let mut b = [0.0; 4];
            for (k, t) in self.bounds.iter().enumerate() {
                b[k] = number(t).ok_or("Bölgenin kenarları sayı olmalı: sayfanın sol alt köşesinden milimetre.")?;
            }
            input["region"] = json!({ "a": { "x": b[0], "y": b[1] }, "b": { "x": b[2], "y": b[3] } });
        }
        Ok(input)
    }
}

impl App {
    pub(super) fn pdf_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("PDF içe aktar")
                    .add_filter("PDF", &["pdf"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Picked(p)),
        )
    }

    fn pdf_state(&mut self) -> Option<&mut State> {
        match &mut self.exchange {
            Some(Window::PdfImport(s)) => Some(s),
            _ => None,
        }
    }

    pub(super) fn pdf_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(path)) => {
                let scale = self.document.as_ref().map_or(1.0, |d| d.settings().plot_scale.max(1.0));
                match describe(&path, scale) {
                    Ok(state) => self.open_window(Window::PdfImport(Box::new(state))),
                    Err(why) => {
                        self.say(Level::Error, format!("{} açılamadı: {why}", path.display()));
                        return Task::none();
                    }
                }
            }
            Event::Page(n) => {
                if let Some(s) = self.pdf_state() {
                    s.page = n;
                    if let Ok(bytes) = std::fs::read(&s.path)
                        && let Ok(page) = kentos_formats::pdf::read_page(&bytes, n)
                    {
                        s.layers = layers_of(&page);
                        s.take = vec![true; s.layers.len()];
                        s.bounds[2] = kentos_interaction::fixed(page.width * 25.4 / 72.0, 1);
                        s.bounds[3] = kentos_interaction::fixed(page.height * 25.4 / 72.0, 1);
                    }
                }
            }
            Event::Scale(t) => {
                if let Some(s) = self.pdf_state() {
                    s.scale = t;
                }
            }
            Event::Tolerance(t) => {
                if let Some(s) = self.pdf_state() {
                    s.tolerance = t;
                }
            }
            Event::Curves(c) => {
                if let Some(s) = self.pdf_state() {
                    s.curves = c;
                }
            }
            Event::Strokes | Event::Fills | Event::Texts => {
                if let Some(s) = self.pdf_state() {
                    let b = match e {
                        Event::Strokes => &mut s.strokes,
                        Event::Fills => &mut s.fills,
                        _ => &mut s.texts,
                    };
                    *b = !*b;
                }
            }
            Event::Layer(i) => {
                if let Some(s) = self.pdf_state()
                    && let Some(t) = s.take.get_mut(i)
                {
                    *t = !*t;
                }
            }
            Event::Region => {
                if let Some(s) = self.pdf_state() {
                    s.region = !s.region;
                }
            }
            Event::Bound(k, t) => {
                if let Some(s) = self.pdf_state()
                    && let Some(b) = s.bounds.get_mut(k)
                {
                    *b = t;
                }
            }
            Event::Pick => {
                if let Some(s) = self.pdf_state() {
                    s.picking = true;
                }
                self.dialog = None;
                self.field = None;
                self.snap = None;
                self.session.run(Box::new(PickPoint::new("", "Sayfanın sol alt köşesinin çizimdeki yeri")));
                self.with_tool(|s, cx| s.activate(cx));
                return Task::none();
            }
            Event::Run => {
                self.pdf_run();
                return Task::none();
            }
        }
        self.pdf_plan();
        Task::none()
    }

    /// The point PDF içe aktar asked for (input.rs); false when it asked none.
    pub(crate) fn pdf_import_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        let Some(s) = self.pdf_state().filter(|s| s.picking) else {
            return false;
        };
        s.picking = false;
        if let Some(p) = p {
            s.at = Some([p.x, p.y]);
        }
        self.dialog = Some(Shown::Exchange);
        self.pdf_plan();
        true
    }

    /// The command's plan for the window's choices.
    pub(crate) fn pdf_plan(&mut self) {
        let Some(Window::PdfImport(s)) = &self.exchange else {
            return;
        };
        let plan = match s.input() {
            Err(why) => Err(why),
            Ok(input) => match &mut self.document {
                None => return,
                Some(doc) => {
                    let answer = kentos_native_application::registry::run(
                        &mut ExecutionContext::new(&mut doc.model),
                        "file.import.pdf",
                        CallOp::Plan,
                        input,
                    );
                    match answer.ok().and_then(|v| serde_json::from_value::<CommandResult<PdfImported>>(v).ok()) {
                        Some(CommandResult::Completed { output, warnings }) => {
                            Ok((output, warnings.into_iter().map(|w| w.message).collect()))
                        }
                        Some(CommandResult::Failed { error } | CommandResult::Conflict { error }) => Err(error.message),
                        _ => Err("Komutun planı okunamadı.".to_owned()),
                    }
                }
            },
        };
        if let Some(s) = self.pdf_state() {
            s.plan = Some(plan);
            s.status = None;
        }
    }

    fn pdf_run(&mut self) {
        let Some(Window::PdfImport(s)) = &self.exchange else {
            return;
        };
        let input = match s.input() {
            Ok(input) => input,
            Err(why) => {
                if let Some(s) = self.pdf_state() {
                    s.status = Some(why);
                }
                return;
            }
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let first = doc.model.next_slot();
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            "file.import.pdf",
            CallOp::Execute,
            input,
        );
        match answer.ok().and_then(|v| serde_json::from_value::<CommandResult<PdfImported>>(v).ok()) {
            Some(CommandResult::Completed { output, warnings }) => {
                let last = doc.model.next_slot();
                let slots: Vec<Slot> = (first..last)
                    .filter_map(|n| u32::try_from(n).ok())
                    .map(Slot)
                    .filter(|s| doc.model.get(*s).is_some())
                    .collect();
                self.close_exchange();
                self.zoom_to(&slots);
                self.announce(
                    if warnings.is_empty() { Level::Success } else { Level::Warn },
                    format!(
                        "“{}” {}. sayfası alındı: {} nesne; Ctrl+Z geri alır.",
                        output.file, output.page, output.objects
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Some(CommandResult::Failed { error } | CommandResult::Conflict { error }) => {
                if let Some(s) = self.pdf_state() {
                    s.status = Some(error.message);
                }
            }
            _ => {
                if let Some(s) = self.pdf_state() {
                    s.status = Some("PDF alınamadı: komutun cevabı okunamadı.".to_owned());
                }
            }
        }
    }

    pub(super) fn pdf_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        if s.picking {
            return iced::widget::text("").into();
        }
        let format = self.document.as_ref().map(|d| kentos_project::format::Format::of(d.model.settings()));
        let input = |placeholder: &'a str, value: &'a str, width: f32, on: fn(String) -> Event| {
            kentos_ui::widget::focus_ring(
                text_input(placeholder, value)
                    .on_input(move |t| event(on(t)))
                    .padding([5, 8])
                    .width(width)
                    .size(kentos_ui::theme::typography::body())
                    .style(style::field::input),
            )
        };
        let mut body = Column::new().spacing(12).push(words::file_line(
            &s.file,
            format!("{} sayfa · {}", s.pages, s.path.display()),
        ));
        if s.pages > 1 {
            let pages = Select::new(
                (1..=s.pages).map(|n| Choice::new(format!("{n}. sayfa"))),
                Some((s.page - 1) as usize),
                |i| event(Event::Page(i as u32 + 1)),
            )
            .searchable(s.pages > 12);
            body = body.push(words::field("Sayfa", pages, None));
        }
        let scale = row![label::body("1 :"), input("50", &s.scale, 96.0, Event::Scale)]
            .spacing(6)
            .align_y(Center);
        body = body.push(words::field(
            "Ölçek",
            scale,
            Some("Sayfanın basıldığı ölçek: 1:50'de kâğıdın 1 cm'i çizimde 50 cm olur.".to_owned()),
        ));
        let at = s.at.map_or_else(
            || "Çizimin başlangıcı (0, 0)".to_owned(),
            |[x, y]| format.as_ref().map_or_else(|| format!("{x:.3}, {y:.3}"), |f| f.point(kentos_interaction::Vec2::new(x, y))),
        );
        let pick = button(
            row![icon(Icon::Target).size(14.0).tone(if s.at.is_some() { Tone::Accent } else { Tone::Inherit }), label::body("Çizimden seç")]
                .spacing(6)
                .align_y(Center),
        )
        .padding([5, 10])
        .style(style::button::secondary)
        .on_press(event(Event::Pick));
        body = body.push(words::field(
            "Yer",
            row![label::body(at), pick].spacing(10).align_y(Center),
            Some("Sayfanın sol alt köşesinin çizimdeki yeri.".to_owned()),
        ));
        let take = Row::new()
            .spacing(16)
            .push(words::check(s.strokes, "Çizgiler", Some(event(Event::Strokes))))
            .push(words::check(s.fills, "Dolgular", Some(event(Event::Fills))))
            .push(words::check(s.texts, "Yazılar", Some(event(Event::Texts))));
        body = body.push(words::field("Alınacaklar", take, None));
        let curves = Segmented::new(
            [Curves(PdfCurveOutput::Polyline), Curves(PdfCurveOutput::Nurbs)],
            Curves(s.curves),
            |c| event(Event::Curves(c.0)),
        );
        let tolerance = row![label::body("Tolerans"), input("0.01", &s.tolerance, 80.0, Event::Tolerance), label::body("mm")]
            .spacing(6)
            .align_y(Center);
        body = body.push(words::field(
            "Eğriler",
            row![curves, tolerance].spacing(16).align_y(Center),
            Some(match s.curves {
                PdfCurveOutput::Polyline => {
                    "Eğriler kâğıtta bu toleransın içinde kalan çoklu çizgi olur.".to_owned()
                }
                PdfCurveOutput::Nurbs => {
                    "Uç uca eğriler kesin üçüncü dereceden NURBS eğrisi olur; dolguların kenarı toleransla düzleşir.".to_owned()
                }
            }),
        ));
        let mut region = Column::new()
            .spacing(6)
            .push(words::check(s.region, "Yalnız bir bölge", Some(event(Event::Region))));
        if s.region {
            let mut sides = Row::new().spacing(8).align_y(Center);
            let names = ["Sol", "Alt", "Sağ", "Üst"];
            let ons: [fn(String) -> Event; 4] = [
                |t| Event::Bound(0, t),
                |t| Event::Bound(1, t),
                |t| Event::Bound(2, t),
                |t| Event::Bound(3, t),
            ];
            for k in 0..4 {
                sides = sides.push(label::body(names[k])).push(input("0", &s.bounds[k], 64.0, ons[k]));
            }
            region = region.push(sides.push(label::body("mm")));
        }
        body = body.push(words::field(
            "Bölge",
            region,
            s.region
                .then(|| "Sayfanın sol alt köşesinden milimetre: yalnız bütünüyle içinde kalan çizgiler ve başı içindeki yazılar alınır.".to_owned()),
        ));
        let mut layers = Column::new().spacing(6);
        for (i, (name, hidden)) in s.layers.iter().enumerate() {
            let text = if *hidden { format!("{name} (PDF'te gizli)") } else { name.clone() };
            layers = layers.push(words::check(s.take[i], text, Some(event(Event::Layer(i)))));
        }
        body = body.push(words::field("Katmanlar", layers, None));
        let mut lines = Vec::new();
        match &s.plan {
            None => {}
            Some(Err(why)) => lines.push(words::text_line(Line::Warn, why.clone())),
            Some(Ok((plan, warnings))) => {
                let t = &plan.taken;
                let mut kinds = Vec::new();
                for (n, w) in [(t.lines, "çizgi"), (t.polylines, "çoklu çizgi"), (t.curves, "eğri"), (t.areas, "alan"), (t.texts, "yazı")] {
                    if n > 0 {
                        kinds.push(format!("{n} {w}"));
                    }
                }
                lines.push(words::text_line(
                    Line::Ok,
                    format!("{} nesne alınacak: {}.", plan.objects, kinds.join(", ")),
                ));
                if let (Some(b), Some(f)) = (plan.bounds, &format) {
                    lines.push(words::text_line(
                        Line::Info,
                        format!(
                            "Çizimde {} × {}, sol alt köşesi {}. Kâğıt {:.0} × {:.0} mm.",
                            f.length_bare(b[2] - b[0]),
                            f.length(b[3] - b[1]),
                            f.point(kentos_interaction::Vec2::new(b[0], b[1])),
                            plan.paper[0],
                            plan.paper[1]
                        ),
                    ));
                }
                let targets: Vec<String> = plan
                    .targets
                    .iter()
                    .map(|(source, target)| {
                        let made = plan.layers.iter().any(|l| &l.id == target);
                        let name = self
                            .document
                            .as_ref()
                            .and_then(|d| d.model.layers().get(target).map(|n| n.name.clone()))
                            .unwrap_or_else(|| source.clone());
                        if made { format!("{source} → yeni katman") } else { format!("{source} → {name}") }
                    })
                    .collect();
                lines.push(words::text_line(Line::Info, format!("Katmanlar: {}.", targets.join(", "))));
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.clone()));
                }
                for n in &plan.notes {
                    lines.push(words::text_line(Line::Info, n.clone()));
                }
            }
        }
        if !lines.is_empty() {
            body = body.push(words::summary(lines));
        }
        if let Some(text) = &s.status {
            body = body.push(words::text_line(Line::Error, text.clone()));
        }
        let ready = matches!(s.plan, Some(Ok(_)));
        overlay::blocking(
            Dialog::new("PDF içe aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", ready.then(|| event(Event::Run))))
                .width(680.0)
                .max_height(760.0),
        )
    }
}
