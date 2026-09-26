//! GeoJSON dışa aktar (the web's `ui/io/GeoJsonExportDialog.ts`,
//! docs/adr/0046): the objects of the selection, of the visible layers or of
//! the whole drawing as a FeatureCollection (the shared writer, off the UI
//! thread). Coordinates are written exactly and never transformed: a WGS 84
//! project (EPSG:4326) gives RFC 7946 GeoJSON; any other its own coordinates
//! with the 2008 `crs` member naming them, which the window says before
//! writing (RFC 7946 knows only WGS 84, and there is no coordinate
//! transformation yet). What GeoJSON cannot hold is said before writing (the
//! summary) and after (the writer's report, in the command line). An export
//! is not a save: the drawing stays unsaved if it was.

use std::collections::BTreeSet;

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{Entity, ExportReport, GeoJsonLayer, GeoJsonWriteInput};
use kentos_interaction::Level;
use kentos_ui::label;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, overlay, swatch};

use super::dxf_export::{Counted, Scope};
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};
use crate::view::hex_color;

/// Written as lines, sampled by the application (GeoJSON has no curves).
const CURVES: [&str; 4] = ["circle", "arc", "ellipse", "spline"];
/// Not written: GeoJSON has no text, dimension or infinite line.
const UNWRITTEN: [&str; 4] = ["text", "dimension", "xline", "ray"];

#[derive(Debug, Clone)]
pub struct State {
    scope: Scope,
    /// Layers the user left out.
    excluded: BTreeSet<String>,
    writing: bool,
    status: Option<(bool, String)>,
    /// What the writer said and the system it wrote in: logged once the file is written.
    written: Option<(ExportReport, u32)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Toggle(String),
    ToggleAll,
    Run,
    Bytes(Vec<u8>, ExportReport, u32),
}

fn event(e: Event) -> Message {
    message(Exchange::GeoJsonExport(e))
}

impl State {
    pub fn new(app: &App) -> Self {
        Self {
            scope: if app.selection.is_empty() {
                Scope::Visible
            } else {
                Scope::Selection
            },
            excluded: BTreeSet::new(),
            writing: false,
            status: None,
            written: None,
        }
    }
}

/// A path or area with an arc among its edges (a bulge), holes included.
fn bulged(e: &Entity) -> bool {
    let arcs = |b: &Option<Vec<f64>>| {
        b.as_ref()
            .is_some_and(|b| b.iter().any(|v| v.abs() > 1e-12))
    };
    match e {
        Entity::Polyline(p) | Entity::Polygon(p) => {
            arcs(&p.bulges)
                || p.holes
                    .as_ref()
                    .is_some_and(|holes| holes.iter().any(|r| arcs(&r.bulges)))
        }
        _ => false,
    }
}

impl App {
    /// The objects that will be written: the scope's, less the layers left out.
    fn geojson_chosen<'a>(&'a self, s: &State) -> Vec<(String, Vec<&'a Entity>)> {
        let mut all = self.by_layer(s.scope);
        all.retain(|(id, _)| !s.excluded.contains(id));
        all
    }

    pub(super) fn geojson_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Run => return self.geojson_export_run(),
            Event::Bytes(bytes, report, srid) => {
                if let Some(Window::GeoJsonExport(s)) = &mut self.exchange {
                    s.written = Some((report, srid));
                    let name = self.export_name(".geojson");
                    return self.save_export(bytes, name, ("GeoJSON", "geojson"));
                }
                return Task::none();
            }
            _ => {}
        }
        let ids: Vec<String> = match &self.exchange {
            Some(Window::GeoJsonExport(s)) => self
                .by_layer(s.scope)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            _ => return Task::none(),
        };
        let Some(Window::GeoJsonExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Scope(scope) => s.scope = scope,
            Event::Toggle(id) => {
                if !s.excluded.remove(&id) {
                    s.excluded.insert(id);
                }
            }
            Event::ToggleAll => {
                if ids.iter().any(|id| s.excluded.contains(id)) {
                    for id in &ids {
                        s.excluded.remove(id);
                    }
                } else {
                    s.excluded.extend(ids);
                }
            }
            Event::Run | Event::Bytes(..) => {}
        }
        Task::none()
    }

    fn geojson_export_run(&mut self) -> Task<Message> {
        let (Some(Window::GeoJsonExport(s)), Some(doc)) = (&self.exchange, &self.document) else {
            return Task::none();
        };
        if s.writing {
            return Task::none();
        }
        let chosen = self.geojson_chosen(s);
        let entities: Vec<Entity> = chosen
            .iter()
            .flat_map(|(_, list)| list.iter().map(|e| (*e).clone()))
            .collect();
        if !entities.iter().any(|e| !UNWRITTEN.contains(&e.kind())) {
            return Task::none();
        }
        let layers = doc.model.layers();
        let srid = doc.settings().srid;
        let name = doc
            .name()
            .trim_end_matches(".kcad")
            .trim_end_matches(".KCAD")
            .to_owned();
        let input = GeoJsonWriteInput {
            entities,
            layers: chosen
                .iter()
                .filter_map(|(id, _)| {
                    layers.get(id).map(|l| GeoJsonLayer {
                        id: id.clone(),
                        name: l.name.clone(),
                    })
                })
                .collect(),
            srid,
            name: if name.is_empty() {
                "cizim".to_owned()
            } else {
                name
            },
        };
        if let Some(Window::GeoJsonExport(s)) = &mut self.exchange {
            s.writing = true;
            s.status = Some((false, "Yazılıyor…".to_owned()));
        }
        off_thread(
            move || kentos_formats::geojson::write(&input),
            move |(bytes, report)| Exchange::GeoJsonExport(Event::Bytes(bytes, report, srid)),
        )
    }

    pub(super) fn geojson_export_written(
        &mut self,
        outcome: Option<Result<String, String>>,
    ) -> Task<Message> {
        let Some(Window::GeoJsonExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.writing = false;
        match outcome {
            None => s.status = None,
            Some(Err(e)) => s.status = Some((true, format!("Yazılamadı: {e}"))),
            Some(Ok(name)) => {
                let (report, srid) = s.written.take().unwrap_or_default();
                let written: u32 = report.counts.values().sum();
                let system = if srid == 4326 {
                    ", RFC 7946".to_owned()
                } else {
                    format!(", EPSG:{srid}")
                };
                self.say(
                    Level::Success,
                    format!("“{name}” yazıldı: {written} nesne (GeoJSON{system})."),
                );
                for item in &report.notes {
                    self.output(words::report_text(item));
                }
                if !report.skipped.is_empty() {
                    let skipped: Vec<String> =
                        report.skipped.iter().map(words::report_text).collect();
                    self.warn(format!(
                        "“{name}” içine yazılmayanlar: {}",
                        skipped.join(" ")
                    ));
                }
                self.close_exchange();
            }
        }
        Task::none()
    }

    pub(super) fn geojson_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let counts: Vec<Counted> = Scope::ALL
            .iter()
            .map(|scope| Counted(*scope, self.scope_entities(*scope).len()))
            .collect();
        let current = counts
            .iter()
            .copied()
            .find(|c| c.0 == s.scope)
            .unwrap_or(Counted(s.scope, 0));
        let scope = Segmented::new_with(
            counts.clone(),
            current,
            |c| event(Event::Scope(c.0)),
            |c| c.1 > 0,
        );
        let chosen = self.geojson_chosen(s);
        let writable = chosen
            .iter()
            .flat_map(|(_, list)| list.iter())
            .any(|e| !UNWRITTEN.contains(&e.kind()));
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Yazılacak nesneler",
                scope,
                Some(
                    "GeoJSON FeatureCollection (UTF-8); koordinatlar yuvarlanmadan, tam yazılır."
                        .to_owned(),
                ),
            ))
            .push(self.geojson_export_layers(s))
            .push(self.geojson_export_summary(&chosen));
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("GeoJSON dışa aktar")
                // As tall as its content; a long body (a drawing with many layers) scrolls, the buttons stay in view.
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    (writable && !s.writing).then(|| event(Event::Run)),
                ))
                .width(820.0)
                .max_height(780.0),
        )
    }

    fn geojson_export_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let all = self.by_layer(s.scope);
        let Some(doc) = self.document.as_ref().filter(|_| !all.is_empty()) else {
            return words::empty("Bu kapsamda nesne yok.");
        };
        let layers = doc.model.layers();
        let out = all.iter().filter(|(id, _)| s.excluded.contains(id)).count();
        let every = match out {
            0 => Check::Checked,
            n if n == all.len() => Check::Unchecked,
            _ => Check::Mixed,
        };
        let head = row![
            check_box(every, Some(event(Event::ToggleAll))),
            label::caption(format!("Bütün katmanlar ({})", all.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let rows = all.iter().map(|(id, list)| {
            let layer = layers.get(id);
            let (name_cell, written_as): (Element<'a, Message>, String) = match layer {
                Some(l) => (
                    row![
                        swatch(hex_color(&l.style.color)),
                        label::body(self.layer_path(id))
                    ]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                    format!("kentos.layer: “{}”", l.name),
                ),
                None => (
                    label::body("Katmanı olmayan nesneler").into(),
                    "katmansız".to_owned(),
                ),
            };
            TableRow::new([
                check_box(
                    if s.excluded.contains(id) {
                        Check::Unchecked
                    } else {
                        Check::Checked
                    },
                    Some(event(Event::Toggle(id.clone()))),
                ),
                name_cell,
                label::body(list.len().to_string()).into(),
                label::caption(written_as).into(),
            ])
        });
        let table = words::fitted(
            Table::new([
                TableColumn::new("").width(22),
                TableColumn::new("Katman").width(Length::FillPortion(3)),
                TableColumn::new("Nesne").width(60).align_right(),
                TableColumn::new("GeoJSON'da").width(Length::FillPortion(2)),
            ])
            .extend(rows),
            all.len(),
        );
        column![head, table].spacing(6).width(Fill).into()
    }

    fn geojson_export_summary<'a>(
        &'a self,
        chosen: &[(String, Vec<&'a Entity>)],
    ) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return words::summary(Vec::new());
        };
        let list: Vec<&Entity> = chosen.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        let mut kinds = Vec::new();
        for e in &list {
            words::count(&mut kinds, e.kind());
        }
        let written = list
            .iter()
            .filter(|e| !UNWRITTEN.contains(&e.kind()))
            .count();
        let curves = list
            .iter()
            .filter(|e| CURVES.contains(&e.kind()) || bulged(e))
            .count();
        let hatches = list.iter().filter(|e| e.kind() == "hatch").count();
        let areas = list
            .iter()
            .filter(|e| matches!(e.kind(), "polygon" | "hatch"))
            .count();
        let unwritten = list.len() - written;
        let data = list.iter().any(|e| {
            let b = e.base();
            b.label.as_deref().is_some_and(|l| !l.is_empty()) || !b.attrs.is_empty()
        });
        let styled = list
            .iter()
            .filter(|e| e.base().color.is_some() || e.base().symbol.is_some())
            .count();
        let srid = doc.settings().srid;
        let system =
            crate::crs::system(srid).map_or_else(|| format!("EPSG:{srid}"), |s| s.name.clone());
        let mut lines = vec![if written > 0 {
            words::text_line(
                Line::Ok,
                format!("{written} nesne yazılacak: {}.", words::kind_counts(&kinds)),
            )
        } else {
            words::text_line(
                Line::Warn,
                "Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin; yazı, ölçü ve sonsuz doğrular GeoJSON'a yazılmaz.",
            )
        }];
        lines.push(if srid == 4326 {
            words::text_line(
                Line::Ok,
                "Proje WGS 84 (EPSG:4326) sisteminde: dosya RFC 7946 GeoJSON olur (boylam, enlem).",
            )
        } else {
            words::text_line(Line::Warn, format!("Dosya RFC 7946 GeoJSON olmayacak. Proje {system} (EPSG:{srid}) sisteminde; RFC 7946 yalnız WGS 84 boylam, enlem ister. Koordinatlar dönüştürülmeden projenin sisteminde yazılır ve eski (2008) crs üyesiyle adlandırılır: QGIS ve GDAL doğru okur, yalnız RFC 7946'yı bilen programlar (web haritaları) yanlış yere koyar. WGS 84'e koordinat dönüşümü henüz yok (geliştirme aşamasında)."))
        });
        if curves > 0 {
            lines.push(words::text_line(Line::Info, format!("{curves} eğri (daire, yay, elips, spline, yaylı çizgi) GeoJSON'da olmadığı için uygulamanın kendi örneklemesiyle (turda 72 adım) çizgiye çevrilir.")));
        }
        if hatches > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{hatches} tarama alan (Polygon) olarak yazılır; deseni yazılmaz."),
            ));
        }
        if areas > 0 {
            lines.push(words::text_line(Line::Info, "Alan halkaları RFC 7946'nın sağ el kuralına göre yazılır (dış sınır saat yönünün tersine); köşeler değişmez."));
        }
        if unwritten > 0 {
            lines.push(words::text_line(
                Line::Warn,
                format!(
                    "{unwritten} yazı, ölçü ya da sonsuz doğru GeoJSON'da gösterilemez; yazılmaz."
                ),
            ));
        }
        if data {
            lines.push(words::text_line(Line::Info, "Öznitelikler metin özellik (properties) olarak yazılır; katman ve etiket KentOS'un “kentos” üyesinde: KentOS geri okur, öbür programlar yok sayar."));
        }
        if styled > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{styled} nesnenin kendi rengi ya da sembolü GeoJSON'a yazılmaz."),
            ));
        }
        words::summary(lines)
    }
}
