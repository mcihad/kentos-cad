//! `cad.bcf.write` and `cad.bcf.read` v1 (plan/ifc-uyum.md IFC-10, docs/adr/8103): issues exchanged as BCF through
//! `kentos-ifc`'s BCF (`kentos_ifc::bcf`). Writing reads the IFC file the topics' objects are in, for their bodies (the
//! viewpoint's camera) and its project's GlobalId; the drawing does not change. Reading leads each selected object to
//! the IFC file named and to the drawing's object a KentOS GlobalId stands for.

use std::collections::BTreeSet;
use std::path::Path;

use kentos_commands::{
    BcfCamera, BcfComment, BcfObject, BcfRead, BcfReadOutput, BcfTopic, BcfTopicInput,
    BcfViewpoint, BcfWrite, BcfWritten, CommandResult, MAX_BCF_OBJECTS, MAX_BCF_TEXT,
    MAX_BCF_TITLE, MAX_BCF_TOPICS,
};
use kentos_domain::{Document, Uuid};
use kentos_ifc::bcf::{self, Camera, Comment, FileRef, Topic, Viewpoint};

use crate::checks::{self, Stop};
use crate::codes;
use crate::ifc_compare::{ifc_file, named};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// A path that must name a file of one of `extensions` (`field`'s), the file there when `there`, its folder there.
fn path_ok(text: &str, field: &str, extensions: &[&str], there: bool) -> Result<(), Stop> {
    let path = Path::new(text.trim());
    let wrong = |why: String| {
        fail(
            codes::INVALID_FILE_PATH,
            format!("Dosyanın yolu {why}: mutlak bir yol verin."),
            field,
        )
    };
    if text.trim().is_empty() {
        return Err(wrong("boş".into()));
    }
    if !path.is_absolute() {
        return Err(wrong("göreli".into()));
    }
    if !path
        .extension()
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
    {
        return Err(wrong(format!(
            "{} ile bitmiyor",
            extensions
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(" ya da ")
        )));
    }
    if there && !path.is_file() {
        return Err(wrong("bir dosya göstermiyor".into()));
    }
    if !there && !path.parent().is_some_and(Path::is_dir) {
        return Err(wrong("var olmayan bir klasörde".into()));
    }
    Ok(())
}

/// Whether `s` reads as an `xs:dateTime`: `YYYY-MM-DDThh:mm:ss`, a fraction of a second, a zone (`Z`, `±hh:mm`,
/// `±hhmm`) or none.
pub(crate) fn date_ok(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 19 {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let fixed = digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19);
    if !fixed {
        return false;
    }
    let field = |r: std::ops::Range<usize>| s[r].parse::<u32>().unwrap_or(99);
    if !(1..=12).contains(&field(5..7))
        || !(1..=31).contains(&field(8..10))
        || field(11..13) > 23
        || field(14..16) > 59
        || field(17..19) > 60
    {
        return false;
    }
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        let n = r.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        rest = &r[n..];
    }
    match rest.as_bytes() {
        [] | [b'Z'] => true,
        [b'+' | b'-', h1, h2, b':', m1, m2] | [b'+' | b'-', h1, h2, m1, m2] => {
            [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// Text a topic's field may hold: not blank, at most `most` characters, no control character but a line break or a tab.
fn text_ok(s: &str, most: usize) -> bool {
    !s.trim().is_empty()
        && s.chars().count() <= most
        && !s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
}

pub mod write {
    use super::*;

    struct Planned {
        topics: Vec<Topic>,
        out: BcfWritten,
    }

    fn checked(input: &BcfWrite) -> Result<Planned, Stop> {
        path_ok(&input.path, "path", &["bcf"], false)?;
        if let Some(m) = &input.model {
            path_ok(m, "model", &["ifc"], true)?;
        }
        if Path::new(input.path.trim()).exists() && !input.overwrite {
            return Err(fail(
                codes::FILE_EXISTS,
                format!(
                    "{} var; üzerine yazmak için overwrite verin.",
                    input.path.trim()
                ),
                "path",
            ));
        }
        let bad = |why: String, path: String| fail(codes::INVALID_BCF_TOPIC, why, &path);
        if input.topics.is_empty() {
            return Err(bad("Yazılacak konu yok.".into(), "topics".into()));
        }
        if input.topics.len() > MAX_BCF_TOPICS {
            return Err(bad(
                format!("En çok {MAX_BCF_TOPICS} konu yazılır."),
                "topics".into(),
            ));
        }
        let (d, t) = crate::exchange_files::now();
        let now = format!("{d}T{t}Z");
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut drafts: Vec<(Topic, Vec<String>)> = Vec::new();
        for (k, t) in input.topics.iter().enumerate() {
            let at = |f: &str| format!("topics.{k}.{f}");
            let guid = match &t.guid {
                Some(g) if !bcf::guid_ok(g) => {
                    return Err(bad(
                        format!("“{g}” küçük harfli, tireli bir GUID değil."),
                        at("guid"),
                    ));
                }
                Some(g) => g.clone(),
                None => bcf::guid_from(input.path.trim(), k as u64),
            };
            if !seen.insert(guid.clone()) {
                return Err(bad(format!("“{guid}” GUID'i iki konuda."), at("guid")));
            }
            if !text_ok(&t.title, MAX_BCF_TITLE) || t.title.contains('\n') {
                return Err(bad(
                    format!("Başlık boş olamaz, tek satır ve en çok {MAX_BCF_TITLE} karakter."),
                    at("title"),
                ));
            }
            if t.description
                .as_deref()
                .is_some_and(|d| !text_ok(d, MAX_BCF_TEXT))
            {
                return Err(bad(
                    format!("Açıklama boş olamaz ve en çok {MAX_BCF_TEXT} karakter."),
                    at("description"),
                ));
            }
            for (field, value) in [
                ("topicType", &t.topic_type),
                ("status", &t.status),
                ("priority", &t.priority),
                ("assignedTo", &t.assigned_to),
            ] {
                if value
                    .as_deref()
                    .is_some_and(|v| !text_ok(v, 120) || v.contains('\n'))
                {
                    return Err(bad(
                        "Boş olamaz, tek satır ve en çok 120 karakter.".into(),
                        at(field),
                    ));
                }
            }
            if t.labels
                .iter()
                .any(|l| !text_ok(l, 120) || l.contains('\n'))
            {
                return Err(bad("Bir etiket boş ya da çok uzun.".into(), at("labels")));
            }
            if !text_ok(&t.author, 120) || t.author.contains('\n') {
                return Err(bad(
                    "Yazar boş olamaz, en çok 120 karakter.".into(),
                    at("author"),
                ));
            }
            let date = match &t.date {
                Some(d) if !date_ok(d) => {
                    return Err(bad(
                        format!("“{d}” bir gün ve saat değil: YYYY-AA-GGTss:dd:ss yazın."),
                        at("date"),
                    ));
                }
                Some(d) => d.clone(),
                None => now.clone(),
            };
            if t.objects.len() > MAX_BCF_OBJECTS {
                return Err(bad(
                    format!("Bir konu en çok {MAX_BCF_OBJECTS} nesne seçer."),
                    at("objects"),
                ));
            }
            let mut objects: Vec<String> = Vec::new();
            for g in &t.objects {
                if !bcf::ifc_guid_ok(g) {
                    return Err(bad(
                        format!("“{g}” bir IFC GlobalId'si değil (22 karakter, 0-9A-Za-z_$)."),
                        at("objects"),
                    ));
                }
                if objects.contains(g) {
                    return Err(bad(format!("“{g}” iki kez seçilmiş."), at("objects")));
                }
                objects.push(g.clone());
            }
            if !objects.is_empty() && input.model.is_none() {
                return Err(bad(
                    "Nesneli konu için model (IFC dosyası) gerekir: bakışın kamerası nesnelerin gövdelerinden kurulur.".into(),
                    "model".into(),
                ));
            }
            let mut comments = Vec::new();
            for (c, x) in t.comments.iter().enumerate() {
                let at = |f: &str| format!("topics.{k}.comments.{c}.{f}");
                if !text_ok(&x.author, 120) || x.author.contains('\n') {
                    return Err(bad("Yorumun yazarı boş olamaz.".into(), at("author")));
                }
                if !text_ok(&x.text, MAX_BCF_TEXT) {
                    return Err(bad(
                        format!("Yorum boş olamaz ve en çok {MAX_BCF_TEXT} karakter."),
                        at("text"),
                    ));
                }
                let when = match &x.date {
                    Some(d) if !date_ok(d) => {
                        return Err(bad(format!("“{d}” bir gün ve saat değil."), at("date")));
                    }
                    Some(d) => d.clone(),
                    None => date.clone(),
                };
                comments.push(Comment {
                    guid: bcf::guid_from(&guid, 1000 + c as u64),
                    date: when,
                    author: x.author.clone(),
                    text: Some(x.text.clone()),
                    viewpoint: None,
                });
            }
            drafts.push((topic_of(t, guid, date, comments), objects));
        }
        // The model: the objects' bodies and the project's GlobalId.
        let mut topics = Vec::with_capacity(drafts.len());
        let model = match &input.model {
            Some(m) => Some(ifc_file(m, "model")?),
            None => None,
        };
        let boxes = match &model {
            Some((bytes, m)) if drafts.iter().any(|(_, o)| !o.is_empty()) => {
                let made = m
                    .bodies(bytes, &|| false)
                    .map_err(|e| fail(codes::IFC_UNREADABLE, e.to_string(), "model"))?;
                let mut out = vec![None::<[f64; 6]>; m.objects.len()];
                for b in made.bodies {
                    let Some(slot) = out.get_mut(b.object) else {
                        continue;
                    };
                    for p in &b.positions {
                        let k = slot.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
                        for i in 0..3 {
                            k[i] = k[i].min(p[i]);
                            k[i + 3] = k[i + 3].max(p[i]);
                        }
                    }
                }
                out
            }
            _ => Vec::new(),
        };
        let file = model.as_ref().map(|(_, m)| FileRef {
            ifc_project: m
                .objects
                .iter()
                .find(|o| o.class == "IfcProject")
                .map(|o| o.global_id.clone()),
            filename: Path::new(input.model.as_deref().unwrap_or_default().trim())
                .file_name()
                .map(|n| n.to_string_lossy().into_owned()),
            date: None,
            reference: None,
        });
        let mut viewpoints = 0u32;
        for (k, (mut topic, objects)) in drafts.into_iter().enumerate() {
            topic.files = file.clone().into_iter().collect();
            if !objects.is_empty() {
                let Some((_, m)) = &model else {
                    unreachable!("checked above")
                };
                let mut union: Option<[f64; 6]> = None;
                for g in &objects {
                    let Some(i) = m.objects.iter().position(|o| &o.global_id == g) else {
                        return Err(fail(
                            codes::IFC_OBJECT_NOT_FOUND,
                            format!("Modelde GlobalId'si “{g}” olan nesne yok."),
                            &format!("topics.{k}.objects"),
                        ));
                    };
                    if let Some(b) = boxes.get(i).copied().flatten() {
                        let u = union.get_or_insert(b);
                        for a in 0..3 {
                            u[a] = u[a].min(b[a]);
                            u[a + 3] = u[a + 3].max(b[a + 3]);
                        }
                    }
                }
                let Some(b) = union else {
                    return Err(fail(
                        codes::IFC_NO_BODY,
                        format!(
                            "“{}” konusunun nesnelerinin hiçbirinin gövdesi yok: bakılacak yer bilinmiyor.",
                            topic.title
                        ),
                        &format!("topics.{k}.objects"),
                    ));
                };
                topic.viewpoints.push(Viewpoint {
                    guid: bcf::guid_from(&topic.guid, 1),
                    selection: objects,
                    visible: true,
                    exceptions: Vec::new(),
                    coloring: Vec::new(),
                    camera: Some(bcf::camera_for(b)),
                    snapshot: None,
                });
                viewpoints += 1;
            }
            topics.push(topic);
        }
        let out = BcfWritten {
            path: input.path.trim().to_owned(),
            topics: topics.len() as u32,
            viewpoints,
            guids: topics.iter().map(|t| t.guid.clone()).collect(),
        };
        Ok(Planned { topics, out })
    }

    fn topic_of(t: &BcfTopicInput, guid: String, date: String, comments: Vec<Comment>) -> Topic {
        Topic {
            guid,
            topic_type: t.topic_type.clone().unwrap_or_else(|| "Issue".into()),
            status: t.status.clone().unwrap_or_else(|| "Open".into()),
            title: t.title.trim().to_owned(),
            priority: t.priority.clone(),
            labels: t.labels.clone(),
            created: date,
            author: t.author.trim().to_owned(),
            modified: None,
            modified_by: None,
            due: None,
            assigned_to: t.assigned_to.clone(),
            stage: None,
            description: t.description.clone(),
            files: Vec::new(),
            comments,
            viewpoints: Vec::new(),
        }
    }

    pub fn validate(_: &crate::ExecutionContext<'_>, input: &BcfWrite) -> CommandResult<()> {
        match checked(input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(_: &crate::ExecutionContext<'_>, input: &BcfWrite) -> CommandResult<BcfWritten> {
        match checked(input) {
            Ok(p) => CommandResult::Completed {
                output: p.out,
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// Writes the file; the drawing does not change.
    pub fn execute(
        _: &mut crate::ExecutionContext<'_>,
        input: BcfWrite,
    ) -> CommandResult<BcfWritten> {
        let p = match checked(&input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        let bytes = match bcf::write(&p.topics) {
            Ok(b) => b,
            Err(why) => return fail(codes::INVALID_BCF_TOPIC, why, "topics").into(),
        };
        if let Err(why) = crate::exchange_files::write_bytes(Path::new(input.path.trim()), &bytes) {
            return fail(
                codes::INVALID_FILE_PATH,
                format!("BCF yazılamadı: {why}. Hiçbir şey değişmedi."),
                "path",
            )
            .into();
        }
        CommandResult::Completed {
            output: p.out,
            warnings: Vec::new(),
        }
    }
}

pub mod read {
    use super::*;

    fn camera(c: &Camera) -> BcfCamera {
        match c {
            Camera::Perspective {
                eye,
                direction,
                up,
                field_of_view,
                aspect,
            } => BcfCamera {
                kind: "perspective".into(),
                eye: eye.to_vec(),
                direction: direction.to_vec(),
                up: up.to_vec(),
                field_of_view: Some(*field_of_view),
                scale: None,
                aspect: *aspect,
            },
            Camera::Orthogonal {
                eye,
                direction,
                up,
                scale,
                aspect,
            } => BcfCamera {
                kind: "orthogonal".into(),
                eye: eye.to_vec(),
                direction: direction.to_vec(),
                up: up.to_vec(),
                field_of_view: None,
                scale: Some(*scale),
                aspect: *aspect,
            },
        }
    }

    /// The drawing's object a GlobalId KentOS wrote stands for: its persistent id, when the drawing holds it.
    fn uid_of(doc: &Document, gid: &str) -> Option<String> {
        let n = kentos_ifc::step::unguid(gid)?;
        let u = Uuid::from_u128(n);
        doc.slot_of(u).map(|_| u.to_string())
    }

    fn checked(doc: &Document, input: &BcfRead) -> Result<BcfReadOutput, Stop> {
        path_ok(&input.path, "path", &["bcf", "bcfzip"], true)?;
        if let Some(m) = &input.model {
            path_ok(m, "model", &["ifc"], true)?;
        }
        let bytes = std::fs::read(input.path.trim()).map_err(|e| {
            fail(
                codes::INVALID_FILE_PATH,
                format!("{} okunamadı: {e}", input.path.trim()),
                "path",
            )
        })?;
        let b = bcf::read(&bytes).map_err(|why| fail(codes::BCF_UNREADABLE, why, "path"))?;
        let model = match &input.model {
            Some(m) => Some(ifc_file(m, "model")?),
            None => None,
        };
        let topics: Vec<BcfTopic> = b
            .topics
            .iter()
            .map(|t| BcfTopic {
                guid: t.guid.clone(),
                topic_type: t.topic_type.clone(),
                status: t.status.clone(),
                title: t.title.clone(),
                priority: t.priority.clone(),
                labels: t.labels.clone(),
                created: t.created.clone(),
                author: t.author.clone(),
                modified: t.modified.clone(),
                assigned_to: t.assigned_to.clone(),
                description: t.description.clone(),
                files: t
                    .files
                    .iter()
                    .filter_map(|f| f.filename.clone().or_else(|| f.reference.clone()))
                    .collect(),
                comments: t
                    .comments
                    .iter()
                    .map(|c| BcfComment {
                        guid: c.guid.clone(),
                        date: c.date.clone(),
                        author: c.author.clone(),
                        text: c.text.clone(),
                        viewpoint: c.viewpoint.clone(),
                    })
                    .collect(),
                viewpoints: t
                    .viewpoints
                    .iter()
                    .map(|v| BcfViewpoint {
                        guid: v.guid.clone(),
                        selection: v.selection.clone(),
                        visible: v.visible,
                        exceptions: v.exceptions.clone(),
                        camera: v.camera.as_ref().map(camera),
                        snapshot: v.snapshot.clone(),
                    })
                    .collect(),
            })
            .collect();
        let selected: BTreeSet<&str> = b
            .topics
            .iter()
            .flat_map(|t| &t.viewpoints)
            .flat_map(|v| &v.selection)
            .map(String::as_str)
            .collect();
        let objects = selected
            .into_iter()
            .map(|g| BcfObject {
                global_id: g.to_owned(),
                model: model.as_ref().and_then(|(_, m)| {
                    m.objects
                        .iter()
                        .position(|o| o.global_id == g)
                        .map(|i| named(m, i))
                }),
                uid: uid_of(doc, g),
            })
            .collect();
        Ok(BcfReadOutput {
            path: input.path.trim().to_owned(),
            version: b.version,
            topics,
            objects,
        })
    }

    pub fn validate(cx: &crate::ExecutionContext<'_>, input: &BcfRead) -> CommandResult<()> {
        match checked(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &crate::ExecutionContext<'_>, input: &BcfRead) -> CommandResult<BcfReadOutput> {
        match checked(cx.doc, input) {
            Ok(output) => CommandResult::Completed {
                output,
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// Reads only: its answer is its plan's.
    pub fn execute(
        cx: &mut crate::ExecutionContext<'_>,
        input: BcfRead,
    ) -> CommandResult<BcfReadOutput> {
        plan(cx, &input)
    }
}

#[cfg(test)]
mod tests {
    use super::date_ok;

    #[test]
    fn dates_read_as_bcf_writes_them() {
        for ok in [
            "2026-11-17T12:00:00",
            "2026-11-17T12:00:00Z",
            "2026-11-17T12:00:00.250+03:00",
            "2026-11-17T12:00:00-0500",
        ] {
            assert!(date_ok(ok), "{ok}");
        }
        for bad in [
            "2026-11-17",
            "2026-13-17T12:00:00",
            "2026-11-17 12:00:00",
            "2026-11-17T12:00:00+3",
            "17.11.2026",
        ] {
            assert!(!date_ok(bad), "{bad}");
        }
    }
}
