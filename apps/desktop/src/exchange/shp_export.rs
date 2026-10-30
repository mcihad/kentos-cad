//! Shapefile dışa aktar (docs/adr/0514, plan/gis.md CBS-05): the window shows, before anything is written, the
//! Shapefiles the layers make (a layer of several geometry families splits), their shapes and rows, and what the format
//! cannot hold; the code page (UTF-8 or Windows-1254) and where they go (one archive, a new folder, or one .shp when
//! the export makes one) are chosen; Dışa aktar… asks for the path and writes through `file.export.shp`, so a script
//! and the window do the same.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandWarning, ExportShp, ShpEncoding, ShpExported};
use kentos_interaction::Level;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// Where the Shapefiles go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Zip,
    Folder,
    Shp,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Target::Zip => "Tek arşiv (.zip)",
            Target::Folder => "Yeni klasör",
            Target::Shp => "Tek Shapefile (.shp)",
        })
    }
}

/// The code page, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code(pub ShpEncoding);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ShpEncoding::Utf8 => "UTF-8",
            ShpEncoding::Windows1254 => "Windows-1254 (Türkçe)",
        })
    }
}

/// A Shapefile the export makes.
#[derive(Debug, Clone)]
pub struct Planned {
    pub name: String,
    pub shape: Option<&'static str>,
    pub rows: u32,
}

#[derive(Debug)]
pub struct State {
    pub encoding: ShpEncoding,
    pub target: Target,
    /// The Shapefiles and what the format cannot hold, or why none can be written.
    pub plan: Result<(Vec<Planned>, Vec<String>), String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Encoding(Code),
    Target(Target),
    /// Dışa aktar…: the path asked for.
    Save,
    /// The path; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(ShpExported, Vec<CommandWarning>), String>>),
}

fn event(e: Event) -> Message {
    message(Exchange::ShpExport(e))
}

impl App {
    /// What the export would write with `encoding`: every layer with objects.
    pub(super) fn shp_plan(
        &self,
        encoding: ShpEncoding,
    ) -> Result<(Vec<Planned>, Vec<String>), String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        // The objects the export's time shows (docs/adr/0646 §2).
        let time = self.export_time();
        let (model, _) =
            kentos_native_application::time_export::filtered(&doc.model, &[], time.as_ref())
                .map_err(|(_, why, _)| why)?;
        let model = model.as_ref();
        let ids = kentos_native_application::shapefile::layer_ids(model, &[])
            .map_err(|(_, id)| format!("{id} katmanı yok"))?;
        let written = kentos_native_application::shapefile::write(model, &ids, encoding)?;
        if written.sets.is_empty() {
            return Err("Çizimde Shapefile'a yazılacak nesne yok: yazı, ölçü, tablo, resim, raster ve nokta bulutunun Shapefile'da geometrisi yok.".into());
        }
        let sets = written
            .sets
            .iter()
            .map(|s| Planned {
                name: s.name.clone(),
                shape: s.shape,
                rows: s.rows,
            })
            .collect();
        Ok((sets, kentos_headless::shp_export::losses(model, &written)))
    }

    pub(super) fn shp_export_open(&mut self) -> Task<Message> {
        let plan = self.shp_plan(ShpEncoding::Utf8);
        let target = match &plan {
            Ok((sets, _)) if sets.len() == 1 => Target::Shp,
            _ => Target::Zip,
        };
        self.open_window(Window::ShpExport(State {
            encoding: ShpEncoding::Utf8,
            target,
            plan,
            status: None,
        }));
        Task::none()
    }

    pub(super) fn shp_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Encoding(Code(encoding)) => {
                let plan = self.shp_plan(encoding);
                if let Some(Window::ShpExport(s)) = &mut self.exchange {
                    s.encoding = encoding;
                    s.plan = plan;
                }
                Task::none()
            }
            Event::Target(t) => {
                if let Some(Window::ShpExport(s)) = &mut self.exchange {
                    s.target = t;
                }
                Task::none()
            }
            Event::Save => {
                let Some(Window::ShpExport(s)) = &self.exchange else {
                    return Task::none();
                };
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Picked(Some(path.clone()))));
                }
                let target = s.target;
                let suggested = self.export_name(match target {
                    Target::Zip => ".zip",
                    Target::Shp => ".shp",
                    Target::Folder => "-shapefile",
                });
                Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new()
                            .set_title(match target {
                                Target::Folder => "Shapefile dışa aktar: yeni klasörün adı",
                                _ => "Shapefile dışa aktar",
                            })
                            .set_file_name(suggested);
                        dialog = match target {
                            Target::Zip => dialog.add_filter("Shapefile arşivi", &["zip"]),
                            Target::Shp => dialog.add_filter("Shapefile", &["shp"]),
                            Target::Folder => dialog,
                        };
                        let file = dialog.save_file().await?;
                        let mut path = file.path().to_path_buf();
                        let ext = match target {
                            Target::Zip => Some("zip"),
                            Target::Shp => Some("shp"),
                            Target::Folder => None,
                        };
                        if let Some(ext) = ext
                            && path.extension().is_none()
                        {
                            path.set_extension(ext);
                        }
                        Some(path)
                    },
                    |p| event(Event::Picked(p)),
                )
            }
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => {
                let time = self.export_time();
                let (Some(doc), Some(Window::ShpExport(s))) = (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let input = ExportShp {
                    path: path.display().to_string(),
                    layers: Vec::new(),
                    encoding: s.encoding,
                    // A folder is never written over; an archive or a .shp the dialog asked about is.
                    overwrite: s.target != Target::Folder,
                    time,
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                off_thread(
                    move || {
                        kentos_headless::file_commands::export_shp(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::ShpExport(Event::Done(Box::new(r))),
                )
            }
            Event::Done(done) => match *done {
                Ok((out, warnings)) => {
                    let rows: u32 = out.sets.iter().map(|s| s.rows).sum();
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!(
                            "{} yazıldı: {} Shapefile, {rows} satır.",
                            out.path,
                            out.sets.len()
                        ),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                    Task::none()
                }
                Err(why) => {
                    if let Some(Window::ShpExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("Shapefile yazılamadı. {why}")));
                    }
                    Task::none()
                }
            },
        }
    }

    pub(super) fn shp_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok((sets, losses)) => {
                let mut list = Column::new().spacing(4);
                for p in sets {
                    let what = match p.shape {
                        Some(shape) => {
                            format!("{} — {}, {} satır", p.name, shape_name(shape), p.rows)
                        }
                        None => format!(
                            "{} — yalnız öznitelik tablosu (.dbf), {} satır",
                            p.name, p.rows
                        ),
                    };
                    list = list.push(words::text_line(Line::Info, what));
                }
                body = body.push(words::field(
                    "Shapefile'lar",
                    list,
                    Some("Bir Shapefile tek geometri türü taşır: noktası, çizgisi ve alanı olan katman _nokta, _cizgi, _alan ekli ayrı Shapefile'lar olur.".to_owned()),
                ));
                for l in losses {
                    lines.push(words::text_line(Line::Warn, l.clone()));
                }
                if losses.is_empty() {
                    lines.push(words::text_line(
                        Line::Ok,
                        "Her nesne ve değer olduğu gibi yazılır.",
                    ));
                }
                let several = sets.len() > 1;
                if several && s.target == Target::Shp {
                    lines.push(words::text_line(
                        Line::Warn,
                        format!(
                            "Dışa aktarma {} Shapefile yapıyor; tek .shp yalnız birini alır. Arşiv ya da klasör seçin.",
                            sets.len()
                        ),
                    ));
                }
                !(several && s.target == Target::Shp)
            }
            Err(why) => {
                lines.push(words::text_line(Line::Warn, why.clone()));
                false
            }
        };
        body = body.push(words::field(
            "Kod sayfası",
            Segmented::new(
                [Code(ShpEncoding::Utf8), Code(ShpEncoding::Windows1254)],
                Code(s.encoding),
                |c| event(Event::Encoding(c)),
            ),
            Some(".dbf'in metinleri bu kod sayfasında yazılır ve .cpg'de adlandırılır. Windows-1254 yalnız eski kod sayfalarını okuyan programlar içindir; onda olmayan harfler ? olur.".to_owned()),
        ));
        let one = matches!(&s.plan, Ok((sets, _)) if sets.len() == 1);
        body = body.push(words::field(
            "Hedef",
            Segmented::new_with(
                [Target::Zip, Target::Folder, Target::Shp],
                s.target,
                |t| event(Event::Target(t)),
                move |t| t != Target::Shp || one,
            ),
            Some(match s.target {
                Target::Zip => "Bütün Shapefile'lar tek .zip arşivinde; QGIS ve ArcGIS arşivi açmadan okur.",
                Target::Folder => "Yeni bir klasör açılır; her Shapefile katmanın adıyla. Var olan bir klasörün üzerine yazılmaz.",
                Target::Shp => "Tek Shapefile verilen adla, parçaları (.shx, .dbf, .prj, .cpg) yanında.",
            }
            .to_owned()),
        ));
        if let Some(row) = self.export_time_row() {
            body = body.push(row);
        }
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("Shapefile dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(640.0)
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}

/// A shape type as the list says it.
fn shape_name(shape: &str) -> &'static str {
    match shape {
        "POINT" => "nokta",
        "POINTZ" => "kotlu nokta",
        "POINTM" => "ölçülü nokta",
        "MULTIPOINT" => "çok nokta",
        "MULTIPOINTZ" => "kotlu çok nokta",
        "MULTIPOINTM" => "ölçülü çok nokta",
        "POLYLINE" => "çizgi",
        "POLYLINEZ" => "kotlu çizgi",
        "POLYLINEM" => "ölçülü çizgi",
        "POLYGON" => "alan",
        "POLYGONZ" => "kotlu alan",
        "POLYGONM" => "ölçülü alan",
        _ => "geometri",
    }
}
