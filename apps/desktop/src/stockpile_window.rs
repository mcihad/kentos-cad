//! Stok hacmi (docs/adr/3140; `terrain.stockpile`): a stockpile's volume inside a closed boundary over a base drawn from
//! the boundary's corners, by `road.terrains.stockpile` (Pix4D's volume bases). The surface and the boundary on one
//! line (the selected closed object first, else the drawing's first), the base on the next (the corners' lowest,
//! highest or average height, the plane fitted to them, or a height typed); the summary is the command's answer: cut
//! above the base, fill below, their total and areas, the base, the corners' heights, the warnings and notes. Reads
//! only: nothing is written.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandWarning, Entity, TerrainsStockpile, TerrainsStockpiled};
use kentos_domain::format::fixed;
use kentos_native_application::road_terrains_stockpile;
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Stok hacmi";
const CLOSE: &str = "Kapat";

/// The base, as the command's `base`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Base {
    Lowest,
    Highest,
    Average,
    #[default]
    Fit,
    Elevation,
}

impl Base {
    fn id(self) -> &'static str {
        match self {
            Base::Lowest => "lowest",
            Base::Highest => "highest",
            Base::Average => "average",
            Base::Fit => "fit",
            Base::Elevation => "elevation",
        }
    }
}

impl std::fmt::Display for Base {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Base::Lowest => "En alçak köşe",
            Base::Highest => "En yüksek köşe",
            Base::Average => "Ortalama",
            Base::Fit => "Eğimli düzlem",
            Base::Elevation => "Kot",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The project's surfaces: id and name.
    pub surfaces: Vec<(String, String)>,
    pub surface: String,
    /// The drawing's closed polylines and areas: lasting id and name.
    pub boundaries: Vec<(String, String)>,
    pub boundary: String,
    pub base: Base,
    pub elevation: f64,
    pub answer: Option<Result<(TerrainsStockpiled, Vec<CommandWarning>), String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Surface(String),
    Boundary(String),
    Base(Base),
    Elevation(f64),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Stockpile(e)
}

impl State {
    pub fn input(&self) -> TerrainsStockpile {
        TerrainsStockpile {
            surface: self.surface.clone(),
            boundary: self.boundary.clone(),
            base: self.base.id().to_owned(),
            elevation: (self.base == Base::Elevation).then_some(self.elevation),
        }
    }
}

impl App {
    fn stockpile_run(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        self.stockpile.answer = Some(road_terrains_stockpile::preview(&doc.model, &self.stockpile.input()));
    }

    /// Stok hacmi (`terrain.stockpile`): the project's first surface (the selection's own when it has one), the
    /// selected closed polyline or area as the boundary, else the drawing's first; the base the fitted plane.
    pub(crate) fn open_stockpile(&mut self) -> Task<Message> {
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
        let picked = self.selection_roles().surface.filter(|id| surfaces.iter().any(|(s, _)| s == id));
        let Some(first) = picked.or_else(|| surfaces.first().map(|s| s.0.clone())) else {
            self.warn("Stok hacmi: projede arazi yüzeyi yok; önce Arazi › Yeni yüzey ile kurun.");
            return Task::none();
        };
        let layer_name = |id: &str| doc.model.layers().get(id).map(|n| n.name.clone()).unwrap_or_default();
        let boundaries: Vec<(String, String)> = doc
            .model
            .entities_with_uids()
            .filter(|(_, e)| matches!(e, Entity::Polygon(_)) || matches!(e, Entity::Polyline(p) if p.closed))
            .map(|(u, e)| (u.to_string(), format!("{} #{}", layer_name(&e.base().layer_id), e.base().id)))
            .collect();
        let selected = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .find(|u| boundaries.iter().any(|(b, _)| b == u));
        let boundary = selected.or_else(|| boundaries.first().map(|b| b.0.clone())).unwrap_or_default();
        self.stockpile = State {
            surfaces,
            surface: first,
            boundaries,
            boundary,
            base: Base::Fit,
            elevation: 0.0,
            answer: None,
        };
        self.stockpile_run();
        self.dialog = Some(Dialog::Stockpile);
        Task::none()
    }

    pub(crate) fn stockpile_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.stockpile;
        match event {
            Event::Surface(v) => s.surface = v,
            Event::Boundary(v) => s.boundary = v,
            Event::Base(v) => {
                // A typed height starts at the corners' lowest, the base most piles stand on.
                if v == Base::Elevation && s.elevation == 0.0
                    && let Some(Ok((r, _))) = &s.answer
                    && let Some(low) = r.corners.iter().copied().reduce(f64::min)
                {
                    s.elevation = (low * 100.0).round() / 100.0;
                }
                s.base = v;
            }
            Event::Elevation(v) => s.elevation = v,
            Event::Close => {
                self.stockpile = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.stockpile_run();
        Task::none()
    }

    fn stockpile_lines(&self, r: &TerrainsStockpiled, warnings: &[CommandWarning]) -> Vec<Element<'_, Message>> {
        let format = self.format();
        let mut lines = vec![
            words::text_line(
                Kind::Ok,
                format!(
                    "Kazı (tabanın üstü) {} m³, dolgu (tabanın altı) {} m³, toplam {} m³.",
                    fixed(r.cut, 2),
                    fixed(r.fill, 2),
                    fixed(r.total, 2)
                ),
            ),
            words::text_line(
                Kind::Info,
                format!(
                    "Sınırın alanı {}: kazı {}, dolgu {}{}.",
                    format.area(r.area),
                    format.area(r.cut_area),
                    format.area(r.fill_area),
                    // The command's own threshold for the part off the surface (its warning's).
                    if r.uncovered > 1e-6 * r.area.max(1.0) { format!(", yüzeyin dışında {}", format.area(r.uncovered)) } else { String::new() }
                ),
            ),
        ];
        let [a, b, c] = r.plane;
        let base = if a == 0.0 && b == 0.0 {
            format!("Taban kotu {} m.", fixed(c, 3))
        } else {
            let first = self
                .document
                .as_ref()
                .and_then(|d| kentos_native_application::road_terrains_coverage::boundary_region(&d.model, &self.stockpile.boundary))
                .and_then(|g| g.outer.first().copied());
            let at = first.map_or(String::new(), |p| format!(", sınırın ilk köşesinde {} m", fixed(a * p[0] + b * p[1] + c, 3)));
            format!("Taban düzlemi: eğimi %{}{at}.", fixed(100.0 * a.hypot(b), 2))
        };
        lines.push(words::text_line(Kind::Info, base));
        if let (Some(lo), Some(hi)) = (r.corners.iter().copied().reduce(f64::min), r.corners.iter().copied().reduce(f64::max)) {
            lines.push(words::text_line(
                Kind::Info,
                format!("Köşelerin yüzeydeki kotları {}–{} m ({} köşe).", fixed(lo, 3), fixed(hi, 3), r.corners.len()),
            ));
        }
        for w in warnings {
            lines.push(words::text_line(Kind::Warn, w.message.clone()));
        }
        for n in &r.notes {
            lines.push(words::text_line(Kind::Info, n.clone()));
        }
        lines
    }

    pub(crate) fn stockpile_view(&self) -> Element<'_, Message> {
        let s = &self.stockpile;
        let base: Element<'_, Message> = {
            let choice =
                Segmented::new([Base::Lowest, Base::Highest, Base::Average, Base::Fit, Base::Elevation], s.base, |v| msg(Event::Base(v)));
            if s.base == Base::Elevation {
                row![
                    choice,
                    label::caption("Kot (m)"),
                    NumberInput::new(s.elevation, |v| msg(Event::Elevation(v))).step(0.5).decimals(3).point().width(110)
                ]
                .spacing(10)
                .align_y(Center)
                .into()
            } else {
                choice.into()
            }
        };
        let form = Form::new()
            .label_width(110.0)
            .field(
                "Yüzey",
                row![
                    container(fields::select_owned(s.surfaces.clone(), &s.surface, |v| msg(Event::Surface(v)))).width(230),
                    label::caption("Sınır"),
                    container(fields::select_owned(s.boundaries.clone(), &s.boundary, |v| msg(Event::Boundary(v)))).width(230)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field("Taban", base);
        let lines = match &s.answer {
            Some(Ok((r, warnings))) => self.stockpile_lines(r, warnings),
            Some(Err(why)) => vec![words::text_line(Kind::Error, why.clone())],
            None => vec![words::text_line(Kind::Info, "Kapalı bir çoklu çizgi ya da alan seçin.".to_owned())],
        };
        let lead = words::text_line(
            Kind::Info,
            "Yığının hacmi sınırından çizilen tabana göre: tabanın üstü kazı, altı dolgu. Çizim değişmez.".to_owned(),
        );
        let body = Column::new().spacing(12).push(lead).push(form).push(words::summary(lines));
        Frame::new(TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(820.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn stockpile_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
