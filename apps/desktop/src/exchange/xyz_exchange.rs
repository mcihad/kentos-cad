//! XYZ dışa aktar (docs/adr/0545, plan/gis.md CBS-05): the points or every vertex as `X Y Z` lines. The window
//! chooses which positions, the separator, the decimals, the header and what a position without an elevation
//! becomes, and shows before anything is written the lines, what is left out and the system the file does not name;
//! Dışa aktar… asks for the path and writes through `file.export.xyz`. A script and the window do the same.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandWarning, ExportXyz, XyzExported, XyzSeparator, XyzVertices, XyzWithoutZ,
};
use kentos_interaction::Level;
use kentos_native_application::xyz_table;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// Which positions, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Which(pub XyzVertices);

impl fmt::Display for Which {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            XyzVertices::Points => "Noktalar",
            XyzVertices::All => "Bütün köşeler",
        })
    }
}

/// The separator, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sep(pub XyzSeparator);

impl fmt::Display for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            XyzSeparator::Space => "Boşluk",
            XyzSeparator::Comma => "Virgül",
            XyzSeparator::Semicolon => "Noktalı virgül",
            XyzSeparator::Tab => "Sekme",
        })
    }
}

/// The decimals, as the control names it: the shortest text, or places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Places(pub Option<u8>);

impl fmt::Display for Places {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("En kısa"),
            Some(n) => write!(f, "{n}"),
        }
    }
}

/// What a position without an elevation becomes, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Missing(pub XyzWithoutZ);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            XyzWithoutZ::Zero => "Kot 0 yaz",
            XyzWithoutZ::Skip => "Yazma",
        })
    }
}

#[derive(Debug)]
pub struct ExportState {
    pub vertices: XyzVertices,
    pub separator: XyzSeparator,
    pub decimals: Option<u8>,
    pub header: bool,
    pub without_z: XyzWithoutZ,
    /// What would be written, or why nothing.
    pub plan: Result<XyzExported, String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Which(Which),
    Separator(Sep),
    Places(Places),
    Header,
    Missing(Missing),
    /// Dışa aktar…: the path asked for.
    Save,
    /// The export's path; none when the dialog was cancelled.
    Saved(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(XyzExported, Vec<CommandWarning>), String>>),
}

fn event(e: Event) -> Message {
    message(Exchange::Xyz(e))
}

impl ExportState {
    fn input(&self, path: String, time: Option<kentos_commands::QueryTime>) -> ExportXyz {
        ExportXyz {
            path,
            layers: Vec::new(),
            vertices: self.vertices,
            separator: self.separator,
            decimals: self.decimals,
            header: self.header,
            without_z: self.without_z,
            // The dialog asked before writing over.
            overwrite: true,
            time,
        }
    }
}

impl App {
    /// The plan for the window's choices.
    pub(super) fn xyz_replan(&mut self) {
        let Some(Window::XyzExport(s)) = &self.exchange else {
            return;
        };
        let plan = match &self.document {
            Some(doc) => xyz_table::export(&doc.model, &s.input(self.export_name(".xyz"), self.export_time()))
                .map(|m| m.output)
                .map_err(|(_, message, _)| message),
            None => Err("Açık çizim yok.".into()),
        };
        if let Some(Window::XyzExport(s)) = &mut self.exchange {
            s.plan = plan;
        }
    }

    pub(super) fn xyz_export_open(&mut self) -> Task<Message> {
        self.open_window(Window::XyzExport(ExportState {
            vertices: XyzVertices::Points,
            separator: XyzSeparator::Space,
            decimals: None,
            header: false,
            without_z: XyzWithoutZ::Zero,
            plan: Err(String::new()),
            status: None,
        }));
        self.xyz_replan();
        Task::none()
    }

    pub(super) fn xyz_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Which(w) => {
                if let Some(Window::XyzExport(s)) = &mut self.exchange {
                    s.vertices = w.0;
                }
                self.xyz_replan();
            }
            Event::Separator(p) => {
                if let Some(Window::XyzExport(s)) = &mut self.exchange {
                    s.separator = p.0;
                }
                self.xyz_replan();
            }
            Event::Places(p) => {
                if let Some(Window::XyzExport(s)) = &mut self.exchange {
                    s.decimals = p.0;
                }
                self.xyz_replan();
            }
            Event::Header => {
                if let Some(Window::XyzExport(s)) = &mut self.exchange {
                    s.header = !s.header;
                }
                self.xyz_replan();
            }
            Event::Missing(m) => {
                if let Some(Window::XyzExport(s)) = &mut self.exchange {
                    s.without_z = m.0;
                }
                self.xyz_replan();
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Saved(Some(path.clone()))));
                }
                let suggested = self.export_name(".xyz");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("XYZ dışa aktar")
                            .set_file_name(suggested)
                            .add_filter("XYZ", &["xyz", "txt", "csv", "pts"])
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("xyz");
                        }
                        Some(path)
                    },
                    |p| event(Event::Saved(p)),
                );
            }
            Event::Saved(None) => {}
            Event::Saved(Some(path)) => {
                let time = self.export_time();
                let (Some(doc), Some(Window::XyzExport(s))) = (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let input = s.input(path.display().to_string(), time);
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_xyz(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Xyz(Event::Done(Box::new(r))),
                );
            }
            Event::Done(done) => match *done {
                Ok((out, warnings)) => {
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!("{} yazıldı: {} satır.", out.path, out.points),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::XyzExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("XYZ yazılamadı. {why}")));
                    }
                }
            },
        }
        Task::none()
    }

    pub(super) fn xyz_export_view<'a>(&'a self, s: &'a ExportState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        body = body.push(words::field(
            "Konumlar",
            Segmented::new(
                [Which(XyzVertices::Points), Which(XyzVertices::All)],
                Which(s.vertices),
                |w| event(Event::Which(w)),
            ),
            Some(match s.vertices {
                XyzVertices::Points => "Nokta nesnelerinin konumu; çizgiler ve alanlar yazılmaz.".to_owned(),
                XyzVertices::All => "Her nesnenin her köşesi: çizgiler, alanın dış ve delik halkaları (kapalı halkanın son köşesi bir kez), yayın uç noktaları.".to_owned(),
            }),
        ));
        body = body.push(words::field(
            "Ayırıcı",
            Segmented::new(
                [
                    Sep(XyzSeparator::Space),
                    Sep(XyzSeparator::Comma),
                    Sep(XyzSeparator::Semicolon),
                    Sep(XyzSeparator::Tab),
                ],
                Sep(s.separator),
                |p| event(Event::Separator(p)),
            ),
            None,
        ));
        body = body.push(words::field(
            "Ondalık basamak",
            Segmented::new(
                [
                    Places(None),
                    Places(Some(0)),
                    Places(Some(1)),
                    Places(Some(2)),
                    Places(Some(3)),
                    Places(Some(6)),
                ],
                Places(s.decimals),
                |p| event(Event::Places(p)),
            ),
            Some("En kısa: sayıya geri okunan en kısa metin; basamak verilirse hepsi o kadar basamakla yazılır.".to_owned()),
        ));
        body = body.push(words::field(
            "Kotu olmayan konum",
            Segmented::new(
                [Missing(XyzWithoutZ::Zero), Missing(XyzWithoutZ::Skip)],
                Missing(s.without_z),
                |m| event(Event::Missing(m)),
            ),
            Some("Dosyanın üçüncü sütunu hep vardır; kotsuz konum 0 kotla yazılır ya da yazılmaz. Çizimde konum kotsuz kalır.".to_owned()),
        ));
        body = body.push(words::check(
            s.header,
            "İlk satıra sütun adlarını (X Y Z) yaz",
            Some(event(Event::Header)),
        ));
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok(out) => {
                lines.push(words::text_line(
                    Line::Info,
                    format!("{} satır, {} nesneden; {} satırın kotu çizimin kotu.", out.points, out.objects, out.with_z),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    match out.srid {
                        Some(srid) => format!("Dosya koordinat sistemi taşımaz: sayılar projenin sisteminde (EPSG:{srid}), X doğu, Y kuzey."),
                        None => "Dosya koordinat sistemi taşımaz; projenin kayıtlı bir sistemi yok.".to_owned(),
                    },
                ));
                for l in &out.lost {
                    lines.push(words::text_line(Line::Warn, l.clone()));
                }
                if out.lost.is_empty() {
                    lines.push(words::text_line(Line::Ok, "Her konum olduğu gibi yazılır."));
                }
                true
            }
            Err(why) => {
                if !why.is_empty() {
                    lines.push(words::text_line(Line::Warn, why.clone()));
                }
                false
            }
        };
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
            Dialog::new("XYZ dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("Dışa aktar…", ready.then(|| event(Event::Save))))
                .width(660.0)
                .max_height(760.0),
        )
    }
}
