//! KML/KMZ ve GPX dışa aktar (the web's `ui/io/Wgs84ExportDialog.ts`,
//! docs/adr/0222 §4, §5): the objects of the selection, of the visible
//! layers or of the whole drawing, moved from the project's system into WGS
//! 84 longitude and latitude by the shared writers (off the UI thread), the
//! way Koordinat dönüştür moves a point; the way and its accuracy are said
//! before writing. KML keeps the layers as folders, the colours and line
//! weights as styles and the attributes as data; GPX writes points as
//! waypoints and lines as tracks or routes. A project without a coordinate
//! system writes neither: its coordinates are bound to no place. What the
//! format cannot hold is said before writing and after. An export is not a
//! save: the drawing stays unsaved if it was.

use std::collections::BTreeSet;

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{Entity, ExportReport, GpxWriteInput, KmlLayer, KmlWriteInput};
use kentos_geometry_core::crs as core;
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

/// Written as lines, sampled by the application (neither format has curves).
const CURVES: [&str; 4] = ["circle", "arc", "ellipse", "spline"];
/// Not written: neither format has them.
const UNWRITTEN: [&str; 7] = [
    "dimension",
    "table",
    "image",
    "xline",
    "ray",
    "raster",
    "pointcloud",
];

/// What the window writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Kml,
    Gpx,
}

impl Format {
    fn title(self) -> &'static str {
        match self {
            Format::Kml => "KML/KMZ dışa aktar",
            Format::Gpx => "GPX dışa aktar",
        }
    }
}

/// The file's kind as the segmented control shows it: KML or KMZ; tracks or routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice(Format, bool);

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Choice(Format::Kml, false) => "KML",
            Choice(Format::Kml, true) => "KMZ (sıkıştırılmış)",
            Choice(Format::Gpx, false) => "İz (trk)",
            Choice(Format::Gpx, true) => "Rota (rte)",
        })
    }
}

#[derive(Debug, Clone)]
pub struct State {
    format: Format,
    scope: Scope,
    /// Layers the user left out.
    excluded: BTreeSet<String>,
    /// KMZ rather than KML; routes rather than tracks.
    other: bool,
    writing: bool,
    status: Option<(bool, String)>,
    /// What the writer said: logged once the file is written.
    written: Option<ExportReport>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Toggle(String),
    ToggleAll,
    Other(bool),
    Run,
    Bytes(Vec<u8>, ExportReport),
}

fn event(e: Event) -> Message {
    message(Exchange::Wgs84Export(e))
}

impl State {
    pub fn new(app: &App, format: Format) -> Self {
        Self {
            format,
            scope: if app.selection.is_empty() {
                Scope::Visible
            } else {
                Scope::Selection
            },
            excluded: BTreeSet::new(),
            other: false,
            writing: false,
            status: None,
            written: None,
        }
    }

    /// The file's extension and the save dialog's filter.
    fn file(&self) -> (&'static str, (&'static str, &'static str)) {
        match (self.format, self.other) {
            (Format::Kml, false) => (".kml", ("KML", "kml")),
            (Format::Kml, true) => (".kmz", ("KMZ", "kmz")),
            (Format::Gpx, _) => (".gpx", ("GPX", "gpx")),
        }
    }
}

impl App {
    /// The objects that will be written: the scope's, less the layers left out.
    fn wgs84_chosen<'a>(&'a self, s: &State) -> Vec<(String, Vec<&'a Entity>)> {
        let mut all = self.by_layer(s.scope);
        all.retain(|(id, _)| !s.excluded.contains(id));
        all
    }

    /// The project's system as the transforms read it and its datum choices; none without one.
    pub(super) fn wgs84_from(&self) -> Option<(core::System, Vec<core::Choice>)> {
        let settings = self.document.as_ref()?.settings();
        let system = kentos_project::systems::own(settings)?.system?;
        Some((system, kentos_project::systems::choices(settings)))
    }

    /// How the move into WGS 84 goes at the middle of `list`: its accuracy, or why not.
    pub(super) fn wgs84_way(
        &self,
        list: &[&Entity],
    ) -> Option<Result<core::Transformed, core::Unreached>> {
        let (from, choices) = self.wgs84_from()?;
        let doc = self.document.as_ref()?;
        let b = kentos_render_wgpu::scene::extents_of(doc, list.iter().copied())?;
        let mid =
            kentos_geometry_core::Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
        let to = core::System::Geographic {
            datum: core::Datum::Wgs84,
        };
        Some(core::transform_in(&from, &to, mid, &choices))
    }

    pub(super) fn wgs84_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Run => return self.wgs84_export_run(),
            Event::Bytes(bytes, report) => {
                if let Some(Window::Wgs84Export(s)) = &mut self.exchange {
                    s.written = Some(report);
                    let (extension, filter) = s.file();
                    let name = self.export_name(extension);
                    return self.save_export(bytes, name, filter);
                }
                return Task::none();
            }
            _ => {}
        }
        let ids: Vec<String> = match &self.exchange {
            Some(Window::Wgs84Export(s)) => self
                .by_layer(s.scope)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            _ => return Task::none(),
        };
        let Some(Window::Wgs84Export(s)) = &mut self.exchange else {
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
            Event::Other(on) => s.other = on,
            Event::Run | Event::Bytes(..) => {}
        }
        Task::none()
    }

    /// Whether the window can write: objects it holds, and a move into WGS 84 that reaches them.
    fn wgs84_can_write(&self, s: &State, list: &[&Entity]) -> bool {
        list.iter().any(|e| !UNWRITTEN.contains(&e.kind()))
            && matches!(self.wgs84_way(list), Some(Ok(_)))
            && !s.writing
    }

    fn wgs84_export_run(&mut self) -> Task<Message> {
        let (Some(Window::Wgs84Export(s)), Some(doc)) = (&self.exchange, &self.document) else {
            return Task::none();
        };
        let chosen = self.wgs84_chosen(s);
        let list: Vec<&Entity> = chosen.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        if !self.wgs84_can_write(s, &list) {
            return Task::none();
        }
        let Some((from, choices)) = self.wgs84_from() else {
            return Task::none();
        };
        let entities: Vec<Entity> = list.iter().map(|e| (*e).clone()).collect();
        let name = doc
            .name()
            .trim_end_matches(".kcad")
            .trim_end_matches(".KCAD")
            .to_owned();
        let name = if name.is_empty() {
            "cizim".to_owned()
        } else {
            name
        };
        // An insert is written as its block's objects placed (docs/adr/0144 §5).
        let blocks = doc.model.blocks().iter().map(|b| (**b).clone()).collect();
        let format = s.format;
        let other = s.other;
        let layers = doc.model.layers();
        let kml_layers: Vec<KmlLayer> = chosen
            .iter()
            .filter_map(|(id, _)| {
                let l = layers.get(id)?;
                // The groups the layer sits in, outermost first: a Folder each.
                let mut path = Vec::new();
                let mut at = layers.parent(id);
                while let Some(group) = at {
                    path.push(group.name.clone());
                    at = layers.parent(&group.id);
                }
                path.reverse();
                Some(KmlLayer {
                    id: id.clone(),
                    name: l.name.clone(),
                    path,
                    color: l.style.color.clone(),
                    line_weight: Some(l.style.line_weight),
                })
            })
            .collect();
        if let Some(Window::Wgs84Export(s)) = &mut self.exchange {
            s.writing = true;
            s.status = Some((false, "Yazılıyor…".to_owned()));
        }
        off_thread(
            move || match format {
                Format::Kml => {
                    let input = KmlWriteInput {
                        entities,
                        layers: kml_layers,
                        name,
                        blocks,
                        kmz: other,
                    };
                    kentos_formats::kml::write(&input, &from, &choices)
                }
                Format::Gpx => {
                    let input = GpxWriteInput {
                        entities,
                        name,
                        blocks,
                        routes: other,
                    };
                    kentos_formats::gnss::gpx_write::write(&input, &from, &choices)
                }
            },
            move |(bytes, report)| Exchange::Wgs84Export(Event::Bytes(bytes, report)),
        )
    }

    pub(super) fn wgs84_export_written(
        &mut self,
        outcome: Option<Result<String, String>>,
    ) -> Task<Message> {
        let Some(Window::Wgs84Export(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.writing = false;
        match outcome {
            None => s.status = None,
            Some(Err(e)) => s.status = Some((true, format!("Yazılamadı: {e}"))),
            Some(Ok(name)) => {
                let report = s.written.take().unwrap_or_default();
                let what = s.file().1.0;
                let written: u32 = report.counts.values().sum();
                self.say(
                    Level::Success,
                    format!("“{name}” yazıldı: {written} nesne ({what}, WGS 84)."),
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

    pub(super) fn wgs84_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
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
        let kinds = [Choice(s.format, false), Choice(s.format, true)];
        let kind = Segmented::new(kinds, Choice(s.format, s.other), |c| {
            event(Event::Other(c.1))
        });
        let chosen = self.wgs84_chosen(s);
        let list: Vec<&Entity> = chosen.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        let (kind_label, kind_hint) = match s.format {
            Format::Kml => (
                "Dosya",
                "KMZ, KML'nin sıkıştırılmış hâlidir (içinde doc.kml); Google Earth ikisini de açar.",
            ),
            Format::Gpx => (
                "Çizgiler",
                "İz (trk) kaydedilmiş bir yolu, rota (rte) izlenecek bir yolu anlatır; el GPS'leri ikisini de okur. Alanların sınırları her zaman iz olarak yazılır.",
            ),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Yazılacak nesneler",
                scope,
                Some(
                    "Koordinatlar projenin sisteminden WGS 84 boylam ve enlemine çevrilir; on basamakla (yaklaşık 0,01 mm) yazılır."
                        .to_owned(),
                ),
            ))
            .push(words::field(kind_label, kind, Some(kind_hint.to_owned())))
            .push(self.wgs84_export_layers(s))
            .push(self.wgs84_export_summary(s, &list));
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        let can = self.wgs84_can_write(s, &list);
        overlay::blocking(
            Dialog::new(s.format.title())
                // As tall as its content; a long body scrolls, the buttons stay in view.
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    can.then(|| event(Event::Run)),
                ))
                .width(820.0)
                .max_height(780.0),
        )
    }

    fn wgs84_export_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
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
            let (name_cell, written_as): (Element<'a, Message>, String) = match layers.get(id) {
                Some(l) => (
                    row![
                        swatch(hex_color(&l.style.color)),
                        label::body(self.layer_path(id))
                    ]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                    match s.format {
                        Format::Kml => format!("klasör “{}”", self.layer_path(id)),
                        Format::Gpx => "yol noktası, iz ya da rota".to_owned(),
                    },
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
                TableColumn::new(match s.format {
                    Format::Kml => "KML'de",
                    Format::Gpx => "GPX'te",
                })
                .width(Length::FillPortion(2)),
            ])
            .extend(rows),
            all.len(),
        );
        column![head, table].spacing(6).width(Fill).into()
    }

    fn wgs84_export_summary<'a>(
        &'a self,
        s: &'a State,
        list: &[&'a Entity],
    ) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return words::summary(Vec::new());
        };
        // The name and its Turkish endings: “KML'de”, “KML'ye”; “GPX'te”, “GPX'e”.
        let (format, at, to) = match s.format {
            Format::Kml => ("KML", "KML'de", "KML'ye"),
            Format::Gpx => ("GPX", "GPX'te", "GPX'e"),
        };
        let mut kinds = Vec::new();
        for e in list {
            words::count(&mut kinds, e.kind());
        }
        let unwritten = list
            .iter()
            .filter(|e| UNWRITTEN.contains(&e.kind()))
            .count();
        let written = list.len() - unwritten;
        let mut lines = vec![if written > 0 {
            words::text_line(
                Line::Ok,
                format!("{written} nesne yazılacak: {}.", words::kind_counts(&kinds)),
            )
        } else {
            words::text_line(
                Line::Warn,
                format!(
                    "Yazılacak nesne yok. Başka bir kapsam ya da en az bir katman seçin; ölçü, tablo, resim, raster, nokta bulutu ve sonsuz doğrular {to} yazılmaz."
                ),
            )
        }];
        let system = crate::crs::project_name(doc.settings());
        lines.push(match self.wgs84_way(list) {
            None if self.wgs84_from().is_none() => words::text_line(
                Line::Warn,
                format!("Proje yerel (koordinat sistemi yok): koordinatlar bir konuma bağlı değil, WGS 84'e çevrilemez; {format} yazılamaz. Projeye Proje ayarları › Koordinat sistemi'nden bir sistem atayın."),
            ),
            None => words::text_line(Line::Info, "Dönüşüm, yazılacak nesneler seçilince gösterilir."),
            Some(Ok(t)) => {
                let how = kentos_interaction::second::accuracy_text(&t);
                words::text_line(
                    if t.unofficial { Line::Warn } else { Line::Ok },
                    format!("Koordinatlar {system} sisteminden WGS 84 boylam ve enlemine çevrilir; dönüşüm: {how}. Kotlar olduğu gibi yazılır."),
                )
            }
            Some(Err(why)) => words::text_line(
                Line::Warn,
                format!("Koordinatlar WGS 84'e çevrilemez: ortaları {}. {format} yazılamaz.", kentos_formats::reproject::why(why)),
            ),
        });
        let curves = list
            .iter()
            .filter(|e| CURVES.contains(&e.kind()) || super::geojson_export::bulged(e))
            .count();
        if curves > 0 {
            lines.push(words::text_line(Line::Info, format!("{curves} eğri (daire, yay, elips, spline, yaylı çizgi) {at} olmadığı için uygulamanın kendi örneklemesiyle (turda 72 adım) çizgiye çevrilir.")));
        }
        let texts = list.iter().filter(|e| e.kind() == "text").count();
        if texts > 0 {
            lines.push(words::text_line(
                Line::Info,
                match s.format {
                    Format::Kml => {
                        format!("{texts} yazı, metni adı olan nokta yer imi olarak yazılır.")
                    }
                    Format::Gpx => {
                        format!("{texts} yazı, metni adı olan yol noktası (wpt) olarak yazılır.")
                    }
                },
            ));
        }
        let hatches = list.iter().filter(|e| e.kind() == "hatch").count();
        if hatches > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{hatches} taramanın deseni yazılmaz; sınırı alan olarak yazılır."),
            ));
        }
        let areas = list
            .iter()
            .filter(|e| matches!(e.kind(), "polygon" | "hatch"))
            .count();
        if areas > 0 && s.format == Format::Gpx {
            lines.push(words::text_line(Line::Info, format!("GPX'te alan yok: {areas} alanın sınırı ve delikleri kapalı iz (trk) parçaları olarak yazılır.")));
        }
        let inserts = list.iter().filter(|e| e.kind() == "insert").count();
        if inserts > 0 {
            lines.push(words::text_line(Line::Info, format!("{inserts} blok yerleştirmesi, bloğunun nesneleriyle açılmış olarak yazılır; {at} blok yoktur.")));
        }
        if unwritten > 0 {
            lines.push(words::text_line(
                Line::Warn,
                format!("{unwritten} ölçü, tablo, resim, raster, nokta bulutu ya da sonsuz doğru {at} gösterilemez; yazılmaz."),
            ));
        }
        lines.push(words::text_line(
            Line::Info,
            match s.format {
                Format::Kml => "Katmanlar grup yollarıyla klasör, nesneler yer imi olur: adı etiketi (yoksa Ad özniteliği), öznitelikleri veri (ExtendedData), rengi ve kalınlığı biçem (alanlar beşte iki saydam dolguyla).",
                Format::Gpx => "Yol noktalarının, izlerin ve rotaların adı etiket (yoksa Ad özniteliği), açıklaması Açıklama, türü Kod özniteliğidir; öbür öznitelikler ve renkler GPX'e yazılmaz.",
            },
        ));
        words::summary(lines)
    }
}
