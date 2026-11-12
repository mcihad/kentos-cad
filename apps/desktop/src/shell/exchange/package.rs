//! Paketle (docs/adr/0506, 0596, 2780): a window first asks, with bound layers, how their sources go (GeoPackages and
//! database tables each copied, kept as linked or embedded) and which links stay out of the package, and the open
//! formats the package delivers its project in too (Devir dosyaları, each read back and its losses reported); then the
//! folder is asked, and `file.package` runs on a copy of the drawing off the interface's thread (the files are read
//! whole and copied, a database table read and written as a GeoPackage): the project file, `veri/` with the files it
//! uses and `devir/` with the open files; what was left said one by one.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandWarning, ExchangeFormat, ExchangeMatrix, PackageProject, PackageWay, ProjectPackaged, SourceKind};
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
    /// Devir dosyaları (docs/adr/2780): the open formats ticked, in their order; none when the window opens.
    pub formats: Vec<ExchangeFormat>,
}

#[derive(Debug, Clone)]
pub enum Event {
    GeoPackages(Way),
    Databases(Way),
    /// A link ticked in or out.
    Take(usize),
    /// An open format ticked in or out (docs/adr/2780).
    Format(ExchangeFormat),
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
    /// Paketle: its window (the bound layers' ways and links when the drawing has any, and the open files).
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
        self.open_window(Window::Package(State {
            geopackages: PackageWay::Copy,
            databases: PackageWay::Keep,
            links,
            formats: Vec::new(),
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
            // The open formats, in the matrix's order (docs/adr/2780).
            if !s.formats.is_empty() {
                let formats = ExchangeFormat::ALL.into_iter().filter(|f| s.formats.contains(f)).collect();
                input.exchange = Some(ExchangeMatrix { formats, ..Default::default() });
            }
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
            Event::Format(f) => {
                if let Some(Window::Package(s)) = &mut self.exchange {
                    if let Some(at) = s.formats.iter().position(|x| *x == f) {
                        s.formats.remove(at);
                    } else {
                        s.formats.push(f);
                    }
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
                match count(&|f| f.kind == kentos_commands::PackagedKind::WorkFile) {
                    0 => {}
                    n => extra += &format!(", {n} harita işi dosyası"),
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
                // docs/adr/3180: the grids the project names, their notes not counted.
                match count(&|f| f.kind == kentos_commands::PackagedKind::Grid && f.links > 0) {
                    0 => {}
                    n => extra += &format!(", {n} ızgara"),
                }
                // The open files (docs/adr/2780): the formats and where their losses are.
                if let Some(m) = &out.exchange {
                    let names: Vec<&str> = m.checks.iter().map(|c| c.format.title()).collect();
                    extra += &format!(
                        ", {} devir dosyası: {}; kayıpları {}/{}'de",
                        names.len(),
                        names.join(", "),
                        kentos_headless::package_exchange::FOLDER,
                        kentos_headless::package_exchange::REPORT
                    );
                }
                let mut embedded = match out.embedded.len() {
                    0 => String::new(),
                    n => format!(" {n} bağlı katman projeye gömüldü."),
                };
                // The resources' report (docs/adr/1111): how many, and the findings among them.
                let findings = out
                    .resources
                    .iter()
                    .filter(|r| {
                        matches!(
                            r.how,
                            kentos_commands::PackageResourceHow::Missing
                                | kentos_commands::PackageResourceHow::Carried
                        )
                    })
                    .count();
                if !out.resources.is_empty() {
                    embedded += &format!(
                        " Yazı tipi, SHX, çizgi tipi, tarama deseni ve baskı stili raporu: {} kaynak{}.",
                        out.resources.len(),
                        match findings {
                            0 => String::new(),
                            n => format!(", {n} tanesi pakette tanımlı değil"),
                        }
                    );
                }
                self.announce(
                    if out.left.is_empty() && findings == 0 { Level::Success } else { Level::Warn },
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
        if !s.links.is_empty() {
            body = body.push(words::field(
                "Pakete alınacak bağlı katmanlar",
                list,
                Some("İşareti kaldırılan katmanın bağı ve nesneleri pakete girmez: lisansı ya da erişimi dağıtmaya izin vermeyen veri için. Çizim değişmez.".to_owned()),
            ));
        }
        // Devir dosyaları (docs/adr/2780): the open formats, three to a row.
        let mut formats = Column::new().spacing(4);
        for three in ExchangeFormat::ALL.chunks(3) {
            let mut line = iced::widget::Row::new().spacing(16);
            for &f in three {
                line = line.push(
                    iced::widget::container(words::check(s.formats.contains(&f), f.title(), Some(event(Event::Format(f)))))
                        .width(kentos_ui::theme::typography::scaled(120.0)),
                );
            }
            formats = formats.push(line);
        }
        body = body.push(words::field(
            "Devir dosyaları",
            formats,
            Some(format!(
                "İşaretli biçimlerde projenin açık kopyası {}/ altına yazılır, KentOS'un okuyucusuyla geri okunur; neyin korunup neyin kaybolduğu paket.json'da ve {}/{}'de. Proje dosyası kaynaktır.",
                kentos_headless::package_exchange::FOLDER,
                kentos_headless::package_exchange::FOLDER,
                kentos_headless::package_exchange::REPORT
            )),
        ));
        let out = s.links.iter().filter(|l| !l.take).count();
        let mut lines = Vec::new();
        if !s.links.is_empty() {
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
        }
        lines.push(words::text_line(
            Line::Info,
            match s.formats.len() {
                0 => "Devir dosyası yok: proje dosyası ve kullandığı dosyalar.".to_owned(),
                n => format!(
                    "{n} biçimde devir dosyası: {}.{}",
                    ExchangeFormat::ALL.into_iter().filter(|f| s.formats.contains(f)).map(|f| f.title()).collect::<Vec<_>>().join(", "),
                    if s.links.is_empty() { "" } else { " Dışarıda bırakılan katmanın nesneleri bunlara da girmez." }
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
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
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
