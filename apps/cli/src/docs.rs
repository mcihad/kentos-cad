//! `kentos kek docs <klasör>` (plan/eklenti.md ADIM 21, EKL-06: "sürümlü Python API referansı … çevrimdışı belge
//! paketi"; docs/adr/8057 §101): this KentOS's extension SDK written as files, for a person to read and an agent to
//! take without a network: the guide and the Python API (`llms.txt`, `kentos_ext.pyi`), every schema, the verification
//! codes, the capabilities and contributions, the workflow, the icons, a whole example extension, and a page that says
//! what is where and for which version. The same pack `kek sdk` and the MCP tool give, from the same code: the
//! bundle cannot drift from the program that wrote it.

use std::path::Path;
use std::process::ExitCode;

use kentos_extensions::compat::SDK_PROTOCOL;
use kentos_extensions::sdk_pack::{EXT_STUB, GUIDE, sdk_part};
use serde_json::Value;

/// The file a bundle carries its version in: a folder holding it is a bundle, written over again.
const VERSION_FILE: &str = "surum.json";

/// The parts written as their own files: the part, the file, its line on the index page.
const FILES: &[(&str, &str, &str)] = &[
    (
        "manifestSchema",
        "semalar/manifest.schema.json",
        "manifest.json'un JSON Schema'sı",
    ),
    (
        "panelSchema",
        "semalar/panel.schema.json",
        "panel tanımının şeması (kentos.ui.panel)",
    ),
    (
        "layerSchema",
        "semalar/katman-semasi.schema.json",
        "katman şeması (kentos.layerSchema)",
    ),
    (
        "toolSchema",
        "semalar/islem-araci.schema.json",
        "İşlemler aracı tanımı (kentos.processingTool)",
    ),
    (
        "factsSchema",
        "semalar/olgular.schema.json",
        "denetim kurallarının okuduğu olgular",
    ),
    (
        "auditExamplesSchema",
        "semalar/denetim-ornekleri.schema.json",
        "denetim kuralı örnek dosyası",
    ),
    (
        "capabilities",
        "yetenekler.json",
        "istenebilen yetenekler ve anlamları",
    ),
    (
        "modules",
        "programlar.json",
        "KentOS programları (cbs, harita, mimari, yol)",
    ),
    (
        "contributions",
        "katkilar.json",
        "katkı türleri ve giriş noktaları",
    ),
    (
        "toolDescriptor",
        "islem-araci.json",
        "İşlemler aracı tanımının kısa anlatımı",
    ),
    (
        "verifyCodes",
        "dogrulama-kodlari.json",
        "`kentos kek verify`'ın kodları ve anlamları",
    ),
    (
        "panelBehaviour",
        "panel-davranisi.json",
        "panel bileşenlerinin davranış tablosu",
    ),
    (
        "panelBuilders",
        "panel-kuruculari.py",
        "panel bileşenlerinin Python kurucuları",
    ),
];

/// Writes the bundle into `folder` (made when missing); a folder that holds something else is refused, a bundle is
/// written over.
pub fn run(folder: &Path) -> ExitCode {
    match write(folder) {
        Ok(count) => {
            say!(
                "SDK belge paketi yazıldı: {} ({count} dosya; KentOS {}, SDK protokolü {SDK_PROTOCOL}). Başlangıç: \
                 BENIOKU.md; yapay zekâ ajanları için llms.txt.",
                folder.display(),
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::from(2)
        }
    }
}

/// The bundle's files, written; how many.
pub(crate) fn write(folder: &Path) -> Result<usize, String> {
    if folder.is_dir()
        && !folder.join(VERSION_FILE).is_file()
        && std::fs::read_dir(folder).is_ok_and(|mut e| e.next().is_some())
    {
        return Err(format!(
            "{} boş değil ve bir belge paketi de değil: boş ya da yeni bir klasör verin.",
            folder.display()
        ));
    }
    let version = env!("CARGO_PKG_VERSION");
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut put = |path: &str, bytes: Vec<u8>| files.push((path.to_owned(), bytes));
    put(
        "llms.txt",
        format!("{GUIDE}\n## kentos.ext\n\n```python\n{EXT_STUB}```\n").into_bytes(),
    );
    put("kentos_ext.pyi", EXT_STUB.as_bytes().to_vec());
    put(
        "sdk.json",
        pretty(&kentos_extensions::sdk_pack::sdk_pack()).into_bytes(),
    );
    for (part, path, _) in FILES {
        let Some(value) = sdk_part(part) else {
            continue;
        };
        let bytes = match value {
            Value::String(text) => text.into_bytes(),
            other => pretty(&other).into_bytes(),
        };
        put(path, bytes);
    }
    if let Some(Value::Array(steps)) = sdk_part("workflow") {
        let lines: Vec<String> = steps
            .iter()
            .filter_map(Value::as_str)
            .enumerate()
            .map(|(i, step)| format!("{}. {step}", i + 1))
            .collect();
        put(
            "is-akisi.md",
            format!("# Eklenti yazma iş akışı\n\n{}\n", lines.join("\n")).into_bytes(),
        );
    }
    if let Some(Value::Array(icons)) = sdk_part("icons") {
        let names: Vec<&str> = icons.iter().filter_map(Value::as_str).collect();
        put(
            "ikonlar.txt",
            format!("{}\n", names.join("\n")).into_bytes(),
        );
    }
    if let Some(Value::Object(example)) = sdk_part("example") {
        for (path, text) in &example {
            if let Some(text) = text.as_str().filter(|_| safe(path)) {
                put(&format!("ornek-eklenti/{path}"), text.as_bytes().to_vec());
            }
        }
    }
    put("BENIOKU.md", index(version).into_bytes());
    put(
        VERSION_FILE,
        pretty(&serde_json::json!({
            "format": "kentos.sdk-docs",
            "kentos": version,
            "sdkProtocol": SDK_PROTOCOL,
        }))
        .into_bytes(),
    );
    for (path, bytes) in &files {
        let to = folder.join(path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("{} açılamadı: {e}", parent.display()))?;
        }
        std::fs::write(&to, bytes).map_err(|e| format!("{} yazılamadı: {e}", to.display()))?;
    }
    Ok(files.len())
}

/// A path of the example that stays in its folder: relative, no `..`.
fn safe(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path.split(['/', '\\']).all(|p| !p.is_empty() && p != "..")
}

fn pretty(value: &Value) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(value).unwrap_or_default()
    )
}

/// The index page: the version, what each file is, and how an agent takes it.
fn index(version: &str) -> String {
    let mut lines = vec![
        format!("# KentOS eklenti SDK'sı {version}"),
        String::new(),
        format!(
            "Bu klasör KentOS {version} sürümünün eklenti SDK'sıdır (SDK protokolü {SDK_PROTOCOL}): ağsız okunur. \
             Başka bir KentOS sürümünde ayrıntılar farklı olabilir; o sürümün `kentos kek docs <klasör>` komutuyla \
             yeniden üretin. Aynı içeriği `kentos kek sdk` ve MCP ajanlarının SDK aracı da verir."
        ),
        String::new(),
        "## Başlangıç".to_owned(),
        String::new(),
        "- `llms.txt`: rehber ve `kentos.ext` imzaları tek dosyada; bir yapay zekâ ajanına önce bunu verin.".to_owned(),
        "- `kentos_ext.pyi`: Python API'sinin imzaları (düzenleyicinin tamamlaması için).".to_owned(),
        "- `is-akisi.md`: eklentiyi istekten yayına götüren adımlar.".to_owned(),
        "- `ornek-eklenti/`: her katkı türünün küçük, çalışan örneği; `kentos kek verify ornek-eklenti` ile \
         doğrulanır."
            .to_owned(),
        "- `sdk.json`: paketin tamamı JSON olarak (araçlar için).".to_owned(),
        String::new(),
        "## Başvuru".to_owned(),
        String::new(),
    ];
    for (_, path, words) in FILES {
        lines.push(format!("- `{path}`: {words}."));
    }
    lines.push(
        "- `ikonlar.txt`: manifestte ve panellerde kullanılabilen ikonların adları.".to_owned(),
    );
    lines.push(format!(
        "- `{VERSION_FILE}`: bu paketin KentOS sürümü ve SDK protokolü."
    ));
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every part is written as its own file, the schemas and tables as JSON that reads, the example extension
    /// verifies, the index says the version; a bundle is written over, a folder of other files is refused.
    #[test]
    fn the_bundle_is_the_sdk_written_as_files() {
        let dir = std::env::temp_dir().join(format!("kentos-sdk-belgeleri-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let count = write(&dir).unwrap();
        assert!(count > 20, "{count}");
        let read = |p: &str| std::fs::read_to_string(dir.join(p)).unwrap();
        assert!(read("llms.txt").starts_with("# KentOS eklentisi yazmak"));
        assert_eq!(read("kentos_ext.pyi"), EXT_STUB);
        for (_, path, _) in FILES {
            if path.ends_with(".json") {
                serde_json::from_str::<Value>(&read(path))
                    .unwrap_or_else(|e| panic!("{path}: {e}"));
            }
        }
        let version: Value = serde_json::from_str(&read(VERSION_FILE)).unwrap();
        assert_eq!(version["kentos"], env!("CARGO_PKG_VERSION"));
        assert!(read("BENIOKU.md").contains(&format!("KentOS {}", env!("CARGO_PKG_VERSION"))));
        assert!(read("is-akisi.md").contains("1. "));
        // The example extension is whole: it passes the same verification an author's folder does.
        let (files, _) = crate::read_folder(&dir.join("ornek-eklenti")).unwrap();
        let verified = kentos_extensions::verify(&files, &kentos_extensions::Limits::default());
        assert!(verified.problems.is_empty(), "{:#?}", verified.problems);
        // Written over as a bundle; a folder of something else is refused.
        assert_eq!(write(&dir).unwrap(), count);
        let other = dir.join("ornek-eklenti");
        assert!(write(&other).unwrap_err().contains("boş değil"));
        assert!(!safe("../disari.py") && !safe("/kok") && !safe("C:/x") && safe("src/a.py"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
