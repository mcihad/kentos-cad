//! Finding a program to start (docs/adr/0800, 0801): KentOS's own programs
//! are installed side by side, so the launcher and a module handing a file to
//! another module look next to themselves first, then on `PATH`.

use std::path::{Path, PathBuf};

/// `name` as a file in the first of `dirs` that has it (`name.exe` on
/// Windows when `name` has no extension). A path with a folder in it is taken
/// as it is, if it exists.
pub fn find_in(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let given = Path::new(name);
    if given.components().count() > 1 || given.is_absolute() {
        return given.is_file().then(|| given.to_path_buf());
    }
    let file = file_name(name);
    dirs.iter().map(|d| d.join(&file)).find(|p| p.is_file())
}

/// `name` next to the running program, else on `PATH`.
pub fn find(name: &str) -> Option<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    find_in(name, &dirs)
}

fn file_name(name: &str) -> String {
    if cfg!(windows) && Path::new(name).extension().is_none() {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_folder_that_has_it_wins_and_a_path_is_taken_as_it_is() {
        let base = std::env::temp_dir().join(format!("kentos-programs-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).expect("a");
        std::fs::create_dir_all(&b).expect("b");
        let name = file_name("kentos-gis");
        std::fs::write(b.join(&name), b"").expect("program");
        assert_eq!(
            find_in("kentos-gis", &[a.clone(), b.clone()]),
            Some(b.join(&name))
        );
        std::fs::write(a.join(&name), b"").expect("program");
        assert_eq!(
            find_in("kentos-gis", &[a.clone(), b.clone()]),
            Some(a.join(&name))
        );
        assert_eq!(find_in("kentos-yok", &[a.clone(), b.clone()]), None);
        let full = b.join(&name);
        assert_eq!(find_in(&full.to_string_lossy(), &[]), Some(full.clone()));
        assert_eq!(find_in(&b.join("yok").to_string_lossy(), &[a]), None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
