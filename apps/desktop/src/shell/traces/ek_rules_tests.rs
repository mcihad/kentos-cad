//! EK-3 and EK-4 on the 2+1 flat (docs/adr/8064; plan/bim-plan.md BIM-03): the mouse-and-keyboard trace
//! (fixtures/interaction/v1/acceptance-flat.json) played as KentOS Mimari starts, then the Denetim tab's check with the
//! standards package's rule.

use super::player::AREA;
use super::{Player, Trace, VARIANTS, scratch_file};
use crate::app::{App, Message};

/// The 2+1 flat as KentOS Mimari draws it (the layer key on, Dış duvar 25).
pub(crate) fn flat() -> App {
    let traces = Trace::all().expect("the traces read");
    let trace = traces.iter().find(|t| t.id == "acceptance-flat").expect("the 2+1 trace");
    let variant = VARIANTS[0];
    let (mut app, _) = App::boot(None);
    app.memory.layer_keys = true;
    let style = kentos_commands::builtin_wall_styles()
        .into_iter()
        .find(|s| s.id == crate::arch_start::FIRST_WALL_STYLE)
        .expect("KentOS's exterior wall");
    kentos_interaction::wall::take_style(&mut app.memory, &style);
    let problems = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))
        .expect("the trace starts")
        .play(None);
    assert!(problems.is_empty(), "{problems:#?}");
    app
}

/// A KentOS command on the drawing, as the Denetim tab's writes run one; its answer must say it was done.
fn run(app: &mut App, command: &str, input: serde_json::Value) {
    let doc = &mut app.document.as_mut().expect("the flat").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        command,
        kentos_commands::CallOp::Execute,
        input.clone(),
    )
    .unwrap_or_else(|e| panic!("{command} {input}: {e:?}"));
    assert_eq!(answer["status"], "completed", "{command} {input}: {answer}");
}

/// The Denetim tab checked with the rule: the EK-3 and EK-4 findings' rules (their codes past `digital_ek`).
fn rules(app: &mut App) -> Vec<String> {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.standards_failed, None);
    let mut out: Vec<String> = app.arch.arch_audit.standards.iter().map(|f| f.code.clone()).collect();
    out.sort();
    out.dedup();
    out
}

/// The finding of `code` chosen and Düzelt pressed: its fix as one undo step.
fn fix(app: &mut App, code: &str) {
    let found = app.arch.arch_audit.audited.as_ref().expect("checked");
    let at = found.findings.iter().position(|f| f.code == code).unwrap_or_else(|| panic!("no {code}"));
    assert!(found.findings[at].fix.is_some(), "{code} has a fix");
    app.arch.arch_audit.selected = Some(at);
    let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Fix)));
}

fn rooms(app: &App) -> Vec<(String, String)> {
    let doc = &app.document.as_ref().expect("the flat").model;
    let mut out: Vec<(String, String)> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| match e {
            kentos_commands::Entity::Space(s) => Some((s.space.name.clone().unwrap_or_default(), uid.to_string())),
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn uid_of(app: &App, name: &str) -> String {
    rooms(app).into_iter().find(|(n, _)| n == name).map(|(_, u)| u).unwrap_or_else(|| panic!("no room {name}"))
}

const DIM: &str = "digital_ek4_dimension_font";
const BLOCK: &str = "digital_ek4_block_name";

/// Each EK-3 and EK-4 rule passing and failing on the 2+1 flat, its fix making it pass again where it has one.
#[test]
fn every_ek3_and_ek4_rule_passes_and_fails_on_the_2p1_flat() {
    let mut app = flat();
    // As drawn: its nine layers are EK-4's, its rooms named from Tablo 3.2 and not numbered; its dimensions are in
    // Barlow and its kitchen blocks named in Turkish.
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // EK-3: the storey's rooms numbered clockwise, as Mahal numarala's EK-3 does.
    let clockwise = ["salon", "koridor", "banyo", "yatak odası 2", "yatak odası 1"];
    let uids: Vec<String> = clockwise.iter().map(|n| uid_of(&app, n)).collect();
    run(&mut app, "cad.spaces.number", serde_json::json!({"uids": uids, "first": "K00_D01_001"}));
    assert_eq!(rules(&mut app), [BLOCK, DIM], "numbered by EK-3: every EK-3 rule passes");
    // number_pattern
    let salon = uid_of(&app, "salon");
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [salon], "operation": {"kind": "set", "number": "101"}}));
    assert!(rules(&mut app).contains(&"digital_ek3_number_pattern".to_owned()));
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [salon], "operation": {"kind": "set", "number": "K00_D01_001"}}));
    // storey_code, fixed by its Düzelt
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [salon], "operation": {"kind": "set", "number": "K01_D01_001"}}));
    assert!(rules(&mut app).contains(&"digital_ek3_storey_code".to_owned()));
    fix(&mut app, "digital_ek3_storey_code");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // sequence and clockwise: two numbers swapped, then one given twice; Düzelt numbers the storey again
    let (banyo, koridor) = (uid_of(&app, "banyo"), uid_of(&app, "koridor"));
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [banyo], "operation": {"kind": "set", "number": "K00_D01_002"}}));
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [koridor], "operation": {"kind": "set", "number": "K00_D01_003"}}));
    assert!(rules(&mut app).contains(&"digital_ek3_clockwise".to_owned()));
    fix(&mut app, "digital_ek3_clockwise");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [banyo], "operation": {"kind": "set", "number": "K00_D01_002"}}));
    assert!(rules(&mut app).contains(&"digital_ek3_sequence".to_owned()));
    fix(&mut app, "digital_ek3_sequence");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // room_name: spelt without its Turkish letter; Düzelt writes the table's
    let yatak = uid_of(&app, "yatak odası 2");
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [yatak], "operation": {"kind": "set", "name": "Yatak Odasi 2"}}));
    assert!(rules(&mut app).contains(&"digital_ek3_room_name".to_owned()));
    fix(&mut app, "digital_ek3_room_name");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    assert!(rooms(&app).iter().any(|(n, _)| n == "Yatak Odası 2"));
    // layer_name: a layer renamed in small letters; Düzelt gives its EK-4 name back
    run(&mut app, "cad.layers.set", serde_json::json!({"operation": "rename", "layer": "mm-duva-tugla", "name": "mm-duva-tugla"}));
    assert!(rules(&mut app).contains(&"digital_ek4_layer_name".to_owned()));
    fix(&mut app, "digital_ek4_layer_name");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // layer_zero and emsal_layer: a point drawn on “0”, another on an emsal layer whose type is not EK-4's
    for (id, name) in [("sifir", "0"), ("emsal", "MM-EMSL-BALKON")] {
        run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": id, "name": name}));
        run(&mut app, "cad.entities.create", serde_json::json!({"layerId": id, "objects": [{"geometry": {"kind": "point", "p": {"x": 1.0, "y": 1.0}}}]}));
    }
    let found = rules(&mut app);
    assert!(found.contains(&"digital_ek4_layer_zero".to_owned()) && found.contains(&"digital_ek4_emsal_layer".to_owned()), "{found:?}");
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "remove", "layer": "sifir"}));
    run(&mut app, "cad.layers.set", serde_json::json!({"operation": "rename", "layer": "emsal", "name": "MM-EMSL-DAHIL"}));
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // axis_line: a grid on a dash-dot MM-AKSS-GENEL passes; drawn continuous it fails; Düzelt makes it dash-dot again
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": "aks", "name": "MM-AKSS-GENEL", "lineType": "dashdot"}));
    run(&mut app, "cad.grids.create", serde_json::json!({"layerId": "aks", "grids": [{"pts": [{"x": 0.0, "y": -1.0}, {"x": 0.0, "y": 9.0}], "label": "A"}]}));
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "style", "layer": "aks", "lineType": "continuous"}));
    assert!(rules(&mut app).contains(&"digital_ek4_axis_line".to_owned()));
    fix(&mut app, "digital_ek4_axis_line");
    assert_eq!(rules(&mut app), [BLOCK, DIM]);
    // dimension_font: the drawing's typeface Arimo, Arial's metrics: no longer a warning, a note for the administration
    run(&mut app, "cad.settings.set", serde_json::json!({"drawingFont": "arimo"}));
    rules(&mut app);
    let dims: Vec<_> = app.arch.arch_audit.standards.iter().filter(|f| f.code == DIM).collect();
    assert!(dims.len() == 1 && dims[0].message.contains("K29"), "{dims:?}");
}

/// The rules an independent reading of a DXF can judge, with the finding codes the standards package gives them.
const READ_IN_DXF: [(&str, &str); 9] = [
    ("number_pattern", "digital_ek3_number_pattern"),
    ("sequence", "digital_ek3_sequence"),
    ("clockwise", "digital_ek3_clockwise"),
    ("layer_name", "digital_ek4_layer_name"),
    ("layer_zero", "digital_ek4_layer_zero"),
    ("hatch_layer", "digital_ek4_hatch_layer"),
    ("emsal_layer", "digital_ek4_emsal_layer"),
    ("axis_line", "digital_ek4_axis_line"),
    ("block_name", "digital_ek4_block_name"),
];

/// The drawing written as DXF (as another program reads it: KentOS's data left out) and read by
/// scripts/mevzuat/ek34_bagimsiz.py with ezdxf, no KentOS code; its verdicts against the package's on the drawing.
fn agree(app: &mut App, file: &str) -> (Vec<(String, String, String)>, serde_json::Value) {
    let found = rules(app);
    let doc = &app.document.as_ref().expect("the flat").model;
    let objects: Vec<&kentos_commands::Entity> = doc.entities().collect();
    let chosen = kentos_headless::export::by_layer(doc, objects);
    let mut input = kentos_headless::export::dxf_input(doc, &chosen);
    input.annotations.metadata = false;
    let (bytes, _) = kentos_formats::dxf::write(&input);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join(".run/gui-evidence/dijital-ek34-8064");
    std::fs::create_dir_all(&out).expect("a folder");
    let path = out.join(file);
    std::fs::write(&path, &bytes).expect("the DXF");
    let read = std::process::Command::new("uv")
        .args(["run", "--offline", "--no-project", "--with", "ezdxf==1.4.5", "python", "-I"])
        .arg(root.join("scripts/mevzuat/ek34_bagimsiz.py"))
        .arg(&path)
        .output()
        .expect("uv runs");
    assert!(read.status.success(), "{}", String::from_utf8_lossy(&read.stderr));
    let theirs: serde_json::Value = serde_json::from_slice(&read.stdout).expect("its verdicts");
    let rows = READ_IN_DXF
        .iter()
        .map(|(rule, code)| {
            let ours = if found.iter().any(|f| f == code) { "fail" } else { "pass" };
            (rule.to_string(), ours.to_owned(), theirs[rule].as_str().unwrap_or("?").to_owned())
        })
        .collect();
    (rows, theirs)
}

/// The independent reading agrees with the package's rule on the 2+1 flat, numbered and with an axis and an emsal
/// layer, rule by rule, and again once three rules are broken. Needs uv with ezdxf 1.4.5 in its cache; writes the two
/// DXFs to `.run/gui-evidence/dijital-ek34-8064/`.
#[test]
#[ignore = "needs uv and ezdxf; run by hand"]
fn an_independent_reading_of_the_flat_s_dxf_agrees_rule_by_rule() {
    let mut app = flat();
    let clockwise = ["salon", "koridor", "banyo", "yatak odası 2", "yatak odası 1"];
    let uids: Vec<String> = clockwise.iter().map(|n| uid_of(&app, n)).collect();
    run(&mut app, "cad.spaces.number", serde_json::json!({"uids": uids, "first": "K00_D01_001"}));
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": "aks", "name": "MM-AKSS-GENEL", "lineType": "dashdot"}));
    run(&mut app, "cad.grids.create", serde_json::json!({"layerId": "aks", "grids": [{"pts": [{"x": 0.0, "y": -1.0}, {"x": 0.0, "y": 9.0}], "label": "A"}]}));
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": "emsal", "name": "MM-EMSL-DAHIL"}));
    run(&mut app, "cad.entities.create", serde_json::json!({"layerId": "emsal", "objects": [{"geometry": {"kind": "point", "p": {"x": 1.0, "y": 1.0}}}]}));
    let (rows, theirs) = agree(&mut app, "2p1-ek34-gecen.dxf");
    println!("geçen: {rows:?} {}", theirs["_numbers"]);
    assert_eq!(theirs["_numbers"].as_array().map(Vec::len), Some(5), "the five numbers read in the file");
    for (rule, ours, read) in &rows {
        assert_eq!(ours, read, "{rule}");
    }
    // Broken: two numbers swapped, a point on “0”, a layer in small letters.
    let (banyo, koridor) = (uid_of(&app, "banyo"), uid_of(&app, "koridor"));
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [banyo], "operation": {"kind": "set", "number": "K00_D01_002"}}));
    run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [koridor], "operation": {"kind": "set", "number": "K00_D01_003"}}));
    run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": "sifir", "name": "0"}));
    run(&mut app, "cad.entities.create", serde_json::json!({"layerId": "sifir", "objects": [{"geometry": {"kind": "point", "p": {"x": 2.0, "y": 2.0}}}]}));
    run(&mut app, "cad.layers.set", serde_json::json!({"operation": "rename", "layer": "mm-penc-genel", "name": "mm-penc-genel"}));
    let (rows, _) = agree(&mut app, "2p1-ek34-kalan.dxf");
    println!("kalan: {rows:?}");
    for (rule, ours, read) in &rows {
        assert_eq!(ours, read, "{rule}");
    }
    let failed: Vec<&str> = rows.iter().filter(|r| r.1 == "fail").map(|r| r.0.as_str()).collect();
    assert_eq!(failed, ["clockwise", "layer_name", "layer_zero", "block_name"]);
}

/// The Denetim tab's Mevzuat group with the EK-3 and EK-4 findings, a fix chosen and done, light and dark;
/// `.run/gui-evidence/dijital-ek34-8064/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ek_rules_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-ek34-8064");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1600.0, 1000.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = flat();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        // Numbered by EK-3, then two numbers swapped, one room's name spelt without its Turkish letter, a layer in
        // small letters and an axis drawn continuous.
        let clockwise = ["salon", "koridor", "banyo", "yatak odası 2", "yatak odası 1"];
        let uids: Vec<String> = clockwise.iter().map(|n| uid_of(&app, n)).collect();
        run(&mut app, "cad.spaces.number", serde_json::json!({"uids": uids, "first": "K00_D01_001"}));
        let (banyo, koridor, yatak) = (uid_of(&app, "banyo"), uid_of(&app, "koridor"), uid_of(&app, "yatak odası 2"));
        run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [banyo], "operation": {"kind": "set", "number": "K00_D01_002"}}));
        run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [koridor], "operation": {"kind": "set", "number": "K00_D01_003"}}));
        run(&mut app, "cad.spaces.edit", serde_json::json!({"uids": [yatak], "operation": {"kind": "set", "name": "Yatak Odasi 2"}}));
        run(&mut app, "cad.layers.set", serde_json::json!({"operation": "rename", "layer": "mm-penc-genel", "name": "mm-penc-genel"}));
        run(&mut app, "cad.layers.edit", serde_json::json!({"operation": "add", "id": "aks", "name": "MM-AKSS-GENEL", "lineType": "continuous"}));
        run(&mut app, "cad.grids.create", serde_json::json!({"layerId": "aks", "grids": [{"pts": [{"x": -1.0, "y": -1.0}, {"x": -1.0, "y": 9.0}], "label": "A"}]}));
        // The drawing's work said (else the window asks it over everything), the tab tall enough for the group.
        run(&mut app, "cad.settings.set", serde_json::json!({"cadWork": "architecture"}));
        app.dialog = None;
        let _ = app.update(Message::Run("arch.audit"));
        app.bottom_dragged(Some(size.height * 0.42), std::time::Instant::now());
        app.arch_audit_standards_settle();
        let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Group(Some(
            kentos_commands::ArchAuditCheck::Regulation,
        )))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |snapshot: &mut Snapshot, app: &mut App, file: String| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(file);
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut snapshot, &mut app, format!("1-mevzuat-ek34-{name}.png"));
        // The clockwise finding chosen: its rooms selected, its Düzelt ready.
        let at = app
            .arch_audit_shown()
            .into_iter()
            .position(|i| {
                app.arch.arch_audit.audited.as_ref().is_some_and(|a| a.findings[i].code == "digital_ek3_clockwise")
            })
            .expect("the clockwise finding shown");
        let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Press(at))));
        shot(&mut snapshot, &mut app, format!("2-saat-yonu-secili-{name}.png"));
        fix(&mut app, "digital_ek3_clockwise");
        app.arch_audit_standards_settle();
        shot(&mut snapshot, &mut app, format!("3-duzeltildi-{name}.png"));
        // The table's end: the layer, the axis, the dimensions' typeface and the kitchen blocks.
        let table = iced::Point::new(size.width * 0.35, size.height - 140.0);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(table, -20.0));
        shot(&mut snapshot, &mut app, format!("4-katman-aks-yazi-blok-{name}.png"));
    }
}

#[test]
#[ignore = "prints the flat's EK-3 and EK-4 findings, by hand"]
fn print_the_flat_s_findings() {
    let mut app = flat();
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    println!("failed: {:?}", app.arch.arch_audit.standards_failed);
    for f in &app.arch.arch_audit.standards {
        println!("{} | {} | {} | fix: {:?}", f.code, f.message, f.uids.len(), f.fix.as_ref().map(|x| &x.label));
    }
    let facts = app.arch_audit_standards_facts().expect("facts");
    println!("{}", serde_json::to_string_pretty(&facts["cad"]).unwrap());
}
