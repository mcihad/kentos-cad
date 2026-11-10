//! Pafta eğrileri (docs/adr/1820; `terrain.sheetContours`): a surface's contour lines and heights drawn for a sheet by
//! `road.terrains.sheetcontours` (BÖHHBÜY m. 78(1)(c)–(g)). The surface and the layer on one line, the scale and the
//! interval (empty: the package's for the scale) on the next, the rules (e) and (f) as ticks, the detail layers (g) as
//! ticks; the summary is the command's plan, counted by kind, with its notes. Çiz writes them in one undo step.
//! Şevler (ğ, docs/adr/3120): a tick and the layers of the slopes' tops and bottoms (guessed from their names, left
//! out of the details), whose lines' corners take spot heights. Seçilen noktalar (ğ, docs/adr/3520): a tick and the
//! layer of the points the surveyor chose (guessed from its name: seçilen, karakteristik, kot noktası), each taking its
//! own height or the surface's; their own heights against the surface's by m. 93(1)(c)'s limit (docs/adr/4740), said
//! in the summary. Yerleşik alan (m. 63(2), docs/adr/4740): a tick and the layers of the built-up areas and of the
//! roads' lines (guessed from their names), heights about every 2 cm on the sheet.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, SheetBuiltUp, SheetSlopes, TerrainsSheetContoured, TerrainsSheetContours, contour_rules_default};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_terrains_sheet};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Pafta eğrileri";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Çiz";
/// A slope layer's choice when there is none.
const NONE: &str = "";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The project's surfaces: id and name.
    pub surfaces: Vec<(String, String)>,
    pub surface: String,
    /// The drawing's layers: id and name.
    pub layers: Vec<(String, String)>,
    pub layer: String,
    pub scale: f64,
    /// Empty: the package's interval for the scale.
    pub interval: String,
    pub steep: bool,
    pub auxiliary: bool,
    pub labels: bool,
    /// The summits' and pits' heights (ğ, docs/adr/2320).
    pub spots: bool,
    /// The detail layers ticked (g).
    pub details: Vec<String>,
    /// The slopes' spot heights (ğ, docs/adr/3120): ticked, and the layers of their tops and bottoms ("" none).
    pub slopes: bool,
    pub slope_top: String,
    pub slope_bottom: String,
    /// The surveyor's chosen points (ğ, docs/adr/3520): ticked, and their layer ("" none).
    pub chosen: bool,
    pub chosen_layer: String,
    /// The heights of built-up areas and roads (m. 63(2), docs/adr/4740): ticked, and the layers of the areas' bounds and
    /// of the roads' lines ("" none).
    pub built: bool,
    pub built_area: String,
    pub built_road: String,
    pub answer: Option<Result<TerrainsSheetContoured, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Surface(String),
    Layer(String),
    Scale(f64),
    Interval(String),
    Steep(bool),
    Auxiliary(bool),
    Labels(bool),
    Spots(bool),
    Detail(String, bool),
    Slopes(bool),
    SlopeTop(String),
    SlopeBottom(String),
    Chosen(bool),
    ChosenLayer(String),
    Built(bool),
    BuiltArea(String),
    BuiltRoad(String),
    Draw,
    Close,
}

fn msg(e: Event) -> Message {
    Message::SheetContours(e)
}

impl State {
    fn input(&self) -> Result<TerrainsSheetContours, String> {
        let interval = match self.interval.trim() {
            "" => None,
            t => Some(t.replace(',', ".").parse::<f64>().map_err(|_| format!("Aralık “{t}” bir sayı değil."))?),
        };
        Ok(TerrainsSheetContours {
            surface: self.surface.clone(),
            layer_id: self.layer.clone(),
            scale: Some(self.scale.max(0.0) as u32),
            interval,
            details: self.details.clone(),
            steep: Some(self.steep),
            auxiliary: Some(self.auxiliary),
            labels: Some(self.labels),
            spots: Some(self.spots),
            slopes: self.slopes.then(|| SheetSlopes {
                top: [&self.slope_top].into_iter().filter(|l| !l.is_empty()).cloned().collect(),
                bottom: [&self.slope_bottom].into_iter().filter(|l| !l.is_empty()).cloned().collect(),
            }),
            points: if self.chosen && !self.chosen_layer.is_empty() { vec![self.chosen_layer.clone()] } else { Vec::new() },
            built_up: self.built.then(|| SheetBuiltUp {
                areas: [&self.built_area].into_iter().filter(|l| !l.is_empty()).cloned().collect(),
                roads: [&self.built_road].into_iter().filter(|l| !l.is_empty()).cloned().collect(),
                every: None,
            }),
            ..Default::default()
        })
    }
}

impl App {
    fn sheet_contours_run(&mut self) {
        let input = match self.sheet_contours.input() {
            Ok(i) => i,
            Err(why) => {
                self.sheet_contours.answer = Some(Err(why));
                return;
            }
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.sheet_contours.answer = Some(match road_terrains_sheet::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Eğriler hazırlanamadı.".to_owned()),
        });
    }

    /// Pafta eğrileri (`terrain.sheetContours`): the project's first surface, a layer named for contours, the detail
    /// layers guessed from their names (bina, yol, dere, kanal, şev), the slopes' top and bottom layers too (şev with
    /// üst; şev with dip or dibi, alt or taban), those not among the details.
    pub(crate) fn open_sheet_contours(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let surfaces: Vec<(String, String)> = doc
            .model
            .settings()
            .road
            .as_ref()
            .map(|r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
            .unwrap_or_default();
        // The selection's own surface when every selected object is its source (docs/adr/2480), else the first.
        let picked = self.selection_roles().surface.filter(|id| surfaces.iter().any(|(s, _)| s == id));
        let Some(first) = picked.or_else(|| surfaces.first().map(|s| s.0.clone())) else {
            self.warn("Pafta eğrileri: projede arazi yüzeyi yok; önce Arazi › Yeni yüzey ile kurun.");
            return Task::none();
        };
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let named = |words: &[&str]| -> Vec<String> {
            layers
                .iter()
                .filter(|(_, n)| words.iter().any(|w| n.to_lowercase().contains(w)))
                .map(|(id, _)| id.clone())
                .collect()
        };
        let layer = named(&["eğri", "egri", "yükselti", "yukselti", "kontur"])
            .into_iter()
            .next()
            .unwrap_or_else(|| doc.model.layers().active().to_owned());
        let slope = |ends: &[&str]| -> String {
            layers
                .iter()
                .find(|(_, n)| {
                    let n = n.to_lowercase();
                    (n.contains("şev") || n.contains("sev")) && ends.iter().any(|w| n.contains(w))
                })
                .map(|(id, _)| id.clone())
                .unwrap_or_default()
        };
        let (slope_top, slope_bottom) = (slope(&["üst", "ust"]), slope(&["dip", "dib", "alt", "taban"]));
        let chosen_layer = named(&["seçilen", "secilen", "karakteristik", "kot noktası", "kot noktasi"]).into_iter().next().unwrap_or_default();
        // Docs/adr/4740 (m. 63(2)): the built-up areas' and the roads' centre lines' layers.
        let built_area = named(&["yerleşik", "yerlesik", "yerleşim", "yerlesim", "iskân", "iskan"]).into_iter().next().unwrap_or_default();
        let built_road = named(&["yol ekseni", "yol-ekseni", "eksen", "aks"]).into_iter().next().unwrap_or_default();
        let details: Vec<String> = named(&["bina", "yol", "dere", "kanal", "ark", "şev", "sev"])
            .into_iter()
            .filter(|d| *d != slope_top && *d != slope_bottom && *d != chosen_layer && *d != built_area && *d != built_road)
            .collect();
        self.sheet_contours = State {
            surfaces,
            surface: first,
            layers,
            layer,
            scale: doc.model.settings().plot_scale.round().max(1.0),
            interval: String::new(),
            steep: true,
            auxiliary: true,
            labels: true,
            spots: false,
            details,
            slopes: false,
            slope_top,
            slope_bottom,
            chosen: false,
            chosen_layer,
            built: false,
            built_area,
            built_road,
            answer: None,
        };
        self.sheet_contours_run();
        self.dialog = Some(Dialog::SheetContours);
        Task::none()
    }

    /// Çiz: the lines and heights written, one undo step.
    pub(crate) fn sheet_contours_draw(&mut self) {
        let input = match self.sheet_contours.input() {
            Ok(i) => i,
            Err(why) => return self.warn(why),
        };
        let Some(doc) = self.document.as_mut() else { return };
        let result = road_terrains_sheet::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!(
                    "{TITLE}: {} eğri, {} eğri kotu ve {} nokta kotu çizildi; çizim yüzeyine ve okuduğu katmanlara bağlı tutulur.",
                    out.lines.len(),
                    out.labels.len(),
                    out.spots.len()
                ),
            );
            self.sheet_contours = State::default();
            self.dialog = None;
        }
    }

    pub(crate) fn sheet_contours_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.sheet_contours;
        match event {
            Event::Surface(v) => s.surface = v,
            Event::Layer(v) => s.layer = v,
            Event::Scale(v) => s.scale = v,
            Event::Interval(v) => s.interval = v,
            Event::Steep(v) => s.steep = v,
            Event::Auxiliary(v) => s.auxiliary = v,
            Event::Labels(v) => s.labels = v,
            Event::Spots(v) => s.spots = v,
            Event::Detail(id, on) => {
                s.details.retain(|d| *d != id);
                if on {
                    s.details.push(id);
                }
            }
            Event::Slopes(v) => s.slopes = v,
            Event::SlopeTop(v) => s.slope_top = v,
            Event::SlopeBottom(v) => s.slope_bottom = v,
            Event::Chosen(v) => s.chosen = v,
            Event::ChosenLayer(v) => s.chosen_layer = v,
            Event::Built(v) => s.built = v,
            Event::BuiltArea(v) => s.built_area = v,
            Event::BuiltRoad(v) => s.built_road = v,
            Event::Draw => {
                self.sheet_contours_draw();
                return Task::none();
            }
            Event::Close => {
                self.sheet_contours = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.sheet_contours_run();
        Task::none()
    }

    pub(crate) fn sheet_contours_view(&self) -> Element<'_, Message> {
        let s = &self.sheet_contours;
        let package = contour_rules_default()
            .and_then(|r| r.intervals.iter().find(|i| f64::from(i.scale) == s.scale).map(|i| i.interval))
            .map_or("ölçek için aralık yok".to_owned(), |i| format!("{i} (m. 78(1)(c))"));
        let mut ticks = row![].spacing(14).align_y(Center);
        for (id, name) in &s.layers {
            if *id == s.layer
                || (s.slopes && (*id == s.slope_top || *id == s.slope_bottom))
                || (s.chosen && *id == s.chosen_layer)
                || (s.built && (*id == s.built_area || *id == s.built_road))
            {
                continue;
            }
            let on = s.details.contains(id);
            let id = id.clone();
            ticks = ticks.push(words::check(on, name, Some(msg(Event::Detail(id, !on)))));
        }
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Yüzey",
                row![
                    container(fields::select_owned(s.surfaces.clone(), &s.surface, |v| msg(Event::Surface(v)))).width(230),
                    label::caption("Katman"),
                    container(fields::select_owned(s.layers.clone(), &s.layer, |v| msg(Event::Layer(v)))).width(230)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Ölçek 1/",
                row![
                    container(NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(500.0).decimals(0)).width(230),
                    label::caption("Aralık (m)"),
                    container(fields::input(None, &package, &s.interval, false, false).on_input(|v| msg(Event::Interval(v)))).width(230)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Kurallar",
                row![
                    words::check(s.steep, "Dik yerde yalnız kalın (e)", Some(msg(Event::Steep(!s.steep)))),
                    words::check(s.auxiliary, "Yardımcı eğri (e)", Some(msg(Event::Auxiliary(!s.auxiliary)))),
                    words::check(s.labels, "Kot yazısı (f)", Some(msg(Event::Labels(!s.labels)))),
                    words::check(s.spots, "Tepe ve çukur kotları (ğ)", Some(msg(Event::Spots(!s.spots))))
                ]
                .spacing(14)
                .align_y(Center),
            )
            // Many layers wrap onto further lines rather than run out of the window.
            .field("Kesmeyeceği (g)", ticks.wrap().vertical_spacing(6))
            .field("Şevler (ğ)", {
                let mut choices = vec![(NONE.to_owned(), "Yok".to_owned())];
                choices.extend(s.layers.iter().filter(|(id, _)| *id != s.layer).cloned());
                row![
                    words::check(s.slopes, "Şev kotları", Some(msg(Event::Slopes(!s.slopes)))),
                    label::caption("Üstü"),
                    container(fields::select_owned(choices.clone(), &s.slope_top, |v| msg(Event::SlopeTop(v)))).width(190),
                    label::caption("Dibi"),
                    container(fields::select_owned(choices, &s.slope_bottom, |v| msg(Event::SlopeBottom(v)))).width(190)
                ]
                .spacing(10)
                .align_y(Center)
            })
            // docs/adr/3520: the surveyor's other points (m. 78(1)(ğ)).
            .field("Seçilen noktalar (ğ)", {
                let mut choices = vec![(NONE.to_owned(), "Yok".to_owned())];
                choices.extend(s.layers.iter().filter(|(id, _)| *id != s.layer).cloned());
                row![
                    words::check(s.chosen, "Nokta kotları", Some(msg(Event::Chosen(!s.chosen)))),
                    label::caption("Katman"),
                    container(fields::select_owned(choices, &s.chosen_layer, |v| msg(Event::ChosenLayer(v)))).width(190),
                    label::caption("Kendi kotu varsa o, yoksa yüzeyden")
                ]
                .spacing(10)
                .align_y(Center)
            })
            // docs/adr/4740: m. 63(2)'s heights in built-up areas and along roads, about every 2 cm on the sheet.
            .field("Yerleşik alan (63/2)", {
                let mut choices = vec![(NONE.to_owned(), "Yok".to_owned())];
                choices.extend(s.layers.iter().filter(|(id, _)| *id != s.layer).cloned());
                row![
                    words::check(s.built, "2 cm'de bir kot", Some(msg(Event::Built(!s.built)))),
                    label::caption("Alan"),
                    container(fields::select_owned(choices.clone(), &s.built_area, |v| msg(Event::BuiltArea(v)))).width(190),
                    label::caption("Yol"),
                    container(fields::select_owned(choices, &s.built_road, |v| msg(Event::BuiltRoad(v)))).width(190)
                ]
                .spacing(10)
                .align_y(Center)
            });
        let mut lines = Vec::new();
        match &s.answer {
            Some(Ok(r)) => {
                let count = |k: &str| r.lines.iter().filter(|l| l.kind == k).count();
                lines.push(words::text_line(
                    Kind::Ok,
                    format!(
                        "{} eğri: {} kalın, {} ara, {} yardımcı; {} kot yazısı (1/{}, aralık {} m).",
                        r.lines.len(),
                        count("major"),
                        count("minor"),
                        count("auxiliary"),
                        r.labels.len(),
                        r.scale,
                        r.interval
                    ),
                ));
                for sp in r.spots.iter().filter(|sp| sp.kind == "summit" || sp.kind == "pit") {
                    lines.push(words::text_line(
                        Kind::Ok,
                        format!(
                            "{} kotu {} (Y {}, X {}).",
                            if sp.kind == "summit" { "Tepe" } else { "Çukur" },
                            sp.text,
                            self.format().coord(sp.x),
                            self.format().coord(sp.y)
                        ),
                    ));
                }
                // Docs/adr/4740: the chosen points' heights against the surface, by m. 93(1)(c)'s limit.
                if let Some(c) = &r.check {
                    lines.push(words::text_line(
                        if c.within { Kind::Ok } else { Kind::Warn },
                        format!(
                            "Ölçülen kot − yüzey: {} noktada KOH {}, m. 93(1)(c) sınırı {}: {}.",
                            c.compared,
                            self.format().length(c.rms),
                            self.format().length(c.limit),
                            if c.within { "altında" } else { "aşıyor" }
                        ),
                    ));
                }
                let built = |k: &str| r.spots.iter().filter(|sp| sp.kind == k).count();
                if built("road") + built("builtUp") > 0 {
                    lines.push(words::text_line(
                        Kind::Ok,
                        format!("m. 63(2): yolda {}, yerleşik alanda {} kot.", built("road"), built("builtUp")),
                    ));
                }
                for n in r.notes.iter().skip(1) {
                    lines.push(words::text_line(Kind::Info, n.clone()));
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let body = Column::new().spacing(12).push(form).push(words::summary(lines));
        let ready = matches!(s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(DRAW, ready.then(|| msg(Event::Draw))))
            .width(820.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn sheet_contours_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
