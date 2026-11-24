//! Whether a place the user showed (Konumu seç…, docs/adr/0800 §4) can be the program an app starts, and which program
//! it is. A drawing or any other file the system cannot run is not a program: it is refused when chosen, and one kept
//! from before is forgotten when the launcher starts, with a notice, so that the card searches again. A KentOS
//! module's place is `kentos-cad` or the module's own program (`kentos-gis`); another module's program would open the
//! wrong module. An application package (`KentOS.app`) stands for the KentOS program in it.

use std::path::{Path, PathBuf};

use crate::manifest::{Launch, Manifest};
use crate::resolve::{KENTOS_CAD, is_kentos_cad};

/// The program `place` starts for `app`, or why it cannot be one (the sentence the card and the notice say).
pub fn program_at(app: &Manifest, place: &Path) -> Result<PathBuf, String> {
    if !place.exists() {
        return Err(format!(
            "Gösterilen yer “{}” artık yok. Konumu yeniden seçin.",
            place.display()
        ));
    }
    let program = if place.is_dir() {
        in_package(app, place).ok_or_else(|| {
            format!(
                "“{}” bir klasör, program değil. {}",
                name(place),
                what_to_choose(app)
            )
        })?
    } else {
        place.to_path_buf()
    };
    if !runnable(&program) {
        return Err(format!(
            "“{}” bir program değil. {}",
            name(place),
            what_to_choose(app)
        ));
    }
    if let Some(Launch::Kentos(module)) = &app.launch
        && !is_kentos_cad(&program)
        && !named(&program, module.program())
    {
        return Err(format!(
            "“{}” başka bir program; {} açılmaz. {}",
            name(&program),
            app.name,
            what_to_choose(app)
        ));
    }
    Ok(program)
}

/// Why a kept place is to be forgotten: it is there but cannot be the program. A place that is gone is kept (the drive
/// it was on may come back); the card says it is gone.
pub fn stale(app: &Manifest, place: &Path) -> Option<String> {
    if place.exists() {
        program_at(app, place).err()
    } else {
        None
    }
}

/// What the user is to choose instead.
fn what_to_choose(app: &Manifest) -> String {
    match &app.launch {
        Some(Launch::Kentos(module)) => format!(
            "{} için {} ya da {KENTOS_CAD} programını seçin.",
            app.name,
            module.program()
        ),
        _ => format!("{} programının kendisini seçin.", app.name),
    }
}

/// The KentOS program in an application package: the module's own, else `kentos-cad`.
fn in_package(app: &Manifest, place: &Path) -> Option<PathBuf> {
    let Some(Launch::Kentos(module)) = &app.launch else {
        return None;
    };
    if !place
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("app"))
    {
        return None;
    }
    let folder = place.join("Contents").join("MacOS");
    [module.program(), KENTOS_CAD]
        .into_iter()
        .map(|program| folder.join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))
        .find(|program| runnable(program))
}

/// Whether the system can start the file.
#[cfg(unix)]
fn runnable(program: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(program).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// Whether the system can start the file.
#[cfg(not(unix))]
fn runnable(program: &Path) -> bool {
    program.is_file()
        && program.extension().is_some_and(|e| {
            ["exe", "com", "bat", "cmd"]
                .iter()
                .any(|x| e.eq_ignore_ascii_case(x))
        })
}

/// Whether a program has the given name (without the system's `.exe`).
fn named(program: &Path, name: &str) -> bool {
    program
        .file_stem()
        .is_some_and(|stem| stem.eq_ignore_ascii_case(name))
}

/// A place's own name, as the sentences say it.
fn name(place: &Path) -> String {
    place.file_name().map_or_else(
        || place.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Folder, Origin};

    fn app(json: &str) -> Manifest {
        crate::manifest::read(
            json,
            Origin::File {
                path: PathBuf::from("/a/b.json"),
                folder: Folder::User,
            },
        )
        .expect("reads")
        .manifest
    }

    fn gis() -> Manifest {
        app(
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-gis", "launch": {"kind": "kentos", "module": "cbs"}}"#,
        )
    }

    /// A file the system can run (on Unix by its mode, elsewhere by `.exe`).
    fn program(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::create_dir_all(dir).expect("a folder");
        std::fs::write(&path, b"#!/bin/sh\n").expect("a program");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("runnable");
        }
        path
    }

    #[test]
    fn a_drawing_chosen_for_a_module_is_refused_saying_what_to_choose() {
        let dir = crate::files::scratch("place-drawing");
        let drawing = dir.join("konut-3-1.dxf");
        std::fs::write(&drawing, b"0\nEOF\n").expect("a drawing");
        let why = program_at(&gis(), &drawing).expect_err("not a program");
        assert!(
            why.contains("“konut-3-1.dxf” bir program değil")
                && why.contains("KentOS GIS için kentos-gis ya da kentos-cad programını seçin."),
            "{why}"
        );
        assert_eq!(
            stale(&gis(), &drawing),
            Some(why),
            "kept from before: forgotten"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_modules_own_program_and_kentos_cad_are_taken_another_modules_is_not() {
        let dir = crate::files::scratch("place-programs");
        let own = program(&dir, "kentos-gis");
        let cad = program(&dir, "kentos-cad");
        let other = program(&dir, "kentos-mimari");
        assert_eq!(program_at(&gis(), &own), Ok(own.clone()));
        assert_eq!(program_at(&gis(), &cad), Ok(cad.clone()));
        let why = program_at(&gis(), &other).expect_err("another module");
        assert!(
            why.contains("başka bir program; KentOS GIS açılmaz"),
            "{why}"
        );
        assert_eq!(stale(&gis(), &own), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_the_system_cannot_run_is_not_a_program_even_by_its_name() {
        let dir = crate::files::scratch("place-mode");
        std::fs::create_dir_all(&dir).expect("a folder");
        let plain = dir.join("kentos-cad");
        std::fs::write(&plain, b"").expect("a file");
        let why = program_at(&gis(), &plain).expect_err("no run bit");
        assert!(why.contains("“kentos-cad” bir program değil"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_application_package_stands_for_the_kentos_program_in_it() {
        let dir = crate::files::scratch("place-package");
        let package = dir.join("KentOS.app");
        let inside = program(&package.join("Contents").join("MacOS"), "kentos-cad");
        assert_eq!(program_at(&gis(), &package), Ok(inside));
        let empty = dir.join("Boş.app");
        std::fs::create_dir_all(&empty).expect("a folder");
        let why = program_at(&gis(), &empty).expect_err("nothing in it");
        assert!(why.contains("“Boş.app” bir klasör, program değil"), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_gone_place_is_said_and_kept() {
        let dir = crate::files::scratch("place-gone");
        let gone = dir.join("silindi");
        let why = program_at(&gis(), &gone).expect_err("gone");
        assert!(why.contains("artık yok"), "{why}");
        assert_eq!(stale(&gis(), &gone), None, "the drive may come back");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn another_apps_place_is_any_program_by_its_own_name() {
        let report = app(
            r#"{"format": "kentos.launcher-app/1", "id": "r", "name": "Rapor",
                "launch": {"kind": "program", "program": "rapor"}}"#,
        );
        let dir = crate::files::scratch("place-other");
        let any = program(&dir, "rapor-2");
        assert_eq!(program_at(&report, &any), Ok(any.clone()));
        let text = dir.join("not.txt");
        std::fs::write(&text, b"").expect("a file");
        let why = program_at(&report, &text).expect_err("not a program");
        assert!(why.contains("Rapor programının kendisini seçin."), "{why}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
