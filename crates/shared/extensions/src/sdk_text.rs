//! The SDK pack as one plain text file (plan/eklenti.md ADIM 21, "düz metin SDK belgesi (`llms.txt`)"; docs/adr/8057
//! §77): what an agent or a person reads in one go where JSON is not the reader's form, a coding agent in an editor,
//! a model's context, a search over documents. Made from the pack (`sdk_pack`), never written by hand: the guide, the
//! steps, the capabilities, the contributions, the tool descriptor's vocabulary, how each part is asked for and
//! `kentos.ext`'s signatures. Committed beside the stub (`sdk/llms.txt`): its test fails when the pack changed and the
//! file was not written again (`KENTOS_WRITE_SDK=1`).
//!
//! The form is the `llms.txt` proposal's: a title, a one-paragraph summary as a quote, then sections. KentOS has no
//! site to link to, so the sections hold the text itself; a long list (the icons) or a schema stays a part of its own.

use std::fmt::Write as _;

use serde_json::Value;

/// The pack's text, as `kentos kek sdk --text`, MCP's `kentos://sdk/llms.txt` and the committed `sdk/llms.txt` give it.
pub fn llms_txt() -> String {
    let pack = crate::sdk_pack();
    let mut out = String::new();
    let _ = writeln!(out, "# KentOS eklenti SDK'sı\n");
    let _ = writeln!(
        out,
        "> KentOS'un `.kek` eklentilerini yazmak için gereken her şey tek dosyada: rehber, adımlar, yetenekler, \
         katkılar, İşlemler aracı tanımı ve `kentos.ext` imzaları. Bu metin `kentos://sdk/ai-kek-pack`'in (MCP), \
         `extension.sdk`'nın (ajan aracı) ve `kentos kek sdk --json`'un düz metin hâlidir ve onlardan üretilir; \
         şemalar ve uzun listeler kendi bölümlerinden alınır (\"Bölümler\").\n"
    );
    let _ = writeln!(
        out,
        "SDK protokolü {} · paket biçimi {} sürüm {}\n",
        crate::SDK_PROTOCOL,
        text(&pack["format"]),
        pack["version"]
    );
    // The guide is Markdown already, its first heading the pack's: its headings one level down under this file's
    // title; a code block's lines (a Python comment starts with `#` too) as they are.
    let mut code = false;
    for line in text(&pack["guide"]).lines() {
        if line.starts_with("```") {
            code = !code;
        }
        let heading =
            !code && line.starts_with('#') && line.trim_start_matches('#').starts_with(' ');
        if heading {
            let _ = writeln!(out, "#{line}");
        } else {
            let _ = writeln!(out, "{line}");
        }
    }
    let _ = writeln!(out, "\n## Adımlar\n");
    for (i, step) in list(&pack["workflow"]).iter().enumerate() {
        let _ = writeln!(out, "{}. {}", i + 1, text(step));
    }
    let _ = writeln!(out, "\n## Yetenekler\n");
    for c in list(&pack["capabilities"]) {
        let scopes: Vec<&str> = list(&c["scopes"]).iter().map(text).collect();
        let _ = writeln!(
            out,
            "- `{}`: {} (kapsam: {})",
            text(&c["id"]),
            text(&c["title"]),
            scopes.join(", ")
        );
    }
    let _ = writeln!(out, "\n## Katkılar\n");
    for c in list(&pack["contributions"]) {
        let mut said = format!(
            "- **{}**: manifestte `{}`",
            text(&c["kind"]),
            text(&c["manifest"])
        );
        if let Some(mark) = c["decorator"].as_str() {
            let _ = write!(said, ", işareti `{mark}`");
        }
        for key in ["descriptor", "file", "examples"] {
            if let Some(path) = c[key].as_str() {
                let _ = write!(said, ", `{key}`: `{path}`");
            }
        }
        let wants: Vec<&str> = list(&c["capabilities"]).iter().map(text).collect();
        if !wants.is_empty() {
            let _ = write!(said, "; ister: {}", wants.join(", "));
        }
        if let Some(note) = c["note"].as_str() {
            let _ = write!(said, ". {note}");
        }
        let _ = writeln!(out, "{said}");
    }
    let tool = &pack["toolDescriptor"];
    let _ = writeln!(
        out,
        "\n## İşlemler aracı tanımı (`{}` {})\n",
        text(&tool["format"]),
        tool["schemaVersion"]
    );
    let _ = writeln!(out, "- Alanlar: {}", text(&tool["fields"]));
    for (title, key) in [
        ("Parametre türleri", "parameterTypes"),
        ("Çıktı türleri", "outputTypes"),
    ] {
        let kinds: Vec<String> = list(&tool[key])
            .iter()
            .map(|k| format!("`{}`", text(k)))
            .collect();
        let _ = writeln!(out, "- {title}: {}", kinds.join(", "));
    }
    let _ = writeln!(out, "- Notlar: {}", text(&tool["notes"]));
    let _ = writeln!(out, "\n## Bölümler\n");
    let _ = writeln!(
        out,
        "Her bölüm tek başına alınır: `extension.sdk` ile `part`, ya da `kentos kek sdk --part <bölüm>`.\n"
    );
    if let Some(parts) = pack.as_object() {
        for (name, value) in parts {
            if let Some(how) = value.get("how").and_then(Value::as_str) {
                let _ = writeln!(out, "- `{name}`: {how}");
            }
        }
    }
    let _ = writeln!(out, "\n## `{}` imzaları\n", text(&pack["api"]["module"]));
    let _ = writeln!(
        out,
        "```python\n{}\n```",
        text(&pack["api"]["stub"]).trim_end()
    );
    out
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed `sdk/llms.txt` is the pack's text: written by `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions
    /// --lib -- sdk_pack` with the SDK's other files (this test's name holds `sdk_pack`). It holds the guide, every
    /// capability, contribution and part, and the signatures, and is the llms.txt form: a title, then a quote.
    #[test]
    fn the_sdk_pack_as_llms_txt_is_committed_and_whole() {
        let made = llms_txt();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(root.join("sdk/llms.txt"), &made).expect("the text");
            return;
        }
        assert!(
            include_str!("../sdk/llms.txt") == made,
            "the SDK pack changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
        );
        let mut lines = made.lines();
        assert_eq!(lines.next(), Some("# KentOS eklenti SDK'sı"));
        assert_eq!(lines.next(), Some(""));
        assert!(lines.next().is_some_and(|l| l.starts_with("> ")));
        let pack = crate::sdk_pack();
        for c in list(&pack["capabilities"]) {
            assert!(
                made.contains(&format!("`{}`", text(&c["id"]))),
                "{}",
                c["id"]
            );
        }
        for c in list(&pack["contributions"]) {
            assert!(
                made.contains(&format!("**{}**", text(&c["kind"]))),
                "{}",
                c["kind"]
            );
        }
        for part in crate::sdk_pack::PARTS {
            if pack[part].get("how").is_some() {
                assert!(made.contains(&format!("- `{part}`: ")), "{part}");
            }
        }
        assert!(made.contains("def command(local: str)"), "the signatures");
        assert!(
            made.contains("## Kurallar"),
            "the guide's sections, one level down"
        );
        assert!(!made.contains("\n# KentOS eklentisi"), "one title only");
    }
}
