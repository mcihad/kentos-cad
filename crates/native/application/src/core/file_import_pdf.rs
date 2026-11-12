//! `file.import.pdf` v1 (docs/adr/0352, plan/cad.md CAD-IO-21–26): a PDF page's strokes, fills and texts taken into
//! the drawing as its own objects, one undo step “PDF: ad.pdf”. The page is read by our own reader
//! (`kentos_formats::pdf`, docs/adr/0351 §2) in points from its lower left corner as it shows; a point goes into the
//! drawing at `at + p · (0.0254 / 72) · scale`. A stroke is a line or a polyline (its colour, its width on paper, its
//! dash), a fill an area (white is the paper and is left out), a text a text with its height and turn; black is the
//! drawing's ink. Curves are flattened again at the tolerance, or kept exactly as cubic NURBS curves. The page's
//! optional content groups are its layers (“PDF” for what is in none), taken by the import's rule (`import_apply`).
//! The shared cases are `fixtures/commands/v1/file.import.pdf.json` (`scripts/fixtures/pdf_import_cases.py`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CommandResult, CommandWarning, Entity, ImportPdf, ImportedLayer, LayerNodeType, PdfCurveOutput,
    PdfImported, PdfLeftOut, PdfTaken,
};
use kentos_domain::Document;
use kentos_formats::pdf::content::flatten_cubic;
use kentos_formats::pdf::{PdfCurve, PdfError, PdfPage};
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::import_apply::{ImportPlan, LayerTarget, apply_import, fold_turkish, layer_named};

/// Points to paper metres: 1 pt is 1/72 inch.
const PT: f64 = 0.0254 / 72.0;
/// Millimetres to points.
const MM: f64 = 72.0 / 25.4;
/// The tolerance a curve is flattened at when none is given, paper millimetres.
pub const DEFAULT_TOLERANCE: f64 = 0.01;
/// The source layer of what is in no optional content group.
pub const PLAIN: &str = "PDF";
/// What a PDF cannot carry back (docs/adr/0352 §5).
const NOTES: [&str; 2] = [
    "PDF yazdırılanı taşır, çizimi değil: ölçüler, kılavuzlar, taramalar ve tablolar çizgileri ve yazılarıyla geldi; ilişkileri ve değerleri yok.",
    "Ölçüler kâğıttan okunan çizgilerdir: kaynağın kesinliğini değil, basılmış sayfanınkini taşır.",
];

/// The undo step's name: “PDF: ad.pdf”.
pub fn step_name(file: &str) -> String {
    format!("PDF: {file}")
}

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// Checks `input`, reading the file.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportPdf) -> CommandResult<()> {
    let mut copy = cx.doc.reading_copy();
    match run(&mut copy, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the import would take: the box, the layers and what is left out; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportPdf) -> CommandResult<PdfImported> {
    let mut copy = cx.doc.reading_copy();
    match run(&mut copy, input) {
        Ok((mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Takes the page in, one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImportPdf) -> CommandResult<PdfImported> {
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_PDF,
        kentos_commands::FILE_IMPORT_PDF_VERSION,
        input,
        |cx, input| match run(cx.doc, &input) {
            Ok((output, warnings)) => CommandResult::Completed { output, warnings },
            Err(stop) => stop.into(),
        },
    )
}

/// The numbers: a scale, a tolerance and a region that make sense.
fn check_numbers(input: &ImportPdf) -> Result<(), Stop> {
    let path = Path::new(&input.path);
    if input.path.trim().is_empty() || !path.is_absolute() || path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” okunacak bir dosyanın mutlak yolu değil; PDF dosyasının tam yolunu verin (/klasör/ad.pdf).",
                input.path
            ),
            "path",
        ));
    }
    if let Some(s) = input.scale
        && !(s.is_finite() && s > 0.0 && s <= 1_000_000.0)
    {
        return Err(refuse(
            codes::INVALID_SCALE,
            format!(
                "Ölçek 1:{s} olamaz; sayfanın basıldığı ölçeğin paydasını verin (1 ile 1 000 000 arası, 50 → 1:50)."
            ),
            "scale",
        ));
    }
    if let Some(t) = input.tolerance
        && !(t.is_finite() && t > 0.0 && t <= 100.0)
    {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            format!(
                "Eğrinin toleransı {t} mm olamaz; kâğıtta sıfırdan büyük, en çok 100 mm bir uzaklık verin."
            ),
            "tolerance",
        ));
    }
    if let Some(r) = &input.region {
        let finite = [r.a.x, r.a.y, r.b.x, r.b.y].iter().all(|v| v.is_finite());
        if !finite || (r.a.x - r.b.x).abs() <= 0.0 || (r.a.y - r.b.y).abs() <= 0.0 {
            return Err(refuse(
                codes::INVALID_PDF_REGION,
                "Bölgenin eni ya da boyu yok; sayfada iki karşı köşesi farklı bir dikdörtgen verin (milimetre).",
                "region",
            ));
        }
    }
    Ok(())
}

struct Read {
    page: PdfPage,
    file: String,
    bytes: u64,
    sha256: String,
}

fn read_file(input: &ImportPdf) -> Result<Read, Stop> {
    let path = Path::new(&input.path);
    if !path.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} yok; dosyanın yolunu denetleyin.", input.path),
            "path",
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!(
                "{} okunamadı: {e}. Dosyanın izinlerini denetleyin.",
                input.path
            ),
            "path",
        )
    })?;
    let file = path
        .file_name()
        .map_or_else(|| input.path.clone(), |n| n.to_string_lossy().into_owned());
    let number = input.page.unwrap_or(1);
    let page = kentos_formats::pdf::read_page(&bytes, number).map_err(|e| match e {
        PdfError::NoPage { count } => refuse(
            codes::PDF_PAGE_NOT_FOUND,
            format!(
                "{file} dosyasında {number}. sayfa yok; 1 ile {count} arasında bir sayfa verin."
            ),
            "page",
        ),
        e => refuse(
            codes::IMPORT_UNREADABLE,
            format!("{file} PDF olarak okunamadı: {e}."),
            "path",
        ),
    })?;
    Ok(Read {
        page,
        file,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
    })
}

/// The page's box of the region, in points, and whether a point lies in it (its edges count).
struct Region([f64; 4]);

impl Region {
    fn holds(&self, p: [f64; 2]) -> bool {
        p[0] >= self.0[0] && p[0] <= self.0[2] && p[1] >= self.0[1] && p[1] <= self.0[3]
    }
}

/// How a page's point goes into the drawing.
struct Place {
    at: [f64; 2],
    k: f64,
}

impl Place {
    fn point(&self, p: [f64; 2]) -> [f64; 2] {
        [self.at[0] + p[0] * self.k, self.at[1] + p[1] * self.k]
    }
    fn json(&self, p: [f64; 2]) -> Value {
        let q = self.point(p);
        json!({ "x": q[0], "y": q[1] })
    }
}

fn color(c: [u8; 3]) -> Option<String> {
    (c != [0, 0, 0]).then(|| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
}

fn with_look(mut e: Value, c: Option<String>, weight: Option<f64>, dashed: bool) -> Value {
    if let Some(c) = c {
        e["color"] = json!(c);
    }
    if let Some(w) = weight {
        e["lineWeight"] = json!(w);
    }
    if dashed {
        e["lineType"] = json!("dashed");
    }
    e
}

/// A part's points with its curves flattened at `tol` points (the reader's own flattening between them kept).
fn flattened(points: &[[f64; 2]], curves: &[PdfCurve], tol: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(points.len());
    let mut i = 0;
    for c in curves {
        while i <= c.at && i < points.len() {
            out.push(points[i]);
            i += 1;
        }
        flatten_cubic(c.c, tol, &mut out);
        i = c.at + c.n + 1;
    }
    out.extend_from_slice(points.get(i..).unwrap_or(&[]));
    out
}

/// A run of a part as it goes out: straight points, or cubic pieces end to end.
enum Run {
    Straight(Vec<[f64; 2]>),
    Curved(Vec<[[f64; 2]; 4]>),
}

/// A part split into its straight runs and its runs of curves; a closed part's closing edge is a straight run.
fn runs(points: &[[f64; 2]], curves: &[PdfCurve], closed: bool) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    let mut straight: Vec<[f64; 2]> = Vec::new();
    let mut i = 0;
    for c in curves {
        while i <= c.at && i < points.len() {
            straight.push(points[i]);
            i += 1;
        }
        if straight.len() >= 2 {
            out.push(Run::Straight(std::mem::take(&mut straight)));
        }
        straight.clear();
        match out.last_mut() {
            Some(Run::Curved(pieces)) if pieces.last().is_some_and(|l| l[3] == c.c[0]) => {
                pieces.push(c.c)
            }
            _ => out.push(Run::Curved(vec![c.c])),
        }
        i = c.at + c.n + 1;
        straight.push(c.c[3]);
    }
    straight.extend_from_slice(points.get(i..).unwrap_or(&[]));
    if closed
        && let (Some(first), Some(last)) = (points.first(), straight.last())
        && first != last
    {
        straight.push(*first);
    }
    if straight.len() >= 2 {
        out.push(Run::Straight(straight));
    }
    out
}

/// A run of `m` cubic pieces as a NURBS curve: their control points end to end, the knots `0 0 0 0 1 1 1 … m m m m`.
fn nurbs(pieces: &[[[f64; 2]; 4]], place: &Place) -> Value {
    let mut ctrl = vec![place.json(pieces[0][0])];
    for p in pieces {
        ctrl.extend(p[1..].iter().map(|q| place.json(*q)));
    }
    let m = pieces.len();
    let mut knots = vec![0.0; 4];
    for k in 1..m {
        knots.extend([k as f64; 3]);
    }
    knots.extend([m as f64; 4]);
    let mut e = json!({ "kind": "nurbs", "degree": 3, "ctrl": ctrl, "knots": knots });
    if pieces[0][0] == pieces[m - 1][3] {
        e["closed"] = json!(true);
    }
    e
}

/// A stroked line of points as a line or a polyline; none for fewer than two different points.
fn stroke_object(points: &[[f64; 2]], closed: bool, place: &Place) -> Option<Value> {
    let mut line: Vec<[f64; 2]> = points.to_vec();
    let mut closed = closed;
    if line.len() > 2 && line.first() == line.last() {
        line.pop();
        closed = true;
    }
    line.dedup();
    if line.len() < 2 {
        return None;
    }
    Some(if line.len() == 2 && !closed {
        json!({ "kind": "line", "a": place.json(line[0]), "b": place.json(line[1]) })
    } else {
        let mut e = json!({ "kind": "polyline", "pts": line.iter().map(|p| place.json(*p)).collect::<Vec<_>>() });
        if closed {
            e["closed"] = json!(true);
        }
        e
    })
}

/// The objects of the page, each with its source layer, and what was counted on the way.
struct Made {
    objects: Vec<(String, Value)>,
    taken: PdfTaken,
    left: PdfLeftOut,
}

fn layer_name(page: &PdfPage, i: Option<usize>) -> String {
    i.and_then(|i| page.layers.get(i).cloned())
        .unwrap_or_else(|| PLAIN.to_owned())
}

fn made(page: &PdfPage, input: &ImportPdf, region: Option<&Region>, place: &Place) -> Made {
    let tol = input.tolerance.unwrap_or(DEFAULT_TOLERANCE) * MM;
    let mut out = Made {
        objects: Vec::new(),
        taken: PdfTaken::default(),
        left: PdfLeftOut {
            images: page.left.images,
            shadings: page.left.shadings,
            unreadable: page.left.unreadable,
            white: 0,
            outside: 0,
            truncated: page.left.truncated,
        },
    };
    let push = |out: &mut Made, layer: &str, e: Value| {
        match e["kind"].as_str() {
            Some("line") => out.taken.lines += 1,
            Some("polyline") => out.taken.polylines += 1,
            Some("nurbs") => out.taken.curves += 1,
            Some("polygon") => out.taken.areas += 1,
            _ => out.taken.texts += 1,
        }
        out.objects.push((layer.to_owned(), e));
    };
    for path in &page.paths {
        let layer = layer_name(page, path.layer);
        let fill = path.fill.filter(|_| input.fills);
        let stroke = path.stroke.as_ref().filter(|_| input.strokes);
        if fill.is_none() && stroke.is_none() {
            continue;
        }
        let empty = Vec::new();
        for (k, (points, closed)) in path.parts.iter().enumerate() {
            if region.is_some_and(|r| !points.iter().all(|p| r.holds(*p))) {
                out.left.outside += 1;
                continue;
            }
            let curves = path.curves.get(k).unwrap_or(&empty);
            if let Some(fill) = fill {
                if fill == [255, 255, 255] {
                    out.left.white += 1;
                } else {
                    let mut ring = flattened(points, curves, tol);
                    if ring.len() > 1 && ring.first() == ring.last() {
                        ring.pop();
                    }
                    ring.dedup();
                    if ring.len() >= 3 {
                        let e = json!({ "kind": "polygon", "pts": ring.iter().map(|p| place.json(*p)).collect::<Vec<_>>() });
                        push(&mut out, &layer, with_look(e, color(fill), None, false));
                    }
                }
            }
            let Some(stroke) = stroke else { continue };
            // Its width on paper, millimetres; a hairline (0) is the thinnest.
            let weight = (stroke.width > 0.0)
                .then(|| (stroke.width * 25.4 / 72.0 * 100.0).round() / 100.0)
                .filter(|w| *w > 0.0);
            let look = |e: Value| with_look(e, color(stroke.color), weight, stroke.dashed);
            if input.curves == PdfCurveOutput::Nurbs && !curves.is_empty() {
                for run in runs(points, curves, *closed) {
                    match run {
                        Run::Straight(pts) => {
                            if let Some(e) = stroke_object(&pts, false, place) {
                                push(&mut out, &layer, look(e));
                            }
                        }
                        Run::Curved(pieces) => push(&mut out, &layer, look(nurbs(&pieces, place))),
                    }
                }
            } else if let Some(e) = stroke_object(&flattened(points, curves, tol), *closed, place) {
                push(&mut out, &layer, look(e));
            }
        }
    }
    if input.texts {
        for t in &page.texts {
            if region.is_some_and(|r| !r.holds(t.p)) {
                out.left.outside += 1;
                continue;
            }
            let e = json!({ "kind": "text", "p": place.json(t.p), "text": t.text, "height": t.height * place.k,
                            "rotation": t.angle.to_degrees() });
            push(
                &mut out,
                &layer_name(page, t.layer),
                with_look(e, color(t.color), None, false),
            );
        }
    }
    out
}

/// Where each source layer goes: the layers left out skipped, the routed ones checked, the others onto the drawing's
/// layer of the same name or a new one.
fn plan_of(
    doc: &Document,
    input: &ImportPdf,
    page: &PdfPage,
    used: &[String],
    file: &str,
) -> Result<ImportPlan, Stop> {
    let routes: HashMap<String, (&String, &String)> = input
        .layers
        .iter()
        .map(|(s, id)| (fold_turkish(s), (s, id)))
        .collect();
    let mut layers = Vec::new();
    for name in used {
        let target = if let Some((source, id)) = routes.get(&fold_turkish(name)) {
            let at = format!("layers.{source}");
            match doc.layers().get(id) {
                None => {
                    return Err(refuse(
                        codes::LAYER_NOT_FOUND,
                        format!(
                            "“{id}” kimlikli katman çizimde yok; “{name}” katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin."
                        ),
                        &at,
                    ));
                }
                Some(node) if node.kind != LayerNodeType::Layer => {
                    return Err(refuse(
                        codes::NOT_A_LAYER,
                        format!(
                            "“{}” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı verin.",
                            node.name
                        ),
                        &at,
                    ));
                }
                Some(node) if doc.layers().is_locked(id) => {
                    return Err(refuse(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                            node.name
                        ),
                        &at,
                    ));
                }
                Some(_) => LayerTarget::Existing((*id).clone()),
            }
        } else if let Some(node) = layer_named(doc, name) {
            if doc.layers().is_locked(&node.id) {
                return Err(refuse(
                    codes::LAYER_LOCKED,
                    format!(
                        "PDF'in “{name}” katmanı çizimdeki aynı adlı “{}” katmanına gider ama o katman kilitli. Kilidi açın, katmanı skipLayers ile dışarıda bırakın ya da layers ile başka bir katmana yönlendirin.",
                        node.name
                    ),
                    "layers",
                ));
            }
            LayerTarget::Existing(node.id.clone())
        } else {
            let hidden = page
                .layers
                .iter()
                .position(|l| l == name)
                .is_some_and(|i| page.hidden.get(i).copied().unwrap_or(false));
            LayerTarget::New {
                name: name.clone(),
                style: Box::new(kentos_domain::default_style()),
                visible: !hidden,
                locked: false,
                fields: Vec::new(),
                plot: None,
                frozen: None,
            }
        };
        layers.push((name.clone(), target));
    }
    let group = match &input.group {
        None => Some(file.to_owned()),
        Some(name) if name.trim().is_empty() => None,
        Some(name) => Some(name.trim().to_owned()),
    };
    Ok(ImportPlan {
        source_srid: None,
        label: step_name(file),
        layers,
        group,
    })
}

fn bounds(objects: &[(String, Value)]) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    let mut add = |p: &Value| {
        if let (Some(x), Some(y)) = (p["x"].as_f64(), p["y"].as_f64()) {
            let r = b.get_or_insert([x, y, x, y]);
            r[0] = r[0].min(x);
            r[1] = r[1].min(y);
            r[2] = r[2].max(x);
            r[3] = r[3].max(y);
        }
    };
    for (_, e) in objects {
        for key in ["a", "b", "p"] {
            add(&e[key]);
        }
        for key in ["pts", "ctrl"] {
            for p in e[key].as_array().into_iter().flatten() {
                add(p);
            }
        }
    }
    b
}

fn losses(left: &PdfLeftOut) -> Option<String> {
    let mut parts = Vec::new();
    if left.images > 0 {
        parts.push(format!("{} resim", left.images));
    }
    if left.shadings > 0 {
        parts.push(format!("{} gölgeleme", left.shadings));
    }
    if left.unreadable > 0 {
        parts.push(format!(
            "{} okunamayan harf (yazı tipinin harf eşlemesi yok)",
            left.unreadable
        ));
    }
    if left.white > 0 {
        parts.push(format!("{} beyaz dolgu (kâğıt sayıldı)", left.white));
    }
    if left.truncated {
        parts.push("sayfanın sınırı aşan geri kalanı".to_owned());
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn run(doc: &mut Document, input: &ImportPdf) -> Result<(PdfImported, Vec<CommandWarning>), Stop> {
    check_numbers(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let read = read_file(input)?;
    let page = &read.page;
    let region = match &input.region {
        None => None,
        Some(r) => {
            let b = [
                r.a.x.min(r.b.x) * MM,
                r.a.y.min(r.b.y) * MM,
                r.a.x.max(r.b.x) * MM,
                r.a.y.max(r.b.y) * MM,
            ];
            if b[2] < 0.0 || b[3] < 0.0 || b[0] > page.width || b[1] > page.height {
                return Err(refuse(
                    codes::INVALID_PDF_REGION,
                    format!(
                        "Bölge sayfanın dışında; sayfa {:.1} × {:.1} mm, bölgeyi onun sol alt köşesinden milimetreyle verin.",
                        page.width / MM,
                        page.height / MM
                    ),
                    "region",
                ));
            }
            Some(Region(b))
        }
    };
    let at = input.at.map_or([0.0, 0.0], |p| [p.x, p.y]);
    let place = Place {
        at,
        k: PT * input.scale.unwrap_or(1.0),
    };
    let skip: HashSet<String> = input.skip_layers.iter().map(|n| fold_turkish(n)).collect();
    let mut made = made(page, input, region.as_ref(), &place);
    made.objects
        .retain(|(l, _)| !skip.contains(&fold_turkish(l)));
    if made.objects.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_IMPORT,
            format!(
                "{} dosyasının {}. sayfasında seçilenlerden alınacak nesne yok; bölgeyi, katmanları ve alınacakları denetleyin.",
                read.file, page.number
            ),
            if input.skip_layers.is_empty() {
                "path"
            } else {
                "skipLayers"
            },
        ));
    }
    // The source layers in the page's order: what is in no group first.
    let mut used: Vec<String> = Vec::new();
    for name in std::iter::once(PLAIN.to_owned()).chain(page.layers.iter().cloned()) {
        if !used.contains(&name) && made.objects.iter().any(|(l, _)| *l == name) {
            used.push(name);
        }
    }
    let plan = plan_of(doc, input, page, &used, &read.file)?;
    let entities = made
        .objects
        .iter()
        .map(|(layer, e)| {
            let mut e = e.clone();
            e["id"] = json!(0);
            e["layerId"] = json!(layer);
            e["attrs"] = json!({});
            serde_json::from_value::<Entity>(e).map_err(|why| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    format!("{} sayfasından bir nesne kurulamadı: {why}.", read.file),
                    "path",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let applied = apply_import(doc, entities, Vec::new(), &plan)
        .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "path"))?;
    let mut warnings = Vec::new();
    if let Some(words) = losses(&made.left) {
        warnings.push(CommandWarning {
            code: codes::IMPORT_LOSSES.into(),
            message: format!("{} sayfasından alınmadı: {words}.", read.file),
            path: Some("path".into()),
        });
    }
    let targets: BTreeMap<String, String> = applied.targets.iter().cloned().collect();
    Ok((
        PdfImported {
            file: read.file.clone(),
            bytes: read.bytes,
            sha256: read.sha256.clone(),
            page: page.number,
            pages: page.count,
            paper: [page.width / MM, page.height / MM],
            bounds: bounds(&made.objects),
            objects: applied.slots.len() as u32,
            taken: made.taken,
            left: made.left,
            targets,
            layers: applied
                .made
                .iter()
                .map(|(id, name)| ImportedLayer {
                    id: id.clone(),
                    name: name.clone(),
                })
                .collect(),
            group: applied.group.clone(),
            notes: NOTES.iter().map(|n| (*n).to_owned()).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}
