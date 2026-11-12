//! DWG through OpenCADStudio's pinned OpenCADCodec (ADR 0586).
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

pub const PROVIDER: &str = "OpenCADStudio / acadrust";
pub const REVISION: &str = "93449222904e9df1ba14e275b6b764df93227b33";
/// An explicit input limit, not an estimate of decompressed memory.
pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
/// The in-memory DXF bridge's limit. A DWG writes several times its size as DXF text: a 32 MB architectural DWG of
/// 725 000 objects (12 November 2026) wrote 270 MB, which the file limit refused; its open then took 1.4 GB.
pub const MAX_BRIDGE_BYTES: usize = 1024 * 1024 * 1024;

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
    if let Some(v) = DwgVersion::ALL
        .into_iter()
        .find(|v| Some(v.signature()) == signature)
    {
        return Ok(v);
    }
    Err(error(
        "unsupported_cad_version",
        match signature.filter(|s| s.starts_with("AC1")) {
            Some(s) => format!(
                "Bu DWG'nin sürümü ({s}) desteklenmiyor. Desteklenenler R14–2018 biçimleridir (AC1014–AC1032); dosyayı bunlardan biri olarak kaydedin."
            ),
            None => not_a_dwg(bytes),
        },
    ))
}

/// What a file named .dwg is when its start is not a DWG's signature: one of the downloads was a JPEG photograph
/// (12 November 2026), and "the signature is not supported" said nothing of that.
fn not_a_dwg(bytes: &[u8]) -> String {
    let what = [
        (&b"\xFF\xD8\xFF"[..], "bir JPEG resmi"),
        (&b"\x89PNG"[..], "bir PNG resmi"),
        (&b"GIF8"[..], "bir GIF resmi"),
        (&b"%PDF"[..], "bir PDF belgesi"),
        (&b"PK\x03\x04"[..], "bir ZIP arşivi"),
        (&b"Rar!"[..], "bir RAR arşivi"),
        (&b"7z\xBC\xAF"[..], "bir 7z arşivi"),
        (&b"AutoCAD Binary DXF"[..], "bir DXF çizimi"),
    ]
    .into_iter()
    .find(|(magic, _)| bytes.starts_with(magic))
    .map(|(_, what)| what)
    .or_else(|| {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64)]).into_owned();
        let mut lines = head.lines().map(str::trim);
        (lines.next() == Some("0") && lines.next() == Some("SECTION")).then_some("bir DXF çizimi")
    });
    match what {
        Some(what) => format!("Bu dosya bir DWG çizimi değil: adı .dwg ile bitse de içinde {what} var."),
        None => "Bu dosya bir DWG çizimi değil: başında DWG sürüm imzası (AC10…) yok. Dosya bozulmuş ya da başka bir biçimde olabilir.".into(),
    }
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

/// A DWG the codec read to its end opens; the records it could not decode are said in the report, not a refusal.
/// Real drawings have them: 8 579 DGN line style records in one, one in another, none of them drawn (12 November
/// 2026); refusing them kept every such drawing closed. A stream not read to its end is refused: what came out of it
/// is part of a drawing.
fn complete(stats: &acadrust::ReadStats) -> Result<(), Error> {
    if !stats.stream_completed {
        return Err(error(
            "import_unreadable",
            "DWG sonuna kadar okunamadı; yarım bir çizim açılmaz. Dosyayı kaynak yazılımda denetleyip yeniden kaydedin.",
        ));
    }
    Ok(())
}

/// A diagnostic's record kind by its name: the codec gives a class's number as its type ("551") and names it only in
/// the words ("Preserved auxiliary object LSSTROKEPATTERNCOMPONENT at handle 0x656DF8, type_code=551 …").
fn record_kind(d: &acadrust::ReadDiagnostic) -> Option<&str> {
    d.message
        .split_once("auxiliary object ")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .or(d.record_type.as_deref())
}

/// The records the codec could not decode, said by their kinds as its diagnostics name them (a DGN line style's
/// component is not drawn; another kind may be a missing object).
fn recovered_said(stats: &acadrust::ReadStats) -> String {
    let mut kinds = std::collections::BTreeMap::<&str, usize>::new();
    for d in &stats.diagnostics {
        if let Some(kind) = record_kind(d) {
            *kinds.entry(kind).or_default() += 1;
        }
    }
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|k| std::cmp::Reverse(k.1));
    let named = kinds
        .iter()
        .take(4)
        .map(|(kind, n)| {
            let drawn = if DGN_LINE_STYLES.contains(kind) {
                ", DGN çizgi stili, çizilmez"
            } else {
                ""
            };
            format!("{kind} {n}{drawn}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    let named = if named.is_empty() {
        String::new()
    } else {
        format!(" Türleri (ilk {} tanı): {named}.", stats.diagnostics.len())
    };
    format!("Kodek bu kayıtları çözemedi; çizim onlarsız açıldı.{named}")
}

/// What KentOS writes must read back whole: the writer's own DXF and the DWG made of it, no record recovered.
fn strict(stats: &acadrust::ReadStats) -> Result<(), Error> {
    if !stats.stream_completed || stats.recovered_errors > 0 {
        return Err(error(
            "file_unwritable",
            format!(
                "Yazılan veri tam okunamadı ({} kurtarılan hata); eksik dosya yazılmadı.",
                stats.recovered_errors
            ),
        ));
    }
    Ok(())
}

/// A DGN line style's records (DgnLS; a DWG made from MicroStation drawings holds them).
const DGN_LINE_STYLES: [&str; 6] = [
    "LSDEFINITION",
    "LSSYMBOLCOMPONENT",
    "LSCOMPOUNDCOMPONENT",
    "LSSTROKEPATTERNCOMPONENT",
    "LSPOINTCOMPONENT",
    "LSINTERNALCOMPONENT",
];

/// Objects KentOS's DXF reading never takes, out of the temporary document before it is written as DXF text (the
/// source DWG stays as it is): DGN line style records, decoded or kept raw after a decode failure, and proxy objects.
/// In one drawing (12 November 2026) they were 380 of the bridge's 463 MB and most of its 57 s. Their dictionaries'
/// entries go with them. Answers how many went.
fn drop_unread(doc: &mut acadrust::CadDocument) -> usize {
    use acadrust::objects::ObjectType;
    let gone: std::collections::HashSet<acadrust::Handle> = doc
        .objects
        .iter()
        .filter(|(_, o)| match o {
            ObjectType::DgnLineStyle(_) | ObjectType::ProxyObject(_) => true,
            ObjectType::Unknown { type_name, .. } => DGN_LINE_STYLES.contains(&type_name.as_str()),
            _ => false,
        })
        .map(|(h, _)| *h)
        .collect();
    if gone.is_empty() {
        return 0;
    }
    doc.objects.retain(|h, _| !gone.contains(h));
    for o in doc.objects.values_mut() {
        if let ObjectType::Dictionary(d) = o {
            d.entries.retain(|(_, h)| !gone.contains(h));
            let kept: std::collections::HashSet<&String> =
                d.entries.iter().map(|(name, _)| name).collect();
            d.hard_owner_entries.retain(|name| kept.contains(name));
        }
    }
    gone.len()
}

/// Whether a DWG registers KENTOS in its APPID table, KentOS's own export (docs/adr/1061 §5): decoded through the same
/// codec to DXF text and looked at there. A file the codec cannot read is not KentOS's.
pub fn written_by_kentos(bytes: &[u8]) -> bool {
    if version(bytes).is_err() {
        return false;
    }
    let dxf = guarded(|| {
        let outcome = DwgReader::from_stream(Cursor::new(bytes))
            .read_with_stats()
            .map_err(|e| error("import_unreadable", format!("DWG okunamadı: {e}")))?;
        DxfWriter::new(&outcome.document)
            .with_version(DxfVersion::AC1032)
            .write_to_vec()
            .map_err(|e| error("import_unreadable", format!("DWG aktarma verisi üretilemedi: {e}")))
    });
    let Ok(dxf) = dxf else { return false };
    let text = String::from_utf8_lossy(&dxf);
    let mut lines = text.lines().map(str::trim);
    while let Some(code) = lines.next() {
        let Some(value) = lines.next() else { break };
        if code == "2" && value == kentos_formats::dxf::xdata::APP {
            return true;
        }
    }
    false
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
    let bridge = bridge(bytes, watch)?;
    read_bridge(&bridge, options, watch)
}

/// A DWG decoded once and written as the DXF text the reader takes: an open reads it twice (its unit, then at its
/// scale), and decoding a 32 MB DWG the second time cost 9 of its 24 s (12 November 2026).
pub struct Bridge {
    dxf: Vec<u8>,
    stats: acadrust::ReadStats,
    dropped: usize,
    source: DwgVersion,
}

/// [`read_watched`]'s first half: the DWG decoded, the objects KentOS does not read dropped and the rest written as
/// DXF text (the watch's first 40 %).
pub fn bridge(bytes: &[u8], watch: &mut dyn Watch) -> Result<Bridge, Error> {
    let source = version(bytes)?;
    if !watch.step(0, 1000) {
        return Err(error("cancelled", STOPPED));
    }
    let (dxf, stats, dropped) = guarded(|| {
        let mut outcome = DwgReader::from_stream(Cursor::new(bytes))
            .read_with_stats()
            .map_err(|e| error("import_unreadable", format!("DWG okunamadı: {e}")))?;
        complete(&outcome.stats)?;
        let dropped = drop_unread(&mut outcome.document);
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
        if dxf.len() > MAX_BRIDGE_BYTES {
            return Err(error(
                "cad_file_too_large",
                format!(
                    "DWG'nin aktarma verisi {} MiB sınırını aşıyor; çizimi kaynak yazılımda bölerek kaydedin.",
                    MAX_BRIDGE_BYTES / (1024 * 1024)
                ),
            ));
        }
        Ok((dxf, outcome.stats, dropped))
    })?;
    if !watch.step(400, 1000) {
        return Err(error("cancelled", STOPPED));
    }
    Ok(Bridge {
        dxf,
        stats,
        dropped,
        source,
    })
}

/// [`read_watched`]'s second half: the bridge read with `options` (the watch's last 60 %), what the DWG lost said.
pub fn read_bridge(
    bridge: &Bridge,
    options: &DxfReadOptions,
    watch: &mut dyn Watch,
) -> Result<ImportResult, Error> {
    let Bridge {
        dxf,
        stats,
        dropped,
        source,
    } = bridge;
    let (dropped, source) = (*dropped, *source);
    let mut phase = kentos_formats::watch::Steps(|done: u64, total: u64| {
        watch.step(400 + done.saturating_mul(600) / total.max(1), 1000)
    });
    let mut result = kentos_formats::dxf::read_checked_watched_within(
        dxf,
        options,
        &mut phase,
        MAX_BRIDGE_BYTES,
    )
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
    if stats.recovered_errors > 0 {
        result.report.skipped.push(kentos_contracts::ReportItem {
            severity: None,
            details: Vec::new(),
            what: "DWG kaydı".into(),
            count: stats.recovered_errors.min(u32::MAX as usize) as u32,
            reason: recovered_said(stats),
            lines: Vec::new(),
        });
    }
    if dropped > 0 {
        result.report.notes.push(kentos_contracts::ReportItem {
            severity: None,
            details: Vec::new(),
            what: "DWG nesnesi".into(),
            count: dropped.min(u32::MAX as usize) as u32,
            reason: "DGN çizgi stili bileşenleri ve vekil (proxy) nesneler okunmadı: KentOS onları çizmez; çizgiler çizgi tipinin kesik deseniyle çizilir.".into(),
            lines: Vec::new(),
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
    // The codec's own words for what it kept or left, a few: a DGN line style's are counted above, and a hundred lines
    // of them filled Geçmiş on opening a drawing (12 November 2026).
    for d in stats
        .diagnostics
        .iter()
        .filter(|d| !record_kind(d).is_some_and(|t| DGN_LINE_STYLES.contains(&t)))
        .take(10)
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use acadrust::Handle;
    use acadrust::objects::{Dictionary, ObjectType};

    /// A DWG read to its end opens with the records the codec recovered; only one not read to its end is refused.
    #[test]
    fn recovered_records_open_an_unfinished_stream_does_not() {
        let read = |stream_completed, recovered_errors| {
            complete(&acadrust::ReadStats {
                stream_completed,
                recovered_errors,
                ..Default::default()
            })
        };
        assert!(read(true, 0).is_ok());
        assert!(
            read(true, 8579).is_ok(),
            "8 579 DGN line style records kept a real drawing closed"
        );
        let refused = read(false, 0).expect_err("a stream not read to its end");
        assert_eq!(refused.code, "import_unreadable");
        assert!(
            refused.message.contains("sonuna kadar okunamadı"),
            "{}",
            refused.message
        );
        // What KentOS writes must still read back whole.
        assert!(
            strict(&acadrust::ReadStats {
                stream_completed: true,
                recovered_errors: 1,
                ..Default::default()
            })
            .is_err()
        );
    }

    /// DGN line style records (decoded or raw) and proxy objects leave the temporary document with their dictionary
    /// entries; a layout and its entry stay.
    #[test]
    fn unread_objects_leave_the_bridge_with_their_entries() {
        let mut doc = acadrust::CadDocument::new();
        let (style, layout, table) = (
            Handle::new(0x9001),
            Handle::new(0x9002),
            Handle::new(0x9003),
        );
        doc.objects.insert(
            style,
            ObjectType::Unknown {
                type_name: "LSSTROKEPATTERNCOMPONENT".into(),
                handle: style,
                owner: table,
                raw_dxf_codes: None,
                raw_dwg_data: Some(vec![0; 8]),
                raw_dwg_handle_bits: 0,
                raw_dwg_version: None,
            },
        );
        let mut dictionary = Dictionary::new();
        dictionary.handle = table;
        dictionary.entries = vec![("Stroke 1".into(), style), ("Pafta".into(), layout)];
        dictionary.hard_owner_entries = vec!["Stroke 1".into(), "Pafta".into()];
        doc.objects
            .insert(table, ObjectType::Dictionary(dictionary));
        let before = doc.objects.len();
        assert_eq!(drop_unread(&mut doc), 1);
        assert_eq!(doc.objects.len(), before - 1);
        let Some(ObjectType::Dictionary(d)) = doc.objects.get(&table) else {
            panic!("the dictionary stays")
        };
        assert_eq!(d.entries, vec![("Pafta".to_owned(), layout)]);
        assert_eq!(d.hard_owner_entries, vec!["Pafta".to_owned()]);
        assert_eq!(drop_unread(&mut doc), 0, "nothing more to drop");
    }

    /// The recovered records are said by their kinds; a DGN line style's is said not to be drawn.
    #[test]
    fn recovered_records_are_said_by_their_kinds() {
        // As the codec says them: the class's number as the type, its name in the words; or a kind it names.
        let diagnostic = |kind: &str| acadrust::ReadDiagnostic {
            code: "auxiliary-object-preserved".into(),
            stage: acadrust::ReadStage::RecordStream,
            section: None,
            source_offset: None,
            source_offset_basis: None,
            source_line: None,
            record_handle: None,
            record_type: Some(if kind == "HATCH" {
                kind.into()
            } else {
                "551".into()
            }),
            message: if kind == "HATCH" {
                "Parse error".into()
            } else {
                format!(
                    "Preserved auxiliary object {kind} at handle 0x656DF8, type_code=551 as raw DWG data"
                )
            },
        };
        let stats = acadrust::ReadStats {
            stream_completed: true,
            recovered_errors: 3,
            diagnostics: vec![
                diagnostic("LSSTROKEPATTERNCOMPONENT"),
                diagnostic("LSSTROKEPATTERNCOMPONENT"),
                diagnostic("HATCH"),
            ],
            ..Default::default()
        };
        let said = recovered_said(&stats);
        assert!(
            said.starts_with("Kodek bu kayıtları çözemedi; çizim onlarsız açıldı."),
            "{said}"
        );
        assert!(
            said.contains("LSSTROKEPATTERNCOMPONENT 2, DGN çizgi stili, çizilmez; HATCH 1"),
            "{said}"
        );
    }
}
