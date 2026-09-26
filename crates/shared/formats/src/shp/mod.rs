//! ESRI Shapefile in (docs/adr/0046): one layer from its files, chosen
//! together: the shapes (.shp), their index (.shx, only checked: the
//! records are read in order), the attribute table (.dbf), the coordinate
//! system (.prj) and the table's code page (.cpg). Points keep their
//! height; lines, paths and areas with holes are plane; M values are
//! dropped; each record's attributes go on every object it makes. What is
//! dropped or converted is counted in the report. A zipped Shapefile is
//! read the same way from inside the archive ([`zip_layers`], [`read_zip`];
//! docs/adr/0053): each .shp in it is a layer, its parts found by name.

pub mod dbf;
pub mod prj;
pub mod shape;

use std::collections::BTreeMap;

use kentos_contracts::{CrsSource, DeclaredCrs, ImportResult, ShapefileReadOptions};

use crate::gis::{Collect, Shape};
use dbf::EncodingSource;
use shape::{Found, Records};

/// The Shapefile layers of a zip archive: the paths of its .shp files
/// without extension (`katmanlar/yollar`), in the archive's order. A
/// layer's parts are the files beside its .shp with its name.
pub fn zip_layers(bytes: &[u8]) -> Result<Vec<String>, String> {
    let listed = crate::zip::list(bytes, &crate::zip::Limits::default())?;
    let layers: Vec<String> = listed
        .iter()
        .filter_map(|l| stem_of(&l.name, "shp").map(str::to_owned))
        .collect();
    if layers.is_empty() {
        return Err(
            "Zip arşivinde Shapefile (.shp) yok. Arşivde .shp, .shx ve .dbf dosyaları olmalı."
                .to_owned(),
        );
    }
    Ok(layers)
}

/// A zip layer's name: its path's last part (`katmanlar/yollar` → `yollar`).
pub fn zip_layer_name(layer: &str) -> &str {
    layer.rsplit('/').next().unwrap_or(layer)
}

/// Reads the layer `layer` of a zipped Shapefile (a path of [`zip_layers`]):
/// its .shp, .shx, .dbf, .prj and .cpg, found beside each other by name
/// whatever their case, unpacked and read as [`read`] reads them. The report
/// says the parts came from the archive.
pub fn read_zip(
    bytes: &[u8],
    layer: &str,
    opts: &ShapefileReadOptions,
) -> Result<ImportResult, String> {
    const PARTS: [&str; 5] = ["shp", "shx", "dbf", "prj", "cpg"];
    let limits = crate::zip::Limits::default();
    let listed = crate::zip::list(bytes, &limits)?;
    let is =
        |name: &str, ext: &str| stem_of(name, ext).is_some_and(|s| s.eq_ignore_ascii_case(layer));
    let entries = crate::zip::unpack(bytes, &listed, &limits, |l| {
        PARTS.iter().any(|ext| is(&l.name, ext))
    })?;
    let find = |ext: &str| {
        entries
            .iter()
            .find(|e| is(&e.name, ext))
            .map(|e| e.data.as_slice())
    };
    let shp = find("shp").ok_or_else(|| format!("Zip arşivinde “{layer}.shp” yok."))?;
    let files = Files {
        shp,
        shx: find("shx"),
        dbf: find("dbf"),
        prj: find("prj"),
        cpg: find("cpg"),
    };
    let mut result = read(&files, opts)?;
    let found: Vec<String> = PARTS
        .into_iter()
        .filter(|ext| find(ext).is_some())
        .map(|ext| format!(".{ext}"))
        .collect();
    result.report.source.insert(
        0,
        kentos_contracts::SourceFact {
            label: "Zip arşivinden".to_owned(),
            value: found.join(", "),
        },
    );
    Ok(result)
}

/// `name` without the extension `ext` (case aside), when it has it.
fn stem_of<'a>(name: &'a str, ext: &str) -> Option<&'a str> {
    let (stem, e) = name.rsplit_once('.')?;
    (e.eq_ignore_ascii_case(ext) && !stem.is_empty() && !stem.ends_with('/')).then_some(stem)
}

/// The files of one Shapefile layer.
#[derive(Clone, Copy, Default)]
pub struct Files<'a> {
    pub shp: &'a [u8],
    pub shx: Option<&'a [u8]>,
    pub dbf: Option<&'a [u8]>,
    pub prj: Option<&'a [u8]>,
    pub cpg: Option<&'a [u8]>,
}

/// Reads a Shapefile layer. A .shp that is not a Shapefile is refused with
/// the reason; a broken .dbf is reported and the shapes are read without it.
pub fn read(files: &Files, opts: &ShapefileReadOptions) -> Result<ImportResult, String> {
    let head = shape::header(files.shp, ".shp")?;
    let layer = if opts.layer.is_empty() {
        "Shapefile".to_string()
    } else {
        opts.layer.clone()
    };
    let mut c = Collect::new(opts.max_entities);

    // The attribute table and its code page.
    let driver = files.dbf.and_then(|d| d.get(29)).copied().unwrap_or(0);
    let (enc, source) = dbf::encoding(files.cpg, driver);
    let table = match files.dbf {
        Some(d) => match dbf::Table::open(d, enc) {
            Ok(t) => Some(t),
            Err(e) => {
                c.report.skip(
                    "Öznitelik tablosu (.dbf)",
                    &format!("{e} Nesneler özniteliksiz alındı"),
                    0,
                );
                None
            }
        },
        None => {
            c.report.note(
                "Öznitelik tablosu (.dbf)",
                "seçilmedi; nesneler özniteliksiz alındı (katmanın .dbf dosyasını da seçin)",
                0,
            );
            None
        }
    };

    let mut found = Found::default();
    let mut records = Records::new(files.shp);
    let mut n = 0usize;
    let mut deleted = 0u32;
    let none = BTreeMap::new();
    let mut shapes: Vec<Shape> = Vec::new();
    for content in records.by_ref() {
        let row = table.as_ref().and_then(|t| t.record(n));
        n += 1;
        if row.as_ref().is_some_and(|r| r.0) {
            deleted += 1;
            continue;
        }
        shapes.clear();
        shape::shapes(content, &mut shapes, &mut found);
        let attrs = row.as_ref().map_or(&none, |r| &r.1);
        for s in shapes.drain(..) {
            c.add(s, &layer, attrs, None);
        }
    }

    // What the report says of the records.
    let r = &mut c.report;
    if let Some(at) = records.cut {
        r.skip(
            "Yarım kayıt",
            &format!("“.shp” {at}. baytta bir kaydın uzunluğu dosyanın sonunu aşıyor: dosya yarım ya da bozuk; o kayıttan sonrası alınmadı"),
            0,
        );
    }
    let skip =
        |r: &mut crate::report::Report, n: u32, what: &str, why: &str| r.skip_n(what, why, 0, n);
    skip(
        r,
        deleted,
        "Silinmiş kayıt",
        "öznitelik tablosunda silinmiş işaretli; alınmadı",
    );
    skip(
        r,
        found.short,
        "Kısa kayıt",
        "şeklin gerektirdiği baytlardan kısa; alınmadı",
    );
    skip(
        r,
        found.bad_parts,
        "Bozuk parça dizini",
        "parçalar 0'dan başlayıp sırayla ilerlemiyor; kayıt alınmadı",
    );
    skip(
        r,
        found.not_finite,
        "Sonlu olmayan koordinat",
        "sayı değil ya da sonsuz; o nokta, parça ya da alan alınmadı",
    );
    skip(
        r,
        found.short_parts,
        "Kısa parça",
        "ikiden az noktası var; çizgi olamaz, alınmadı",
    );
    skip(
        r,
        found.short_rings,
        "Kısa halka",
        "üçten az köşesi ya da sıfır alanı var; alınmadı",
    );
    skip(
        r,
        found.multipatch,
        "MultiPatch",
        "üç boyutlu yüzeyler alınmaz",
    );
    skip(
        r,
        found.unknown,
        "Bilinmeyen şekil türü",
        "Shapefile belirtiminde yok; alınmadı",
    );
    r.note_n(
        "Z (yükseklik)",
        "çizgi ve alanların Z değerleri alınmadı: yalnız noktalar Z taşır",
        0,
        found.z_dropped,
    );
    r.note_n(
        "M (ölçü) değerleri",
        "KentOS nesneleri M taşımaz; alınmadı",
        0,
        found.m,
    );
    r.note_n(
        "Dış sınırsız delik",
        "hiçbir dış sınırın (saat yönünde halka) içinde değil; ayrı alan olarak alındı",
        0,
        found.lone_holes,
    );
    r.note_n(
        "Yönsüz halkalar",
        "kayıtta saat yönünde halka yok; her halka ayrı alan olarak alındı",
        0,
        found.no_outline,
    );
    if let Some(t) = &table {
        if t.count != n {
            r.note(
                "Kayıt sayısı",
                &format!(
                    "“.dbf” {} kayıt, “.shp” {n} kayıt taşıyor; öznitelikler sırayla eşlendi",
                    t.count
                ),
                0,
            );
        }
        if t.declared > t.count {
            r.note(
                "Yarım öznitelik tablosu",
                &format!(
                    "“.dbf” başlığı {} kayıt diyor, dosyada {} tam kayıt var",
                    t.declared, t.count
                ),
                0,
            );
        }
        let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
        for f in &t.fields {
            if !matches!(f.kind, b'C' | b'N' | b'F' | b'L' | b'D') {
                *kinds.entry(dbf::kind_name(f.kind)).or_insert(0) += 1;
            }
        }
        for (k, count) in kinds {
            r.note_n(
                &format!("Öznitelik alanı türü {k}"),
                "desteklenmiyor; bu alanlar alınmadı",
                0,
                count,
            );
        }
        let outside = t.outside().count() as u32;
        r.note_n(
            "Öznitelik alanı",
            "kayıt uzunluğunu aşıyor (bozuk başlık); alınmadı",
            0,
            outside,
        );
    }
    if let Some(shx) = files.shx {
        match shape::header(shx, ".shx") {
            Ok(_) => {
                let listed = shx.len().saturating_sub(100) / 8;
                if listed != n {
                    r.note(
                        "Dizin (.shx)",
                        &format!("{listed} kayıt listeliyor, “.shp”de {n} kayıt okundu; kayıtlar “.shp”den sırayla okundu"),
                        0,
                    );
                }
            }
            Err(e) => r.note(
                "Dizin (.shx)",
                &format!("{e} Kayıtlar “.shp”den sırayla okundu"),
                0,
            ),
        }
    }

    // Facts shown before the import.
    r.fact("Şekil türü", shape::type_name(head.shape_type));
    r.fact("Kayıt", n.to_string());
    if head.version != 1000 {
        r.fact("Sürüm", head.version.to_string());
    }
    if let Some(t) = &table {
        r.fact("Öznitelik alanı", t.fields.len().to_string());
        let said = match &source {
            EncodingSource::Cpg(s) => format!("{} (.cpg: {s})", enc.label()),
            EncodingSource::UnknownCpg(s) => {
                format!("{} (varsayılan; .cpg “{s}” tanınmadı)", enc.label())
            }
            EncodingSource::Driver(b) => format!("{} (dil sürücüsü 0x{b:02X})", enc.label()),
            EncodingSource::Default => format!("{} (varsayılan)", enc.label()),
        };
        r.fact("Kodlama", said);
        if matches!(
            source,
            EncodingSource::Default | EncodingSource::UnknownCpg(_)
        ) {
            r.note(
                "Kodlama",
                "dosya kodlamasını belirtmiyor ya da tanınmıyor; Windows-1254 (Türkçe) varsayıldı. Yazılar bozuk görünürse katmanın .cpg dosyasını da seçin",
                0,
            );
        }
    }
    let declared = files.prj.map(|p| {
        let prj = prj::read(p);
        DeclaredCrs {
            srid: prj.srid,
            text: prj.name,
            source: CrsSource::Prj,
        }
    });
    r.fact(
        "Koordinat sistemi (.prj)",
        match &declared {
            Some(d) => match d.srid {
                Some(s) => format!("{} (EPSG:{s})", d.text),
                None => format!("{} (tanınmadı)", d.text),
            },
            None => "yok".into(),
        },
    );
    Ok(c.finish(&layer, declared))
}

/// The encoding name the rules use (for the fixtures' canonical output).
pub fn encoding_name(files: &Files) -> &'static str {
    let driver = files.dbf.and_then(|d| d.get(29)).copied().unwrap_or(0);
    dbf::encoding(files.cpg, driver).0.name()
}
