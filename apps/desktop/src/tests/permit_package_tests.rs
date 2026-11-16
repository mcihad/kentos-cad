//! 2B ruhsat paketi (docs/adr/8066; plan/bim-plan.md BIM-05): a permit set's publication passes its gate when its files are
//! PDF/A, one a sheet, on A4 multiples and named by EK-2, and writes the delivery list beside them; a wrong name, one file
//! for all and PDF/A off shut the gate, each said. By hand: an independent reading (qpdf, Poppler) of the section
//! building's and the 2+1 flat's packages, and of files broken on purpose.

use std::path::{Path, PathBuf};

use crate::app::{App, Message};
use crate::exchange::Window;
use crate::exchange::permit_set::Event as Permit;
use crate::exchange::sheet_publish::Event as Publish;
use kentos_commands::Entity;

/// The particulars a set asks without defaults, given.
const PARTICULARS: [(&str, &str); 11] = [
    ("il", "Sivas"),
    ("ilce", "Merkez"),
    ("mahalle", "Yenişehir"),
    ("ada", "101"),
    ("parsel", "7"),
    ("yapi_sahibi", "Ayşe Demir"),
    ("tasiyici_sistem", "Betonarme karkas"),
    ("mimar", "Deniz Kaya"),
    ("sicil_no", "12345"),
    ("btb_no", "6789"),
    ("buro_adres", "İstasyon Cad. 1, Sivas"),
];

fn evidence(case: &str) -> PathBuf {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-paket-8066").join(case);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.parent().expect("a parent")).expect("a folder");
    out
}

pub(crate) fn section_building() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad"))
        .expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let b = app
        .document
        .as_ref()
        .unwrap()
        .model
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::ViewMark(v) if v.mark.mark == "B").then(|| u.to_string()))
        .expect("the B mark");
    let doc = &mut app.document.as_mut().unwrap().model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.sections.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"operation": {"kind": "derive", "mark": b, "at": {"x": 40.0, "y": 0.0}}}),
    )
    .expect("derived");
    assert_eq!(answer["status"], "completed");
    app
}

/// Ruhsat takımı with its particulars given, the set made and its publication's check finished, into `path`. The
/// parcel's İmar durumu given first (the first sheet's table reads its area), as Mimari › Parsel would.
pub(crate) fn permit_publication(app: &mut App, path: &Path) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    // A drawing without a building program says its independent units and its use by hand.
    for (name, label, value) in [("bb_sayisi", "Bağımsız bölüm sayısı", "2"), ("kullanim_amaci", "Kullanım amacı", "Konut")] {
        if !settings.variables.iter().any(|v| v.name == name) {
            settings.variables.push(kentos_commands::ProjectVariable {
                name: name.into(),
                label: label.into(),
                kind: kentos_commands::VariableKind::Text,
                value: kentos_commands::VariableValue::Text(value.into()),
            });
        }
    }
    doc.set_settings(settings);
    let _ = app.update(Message::Run("arch.permitSet"));
    for (name, value) in PARTICULARS {
        let at = match &app.exchange {
            Some(Window::PermitSet(s)) => s.fields.iter().position(|f| f.name == name).expect(name),
            _ => panic!("Ruhsat takımı is open"),
        };
        let _ = app.permit_set_event(Permit::Field(at, value.into()));
    }
    let _ = app.permit_set_event(Permit::Make);
    finish(app);
    let _ = app.sheet_publish_event(Publish::Path(path.display().to_string()));
    check(app);
}

/// The window's job let finish, as its ticks do.
pub(crate) fn finish(app: &mut App) {
    for _ in 0..1200 {
        if !app.sheet_publish_running() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = app.sheet_publish_event(Publish::Tick);
    }
    panic!("the job did not finish");
}

/// Denetle, its report, and the gate's answer.
pub(crate) fn check(app: &mut App) {
    let _ = app.sheet_publish_event(Publish::Plan);
    finish(app);
    app.permit_gate_settle();
}

fn gate(app: &App) -> Vec<String> {
    match &app.exchange {
        Some(Window::SheetPublish(s)) => {
            let g = s.permit.as_ref().expect("a permit publication");
            assert_eq!(g.failed, None);
            g.findings.iter().map(|f| f.message.clone()).collect()
        }
        _ => panic!("Paftaları toplu yayınla is open"),
    }
}

fn report_ready(app: &App) -> bool {
    match &app.exchange {
        Some(Window::SheetPublish(s)) => {
            let Some(r) = &s.report else { return false };
            for i in r.issues.iter().filter(|i| i.error) {
                eprintln!("error: {} · {} · {}", i.sheet, i.code, i.message);
            }
            r.ready
        }
        _ => false,
    }
}

/// The delivery list read: (file, SHA-256) a row.
fn delivery(folder: &Path) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(folder.join("teslim-listesi.csv")).expect("the delivery list");
    let text = text.strip_prefix('\u{feff}').expect("UTF-8 with its mark");
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("Sıra;Dosya;SHA-256;Bayt;Pafta no;Pafta adı;Revizyon;Model revizyonu;Model özeti")
    );
    lines.map(|l| {
        let f: Vec<&str> = l.split(';').collect();
        (f[1].to_owned(), f[2].to_owned())
    })
    .collect()
}

fn sha256(bytes: &[u8]) -> String {
    kentos_commands::encoding::sha256_hex(bytes)
}

#[test]
fn the_permit_package_passes_its_gate_and_writes_its_delivery_list() {
    let mut app = section_building();
    let folder = evidence("kesit-binasi");
    permit_publication(&mut app, &folder);
    assert_eq!(gate(&app), Vec::<String>::new(), "PDF/A, one sheet a file, A sizes, EK-2 names");
    assert!(report_ready(&app), "the check passes");
    let _ = app.sheet_publish_event(Publish::Publish);
    finish(&mut app);
    let mut pdfs: Vec<String> = std::fs::read_dir(&folder)
        .expect("the delivery folder")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".pdf"))
        .collect();
    pdfs.sort();
    assert!(pdfs.len() >= 6, "{pdfs:?}");
    let listed = delivery(&folder);
    let mut names: Vec<String> = listed.iter().map(|r| r.0.clone()).collect();
    names.sort();
    assert_eq!(names, pdfs, "every file listed once");
    for (name, digest) in &listed {
        assert_eq!(&sha256(&std::fs::read(folder.join(name)).unwrap()), digest, "{name}");
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.join("yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["deliveryList"], "teslim-listesi.csv");
}

#[test]
fn a_wrong_name_one_file_for_all_or_no_pdfa_shuts_the_gate() {
    let mut app = section_building();
    let folder = evidence("kapali");
    permit_publication(&mut app, &folder);
    assert_eq!(gate(&app), Vec::<String>::new());
    let _ = app.sheet_publish_event(Publish::Naming("{index}-{sheet}".into()));
    check(&mut app);
    let shut = gate(&app);
    assert!(!shut.is_empty() && shut.iter().all(|m| m.contains("EK-2 adı değil")), "{shut:?}");
    let _ = app.sheet_publish_event(Publish::Naming(kentos_sheet::permit::FILE_NAMING.into()));
    let _ = app.sheet_publish_event(Publish::Separate(false));
    check(&mut app);
    let shut = gate(&app);
    assert!(shut.iter().any(|m| m.contains("pafta var: her çizim ayrı dosyadır")), "{shut:?}");
    let _ = app.sheet_publish_event(Publish::Separate(true));
    let _ = app.sheet_publish_event(Publish::Archive(false));
    check(&mut app);
    let shut = gate(&app);
    assert!(shut.len() == 1 && shut[0].starts_with("PDF/A seçilmemiş"), "{shut:?}");
}

/// qpdf and Poppler, KentOS's code aside: `scripts/mevzuat/pdfa_bagimsiz.py` on a folder, its JSON and whether it passed.
fn independent(folder: &Path) -> (bool, serde_json::Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(root.join("scripts/mevzuat/pdfa_bagimsiz.py"))
        .arg(folder)
        .output()
        .expect("python3 runs");
    let verdict: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
    });
    (out.status.success(), verdict)
}

/// The section building's and the 2+1 flat's packages pass an independent reading; a renamed file, two sheets in one
/// file and a font not embedded are each refused by it, with the reason. Needs qpdf and Poppler; writes the packages
/// to `.run/gui-evidence/dijital-paket-8066/`.
#[test]
#[ignore = "needs qpdf and Poppler; run by hand"]
fn an_independent_reading_passes_the_packages_and_refuses_broken_files() {
    let mut building = section_building();
    let first = evidence("bagimsiz-kesit-binasi");
    permit_publication(&mut building, &first);
    let _ = building.sheet_publish_event(Publish::Publish);
    finish(&mut building);
    let (ok, verdict) = independent(&first);
    assert!(ok, "{verdict}");
    let mut flat = crate::traces::ek_rules_tests::flat();
    let second = evidence("bagimsiz-2p1");
    permit_publication(&mut flat, &second);
    assert!(report_ready(&flat), "the flat's check passes");
    let _ = flat.sheet_publish_event(Publish::Publish);
    finish(&mut flat);
    let (ok, verdict) = independent(&second);
    assert!(ok, "{verdict}");
    // Broken on purpose: one of the files renamed, two of them in one file, a page whose font is not embedded.
    let broken = evidence("bozuk");
    std::fs::create_dir_all(&broken).unwrap();
    let mut pdfs: Vec<PathBuf> = std::fs::read_dir(&first).unwrap().filter_map(|e| Some(e.ok()?.path())).filter(|p| p.extension().is_some_and(|x| x == "pdf")).collect();
    pdfs.sort();
    std::fs::copy(&pdfs[0], broken.join("yanlis-ad.pdf")).unwrap();
    let two = broken.join("000000_00_MM_XXXX_PL_MMC_K00_01_000.pdf");
    let joined = std::process::Command::new("qpdf")
        .args(["--empty", "--pages"])
        .arg(&pdfs[0])
        .arg(&pdfs[1])
        .arg("--")
        .arg(&two)
        .status()
        .expect("qpdf runs");
    assert!(joined.success() || joined.code() == Some(3));
    std::fs::write(broken.join("000000_00_MM_XXXX_PL_MMC_K00_02_000.pdf"), helvetica_pdf()).unwrap();
    let (ok, verdict) = independent(&broken);
    assert!(!ok);
    let said = |name: &str, what: &str| {
        verdict["files"][name].as_array().is_some_and(|p| p.iter().any(|x| x.as_str().is_some_and(|x| x.contains(what))))
    };
    assert!(said("yanlis-ad.pdf", "EK-2"), "{verdict}");
    assert!(said("000000_00_MM_XXXX_PL_MMC_K00_01_000.pdf", "2 sayfa"), "{verdict}");
    assert!(said("000000_00_MM_XXXX_PL_MMC_K00_02_000.pdf", "gömülmemiş yazı tipi"), "{verdict}");
    assert!(said("000000_00_MM_XXXX_PL_MMC_K00_02_000.pdf", "PDF/A-2b kimliği yok"), "{verdict}");
}

/// An A4 page with a line of text in Helvetica, the font named and not embedded: what a file made elsewhere may be.
fn helvetica_pdf() -> Vec<u8> {
    let content = b"BT /F1 12 Tf 72 720 Td (Bozuk dosya) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.28 841.89] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), String::from_utf8_lossy(content)),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    out
}

/// The permit publication's window: the gate open over a right package, shut by a name EK-2 does not know, and the
/// publication done with its delivery list; light and dark; `.run/gui-evidence/dijital-paket-8066/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib permit_package_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-paket-8066");
    for (theme, size, name) in [("light", Size::new(1600.0, 1000.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = section_building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let folder = evidence(&format!("resim-{name}"));
        permit_publication(&mut app, &folder);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let middle = iced::Point::new(size.width / 2.0, size.height / 2.0);
        let shot = |snapshot: &mut Snapshot, app: &mut App, file: &str| {
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(app, App::view, &mut update);
            snapshot.input(app, App::view, &mut update, Input::Scroll(middle, -80.0));
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut snapshot, &mut app, "1-kapi-acik");
        let _ = app.sheet_publish_event(Publish::Naming("{index}-{sheet}".into()));
        check(&mut app);
        shot(&mut snapshot, &mut app, "2-ad-ek2-degil");
        let _ = app.sheet_publish_event(Publish::Naming(kentos_sheet::permit::FILE_NAMING.into()));
        check(&mut app);
        let _ = app.sheet_publish_event(Publish::Publish);
        finish(&mut app);
        shot(&mut snapshot, &mut app, "3-yayinlandi");
    }
}
