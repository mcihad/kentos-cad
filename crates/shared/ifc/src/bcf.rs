//! Issues as buildingSMART's BCF carries them (docs/adr/8103; plan/ifc-uyum.md IFC-10): a zip of `bcf.version`,
//! `extensions.xml` and a folder per topic, named by its GUID, holding its `markup.bcf` and its viewpoints (`.bcfv`).
//! Written as BCF 3.0 (BCF-XML 3.0's schemas: the elements in their sequences' order, the dates `xs:dateTime`, the
//! lengths metres in the IFC file's world); read as 3.0 or 2.1 (2.1 keeps the comments and viewpoints beside the topic,
//! its cameras optional). A viewpoint's components are the objects by their IFC GlobalIds: a viewer opening the same
//! IFC file shows the same objects. Nothing here touches the file system; a hostile archive is bounded by the zip
//! reader's limits.

use std::fmt::Write as _;

/// The BCF version written.
pub const VERSION: &str = "3.0";
/// The most topics one file is read or written with.
pub const MAX_TOPICS: usize = 10_000;

/// Where a viewpoint's camera stands: its eye, the way it looks, its up and, for an orthogonal camera, how many metres
/// the view's height shows (`ViewToWorldScale`), for a perspective one its vertical field of view in degrees.
#[derive(Debug, Clone, PartialEq)]
pub enum Camera {
    Orthogonal {
        eye: [f64; 3],
        direction: [f64; 3],
        up: [f64; 3],
        scale: f64,
        aspect: f64,
    },
    Perspective {
        eye: [f64; 3],
        direction: [f64; 3],
        up: [f64; 3],
        field_of_view: f64,
        aspect: f64,
    },
}

/// A viewpoint: its GUID, the objects it selects (IFC GlobalIds), whether the rest stays visible, its camera.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewpoint {
    pub guid: String,
    pub selection: Vec<String>,
    /// Every object shows (`DefaultVisibility`), but those in `exceptions`.
    pub visible: bool,
    pub exceptions: Vec<String>,
    /// Colours (`RRGGBB` or `RRGGBBAA`) and the objects they paint.
    pub coloring: Vec<(String, Vec<String>)>,
    /// None only in a 2.1 file, whose cameras are optional.
    pub camera: Option<Camera>,
    /// Its snapshot's name in the topic's folder, when it has one.
    pub snapshot: Option<String>,
}

/// A comment: its GUID, date, author, text and the viewpoint it points at.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub guid: String,
    pub date: String,
    pub author: String,
    pub text: Option<String>,
    pub viewpoint: Option<String>,
}

/// A file a topic is about (`Header/Files/File`): its project's GlobalId, its name, date and where it is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileRef {
    pub ifc_project: Option<String>,
    pub filename: Option<String>,
    pub date: Option<String>,
    pub reference: Option<String>,
}

/// A topic: an issue with its type, status, title and the rest of BCF's fields, its comments and viewpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct Topic {
    pub guid: String,
    pub topic_type: String,
    pub status: String,
    pub title: String,
    pub priority: Option<String>,
    pub labels: Vec<String>,
    pub created: String,
    pub author: String,
    pub modified: Option<String>,
    pub modified_by: Option<String>,
    pub due: Option<String>,
    pub assigned_to: Option<String>,
    pub stage: Option<String>,
    pub description: Option<String>,
    pub files: Vec<FileRef>,
    pub comments: Vec<Comment>,
    pub viewpoints: Vec<Viewpoint>,
}

/// A BCF file read: its version and topics.
#[derive(Debug, Clone, PartialEq)]
pub struct Bcf {
    pub version: String,
    pub topics: Vec<Topic>,
}

/// A BCF GUID: lowercase, hyphenated (BCF-XML's `Guid`).
pub fn guid_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
        })
}

/// A GUID made from `seed` and `k`, the same each time (a topic's viewpoint from the topic's GUID, a topic without one
/// from the file's place and its order): FNV-1a over 128 bits, written as a UUID of version 8 (made by its own rule).
pub fn guid_from(seed: &str, k: u64) -> String {
    let n = crate::step::fnv(seed, k);
    let n = (n & !(0xF << 76) & !(0x3 << 62)) | (0x8 << 76) | (0x2 << 62);
    let h = format!("{n:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// An IFC GlobalId: 22 characters of `0-9A-Za-z_$` (BCF-XML's `IfcGuid`).
pub fn ifc_guid_ok(s: &str) -> bool {
    s.len() == 22
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
}

/// A camera looking at the box `b` (x, y, z from, x, y, z to; metres): a perspective camera from the south-west and
/// above, 60° high, at the distance the box's diagonal fills, up along z as near as the direction lets it.
pub fn camera_for(b: [f64; 6]) -> Camera {
    let middle = [
        (b[0] + b[3]) / 2.0,
        (b[1] + b[4]) / 2.0,
        (b[2] + b[5]) / 2.0,
    ];
    let diagonal =
        crate::math::hypot(crate::math::hypot(b[3] - b[0], b[4] - b[1]), b[5] - b[2]).max(1.0);
    // From the south-west, above: (1, 1, −1) normalised.
    let k = 1.0 / 3f64.sqrt();
    let direction = [k, k, -k];
    // The diagonal fills a 60° view at twice its half over tan 30°, and a little more.
    let distance = diagonal / (2.0 * crate::math::tan(30f64.to_radians())) * 1.2;
    let eye = [
        middle[0] - direction[0] * distance,
        middle[1] - direction[1] * distance,
        middle[2] - direction[2] * distance,
    ];
    // Up: z less its part along the direction.
    let along = direction[2];
    let up = [
        -along * direction[0],
        -along * direction[1],
        1.0 - along * direction[2],
    ];
    let n = crate::math::hypot(crate::math::hypot(up[0], up[1]), up[2]);
    Camera::Perspective {
        eye,
        direction,
        up: [up[0] / n, up[1] / n, up[2] / n],
        field_of_view: 60.0,
        aspect: 16.0 / 9.0,
    }
}

/// Text as XML writes it between tags and in attributes.
fn escaped(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A number as XML writes it: the shortest decimal that reads back to it.
fn number(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

fn element(out: &mut String, indent: usize, name: &str, text: &str) {
    let _ = writeln!(out, "{:indent$}<{name}>{}</{name}>", "", escaped(text));
}

fn optional(out: &mut String, indent: usize, name: &str, text: Option<&str>) {
    if let Some(t) = text {
        element(out, indent, name, t);
    }
}

fn point(out: &mut String, indent: usize, name: &str, p: [f64; 3]) {
    let _ = writeln!(out, "{:indent$}<{name}>", "");
    for (axis, v) in ["X", "Y", "Z"].iter().zip(p) {
        let _ = writeln!(
            out,
            "{:w$}<{axis}>{}</{axis}>",
            "",
            number(v),
            w = indent + 2
        );
    }
    let _ = writeln!(out, "{:indent$}</{name}>", "");
}

const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

/// A topic's `markup.bcf` (BCF 3.0).
fn markup(t: &Topic) -> String {
    let mut out = String::from(XML);
    out.push_str("<Markup>\n");
    if !t.files.is_empty() {
        out.push_str("  <Header>\n    <Files>\n");
        for f in &t.files {
            let project = f
                .ifc_project
                .as_deref()
                .map(|p| format!(" IfcProject=\"{}\"", escaped(p)))
                .unwrap_or_default();
            let _ = writeln!(out, "      <File{project} IsExternal=\"true\">");
            optional(&mut out, 8, "Filename", f.filename.as_deref());
            optional(&mut out, 8, "Date", f.date.as_deref());
            optional(&mut out, 8, "Reference", f.reference.as_deref());
            out.push_str("      </File>\n");
        }
        out.push_str("    </Files>\n  </Header>\n");
    }
    let _ = writeln!(
        out,
        "  <Topic Guid=\"{}\" TopicType=\"{}\" TopicStatus=\"{}\">",
        escaped(&t.guid),
        escaped(&t.topic_type),
        escaped(&t.status)
    );
    element(&mut out, 4, "Title", &t.title);
    optional(&mut out, 4, "Priority", t.priority.as_deref());
    if !t.labels.is_empty() {
        out.push_str("    <Labels>\n");
        for l in &t.labels {
            element(&mut out, 6, "Label", l);
        }
        out.push_str("    </Labels>\n");
    }
    element(&mut out, 4, "CreationDate", &t.created);
    element(&mut out, 4, "CreationAuthor", &t.author);
    optional(&mut out, 4, "ModifiedDate", t.modified.as_deref());
    optional(&mut out, 4, "ModifiedAuthor", t.modified_by.as_deref());
    optional(&mut out, 4, "DueDate", t.due.as_deref());
    optional(&mut out, 4, "AssignedTo", t.assigned_to.as_deref());
    optional(&mut out, 4, "Stage", t.stage.as_deref());
    optional(&mut out, 4, "Description", t.description.as_deref());
    if !t.comments.is_empty() {
        out.push_str("    <Comments>\n");
        for c in &t.comments {
            let _ = writeln!(out, "      <Comment Guid=\"{}\">", escaped(&c.guid));
            element(&mut out, 8, "Date", &c.date);
            element(&mut out, 8, "Author", &c.author);
            optional(&mut out, 8, "Comment", c.text.as_deref());
            if let Some(v) = &c.viewpoint {
                let _ = writeln!(out, "        <Viewpoint Guid=\"{}\"/>", escaped(v));
            }
            out.push_str("      </Comment>\n");
        }
        out.push_str("    </Comments>\n");
    }
    if !t.viewpoints.is_empty() {
        out.push_str("    <Viewpoints>\n");
        for v in &t.viewpoints {
            let _ = writeln!(out, "      <ViewPoint Guid=\"{}\">", escaped(&v.guid));
            element(&mut out, 8, "Viewpoint", &format!("{}.bcfv", v.guid));
            optional(&mut out, 8, "Snapshot", v.snapshot.as_deref());
            out.push_str("      </ViewPoint>\n");
        }
        out.push_str("    </Viewpoints>\n");
    }
    out.push_str("  </Topic>\n</Markup>\n");
    out
}

fn components(out: &mut String, indent: usize, guids: &[String]) {
    for g in guids {
        let _ = writeln!(out, "{:indent$}<Component IfcGuid=\"{}\"/>", "", escaped(g));
    }
}

/// A viewpoint's `.bcfv` (BCF 3.0).
fn visinfo(v: &Viewpoint) -> Result<String, String> {
    let Some(camera) = &v.camera else {
        return Err(format!(
            "“{}” bakışının kamerası yok: BCF 3.0 her bakışta bir kamera ister.",
            v.guid
        ));
    };
    let mut out = String::from(XML);
    let _ = writeln!(out, "<VisualizationInfo Guid=\"{}\">", escaped(&v.guid));
    out.push_str("  <Components>\n");
    if !v.selection.is_empty() {
        out.push_str("    <Selection>\n");
        components(&mut out, 6, &v.selection);
        out.push_str("    </Selection>\n");
    }
    let _ = writeln!(out, "    <Visibility DefaultVisibility=\"{}\">", v.visible);
    if !v.exceptions.is_empty() {
        out.push_str("      <Exceptions>\n");
        components(&mut out, 8, &v.exceptions);
        out.push_str("      </Exceptions>\n");
    }
    out.push_str("    </Visibility>\n");
    if !v.coloring.is_empty() {
        out.push_str("    <Coloring>\n");
        for (color, guids) in &v.coloring {
            let _ = writeln!(out, "      <Color Color=\"{}\">", escaped(color));
            out.push_str("        <Components>\n");
            components(&mut out, 10, guids);
            out.push_str("        </Components>\n      </Color>\n");
        }
        out.push_str("    </Coloring>\n");
    }
    out.push_str("  </Components>\n");
    let (name, eye, direction, up, last, value, aspect) = match camera {
        Camera::Orthogonal {
            eye,
            direction,
            up,
            scale,
            aspect,
        } => (
            "OrthogonalCamera",
            eye,
            direction,
            up,
            "ViewToWorldScale",
            scale,
            aspect,
        ),
        Camera::Perspective {
            eye,
            direction,
            up,
            field_of_view,
            aspect,
        } => (
            "PerspectiveCamera",
            eye,
            direction,
            up,
            "FieldOfView",
            field_of_view,
            aspect,
        ),
    };
    let _ = writeln!(out, "  <{name}>");
    point(&mut out, 4, "CameraViewPoint", *eye);
    point(&mut out, 4, "CameraDirection", *direction);
    point(&mut out, 4, "CameraUpVector", *up);
    element(&mut out, 4, last, &number(*value));
    element(&mut out, 4, "AspectRatio", &number(*aspect));
    let _ = writeln!(out, "  </{name}>");
    out.push_str("</VisualizationInfo>\n");
    Ok(out)
}

/// The lists `extensions.xml` gives: the topics' types, statuses, priorities, labels, users and stages used.
fn extensions(topics: &[Topic]) -> String {
    let mut lists: [(&str, &str, Vec<String>); 6] = [
        ("TopicTypes", "TopicType", Vec::new()),
        ("TopicStatuses", "TopicStatus", Vec::new()),
        ("Priorities", "Priority", Vec::new()),
        ("TopicLabels", "TopicLabel", Vec::new()),
        ("Users", "User", Vec::new()),
        ("Stages", "Stage", Vec::new()),
    ];
    let mut add = |k: usize, v: &str| {
        if !lists[k].2.iter().any(|x| x == v) {
            lists[k].2.push(v.to_owned());
        }
    };
    for t in topics {
        add(0, &t.topic_type);
        add(1, &t.status);
        if let Some(p) = &t.priority {
            add(2, p);
        }
        for l in &t.labels {
            add(3, l);
        }
        add(4, &t.author);
        if let Some(a) = &t.assigned_to {
            add(4, a);
        }
        for c in &t.comments {
            add(4, &c.author);
        }
        if let Some(s) = &t.stage {
            add(5, s);
        }
    }
    let mut out = String::from(XML);
    out.push_str("<Extensions>\n");
    for (list, item, values) in &lists {
        if values.is_empty() {
            continue;
        }
        let _ = writeln!(out, "  <{list}>");
        for v in values {
            element(&mut out, 4, item, v);
        }
        let _ = writeln!(out, "  </{list}>");
    }
    out.push_str("</Extensions>\n");
    out
}

/// What a topic must hold to be written: why not, with the field.
pub fn topic_problem(t: &Topic) -> Option<(String, String)> {
    let blank = |s: &str| s.trim().is_empty();
    if !guid_ok(&t.guid) {
        return Some((
            "guid".into(),
            format!("“{}” küçük harfli, tireli bir GUID değil.", t.guid),
        ));
    }
    for (field, value) in [
        ("type", &t.topic_type),
        ("status", &t.status),
        ("title", &t.title),
        ("author", &t.author),
        ("created", &t.created),
    ] {
        if blank(value) {
            return Some((field.into(), "Boş olamaz.".into()));
        }
    }
    for v in &t.viewpoints {
        if !guid_ok(&v.guid) {
            return Some((
                "viewpoints".into(),
                format!("Bakışın GUID'i “{}” geçerli değil.", v.guid),
            ));
        }
        if let Some(g) = v
            .selection
            .iter()
            .chain(&v.exceptions)
            .find(|g| !ifc_guid_ok(g))
        {
            return Some((
                "objects".into(),
                format!("“{g}” bir IFC GlobalId'si değil (22 karakter, 0-9A-Za-z_$)."),
            ));
        }
        if v.camera.is_none() {
            return Some(("viewpoints".into(), "Bakışın kamerası yok.".into()));
        }
    }
    for c in &t.comments {
        if !guid_ok(&c.guid) || blank(&c.date) || blank(&c.author) {
            return Some((
                "comments".into(),
                "Yorumun GUID'i, günü ve yazarı olmalı.".into(),
            ));
        }
        if c.text.as_deref().is_none_or(blank) && c.viewpoint.is_none() {
            return Some((
                "comments".into(),
                "Yorumun metni ya da bakışı olmalı.".into(),
            ));
        }
    }
    None
}

/// The BCF 3.0 file of `topics`.
pub fn write(topics: &[Topic]) -> Result<Vec<u8>, String> {
    if topics.len() > MAX_TOPICS {
        return Err(format!(
            "Bir BCF dosyasına en çok {MAX_TOPICS} konu yazılır."
        ));
    }
    let version = format!("{XML}<Version VersionId=\"{VERSION}\"/>\n");
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("bcf.version".into(), version.into_bytes()),
        ("extensions.xml".into(), extensions(topics).into_bytes()),
    ];
    for t in topics {
        if let Some((field, why)) = topic_problem(t) {
            return Err(format!("“{}” konusu ({field}): {why}", t.title));
        }
        files.push((format!("{}/", t.guid), Vec::new()));
        files.push((format!("{}/markup.bcf", t.guid), markup(t).into_bytes()));
        for v in &t.viewpoints {
            files.push((
                format!("{}/{}.bcfv", t.guid, v.guid),
                visinfo(v)?.into_bytes(),
            ));
        }
    }
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    kentos_zip::write(&entries)
}

// Reading.

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn children<'a, 'i>(
    n: roxmltree::Node<'a, 'i>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    child(n, name)
        .and_then(|c| c.text())
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

fn point_of(n: roxmltree::Node<'_, '_>, name: &str) -> Option<[f64; 3]> {
    let p = child(n, name)?;
    let v = |axis: &str| {
        text_of(p, axis)
            .and_then(|t| t.parse::<f64>().ok())
            .filter(|v| v.is_finite())
    };
    Some([v("X")?, v("Y")?, v("Z")?])
}

fn guids_under(n: Option<roxmltree::Node<'_, '_>>) -> Vec<String> {
    n.map(|n| {
        children(n, "Component")
            .filter_map(|c| c.attribute("IfcGuid").map(|g| g.trim().to_owned()))
            .filter(|g| !g.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn parse<'i>(name: &str, bytes: &'i [u8]) -> Result<roxmltree::Document<'i>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| format!("{name} UTF-8 değil."))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    roxmltree::Document::parse(text).map_err(|e| format!("{name} okunamadı: {e}"))
}

fn viewpoint_of(
    name: &str,
    bytes: &[u8],
    guid: String,
    snapshot: Option<String>,
) -> Result<Viewpoint, String> {
    let doc = parse(name, bytes)?;
    let root = doc.root_element();
    if root.tag_name().name() != "VisualizationInfo" {
        return Err(format!("{name} bir bakış (VisualizationInfo) değil."));
    }
    let comps = child(root, "Components");
    let visibility = comps.and_then(|c| child(c, "Visibility"));
    let visible = visibility
        .and_then(|v| v.attribute("DefaultVisibility"))
        .map(|v| v.trim() == "true" || v.trim() == "1");
    let coloring = comps
        .and_then(|c| child(c, "Coloring"))
        .map(|c| {
            children(c, "Color")
                .filter_map(|k| {
                    Some((
                        k.attribute("Color")?.trim().to_owned(),
                        guids_under(child(k, "Components")),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let camera = if let Some(c) = child(root, "PerspectiveCamera") {
        Some(Camera::Perspective {
            eye: point_of(c, "CameraViewPoint")
                .ok_or_else(|| format!("{name}: kameranın yeri okunamadı."))?,
            direction: point_of(c, "CameraDirection")
                .ok_or_else(|| format!("{name}: kameranın yönü okunamadı."))?,
            up: point_of(c, "CameraUpVector")
                .ok_or_else(|| format!("{name}: kameranın yukarısı okunamadı."))?,
            field_of_view: text_of(c, "FieldOfView")
                .and_then(|t| t.parse().ok())
                .unwrap_or(60.0),
            aspect: text_of(c, "AspectRatio")
                .and_then(|t| t.parse().ok())
                .unwrap_or(1.0),
        })
    } else if let Some(c) = child(root, "OrthogonalCamera") {
        Some(Camera::Orthogonal {
            eye: point_of(c, "CameraViewPoint")
                .ok_or_else(|| format!("{name}: kameranın yeri okunamadı."))?,
            direction: point_of(c, "CameraDirection")
                .ok_or_else(|| format!("{name}: kameranın yönü okunamadı."))?,
            up: point_of(c, "CameraUpVector")
                .ok_or_else(|| format!("{name}: kameranın yukarısı okunamadı."))?,
            scale: text_of(c, "ViewToWorldScale")
                .and_then(|t| t.parse().ok())
                .unwrap_or(1.0),
            aspect: text_of(c, "AspectRatio")
                .and_then(|t| t.parse().ok())
                .unwrap_or(1.0),
        })
    } else {
        None
    };
    Ok(Viewpoint {
        guid: root.attribute("Guid").map(str::to_owned).unwrap_or(guid),
        selection: guids_under(comps.and_then(|c| child(c, "Selection"))),
        // The schema's default when the file says nothing.
        visible: visible.unwrap_or(false),
        exceptions: guids_under(visibility.and_then(|v| child(v, "Exceptions"))),
        coloring,
        camera,
        snapshot,
    })
}

/// The BCF file in `bytes` (3.0 or 2.1).
pub fn read(bytes: &[u8]) -> Result<Bcf, String> {
    let limits = kentos_zip::Limits {
        entries: 4 * MAX_TOPICS,
        ..kentos_zip::Limits::default()
    };
    let listed = kentos_zip::list(bytes, &limits)
        .map_err(|e| format!("BCF dosyası bir zip arşivi olarak okunamadı: {e}"))?;
    let entries = kentos_zip::unpack(bytes, &listed, &limits, |l| {
        let n = l.name.to_ascii_lowercase();
        n.ends_with(".bcf") || n.ends_with(".bcfv") || n.ends_with("bcf.version")
    })?;
    let get = |name: &str| {
        entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.data.as_slice())
    };
    let version = match get("bcf.version") {
        Some(b) => {
            let doc = parse("bcf.version", b)?;
            doc.root_element()
                .attribute("VersionId")
                .map(|v| v.trim().to_owned())
                .unwrap_or_default()
        }
        None => return Err("BCF dosyasında bcf.version yok: sürümü bilinmiyor.".into()),
    };
    if version != "3.0" && version != "2.1" && version != "2.0" {
        return Err(format!(
            "BCF sürümü {version} okunmuyor: 3.0 ve 2.1 okunur."
        ));
    }
    let three = version == "3.0";
    let mut topics = Vec::new();
    let mut folders: Vec<&str> = entries
        .iter()
        .filter_map(|e| e.name.strip_suffix("/markup.bcf"))
        .filter(|f| !f.contains('/'))
        .collect();
    folders.sort_unstable();
    if folders.len() > MAX_TOPICS {
        return Err(format!("BCF dosyasında {MAX_TOPICS}'den çok konu var."));
    }
    for folder in folders {
        let name = format!("{folder}/markup.bcf");
        let doc = parse(&name, get(&name).unwrap_or_default())?;
        let root = doc.root_element();
        let Some(topic) = child(root, "Topic") else {
            return Err(format!("{name}: konu (Topic) yok."));
        };
        let files = child(root, "Header")
            .map(|h| {
                let list = child(h, "Files").unwrap_or(h);
                children(list, "File")
                    .map(|f| FileRef {
                        ifc_project: f.attribute("IfcProject").map(str::to_owned),
                        filename: text_of(f, "Filename"),
                        date: text_of(f, "Date"),
                        reference: text_of(f, "Reference"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        // 3.0 keeps comments and viewpoints in the topic; 2.1 beside it.
        let holder = if three { topic } else { root };
        let comment_nodes: Vec<roxmltree::Node<'_, '_>> = if three {
            child(holder, "Comments")
                .map(|c| children(c, "Comment").collect())
                .unwrap_or_default()
        } else {
            children(holder, "Comment").collect()
        };
        let comments = comment_nodes
            .into_iter()
            .map(|c| Comment {
                guid: c.attribute("Guid").unwrap_or_default().to_owned(),
                date: text_of(c, "Date").unwrap_or_default(),
                author: text_of(c, "Author").unwrap_or_default(),
                text: text_of(c, "Comment"),
                viewpoint: child(c, "Viewpoint")
                    .and_then(|v| v.attribute("Guid"))
                    .map(str::to_owned),
            })
            .collect();
        let view_nodes: Vec<roxmltree::Node<'_, '_>> = if three {
            child(holder, "Viewpoints")
                .map(|v| children(v, "ViewPoint").collect())
                .unwrap_or_default()
        } else {
            children(holder, "Viewpoints").collect()
        };
        let mut viewpoints = Vec::new();
        for v in view_nodes {
            let guid = v.attribute("Guid").unwrap_or_default().to_owned();
            let file = text_of(v, "Viewpoint").unwrap_or_else(|| "viewpoint.bcfv".into());
            let path = format!("{folder}/{file}");
            let Some(data) = get(&path) else {
                return Err(format!("{name} “{file}” bakışını anıyor ama arşivde yok."));
            };
            viewpoints.push(viewpoint_of(&path, data, guid, text_of(v, "Snapshot"))?);
        }
        topics.push(Topic {
            guid: topic.attribute("Guid").unwrap_or(folder).to_owned(),
            topic_type: topic.attribute("TopicType").unwrap_or_default().to_owned(),
            status: topic
                .attribute("TopicStatus")
                .unwrap_or_default()
                .to_owned(),
            title: text_of(topic, "Title").unwrap_or_default(),
            priority: text_of(topic, "Priority"),
            labels: if three {
                child(topic, "Labels")
                    .map(|l| {
                        children(l, "Label")
                            .filter_map(|x| x.text().map(|t| t.trim().to_owned()))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                children(topic, "Labels")
                    .filter_map(|x| x.text().map(|t| t.trim().to_owned()))
                    .collect()
            },
            created: text_of(topic, "CreationDate").unwrap_or_default(),
            author: text_of(topic, "CreationAuthor").unwrap_or_default(),
            modified: text_of(topic, "ModifiedDate"),
            modified_by: text_of(topic, "ModifiedAuthor"),
            due: text_of(topic, "DueDate"),
            assigned_to: text_of(topic, "AssignedTo"),
            stage: text_of(topic, "Stage"),
            description: text_of(topic, "Description"),
            files,
            comments,
            viewpoints,
        });
    }
    Ok(Bcf { version, topics })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic() -> Topic {
        Topic {
            guid: "0192f5a1-8103-7000-8000-000000000001".into(),
            topic_type: "Issue".into(),
            status: "Open".into(),
            title: "Duvar 50 cm taşınmış & <yangın> sınıfı".into(),
            priority: Some("High".into()),
            labels: vec!["Mimari".into()],
            created: "2026-11-17T12:00:00Z".into(),
            author: "mimar@buro".into(),
            modified: None,
            modified_by: None,
            due: None,
            assigned_to: Some("statik@buro".into()),
            stage: None,
            description: Some("İkinci revizyonda.".into()),
            files: vec![FileRef {
                ifc_project: Some("0YvctVUKr0kugbFTf53O9L".into()),
                filename: Some("kesit-binasi-r2.ifc".into()),
                date: None,
                reference: None,
            }],
            comments: vec![Comment {
                guid: "0192f5a1-8103-7000-8000-0000000000c1".into(),
                date: "2026-11-17T12:05:00Z".into(),
                author: "statik@buro".into(),
                text: Some("Kontrol edildi.".into()),
                viewpoint: Some("0192f5a1-8103-7000-8000-0000000000b1".into()),
            }],
            viewpoints: vec![Viewpoint {
                guid: "0192f5a1-8103-7000-8000-0000000000b1".into(),
                selection: vec!["0CmAx$J2b4n8dVdl_wLq1s".into()],
                visible: true,
                exceptions: Vec::new(),
                coloring: vec![("FF0000".into(), vec!["0CmAx$J2b4n8dVdl_wLq1s".into()])],
                camera: Some(camera_for([0.0, -0.1, 0.0, 4.0, 0.1, 3.0])),
                snapshot: None,
            }],
        }
    }

    #[test]
    fn a_topic_written_reads_back_the_same() {
        let t = topic();
        let bytes = write(std::slice::from_ref(&t)).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back.version, "3.0");
        assert_eq!(back.topics, vec![t]);
        // The folder has its own entry, as BCF asks.
        let names: Vec<String> = kentos_zip::list_every(&bytes, &kentos_zip::Limits::default())
            .unwrap()
            .into_iter()
            .map(|l| l.name)
            .collect();
        assert!(
            names.contains(&"0192f5a1-8103-7000-8000-000000000001/".to_owned()),
            "{names:?}"
        );
    }

    #[test]
    fn a_camera_looks_at_the_box_s_middle_with_z_up() {
        let Camera::Perspective {
            eye,
            direction,
            up,
            field_of_view,
            ..
        } = camera_for([0.0, 0.0, 0.0, 4.0, 2.0, 3.0])
        else {
            panic!()
        };
        // The middle lies on the line of sight.
        let m = [2.0, 1.0, 1.5];
        let to = [m[0] - eye[0], m[1] - eye[1], m[2] - eye[2]];
        let cross = [
            to[1] * direction[2] - to[2] * direction[1],
            to[2] * direction[0] - to[0] * direction[2],
            to[0] * direction[1] - to[1] * direction[0],
        ];
        assert!(cross.iter().all(|c| c.abs() < 1e-9), "{cross:?}");
        let dot = direction[0] * up[0] + direction[1] * up[1] + direction[2] * up[2];
        assert!(dot.abs() < 1e-12 && up[2] > 0.0, "{up:?}");
        assert_eq!(field_of_view, 60.0);
    }

    #[test]
    fn a_topic_without_a_title_or_with_a_wrong_globalid_is_not_written() {
        let mut t = topic();
        t.title = " ".into();
        assert!(write(&[t]).unwrap_err().contains("(title)"));
        let mut t = topic();
        t.viewpoints[0].selection = vec!["kisa".into()];
        assert!(write(&[t]).unwrap_err().contains("(objects)"));
    }

    #[test]
    fn a_bcf_2_1_file_reads_its_comments_and_viewpoints_beside_the_topic() {
        let markup = br#"<?xml version="1.0" encoding="UTF-8"?>
<Markup>
  <Topic Guid="0192f5a1-8103-7000-8000-000000000002" TopicType="Clash" TopicStatus="Active">
    <Title>Kolon kirisi kesiyor</Title>
    <CreationDate>2026-11-17T10:00:00</CreationDate>
    <CreationAuthor>statik</CreationAuthor>
  </Topic>
  <Comment Guid="0192f5a1-8103-7000-8000-0000000000c2">
    <Date>2026-11-17T10:01:00</Date>
    <Author>mimar</Author>
    <Comment>Bakiyorum</Comment>
    <Topic Guid="0192f5a1-8103-7000-8000-000000000002"/>
  </Comment>
  <Viewpoints Guid="0192f5a1-8103-7000-8000-0000000000b2">
    <Viewpoint>viewpoint.bcfv</Viewpoint>
  </Viewpoints>
</Markup>
"#;
        let view = br#"<?xml version="1.0" encoding="UTF-8"?>
<VisualizationInfo Guid="0192f5a1-8103-7000-8000-0000000000b2">
  <Components><Selection><Component IfcGuid="0CmAx$J2b4n8dVdl_wLq1s"/></Selection><Visibility DefaultVisibility="true"/></Components>
  <OrthogonalCamera>
    <CameraViewPoint><X>1</X><Y>2</Y><Z>3</Z></CameraViewPoint>
    <CameraDirection><X>0</X><Y>0</Y><Z>-1</Z></CameraDirection>
    <CameraUpVector><X>0</X><Y>1</Y><Z>0</Z></CameraUpVector>
    <ViewToWorldScale>10</ViewToWorldScale>
  </OrthogonalCamera>
</VisualizationInfo>
"#;
        let version = br#"<?xml version="1.0" encoding="UTF-8"?><Version VersionId="2.1"><DetailedVersion>2.1</DetailedVersion></Version>"#;
        let bytes = kentos_zip::write(&[
            ("bcf.version", version.as_slice()),
            (
                "0192f5a1-8103-7000-8000-000000000002/markup.bcf",
                markup.as_slice(),
            ),
            (
                "0192f5a1-8103-7000-8000-000000000002/viewpoint.bcfv",
                view.as_slice(),
            ),
        ])
        .unwrap();
        let b = read(&bytes).unwrap();
        assert_eq!(b.version, "2.1");
        let t = &b.topics[0];
        assert_eq!(
            (t.topic_type.as_str(), t.status.as_str(), t.title.as_str()),
            ("Clash", "Active", "Kolon kirisi kesiyor")
        );
        assert_eq!(t.comments[0].text.as_deref(), Some("Bakiyorum"));
        assert_eq!(t.viewpoints[0].selection, ["0CmAx$J2b4n8dVdl_wLq1s"]);
        assert!(
            matches!(t.viewpoints[0].camera, Some(Camera::Orthogonal { scale, aspect, .. }) if scale == 10.0 && aspect == 1.0)
        );
    }

    #[test]
    fn a_made_guid_is_a_guid_and_the_same_each_time() {
        let a = guid_from("0192f5a1-8103-7000-8000-000000000001", 1);
        assert!(guid_ok(&a), "{a}");
        assert_eq!(a, guid_from("0192f5a1-8103-7000-8000-000000000001", 1));
        assert_ne!(a, guid_from("0192f5a1-8103-7000-8000-000000000001", 2));
        assert_eq!(&a[14..15], "8");
    }

    #[test]
    fn what_is_no_bcf_is_said() {
        assert!(read(b"not a zip").unwrap_err().contains("zip"));
        let bytes = kentos_zip::write(&[("a.txt", b"x".as_slice())]).unwrap();
        assert!(read(&bytes).unwrap_err().contains("bcf.version"));
    }
}
