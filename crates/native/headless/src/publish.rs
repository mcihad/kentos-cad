//! Atomic sheet delivery. Planning writes nothing; publication exposes the whole folder in one rename.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use kentos_commands::{
    PublishSheets, SheetPublishMode, SheetPublishPin, SheetPublishedFile, SheetPublishIssue,
    SheetsPublished, CallOp, CommandResult, CommandError, FileEffect, encoding::sha256_hex,
    SheetPublishFrame, SheetPublishScope, SheetPublishedPage,
};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
use kentos_sheet::{
    SheetBook,
    project::read_project_sheets,
};
use crate::file_commands::Refused;
const MANIFEST: &str = "yayin.json";
pub(crate) fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(CommandError {
        code: code.into(),
        message: message.into(),
        path: Some(path.into()),
        ..Default::default()
    })
}
pub(crate) fn stopped(progress: Option<&Progress>) -> Result<(), Refused> {
    if progress.is_some_and(Progress::stopped) {
        Err(refuse(
            "cancelled",
            "Yayın iptal edildi; teslim klasörü oluşturulmadı.",
            "path",
        ))
    } else {
        Ok(())
    }
}
/// A deterministic document pin. Linked-resource version pinning extends this at the plot boundary.
pub fn pin(doc: &Document) -> Result<SheetPublishPin, Refused> {
    kentos_plot::provenance::document_pin(doc).map_err(|e|refuse(&e.code,e.message,"pin"))
}
fn stem(value: &str) -> String {
    let mut s: String = value
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim().trim_matches('.').trim_end().to_owned();
    if s.to_ascii_lowercase().ends_with(".pdf") {
        s.truncate(s.len() - 4);
    }
    while s.len() > 200 {
        s.pop();
    }
    let base = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (1..=9).any(|i| base == format!("COM{i}") || base == format!("LPT{i}"))
    {
        s.insert(0, '_');
    }
    if s.is_empty() { "pafta".into() } else { s }
}
fn names(
    book: &SheetBook,
    chosen: &[String],
    input: &PublishSheets,
) -> Result<Vec<(String, Vec<String>)>, Refused> {
    if input.mode == SheetPublishMode::Single {
        return Ok(vec![(
            format!("{}.pdf", stem(input.name.as_deref().unwrap_or("paftalar"))),
            chosen.to_vec(),
        )]);
    }
    if input.naming.trim().is_empty()
        || input.naming.len() > 1000
        || input.naming.contains(['/', '\\', '\0'])
    {
        return Err(refuse(
            "invalid_input",
            "Dosya adı şablonu boş olamaz; klasör ayırıcısı içeremez. {index}, {sheet}, {id} kullanın.",
            "naming",
        ));
    }
    let rest = input
        .naming
        .replace("{index}", "")
        .replace("{sheet}", "")
        .replace("{id}", "")
        .replace("{number}", "")
        .replace("{title}", "")
        .replace("{revision}", "")
        .replace("{status}", "")
        .replace("{subset}", "");
    if rest.contains(['{', '}']) {
        return Err(refuse(
            "invalid_input",
            "Dosya adında tanınmayan yer tutucu var; {index}, {sheet}, {id} kullanın.",
            "naming",
        ));
    }
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for (n, id) in chosen.iter().enumerate() {
        let sheet = book
            .sheet(id)
            .ok_or_else(|| refuse("unknown_sheet", format!("“{id}” paftası yok."), "sheets"))?;
        let info = sheet.drawing_info.clone().unwrap_or_default();
        let mut rendered = String::new();
        let mut rest = input.naming.as_str();
        while let Some((before, tail)) = rest.split_once('{') {
            rendered.push_str(before);
            let Some((key, after)) = tail.split_once('}') else {
                break;
            };
            rendered.push_str(&match key {
                "index" => format!("{:03}", n + 1),
                "sheet" => sheet.name.clone(),
                "id" => id.clone(),
                "number" => info.number.clone(),
                "title" => {
                    if info.title.is_empty() {
                        sheet.name.clone()
                    } else {
                        info.title.clone()
                    }
                }
                "revision" => info.revision.clone(),
                "status" => info.status.clone(),
                "subset" => info.subset.join("-"),
                _ => String::new(),
            });
            rest = after;
        }
        rendered.push_str(rest);
        let base = stem(&rendered);
        let mut name = format!("{base}.pdf");
        let mut suffix = 2;
        while !used.insert(name.to_lowercase()) {
            name = format!("{base}-{suffix}.pdf");
            suffix += 1;
        }
        out.push((name, vec![id.clone()]));
    }
    Ok(out)
}
struct Stage {
    path: PathBuf,
    committed: bool,
    /// A resumable publication's stage survives a stop or failure (docs/adr/1090 §7).
    keep: bool,
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.committed && !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn stage(target: &Path) -> Result<Stage, Refused> {
    let parent = target.parent().ok_or_else(|| {
        refuse(
            "invalid_file_path",
            "Teslim klasörünün üst klasörü yok.",
            "path",
        )
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = parent.join(format!(
        ".kentos-yayin-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir(&path).map_err(|e| {
        refuse(
            "file_unwritable",
            format!("Geçici teslim klasörü açılamadı: {e}"),
            "path",
        )
    })?;
    Ok(Stage {
        path,
        committed: false,
        keep: false,
    })
}
pub(crate) fn issue(sheet: &str, code: &str, message: String, error: bool, fix: String) -> SheetPublishIssue {
    SheetPublishIssue {
        sheet: sheet.into(),
        code: code.into(),
        message,
        fix,
        error,
    }
}
/// The hidden sibling folder a resumable publication of `target` keeps its finished pages and journal in
/// (docs/adr/1090 §7): `.<folder>.kentos-devam`, never inside the delivery folder.
pub(crate) fn journal_dir(target: &Path) -> Option<PathBuf> {
    let leaf = target.file_name()?.to_string_lossy().into_owned();
    Some(target.parent()?.join(format!(".{leaf}.kentos-devam")))
}
pub(crate) const JOURNAL: &str = "devam.json";
pub(crate) const JOURNAL_FORMAT: &str = "kentos.publish-resume/1";

/// The options a resumed publication must share with the one it goes on from.
fn journal_options(input: &PublishSheets) -> serde_json::Value {
    serde_json::json!({"mode":input.mode,"naming":input.naming,"name":input.name,"dpi":input.dpi,"geo":input.geo,"layers":input.layers,"resources":input.resources,"drawing":input.drawing})
}

/// The scope summary of the chosen sheets (docs/adr/1090 §7).
pub(crate) fn scope_of(book: &SheetBook, chosen: &[String], views: &[kentos_commands::NamedView]) -> Vec<SheetPublishScope> {
    chosen
        .iter()
        .filter_map(|id| book.sheet(id))
        .map(|s| {
            let info = s.drawing_info.clone().unwrap_or_default();
            let frames = s
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    kentos_sheet::ItemKind::Map(m) if m.overview_of.is_none() => Some(SheetPublishFrame {
                        item: item.id.clone(),
                        name: item.name.clone(),
                        denominator: match &m.view {
                            kentos_sheet::MapView::Fixed(_) => kentos_sheet::viewport::effective_fixed_view(item, views)
                                .ok()
                                .map(|v| v.denominator()),
                            kentos_sheet::MapView::Atlas(_) => None,
                        },
                        locked: m.view_locked,
                        layers: match &m.layers {
                            kentos_sheet::kinds::MapLayers::All(_) => "drawing".into(),
                            kentos_sheet::kinds::MapLayers::List(_) => "list".into(),
                            kentos_sheet::kinds::MapLayers::Theme(t) => format!("theme:{}", t.name),
                        },
                        annotation: m.annotation_scale,
                    }),
                    _ => None,
                })
                .collect();
            SheetPublishScope {
                sheet: s.id.clone(),
                name: s.name.clone(),
                number: info.number,
                revision: info.revision,
                paper: [f64::from(s.page.size.width) / 1000., f64::from(s.page.size.height) / 1000.],
                profile: kentos_sheet::plot_profile::for_sheet(book, &s.id).ok().flatten().map(|p| p.name.clone()),
                frames,
                tables: s.items.iter().filter(|i| matches!(i.kind, kentos_sheet::ItemKind::Table(_))).count() as u32,
            }
        })
        .collect()
}

/// A written page's audit as the contract carries it, with its findings (docs/adr/1090 §4): a zero-width line, lines
/// under 0,05 mm, transparency, a viewport drawn as a picture or empty, a viewport that follows the drawing's live
/// layer visibility. None is an error by itself.
pub(crate) fn page_of(a: &kentos_sheet::pdf::PageAudit, file: &str, digest: &str, scope: Option<&SheetPublishScope>) -> SheetPublishedPage {
    let mut findings = Vec::new();
    let mut note = |code: &str, message: String, fix: &str| {
        findings.push(issue(&a.sheet, code, message, false, fix.into()));
    };
    if a.hairlines > 0 {
        note("pdf_hairline", format!("{} çizgi sıfır kalınlıkta (cihazın en ince çizgisi): yazıcıdan yazıcıya değişir.", a.hairlines), "Baskı profilinde ya da katmanda bir çizgi kalınlığı verin.");
    }
    if a.thin_lines > 0 {
        note("pdf_thin_line", format!("{} çizgi {} mm'den ince; en incesi {:.3} mm.", a.thin_lines, kentos_sheet::pdf::THIN_LINE_MM, a.min_stroke_mm.unwrap_or(0.)), "Okunmayacak kadar ince çizgileri baskı profilinde kalınlaştırın.");
    }
    if a.transparent > 0 {
        note("pdf_transparency", format!("{} boyama saydam; bazı yazıcılar ve RIP'ler saydamlığı düzleştirir.", a.transparent), "Baskı önizlemesiyle denetleyin; gerekirse saydamlığı kaldırın.");
    }
    if a.raster_maps > 0 {
        note("pdf_rasterized_view", format!("{} görünüm penceresi resim olarak yazıldı: çizgileri ve taramaları vektör değil, yazıları aranamaz.", a.raster_maps), "Resme dönmeyi gerektiren kaynağı (raster, nokta bulutu, servis) denetleyin ya da çözünürlüğü artırın.");
    }
    if a.placeholders > 0 {
        note("pdf_empty_view", format!("{} görünüm penceresi boş “harita” kutusu olarak yazıldı.", a.placeholders), "Görünüm penceresinin kaynağını ve yerleşimini denetleyin.");
    }
    for f in scope.into_iter().flat_map(|s| s.frames.iter()).filter(|f| f.layers == "drawing" && f.denominator.is_some()) {
        note("viewport_live_layers", format!("“{}” görünümü çizimin anlık katman görünürlüğünü izliyor: aynı projede biri katman açıp kapatırsa baskı değişir.", f.name), "Görünüme bir katman durumu (tema) atayın.");
    }
    SheetPublishedPage {
        sheet: a.sheet.clone(),
        file: file.into(),
        digest: digest.into(),
        date: None,
        strokes: a.strokes,
        fills: a.fills,
        even_odd_fills: a.even_odd_fills,
        hairlines: a.hairlines,
        thin_lines: a.thin_lines,
        min_stroke_mm: a.min_stroke_mm,
        texts: a.texts,
        fonts: a.fonts.clone(),
        clips: a.clips,
        transparent: a.transparent,
        images: a.images,
        raster_maps: a.raster_maps,
        placeholders: a.placeholders,
        findings,
    }
}

/// Each chosen sheet's content digest without the title context (docs/adr/1090 §4), by id; a sheet whose page cannot
/// be made has none.
pub(crate) fn digests(
    captured: &kentos_plot::provenance::Capture,
    chosen: &[String],
    geo: bool,
    layers: bool,
    progress: Option<&Progress>,
) -> Result<Vec<(String, Result<String, (String, String)>)>, Refused> {
    let neutral = captured.neutral();
    let mut out = Vec::with_capacity(chosen.len());
    for id in chosen {
        stopped(progress)?;
        let opts = kentos_sheet::pdf::PdfOptions { sheets: vec![id.clone()], geo, layers };
        let got = match kentos_plot::book::book_pdf_captured(&neutral, std::slice::from_ref(id), Some(opts), None) {
            Ok(pdf) => pdf.pages.first().map(|p| p.digest.clone()).ok_or_else(|| ("pdf_no_page".to_owned(), "Sayfa yazılmadı.".to_owned())),
            Err(e) if e.code == "cancelled" => return Err(refuse("cancelled", e.message, "path")),
            Err(e) => Err((e.code, e.message)),
        };
        out.push((id.clone(), got));
    }
    Ok(out)
}

/// Shared by direct, job, Python, MCP and desktop publication. No delivery folder survives an error/cancel; a
/// resumable one keeps its finished pages beside it (docs/adr/1090 §7).
fn work(
    doc: &Document,
    input: &PublishSheets,
    op: CallOp,
    progress: Option<&Progress>,
) -> Result<(SheetsPublished, Option<FileEffect>), Refused> {
    let previous = crate::publish_update::read(input, Path::new(&input.path))?;
    let target = crate::package::target(&input.path)?;
    stopped(progress)?;
    let source = pin(doc)?;
    if input.pin.as_ref().is_some_and(|p| p.revision != source.revision || p.sha256 != source.sha256) {
        return Err(refuse(
            "pin_mismatch",
            "Çizim önizlemeden sonra değişti; yayını yeniden denetleyin.",
            "pin",
        ));
    }
    if input.resume && input.mode != SheetPublishMode::Separate {
        return Err(refuse(
            "invalid_input",
            "Yalnız ayrı PDF'lerle yayın devam ettirilebilir; tek PDF baştan yazılır.",
            "resume",
        ));
    }
    let kept = doc.sheets().ok_or_else(|| {
        refuse(
            "no_sheets",
            "Çizimde pafta yok; önce bir pafta oluşturun.",
            "sheets",
        )
    })?;
    let (mut book, _) =
        read_project_sheets(kept).map_err(|e| refuse("book_unreadable", e.message, "sheets"))?;
    let chosen = input
        .sheets
        .clone()
        .unwrap_or_else(|| book.sheets.iter().map(|s| s.id.clone()).collect());
    if chosen.is_empty() {
        return Err(refuse(
            "no_sheets",
            "Yayınlanacak en az bir pafta seçin.",
            "sheets",
        ));
    }
    let mut seen = BTreeSet::new();
    for id in &chosen {
        if book.sheet(id).is_none() {
            return Err(refuse(
                "unknown_sheet",
                format!("“{id}” paftası kitapta yok."),
                "sheets",
            ));
        }
        if !seen.insert(id) {
            return Err(refuse(
                "invalid_input",
                format!("“{id}” iki kez seçildi; her pafta bir kez yayınlanır."),
                "sheets",
            ));
        }
    }
    // The book as kept: its sheets' records are what a later status compares (docs/adr/1320 §2).
    let kept_book = book.clone();
    if let Some(dpi) = input.dpi {
        if !(36..=1200).contains(&dpi) {
            return Err(refuse(
                "invalid_input",
                "Çözünürlük 36–1200 dpi olmalı.",
                "dpi",
            ));
        }
        for s in &mut book.sheets {
            s.export.dpi = dpi;
        }
    }
    let planned = names(&book, &chosen, input)?;
    let scope = scope_of(&book, &chosen, &doc.settings().drawn_views());
    if matches!(op, CallOp::Validate) {
        return Ok((
            SheetsPublished {
                resources: None,
                path: input.path.clone(),
                sheets: chosen,
                files: Vec::new(),
                issues: Vec::new(),
                source,
                ready: false,
                manifest: target.join(MANIFEST).display().to_string(),
                sha256: String::new(),
                scope,
                pages: Vec::new(),
                resumed: Vec::new(),
                previous: previous.map(|p| p.report),
                carried: Vec::new(),
            },
            None,
        ));
    }
    let resumable = input.mode == SheetPublishMode::Separate
        && (input.resumable || input.resume)
        && matches!(op, CallOp::Execute);
    let journal_at = journal_dir(&target);
    // A resumed publication takes its title context and finished pages from its journal.
    let journal: Option<serde_json::Value> = if input.resume && matches!(op, CallOp::Execute) {
        let at = journal_at.as_ref().map(|d| d.join(JOURNAL));
        let read = at
            .as_ref()
            .and_then(|p| fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .filter(|j| j["format"] == JOURNAL_FORMAT);
        Some(read.ok_or_else(|| {
            refuse(
                "no_resume",
                "Bu klasörün durdurulmuş bir yayını yok; yayını baştan başlatın.",
                "resume",
            )
        })?)
    } else {
        None
    };
    let (user, date) = match &journal {
        Some(j) => (j["user"].as_str().unwrap_or_default().to_owned(), j["date"].as_str().unwrap_or_default().to_owned()),
        None => (kentos_plot::book::current_user(), kentos_plot::book::today()),
    };
    let mut captured=kentos_plot::provenance::Capture::new(doc,&user,&date,input.drawing.as_deref()).map_err(|e|refuse(&e.code,e.message,"pin"))?;
    if input.pin.as_ref().is_some_and(|p|!captured.matches(p)){return Err(refuse("pin_mismatch","Çizim, bağlı resim sürümü veya başlık bilgileri ön denetimden sonra değişti; yeniden denetleyin.","pin"));}
    captured.dpi_override(input.dpi).map_err(|e|refuse(&e.code,e.message,"dpi"))?;
    let source=captured.pin().clone();
    if let Some(j) = &journal {
        let was: Option<SheetPublishPin> = serde_json::from_value(j["source"].clone()).ok();
        let same_source = was.as_ref().is_some_and(|w| {
            w.sha256 == source.sha256 && w.resources_sha256 == source.resources_sha256 && w.context_sha256 == source.context_sha256
        });
        if !same_source || j["options"] != journal_options(input) || j["sheets"] != serde_json::json!(chosen) {
            return Err(refuse(
                "resume_stale",
                "Çizim, bağlı kaynakları ya da yayın seçenekleri durdurulan yayından sonra değişti; bitmiş sayfalar yeni kaynakla karıştırılmaz. Yayını baştan başlatın.",
                "resume",
            ));
        }
    }
    let mut issues = Vec::new();
    let resource_plan = if input.resources {
        match crate::sheet_resources::Prepared::plan(doc, input, &book, &chosen) {
            Ok(p) => {
                issues.extend(p.issues.clone());
                Some(p)
            }
            Err(e) => {
                issues.push(issue(
                    "",
                    &e.code,
                    e.message,
                    true,
                    "Kaynak paketi sorununu düzeltip yeniden denetleyin.".into(),
                ));
                None
            }
        }
    } else {
        None
    };
    let resources = resource_plan.as_ref().map(|p| p.output.clone());
    let total_work = 2 * chosen.len() as u64 + resources.as_ref().map_or(0, |r| r.files.len() as u64);
    let mut staging = if resumable {
        let dir = journal_at.clone().ok_or_else(|| refuse("invalid_file_path", "Teslim klasörünün adı yok.", "path"))?;
        if journal.is_none() && dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| refuse("file_unwritable", format!("Eski devam klasörü silinemedi: {e}"), "path"))?;
        }
        if !dir.exists() {
            fs::create_dir(&dir).map_err(|e| refuse("file_unwritable", format!("Devam klasörü açılamadı: {e}"), "path"))?;
        }
        Some(Stage { path: dir, committed: false, keep: true })
    } else if matches!(op, CallOp::Execute) {
        Some(stage(&target)?)
    } else {
        None
    };
    let kept_files: Vec<SheetPublishedFile> = journal
        .as_ref()
        .and_then(|j| serde_json::from_value(j["files"].clone()).ok())
        .unwrap_or_default();
    let kept_pages: Vec<SheetPublishedPage> = journal
        .as_ref()
        .and_then(|j| serde_json::from_value(j["pages"].clone()).ok())
        .unwrap_or_default();
    let mut files = Vec::new();
    let mut pages: Vec<SheetPublishedPage> = Vec::new();
    let mut resumed = Vec::new();
    let mut carried: Vec<String> = Vec::new();
    let save = |files: &[SheetPublishedFile], pages: &[SheetPublishedPage], issues: &[SheetPublishIssue], reason: &str, at: Option<&Path>| {
        if let (true, Some(dir)) = (resumable, at) {
            let j = serde_json::json!({"format":JOURNAL_FORMAT,"path":input.path,"user":user,"date":date,"source":source,"options":journal_options(input),"sheets":chosen,"files":files,"pages":pages,"issues":issues,"reason":reason});
            let tmp = dir.join(format!("{JOURNAL}.tmp"));
            if let Ok(bytes) = serde_json::to_vec_pretty(&j)
                && crate::files::write_bytes(&bytes, &tmp).is_ok()
            {
                let _ = fs::rename(&tmp, dir.join(JOURNAL));
            }
        }
    };
    if let Some(p) = progress {
        p.set(0, total_work);
    }
    let digest_of = digests(&captured, &chosen, input.geo, input.layers, progress.map(|p| p.span(0, chosen.len() as u64, total_work)).as_ref())
        .inspect_err(|_| save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path())))?;
    // Pages are carried only into an output made the earlier delivery's way.
    let carrying = previous.as_ref().filter(|p| p.same_output(input, &source));
    let digest_for = |id: &str| digest_of.iter().find(|(s, _)| s == id).and_then(|(_, d)| d.as_ref().ok()).cloned().unwrap_or_default();
    let base = chosen.len() as u64;
    for (n, (name, sheets)) in planned.iter().enumerate() {
        if let Err(e) = stopped(progress) {
            save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path()));
            return Err(e);
        }
        // A finished page of the stopped publication is kept when its file still has its bytes.
        if let (Some(stage), Some(f)) = (&staging, kept_files.iter().find(|f| &f.name == name && &f.sheets == sheets && !f.sha256.is_empty())) {
            let on_disk = fs::read(stage.path.join(name)).ok().map(|b| sha256_hex(&b));
            if on_disk.as_deref() == Some(f.sha256.as_str()) {
                files.push(f.clone());
                pages.extend(kept_pages.iter().filter(|p| &p.file == name).cloned());
                resumed.extend(sheets.iter().cloned());
                continue;
            }
        }
        // An update carries a page that draws what the earlier delivery's did, its bytes and date unchanged.
        if let Some(c) = carrying.and_then(|p| match sheets.as_slice() {
            [one] => p.carry(name, one, &digest_for(one)),
            _ => None,
        }) {
            if let Some(stage) = &staging {
                crate::files::write_bytes(&c.bytes, &stage.path.join(name))
                    .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
            }
            files.push(c.file);
            pages.push(c.page);
            issues.extend(c.issues);
            carried.extend(sheets.iter().cloned());
            if let Some(p) = progress {
                p.set(base + (n + 1) as u64, total_work);
            }
            continue;
        }
        let opts = kentos_sheet::pdf::PdfOptions {
            sheets: sheets.clone(),
            geo: input.geo,
            layers: input.layers,
        };
        let page_progress = progress.map(|p| {
            p.span(
                base + if input.mode == SheetPublishMode::Single {
                    0
                } else {
                    n as u64
                },
                if input.mode == SheetPublishMode::Single {
                    chosen.len() as u64
                } else {
                    1
                },
                total_work,
            )
        });
        match kentos_plot::book::book_pdf_captured(&captured,sheets,Some(opts),page_progress.as_ref()) {
            Ok(pdf) => {
                for (owner, f) in pdf.sheet_findings {
                    issues.push(issue(
                        &owner,
                        &f.code,
                        f.message,
                        f.severity == kentos_sheet::preflight::Severity::Error
                            || f.code == "map_unplaced",
                        f.fix,
                    ));
                }
                for note in pdf.notes {
                    issues.push(issue(
                        &sheets.join(","),
                        "plot_rasterized",
                        note,
                        false,
                        "Çözünürlüğü ve raster kaynaklarını denetleyin.".into(),
                    ));
                }
                if let Err(e) = stopped(progress) {
                    save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path()));
                    return Err(e);
                }
                if let Some(stage) = &staging {
                    let path = stage.path.join(name);
                    if path.exists() && !resumable {
                        return Err(refuse(
                            "file_exists",
                            "Çıktı adları bu dosya sisteminde çakışıyor; ad şablonuna {index} ekleyin.",
                            "naming",
                        ));
                    }
                    crate::files::write_bytes(&pdf.bytes, &path)
                        .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
                }
                for a in &pdf.pages {
                    let own = scope.iter().find(|s| s.sheet == a.sheet);
                    let digest = digest_for(&a.sheet);
                    let mut page = page_of(a, name, &digest, own);
                    page.date = Some(date.clone());
                    pages.push(page);
                    if let Some(note) = previous.as_ref().and_then(|p| p.revision_note(&a.sheet, &digest, own)) {
                        issues.push(note);
                    }
                }
                files.push(SheetPublishedFile {
                    name: name.clone(),
                    sheets: sheets.clone(),
                    bytes: pdf.bytes.len() as u64,
                    sha256: sha256_hex(&pdf.bytes),
                });
                save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path()));
            }
            Err(e) => {
                let before = issues.len();
                if e.code == "cancelled" {
                    save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path()));
                    return Err(refuse("cancelled", e.message, "path"));
                }
                // A single combined failure is diagnosed page by page so the whole missing-source report is useful.
                if sheets.len() > 1 {
                    for id in sheets {
                        stopped(progress)?;
                        if let Err(page)=kentos_plot::book::book_pdf_captured(&captured,std::slice::from_ref(id),None,progress) {
                            issues.push(issue(
                                id,
                                &page.code,
                                page.message,
                                true,
                                "Paftanın kaynaklarını düzeltip yeniden denetleyin.".into(),
                            ));
                        }
                    }
                }
                if !issues[before..].iter().any(|i| i.error) {
                    issues.push(issue(
                        &sheets.join(","),
                        &e.code,
                        e.message,
                        true,
                        "Kaynakları ve ön denetimi düzeltin.".into(),
                    ));
                }
                files.push(SheetPublishedFile {
                    name: name.clone(),
                    sheets: sheets.clone(),
                    bytes: 0,
                    sha256: String::new(),
                });
            }
        }
        if let Some(p) = progress {
            p.set(
                base + if input.mode == SheetPublishMode::Single {
                    chosen.len() as u64
                } else {
                    (n + 1) as u64
                },
                total_work,
            );
        }
    }
    if let Err(e) = stopped(progress) {
        save(&files, &pages, &issues, "cancelled", staging.as_ref().map(|s| s.path.as_path()));
        return Err(e);
    }
    let ready = !issues.iter().any(|i| i.error);
    let bytes=serde_json::to_vec_pretty(&serde_json::json!({"format":"kentos.publish/1","snapshot":captured.metadata().map_err(|e|refuse(&e.code,e.message,"pin"))?,"drawingSet":chosen.iter().filter_map(|id| book.sheet(id).map(|s|serde_json::json!({"id":s.id,"name":s.name,"info":s.drawing_info}))).collect::<Vec<_>>(),"source":source,"date":date,"user":user,"options":{"mode":input.mode,"naming":input.naming,"dpi":input.dpi,"geo":input.geo,"layers":input.layers},"sheets":chosen,"files":files,"pages":pages,"scope":scope,"resources":resources,"issues":issues,"previous":previous.as_ref().map(|p|&p.report),"carried":carried,"sources":if matches!(op,CallOp::Execute){crate::publish_update::source_record(doc,&kept_book,&chosen)}else{serde_json::Value::Null}})).map_err(|e|refuse("file_unwritable",e.to_string(),"path"))?;
    let report = SheetsPublished {
        resources,
        path: input.path.clone(),
        sheets: chosen.clone(),
        files,
        issues,
        source: source.clone(),
        ready,
        manifest: target.join(MANIFEST).display().to_string(),
        sha256: sha256_hex(&bytes),
        scope: scope.clone(),
        pages,
        resumed,
        previous: previous.map(|p| p.report),
        carried,
    };
    if let Some(stage) = &mut staging {
        if !ready {
            let kept = report.files.iter().filter(|f| !f.sha256.is_empty()).count();
            save(&report.files.iter().filter(|f| !f.sha256.is_empty()).cloned().collect::<Vec<_>>(), &report.pages, &report.issues, "failed", Some(stage.path.as_path()));
            let mut message = report
                .issues
                .iter()
                .filter(|i| i.error)
                .map(|i| format!("{}: {} {}", i.sheet, i.message, i.fix))
                .collect::<Vec<_>>()
                .join(" ");
            if resumable {
                message.push_str(&format!(
                    " Teslim klasörü oluşturulmadı; bitmiş {kept} sayfa devam için saklandı (resume)."
                ));
            }
            return Err(refuse("publish_blocked", message, "sheets"));
        }
        if let Some(resources) = &resource_plan {
            let phase = progress.map(|p| {
                p.span(
                    2 * report.sheets.len() as u64,
                    resources.output.files.len() as u64,
                    total_work,
                )
            });
            resources.write(&stage.path, phase.as_ref())?;
        }
        if let Err(e) = stopped(progress) {
            save(&report.files, &report.pages, &report.issues, "cancelled", Some(stage.path.as_path()));
            return Err(e);
        }
        let _ = fs::remove_file(stage.path.join(JOURNAL));
        crate::files::write_bytes(&bytes, &stage.path.join(MANIFEST))
            .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
        stopped(progress)?;
        crate::package::target(&input.path)?; // A concurrent writer must not lose its published folder.
        if target.exists() {
            fs::remove_dir(&target)
                .map_err(|e| refuse("file_unwritable", e.to_string(), "path"))?;
        }
        fs::rename(&stage.path, &target)
            .map_err(|e| refuse("file_unwritable", e.to_string(), "path"))?;
        stage.committed = true;
        if let Some(p) = progress {
            p.set(total_work, total_work);
        }
        return Ok((
            report.clone(),
            Some(FileEffect {
                path: report.manifest,
                bytes: bytes.len() as u64,
                sha256: report.sha256,
            }),
        ));
    }
    Ok((report, None))
}
pub(crate) fn run(
    doc: &Document,
    input: &PublishSheets,
    op: CallOp,
    progress: Option<&Progress>,
) -> (CommandResult<serde_json::Value>, Option<FileEffect>) {
    match work(doc, input, op, progress) {
        Ok((report, effect)) => (
            CommandResult::Completed {
                output: if matches!(op, CallOp::Validate) {
                    serde_json::Value::Null
                } else {
                    serde_json::to_value(report).unwrap_or_default()
                },
                warnings: Vec::new(),
            },
            effect,
        ),
        Err(e) if e.code == "cancelled" => (CommandResult::Cancelled, None),
        Err(e) => (crate::file_commands::refused_result(*e), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_publication_never_creates_a_target_or_temporary_delivery() {
        let fx: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/commands/v1/file.publish.sheets.json"
        ))
        .unwrap();
        let mut doc =
            Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).unwrap()).unwrap();
        let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
        doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let root =
            std::env::temp_dir().join(format!("kentos-publish-cancel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let path = root.join("delivery");
        let input: PublishSheets =
            serde_json::from_value(serde_json::json!({"path":path})).unwrap();
        let progress = Progress::default();
        progress.stop();
        let (result, effect) = run(&doc, &input, CallOp::Execute, Some(&progress));
        assert!(matches!(result, CommandResult::Cancelled));
        assert!(effect.is_none());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir(root).unwrap();
    }
}
