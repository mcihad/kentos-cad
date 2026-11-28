//! The fonts folders (docs/adr/9313, CLAUDE.md §26): every font file under the install's `fonts` and the user's, read
//! once when a program starts. Each face is listed with what KentOS does with it:
//! - KentOS's own faces (the drawing's, Times New Roman's twin, the fallbacks) draw from the program's own copy;
//! - a licensed Arial or Times New Roman is given to the core in place of its twin (docs/adr/9311);
//! - any other family is given to the core, and texts can take it by its name.
//!
//! Arial and Times New Roman that the folders do not hold come from the operating system's fonts.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use kentos_commands::{DrawingFont, FontFamily, FontFile, FontFolder, FontRole};
use kentos_geometry_core::text::face_info::{self, FaceInfo};
use kentos_geometry_core::text::font_data::{
    ADDED_FACES, ADDED_KEYS, DRAWING_FACES, DRAWING_KEYS, FALLBACK_FACES, InstalledFace, give,
    install,
};

/// A face the core takes in place of a bundled twin, by its key (`("arial", 400, false)`).
type Given = ((&'static str, u16, bool), &'static [u8]);

/// Where an operating system keeps its own Arial and Times New Roman, by key (docs/adr/9311): KentOS uses them but
/// does not ship them (their licences do not allow it). A family without a file of its own (Arial 500) takes the
/// nearest weight a browser would.
const SYSTEM_FILES: [((&str, u16, bool), &[&str]); 8] = [
    (("arial", 400, false), &["Arial.ttf", "arial.ttf"]),
    (("arial", 500, false), &["Arial.ttf", "arial.ttf"]),
    (
        ("arial", 600, false),
        &["Arial Bold.ttf", "arialbd.ttf", "Arial_Bold.ttf"],
    ),
    (
        ("arial", 700, false),
        &["Arial Bold.ttf", "arialbd.ttf", "Arial_Bold.ttf"],
    ),
    (
        ("times-new-roman", 400, false),
        &["Times New Roman.ttf", "times.ttf", "Times_New_Roman.ttf"],
    ),
    (
        ("times-new-roman", 700, false),
        &[
            "Times New Roman Bold.ttf",
            "timesbd.ttf",
            "Times_New_Roman_Bold.ttf",
        ],
    ),
    (
        ("times-new-roman", 400, true),
        &[
            "Times New Roman Italic.ttf",
            "timesi.ttf",
            "Times_New_Roman_Italic.ttf",
        ],
    ),
    (
        ("times-new-roman", 700, true),
        &[
            "Times New Roman Bold Italic.ttf",
            "timesbi.ttf",
            "Times_New_Roman_Bold_Italic.ttf",
        ],
    ),
];

static READ: OnceLock<Vec<FontFile>> = OnceLock::new();

/// Reads the fonts folders and gives the core their faces, once, before the first text is laid out. A program's start
/// calls it (the desktop, `kentos`, `kentos-mcp`, the Python module). The tests do not: they draw in KentOS's own
/// faces. Later calls return the first listing.
pub fn load() -> &'static [FontFile] {
    READ.get_or_init(|| {
        let mut folders: Vec<(FontFolder, PathBuf)> = Vec::new();
        folders.extend(kentos_launch::install::fonts().map(|d| (FontFolder::Program, d)));
        folders.extend(kentos_launch::install::user_fonts().map(|d| (FontFolder::User, d)));
        let (mut files, given, installed) = read(&folders, &system_folders());
        // Given before the core read a face: a program that drew a text first keeps the twins and says so.
        let late = !give(&given) || !install(installed);
        if late {
            for f in files
                .iter_mut()
                .filter(|f| matches!(f.role, FontRole::Given | FontRole::Family))
            {
                f.role = FontRole::Unused;
                f.face = None;
                f.note = Some(
                    "program yazı tiplerini okumadan önce yazı çizdi; programı yeniden başlatın"
                        .to_owned(),
                );
            }
        }
        files
    })
}

/// The listing [`load`] made; empty when the program did not read the folders.
pub fn listing() -> &'static [FontFile] {
    READ.get().map_or(&[], Vec::as_slice)
}

/// The typefaces a person can pick, in the order the lists show them: KentOS's own (`DrawingFont::CHOICES`), then the
/// folders' families by name. `also` is added after them when the lists do not hold it (the value a list shows now).
pub fn choices(also: Option<DrawingFont>) -> Vec<DrawingFont> {
    let mut out: Vec<DrawingFont> = DrawingFont::CHOICES.to_vec();
    let mut families: Vec<DrawingFont> = listing()
        .iter()
        .filter(|f| f.role == FontRole::Family)
        .filter_map(|f| f.face)
        .collect();
    families.sort_by(|a, b| a.label().to_lowercase().cmp(&b.label().to_lowercase()));
    families.dedup();
    out.extend(families);
    if let Some(f) = also.filter(|f| !out.contains(f)) {
        out.push(f);
    }
    out
}

/// Whether this program can draw the typeface in its own faces: KentOS's own always, a folder family when the
/// program's folders hold it.
pub fn drawable(font: DrawingFont) -> bool {
    match font.family() {
        None => true,
        Some(family) => listing()
            .iter()
            .any(|f| f.role == FontRole::Family && f.face == Some(DrawingFont::Installed(family))),
    }
}

/// Reads `folders` in order, then the `system` folders for the Arial and Times New Roman faces the folders did not
/// give. The files, the faces given in place of the twins, the families installed.
fn read(
    folders: &[(FontFolder, PathBuf)],
    system: &[PathBuf],
) -> (Vec<FontFile>, Vec<Given>, Vec<InstalledFace>) {
    let mut files: Vec<FontFile> = Vec::new();
    let mut given: Vec<Given> = Vec::new();
    let mut installed: Vec<InstalledFace> = Vec::new();
    for (folder, dir) in folders {
        for path in kentos_launch::install::font_files(dir) {
            let file = relative(dir, &path);
            match std::fs::read(&path) {
                Ok(bytes) => classify(file, *folder, bytes, &mut files, &mut given, &mut installed),
                Err(e) => files.push(unread(file, *folder, format!("okunamadı ({e})"))),
            }
        }
    }
    // One file gives two keys (Arial 400 and 500): read once, listed once.
    let mut read: Vec<(PathBuf, &'static [u8])> = Vec::new();
    for (key, names) in SYSTEM_FILES {
        if given.iter().any(|(k, _)| *k == key) {
            continue;
        }
        let family = if key.0 == "arial" {
            "Arial"
        } else {
            "Times New Roman"
        };
        for path in system
            .iter()
            .flat_map(|d| names.iter().map(move |n| d.join(n)))
        {
            if let Some((_, bytes)) = read.iter().find(|(p, _)| *p == path) {
                given.push((key, *bytes));
                break;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Some(face) = face_info::describe(&bytes)
                .into_iter()
                .next()
                .filter(|f| f.family == family)
            else {
                continue;
            };
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            given.push((key, bytes));
            read.push((path.clone(), bytes));
            files.push(FontFile {
                role: FontRole::Given,
                face: DrawingFont::from_id(key.0),
                note: Some("işletim sisteminin yazı tipi; KentOS onu dağıtmaz".to_owned()),
                ..file_of(path.display().to_string(), FontFolder::System, &face)
            });
            break;
        }
    }
    (files, given, installed)
}

/// The file's place in its folder, with `/` (`cizim/Barlow-400.ttf`).
fn relative(dir: &Path, path: &Path) -> String {
    let inner = path.strip_prefix(dir).unwrap_or(path);
    inner
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A file not read, or not a font.
fn unread(file: String, folder: FontFolder, why: String) -> FontFile {
    FontFile {
        file,
        folder,
        family: String::new(),
        style: String::new(),
        weight: 400,
        italic: false,
        role: FontRole::Unused,
        face: None,
        turkish: false,
        note: Some(why),
    }
}

fn file_of(file: String, folder: FontFolder, face: &FaceInfo) -> FontFile {
    FontFile {
        file,
        folder,
        family: face.family.clone(),
        style: face.style.clone(),
        weight: face.weight,
        italic: face.italic,
        role: FontRole::Unused,
        face: None,
        turkish: face.turkish,
        note: None,
    }
}

/// The added keys a file of Arial or Times New Roman gives: Arial's upright 400 and 500 or 600 and 700 (it has no
/// italic key: KentOS leans Arial), Times New Roman's four by weight and slope.
fn keys_for(own: DrawingFont, face: &FaceInfo) -> Vec<(&'static str, u16, bool)> {
    let bold = face.weight >= 550;
    ADDED_KEYS
        .iter()
        .copied()
        .filter(|(id, weight, italic)| {
            *id == own.id() && *italic == face.italic && (*weight >= 550) == bold
        })
        .collect()
}

fn classify(
    file: String,
    folder: FontFolder,
    bytes: Vec<u8>,
    files: &mut Vec<FontFile>,
    given: &mut Vec<Given>,
    installed: &mut Vec<InstalledFace>,
) {
    let faces = face_info::describe(&bytes);
    let Some(face) = faces.first() else {
        files.push(unread(
            file,
            folder,
            "yazı tipi olarak okunamadı".to_owned(),
        ));
        return;
    };
    if face_info::is_collection(&bytes) {
        for f in &faces {
            files.push(FontFile {
                note: Some("yazı tipi koleksiyonu (.ttc): yüzlerini ayrı .ttf ya da .otf dosyaları olarak koyun".to_owned()),
                ..file_of(file.clone(), folder, f)
            });
        }
        return;
    }
    let base = file_of(file, folder, face);
    let interface = folder == FontFolder::Program && base.file.starts_with("arayuz/");
    // KentOS's own copy, the same bytes: the program draws its own.
    if let Some(i) = DRAWING_FACES.iter().position(|d| *d == bytes.as_slice()) {
        files.push(FontFile {
            role: FontRole::Own,
            face: DrawingFont::from_id(DRAWING_KEYS[i].0),
            ..base
        });
        return;
    }
    if let Some(i) = ADDED_FACES.iter().position(|d| *d == bytes.as_slice()) {
        let face = DrawingFont::from_id(ADDED_KEYS[i].0);
        let note = format!(
            "{}'ın eşdeğeri: {} kurulu olmayan bilgisayarda onunla çizilir",
            face.map_or("", DrawingFont::label),
            face.map_or("", DrawingFont::label)
        );
        files.push(FontFile {
            role: FontRole::Twin,
            face,
            note: Some(note),
            ..base
        });
        return;
    }
    if FALLBACK_FACES.iter().any(|d| *d == bytes.as_slice()) {
        files.push(FontFile {
            role: FontRole::Fallback,
            note: Some("yazı tiplerinde olmayan harfler için".to_owned()),
            ..base
        });
        return;
    }
    match DrawingFont::own(&face.family) {
        Some(own @ (DrawingFont::Arial | DrawingFont::TimesNewRoman)) => {
            let keys: Vec<_> = keys_for(own, face)
                .into_iter()
                .filter(|k| !given.iter().any(|(g, _)| g == k))
                .collect();
            if keys.is_empty() {
                let why = if face.italic && own == DrawingFont::Arial {
                    "Arial'ın eğik yüzü kullanılmaz: KentOS Arial'ı kendisi eğer"
                } else {
                    "aynı yüz önce başka bir dosyadan alındı"
                };
                files.push(FontFile {
                    note: Some(why.to_owned()),
                    ..base
                });
                return;
            }
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            given.extend(keys.into_iter().map(|k| (k, bytes)));
            files.push(FontFile {
                role: FontRole::Given,
                face: Some(own),
                ..base
            });
        }
        Some(own) => {
            let note = format!("KentOS kendi {} yüzüyle çizer", own.label());
            let role = if interface {
                FontRole::Interface
            } else {
                FontRole::Unused
            };
            files.push(FontFile {
                role,
                note: Some(note),
                ..base
            });
        }
        None if !face.embeddable => {
            let note = "lisansı belgeye gömülmesini yasaklıyor: KentOS onunla çizmez (pafta PDF'i harflerini gömer)";
            files.push(FontFile {
                note: Some(note.to_owned()),
                ..base
            });
        }
        None => {
            let Some(family) = FontFamily::new(&face.family) else {
                files.push(FontFile {
                    note: Some(format!(
                        "“{}” bir yazı tipi ailesinin adı olamaz",
                        face.family
                    )),
                    ..base
                });
                return;
            };
            if installed.iter().any(|i| {
                i.family == family.name() && i.weight == face.weight && i.italic == face.italic
            }) {
                files.push(FontFile {
                    note: Some("aynı yüz önce başka bir dosyadan alındı".to_owned()),
                    ..base
                });
                return;
            }
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            installed.push(InstalledFace {
                family: family.name(),
                weight: face.weight,
                italic: face.italic,
                bytes,
            });
            let mut notes: Vec<&str> = Vec::new();
            if interface {
                notes.push("arayüzün de yazı tipi");
            }
            if face.variable {
                notes.push("değişken yazı tipi: varsayılan kalınlığıyla çizilir");
            }
            if !face.turkish {
                notes.push("Türkçe harflerin bazıları yok: onlar yedek yüzle çizilir");
            }
            files.push(FontFile {
                role: FontRole::Family,
                face: Some(DrawingFont::Installed(family)),
                note: (!notes.is_empty()).then(|| notes.join("; ")),
                ..base
            });
        }
    }
}

/// The operating system's font folders, where Arial and Times New Roman are looked for (docs/adr/9311).
fn system_folders() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        out.push("/System/Library/Fonts/Supplemental".into());
        out.push("/Library/Fonts".into());
        out.extend(home.map(|h| h.join("Library/Fonts")));
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR").map_or_else(|| "C:\\Windows".into(), PathBuf::from);
        out.push(windir.join("Fonts"));
        out.extend(
            std::env::var_os("LOCALAPPDATA")
                .map(|l| PathBuf::from(l).join("Microsoft/Windows/Fonts")),
        );
    } else {
        out.push("/usr/share/fonts/truetype/msttcorefonts".into());
        out.push("/usr/share/fonts/TTF".into());
        out.push("/usr/share/fonts/truetype".into());
        out.extend(home.map(|h| h.join(".local/share/fonts")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkout_fonts() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../paket/fonts")
    }

    /// The checkout's folder: the drawing's faces are KentOS's own, Liberation Serif Times New Roman's twin, the
    /// fallbacks fallbacks, the interface's families families texts can take (docs/adr/9313).
    #[test]
    fn the_checkout_folder_lists_every_face_by_its_use() {
        let (files, given, installed) = read(&[(FontFolder::Program, checkout_fonts())], &[]);
        let of = |file: &str| {
            files
                .iter()
                .find(|f| f.file == file)
                .map(|f| (f.role, f.face))
        };
        assert_eq!(
            of("cizim/Barlow-400.ttf"),
            Some((FontRole::Own, Some(DrawingFont::Barlow)))
        );
        assert_eq!(
            of("cizim/LiberationSerif-Bold.ttf"),
            Some((FontRole::Twin, Some(DrawingFont::TimesNewRoman)))
        );
        assert_eq!(
            of("yedek/NotoSansArabic-400.ttf").map(|f| f.0),
            Some(FontRole::Fallback)
        );
        let roboto = DrawingFont::from_name("Roboto");
        assert_eq!(
            of("arayuz/Roboto-Regular.ttf"),
            Some((FontRole::Family, roboto))
        );
        assert_eq!(
            of("arayuz/IBMPlexMono-Regular.ttf").map(|f| f.0),
            Some(FontRole::Interface)
        );
        assert!(given.is_empty());
        let families: Vec<&str> = installed.iter().map(|f| f.family).collect();
        assert!(
            families.contains(&"Roboto") && families.contains(&"Inter"),
            "{families:?}"
        );
        assert!(!families.contains(&"IBM Plex Mono") && !families.contains(&"Barlow"));
        // Every file is listed, with its family and weight from its own name table.
        assert_eq!(
            files.len(),
            kentos_launch::install::font_files(&checkout_fonts()).len()
        );
        let semibold = files
            .iter()
            .find(|f| f.file == "arayuz/Roboto-SemiBold.ttf");
        assert_eq!(
            semibold.map(|f| (f.family.as_str(), f.weight)),
            Some(("Roboto", 600))
        );
        assert!(
            files
                .iter()
                .all(|f| f.turkish || f.role == FontRole::Fallback),
            "every face writes Turkish"
        );
    }

    /// A user's folder: a family texts can take, a file that is not a font, a collection, a second copy of a face.
    #[test]
    fn a_users_folder_gives_its_families_and_says_why_not() {
        let dir = std::env::temp_dir().join(format!("kentos-fonts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let inner = dir.join("kurum");
        assert!(std::fs::create_dir_all(&inner).is_ok());
        let roboto =
            std::fs::read(checkout_fonts().join("arayuz/Roboto-Regular.ttf")).unwrap_or_default();
        assert!(std::fs::write(inner.join("Kurum-Regular.ttf"), &roboto).is_ok());
        // Read after the family's own file (the files are read in their paths' order): the second copy.
        assert!(std::fs::write(dir.join("zz-kopya.ttf"), &roboto).is_ok());
        assert!(std::fs::write(dir.join("bozuk.otf"), b"not a font").is_ok());
        assert!(std::fs::write(dir.join("Barlow.ttf"), DRAWING_FACES[6]).is_ok());
        let (files, _, installed) = read(&[(FontFolder::User, dir.clone())], &[]);
        let _ = std::fs::remove_dir_all(&dir);
        let of = |file: &str| files.iter().find(|f| f.file == file).cloned();
        let kurum = of("kurum/Kurum-Regular.ttf");
        assert_eq!(kurum.as_ref().map(|f| f.role), Some(FontRole::Family));
        assert_eq!(kurum.as_ref().map(|f| f.folder), Some(FontFolder::User));
        assert_eq!(installed.len(), 1, "the copy is not given twice");
        let copy = of("zz-kopya.ttf");
        assert_eq!(copy.as_ref().map(|f| f.role), Some(FontRole::Unused));
        assert!(
            copy.and_then(|f| f.note)
                .is_some_and(|n| n.contains("önce başka"))
        );
        assert!(
            of("bozuk.otf")
                .and_then(|f| f.note)
                .is_some_and(|n| n.contains("okunamadı"))
        );
        assert_eq!(of("Barlow.ttf").map(|f| f.role), Some(FontRole::Own));
    }

    #[test]
    fn choices_start_with_the_own_faces() {
        let list = choices(DrawingFont::from_name("Kurum Yazısı"));
        assert_eq!(&list[..9], &DrawingFont::CHOICES);
        assert_eq!(list.last().map(|f| f.label()), Some("Kurum Yazısı"));
        assert!(drawable(DrawingFont::Arial));
    }
}
