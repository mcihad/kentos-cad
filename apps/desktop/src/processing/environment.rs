//! Analiz ortamı in a raster tool's window (docs/adr/0643 §5): the analysis area, the cell size, the snap raster and
//! the NoData value every raster tool of the session takes, as a GIS's environment settings. The window keeps the
//! session's copy; a change goes to both. The run takes it as `cad.processing.run`'s `environment` resolved the same
//! way (`kentos_processing::raster_env::resolve`): the visible box or the chosen objects' box, the snap raster by its
//! persistent id, the cell size typed in the drawing's unit.

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Center, Element, Fill};
use kentos_commands::{Entity, RasterEnvironment};
use kentos_domain::Document;
use kentos_interaction::Format;
use kentos_processing::raster_env::Environment;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::horizontal_divider;
use kentos_ui::widget::select::{Choice, Select};

use super::Event;
use super::dialog::ToolDialog;
use crate::app::Message;

/// Where the analysis area comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AreaFrom {
    /// The tool's own grid, whole.
    #[default]
    Whole,
    /// The box on screen when Çalıştır is pressed.
    View,
    /// The chosen objects' box.
    Selection,
}

const AREAS: [(AreaFrom, &str); 3] = [
    (AreaFrom::Whole, "Bütün raster"),
    (AreaFrom::View, "Görünen alan"),
    (AreaFrom::Selection, "Seçili nesnelerin kutusu"),
];

/// The session's environment as the window shows it: the numbers as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LocalEnvironment {
    pub open: bool,
    pub area: AreaFrom,
    pub cell: String,
    /// The snap raster's persistent id.
    pub snap: Option<String>,
    pub nodata: String,
}

#[derive(Clone, Debug)]
pub enum EnvironmentEvent {
    Toggle,
    Area(AreaFrom),
    Cell(String),
    Snap(Option<String>),
    Nodata(String),
    /// Every part back to none.
    Clear,
}

fn ev(e: EnvironmentEvent) -> Message {
    Message::Processing(Event::Environment(e))
}

/// A number as typed: a comma is a decimal point; empty is none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Some)
        .ok_or(())
}

impl LocalEnvironment {
    /// How many of its parts are set: the section's badge.
    pub(crate) fn count(&self) -> usize {
        usize::from(self.area != AreaFrom::Whole)
            + usize::from(!self.cell.trim().is_empty())
            + usize::from(self.snap.is_some())
            + usize::from(!self.nodata.trim().is_empty())
    }

    /// The command's environment: none when nothing is set; why not when a number does not read.
    pub(crate) fn command(
        &self,
        format: &Format,
        selection: &[String],
        view: [f64; 4],
    ) -> Result<Option<RasterEnvironment>, String> {
        let cell = number(&self.cell)
            .map_err(|()| "Ortamın hücre boyu bir sayı olmalı.".to_owned())?
            .map(|c| format.to_metres(c));
        let nodata = number(&self.nodata)
            .map_err(|()| "Ortamın NoData değeri bir sayı olmalı.".to_owned())?;
        let (area, area_objects) = match self.area {
            AreaFrom::Whole => (None, Vec::new()),
            AreaFrom::View => (Some(view), Vec::new()),
            AreaFrom::Selection if selection.is_empty() => {
                return Err(
                    "Analiz alanı seçili nesnelerin kutusu; önce çizimde nesne seçin.".to_owned(),
                );
            }
            AreaFrom::Selection => (None, selection.to_vec()),
        };
        let env = RasterEnvironment {
            area,
            area_objects,
            cell,
            snap: self.snap.clone(),
            nodata,
        };
        Ok((env != RasterEnvironment::default()).then_some(env))
    }

    pub(crate) fn apply(&mut self, e: EnvironmentEvent) {
        match e {
            EnvironmentEvent::Toggle => self.open = !self.open,
            EnvironmentEvent::Area(a) => self.area = a,
            EnvironmentEvent::Cell(t) => self.cell = t,
            EnvironmentEvent::Snap(s) => self.snap = s,
            EnvironmentEvent::Nodata(t) => self.nodata = t,
            EnvironmentEvent::Clear => {
                *self = Self {
                    open: self.open,
                    ..Self::default()
                };
            }
        }
    }
}

/// The drawing's rasters as the snap list offers them: persistent id and a name (layer, size in cells, cell size).
fn rasters(doc: &Document, format: &Format) -> Vec<(String, String)> {
    doc.entities_with_uids()
        .filter_map(|(uid, e)| {
            let Entity::Raster(r) = e else { return None };
            let layer = doc
                .layers()
                .get(&r.base.layer_id)
                .map_or_else(|| r.base.layer_id.clone(), |n| n.name.clone());
            let a = r.raster.affine;
            let cell = (a[1] * a[1] + a[4] * a[4]).sqrt();
            Some((
                uid.to_string(),
                format!(
                    "{layer} ({} × {}, {})",
                    r.raster.width,
                    r.raster.height,
                    format.length(cell)
                ),
            ))
        })
        .collect()
}

/// Analiz ortamı: a folding section under the form, its badge the parts set.
pub(crate) fn section<'a>(
    window: &'a ToolDialog,
    doc: &'a Document,
    format: &Format,
) -> Element<'a, Message> {
    let s = &window.environment;
    let count = s.count();
    let mut head = row![
        icon(if s.open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        })
        .size(14.0)
        .tone(Tone::Muted),
        label::caption("Analiz ortamı").font(typography::ui_strong()),
    ]
    .spacing(6)
    .align_y(Center);
    if count > 0 {
        head = head.push(
            container(label::mono_caption(count.to_string()))
                .padding([0, 6])
                .style(style::container::badge),
        );
    }
    let toggle = button(head)
        .padding([0, 0])
        .width(Fill)
        .style(style::button::ghost)
        .on_press(ev(EnvironmentEvent::Toggle));
    let mut out = column![toggle, horizontal_divider()].spacing(6);
    if !s.open {
        return out.into();
    }
    let at = AREAS.iter().position(|(a, _)| *a == s.area);
    let area = Select::new(AREAS.iter().map(|(_, l)| Choice::new(*l)), at, |i| {
        ev(EnvironmentEvent::Area(
            AREAS.get(i).map_or(AreaFrom::Whole, |a| a.0),
        ))
    })
    .prefix("Alan");
    let list = rasters(doc, format);
    let snap_at = match &s.snap {
        None => Some(0),
        Some(id) => list.iter().position(|(u, _)| u == id).map(|i| i + 1),
    };
    let ids: Vec<String> = list.iter().map(|(u, _)| u.clone()).collect();
    let snap = Select::new(
        std::iter::once(Choice::new("Yok")).chain(list.iter().map(|(_, n)| Choice::new(n.clone()))),
        snap_at,
        move |i| {
            ev(EnvironmentEvent::Snap(
                i.checked_sub(1).and_then(|k| ids.get(k).cloned()),
            ))
        },
    )
    .prefix("Hizalama rasteri");
    let field =
        |placeholder: &'static str, value: &str, bad: bool, on: fn(String) -> EnvironmentEvent| {
            text_input(placeholder, value)
                .padding([5, 8])
                .font(typography::ui())
                .size(typography::body())
                .style(style::field::validated(bad))
                .on_input(move |t| ev(on(t)))
                .width(Fill)
        };
    let cell_bad =
        number(&s.cell).is_err() || number(&s.cell).ok().flatten().is_some_and(|c| c <= 0.0);
    let nodata_bad = number(&s.nodata).is_err();
    let unit = format.length_unit_label();
    let numbers = row![
        label::body("Hücre boyu").style(style::text::muted),
        container(field("aracınki", &s.cell, cell_bad, EnvironmentEvent::Cell)).width(Fill),
        label::body(unit).style(style::text::muted),
        label::body("NoData").style(style::text::muted),
        container(field(
            "boş",
            &s.nodata,
            nodata_bad,
            EnvironmentEvent::Nodata
        ))
        .width(Fill),
    ]
    .spacing(8)
    .align_y(Center);
    let mut body = Column::new()
        .spacing(10)
        .push(area)
        .push(snap)
        .push(numbers);
    body = body.push(
        label::caption(
            "Bu oturumdaki bütün raster araçlarına uygulanır. Aracın kendi Kapsam'ı ortamın alanından önce gelir; \
             hücre boyu ve hizalama yeni bir ızgara kurar.",
        )
        .style(style::text::muted),
    );
    if count > 0 {
        body = body.push(
            button(label::body("Temizle"))
                .padding([4, 10])
                .style(style::button::secondary)
                .on_press(ev(EnvironmentEvent::Clear)),
        );
    }
    out = out.push(container(body).padding([6, 0]));
    out.into()
}

impl crate::app::App {
    /// Whether the open window's tool takes the environment: a raster analysis tool, or a model with one among its
    /// steps.
    pub(crate) fn processing_takes_environment(&self, tool: &kentos_processing::Tool) -> bool {
        match tool
            .id
            .strip_prefix(kentos_processing::model_runner::MODEL_PREFIX)
            .and_then(|id| self.processing.registry.model(id))
        {
            Some(m) => m.steps.iter().any(|s| {
                self.processing
                    .registry
                    .tool(&s.tool)
                    .is_some_and(|t| kentos_processing::raster_env::takes(&t))
            }),
            None => kentos_processing::raster_env::takes(tool),
        }
    }

    /// The command's environment for the open window's run: none when its tool does not take one or nothing is set.
    pub(crate) fn processing_environment(&self) -> Result<Option<RasterEnvironment>, String> {
        let (Some(doc), Some(window)) = (self.document.as_ref(), self.processing.dialog.as_ref())
        else {
            return Ok(None);
        };
        if !self.processing_takes_environment(&window.tool) {
            return Ok(None);
        }
        let selection: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let v = self.viewport.camera.visible_bounds();
        window.environment.command(
            &self.format(),
            &selection,
            [v.min_x, v.min_y, v.max_x, v.max_y],
        )
    }

    /// The raster core's environment for a run here, resolved on the drawing.
    pub(crate) fn processing_environment_resolved(&self) -> Result<Option<Environment>, String> {
        let Some(given) = self.processing_environment()? else {
            return Ok(None);
        };
        let Some(doc) = self.document.as_ref() else {
            return Ok(None);
        };
        kentos_processing::raster_env::resolve(&doc.model, &given)
            .map(Some)
            .map_err(|e| e.message)
    }
}
