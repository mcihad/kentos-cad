//! Nokta bulutu alan istatistiği, Nokta bulutu profili and Mesh sor (docs/adr/0552, plan/gis.md CBS-32): three windows
//! over the read-only commands `cad.pointclouds.region`, `cad.pointclouds.profile` and `cad.meshes.query`. The shape
//! is the selection's (an area, a line or a polyline, a point; Seçimden al reads it again), the other values are typed;
//! Çalıştır reads the files off the window's thread at the source's full resolution and shows what the command answers:
//! counts, heights and classes of the area, the stations of the corridor, the face and the values at the place. Panoya
//! kopyala takes the answer as text. A script and the window do the same.

use std::fmt::Write as _;

use iced::widget::{Column, text_input};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CommandResult, CommandWarning, Entity, MeshesQueried, MeshesQuery, PointCloudsProfile,
    PointCloudsProfiled, PointCloudsRegion, PointCloudsRegioned, Vec2,
};
use kentos_native_application::{ExecutionContext, meshes, pointcloud_areas};
use kentos_ui::widget::{Dialog, focus_ring, overlay};
use kentos_ui::style;

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};

/// The rows of a table shown at most; the copy holds them all.
const SHOWN: usize = 60;

/// What a window asked the command, and what it answered.
#[derive(Debug, Clone)]
pub enum Outcome {
    Region(Result<(PointCloudsRegioned, Vec<CommandWarning>), String>),
    Profile(Result<(PointCloudsProfiled, Vec<CommandWarning>), String>),
    Mesh(Result<(MeshesQueried, Vec<CommandWarning>), String>),
}

/// An area taken from the selection: its ring and holes in plan, what it is said to be.
#[derive(Debug, Clone)]
pub struct Shape {
    pub ring: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
    pub said: String,
}

#[derive(Debug)]
pub struct RegionState {
    pub shape: Result<Shape, String>,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug)]
pub struct ProfileState {
    /// The path taken from the selection and what it is said to be.
    pub path: Result<(Vec<Vec2>, String), String>,
    pub width: String,
    pub step: String,
    pub points: bool,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug)]
pub struct MeshState {
    pub x: String,
    pub y: String,
    /// A moment as the windows write it (`2024-05-01 06:30`); empty: the steps the raster shows.
    pub time: String,
    pub series: bool,
    pub result: Option<Outcome>,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Seçimden al: the shape, the path or the place read from the selection again.
    Use,
    Width(String),
    Step(String),
    X(String),
    Y(String),
    Time(String),
    /// Noktaları da listele, Her adımı göster.
    Toggle,
    Run,
    Done(Box<Outcome>),
    Copy,
}

fn event(e: Event) -> Message {
    message(Exchange::Area(e))
}

fn fixed(v: f64) -> String {
    let t = format!("{v:.3}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-" { "0".to_owned() } else { t.to_owned() }
}

/// The area the selection holds: its first area (or closed line), its holes; why not.
fn shape_of(app: &App) -> Result<Shape, String> {
    let Some(doc) = &app.document else {
        return Err("Açık çizim yok.".into());
    };
    for slot in app.selection.ids() {
        let Some(e) = doc.model.get(*slot) else { continue };
        let path = match e {
            Entity::Polygon(p) => Some(p),
            Entity::Polyline(p) if p.closed => Some(p),
            _ => None,
        };
        let Some(p) = path else { continue };
        let curved = p.bulges.as_ref().is_some_and(|b| b.iter().any(|v| *v != 0.0));
        let mut said = format!("{} köşeli alan", p.pts.len());
        if let Some(h) = &p.holes
            && !h.is_empty()
        {
            let _ = write!(said, ", {} delik", h.len());
        }
        if p.parts.as_ref().is_some_and(|x| !x.is_empty()) {
            said.push_str("; yalnız ilk parçası");
        }
        if curved {
            said.push_str("; yaylı kenarlar köşelerini birleştiren doğru sayılır");
        }
        return Ok(Shape {
            ring: p.pts.clone(),
            holes: p
                .holes
                .iter()
                .flatten()
                .map(|h| h.pts.clone())
                .collect(),
            said,
        });
    }
    Err("Seçimde alan yok: bir alan (ya da kapalı çoklu çizgi) seçip Seçimden al'a basın.".into())
}

/// The line or polyline the selection holds; why not.
fn path_of(app: &App) -> Result<(Vec<Vec2>, String), String> {
    let Some(doc) = &app.document else {
        return Err("Açık çizim yok.".into());
    };
    for slot in app.selection.ids() {
        match doc.model.get(*slot) {
            Some(Entity::Line(l)) => return Ok((vec![l.a, l.b], "2 köşeli çizgi".into())),
            Some(Entity::Polyline(p)) if !p.closed => {
                return Ok((p.pts.clone(), format!("{} köşeli çoklu çizgi", p.pts.len())));
            }
            _ => {}
        }
    }
    Err("Seçimde çizgi yok: bir çizgi ya da çoklu çizgi seçip Seçimden al'a basın.".into())
}

/// The first selected point, its place as typed text.
fn place_of(app: &App) -> Option<(String, String)> {
    let doc = app.document.as_ref()?;
    app.selection.ids().iter().find_map(|s| match doc.model.get(*s) {
        Some(Entity::Point(p)) => Some((fixed(p.p.x), fixed(p.p.y))),
        _ => None,
    })
}

fn answered<T>(r: CommandResult<T>, nothing: &str) -> Result<(T, Vec<CommandWarning>), String> {
    match r {
        CommandResult::Completed { output, warnings } => Ok((output, warnings)),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err(nothing.to_owned()),
    }
}

impl App {
    pub(super) fn region_open(&mut self) -> Task<Message> {
        let shape = shape_of(self);
        self.open_window(Window::CloudRegion(RegionState {
            shape,
            result: None,
            running: false,
        }));
        Task::none()
    }

    pub(super) fn profile_open(&mut self) -> Task<Message> {
        let path = path_of(self);
        self.open_window(Window::CloudProfile(ProfileState {
            path,
            width: "10".into(),
            step: "25".into(),
            points: false,
            result: None,
            running: false,
        }));
        Task::none()
    }

    pub(super) fn mesh_query_open(&mut self) -> Task<Message> {
        let (x, y) = place_of(self).unwrap_or_default();
        self.open_window(Window::MeshQuery(MeshState {
            x,
            y,
            time: String::new(),
            series: false,
            result: None,
            running: false,
        }));
        Task::none()
    }

    pub(super) fn area_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Use => {
                match &mut self.exchange {
                    Some(Window::CloudRegion(_)) => {
                        let shape = shape_of(self);
                        if let Some(Window::CloudRegion(s)) = &mut self.exchange {
                            s.shape = shape;
                            s.result = None;
                        }
                    }
                    Some(Window::CloudProfile(_)) => {
                        let path = path_of(self);
                        if let Some(Window::CloudProfile(s)) = &mut self.exchange {
                            s.path = path;
                            s.result = None;
                        }
                    }
                    Some(Window::MeshQuery(_)) => {
                        let at = place_of(self);
                        if let (Some(Window::MeshQuery(s)), Some((x, y))) = (&mut self.exchange, at) {
                            s.x = x;
                            s.y = y;
                            s.result = None;
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Event::Width(t) => {
                if let Some(Window::CloudProfile(s)) = &mut self.exchange {
                    s.width = t;
                }
                Task::none()
            }
            Event::Step(t) => {
                if let Some(Window::CloudProfile(s)) = &mut self.exchange {
                    s.step = t;
                }
                Task::none()
            }
            Event::X(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.x = t;
                }
                Task::none()
            }
            Event::Y(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.y = t;
                }
                Task::none()
            }
            Event::Time(t) => {
                if let Some(Window::MeshQuery(s)) = &mut self.exchange {
                    s.time = t;
                }
                Task::none()
            }
            Event::Toggle => {
                match &mut self.exchange {
                    Some(Window::CloudProfile(s)) => s.points = !s.points,
                    Some(Window::MeshQuery(s)) => s.series = !s.series,
                    _ => {}
                }
                Task::none()
            }
            Event::Run => self.area_run(),
            Event::Done(outcome) => {
                match (&mut self.exchange, *outcome) {
                    (Some(Window::CloudRegion(s)), o @ Outcome::Region(_)) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    (Some(Window::CloudProfile(s)), o @ Outcome::Profile(_)) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    (Some(Window::MeshQuery(s)), o @ Outcome::Mesh(_)) => {
                        s.result = Some(o);
                        s.running = false;
                    }
                    _ => {}
                }
                Task::none()
            }
            Event::Copy => {
                let text = match &self.exchange {
                    Some(Window::CloudRegion(s)) => s.result.as_ref().map(copy_text),
                    Some(Window::CloudProfile(s)) => s.result.as_ref().map(copy_text),
                    Some(Window::MeshQuery(s)) => s.result.as_ref().map(copy_text),
                    _ => None,
                };
                text.map_or_else(Task::none, iced::clipboard::write)
            }
        }
    }

    fn area_run(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = doc.model.clone();
        let drawing = doc.path.as_deref().and_then(|p| p.to_str()).map(str::to_owned);
        let done = |o: Outcome| Exchange::Area(Event::Done(Box::new(o)));
        match &mut self.exchange {
            Some(Window::CloudRegion(s)) => {
                let Ok(shape) = &s.shape else {
                    return Task::none();
                };
                let input = PointCloudsRegion {
                    ring: shape.ring.clone(),
                    holes: shape.holes.clone(),
                    uids: Vec::new(),
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Region(answered(
                            pointcloud_areas::region::plan(&ExecutionContext::new(&mut model), &input),
                            "Alan okunamadı.",
                        ))
                    },
                    done,
                )
            }
            Some(Window::CloudProfile(s)) => {
                let Ok((path, _)) = &s.path else {
                    return Task::none();
                };
                let (width, step) = (parse(&s.width), parse(&s.step));
                let (Some(width), Some(step)) = (width, step) else {
                    s.result = Some(Outcome::Profile(Err(
                        "Genişlik ve adım sayı olmalı (örneğin 10 ve 25).".into(),
                    )));
                    return Task::none();
                };
                let input = PointCloudsProfile {
                    path: path.clone(),
                    width,
                    step,
                    points: s.points,
                    uids: Vec::new(),
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Profile(answered(
                            pointcloud_areas::profile::plan(&ExecutionContext::new(&mut model), &input),
                            "Profil okunamadı.",
                        ))
                    },
                    done,
                )
            }
            Some(Window::MeshQuery(s)) => {
                let (x, y) = (parse(&s.x), parse(&s.y));
                let (Some(x), Some(y)) = (x, y) else {
                    s.result = Some(Outcome::Mesh(Err("Yerin X ve Y'si sayı olmalı.".into())));
                    return Task::none();
                };
                let time = if s.time.trim().is_empty() {
                    None
                } else {
                    match kentos_geometry_core::time::read(s.time.trim()).moment() {
                        Some(t) => Some(t),
                        None => {
                            s.result = Some(Outcome::Mesh(Err(
                                "An okunamadı; 2024-05-01 06:30 gibi yazın.".into(),
                            )));
                            return Task::none();
                        }
                    }
                };
                let input = MeshesQuery {
                    at: Vec2 { x, y },
                    uids: Vec::new(),
                    time,
                    variables: Vec::new(),
                    series: s.series,
                    drawing,
                };
                s.running = true;
                off_thread(
                    move || {
                        let mut model = model;
                        Outcome::Mesh(answered(
                            meshes::query::plan(&ExecutionContext::new(&mut model), &input),
                            "Mesh okunamadı.",
                        ))
                    },
                    done,
                )
            }
            _ => Task::none(),
        }
    }

    pub(super) fn region_view<'a>(&'a self, s: &'a RegionState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let (ready, said) = match &s.shape {
            Ok(shape) => (true, words::text_line(Line::Info, shape.said.clone())),
            Err(why) => (false, words::text_line(Line::Warn, why.clone())),
        };
        body = body.push(words::field(
            "Alan",
            said,
            Some("Alanın içindeki nokta bulutu noktaları kaynak dosyalarının tam çözünürlüğünde okunur; sınırdaki nokta içeridedir.".into()),
        ));
        body = body.push(words::ghost("Seçimden al", Some(event(Event::Use))));
        body = body.push(result_view(s.result.as_ref(), s.running));
        window(
            "Nokta bulutu alan istatistiği",
            body,
            ready && !s.running,
            s.result.is_some(),
        )
    }

    pub(super) fn profile_view<'a>(&'a self, s: &'a ProfileState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let (ready, said) = match &s.path {
            Ok((_, said)) => (true, words::text_line(Line::Info, said.clone())),
            Err(why) => (false, words::text_line(Line::Warn, why.clone())),
        };
        body = body.push(words::field("Yol", said, None));
        body = body.push(words::ghost("Seçimden al", Some(event(Event::Use))));
        body = body.push(words::field(
            "Koridorun genişliği (iki yanda toplam)",
            input("10", &s.width, Event::Width),
            None,
        ));
        body = body.push(words::field(
            "İstasyon adımı (yol boyunca)",
            input("25", &s.step, Event::Step),
            None,
        ));
        body = body.push(words::check(
            s.points,
            "Noktaları da listele (istasyon, ofset, yer, sınıf)",
            Some(event(Event::Toggle)),
        ));
        body = body.push(result_view(s.result.as_ref(), s.running));
        window(
            "Nokta bulutu profili",
            body,
            ready && !s.running,
            s.result.is_some(),
        )
    }

    pub(super) fn mesh_view<'a>(&'a self, s: &'a MeshState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        body = body.push(words::field("X (doğu)", input("500012", &s.x, Event::X), None));
        body = body.push(words::field("Y (kuzey)", input("4420010", &s.y, Event::Y), None));
        body = body.push(words::ghost("Seçili noktadan al", Some(event(Event::Use))));
        body = body.push(words::field(
            "An (isteğe bağlı)",
            input("2024-05-01 06:30", &s.time, Event::Time),
            Some("Boşsa rasterin gösterdiği adımlar; verilirse her veri setinin o andan önceki son adımı.".into()),
        ));
        body = body.push(words::check(
            s.series,
            "Zaman boyutlu veri setlerinin her adımını göster",
            Some(event(Event::Toggle)),
        ));
        body = body.push(result_view(s.result.as_ref(), s.running));
        let ready = !s.running && parse(&s.x).is_some() && parse(&s.y).is_some();
        window("Mesh sor", body, ready, s.result.is_some())
    }
}

fn parse(t: &str) -> Option<f64> {
    t.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

fn input<'a>(hint: &str, value: &str, on: fn(String) -> Event) -> Element<'a, Message> {
    focus_ring(
        text_input(hint, value)
            .on_input(move |t| event(on(t)))
            .padding([5, 8])
            .width(Fill)
            .size(kentos_ui::theme::typography::body())
            .style(style::field::input),
    )
    .into()
}

fn window<'a>(title: &'a str, body: Column<'a, Message>, ready: bool, copyable: bool) -> Element<'a, Message> {
    overlay::blocking(
        Dialog::new(title)
            .scroll(body)
            .action(words::secondary("Kapat", Some(message(Exchange::Close))))
            .action(words::secondary(
                "Panoya kopyala",
                copyable.then(|| event(Event::Copy)),
            ))
            .action(words::primary("Çalıştır", ready.then(|| event(Event::Run))))
            .width(700.0)
            .max_height(760.0),
    )
}

fn result_view(r: Option<&Outcome>, running: bool) -> Element<'_, Message> {
    if running {
        return words::summary(vec![words::text_line(Line::Info, "Dosyalar okunuyor…")]);
    }
    let Some(r) = r else {
        return words::summary(Vec::new());
    };
    let mut lines = Vec::new();
    let warn = |warnings: &[CommandWarning], lines: &mut Vec<Element<'_, Message>>| {
        for w in warnings {
            lines.push(words::text_line(Line::Warn, w.message.clone()));
        }
    };
    match r {
        Outcome::Region(Err(why)) | Outcome::Profile(Err(why)) | Outcome::Mesh(Err(why)) => {
            lines.push(words::text_line(Line::Error, why.clone()));
        }
        Outcome::Region(Ok((o, warnings))) => {
            lines.extend(region_lines(o));
            warn(warnings, &mut lines);
        }
        Outcome::Profile(Ok((o, warnings))) => {
            lines.extend(profile_lines(o));
            warn(warnings, &mut lines);
        }
        Outcome::Mesh(Ok((o, warnings))) => {
            lines.extend(mesh_lines(o));
            warn(warnings, &mut lines);
        }
    }
    words::summary(lines)
}

fn stats_text(s: &kentos_commands::CloudStats) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(z) = &s.z {
        out.push(format!(
            "Yükseklik: en az {}, en çok {}, ortalama {}",
            fixed(z.min),
            fixed(z.max),
            fixed(z.mean)
        ));
    }
    if let Some(i) = &s.intensity {
        out.push(format!(
            "Yoğunluk: en az {}, en çok {}, ortalama {}",
            fixed(i.min),
            fixed(i.max),
            fixed(i.mean)
        ));
    }
    if !s.classes.is_empty() {
        let mut classes: Vec<(&String, &u64)> = s.classes.iter().collect();
        classes.sort_by_key(|(c, _)| c.parse::<u32>().unwrap_or(u32::MAX));
        let said: Vec<String> = classes.iter().map(|(c, n)| format!("{c}: {n}")).collect();
        out.push(format!("Sınıflar: {}", said.join(", ")));
    }
    out
}

fn region_texts(o: &PointCloudsRegioned) -> Vec<String> {
    let mut out = vec![format!(
        "Alan {}; {} nokta{}; {} nokta okundu.",
        fixed(o.area),
        o.total.count,
        o.density
            .map_or_else(String::new, |d| format!(" ({} nokta/birim²)", fixed(d))),
        o.read
    )];
    out.extend(stats_text(&o.total));
    if o.clouds.len() > 1 {
        for (i, c) in o.clouds.iter().enumerate() {
            out.push(format!(
                "{}. bulut: {} nokta, {} okundu",
                i + 1,
                c.stats.count,
                c.read
            ));
        }
    }
    out
}

fn region_lines(o: &PointCloudsRegioned) -> Vec<Element<'_, Message>> {
    let texts = region_texts(o);
    let kind = if o.total.count == 0 { Line::Warn } else { Line::Ok };
    texts
        .into_iter()
        .enumerate()
        .map(|(i, t)| words::text_line(if i == 0 { kind } else { Line::Info }, t))
        .collect()
}

fn profile_texts(o: &PointCloudsProfiled) -> Vec<String> {
    let mut out = vec![format!(
        "Yol {}; koridorda {} nokta; {} nokta okundu; {} istasyon.",
        fixed(o.length),
        o.count,
        o.read,
        o.bins.len()
    )];
    for b in &o.bins {
        out.push(match &b.z {
            Some(z) => format!(
                "{} – {}: {} nokta, yükseklik {} … {} (ort. {})",
                fixed(b.from),
                fixed(b.to),
                b.count,
                fixed(z.min),
                fixed(z.max),
                fixed(z.mean)
            ),
            None => format!("{} – {}: nokta yok", fixed(b.from), fixed(b.to)),
        });
    }
    out
}

fn profile_lines(o: &PointCloudsProfiled) -> Vec<Element<'_, Message>> {
    let texts = profile_texts(o);
    let more = texts.len().saturating_sub(SHOWN + 1);
    let mut lines: Vec<Element<'_, Message>> = texts
        .into_iter()
        .take(SHOWN + 1)
        .enumerate()
        .map(|(i, t)| words::text_line(if i == 0 { Line::Ok } else { Line::Info }, t))
        .collect();
    if more > 0 {
        lines.push(words::text_line(
            Line::Info,
            format!("… {more} istasyon daha (Panoya kopyala hepsini alır)."),
        ));
    }
    if !o.points.is_empty() {
        lines.push(words::text_line(
            Line::Info,
            format!("{} nokta listelendi (Panoya kopyala satır satır alır).", o.points.len()),
        ));
    }
    lines
}

fn mesh_texts(o: &MeshesQueried) -> Vec<String> {
    let Some(f) = &o.face else {
        return vec!["Yer hiçbir meshin yüzünde değil.".to_owned()];
    };
    let nodes: Vec<String> = f.nodes.iter().map(|n| (n + 1).to_string()).collect();
    let mut out = vec![format!(
        "Yüz {} (“{}” ağı, düğümler {}); alanı {}; {} değer okundu.",
        f.face + 1,
        f.topology,
        nodes.join("-"),
        fixed(f.area),
        o.read
    )];
    for v in &o.values {
        let name = match &v.vector {
            Some(y) => format!("{} / {y}", v.variable),
            None => v.variable.clone(),
        };
        let unit = v.units.as_deref().map_or_else(String::new, |u| format!(" {u}"));
        let at = v
            .time
            .map(|t| {
                format!(
                    " ({})",
                    kentos_formats::multidim::cube::dim_labels(&[t], true, None).pop().unwrap_or_default()
                )
            })
            .unwrap_or_default();
        out.push(match v.value {
            Some(x) => format!("{name} [{}]: {}{unit}{at}", v.location, fixed(x)),
            None => format!("{name} [{}]: değer yok{at}", v.location),
        });
        for s in &v.series {
            out.push(match s.value {
                Some(x) => format!("    {}: {}{unit}", s.label, fixed(x)),
                None => format!("    {}: değer yok", s.label),
            });
        }
    }
    out
}

fn mesh_lines(o: &MeshesQueried) -> Vec<Element<'_, Message>> {
    mesh_texts(o)
        .into_iter()
        .take(SHOWN + 1)
        .enumerate()
        .map(|(i, t)| {
            words::text_line(
                if i == 0 {
                    if o.face.is_some() { Line::Ok } else { Line::Warn }
                } else {
                    Line::Info
                },
                t,
            )
        })
        .collect()
}

/// The answer as text for the clipboard: the lines, a profile's points as tab separated rows.
fn copy_text(r: &Outcome) -> String {
    match r {
        Outcome::Region(Ok((o, _))) => region_texts(o).join("\n"),
        Outcome::Profile(Ok((o, _))) => {
            let mut t = profile_texts(o).join("\n");
            if !o.points.is_empty() {
                t.push_str("\nİstasyon\tOfset\tY\tX\tZ\tSınıf\n");
                for p in &o.points {
                    let _ = writeln!(
                        t,
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        fixed(p.station),
                        fixed(p.offset),
                        fixed(p.x),
                        fixed(p.y),
                        fixed(p.z),
                        p.class
                    );
                }
            }
            t
        }
        Outcome::Mesh(Ok((o, _))) => mesh_texts(o).join("\n"),
        Outcome::Region(Err(why)) | Outcome::Profile(Err(why)) | Outcome::Mesh(Err(why)) => why.clone(),
    }
}
