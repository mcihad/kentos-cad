//! GIS türevi dışa aktar (docs/adr/0530, 0634 §3, plan/gis.md CBS-16): the layers as a derived dataset, a GeoPackage,
//! a zip of Shapefiles, GeoJSON or new tables of a PostGIS database (Biçim). The window shows, before anything is
//! written, the tables the layers make, how many rows are chords of a curve and how
//! far they come from it at most, and what is left out and why; the tolerance (the farthest a chord may come from the
//! curve it stands for) and what becomes of texts and blocks are chosen; Dışa aktar… asks for the path and writes
//! through `file.export.gis`, so a script and the window do the same. The plan is made on a thread of its own: it
//! derives every object, as the export does.

use std::fmt;
use std::path::PathBuf;

use iced::widget::{Column, text_input};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CommandWarning, ExportGis, GisAnnotations, GisExported, GisFormat, GisTableRole,
};
use kentos_interaction::Level;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// A tolerance as the control names it: micrometres, so the choices are whole numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tolerance(pub u32);

impl Tolerance {
    pub const CHOICES: [Tolerance; 4] = [
        Tolerance(10),
        Tolerance(100),
        Tolerance(1000),
        Tolerance(10_000),
    ];

    fn metres(self) -> f64 {
        f64::from(self.0) * 1e-6
    }
}

impl fmt::Display for Tolerance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            10 => f.write_str("0,01 mm"),
            100 => f.write_str("0,1 mm"),
            1000 => f.write_str("1 mm"),
            _ => f.write_str("10 mm"),
        }
    }
}

/// What becomes of texts and blocks, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notes(pub GisAnnotations);

impl fmt::Display for Notes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            GisAnnotations::Table => "Ayrı tablolara",
            GisAnnotations::Skip => "Atla",
        })
    }
}

/// Where the dataset goes, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format(pub GisFormat);

impl Format {
    pub const CHOICES: [Format; 4] = [
        Format(GisFormat::GeoPackage),
        Format(GisFormat::Shapefile),
        Format(GisFormat::GeoJson),
        Format(GisFormat::PostGis),
    ];
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            GisFormat::GeoPackage => "GeoPackage",
            GisFormat::Shapefile => "Shapefile",
            GisFormat::GeoJson => "GeoJSON",
            GisFormat::PostGis => "PostGIS",
        })
    }
}

/// What the export would write, as the window shows it.
pub type Plan = Result<(GisExported, Vec<CommandWarning>), String>;

#[derive(Debug)]
pub struct State {
    pub tolerance: Tolerance,
    pub annotations: GisAnnotations,
    pub format: GisFormat,
    /// PostGIS: the database's address and the tables' schema.
    pub address: String,
    pub schema: String,
    /// The plan for the choices; none while it is being made.
    pub plan: Option<Plan>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tolerance(Tolerance),
    Annotations(Notes),
    Format(Format),
    Address(String),
    Schema(String),
    /// The plan for the choices it names, made on a thread of its own.
    Planned(Tolerance, GisAnnotations, Box<Plan>),
    /// Dışa aktar…: the path asked for.
    Save,
    /// The path; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(GisExported, Vec<CommandWarning>), String>>),
}

fn event(e: Event) -> Message {
    message(Exchange::GisExport(e))
}

/// Millimetres as the window says them: four decimals at most, none that are zeros.
fn mm(metres: f64) -> String {
    let text = format!("{:.4}", metres * 1000.0);
    text.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

fn geometry_name(t: &str) -> &'static str {
    match t {
        "POINT" => "nokta",
        "MULTIPOINT" => "çok nokta",
        "LINESTRING" => "çizgi",
        "MULTILINESTRING" => "çok çizgi",
        "POLYGON" => "alan",
        "MULTIPOLYGON" => "çok parçalı alan",
        _ => "karışık geometri",
    }
}

fn role_name(role: GisTableRole) -> &'static str {
    match role {
        GisTableRole::Layer => "katman",
        GisTableRole::Blocks => "blok yerleştirmeleri",
        GisTableRole::Annotations => "yazılar",
        GisTableRole::Dimensions => "ölçüler",
        GisTableRole::Arch => "mimari nesneler",
    }
}

impl App {
    /// Starts the plan for the choices on a thread of its own.
    fn gis_plan(&self, tolerance: Tolerance, annotations: GisAnnotations) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = doc.model.clone();
        let input = ExportGis {
            path: std::env::temp_dir()
                .join("kentos-gis-plan.gpkg")
                .display()
                .to_string(),
            layers: Vec::new(),
            tolerance: Some(tolerance.metres()),
            annotations,
            overwrite: true,
            format: GisFormat::GeoPackage,
            schema: None,
        };
        off_thread(
            move || {
                kentos_headless::gis_export::make(&model, &input)
                    .map(|(_, _, out, warnings)| (out, warnings))
                    .map_err(|e| e.message)
            },
            move |plan| Exchange::GisExport(Event::Planned(tolerance, annotations, Box::new(plan))),
        )
    }

    pub(super) fn gis_export_open(&mut self) -> Task<Message> {
        let (tolerance, annotations) = (Tolerance(100), GisAnnotations::Table);
        let address = self
            .document
            .as_ref()
            .and_then(|d| {
                d.model
                    .settings()
                    .sources
                    .iter()
                    .find(|s| kentos_commands::is_database(&s.path))
                    .map(|s| s.path.clone())
            })
            .unwrap_or_default();
        self.open_window(Window::GisExport(State {
            tolerance,
            annotations,
            format: GisFormat::GeoPackage,
            address,
            schema: "public".into(),
            plan: None,
            status: None,
        }));
        self.gis_plan(tolerance, annotations)
    }

    pub(super) fn gis_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Tolerance(t) => {
                let Some(Window::GisExport(s)) = &mut self.exchange else {
                    return Task::none();
                };
                s.tolerance = t;
                s.plan = None;
                let annotations = s.annotations;
                self.gis_plan(t, annotations)
            }
            Event::Annotations(Notes(a)) => {
                let Some(Window::GisExport(s)) = &mut self.exchange else {
                    return Task::none();
                };
                s.annotations = a;
                s.plan = None;
                let tolerance = s.tolerance;
                self.gis_plan(tolerance, a)
            }
            Event::Format(Format(f)) => {
                if let Some(Window::GisExport(s)) = &mut self.exchange {
                    s.format = f;
                    s.status = None;
                }
                Task::none()
            }
            Event::Address(t) => {
                if let Some(Window::GisExport(s)) = &mut self.exchange {
                    s.address = t;
                }
                Task::none()
            }
            Event::Schema(t) => {
                if let Some(Window::GisExport(s)) = &mut self.exchange {
                    s.schema = t;
                }
                Task::none()
            }
            Event::Planned(tolerance, annotations, plan) => {
                if let Some(Window::GisExport(s)) = &mut self.exchange
                    && s.tolerance == tolerance
                    && s.annotations == annotations
                {
                    s.plan = Some(*plan);
                }
                Task::none()
            }
            Event::Save => {
                let Some(Window::GisExport(s)) = &self.exchange else {
                    return Task::none();
                };
                if s.format == GisFormat::PostGis {
                    let address = s.address.trim().to_owned();
                    return self.gis_write(address);
                }
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Picked(Some(path.clone()))));
                }
                let tables = match &s.plan {
                    Some(Ok((out, _))) => out.tables.len(),
                    _ => 0,
                };
                let (filter, ext) = match s.format {
                    GisFormat::Shapefile => ("Shapefile (zip)", "zip"),
                    GisFormat::GeoJson if tables > 1 => ("GeoJSON (zip)", "zip"),
                    GisFormat::GeoJson => ("GeoJSON", "geojson"),
                    _ => ("GeoPackage", "gpkg"),
                };
                let suggested = self.export_name(&format!("-turev.{ext}"));
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("GIS türevi dışa aktar")
                            .add_filter(filter, &[ext])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension(ext);
                        }
                        Some(path)
                    },
                    |p| event(Event::Picked(p)),
                )
            }
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => self.gis_write(path.display().to_string()),
            Event::Done(done) => match *done {
                Ok((out, warnings)) => {
                    let rows: u32 = out.tables.iter().map(|t| t.rows).sum();
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!(
                            "{} yazıldı: {} tablo, {rows} satır; en büyük sapma {} mm.",
                            out.path,
                            out.tables.len(),
                            mm(out.max_deviation)
                        ),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                    Task::none()
                }
                Err(why) => {
                    if let Some(Window::GisExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("GIS türevi yazılamadı. {why}")));
                    }
                    Task::none()
                }
            },
        }
    }

    /// The dataset written to `path` (a file, or the database's address) through `file.export.gis`.
    fn gis_write(&mut self, path: String) -> Task<Message> {
        let (Some(doc), Some(Window::GisExport(s))) = (&self.document, &mut self.exchange) else {
            return Task::none();
        };
        let schema = s.schema.trim();
        let input = ExportGis {
            path: path.clone(),
            layers: Vec::new(),
            tolerance: Some(s.tolerance.metres()),
            annotations: s.annotations,
            // The save dialog asked about a file that is there; a database's tables of the same names are refused.
            overwrite: s.format != GisFormat::PostGis,
            format: s.format,
            schema: (s.format == GisFormat::PostGis && !schema.is_empty())
                .then(|| schema.to_owned()),
        };
        s.status = Some((false, format!("{path} yazılıyor…")));
        let model = doc.model.clone();
        off_thread(
            move || kentos_headless::gis_export::export(&model, &input).map_err(|e| e.message),
            |r| Exchange::GisExport(Event::Done(Box::new(r))),
        )
    }

    pub(super) fn gis_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let mut lines = Vec::new();
        let ready = match &s.plan {
            None => {
                lines.push(words::text_line(Line::Info, "Türev hesaplanıyor…"));
                false
            }
            Some(Err(why)) => {
                lines.push(words::text_line(Line::Warn, why.clone()));
                false
            }
            Some(Ok((out, warnings))) => {
                let mut list = Column::new().spacing(4);
                for t in &out.tables {
                    let geometry = t
                        .geometry
                        .as_deref()
                        .map_or("yalnız öznitelik tablosu", geometry_name);
                    let mut what = format!(
                        "{} — {}, {geometry}, {} satır",
                        t.table,
                        role_name(t.role),
                        t.rows
                    );
                    if t.chorded > 0 {
                        what.push_str(&format!(
                            "; {} satır kirişli, en çok {} mm sapma",
                            t.chorded,
                            mm(t.max_deviation)
                        ));
                    }
                    list = list.push(words::text_line(Line::Info, what));
                }
                body = body.push(words::field(
                    "Tablolar",
                    list,
                    Some("Her satır kaynak nesnenin kalıcı kimliğini, türünü, sapmasını ve kaynağının özetini taşır; katmanların alanları ve öznitelikleri sütun olur, Z ve M köşelerde taşınır.".to_owned()),
                ));
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} satır kirişli (yay, daire, elips, eğri, NURBS); en büyük sapma {} mm, tolerans {} mm. Kaynak çizimde kalır, dosya onun türevidir.",
                        out.chorded,
                        mm(out.max_deviation),
                        mm(out.tolerance)
                    ),
                ));
                // The warning says what is left out and why, the columns renamed and a deviation over the tolerance.
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
                true
            }
        };
        body = body.push(words::field(
            "Biçim",
            Segmented::new(Format::CHOICES, Format(s.format), |f| {
                event(Event::Format(f))
            }),
            Some(match s.format {
                GisFormat::GeoPackage => "Tek dosya: her tablo kalıcı kimlikleriyle, türetim kaydı GeoPackage'ın metadata uzantısında.".to_owned(),
                GisFormat::Shapefile => "Bir .zip: her tablo geometri türü başına bir Shapefile; uzun sütun adları kısaltılır, türetim kayıtları kentos-turev.json'da.".to_owned(),
                GisFormat::GeoJson => "Tek tablo bir .geojson, birden çok tablo .zip'te birer dosya; koordinatlar projenin sisteminde, M taşınmaz.".to_owned(),
                GisFormat::PostGis => "Veritabanında yeni tablolar, tek işlemde; türetim kaydı tablonun yorumunda, kalıcı kimlikler şemanın kentos_uids'inde.".to_owned(),
            }),
        ));
        if s.format == GisFormat::PostGis {
            let field = |placeholder: &str, value: &str, on: fn(String) -> Event| {
                text_input(placeholder, value)
                    .padding([5, 8])
                    .font(kentos_ui::theme::typography::ui())
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::validated(false))
                    .on_input(move |t| event(on(t)))
                    .width(Fill)
            };
            body = body.push(words::field(
                "Veritabanı",
                Column::new()
                    .spacing(8)
                    .push(field("postgresql://kullanıcı@sunucu/veritabanı", &s.address, Event::Address))
                    .push(field("public", &s.schema, Event::Schema)),
                Some("Adres ve şema. Parola adrese yazılmaz: PGPASSWORD ortam değişkeni ya da ~/.pgpass.".to_owned()),
            ));
        }
        body = body.push(words::field(
            "Tolerans",
            Segmented::new(Tolerance::CHOICES, s.tolerance, |t| {
                event(Event::Tolerance(t))
            }),
            Some("Kirişlerin kaynak eğriden en çok ne kadar uzakta kalabileceği. İnceldikçe yay başına kiriş artar, dosya büyür; sapma her satırın kentos_deviation sütununda, metre olarak yazılır.".to_owned()),
        ));
        body = body.push(words::field(
            "Yazılar ve bloklar",
            Segmented::new(
                [
                    Notes(GisAnnotations::Table),
                    Notes(GisAnnotations::Skip),
                ],
                Notes(s.annotations),
                |n| event(Event::Annotations(n)),
            ),
            Some(match s.annotations {
                GisAnnotations::Table => "Katmanların tablolarına girmez: her blok tanımının yerleştirmeleri blok adıyla nokta olarak, bloğun öznitelikleri sütun; yazılar kentos_annotations tablosunda nokta olarak.".to_owned(),
                GisAnnotations::Skip => "Yazılar ve bloklar türeve yazılmaz; sayıları ve nedeni yukarıda söylenir.".to_owned(),
            }),
        ));
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("GIS türevi dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    if s.format == GisFormat::PostGis {
                        "Veritabanına yaz"
                    } else {
                        "Dışa aktar…"
                    },
                    (ready && (s.format != GisFormat::PostGis || !s.address.trim().is_empty()))
                        .then(|| event(Event::Save)),
                ))
                .width(680.0)
                .max_height(760.0),
        )
    }
}
