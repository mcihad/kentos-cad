//! `kentos`: KentOS's command line (plan/eklenti.md ADIM 16, D2). Its first commands are the extension package's:
//!
//! ```text
//! kentos kek inspect <paket.kek> [--json]
//! kentos kek pack <klasör> [-o <paket.kek>] [--json]
//! kentos kek index <klasör> [--name <ad>]
//! kentos kek dump [--root <klasör>]
//! ```
//!
//! The same inspector the installer, the catalog and `extension.verify` run (kentos_extensions): an author or an agent
//! hears every problem with its file, place and fix before anyone else does. `pack` leaves out what every working
//! folder has (the environment, caches, version control, secrets, AI tool settings) and says what it left out; a
//! symbolic link in the folder is refused, since it can point at a file outside it. Exit codes: 0 done, 1 refused, 2 a
//! wrong command or a file that cannot be read or written.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kentos_extensions::{Limits, Manifest, Problem, catalog, inspect, pack};

const USAGE: &str = "Kullanım:
  kentos kek inspect <paket.kek> [--json]           paketi kodunu çalıştırmadan denetler
  kentos kek pack <klasör> [-o <paket.kek>] [--json] klasörü denetleyip paketler
  kentos kek index <klasör> [--name <ad>]           klasördeki paketlerin katalog dizinini (index.json) yazar
  kentos kek dump [--root <klasör>]                 kurulu eklentileri ve katkılarını JSON olarak döker";

/// Folders the packer leaves out whole, not read at all (a `.venv` holds hundreds of megabytes); the packer's own
/// rules leave out the rest.
const NOT_READ: &[&str] = &[
    ".venv",
    ".git",
    ".hg",
    ".svn",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".nox",
    ".ipynb_checkpoints",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    match args.as_slice() {
        ["kek", "inspect", file] => inspect_file(Path::new(file), json),
        ["kek", "pack", folder] => pack_folder(Path::new(folder), None, json),
        ["kek", "pack", folder, "-o", out] => {
            pack_folder(Path::new(folder), Some(PathBuf::from(out)), json)
        }
        ["kek", "dump"] => dump(None),
        ["kek", "dump", "--root", root] => dump(Some(PathBuf::from(root))),
        ["kek", "index", folder] => index_folder(Path::new(folder), None),
        ["kek", "index", folder, "--name", name] => index_folder(Path::new(folder), Some(name)),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn fail(message: String) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(2)
}

fn print_list(title: &str, problems: &[Problem]) {
    if !problems.is_empty() {
        println!("{title}:");
        for problem in problems {
            println!("  • {problem}");
        }
    }
}

fn inspect_file(file: &Path, json: bool) -> ExitCode {
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(e) => return fail(format!("{} okunamadı: {e}", file.display())),
    };
    let inspection = inspect(&bytes, &Limits::default());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&inspection).unwrap_or_default()
        );
    } else {
        if let Some(m) = &inspection.manifest {
            println!("{} {} — {}", m.id, m.version, m.name);
        }
        if let Some(digest) = &inspection.digest {
            println!("İçerik özeti: {digest}");
        }
        print_list("Sorunlar", &inspection.problems);
        print_list("Uyarılar", &inspection.warnings);
        if inspection.is_valid() {
            println!("{} dosya; paket geçerli.", inspection.files.len());
        } else {
            println!("Paket geçersiz: {} sorun.", inspection.problems.len());
        }
    }
    if inspection.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// A folder read: its files by their paths inside it (`/` between folders), and the folders not read.
type Folder = (BTreeMap<String, Vec<u8>>, Vec<String>);

/// The folder's files; the folders not read are named.
fn read_folder(root: &Path) -> Result<Folder, String> {
    let mut files = BTreeMap::new();
    let mut not_read = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("{} okunamadı: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{} okunamadı: {e}", dir.display()))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(Path::to_str)
                .ok_or_else(|| {
                    format!(
                        "{}: adı UTF-8 değil; dosyayı yeniden adlandırın.",
                        path.display()
                    )
                })?
                .replace('\\', "/");
            let kind = entry
                .file_type()
                .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
            if kind.is_symlink() {
                return Err(format!(
                    "{relative}: sembolik bağ; paket yalnız gerçek dosya taşır (bağ klasörün dışını gösterebilir). \
                     Bağın yerine dosyanın kendisini koyun."
                ));
            }
            if kind.is_dir() {
                let name = relative.rsplit('/').next().unwrap_or(&relative);
                if NOT_READ.contains(&name)
                    || (!relative.contains('/') && matches!(name, "build" | "dist"))
                {
                    not_read.push(format!("{relative}/"));
                } else {
                    stack.push(path);
                }
            } else {
                let data =
                    std::fs::read(&path).map_err(|e| format!("{relative} okunamadı: {e}"))?;
                files.insert(relative, data);
            }
        }
    }
    not_read.sort();
    Ok((files, not_read))
}

fn pack_folder(folder: &Path, out: Option<PathBuf>, json: bool) -> ExitCode {
    let (files, not_read) = match read_folder(folder) {
        Ok(read) => read,
        Err(message) => return fail(message),
    };
    let packed = match pack(&files, &Limits::default()) {
        Ok(packed) => packed,
        Err(problems) => {
            if json {
                let value = serde_json::json!({ "problems": problems });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            } else {
                print_list("Sorunlar", &problems);
                println!("Paketlenmedi: {} sorun.", problems.len());
            }
            return ExitCode::from(1);
        }
    };
    let manifest = files
        .get("manifest.json")
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|t| Manifest::read(t).ok());
    let out = out.unwrap_or_else(|| {
        let name = manifest
            .as_ref()
            .map_or_else(|| "paket".to_owned(), |m| format!("{}-{}", m.id, m.version));
        PathBuf::from(format!("{name}.kek"))
    });
    if let Err(e) = std::fs::write(&out, &packed.bytes) {
        return fail(format!("{} yazılamadı: {e}", out.display()));
    }
    let count = files.len() - packed.left_out.len() + 1;
    let left_out: Vec<String> = not_read.into_iter().chain(packed.left_out).collect();
    if json {
        let value = serde_json::json!({
            "file": out.display().to_string(),
            "bytes": packed.bytes.len(),
            "digest": packed.digest,
            "leftOut": left_out,
            "warnings": packed.warnings,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        println!(
            "Paketlendi: {} ({} bayt, {count} dosya)",
            out.display(),
            packed.bytes.len()
        );
        println!("İçerik özeti: {}", packed.digest);
        if !left_out.is_empty() {
            println!("Pakete girmeyenler: {}", left_out.join(", "));
        }
        print_list("Uyarılar", &packed.warnings);
    }
    ExitCode::SUCCESS
}

/// Writes a catalog folder's `index.json` from its `.kek` files (plan/eklenti.md ADIM 18): every package inspected,
/// none listed that would not install.
fn index_folder(folder: &Path, name: Option<&str>) -> ExitCode {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) => return fail(format!("{} okunamadı: {e}", folder.display())),
    };
    let mut packages = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !file.ends_with(".kek") || !path.is_file() {
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => packages.push((file.to_owned(), bytes)),
            Err(e) => return fail(format!("{} okunamadı: {e}", path.display())),
        }
    }
    packages.sort();
    let title = name.map_or_else(
        || {
            let folder_name = folder
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Eklentiler");
            format!("{folder_name} eklentileri")
        },
        str::to_owned,
    );
    match catalog::index(&title, &packages, &Limits::default()) {
        Ok(index) => {
            let out = folder.join(catalog::FILE);
            if let Err(e) = std::fs::write(&out, index.to_json()) {
                return fail(format!("{} yazılamadı: {e}", out.display()));
            }
            println!(
                "Katalog yazıldı: {} ({} paket)",
                out.display(),
                index.packages.len()
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            print_list("Sorunlar", &problems);
            println!("Katalog yazılmadı: {} sorun.", problems.len());
            ExitCode::from(1)
        }
    }
}

/// Every installed extension with its state, and what the running or ready ones add: the evidence for "where did my
/// tab go?" (plan/eklenti.md ADIM 18, Ö·38). Activates nothing, writes nothing.
fn dump(root: Option<PathBuf>) -> ExitCode {
    let Some(root) = root.or_else(kentos_extension_host::places::user_root) else {
        return fail(
            "Bu bilgisayarda kullanıcı veri klasörü yok; --root ile eklenti klasörünü verin."
                .to_owned(),
        );
    };
    let registry =
        match kentos_extension_host::places::open_to_read(&root, env!("CARGO_PKG_VERSION")) {
            Ok(registry) => registry,
            Err(problem) => return fail(problem.to_string()),
        };
    println!(
        "{}",
        serde_json::to_string_pretty(&registry.dump()).unwrap_or_default()
    );
    ExitCode::SUCCESS
}
