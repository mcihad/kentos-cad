//! DWG through OpenCADStudio's pinned OpenCADCodec (ADR 0580).
//!
//! Only bytes and KentOS contracts cross this boundary. A loss-reporting,
//! in-memory DXF bridge reuses the unit/block/style rules of the existing
//! importer; no application shell, SDK object, file or network is exposed.
#![forbid(unsafe_code)]

use std::fmt;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use acadrust::{DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter};
use kentos_contracts::{DwgVersion, DxfReadOptions, ImportResult};
use kentos_formats::watch::{Quiet, STOPPED, Watch};

pub const PROVIDER: &str = "OpenCADStudio / OpenCADCodec";
pub const REVISION: &str = "ff75fddc322bda4cf817c5f68535c5c2b316b9c4";
/// An explicit input/bridge limit, not an estimate of decompressed memory.
pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}

fn bounded(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(error(
            "cad_file_too_large",
            "CAD dosyası 256 MiB okuma sınırını aşıyor; dosyayı bölerek içe aktarın.",
        ));
    }
    Ok(())
}

/// Identify the file's binary signature, independently of any filename/product year.
pub fn version(bytes: &[u8]) -> Result<DwgVersion, Error> {
    bounded(bytes)?;
    let signature = bytes.get(..6).and_then(|b| std::str::from_utf8(b).ok());
    DwgVersion::ALL.into_iter().find(|v| Some(v.signature()) == signature).ok_or_else(|| {
        error("unsupported_cad_version", format!(
            "DWG imzası {} desteklenmiyor. Desteklenenler R14–2018 biçimleridir (AC1014–AC1032); dosyayı bunlardan biri olarak kaydedin.",
            signature.unwrap_or("okunamadı")
        ))
    })
}

fn codec_version(version: DwgVersion) -> DxfVersion {
    match version {
        DwgVersion::AC1014 => DxfVersion::AC1014,
        DwgVersion::AC1015 => DxfVersion::AC1015,
        DwgVersion::AC1018 => DxfVersion::AC1018,
        DwgVersion::AC1021 => DxfVersion::AC1021,
        DwgVersion::AC1024 => DxfVersion::AC1024,
        DwgVersion::AC1027 => DxfVersion::AC1027,
        DwgVersion::AC1032 => DxfVersion::AC1032,
    }
}

/// Protect the native host from a dependency's recoverable parser panic. This
/// does not promise recovery from allocation failure or wasm panic=abort.
fn guarded<T>(f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| error(
        "import_unreadable", "CAD kodeği dosyayı işlerken durdu; çizim değiştirilmedi. Dosyayı kaynak yazılımda denetleyip yeniden kaydedin."
    ))?
}

fn strict(stats: &acadrust::ReadStats) -> Result<(), Error> {
    if !stats.stream_completed || stats.recovered_errors > 0 {
        return Err(error(
            "import_unreadable",
            format!(
                "CAD kayıtları tam okunamadı ({} kurtarılan hata). Kısmi dosya çizime alınmadı; kaynak yazılımda denetleyin.",
                stats.recovered_errors
            ),
        ));
    }
    Ok(())
}

pub fn read(bytes: &[u8], options: &DxfReadOptions) -> Result<ImportResult, Error> {
    read_watched(bytes, options, &mut Quiet)
}

/// Cancellation is polled at codec stage boundaries and throughout the KentOS
/// DXF phase. The upstream decompressor currently has no cancellation hook.
pub fn read_watched(
    bytes: &[u8],
    options: &DxfReadOptions,
    watch: &mut dyn Watch,
) -> Result<ImportResult, Error> {
    let source = version(bytes)?;
    if !watch.step(0, 1000) {
        return Err(error("cancelled", STOPPED));
    }
    let (dxf, stats) = guarded(|| {
        let mut outcome = DwgReader::from_stream(Cursor::new(bytes))
            .read_with_stats()
            .map_err(|e| error("import_unreadable", format!("DWG okunamadı: {e}")))?;
        strict(&outcome.stats)?;
        // R14 has no per-entity lineweight field. The upstream legacy
        // decoder's zero is a missing field, not a 0 mm object override.
        if source == DwgVersion::AC1014 {
            for entity in outcome.document.entities_mut() {
                entity.common_mut().line_weight = acadrust::LineWeight::ByLayer;
            }
        }
        // OpenCADCodec's DXF bridge writes 40/41=0 even when a DWG uses
        // constant width 43. In DXF those explicit zeros override 43. Expand
        // the decoded constant to vertex widths in this temporary document;
        // all blocks/paper entities are included, source DWG bytes stay intact.
        for entity in outcome.document.entities_mut() {
            if let acadrust::entities::EntityType::LwPolyline(p) = entity
                && p.constant_width != 0.0
                && p.vertices.iter().all(|v| v.start_width == 0.0 && v.end_width == 0.0)
            {
                for vertex in &mut p.vertices {
                    vertex.start_width = p.constant_width;
                    vertex.end_width = p.constant_width;
                }
            }
        }
        let dxf = DxfWriter::new(&outcome.document)
            .with_version(DxfVersion::AC1032)
            .write_to_vec()
            .map_err(|e| {
                error(
                    "import_unreadable",
                    format!("DWG aktarma verisi üretilemedi: {e}"),
                )
            })?;
        bounded(&dxf)?;
        Ok((dxf, outcome.stats))
    })?;
    if !watch.step(400, 1000) {
        return Err(error("cancelled", STOPPED));
    }
    let mut phase = kentos_formats::watch::Steps(|done: u64, total: u64| {
        watch.step(400 + done.saturating_mul(600) / total.max(1), 1000)
    });
    let mut result = kentos_formats::dxf::read_checked_watched(&dxf, options, &mut phase)
        .map_err(|e| error(e.code, e.message))?;
    result.report.source.retain(|f| f.label != "Sürüm");
    for (label, value) in [
        ("Kaynak biçimi", "DWG".to_owned()),
        ("Kaynak DWG sürümü", source.label().to_owned()),
        (
            "Aktarma biçimi",
            "Bellekte ASCII DXF AC1032; kaynak DWG değiştirilmedi".to_owned(),
        ),
        ("Kodek", format!("{PROVIDER} @ {REVISION}")),
        (
            "Satır başvuruları",
            "Bellekteki DXF satırları; kaynak DWG bayt ofseti değildir".to_owned(),
        ),
    ] {
        result.report.source.push(kentos_contracts::SourceFact {
            label: label.into(),
            value,
        });
    }
    if stats.skipped_source_records > 0 {
        result.report.skipped.push(kentos_contracts::ReportItem {
            severity: None,
            details: Vec::new(),
            what: "DWG kaydı".into(),
            count: stats.skipped_source_records.min(u32::MAX as usize) as u32,
            reason: "Kodek bu kayıtları çözmedi; raporlanan temel 2D kapsamı dışında olabilir."
                .into(),
            lines: Vec::new(),
        });
    }
    for d in stats.diagnostics.iter().take(100) {
        result.report.notes.push(kentos_contracts::ReportItem {
            severity: None,
            details: Vec::new(),
            what: d.record_type.clone().unwrap_or_else(|| "DWG".into()),
            count: 1,
            reason: format!(
                "{} [{}]{}: {}",
                d.code,
                d.stage,
                d.record_handle
                    .map(|h| format!(" handle {h:X}"))
                    .unwrap_or_default(),
                d.message
            ),
            lines: Vec::new(),
        });
    }
    Ok(result)
}

/// Convert the KentOS writer's DXF bytes to a selected DWG version. The
/// caller owns atomic file replacement and combines both stages' loss reports.
pub fn encode(dxf: &[u8], target: DwgVersion) -> Result<Vec<u8>, Error> {
    bounded(dxf)?;
    guarded(|| {
        let outcome = DxfReader::from_reader(Cursor::new(dxf.to_vec()))
            .and_then(DxfReader::read_with_stats)
            .map_err(|e| {
                error(
                    "file_unwritable",
                    format!("DWG için DXF aktarması okunamadı: {e}"),
                )
            })?;
        strict(&outcome.stats)?;
        if outcome.stats.skipped_source_records > 0 {
            return Err(error(
                "file_unwritable",
                "DWG kodeği DXF kayıtlarını atladı; eksik dosya yazılmadı.",
            ));
        }
        let mut document = outcome.document;
        document.version = codec_version(target);
        let expected_entities = document.entities().count();
        let expected_blocks = document.block_records.len();
        let bytes = DwgWriter::write_to_vec(&document)
            .map_err(|e| error("file_unwritable", format!("DWG yazılamadı: {e}")))?;
        bounded(&bytes)?;
        let check = DwgReader::from_stream(Cursor::new(&bytes))
            .read_with_stats()
            .map_err(|e| {
                error(
                    "file_unwritable",
                    format!("Yazılan DWG yeniden okunamadı: {e}"),
                )
            })?;
        strict(&check.stats)?;
        if version(&bytes)? != target
            || check.stats.skipped_source_records > 0
            || check.document.entities().count() != expected_entities
            || check.document.block_records.len() != expected_blocks
        {
            return Err(error(
                "file_unwritable",
                "Yazılan DWG'nin sürümü ya da kayıtları doğrulanamadı; hedef dosya korunuyor.",
            ));
        }
        Ok(bytes)
    })
}
