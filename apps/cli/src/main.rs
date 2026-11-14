//! `kentos`: KentOS's command line (plan/eklenti.md ADIM 16, D2). Its first commands are the extension package's:
//!
//! ```text
//! kentos kek inspect <paket.kek> [--json]
//! kentos kek pack <klasör> [-o <paket.kek>] [--json]
//! kentos kek index <klasör> [--name <ad>]
//! kentos kek dump [--root <klasör>]
//! kentos kek log [<kimlik>] [--son <n>] [--root <klasör>] [--json]
//! kentos kek sdk [--part <bölüm>] [--json]
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
  kentos kek verify <klasör> [--json]               klasörü pencere açmadan doğrular: paket, Python işaretleri,
                                                    satırlı rapor (yapay zekâ ajanlarının tek adımda düzelteceği)
  kentos kek new <klasör> --id <kimlik> --name <ad> [--summary <cümle>] [--module cbs|harita|mimari|yol]
                 [--utility] [--with command,processing,audit,expression,mapTool,schema]
                                                    doğrulamadan geçen bir eklenti iskeleti yazar
  kentos kek sdk [--part <bölüm>] [--json]          eklenti yazma rehberi ve kentos.ext imzaları (--json: paketin
                                                    tamamı, ajanların kentos://sdk/ai-kek-pack'i)
  kentos kek index <klasör> [--name <ad>]           klasördeki paketlerin katalog dizinini (index.json) yazar
  kentos kek dump [--root <klasör>]                 kurulu eklentileri ve katkılarını JSON olarak döker
  kentos kek log [<kimlik>] [--son <n>] [--root <klasör>] [--json]
                                                    bir eklentinin günlüğü (kimliksiz: kayıt defterininki)";

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
        ["kek", "verify", folder] => verify_folder(Path::new(folder), json),
        ["kek", "new", folder, rest @ ..] => new_folder(Path::new(folder), rest),
        ["kek", "sdk"] => sdk(None, json),
        ["kek", "sdk", "--part", part] => sdk(Some(part), json),
        ["kek", "pack", folder, "-o", out] => {
            pack_folder(Path::new(folder), Some(PathBuf::from(out)), json)
        }
        ["kek", "dump"] => dump(None),
        ["kek", "dump", "--root", root] => dump(Some(PathBuf::from(root))),
        ["kek", "index", folder] => index_folder(Path::new(folder), None),
        ["kek", "index", folder, "--name", name] => index_folder(Path::new(folder), Some(name)),
        ["kek", "log", rest @ ..] => log(rest, json),
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

/// `kek sdk` (plan/eklenti.md ADIM 21, docs/adr/8057 §20): the guide and `kentos.ext`'s signatures to read, or the
/// pack (`--json`) or one part of it as an agent reads it.
fn sdk(part: Option<&str>, json: bool) -> ExitCode {
    use kentos_extensions::sdk_pack::{EXT_STUB, GUIDE, PARTS, sdk_part};
    let text = match part {
        None if json => {
            serde_json::to_string_pretty(&kentos_extensions::sdk_pack()).unwrap_or_default()
        }
        None => format!("{GUIDE}\n## kentos.ext\n\n```python\n{EXT_STUB}```\n"),
        Some(part) => match sdk_part(part) {
            Some(serde_json::Value::String(text)) if !json => text,
            Some(serde_json::Value::Array(names)) if part == "icons" && !json => names
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
            Some(serde_json::Value::Object(stub)) if part == "api" && !json => stub
                .get("stub")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            Some(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
            None => {
                eprintln!("“{part}” paketin bir bölümü değil: {}.", PARTS.join(", "));
                return ExitCode::from(2);
            }
        },
    };
    println!("{text}");
    ExitCode::SUCCESS
}

/// `kek new` (plan/eklenti.md ADIM 21, docs/adr/8057 §19): a new extension's working folder from
/// `kentos_extensions::blueprint`, written into an empty or new folder; nothing is overwritten.
fn new_folder(folder: &Path, args: &[&str]) -> ExitCode {
    use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
    use kentos_extensions::manifest::Module;
    let mut flags: BTreeMap<&str, &str> = BTreeMap::new();
    let mut utility = false;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--utility" => utility = true,
            flag @ ("--id" | "--name" | "--summary" | "--module" | "--with")
                if i + 1 < args.len() =>
            {
                flags.insert(flag, args[i + 1]);
                i += 1;
            }
            other => return fail(format!("“{other}” bilinmiyor.\n{USAGE}")),
        }
        i += 1;
    }
    let (Some(id), Some(name)) = (flags.get("--id"), flags.get("--name")) else {
        return fail(format!("--id ve --name gerekli.\n{USAGE}"));
    };
    let module = match flags.get("--module").copied().unwrap_or("cbs") {
        "cbs" => Module::Cbs,
        "harita" => Module::Harita,
        "mimari" => Module::Mimari,
        "yol" => Module::Yol,
        other => {
            return fail(format!(
                "“{other}” bir program değil: cbs, harita, mimari ya da yol."
            ));
        }
    };
    let mut contributions = Vec::new();
    for word in flags.get("--with").copied().unwrap_or("command").split(',') {
        match Contribution::parse(word.trim()) {
            Some(c) => contributions.push(c),
            None => {
                return fail(format!(
                    "“{word}” bir katkı değil: command, processing, audit, expression, mapTool, schema."
                ));
            }
        }
    }
    let ask = Ask {
        id: (*id).to_owned(),
        name: (*name).to_owned(),
        summary: flags
            .get("--summary")
            .map_or_else(|| format!("{name} eklentisi."), |s| (*s).to_owned()),
        vertical: !utility,
        module,
        contributions,
    };
    let files = match blueprint(&ask) {
        Ok(files) => files,
        Err(why) => return fail(why),
    };
    if folder.exists() && std::fs::read_dir(folder).is_ok_and(|mut d| d.next().is_some()) {
        return fail(format!(
            "{} boş değil: iskelet yalnız boş ya da yeni bir klasöre yazılır.",
            folder.display()
        ));
    }
    for (path, text) in &files {
        let to = folder.join(path);
        if let Some(parent) = to.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return fail(format!("{} yazılamadı: {e}", parent.display()));
        }
        if let Err(e) = std::fs::write(&to, text) {
            return fail(format!("{} yazılamadı: {e}", to.display()));
        }
    }
    println!(
        "{} dosya yazıldı: {}. Sıradaki: örnekleri değiştirin, `kentos kek verify {}` ile doğrulayın, `kentos kek pack` \
         ile paketleyin.",
        files.len(),
        folder.display(),
        folder.display()
    );
    ExitCode::SUCCESS
}

/// `kek verify` (plan/eklenti.md ADIM 21, docs/adr/8057 §18): the folder through the package's gates and its Python's
/// marks, each finding with its file, its line or pointer and what to write; nothing is packed or run.
fn verify_folder(folder: &Path, json: bool) -> ExitCode {
    let (files, _) = match read_folder(folder) {
        Ok(read) => read,
        Err(message) => return fail(message),
    };
    let v = kentos_extensions::verify(&files, &Limits::default());
    if json {
        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
    } else {
        if let Some(m) = &v.manifest {
            println!("{} {} — {}", m.id, m.version, m.name);
        }
        let line = |p: &Problem| {
            let place = [p.file.as_deref(), p.at.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            if place.is_empty() {
                format!("  • {} ({})", p.message, p.code)
            } else {
                format!("  • {place}: {} ({})", p.message, p.code)
            }
        };
        for (title, list) in [("Sorunlar", &v.problems), ("Uyarılar", &v.warnings)] {
            if !list.is_empty() {
                println!("{title}:");
                for p in list.iter() {
                    println!("{}", line(p));
                }
            }
        }
        if v.is_valid() {
            println!(
                "Doğrulandı: {} uyarı. İçerik özeti {}.",
                v.warnings.len(),
                v.digest.as_deref().unwrap_or("-")
            );
        } else {
            println!("Doğrulanmadı: {} sorun.", v.problems.len());
        }
    }
    if v.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
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

/// An extension's log, or the registry's without an id (docs/adr/8057 §4): its newest records, oldest first, each with
/// its time (UTC), level, code, part and version, the error as it came under it; the error still standing at the end.
/// `--json` gives the records as JSON Lines. Reads only.
fn log(args: &[&str], json: bool) -> ExitCode {
    let mut id: Option<&str> = None;
    let mut count = 50usize;
    let mut root: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match (args[i], args.get(i + 1)) {
            ("--son", Some(n)) => {
                let Ok(n) = n.parse() else {
                    return fail(USAGE.to_owned());
                };
                count = n;
                i += 2;
            }
            ("--root", Some(r)) => {
                root = Some(PathBuf::from(r));
                i += 2;
            }
            (a, _) if !a.starts_with('-') && id.is_none() => {
                id = Some(a);
                i += 1;
            }
            _ => return fail(USAGE.to_owned()),
        }
    }
    let Some(root) = root.or_else(kentos_extension_host::places::user_root) else {
        return fail(
            "Bu bilgisayarda kullanıcı veri klasörü yok; --root ile eklenti klasörünü verin."
                .to_owned(),
        );
    };
    // Opening the registry carries the short history it kept before the log into the log, and says an unreadable
    // registry there; what it says otherwise is the log's to show.
    let _ = kentos_extension_host::places::open_to_read(&root, env!("CARGO_PKG_VERSION"));
    let journal = kentos_extension_host::Journal::new(&root);
    let records = journal.tail(id, count);
    if json {
        for r in &records {
            println!("{}", serde_json::to_string(r).unwrap_or_default());
        }
        return ExitCode::SUCCESS;
    }
    if records.is_empty() {
        println!("Günlükte kayıt yok ({}).", journal.file(id).display());
        return ExitCode::SUCCESS;
    }
    for r in &records {
        let part = r
            .part
            .as_ref()
            .map(|p| format!(" · {p}"))
            .unwrap_or_default();
        let version = r
            .version
            .as_ref()
            .map(|v| format!(" · {v}"))
            .unwrap_or_default();
        println!(
            "{} {:<5} {}{part}{version} — {}",
            utc(r.at),
            r.level.label(),
            r.code,
            r.message
        );
        if let Some(detail) = &r.detail {
            for line in detail.lines() {
                println!("    {line}");
            }
        }
    }
    if let Some(id) = id
        && let Some(p) = kentos_extension_host::journal::problem_in(&records)
    {
        println!("\nDuran hata ({id}): {} — {}", utc(p.at), p.message);
    }
    ExitCode::SUCCESS
}

/// Milliseconds since 1970 as a UTC time: `2026-11-13 18:42:07Z`.
fn utc(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Days since 1970 to a civil date (Howard Hinnant's days_from_civil, inverted).
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_log_time_is_written_in_utc() {
        assert_eq!(super::utc(0), "1970-01-01 00:00:00Z");
        // 2026-11-13T15:42:07.5Z
        assert_eq!(super::utc(1_794_584_527_500), "2026-11-13 15:42:07Z");
        assert_eq!(super::utc(951_782_400_000), "2000-02-29 00:00:00Z");
    }
}
