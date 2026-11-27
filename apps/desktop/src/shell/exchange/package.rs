//! Paketle (docs/adr/0506, 0596, 2780): a window first asks, with bound layers, how their sources go (GeoPackages and
//! database tables each copied, kept as linked or embedded) and which links stay out of the package, and the open
//! formats the package delivers its project in too (Devir dosyaları, each read back and its losses reported); then the
//! folder is asked, and `file.package` runs on a copy of the drawing off the interface's thread (the files are read
//! whole and copied, a database table read and written as a GeoPackage): the project file, `veri/` with the files it
//! uses and `devir/` with the open files; what was left said one by one. Data layers (docs/adr/9338) are listed with
//! their rows, counted from their sources, and what the package does with each source; the open files' coverage of
//! their rows is said before the folder is asked.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandWarning, ExchangeFormat, ExchangeMatrix, PackageProject, PackageWay, ProjectPackaged, SourceKind};
use kentos_interaction::Level;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::source_rows::{Rows, SourceScopes};
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
    /// The data layers' rows, counted from their sources (docs/adr/9338).
    pub(crate) sources: SourceScopes,
    /// What the package takes and what stays out, as `file.package`'s plan says without reading a database
    /// (docs/adr/9342): asked when the window opens and again when a way or a link changes.
    pub preview: Option<Result<Preview, String>>,
}

/// The plan's lists: each file copied (its place in the package and where it comes from) and each link left as it is
/// (its place and why).
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub files: Vec<(String, String)>,
    pub left: Vec<(String, String)>,
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
    /// The plan's lists for the window (docs/adr/9342).
    Previewed(Result<Preview, String>),
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
        let sources = self.source_scopes();
        self.open_window(Window::Package(State {
            geopackages: PackageWay::Copy,
            databases: PackageWay::Keep,
            links,
            formats: Vec::new(),
            sources,
            preview: None,
        }));
        self.package_preview()
    }

    /// The plan's lists asked off the interface's thread: a folder that is not there stands for the package's.
    fn package_preview(&mut self) -> Task<Message> {
        let place = std::env::temp_dir().join(format!("kentos-paket-onizleme-{}", std::process::id()));
        let (Some(doc), Some(mut input)) = (&self.document, self.package_input(&place)) else {
            return Task::none();
        };
        // The open files are not made for a preview.
        input.exchange = None;
        let model = doc.model.clone();
        off_thread(
            move || {
                kentos_headless::package::plan_reading(&model, &input, false).map(|p| preview_of(&p.output)).map_err(|e| e.message)
            },
            |r| Exchange::Package(Event::Previewed(r)),
        )
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
                self.package_preview()
            }
            Event::Databases(Way(w)) => {
                if let Some(Window::Package(s)) = &mut self.exchange {
                    s.databases = w;
                }
                self.package_preview()
            }
            Event::Take(i) => {
                if let Some(Window::Package(s)) = &mut self.exchange
                    && let Some(l) = s.links.get_mut(i)
                {
                    l.take = !l.take;
                }
                self.package_preview()
            }
            Event::Previewed(r) => {
                if let Some(Window::Package(s)) = &mut self.exchange {
                    s.preview = Some(r);
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
                // The links whose files did not come (GIS-046): not there, or not readable. The package is whole only
                // without them; each is said with what keeps it and how to bind it again.
                let missing: Vec<&kentos_commands::PackageLeft> = out
                    .left
                    .iter()
                    .filter(|l| matches!(l.why, kentos_commands::LeftWhy::Missing | kentos_commands::LeftWhy::Unreadable))
                    .collect();
                let whole = if missing.is_empty() {
                    " Klasör başka bilgisayarda açılınca dış başvurular, bağlı dosyalar ve katmanlar kaynaklarını bulur.".to_owned()
                } else {
                    format!(
                        " Paket eksik: {} bağlı dosya bulunamadığı ya da okunamadığı için girmedi; aşağıda tek tek.",
                        missing.len()
                    )
                };
                self.announce(
                    if out.left.is_empty() && findings == 0 { Level::Success } else { Level::Warn },
                    format!(
                        "Paket yapıldı: {} ve {} dosya ({mb:.1} MB{extra}); içindekiler paket.json'da.{embedded}{whole}",
                        out.project,
                        out.files.len(),
                    ),
                );
                for l in &missing {
                    let why = if l.why == kentos_commands::LeftWhy::Missing { "bulunamadı" } else { "okunamadı" };
                    let who = self.package_holder(&l.at);
                    self.say(
                        Level::Warn,
                        format!(
                            "Eksik: “{}” {why}{who}. Dosyayı o yola koyun ya da bağlantıyı Öznitelikler'deki ekten yeniden bağlayın, sonra Paketle'yi yeniden çalıştırın.",
                            l.path
                        ),
                    );
                }
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

    /// What keeps a link left out, as the user knows it (GIS-046): “ (“P-5” nesnesinde, Parsel katmanı)” for an object
    /// by its lasting id, the reference otherwise; empty when it is not found.
    fn package_holder(&self, at: &str) -> String {
        let Some(doc) = &self.document else { return String::new() };
        let object = kentos_domain::Uuid::parse_str(at).ok().and_then(|u| doc.model.slot_of(u)).and_then(|s| doc.model.get(s));
        match object {
            Some(e) => {
                let label = e.base().label.as_deref().map(str::trim).filter(|l| !l.is_empty());
                let layer = doc.model.layers().get(&e.base().layer_id).map(|l| l.name.clone()).unwrap_or_default();
                match label {
                    Some(l) => format!(" (“{l}” nesnesinde, {layer} katmanı)"),
                    None => format!(" ({layer} katmanındaki bir nesnede)"),
                }
            }
            None if at.is_empty() => String::new(),
            None => format!(" ({at})"),
        }
    }

    pub(super) fn package_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let ways = [
            Way(PackageWay::Copy),
            Way(PackageWay::Keep),
            Way(PackageWay::Embed),
        ];
        let mut body = Column::new().spacing(12);
        let data = self.package_data_layers();
        if s.links.iter().any(|l| !l.database) || data.iter().any(|d| !d.database) {
            body = body.push(words::field(
                "GeoPackage kaynakları",
                Segmented::new(ways, Way(s.geopackages), |w| event(Event::GeoPackages(w))),
                Some(way_hint(s.geopackages, false, !data.is_empty()).to_owned()),
            ));
        }
        if s.links.iter().any(|l| l.database) || data.iter().any(|d| d.database) {
            body = body.push(words::field(
                "Veritabanı kaynakları",
                Segmented::new(ways, Way(s.databases), |w| event(Event::Databases(w))),
                Some(way_hint(s.databases, true, !data.is_empty()).to_owned()),
            ));
        }
        if !data.is_empty() {
            body = body.push(words::field(
                "Veri katmanları",
                self.package_data_list(s, &data),
                Some("Satırlar kaynaktadır; paket kaynağı kopyalar ya da adıyla bırakır. Kopyalanan kaynak paketin veri/ klasöründedir: paket başka klasöre taşınınca ve eski dosya yerinde olmasa da okunur.".to_owned()),
            ));
        }
        let services = self.package_services();
        if !services.is_empty() {
            body = body.push(words::field(
                "Servis katmanları",
                words::text_line(
                    Line::Info,
                    format!("{}: adresleriyle kalır. Servis salt okunurdur; verisi kopyalanmaz ve devir dosyalarına girmez.", services.join(", ")),
                ),
                None,
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
        lines.extend(self.package_coverage(s, &data));
        if s.databases == PackageWay::Copy && s.links.iter().any(|l| l.database && l.take) {
            lines.push(words::text_line(
                Line::Info,
                "Veritabanı tabloları paketlenirken okunur; bağın okuduğu satırlar (alanla bağlı katmanda yalnız alanınkiler) GeoPackage olarak veri/ altına yazılır.",
            ));
        }
        body = body.push(words::summary(lines));
        if let Some(list) = package_preview_view(s) {
            body = body.push(list);
        }
        overlay::blocking(
            Dialog::new("Paketle")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Klasör seç…",
                    self.package_ready(s, &data).then(|| event(Event::Pack)),
                ))
                .width(600.0)
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}

/// The plan's files and links, in words: “veri/qa.gpkg ← /iş/qa.gpkg”, “Ortofoto.tif: yerinde yok”.
fn preview_of(out: &ProjectPackaged) -> Preview {
    let kind = |k: kentos_commands::PackagedKind| match k {
        kentos_commands::PackagedKind::Source => "kaynak",
        kentos_commands::PackagedKind::Picture => "resim ya da raster",
        kentos_commands::PackagedKind::Sidecar => "raster yanı",
        kentos_commands::PackagedKind::Table => "tablo",
        kentos_commands::PackagedKind::Attachment => "ek",
        kentos_commands::PackagedKind::Xref => "dış başvuru",
        kentos_commands::PackagedKind::Font => "yazı tipi",
        kentos_commands::PackagedKind::Profile | kentos_commands::PackagedKind::Mapping => "profil",
        kentos_commands::PackagedKind::WorkFile => "harita işi",
        _ => "dosya",
    };
    let why = |w: kentos_commands::LeftWhy| match w {
        kentos_commands::LeftWhy::Missing => "yerinde yok",
        kentos_commands::LeftWhy::Database => "veritabanına bağlı kalır",
        kentos_commands::LeftWhy::Address => "adres",
        kentos_commands::LeftWhy::Substituted => "yerine başka yüz",
        kentos_commands::LeftWhy::Unreadable => "okunamadı",
        kentos_commands::LeftWhy::Kept => "bağlı kalır",
        kentos_commands::LeftWhy::Excluded => "dışarıda bırakıldı",
    };
    Preview {
        files: out.files.iter().map(|f| (f.path.clone(), format!("{} · {}", kind(f.kind), f.from))).collect(),
        left: out.left.iter().map(|l| (l.path.clone(), why(l.why).to_owned())).collect(),
    }
}

/// The plan's lists under the summary: what is copied into the package and what stays as it is; none before it
/// answers.
fn package_preview_view(s: &State) -> Option<Element<'_, Message>> {
    const SHOWN: usize = 10;
    let mut lines = Vec::new();
    match s.preview.as_ref()? {
        Err(why) => lines.push(words::text_line(Line::Warn, format!("Önizleme yapılamadı: {why}"))),
        Ok(p) => {
            if p.files.is_empty() && p.left.is_empty() {
                lines.push(words::text_line(Line::Info, "Pakette yalnız proje dosyası olur: bağlı dosyası yok."));
            }
            for (path, from) in p.files.iter().take(SHOWN) {
                lines.push(words::text_line(Line::Ok, format!("{path} ← {from}")));
            }
            if p.files.len() > SHOWN {
                lines.push(words::text_line(Line::Info, format!("… {} dosya daha kopyalanır.", p.files.len() - SHOWN)));
            }
            for (path, why) in p.left.iter().take(SHOWN) {
                lines.push(words::text_line(Line::Warn, format!("{path}: {why}, pakete kopyalanmaz.")));
            }
            if p.left.len() > SHOWN {
                lines.push(words::text_line(Line::Info, format!("… {} bağ daha olduğu gibi kalır.", p.left.len() - SHOWN)));
            }
        }
    }
    Some(words::field("Pakete girecekler", words::summary(lines), Some("Veritabanı tabloları ve devir dosyaları paketlenirken yazılır; burada sayılmaz.".to_owned())))
}

/// A data layer of the drawing as Paketle lists it.
pub(crate) struct DataLayerRow {
    pub id: String,
    pub name: String,
    /// The GeoPackage's file name, or the database's table.
    pub place: String,
    pub database: bool,
}

impl App {
    /// The drawing's data layers, in the tree's order.
    pub(crate) fn package_data_layers(&self) -> Vec<DataLayerRow> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .filter_map(|l| {
                let d = l.data.as_ref()?;
                let database = d.driver != "gpkg";
                let place = if database {
                    d.table.clone()
                } else {
                    let file = d.address.strip_prefix("gpkg:").unwrap_or(&d.address);
                    let name = std::path::Path::new(file).file_name().map_or_else(|| file.to_owned(), |n| n.to_string_lossy().into_owned());
                    format!("{name}, {}", d.table)
                };
                Some(DataLayerRow { id: l.id.clone(), name: l.name.clone(), place, database })
            })
            .collect()
    }

    /// Each data layer's rows as the window counts them (none: counted yet, or the source not read).
    #[cfg(test)]
    pub(super) fn package_rows_said(&self) -> Vec<Option<usize>> {
        let Some(Window::Package(s)) = &self.exchange else {
            return Vec::new();
        };
        self.package_data_layers().iter().map(|d| self.package_rows(s, &d.id).ok().flatten()).collect()
    }

    /// The service layers' names.
    fn package_services(&self) -> Vec<String> {
        self.document.as_ref().map_or_else(Vec::new, |doc| {
            doc.model.layers().leaves().into_iter().filter(|l| l.service.is_some()).map(|l| l.name.clone()).collect()
        })
    }

    /// A data layer's rows: those read from the source the drawing does not hold, and its own; none while read, why
    /// when they cannot be.
    fn package_rows(&self, s: &State, id: &str) -> Result<Option<usize>, String> {
        let held = self.document.as_ref().map_or(0, |d| d.model.by_layer(id).count());
        match s.sources.layers.iter().find(|l| l.id == id).map(|l| &l.rows) {
            Some(Rows::Ready { rows, .. }) => Ok(Some(rows.len() + held)),
            Some(Rows::Failed(why)) => Err(why.clone()),
            _ => Ok(None),
        }
    }

    fn package_data_list<'a>(&'a self, s: &'a State, data: &[DataLayerRow]) -> Element<'a, Message> {
        let mut list = Column::new().spacing(4);
        for d in data {
            let rows = match self.package_rows(s, &d.id) {
                Ok(Some(n)) => format!("{n} satır"),
                Ok(None) => "satırlar sayılıyor…".to_owned(),
                Err(_) => "kaynak okunamadı".to_owned(),
            };
            let way = match (d.database, if d.database { s.databases } else { s.geopackages }) {
                (false, PackageWay::Keep) => "bağlı kalır, dosya kopyalanmaz".to_owned(),
                (false, _) => "dosyası veri/ altına kopyalanır".to_owned(),
                (true, PackageWay::Copy) => "tablosu GeoPackage olarak veri/ altına kopyalanır".to_owned(),
                (true, _) => "veritabanına bağlı kalır, parola girmez".to_owned(),
            };
            let kind = if d.database { "veritabanı" } else { "GeoPackage" };
            list = list.push(words::text_line(
                if rows == "kaynak okunamadı" { Line::Error } else { Line::Info },
                format!("{}: {} ({kind}) · {rows} · {way}", d.name, d.place),
            ));
        }
        list.into()
    }

    /// The open files' coverage of the data layers' rows (docs/adr/9338), said before the folder is asked.
    fn package_coverage(&self, s: &State, data: &[DataLayerRow]) -> Vec<Element<'_, Message>> {
        if data.is_empty() || s.formats.is_empty() {
            return Vec::new();
        }
        let mut counted = Vec::new();
        let mut reading = Vec::new();
        let mut failed = Vec::new();
        for d in data {
            match self.package_rows(s, &d.id) {
                Ok(Some(n)) => counted.push((d.name.clone(), n)),
                Ok(None) => reading.push(d.name.clone()),
                Err(why) => failed.push((d.name.clone(), why)),
            }
        }
        let mut lines = Vec::new();
        if !counted.is_empty() {
            let total: usize = counted.iter().map(|(_, n)| n).sum();
            let each: Vec<String> = counted.iter().map(|(name, n)| format!("{name} {n}")).collect();
            lines.push(words::text_line(
                Line::Ok,
                format!("Devir dosyaları kaynak tablolardan {total} satırı kapsar: {}. Satırlar paketlenirken kaynaktan okunur; katmanlar devir dosyalarında bağsız, kendi satırlarıyla durur.", each.join(", ")),
            ));
        }
        if !reading.is_empty() {
            lines.push(words::text_line(
                Line::Info,
                format!("Kaynaktan sayılıyor: {}. Klasör seç sayım bitince açılır.", reading.join(", ")),
            ));
        }
        for (name, why) in failed {
            lines.push(words::text_line(
                Line::Error,
                format!("“{name}” katmanının kaynağı okunamadı: {why}. Devir dosyaları bu katmanı taşıyamaz; kaynağa ulaşın ya da devir dosyalarını kaldırın."),
            ));
        }
        lines
    }

    /// Whether Klasör seç is offered: with open files asked, every data layer's rows counted and readable.
    fn package_ready(&self, s: &State, data: &[DataLayerRow]) -> bool {
        s.formats.is_empty() || data.iter().all(|d| matches!(self.package_rows(s, &d.id), Ok(Some(_))))
    }
}

/// What a way does, under its control; `data`: the drawing has data layers.
fn way_hint(way: PackageWay, database: bool, data: bool) -> &'static str {
    if data && way == PackageWay::Embed {
        return if database {
            "Bağlı katmanlar projeye gömülür. Veri katmanları gömülmez, veritabanına bağlı kalır: veri katmanının satırları proje dosyasına yazılmaz."
        } else {
            "Bağlı katmanlar projeye gömülür. Veri katmanlarının dosyası gömülmez, veri/ altına kopyalanır: veri katmanının satırları proje dosyasına yazılmaz."
        };
    }
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
