//! Where a KentOS installation keeps what its programs read, and where a user's own files go (CLAUDE.md §26,
//! docs/adr/9312): the one place a program asks for an install folder, so a packager copies exactly these.
//!
//! - **Programs** ([`program_dir`]): the running program's folder (Windows and Linux: the install folder; macOS:
//!   `KentOS.app/Contents/MacOS`): kentos-cad, kentos-launcher, the modules' names, the command line.
//! - **Resources** ([`resources`]): what the programs read and never write (Windows and Linux: the install folder;
//!   macOS: `KentOS.app/Contents/Resources`). A checkout's are its `paket` folder, which the installer copies whole:
//!   `fonts` (font files and their licences), `eklentiler` (the extensions KentOS brings, the sample Python
//!   processing tool among them), `betikler` (sample Python scripts).
//! - **The user's folder** ([`user_root`]): what the user installs, writes and keeps (`kentos-cad` in the system's
//!   application data folder): `eklentiler`, `fonts`, `betikler`, `python`.
//!
//! `KENTOS_RESOURCES_DIR` and `KENTOS_USER_DIR` point elsewhere (tests, real-window sessions).

use std::path::{Path, PathBuf};

/// The resources' and the user's folder for font files.
pub const FONTS: &str = "fonts";
/// The resources' folder for the extensions KentOS brings; the user's for those they install.
pub const EXTENSIONS: &str = "eklentiler";
/// The resources' folder for sample Python scripts; the user's for their own.
pub const SCRIPTS: &str = "betikler";
/// The file that marks a checkout's resources folder (`paket/README.md`).
const CHECKOUT_MARK: &str = "README.md";

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// The running program's folder; none when its path is not known.
pub fn program_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

/// An installation's resources for a program in `dir` on a system (`macos`, `windows`, `linux`): a macOS bundle's
/// `Contents/Resources`, else the program's own folder.
pub fn resources_for(os: &str, dir: &Path) -> PathBuf {
    match dir.parent() {
        Some(contents) if os == "macos" && dir.ends_with("Contents/MacOS") => contents.join("Resources"),
        _ => dir.to_path_buf(),
    }
}

/// This program's resources: `KENTOS_RESOURCES_DIR`, else the installation's ([`resources_for`]) when it has any of
/// the folders, else a checkout's `paket` above the program (`target/<profile>/`, a test's deeper).
pub fn resources() -> Option<PathBuf> {
    if let Some(dir) = env("KENTOS_RESOURCES_DIR") {
        return Some(dir);
    }
    let dir = program_dir()?;
    let installed = resources_for(std::env::consts::OS, &dir);
    if [FONTS, EXTENSIONS, SCRIPTS].iter().any(|f| installed.join(f).is_dir()) {
        return Some(installed);
    }
    dir.ancestors()
        .take(9)
        .map(|a| a.join("paket"))
        .find(|p| p.join(CHECKOUT_MARK).is_file())
        .or(Some(installed))
}

/// A resources folder that is there.
fn resource(name: &str) -> Option<PathBuf> {
    resources().map(|r| r.join(name)).filter(|p| p.is_dir())
}

/// The installation's font files (a licensed copy of a face KentOS may not ship is put here).
pub fn fonts() -> Option<PathBuf> {
    resource(FONTS)
}

/// The extensions KentOS brings, each a folder (the sample Python processing tool, `ornek-isleme`).
pub fn bundled_extensions() -> Option<PathBuf> {
    resource(EXTENSIONS)
}

/// The sample Python scripts KentOS brings.
pub fn scripts() -> Option<PathBuf> {
    resource(SCRIPTS)
}

/// The user's folder for a system, from its environment: `%APPDATA%\kentos-cad` on Windows, `~/Library/Application
/// Support/kentos-cad` on macOS, `$XDG_DATA_HOME/kentos-cad` (else `~/.local/share/kentos-cad`) elsewhere.
pub fn user_root_for(
    os: &str,
    appdata: Option<&Path>,
    home: Option<&Path>,
    xdg_data_home: Option<&Path>,
) -> Option<PathBuf> {
    let base = match os {
        "windows" => appdata.map(Path::to_path_buf),
        "macos" => home.map(|h| h.join("Library").join("Application Support")),
        _ => xdg_data_home
            .map(Path::to_path_buf)
            .or_else(|| home.map(|h| h.join(".local").join("share"))),
    };
    base.map(|b| b.join("kentos-cad"))
}

/// The user's folder (`KENTOS_USER_DIR`, else [`user_root_for`] this system); none when the system gives no home.
pub fn user_root() -> Option<PathBuf> {
    env("KENTOS_USER_DIR").or_else(|| {
        user_root_for(
            std::env::consts::OS,
            env("APPDATA").as_deref(),
            env("HOME").as_deref(),
            env("XDG_DATA_HOME").as_deref(),
        )
    })
}

/// The user's own font files.
pub fn user_fonts() -> Option<PathBuf> {
    user_root().map(|u| u.join(FONTS)).filter(|p| p.is_dir())
}

/// The user's own Python scripts.
pub fn user_scripts() -> Option<PathBuf> {
    user_root().map(|u| u.join(SCRIPTS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_keeps_its_resources_in_contents_and_a_folder_beside_the_program() {
        let app = Path::new("/Applications/KentOS.app/Contents/MacOS");
        assert_eq!(resources_for("macos", app), Path::new("/Applications/KentOS.app/Contents/Resources"));
        assert_eq!(resources_for("windows", Path::new("C:/KentOS")), Path::new("C:/KentOS"));
        assert_eq!(resources_for("linux", Path::new("/opt/kentos")), Path::new("/opt/kentos"));
        // A program outside a bundle on macOS is a folder install.
        assert_eq!(resources_for("macos", Path::new("/opt/kentos")), Path::new("/opt/kentos"));
    }

    #[test]
    fn the_users_folder_is_the_systems_application_data() {
        let home = Path::new("/home/a");
        assert_eq!(
            user_root_for("linux", None, Some(home), None),
            Some(PathBuf::from("/home/a/.local/share/kentos-cad"))
        );
        assert_eq!(
            user_root_for("linux", None, Some(home), Some(Path::new("/x"))),
            Some(PathBuf::from("/x/kentos-cad"))
        );
        assert_eq!(
            user_root_for("macos", None, Some(Path::new("/Users/a")), None),
            Some(PathBuf::from("/Users/a/Library/Application Support/kentos-cad"))
        );
        assert_eq!(
            user_root_for("windows", Some(Path::new("C:/Users/a/AppData/Roaming")), None, None),
            Some(PathBuf::from("C:/Users/a/AppData/Roaming/kentos-cad"))
        );
        assert_eq!(user_root_for("windows", None, Some(home), None), None);
    }

    /// The checkout's `paket` folder holds what the standard names: the folders a packager copies are there.
    #[test]
    fn the_checkout_holds_the_resources_folders() {
        let paket = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../paket");
        assert!(paket.join(CHECKOUT_MARK).is_file());
        for folder in [FONTS, EXTENSIONS, SCRIPTS] {
            assert!(paket.join(folder).is_dir(), "paket/{folder}");
        }
        assert!(paket.join("eklentiler/ornek-isleme/manifest.json").is_file());
    }
}
