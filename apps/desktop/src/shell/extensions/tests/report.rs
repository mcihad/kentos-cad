//! An extension's debugging report (report.rs, docs/adr/8057 §110, ECO-API-14): the environment, the sources'
//! versions, the schemas and the problem's code with its meaning; no key and no home folder in it; read by an agent
//! with `extension.report` through Asistan's door.

use kentos_headless::agent_tools::{self, Guard};
use serde_json::json;

use super::demo::vitrin_package;
use super::*;

const VITRIN: &str = "tr.ornek.vitrin";

#[test]
fn the_report_says_the_environment_and_hides_the_keys() {
    let root = temp("rapor-vitrin");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    // A failure whose words carry a key and whose traceback the user's home folder, as a request's may.
    let key = format!("{}{}", "sk-p", "roj-") + &"aB3dE5gH7jK9mN2pQ4sT6vW8yZ0cF1x".repeat(2);
    let home = std::env::home_dir().expect("a home folder");
    let traceback = format!(
        "Traceback (most recent call last):\n  File \"{}/eklentiler/vitrin/src/vitrin/logic.py\", line 12, in fetch\n\
         RuntimeError: 401",
        home.display()
    );
    gis.extension_failed(
        VITRIN,
        "veri kaynağı “Örnek Belediyesi altlığı”",
        "source_unreachable",
        &format!("Sunucu anahtarı reddetti: {key}"),
        Some(&traceback),
    );
    let report = gis.extension_report(VITRIN).expect("a report");
    for part in [
        "# Eklenti raporu: Eklenti Vitrini",
        "- Paket özeti: `sha256:",
        "- Manifest: `kentos.extension` şema",
        "## Ortam",
        "- Python gereksinimleri: shapely>=2.0,<2.1, numpy>=2.0,<3",
        "- Yetenekler: 12 istendi, 12 verildi",
        "- uv: ",
        "Kod `source_unreachable`: Veritabanına ulaşılamadı",
        "‹OpenAI API anahtarı gizlendi›",
        "File \"~/eklentiler/vitrin/src/vitrin/logic.py\"",
    ] {
        assert!(report.contains(part), "{part}:\n{report}");
    }
    assert!(!report.contains(&key), "{report}");
    assert!(!report.contains(home.to_str().unwrap()), "{report}");
    // An agent reads it through Asistan's door, as the manager copies it.
    let mut guard = Guard::new();
    let mut host = crate::assistant::host::AppHost(&mut gis);
    let read = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.report",
        &json!({ "id": VITRIN }),
    )
    .expect("the report");
    assert_eq!(read["report"].as_str(), Some(report.as_str()));
    let unknown = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.report",
        &json!({ "id": "tr.ornek.yok" }),
    );
    assert_eq!(unknown.err().map(|e| e.code), Some("unknown_extension"));
    let _ = std::fs::remove_dir_all(&root);
}
