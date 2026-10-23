//! Paketle (docs/adr/0506, 0596): with bound layers, a window first asks how their sources go (GeoPackages and
//! database tables each copied, kept as linked or embedded) and which links stay out of the package; then the folder is
//! asked, and `file.package` runs on a copy of the drawing off the interface's thread (the files are read whole and
//! copied, a database table read and written as a GeoPackage): the project file and `veri/` with the files it uses;
//! what was left said one by one.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandWarning, PackageProject, PackageWay, ProjectPackaged, SourceKind};
use kentos_interaction::Level;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// A way, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Way(pub PackageWay);

impl fmt::Display for Way {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            PackageWay::Copy => "Kopyala",
            PackageWay::Keep => "Bağlı bırak",
            PackageWay::Embed => "Göm",
        })
    }
}

/// A bound layer's link as the window lists it.
#[derive(Debug, Clone)]
pub struct Link {
    pub id: String,
    /// The layer's name and the table (`parsel`, `kadastro.parsel`).
    pub layer: String,
    pub table: String,
    pub database: bool,
    /// Whether it goes into the package (unticked: `leaveOut`).
    pub take: bool,
}

#[derive(Debug)]
pub struct State {
    pub geopackages: PackageWay,
    pub databases: PackageWay,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone)]
pub enum Event {
    GeoPackages(Way),
    Databases(Way),
    /// A link ticked in or out.
    Take(usize),
    /// Paketle…: the folder asked for.
    Pack,
    /// The folder to make; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    /// Packed, or why not.
    Done(Result<Box<(ProjectPackaged, Vec<CommandWarning>)>, String>),
}

fn event(e: Event) -> Message {
    message(Exchange::Package(e))
}

impl App {
    /// Paketle: its window when the drawing has bound layers, else the folder at once.
    pub(super) fn package_open(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let links: Vec<Link> = doc
            .model
            .settings()
            .sources
            .iter()
            .map(|l| Link {
                id: l.id.clone(),
                layer: doc
                    .model
                    .layers()
                    .get(&l.layer)
                    .map_or_else(|| l.layer.clone(), |n| n.name.clone()),
                table: l.table.clone(),
                database: l.kind == SourceKind::Postgis,
                take: true,
            })
            .collect();
        if links.is_empty() {
            return self.package_pick();
        }
        self.open_window(Window::Package(State {
            geopackages: PackageWay::Copy,
            databases: PackageWay::Keep,
            links,
        }));
        Task::none()
    }

    pub(super) fn package_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        let name = self
            .document
            .as_ref()
            .map_or_else(|| "proje".to_owned(), |d| format!("{}-paket", d.name()));
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Paketle: paketin klasörü")
                    .set_file_name(name)
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Picked(p)),
        )
    }

    /// The package's input for the folder `path`: the window's ways and the links ticked out, or the defaults.
    pub(crate) fn package_input(&self, path: &std::path::Path) -> Option<PackageProject> {
        let doc = self.document.as_ref()?;
        let mut input = PackageProject {
            path: path.display().to_string(),
            from: doc
                .path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.display().to_string()),
            ..Default::default()
        };
        if let Some(Window::Package(s)) = &self.exchange {
            input.geopackages = Some(s.geopackages);
            input.databases = Some(s.databases);
            input.leave_out = s
                .links
                .iter()
                .filter(|l| !l.take)
                .map(|l| l.id.clone())
                .collect();
        }
        Some(input)
    }

    pub(super) fn package_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::GeoPackages(Way(w)) => {
                if let Some(Window::Package(s)) = &mut self.exchange {
                    s.geopackages = w;
                }
                Task::none()
            }
            Event::Databases(Way(w)) => {
                if let Some(Window::Package(s)) = &mut self.exchange {
                    s.databases = w;
                }
                Task::none()
            }
            Event::Take(i) => {
                if let Some(Window::Package(s)) = &mut self.exchange
                    && let Some(l) = s.links.get_mut(i)
                {
                    l.take = !l.take;
                }
                Task::none()
            }
            Event::Pack => self.package_pick(),
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => {
                let (Some(doc), Some(input)) = (&self.document, self.package_input(&path)) else {
                    return Task::none();
                };
                let model = doc.model.clone();
                self.close_exchange();
                self.output(format!("{} paketleniyor…", path.display()));
                off_thread(
                    move || {
                        let planned = kentos_headless::package::plan(&model, &input)
                            .map_err(|e| e.message)?;
                        kentos_headless::package::write(&planned).map_err(|e| e.message)?;
                        Ok(Box::new((planned.output, planned.warnings)))
                    },
                    |r| Exchange::Package(Event::Done(r)),
                )
            }
            Event::Done(Ok(done)) => {
                let (out, warnings) = *done;
                let mb = out.bytes as f64 / (1024.0 * 1024.0);
                let count = |f: &dyn Fn(&kentos_commands::PackagedFile) -> bool| {
                    out.files.iter().filter(|x| f(x)).count()
                };
                let mut extra = String::new();
                match count(&|f| f.kind == kentos_commands::PackagedKind::Xref) {
                    0 => {}
                    n => extra += &format!(", {n} dış başvuru kaynağı"),
                }
                match count(&|f| {
                    matches!(
                        f.kind,
                        kentos_commands::PackagedKind::Profile
                            | kentos_commands::PackagedKind::Mapping
                    )
                }) {
                    0 => {}
                    n => extra += &format!(", {n} profil ve eşleme dosyası"),
                }
                match count(&|f| f.table.is_some()) {
                    0 => {}
                    n => extra += &format!(", {n} veritabanı tablosunun kopyası"),
                }
                let embedded = match out.embedded.len() {
                    0 => String::new(),
                    n => format!(" {n} bağlı katman projeye gömüldü."),
                };
                self.announce(
                    if out.left.is_empty() { Level::Success } else { Level::Warn },
                    format!(
                        "Paket yapıldı: {} ve {} dosya ({mb:.1} MB{extra}); içindekiler paket.json'da.{embedded} Klasör başka bilgisayarda açılınca dış başvurular, bağlı dosyalar ve katmanlar kaynaklarını bulur.",
                        out.project,
                        out.files.len(),
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
                Task::none()
            }
            Event::Done(Err(why)) => {
                self.announce(Level::Error, format!("Paket yapılamadı. {why}"));
                Task::none()
            }
        }
    }

    pub(super) fn package_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let ways = [
            Way(PackageWay::Copy),
            Way(PackageWay::Keep),
            Way(PackageWay::Embed),
        ];
        let mut body = Column::new().spacing(12);
        if s.links.iter().any(|l| !l.database) {
            body = body.push(words::field(
                "GeoPackage kaynakları",
                Segmented::new(ways, Way(s.geopackages), |w| event(Event::GeoPackages(w))),
                Some(way_hint(s.geopackages, false).to_owned()),
            ));
        }
        if s.links.iter().any(|l| l.database) {
            body = body.push(words::field(
                "Veritabanı kaynakları",
                Segmented::new(ways, Way(s.databases), |w| event(Event::Databases(w))),
                Some(way_hint(s.databases, true).to_owned()),
            ));
        }
        let mut list = Column::new().spacing(4);
        for (i, l) in s.links.iter().enumerate() {
            let place = if l.database {
                "veritabanı"
            } else {
                "GeoPackage"
            };
            list = list.push(words::check(
                l.take,
                format!("{} — {} ({place})", l.layer, l.table),
                Some(event(Event::Take(i))),
            ));
        }
        body = body.push(words::field(
            "Pakete alınacak bağlı katmanlar",
            list,
            Some("İşareti kaldırılan katmanın bağı ve nesneleri pakete girmez: lisansı ya da erişimi dağıtmaya izin vermeyen veri için. Çizim değişmez.".to_owned()),
        ));
        let out = s.links.iter().filter(|l| !l.take).count();
        let mut lines = Vec::new();
        lines.push(words::text_line(
            Line::Info,
            match out {
                0 => format!("{} bağlı katmanın hepsi pakete girer.", s.links.len()),
                n => format!(
                    "{} bağlı katmandan {n} tanesi dışarıda kalır, nesneleri pakete girmez.",
                    s.links.len()
                ),
            },
        ));
        if s.databases == PackageWay::Copy && s.links.iter().any(|l| l.database && l.take) {
            lines.push(words::text_line(
                Line::Info,
                "Veritabanı tabloları paketlenirken okunur; bağın okuduğu satırlar (alanla bağlı katmanda yalnız alanınkiler) GeoPackage olarak veri/ altına yazılır.",
            ));
        }
        body = body.push(words::summary(lines));
        overlay::blocking(
            Dialog::new("Paketle")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("Klasör seç…", Some(event(Event::Pack))))
                .width(600.0)
                .max_height(720.0),
        )
    }
}

/// What a way does, under its control.
fn way_hint(way: PackageWay, database: bool) -> &'static str {
    match (way, database) {
        (PackageWay::Copy, false) => {
            "Dosyalar bayt bayt veri/ altına kopyalanır; bağ kopyayı gösterir, paket nereye giderse orada bulunur."
        }
        (PackageWay::Copy, true) => {
            "Tablolar GeoPackage olarak veri/ altına kopyalanır; paket veritabanına ulaşmadan açılır ve düzenlenir."
        }
        (PackageWay::Keep, false) => {
            "Bağ olduğu gibi kalır, dosya kopyalanmaz; paketi açan bilgisayar dosyaya aynı yoldan ulaşmalı."
        }
        (PackageWay::Keep, true) => {
            "Bağ olduğu gibi kalır; paketi açan bilgisayar veritabanına kendisi bağlanır. Parola pakete girmez."
        }
        (PackageWay::Embed, _) => {
            "Pakette bağ kalkar; katman nesneleriyle projenin kendi katmanı olur, kaynağa yazılmaz."
        }
    }
}
