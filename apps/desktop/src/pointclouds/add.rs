//! Nokta bulutu ekle (docs/adr/0207 §8, §9; the web's
//! `ui/pointcloud/CloudAddDialog.ts`): LAS, LAZ, COPC or text clouds chosen
//! (a file, several, or a `.vpc`) or an address typed; each file's header read
//! off the interface's thread (a text cloud scanned whole once), with a
//! sample of its points for the look (a COPC's root node, else eight runs
//! spread over the file). The window shows the files, their format, version,
//! point format, points, bounds and system, whether an index is to be made,
//! and what the rule of the systems says (`place::rule`). Several files are
//! one virtual cloud or as many objects. Ekle writes a new layer named after
//! the file and the cloud on it as one undo step (Nokta bulutu ekle), linked
//! (its path), embedded (its bytes in the project's library, at most 32 MB)
//! or read from the address, and shows it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{
    CloudFormat, CloudSource, CommandResult, CreateOperation, EntitiesCreate, EntityGeometry,
    NewObject, PointCloudFields, PointCloudStyle,
};
use kentos_domain::NewLayer;
use kentos_interaction::{Format, Level};
use kentos_native_application::{ExecutionContext, create};
use kentos_pointcloud::look::Sample;
use kentos_pointcloud::place::{self, Rule};
use kentos_pointcloud::source::{Kind, Opening};
use kentos_pointcloud::{Step, text};
use kentos_ui::label;
use kentos_ui::widget::pairs::Pairs;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Clouds;
use super::bytes::{Bytes, Origin, Remote};
use crate::app::{App, Dialog as Asking, Message, Picker};
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "Nokta bulutu ekle";

static READS: AtomicU64 = AtomicU64::new(0);

/// The extensions the file dialog offers.
pub const EXTENSIONS: [&str; 9] = [
    "las", "laz", "copc.laz", "xyz", "pts", "txt", "csv", "vpc", "LAZ",
];

/// A file of the cloud as its header (or the `.vpc`) says.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub origin: Origin,
    pub name: String,
    pub format: CloudFormat,
    /// Its size in bytes; 0 when only the `.vpc` named it.
    pub bytes: u64,
    pub count: u64,
    pub bounds: [f64; 6],
    pub epsg: Option<u32>,
    /// The system's name as the file says it, when it names one without a code.
    pub crs_name: Option<String>,
    /// `1.4`, or none for a text cloud.
    pub version: Option<String>,
    pub point_format: Option<u8>,
    pub rgb: bool,
    /// A text cloud's skipped lines and coarser scale, said.
    pub notes: Vec<String>,
}

/// What the files' headers said.
#[derive(Debug, Clone)]
pub struct Read {
    pub members: Vec<Member>,
    /// The `.vpc` they came from, by its name.
    pub vpc: Option<String>,
    pub style: PointCloudStyle,
}

/// How the cloud is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    Linked,
    Embedded,
}

impl std::fmt::Display for Keep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Keep::Linked => "Bağlı",
            Keep::Embedded => "Göm",
        })
    }
}

/// Where the files come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum From {
    Files,
    Address,
}

impl std::fmt::Display for From {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            From::Files => "Dosya",
            From::Address => "Adres",
        })
    }
}

/// Several files: one virtual cloud, or an object each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Together {
    Virtual,
    Apart,
}

impl std::fmt::Display for Together {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Together::Virtual => "Tek sanal bulut",
            Together::Apart => "Ayrı nesneler",
        })
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub from: From,
    pub paths: Vec<PathBuf>,
    pub address: String,
    reading: u64,
    pub read: Option<Result<Read, String>>,
    /// The user said an unnamed system is the project's.
    pub confirmed: bool,
    pub keep: Keep,
    pub together: Together,
    pub layer: String,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The files to read, or none (the dialog cancelled).
    Picked(Option<Vec<PathBuf>>),
    /// Boxed: what the headers said is the largest event.
    Read(u64, Box<Result<Read, String>>),
    From(From),
    Address(String),
    /// Oku: the address's header read.
    ReadAddress,
    Confirm,
    Keep(Keep),
    Together(Together),
    Layer(String),
    Another,
    Run,
    Close,
}

fn msg(e: Event) -> Message {
    Message::PointClouds(Clouds::Add(e))
}

/// The format a file's name and first bytes say.
fn format_of(name: &str, head: &[u8]) -> CloudFormat {
    match kentos_pointcloud::vpc::format_of(name) {
        "copc" => CloudFormat::Copc,
        "laz" => CloudFormat::Laz,
        "las" if kentos_pointcloud::las::sniff(head) => CloudFormat::Las,
        _ if kentos_pointcloud::las::sniff(head) => CloudFormat::Las,
        _ => CloudFormat::Xyz,
    }
}

/// A file's header (or a text cloud's scan) read: the member, and a sample
/// of its points when `sample` asks for one.
fn member_of(origin: Origin, name: String, sample: Option<&mut Sample>) -> Result<Member, String> {
    let bytes = match &origin {
        Origin::File(p) => Bytes::file(p)?,
        Origin::Url(u) => Bytes::Remote(Remote::open(u)?),
        Origin::Asset(_) => return Err("Gömülü dosya bu pencereden eklenmez.".into()),
    };
    let size = bytes.size();
    let head = bytes.read(0, size.min(4))?;
    let named = format_of(&name, &head);
    if named == CloudFormat::Xyz {
        if matches!(origin, Origin::Url(_)) {
            return Err(format!(
                "“{name}” LAS, LAZ ya da COPC değil; adresten yalnız bunlar okunur. Metin bulutunu dosya olarak ekleyin."
            ));
        }
        let mut scan = text::Scan::new();
        let mut at = 0;
        while at < size {
            let n = (4u64 << 20).min(size - at);
            scan.feed(&bytes.read(at, n)?)
                .map_err(|e| format!("“{name}”: {}", e.0))?;
            at += n;
        }
        let plan = scan.finish().map_err(|e| format!("“{name}”: {}", e.0))?;
        let mut notes = Vec::new();
        if plan.skipped > 0 {
            let first = plan
                .first_skip
                .as_ref()
                .map(|(line, text)| format!(" (ilki {line}. satır: “{text}”)"))
                .unwrap_or_default();
            notes.push(format!(
                "{} satır sayı olmadığı için atlandı{first}.",
                plan.skipped
            ));
        }
        if plan.coarsened {
            notes.push(
                "Koordinatlar 32 bitlik tam sayıya sığmadığı için ölçek bir basamak kabalaştı."
                    .to_owned(),
            );
        }
        if let Some(sample) = sample {
            // The first pieces of the file: its points in the order written.
            let mut parse = text::Parse::new(plan.clone());
            let mut out = Vec::new();
            parse
                .feed(&bytes.read(0, size.min(4 << 20))?, &mut out)
                .map_err(|e| e.0)?;
            let layout =
                kentos_pointcloud::record::Layout::new(plan.columns.format(), parse.record_len());
            let step = (out.len() / layout.len.max(1))
                .div_ceil(place::SAMPLE_POINTS as usize)
                .max(1);
            sample.take(&layout, &out, plan.scale, plan.offset, step);
        }
        return Ok(Member {
            origin,
            name,
            format: CloudFormat::Xyz,
            bytes: size,
            count: plan.count,
            bounds: plan.bounds,
            epsg: None,
            crs_name: None,
            version: None,
            point_format: Some(plan.columns.format()),
            rgb: plan.columns.rgb(),
            notes,
        });
    }
    let mut o = Opening::new(size);
    let cloud = loop {
        match o.step().map_err(|e| format!("“{name}”: {}", e.0))? {
            Step::Done(c) => break c,
            Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
        }
    };
    if let Some(sample) = sample {
        let runs = place::sample_runs(&cloud);
        let step = place::sample_step(&runs);
        let mut records = Vec::new();
        for run in &runs {
            records.clear();
            let raw = bytes.read(run.need.offset, run.need.len)?;
            cloud
                .records(run, &raw, &mut records)
                .map_err(|e| format!("“{name}”: {}", e.0))?;
            sample.take(
                &cloud.layout,
                &records,
                cloud.head.scale,
                cloud.head.offset,
                step,
            );
        }
    }
    let info = cloud.info();
    Ok(Member {
        origin,
        name,
        format: match info.kind {
            Kind::Copc => CloudFormat::Copc,
            Kind::Laz => CloudFormat::Laz,
            Kind::Las => CloudFormat::Las,
        },
        bytes: size,
        count: info.count,
        bounds: info.bounds,
        epsg: info.crs.epsg,
        crs_name: info.crs.name.clone(),
        version: Some(info.version),
        point_format: Some(info.format),
        rgb: info.rgb,
        notes: Vec::new(),
    })
}

/// A `.vpc`'s members: what it says of each, the rest from the member's own
/// header (a sample from at most four of them, spread).
fn members_of_vpc(path: &Path, sample: &mut Sample) -> Result<Vec<Member>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("“{}” okunamadı: {e}.", path.display()))?;
    let listed = kentos_pointcloud::vpc::read(&text).map_err(|e| e.0)?;
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let n = listed.len();
    let sampled: Vec<usize> = (0..n.min(4))
        .map(|k| if n <= 4 { k } else { k * (n - 1) / 3 })
        .collect();
    let mut out = Vec::with_capacity(n);
    for (i, m) in listed.into_iter().enumerate() {
        let lower = m.href.to_ascii_lowercase();
        let origin = if lower.starts_with("http://") || lower.starts_with("https://") {
            Origin::Url(m.href.clone())
        } else {
            let p = Path::new(&m.href);
            Origin::File(if p.is_absolute() {
                p.to_path_buf()
            } else {
                folder.join(p)
            })
        };
        let name = super::source_name(
            &CloudSource {
                asset: None,
                file: matches!(origin, Origin::File(_)).then(|| m.href.clone()),
                url: matches!(origin, Origin::Url(_)).then(|| m.href.clone()),
                format: CloudFormat::Laz,
                count: 0,
                bounds: [0.0; 6],
            },
            None,
        );
        let wants_sample = sampled.contains(&i);
        let complete = m.count.is_some() && m.bounds.is_some() && m.epsg.is_some();
        if complete && !wants_sample {
            let format = match kentos_pointcloud::vpc::format_of(&m.href) {
                "copc" => CloudFormat::Copc,
                "laz" => CloudFormat::Laz,
                "las" => CloudFormat::Las,
                _ => CloudFormat::Xyz,
            };
            out.push(Member {
                origin,
                name,
                format,
                bytes: 0,
                count: m.count.unwrap_or(0),
                bounds: m.bounds.unwrap_or([0.0; 6]),
                epsg: m.epsg,
                crs_name: None,
                version: None,
                point_format: None,
                rgb: false,
                notes: Vec::new(),
            });
            continue;
        }
        let mut member = member_of(origin, name, wants_sample.then_some(&mut *sample))?;
        // What the `.vpc` says stands where the header says nothing.
        if member.epsg.is_none() {
            member.epsg = m.epsg;
        }
        out.push(member);
    }
    Ok(out)
}

/// The headers read: the files (or the address), and the new cloud's look from their sample.
pub fn inspect(paths: &[PathBuf], address: Option<&str>) -> Result<Read, String> {
    let mut sample = Sample::default();
    let mut vpc = None;
    let mut members = Vec::new();
    if let Some(url) = address {
        let url = url.trim();
        if let Some(why) = kentos_contracts::url_problem(url) {
            return Err(why);
        }
        let name = super::source_name(
            &CloudSource {
                asset: None,
                file: None,
                url: Some(url.to_owned()),
                format: CloudFormat::Copc,
                count: 0,
                bounds: [0.0; 6],
            },
            None,
        );
        members.push(member_of(
            Origin::Url(url.to_owned()),
            name,
            Some(&mut sample),
        )?);
    } else {
        let vpcs: Vec<&PathBuf> = paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("vpc")))
            .collect();
        if let Some(v) = vpcs.first() {
            if paths.len() > 1 {
                return Err("Sanal bulut dosyası (.vpc) tek başına seçilmeli.".into());
            }
            vpc = v.file_name().map(|n| n.to_string_lossy().into_owned());
            members = members_of_vpc(v, &mut sample)?;
        } else {
            let n = paths.len();
            for (i, p) in paths.iter().enumerate() {
                let name = p.file_name().map_or_else(
                    || p.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                // A sample from at most four files, spread.
                let wants = n <= 4 || [0, (n - 1) / 3, 2 * (n - 1) / 3, n - 1].contains(&i);
                members.push(member_of(
                    Origin::File(p.clone()),
                    name,
                    wants.then_some(&mut sample),
                )?);
            }
        }
    }
    if members.len() > kentos_contracts::MAX_CLOUD_SOURCES {
        return Err(format!(
            "Bir nokta bulutunun en çok {} dosyası olabilir; {} seçildi.",
            kentos_contracts::MAX_CLOUD_SOURCES,
            members.len()
        ));
    }
    let style = sample.default_style();
    Ok(Read {
        members,
        vpc,
        style,
    })
}

/// A size as the window says it.
pub fn size_words(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!(
            "{} GB",
            kentos_interaction::fixed(bytes as f64 / f64::from(1 << 30), 1).replace('.', ",")
        )
    } else if bytes >= 1 << 20 {
        format!(
            "{} MB",
            kentos_interaction::fixed(bytes as f64 / f64::from(1 << 20), 1).replace('.', ",")
        )
    } else {
        format!("{} KB", (bytes / 1024).max(1))
    }
}

/// A format's name as the windows say it.
pub fn format_words(f: CloudFormat) -> &'static str {
    match f {
        CloudFormat::Las => "LAS",
        CloudFormat::Laz => "LAZ",
        CloudFormat::Copc => "COPC",
        CloudFormat::Xyz => "Metin (XYZ)",
    }
}

/// The members' bounds together.
pub fn union(members: &[Member]) -> [f64; 6] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for m in members {
        for k in 0..3 {
            b[k] = b[k].min(m.bounds[k]);
            b[k + 3] = b[k + 3].max(m.bounds[k + 3]);
        }
    }
    b
}

impl App {
    /// `pointcloud.add`: the files asked for, then the window.
    /// `pointcloud.add`: the window, and the file dialog over it (closing the
    /// dialog leaves the window, for other files or an address).
    pub(crate) fn cloud_add_command(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(Some(vec![path.clone()]))));
        }
        self.cloud_add_open(From::Files, Vec::new(), String::new());
        if let Some(s) = &mut self.clouds.add {
            s.reading = 0;
        }
        self.cloud_add_pick()
    }

    fn cloud_add_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(Some(vec![path.clone()]))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title(TITLE)
                    .add_filter(
                        "Nokta bulutu (LAS, LAZ, COPC, XYZ, PTS, TXT, CSV, VPC)",
                        &EXTENSIONS,
                    )
                    .pick_files()
                    .await
                    .map(|files| files.iter().map(|f| f.path().to_path_buf()).collect())
            },
            |paths| msg(Event::Picked(paths)),
        )
    }

    fn cloud_add_open(&mut self, from: From, paths: Vec<PathBuf>, address: String) -> u64 {
        let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let layer = match (paths.as_slice(), from) {
            ([one], From::Files) => super::file_stem(&one.to_string_lossy()),
            ([first, ..], From::Files) => first.parent().and_then(Path::file_name).map_or_else(
                || "Nokta bulutu".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            ),
            (_, From::Address) if !address.trim().is_empty() => super::file_stem(address.trim()),
            _ => "Nokta bulutu".to_owned(),
        };
        let keep_layer = self
            .clouds
            .add
            .as_ref()
            .filter(|s| {
                s.from == from
                    && from == From::Address
                    && !s.layer.is_empty()
                    && address.trim().is_empty()
            })
            .map(|s| s.layer.clone());
        self.clouds.add = Some(State {
            from,
            paths,
            address,
            reading: id,
            read: None,
            confirmed: false,
            keep: Keep::Linked,
            together: Together::Virtual,
            layer: keep_layer.unwrap_or(layer),
            status: None,
        });
        self.dialog = Some(Asking::PointCloudAdd);
        id
    }

    pub(crate) fn cloud_add_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(paths)) if paths.is_empty() => return Task::none(),
            Event::Picked(Some(paths)) => {
                let id = self.cloud_add_open(From::Files, paths.clone(), String::new());
                return super::super::rasters::off_thread(move || {
                    msg(Event::Read(id, Box::new(inspect(&paths, None))))
                });
            }
            Event::Read(id, read) => {
                let read = *read;
                if let Some(s) = &mut self.clouds.add
                    && s.reading == id
                {
                    if let Ok(r) = &read
                        && r.members.iter().map(|m| m.bytes).max().unwrap_or(0)
                            > super::MOST_EMBEDDED as u64
                    {
                        s.keep = Keep::Linked;
                    }
                    s.read = Some(read);
                }
            }
            Event::From(f) => {
                if let Some(s) = &self.clouds.add
                    && s.from != f
                {
                    if f == From::Files {
                        self.cloud_add_open(From::Files, Vec::new(), String::new());
                        if let Some(s) = &mut self.clouds.add {
                            s.reading = 0;
                        }
                        return self.cloud_add_pick();
                    }
                    self.cloud_add_open(From::Address, Vec::new(), String::new());
                    if let Some(s) = &mut self.clouds.add {
                        s.read = None;
                        s.reading = 0;
                    }
                }
            }
            Event::Address(t) => {
                if let Some(s) = &mut self.clouds.add {
                    s.address = t;
                }
            }
            Event::ReadAddress => {
                let Some(address) = self.clouds.add.as_ref().map(|s| s.address.clone()) else {
                    return Task::none();
                };
                if address.trim().is_empty() {
                    return Task::none();
                }
                let id = self.cloud_add_open(From::Address, Vec::new(), address.clone());
                return super::super::rasters::off_thread(move || {
                    msg(Event::Read(id, Box::new(inspect(&[], Some(&address)))))
                });
            }
            Event::Confirm => {
                if let Some(s) = &mut self.clouds.add {
                    s.confirmed = !s.confirmed;
                }
            }
            Event::Keep(k) => {
                if let Some(s) = &mut self.clouds.add {
                    s.keep = k;
                }
            }
            Event::Together(t) => {
                if let Some(s) = &mut self.clouds.add {
                    s.together = t;
                }
            }
            Event::Layer(t) => {
                if let Some(s) = &mut self.clouds.add {
                    s.layer = t;
                }
            }
            Event::Another => return self.cloud_add_pick(),
            Event::Run => self.cloud_add_run(),
            Event::Close => {
                self.clouds.add = None;
                if self.dialog == Some(Asking::PointCloudAdd) {
                    self.dialog = None;
                }
            }
        }
        Task::none()
    }

    /// The project's system: its SRID and how a sentence names it.
    fn cloud_project(&self) -> (u32, String) {
        let Some(doc) = &self.document else {
            return (0, String::new());
        };
        let srid = doc.settings().srid;
        let name = kentos_project::systems::own(doc.settings())
            .map_or_else(|| "Yerel (koordinat sistemi yok)".to_owned(), |n| n.title);
        (srid, name)
    }

    fn cloud_add_rule(&self, r: &Read) -> Rule {
        let epsgs: Vec<Option<u32>> = r.members.iter().map(|m| m.epsg).collect();
        place::rule(&epsgs, self.cloud_project().0)
    }

    fn cloud_add_can(&self, s: &State) -> bool {
        let Some(Ok(r)) = &s.read else {
            return false;
        };
        place::srid_of(self.cloud_add_rule(r), s.confirmed).is_some()
            && !s.layer.trim().is_empty()
            && r.members.iter().all(|m| m.count > 0)
    }

    /// Ekle: the new layer and the cloud (or clouds) on it, one undo step; embedded first into the library.
    fn cloud_add_run(&mut self) {
        let Some(s) = self.clouds.add.clone() else {
            return;
        };
        let Some(Ok(read)) = s.read.clone() else {
            return;
        };
        let Some(srid) = place::srid_of(self.cloud_add_rule(&read), s.confirmed) else {
            return;
        };
        let mut library = Vec::new();
        let mut sources = Vec::with_capacity(read.members.len());
        for m in &read.members {
            let (mut asset, mut file, mut url) = (None, None, None);
            match &m.origin {
                Origin::File(p) if s.keep == Keep::Embedded => {
                    let bytes = match std::fs::read(p) {
                        Ok(b) => b,
                        Err(e) => {
                            self.cloud_add_status(format!("“{}” okunamadı: {e}.", m.name));
                            return;
                        }
                    };
                    match super::library_item(&m.name, &bytes, m.format) {
                        Ok((id, item)) => {
                            asset = Some(id.clone());
                            library.push((id, item));
                        }
                        Err(why) => {
                            self.cloud_add_status(why);
                            return;
                        }
                    }
                }
                Origin::File(p) => file = Some(p.to_string_lossy().into_owned()),
                Origin::Url(u) => url = Some(u.clone()),
                Origin::Asset(a) => asset = Some(a.clone()),
            }
            sources.push(CloudSource {
                asset,
                file,
                url,
                format: m.format,
                count: m.count,
                bounds: m.bounds,
            });
        }
        let make = |sources: Vec<CloudSource>, members: &[Member]| PointCloudFields {
            bounds: union(members),
            count: sources.iter().map(|s| s.count).sum(),
            sources,
            srid,
            style: read.style.clone(),
            opacity: None,
        };
        let clouds: Vec<PointCloudFields> = if s.together == Together::Apart && sources.len() > 1 {
            sources
                .into_iter()
                .zip(&read.members)
                .map(|(src, m)| make(vec![src], std::slice::from_ref(m)))
                .collect()
        } else {
            vec![make(sources, &read.members)]
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        for (id, item) in library {
            super::keep_in_library(model, &id, item);
        }
        let group = model.begin_group(TITLE);
        let layer = match model.add_layer(NewLayer::layer(s.layer.trim()), None, true) {
            Ok(id) => id,
            Err(why) => {
                model.cancel_group(group);
                self.cloud_add_status(why.0);
                return;
            }
        };
        let input = EntitiesCreate {
            layer_id: layer,
            objects: clouds
                .into_iter()
                .map(|c| NewObject {
                    geometry: EntityGeometry::PointCloud(c),
                    color: None,
                    line_weight: None,
                    attrs: None,
                    label: None,
                    label_of: None,
                    label_scale: None,
                    symbol: None,
                })
                .collect(),
            operation: Some(CreateOperation::PointCloud),
            expected_revision: None,
        };
        let result = create::execute(&mut ExecutionContext::new(model), input);
        match result {
            CommandResult::Completed { output, warnings } => {
                model.end_group(group);
                let slots: Vec<kentos_domain::Slot> =
                    output.ids.iter().map(|&i| kentos_domain::Slot(i)).collect();
                self.clouds.add = None;
                self.dialog = None;
                self.zoom_to(&slots);
                let what = match (&read.vpc, read.members.len(), s.together) {
                    (Some(v), _, _) => format!("“{v}” sanal bulut olarak"),
                    (None, 1, _) => format!("“{}”", read.members[0].name),
                    (None, n, Together::Virtual) => format!("{n} dosya tek sanal bulut olarak"),
                    (None, n, Together::Apart) => format!("{n} nokta bulutu"),
                };
                self.say(
                    Level::Success,
                    format!(
                        "{what} “{}” katmanına eklendi. Ctrl+Z geri alır.",
                        s.layer.trim()
                    ),
                );
                for w in warnings {
                    self.warn(w.message);
                }
                if read.members.iter().any(|m| m.format != CloudFormat::Copc) {
                    self.say(
                        Level::Info,
                        "Bulutun dizini bir kez hazırlanıyor; hazırlanırken yalnız çerçevesi çizilir (sağ altta, Durdur ile).",
                    );
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                model.cancel_group(group);
                self.cloud_add_status(error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                model.cancel_group(group);
                self.cloud_add_status("Nokta bulutu yazılamadı.".to_owned());
            }
        }
    }

    fn cloud_add_status(&mut self, why: String) {
        if let Some(s) = &mut self.clouds.add {
            s.status = Some(why);
        }
    }

    pub(crate) fn cloud_add_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.clouds.add else {
            return iced::widget::text("").into();
        };
        let from = Segmented::new([From::Files, From::Address], s.from, |f| {
            msg(Event::From(f))
        });
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Kaynak", from, None));
        if s.from == From::Address {
            let input = kentos_ui::widget::focus_ring(
                text_input("https://…/bulut.copc.laz", &s.address)
                    .on_input(|t| msg(Event::Address(t)))
                    .on_submit(msg(Event::ReadAddress))
                    .padding([5, 8])
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            );
            let read = button(label::body("Oku"))
                .on_press_maybe((!s.address.trim().is_empty()).then(|| msg(Event::ReadAddress)))
                .padding([5, 12])
                .style(kentos_ui::style::button::secondary);
            body = body.push(words::field(
                "Adres",
                row![container(input).width(Fill), read].spacing(8),
                Some(
                    "Herkese açık HTTP ya da HTTPS adresi; LAS, LAZ ve COPC parça parça okunur."
                        .to_owned(),
                ),
            ));
        }
        let meta = match &s.read {
            None if s.reading == 0 => None,
            None => Some("okunuyor…".to_owned()),
            Some(Err(_)) => Some("okunamadı".to_owned()),
            Some(Ok(r)) => {
                let count: u64 = r.members.iter().map(|m| m.count).sum();
                let bytes: u64 = r.members.iter().map(|m| m.bytes).sum();
                Some(format!(
                    "{} nokta{}",
                    crate::crs::grouped(count as f64),
                    if bytes > 0 {
                        format!(", {}", size_words(bytes))
                    } else {
                        String::new()
                    }
                ))
            }
        };
        if let Some(meta) = meta {
            let title = match (&s.read, s.paths.as_slice()) {
                (Some(Ok(Read { vpc: Some(v), .. })), _) => v.clone(),
                (Some(Ok(r)), _) if r.members.len() == 1 => r.members[0].name.clone(),
                (Some(Ok(r)), _) => format!("{} dosya", r.members.len()),
                (_, [one]) => one
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                (_, many) if many.len() > 1 => format!("{} dosya", many.len()),
                _ => super::file_stem(&s.address),
            };
            body = body.push(words::file_line(&title, meta));
        }
        match &s.read {
            None if s.reading == 0 && s.from == From::Files => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "Dosya seçilmedi: Dosya seç… ile LAS, LAZ, COPC, metin bulutu ya da sanal bulut (.vpc) seçin; ya da Kaynak'ta Adres'i seçip bir adres yazın.",
                )]))
            }
            None if s.reading == 0 => {}
            None => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "Dosyaların başlığı okunuyor…",
                )]))
            }
            Some(Err(why)) => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Error,
                    why.clone(),
                )]))
            }
            Some(Ok(r)) => {
                body = body.push(self.cloud_add_facts(r));
                body = body.push(self.cloud_add_rule_view(s, r));
                body = body.push(self.cloud_add_options(s, r));
            }
        }
        if let Some(why) = &s.status {
            body = body.push(words::text_line(Line::Error, why.clone()));
        }
        let mut dialog = Dialog::new(TITLE).scroll(body);
        if s.from == From::Files {
            let pick = if s.paths.is_empty() {
                "Dosya seç…"
            } else {
                "Başka dosya…"
            };
            dialog = dialog.action(words::ghost(pick, Some(msg(Event::Another))));
        }
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Ekle",
                    self.cloud_add_can(s).then(|| msg(Event::Run)),
                ))
                .width(660.0)
                .max_height(780.0),
        )
    }

    fn cloud_add_facts<'a>(&self, r: &'a Read) -> Element<'a, Message> {
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()));
        let b = union(&r.members);
        let extent = format!(
            "{} × {} {}, Z {} – {}",
            format.length_bare(b[3] - b[0]),
            format.length_bare(b[4] - b[1]),
            format.length_unit_label(),
            format.length_bare(b[2]),
            format.length_bare(b[5])
        );
        let count: u64 = r.members.iter().map(|m| m.count).sum();
        let area = (b[3] - b[0]) * (b[4] - b[1]);
        let density = if area > 0.0 {
            format!(
                "{} nokta/m²",
                kentos_interaction::fixed(count as f64 / area, 2).replace('.', ",")
            )
        } else {
            "—".to_owned()
        };
        let system = match r.members.first().and_then(|m| m.epsg) {
            Some(srid) => crate::crs::title_of(srid),
            None => r
                .members
                .first()
                .and_then(|m| m.crs_name.clone())
                .map_or_else(
                    || "Dosya söylemiyor".to_owned(),
                    |n| format!("{n} (kodsuz)"),
                ),
        };
        let index = if r.members.iter().all(|m| m.format == CloudFormat::Copc) {
            "Dosyanın kendi dizini var (COPC); hemen çizilir".to_owned()
        } else {
            "Dizin bir kez hazırlanır ve bu cihazın önbelleğinde tutulur; o sürece yalnız çerçevesi çizilir"
                .to_owned()
        };
        let mut pairs = Pairs::new();
        if let [m] = r.members.as_slice() {
            let version = match (&m.version, m.point_format) {
                (Some(v), Some(p)) => format!(
                    "LAS {v}, nokta biçimi {p}{}",
                    if m.rgb { ", renkli" } else { "" }
                ),
                (None, Some(p)) => format!(
                    "Metin, nokta biçimi {p}'ya çevrilir{}",
                    if m.rgb { ", renkli" } else { "" }
                ),
                _ => "—".to_owned(),
            };
            pairs = pairs
                .push(label::caption("Biçim"), label::body(format_words(m.format)))
                .push(label::caption("Sürüm"), label::body(version));
        } else {
            let kinds: Vec<&str> = {
                let mut k: Vec<&str> = r.members.iter().map(|m| format_words(m.format)).collect();
                k.sort_unstable();
                k.dedup();
                k
            };
            pairs = pairs.push(
                label::caption("Dosyalar"),
                label::body(format!("{} dosya ({})", r.members.len(), kinds.join(", "))),
            );
        }
        pairs = pairs
            .push(
                label::caption("Nokta sayısı"),
                label::body(crate::crs::grouped(count as f64)),
            )
            .push(label::caption("Kapsam"), label::body(extent))
            .push(label::caption("Yoğunluk"), label::body(density))
            .push(label::caption("Sistem"), label::body(system))
            .push(label::caption("Dizin"), label::body(index));
        let mut col = Column::new().spacing(8).push(pairs);
        if r.members.len() > 1 {
            let mut list = Column::new().spacing(2);
            for m in r.members.iter().take(12) {
                list = list.push(
                    row![
                        container(label::body(m.name.clone())).width(Fill),
                        label::caption(format!(
                            "{}, {} nokta",
                            format_words(m.format),
                            crate::crs::grouped(m.count as f64)
                        )),
                    ]
                    .spacing(8),
                );
            }
            if r.members.len() > 12 {
                list = list.push(label::caption(format!(
                    "… ve {} dosya daha",
                    r.members.len() - 12
                )));
            }
            col = col.push(list);
        }
        for m in &r.members {
            for n in &m.notes {
                col = col.push(words::text_line(Line::Warn, format!("{}: {n}", m.name)));
            }
        }
        container(col)
            .padding([8, 10])
            .width(Fill)
            .style(kentos_ui::style::container::bordered)
            .into()
    }

    fn cloud_add_rule_view<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let (_, project) = self.cloud_project();
        let line = match self.cloud_add_rule(r) {
            Rule::Same { .. } => words::text_line(
                Line::Ok,
                format!("Bulutun sistemi projeninkiyle aynı: {project}. Dosyanın dediği yere eklenir."),
            ),
            Rule::Other { srid } => words::text_line(
                Line::Error,
                format!(
                    "Bulutun sistemi ({}) projeninkinden ({project}) başka; bulut eklenmez. KentOS bulutu yeniden izdüşürmez: bulutu projenin sistemine çevirip yeniden deneyin.",
                    crate::crs::title_of(srid)
                ),
            ),
            Rule::Mixed => words::text_line(
                Line::Error,
                "Dosyalar farklı koordinat sistemleri söylüyor (ya da bazıları hiç söylemiyor); tek bulut olarak eklenmez. Aynı sistemdeki dosyaları seçin.",
            ),
            Rule::Unknown => words::line(
                Line::Warn,
                column![
                    label::body("Dosya koordinat sistemini söylemiyor. Bulut ancak projenin sisteminde olduğu söylenirse eklenir."),
                    words::check(
                        s.confirmed,
                        format!("Bulut projenin sisteminde ({project})"),
                        Some(msg(Event::Confirm)),
                    ),
                ]
                .spacing(6),
            ),
        };
        words::summary(vec![line])
    }

    fn cloud_add_options<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let mut out = Column::new().spacing(12);
        let files = r
            .members
            .iter()
            .all(|m| matches!(m.origin, Origin::File(_)));
        if r.members.len() > 1 && r.vpc.is_none() {
            let together = Segmented::new([Together::Virtual, Together::Apart], s.together, |t| {
                msg(Event::Together(t))
            });
            let hint = match s.together {
                Together::Virtual => {
                    "Dosyalar tek nesne olur; birlikte çizilir, seçilir ve işlenir."
                }
                Together::Apart => "Her dosya aynı katmanda ayrı bir nesne olur.",
            };
            out = out.push(words::field(
                "Birden çok dosya",
                together,
                Some(hint.to_owned()),
            ));
        }
        let largest = r.members.iter().map(|m| m.bytes).max().unwrap_or(0);
        let large = largest > super::MOST_EMBEDDED as u64;
        let keep: Element<'a, Message> = if files && r.vpc.is_none() {
            let keep = Segmented::new_with(
                [Keep::Linked, Keep::Embedded],
                s.keep,
                |k| msg(Event::Keep(k)),
                move |k| k == Keep::Linked || !large,
            );
            let hint = if large {
                format!(
                    "En büyük dosya {}; gömülü bulut en çok {} MB olabilir, bağlı kalır.",
                    size_words(largest),
                    super::MOST_EMBEDDED >> 20
                )
            } else if s.keep == Keep::Embedded {
                "Dosyanın baytları projenin kitaplığına alınır; çizim dosyayla birlikte taşınır."
                    .to_owned()
            } else {
                "Çizim dosyanın yolunu tutar; dosya yerinden oynarsa bulut bulunamaz.".to_owned()
            };
            words::field("Saklama", keep, Some(hint))
        } else if r.vpc.is_some() {
            words::field(
                "Saklama",
                label::body("Bağlı (sanal bulutun dosyaları)"),
                Some("Çizim dosyaların yollarını tutar.".to_owned()),
            )
        } else {
            words::field(
                "Saklama",
                label::body("Adres"),
                Some(
                    "Çizim adresi tutar; bulut her açılışta adresten parça parça okunur."
                        .to_owned(),
                ),
            )
        };
        let name = kentos_ui::widget::focus_ring(
            text_input("Katmanın adı", &s.layer)
                .on_input(|t| msg(Event::Layer(t)))
                .on_submit(msg(Event::Run))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(kentos_ui::style::field::input),
        );
        out = out.push(
            row![
                container(keep).width(iced::Length::FillPortion(3)),
                container(words::field(
                    "Yeni katman",
                    name,
                    Some("Bulut kendi katmanında durur.".to_owned())
                ))
                .width(iced::Length::FillPortion(2)),
            ]
            .spacing(16),
        );
        out.into()
    }
}

#[cfg(test)]
impl App {
    /// The window as the command opens it while its file dialog is up.
    pub(crate) fn cloud_add_open_for_tests(&mut self) {
        self.cloud_add_open(From::Files, Vec::new(), String::new());
        if let Some(s) = &mut self.clouds.add {
            s.reading = 0;
        }
    }
}
