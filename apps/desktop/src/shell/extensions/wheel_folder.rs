//! The organisation's wheel folder on the desktop (docs/adr/8057 §123, plan/eklenti.md PY-UV-27). Municipal computers
//! often reach no package index; without one an extension's Python contributions never ran there. Where the policy
//! names a folder (`wheelFolder`, a network share the IT unit fills), an extension's environment is prepared from it
//! alone and offline, as the KentOS server prepares one (§66): wheels only, nothing built, each file against the
//! lock's hash.
//!
//! Before uv runs, each wheel the lock names for this computer (its platform tags, KentOS's Python) is looked for in the
//! folder by its file name and checked against the lock's SHA-256. What is missing, what is not the lock's file and
//! what has no wheel for this computer at all are said by package, in the preview and before preparing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_extensions::python_builds::{LockedWheels, Target, locked_wheels};
use kentos_python_env::extension::Listed;

/// The folder the organisation's policy names, while it names one.
pub(crate) fn of_policy(policy: Option<&kentos_extension_host::Policy>) -> Option<PathBuf> {
    policy
        .and_then(|p| p.wheel_folder.as_deref())
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(PathBuf::from)
}

/// This computer as the words say it: its system, machine and KentOS's Python.
fn here(target: &Target) -> String {
    format!(
        "{} {}, Python {}.{}",
        target.os,
        target.machine.id(),
        target.python.0,
        target.python.1
    )
}

/// What the package at `package` (its project and lock files) installs on `target` with the optional features `extras`,
/// checked against `folder`: the packages it will install, or what the folder lacks for this computer, each package
/// said.
pub(crate) fn check(
    folder: &Path,
    package: &Path,
    (project_file, lock_file): (&str, &str),
    target: Option<Target>,
    extras: &[String],
) -> Result<Vec<Listed>, String> {
    let Some(target) = target else {
        return Err("KentOS'un Python'u bu bilgisayarda tanınmadı.".to_owned());
    };
    if !folder.is_dir() {
        return Err(format!(
            "Kurumun tekerlek klasörüne ulaşılamıyor ({}): klasörün bu bilgisayara bağlı olduğunu denetleyin ya da \
             bilgi işleme sorun.",
            folder.display()
        ));
    }
    let mut files = BTreeMap::new();
    for path in [project_file, lock_file] {
        if let Ok(bytes) = std::fs::read(package.join(path)) {
            files.insert(path.to_owned(), bytes);
        }
    }
    let project = kentos_extensions::manifest::PythonProject {
        project_file: project_file.to_owned(),
        lock_file: lock_file.to_owned(),
        optional_features: BTreeMap::new(),
    };
    let here = here(&target);
    let wanted = locked_wheels(&project, &files, &target, extras).ok_or_else(|| {
        format!("Eklentinin kilidi bu bilgisayarı ({here}) desteklemiyor ya da okunamadı.")
    })?;
    let gaps = gaps(folder, &wanted, &here);
    if !gaps.is_empty() {
        return Err(format!(
            "Kurumun tekerlek klasöründe ({}) bu bilgisayar için eksik var. {}",
            folder.display(),
            gaps.join(" ")
        ));
    }
    Ok(wanted
        .into_iter()
        .map(|w| Listed {
            name: w.name,
            version: w.version,
        })
        .collect())
}

/// Each package the folder cannot give, in a sentence: no wheel for this computer in the lock, none of its wheels in
/// the folder, or a file of that name that is not the lock's.
fn gaps(folder: &Path, wanted: &[LockedWheels], here: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in wanted {
        let named = format!("“{}” {}", w.name, w.version);
        // The lock's file names only as plain names: the lock is the extension's, not trusted with a path.
        let plain = |file: &str| {
            file.ends_with(".whl") && !file.contains(['/', '\\']) && !file.starts_with('.')
        };
        let files: Vec<&(String, String)> = w.files.iter().filter(|(f, _)| plain(f)).collect();
        if files.is_empty() {
            out.push(format!(
                "{named}: kilitte bu bilgisayar ({here}) için tekerleği yok; tekerlek klasöründen kaynak paketi \
                 derlenmez."
            ));
            continue;
        }
        let present: Vec<&(String, String)> = files
            .iter()
            .copied()
            .filter(|(f, _)| folder.join(f).is_file())
            .collect();
        let Some((first, _)) = present.first() else {
            out.push(format!(
                "{named}: tekerlek klasöründe yok; bu bilgisayar için dosyası {}.",
                files[0].0
            ));
            continue;
        };
        let lock_s = |(file, hash): &&(String, String)| {
            std::fs::File::open(folder.join(file))
                .and_then(|f| {
                    kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f))
                })
                .is_ok_and(|(_, got)| hash.strip_prefix("sha256:") == Some(got.as_str()))
        };
        if !present.iter().any(lock_s) {
            out.push(format!(
                "{named}: tekerlek klasöründeki {first} kilitteki dosya değil (özeti tutmuyor); kurulmaz."
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use kentos_extensions::python_builds::Machine;

    use super::*;

    const HERE: Target = Target {
        os: kentos_extensions::Os::Linux,
        machine: Machine::X86_64,
        python: (3, 12),
    };

    /// A package folder whose lock asks `kucuk` 1.0 as a wheel of `wheel`'s bytes and, with `tasli`, a package with a
    /// source only; and a folder for the wheels.
    fn made(name: &str, wheel: &[u8], tasli: bool) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "kentos-tekerlek-klasoru-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (package, folder) = (dir.join("paket"), dir.join("tekerlekler"));
        std::fs::create_dir_all(&package).unwrap();
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            package.join("pyproject.toml"),
            "[project]\nname = \"denetim\"\n",
        )
        .unwrap();
        let hash = kentos_commands::encoding::sha256_hex(wheel);
        let mut lock = format!(
            "version = 1\nrequires-python = \">=3.10\"\n\n[[package]]\nname = \"denetim\"\nversion = \"1.0\"\n\
             source = {{ virtual = \".\" }}\ndependencies = [{{ name = \"kucuk\" }}{}]\n\n[[package]]\nname = \"kucuk\"\n\
             version = \"1.0\"\nsource = {{ registry = \"https://pypi.org/simple\" }}\nwheels = [\n    {{ url = \
             \"https://files.pythonhosted.org/packages/kucuk-1.0-py3-none-any.whl\", hash = \"sha256:{hash}\" }},\n]\n",
            if tasli { ", { name = \"tasli\" }" } else { "" }
        );
        if tasli {
            lock.push_str(
                "\n[[package]]\nname = \"tasli\"\nversion = \"0.3\"\nsource = { registry = \"https://pypi.org/simple\" }\n\
                 sdist = { url = \"https://files.pythonhosted.org/packages/tasli-0.3.tar.gz\", hash = \"sha256:05\" }\n",
            );
        }
        std::fs::write(package.join("uv.lock"), lock).unwrap();
        (package, folder)
    }

    fn checked(package: &Path, folder: &Path) -> Result<Vec<Listed>, String> {
        check(
            folder,
            package,
            ("pyproject.toml", "uv.lock"),
            Some(HERE),
            &[],
        )
    }

    #[test]
    fn the_folder_gives_each_package_or_says_what_it_lacks() {
        let wheel = b"PK kucuk 1.0".to_vec();
        let (package, folder) = made("eksik", &wheel, true);
        // Nothing in the folder: each package said, a source-only one as having no wheel here.
        let why = checked(&package, &folder).unwrap_err();
        assert!(
            why.contains("“kucuk” 1.0: tekerlek klasöründe yok; bu bilgisayar için dosyası kucuk-1.0-py3-none-any.whl."),
            "{why}"
        );
        assert!(
            why.contains(
                "“tasli” 0.3: kilitte bu bilgisayar (Linux x86_64, Python 3.12) için tekerleği yok"
            ),
            "{why}"
        );
        // A file of that name that is not the lock's.
        let (package, folder) = made("ozet", &wheel, false);
        std::fs::write(folder.join("kucuk-1.0-py3-none-any.whl"), b"PK baska").unwrap();
        let why = checked(&package, &folder).unwrap_err();
        assert!(
            why.contains("“kucuk” 1.0: tekerlek klasöründeki kucuk-1.0-py3-none-any.whl kilitteki dosya değil"),
            "{why}"
        );
        // The lock's file: what it installs.
        std::fs::write(folder.join("kucuk-1.0-py3-none-any.whl"), &wheel).unwrap();
        let listed = checked(&package, &folder).expect("the folder has it");
        assert_eq!(
            listed
                .iter()
                .map(|l| format!("{} {}", l.name, l.version))
                .collect::<Vec<_>>(),
            ["kucuk 1.0"]
        );
        // A folder the computer does not reach.
        let why = checked(&package, &folder.join("yok")).unwrap_err();
        assert!(
            why.starts_with("Kurumun tekerlek klasörüne ulaşılamıyor"),
            "{why}"
        );
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        let _ = std::fs::remove_dir_all(package.parent().unwrap());
    }

    /// The lock is the extension's: a wheel's name that is a path is not read, outside the folder or in it.
    #[test]
    fn a_wheel_named_as_a_path_is_never_read() {
        let (package, folder) = made("yol", b"PK", false);
        let lock = std::fs::read_to_string(package.join("uv.lock"))
            .unwrap()
            .replace(
                "url = \"https://files.pythonhosted.org/packages/kucuk-1.0-py3-none-any.whl\"",
                "filename = \"..\\\\disari\\\\kucuk-1.0-py3-none-any.whl\"",
            );
        std::fs::write(package.join("uv.lock"), lock).unwrap();
        let why = checked(&package, &folder).unwrap_err();
        assert!(why.contains("“kucuk” 1.0: kilitte bu bilgisayar"), "{why}");
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
    }

    #[test]
    fn the_folder_is_the_policys_when_it_names_one() {
        let read = |text: &str| kentos_extension_host::Policy::read(text).unwrap();
        let with = read(
            r#"{"format":"kentos.extension-policy","schemaVersion":1,"wheelFolder":" \\\\sunucu\\kentos\\tekerlek "}"#,
        );
        assert_eq!(
            of_policy(Some(&with)),
            Some(PathBuf::from("\\\\sunucu\\kentos\\tekerlek"))
        );
        let without = read(r#"{"format":"kentos.extension-policy","schemaVersion":1}"#);
        assert_eq!(of_policy(Some(&without)), None);
        assert_eq!(of_policy(None), None);
    }
}
