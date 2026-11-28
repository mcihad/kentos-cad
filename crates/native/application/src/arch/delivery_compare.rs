//! Öncekiyle karşılaştır (docs/adr/8099 §2 K150; plan/bim-plan.md BIM-15): a delivery's package beside the package that
//! went to the same authority before it. A package is a publication folder (`yayin.json`, docs/adr/0617) named by its
//! manifest's SHA-256: each folder is read again and must still be the package the record names.
//!
//! The files pair by the sheets they hold, else by their name, else by their content, else — one left on each side with
//! the same extension — as the same file renamed (the model's EK-2 name carries the revision number). The drawings
//! (`kaynaklar/cizim.kcad`, when both packages carry one) compare by the objects' persistent ids
//! ([`crate::drawing_compare::diff`]); for each review note of the previous answer, how many of the objects it is bound
//! to changed. It writes nothing and reads only the two folders.

use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{COMPARE_TOLERANCE, Delivery, DeliveryPackage, ReviewNote};
use kentos_domain::{Document, Uuid};

use crate::drawing_compare;
use crate::file_import_block::read_drawing;

/// The publication's manifest in its folder.
pub const MANIFEST: &str = "yayin.json";
/// The drawing a publication with its sources carries.
pub const DRAWING: &str = "kaynaklar/cizim.kcad";
/// The manifest's format.
const FORMAT: &str = "kentos.publish/1";

/// One delivered file as the manifest lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFile {
    pub name: String,
    /// The sheets it holds, by id; none for the model and the lists.
    pub sheets: Vec<String>,
    pub sha256: String,
}

/// A publication folder read: the package as a delivery names it, its files, and its drawing when it carries one.
#[derive(Clone, Debug, PartialEq)]
pub struct Publication {
    pub package: DeliveryPackage,
    pub files: Vec<PackageFile>,
    pub drawing: Option<PathBuf>,
}

/// The publication in `folder`; why not, with what to do.
pub fn read(folder: &Path) -> Result<Publication, String> {
    let manifest = folder.join(MANIFEST);
    let bytes = std::fs::read(&manifest).map_err(|_| {
        format!(
            "“{}” klasöründe {MANIFEST} yok: Paftaları toplu yayınla'nın yazdığı yayın klasörünü seçin.",
            folder.display()
        )
    })?;
    let not_ours = || {
        format!(
            "{} bir KentOS yayınının dosyası değil: Paftaları toplu yayınla'nın yazdığı yayın klasörünü seçin.",
            manifest.display()
        )
    };
    let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| not_ours())?;
    if json["format"] != FORMAT {
        return Err(not_ours());
    }
    let listed = json["files"].as_array().ok_or_else(not_ours)?;
    let mut files = Vec::with_capacity(listed.len());
    for f in listed {
        let (Some(name), Some(sha256)) = (f["name"].as_str(), f["sha256"].as_str()) else {
            return Err(not_ours());
        };
        let sheets = f["sheets"]
            .as_array()
            .map(|s| {
                s.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        files.push(PackageFile {
            name: name.to_owned(),
            sheets,
            sha256: sha256.to_owned(),
        });
    }
    let drawing = folder.join(DRAWING);
    Ok(Publication {
        package: DeliveryPackage {
            manifest: sha256_hex(&bytes),
            files: u32::try_from(files.len()).unwrap_or(u32::MAX),
            path: Some(folder.display().to_string()),
            source: json["source"]["revision"].as_str().map(str::to_owned),
        },
        files,
        drawing: drawing.is_file().then_some(drawing),
    })
}

/// What became of a file between the two packages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileChange {
    Same,
    Changed,
    Renamed,
    Added,
    Removed,
}

impl FileChange {
    pub fn label(self) -> &'static str {
        match self {
            FileChange::Same => "Aynı",
            FileChange::Changed => "Değişti",
            FileChange::Renamed => "Adı değişti",
            FileChange::Added => "Yeni",
            FileChange::Removed => "Kalktı",
        }
    }
}

/// A file of either package and what became of it: its name in the previous package and in this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileCompared {
    pub old: Option<String>,
    pub new: Option<String>,
    pub change: FileChange,
}

fn extension(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
}

/// The files paired (by the sheets they hold, by name, by content, by an extension one file on each side is left
/// with), in this package's order; the previous package's files left over after them, in its order.
pub fn files(old: &[PackageFile], new: &[PackageFile]) -> Vec<FileCompared> {
    let mut pair: Vec<Option<usize>> = vec![None; new.len()];
    let mut used = vec![false; old.len()];
    #[allow(clippy::type_complexity)]
    let passes: [&dyn Fn(&PackageFile, &PackageFile) -> bool; 3] = [
        &|o, n| !n.sheets.is_empty() && o.sheets == n.sheets,
        &|o, n| o.name == n.name,
        &|o, n| o.sha256 == n.sha256,
    ];
    for same in passes {
        for (k, n) in new.iter().enumerate() {
            if pair[k].is_some() {
                continue;
            }
            if let Some(i) = (0..old.len()).find(|&i| !used[i] && same(&old[i], n)) {
                pair[k] = Some(i);
                used[i] = true;
            }
        }
    }
    // The model's EK-2 name carries the revision: of the files that hold no sheet, one left on each side with an
    // extension is that file. A sheet's file has its sheet to pair by.
    let loose = |f: &PackageFile, ext: &str| {
        f.sheets.is_empty() && extension(&f.name).as_deref() == Some(ext)
    };
    for (k, n) in new.iter().enumerate() {
        if pair[k].is_some() || !n.sheets.is_empty() {
            continue;
        }
        let Some(ext) = extension(&n.name) else {
            continue;
        };
        let left_new = (0..new.len()).filter(|&j| pair[j].is_none() && loose(&new[j], &ext));
        let left_old: Vec<usize> = (0..old.len())
            .filter(|&i| !used[i] && loose(&old[i], &ext))
            .collect();
        if left_new.count() == 1 && left_old.len() == 1 {
            pair[k] = Some(left_old[0]);
            used[left_old[0]] = true;
        }
    }
    let mut out: Vec<FileCompared> = new
        .iter()
        .zip(&pair)
        .map(|(n, p)| match p {
            Some(i) => {
                let o = &old[*i];
                let change = match (o.sha256 == n.sha256, o.name == n.name) {
                    (true, true) => FileChange::Same,
                    (true, false) => FileChange::Renamed,
                    (false, _) => FileChange::Changed,
                };
                FileCompared {
                    old: Some(o.name.clone()),
                    new: Some(n.name.clone()),
                    change,
                }
            }
            None => FileCompared {
                old: None,
                new: Some(n.name.clone()),
                change: FileChange::Added,
            },
        })
        .collect();
    out.extend(
        old.iter()
            .zip(&used)
            .filter(|(_, u)| !**u)
            .map(|(o, _)| FileCompared {
                old: Some(o.name.clone()),
                new: None,
                change: FileChange::Removed,
            }),
    );
    out
}

/// The two drawings' objects: equal, changed, added, removed; and the project's other differences (sheets, views…).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DrawingCompared {
    pub same: usize,
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
    pub project: usize,
}

/// A review note of the previous answer: the objects it is bound to, how many changed and how many are gone in this
/// package's drawing, how many the previous package's drawing did not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteCompared {
    pub id: String,
    pub number: u32,
    pub text: String,
    pub objects: usize,
    pub changed: usize,
    pub removed: usize,
    pub unknown: usize,
}

/// A delivery's package beside the previous one's.
#[derive(Clone, Debug, PartialEq)]
pub struct Compared {
    /// The previous delivery's id, number and revision.
    pub previous: (String, u32, String),
    pub files: Vec<FileCompared>,
    /// The drawings; why not (a package without its drawing, a drawing that does not read).
    pub drawing: Result<DrawingCompared, String>,
    /// The previous answer's notes, when both drawings read.
    pub notes: Vec<NoteCompared>,
}

/// The package `d` names, read from its folder and checked against the record.
fn package_of(d: &Delivery) -> Result<Publication, String> {
    let Some(path) = d.package.path.as_deref() else {
        return Err(format!(
            "{}. teslimin paket klasörü kayıtlı değil; karşılaştırma klasörü okur.",
            d.number
        ));
    };
    let read = read(Path::new(path)).map_err(|why| format!("{}. teslim: {why}", d.number))?;
    if read.package.manifest != d.package.manifest {
        return Err(format!(
            "“{path}” klasöründeki {MANIFEST} {}. teslimin paketi değil (SHA-256 farklı): klasör sonradan değişmiş ya da \
             başka bir yayın. Kayıttaki paketin klasörünü yerine koyun.",
            d.number
        ));
    }
    Ok(read)
}

fn load(path: &Path) -> Result<Document, String> {
    let name = path.display();
    let bytes = std::fs::read(path)
        .map_err(|_| format!("{name} okunamadı: dosya yok ya da açılamıyor."))?;
    let snapshot = read_drawing(&bytes)
        .map_err(|why| format!("{name} bir KentOS çizimi olarak okunamadı: {why}"))?;
    Document::from_snapshot_v2(snapshot)
        .map_err(|why| format!("{name} bir KentOS çizimi olarak okunamadı: {why}"))
}

/// `chosen`'s package beside `previous`'s, the previous answer's notes found among `notes` (the drawing's review
/// notes); why not (a folder missing or changed).
pub fn compare(
    chosen: &Delivery,
    previous: &Delivery,
    notes: &[ReviewNote],
) -> Result<Compared, String> {
    let old = package_of(previous)?;
    let new = package_of(chosen)?;
    let files = files(&old.files, &new.files);
    let drawings = match (&old.drawing, &new.drawing) {
        (Some(o), Some(n)) => load(o).and_then(|o| load(n).map(|n| (o, n))),
        (None, _) => Err(format!(
            "{}. teslimin paketi çizimi taşımıyor ({DRAWING}): nesneleri karşılaştırmak için paketi kaynaklarıyla \
             yayımlayın.",
            previous.number
        )),
        (_, None) => Err(format!(
            "{}. teslimin paketi çizimi taşımıyor ({DRAWING}): nesneleri karşılaştırmak için paketi kaynaklarıyla \
             yayımlayın.",
            chosen.number
        )),
    };
    let (drawing, notes) = match drawings {
        Ok((o, n)) => {
            let diff = drawing_compare::diff(&o, &n, COMPARE_TOLERANCE);
            let counts = DrawingCompared {
                same: diff.same,
                changed: diff.changed.len(),
                added: diff.added.len(),
                removed: diff.removed.len(),
                project: drawing_compare::project_diff(&o, &n).len(),
            };
            let answered = previous
                .response
                .as_ref()
                .map(|r| r.notes.as_slice())
                .unwrap_or_default();
            let bound = answered
                .iter()
                .filter_map(|id| notes.iter().find(|x| &x.id == id))
                .map(|note| {
                    let mut c = NoteCompared {
                        id: note.id.clone(),
                        number: note.number,
                        text: note.text.clone(),
                        objects: note.anchors.len(),
                        changed: 0,
                        removed: 0,
                        unknown: 0,
                    };
                    for a in &note.anchors {
                        let uid = Uuid::parse_str(&a.uid).ok();
                        let was = uid.and_then(|u| o.slot_of(u));
                        let now = uid.and_then(|u| n.slot_of(u));
                        match (was, now) {
                            (None, _) => c.unknown += 1,
                            (Some(_), None) => c.removed += 1,
                            (Some(w), Some(_)) if diff.changed.iter().any(|x| x.old == w) => {
                                c.changed += 1
                            }
                            _ => {}
                        }
                    }
                    c
                })
                .collect();
            (Ok(counts), bound)
        }
        Err(why) => (Err(why), Vec::new()),
    };
    Ok(Compared {
        previous: (
            previous.id.clone(),
            previous.number,
            previous.revision.clone(),
        ),
        files,
        drawing,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, sheets: &[&str], sha: &str) -> PackageFile {
        PackageFile {
            name: name.into(),
            sheets: sheets.iter().map(|s| (*s).to_owned()).collect(),
            sha256: sha.into(),
        }
    }

    #[test]
    fn the_files_pair_by_sheet_then_name_then_content_then_a_sole_extension() {
        let old = [
            file("001-A01.pdf", &["s1"], "a"),
            file("002-A02.pdf", &["s2"], "b"),
            file("260123_00_MM_0001_MD_MIM_001_01_000.ifc", &[], "m0"),
            file("teslim-listesi.csv", &[], "l0"),
            file("003-A03.pdf", &["s3"], "c"),
            file("notlar.txt", &[], "t"),
        ];
        let new = [
            // The same sheet under a name with its revision: changed.
            file("001-A01-R1.pdf", &["s1"], "a2"),
            // The same sheet, same bytes, same name.
            file("002-A02.pdf", &["s2"], "b"),
            // The model renamed by its revision, its content changed: the sole .ifc on each side.
            file("260123_00_MM_0001_MD_MIM_001_01_001.ifc", &[], "m1"),
            // The list by its name.
            file("teslim-listesi.csv", &[], "l1"),
            // A new sheet.
            file("004-A04.pdf", &["s4"], "d"),
            // The note renamed, its bytes the same.
            file("aciklama.txt", &[], "t"),
        ];
        let paired = files(&old, &new);
        let got: Vec<(Option<&str>, Option<&str>, FileChange)> = paired
            .iter()
            .map(|f| (f.old.as_deref(), f.new.as_deref(), f.change))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    Some("001-A01.pdf"),
                    Some("001-A01-R1.pdf"),
                    FileChange::Changed
                ),
                (Some("002-A02.pdf"), Some("002-A02.pdf"), FileChange::Same),
                (
                    Some("260123_00_MM_0001_MD_MIM_001_01_000.ifc"),
                    Some("260123_00_MM_0001_MD_MIM_001_01_001.ifc"),
                    FileChange::Changed
                ),
                (
                    Some("teslim-listesi.csv"),
                    Some("teslim-listesi.csv"),
                    FileChange::Changed
                ),
                (None, Some("004-A04.pdf"), FileChange::Added),
                (
                    Some("notlar.txt"),
                    Some("aciklama.txt"),
                    FileChange::Renamed
                ),
                (Some("003-A03.pdf"), None, FileChange::Removed),
            ]
        );
        // Two files left with one extension on a side are not guessed at.
        let old = [file("a.ifc", &[], "1"), file("b.ifc", &[], "2")];
        let new = [file("c.ifc", &[], "3")];
        let got: Vec<FileChange> = files(&old, &new).iter().map(|f| f.change).collect();
        assert_eq!(
            got,
            vec![FileChange::Added, FileChange::Removed, FileChange::Removed]
        );
    }

    #[test]
    fn a_folder_is_read_as_the_package_its_manifest_names() {
        let dir = std::env::temp_dir().join(format!("kentos-delivery-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("kaynaklar")).unwrap();
        assert!(read(&dir).unwrap_err().contains("yayin.json yok"));
        std::fs::write(dir.join(MANIFEST), b"{\"format\":\"other\"}").unwrap();
        assert!(
            read(&dir)
                .unwrap_err()
                .contains("KentOS yayınının dosyası değil")
        );
        let manifest = br#"{"format":"kentos.publish/1","source":{"revision":"37","sha256":"x"},
            "files":[{"name":"001-A01.pdf","sheets":["s1"],"bytes":10,"sha256":"aa"},
                     {"name":"m.ifc","sheets":[],"bytes":5,"sha256":"bb"}]}"#;
        std::fs::write(dir.join(MANIFEST), manifest).unwrap();
        let p = read(&dir).unwrap();
        assert_eq!(p.package.manifest, sha256_hex(manifest));
        assert_eq!(
            (p.package.files, p.package.source.as_deref()),
            (2, Some("37"))
        );
        assert_eq!(
            p.package.path.as_deref(),
            Some(dir.display().to_string().as_str())
        );
        assert_eq!(p.files[0], file("001-A01.pdf", &["s1"], "aa"));
        assert_eq!(p.drawing, None);
        std::fs::write(dir.join(DRAWING), b"x").unwrap();
        assert_eq!(read(&dir).unwrap().drawing, Some(dir.join(DRAWING)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
