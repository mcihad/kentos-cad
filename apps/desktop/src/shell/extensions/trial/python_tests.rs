//! A trial's commands and map tool run by real Python through `kentos.ext_host` (docs/adr/8057 §52), as the trial
//! runs them, on the synthetic drawing: a small package of the test's own, written to a folder. Without a Python
//! (`KENTOS_TEST_PYTHON`, else the repository's `.run/py`) the test says so and passes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kentos_extensions::manifest::Capability;
use kentos_python_env::process::{Answer, Ended, Heard, Pool, Start};
use serde_json::{Value, json};

use super::broker::{Ground, run};
use super::drawing::synthetic;
use super::steps::{Rights, Spec};

const LOGIC: &str = r#"
from kentos import cad, ext
from kentos import ui_panel as ui


@ext.command("numarala")
def numarala(ctx):
    cevap = ctx.dialog("Numarala", [
        ui.text_field("onek", "Önek", value="Y-"),
        ui.number_field("ilk", "İlk numara", value=1, min=1, required=True),
    ])
    if cevap is None:
        return ctx.message("Vazgeçildi: numara yazılmadı.")
    kayitlar = list(ctx.doc.entities(layer="yapi"))
    satirlar = []
    for n, k in enumerate(kayitlar, start=int(cevap["ilk"])):
        no = f"{cevap['onek']}{n:02d}"
        cad.entities.set(ctx.doc, uids=[k.uid], operation="attributes", attrs={"no": no})
        satirlar.append({"no": no})
    panel = ctx.open_panel("liste")
    panel.rows("sonuc", satirlar)
    panel.say("Numaralandı", "success")
    print("yazıldı:", len(satirlar))
    return ctx.message(f"{len(satirlar)} yapı numaralandı.")


@ext.command("bozuk")
def bozuk(ctx):
    k = next(iter(ctx.doc.entities(layer="yapi")))
    cad.entities.set(ctx.doc, uids=[k.uid], operation="attributes", attrs={"no": "BOZUK"})
    raise ValueError("bilerek")


@ext.map_tool("yerlestir")
def yerlestir(ctx, sekil):
    return ctx.add_feature("yapi", sekil, {"kaynak": "deneme"})
"#;

/// The interpreter: `KENTOS_TEST_PYTHON`, else the repository's; none when neither is there.
fn python() -> Option<PathBuf> {
    let path = std::env::var_os("KENTOS_TEST_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/py/bin/python"));
    path.exists().then_some(path)
}

fn building(x: f64) -> Value {
    json!({ "layer": "yapi", "geometry": { "type": "Polygon",
            "coordinates": [[[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 12.0], [x, 12.0], [x, 0.0]]] } })
}

fn spec(local: &str, map_tool: bool, params: Value, dialogs: Vec<Value>) -> Spec {
    Spec {
        id: format!("ext.tr.ornek.deneme.{local}"),
        map_tool,
        title: local.to_owned(),
        params,
        panel: None,
        selection: Vec::new(),
        dialogs,
    }
}

/// A command asks its dialog and is answered with the agent's values, writes the synthetic drawing in one step, opens
/// its panel and fills it, and prints; one that fails after writing has its write taken back and its traceback said;
/// a map tool writes the shape it is given.
#[test]
fn a_trials_commands_and_map_tool_run_in_real_python() {
    let Some(python) = python() else {
        eprintln!("Python yok: denemenin komut sınaması atlandı.");
        return;
    };
    let dir = std::env::temp_dir().join(format!("kentos-deneme-komut-{}", std::process::id()));
    let package = dir.join("src/deneme_paket");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("__init__.py"), "").unwrap();
    std::fs::write(package.join("logic.py"), LOGIC).unwrap();
    let pool = Arc::new(Pool::new(Duration::from_secs(60)));
    let start = Start {
        python,
        args: vec!["-I".into(), "-m".into(), "kentos.ext_host".into()],
        hello: json!({ "type": "hello", "protocol": 1, "extension": "tr.ornek.deneme", "version": "1.0.0",
                       "digest": "sha256:deneme", "source": dir.display().to_string(), "paths": ["src"] }),
    };
    let panel = kentos_extensions::ui_panel::read(&json!({
        "format": kentos_extensions::ui_panel::FORMAT, "schemaVersion": kentos_extensions::ui_panel::VERSION,
        "title": "Numaralar", "body": [{ "type": "table", "id": "sonuc", "columns": ["no"] }]
    }));
    let rights = Rights {
        ext: "tr.ornek.deneme".into(),
        capabilities: vec![Capability::DocumentWrite],
        depends: Vec::new(),
        panels: BTreeMap::from([("liste".to_owned(), panel)]),
    };
    let mut made = synthetic(&[building(0.0), building(50.0)]).expect("a drawing");
    let places: BTreeMap<String, usize> = made
        .uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.to_string(), i))
        .collect();
    let mut panels = BTreeMap::new();
    let mut step = |spec: &Spec, handler: &str| {
        let ground = Ground {
            doc: &mut made.doc,
            rights: &rights,
            panels: &mut panels,
            places: &places,
            uids: &made.uids,
        };
        let local = spec.id.rsplit('.').next().unwrap_or_default().to_owned();
        run(
            ground,
            spec,
            &mut |mut invoke: Value,
                  document: bool,
                  answer: &mut Answer<'_>,
                  heard: &mut dyn FnMut(Heard)| {
                invoke["type"] = json!("invoke");
                invoke["local"] = json!(local);
                invoke["handler"] = json!(handler);
                invoke["document"] = json!(document);
                match pool.run_answering(
                    "tr.ornek.deneme@1.0.0#deneme",
                    &|| Ok(start.clone()),
                    invoke,
                    Duration::from_secs(60),
                    &|| false,
                    heard,
                    answer,
                ) {
                    Ended::Answered(done) => Ok(done),
                    other => Err(format!("{other:?}")),
                }
            },
        )
    };
    // The dialog answered B-, 5: both buildings numbered, the panel filled, the print kept.
    let numbered = step(
        &spec(
            "numarala",
            false,
            json!({}),
            vec![json!({ "onek": "B-", "ilk": 5 })],
        ),
        "deneme_paket.logic:numarala",
    );
    assert_eq!(numbered["status"], "done", "{numbered}");
    assert_eq!(numbered["message"], "2 yapı numaralandı.");
    assert_eq!(numbered["changed"]["objects"], 2, "{numbered}");
    assert_eq!(numbered["changed"]["sample"][0]["attrs"]["no"], "B-05");
    assert_eq!(numbered["dialogs"][0]["answer"]["onek"], "B-");
    assert_eq!(
        numbered["panels"][0]["tables"]["sonuc"]["rows"], 2,
        "{numbered}"
    );
    assert_eq!(numbered["panels"][0]["said"]["text"], "Numaralandı");
    assert!(
        numbered["output"]
            .as_str()
            .is_some_and(|o| o.contains("yazıldı: 2")),
        "{numbered}"
    );
    // Not answered: İptal, nothing written.
    let cancelled = step(
        &spec("numarala", false, json!({}), Vec::new()),
        "deneme_paket.logic:numarala",
    );
    assert_eq!(
        cancelled["message"], "Vazgeçildi: numara yazılmadı.",
        "{cancelled}"
    );
    assert!(cancelled.get("changed").is_none());
    // It fails after writing: taken back, its traceback said.
    let broken = step(
        &spec("bozuk", false, json!({}), Vec::new()),
        "deneme_paket.logic:bozuk",
    );
    assert_eq!(broken["status"], "failed", "{broken}");
    assert_eq!(broken["undone"], true);
    assert!(
        broken["traceback"]
            .as_str()
            .is_some_and(|t| t.contains("bilerek")),
        "{broken}"
    );
    assert!(broken.get("changed").is_none(), "{broken}");
    // The map tool writes the rectangle it is given.
    let square = json!({ "type": "Polygon", "coordinates": [[[100.0, 100.0], [110.0, 100.0], [110.0, 112.0],
                                                             [100.0, 112.0], [100.0, 100.0]]] });
    let placed = step(
        &spec("yerlestir", true, json!({ "shape": square }), Vec::new()),
        "deneme_paket.logic:yerlestir",
    );
    assert_eq!(placed["status"], "done", "{placed}");
    assert_eq!(placed["written"][0]["objects"], 1, "{placed}");
    assert_eq!(
        placed["written"][0]["sample"][0]["attrs"]["kaynak"],
        "deneme"
    );
    pool.close("tr.ornek.deneme@1.0.0#deneme");
    let _ = std::fs::remove_dir_all(&dir);
}
