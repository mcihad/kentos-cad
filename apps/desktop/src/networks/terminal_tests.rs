//! Terminals and subnetworks in the desktop (docs/adr/0557), on a small electrical network: two sources (Merkez A and
//! Merkez B), a transformer whose terminals are yg, ag and ort (one path, yg–ag), the pipes' ends naming their terminals,
//! an open valve both sources reach, a pipe attached to the transformer's ort terminal that no path leads to and a pipe
//! no source reaches. A trace goes through the transformer along its path only; Ağlar writes the devices and the
//! subnetworks; Alt ağlar says what each source reaches and writes the names in one step.

use kentos_commands::DocumentSnapshotV1;
use serde_json::{Value, json};

use super::Event as Networks;
use super::tests::click_at;
use super::window::Event;
use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;
use crate::subnetworks_window::{Event as Subnets, Pick};

pub(crate) const E: f64 = 487_000.0;
pub(crate) const N: f64 = 4_420_000.0;

fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn window(app: &mut App, e: Event) {
    send(app, Message::Networks(Networks::Window(e)));
}

/// The network's definition: devices and subnetworks as the ADR describes them.
fn definition() -> Value {
    json!({
        "id": "elektrik", "name": "Elektrik", "kind": "utility",
        "edges": [{ "layer": "hat" }],
        "junctions": [
            { "layer": "kaynak", "role": "source" },
            { "layer": "vana", "role": "valve" },
            { "layer": "trafo", "role": "junction" }
        ],
        "connect": "ends", "tolerance": 0.01, "direction": { "kind": "digitized" },
        "terminals": { "start": "uc_bas", "end": "uc_son", "devices": [{
            "layer": "trafo",
            "terminals": [
                { "id": "yg", "name": "Yüksek gerilim" },
                { "id": "ag", "name": "Alçak gerilim" },
                { "id": "ort", "name": "Orta gerilim" }
            ],
            "paths": [["yg", "ag"]]
        }] },
        "subnetworks": { "name": "Ad", "attribute": "Alt ağ", "reach": "downstream" }
    })
}

/// The drawing: the network of `definition()` with its objects (ids as in the command cases' scene, without the typed
/// and locked layers).
pub(crate) fn electric_app() -> App {
    // The subnetworks commands build the graph through İşlemler's host (the real app registers it at start).
    kentos_processing::command::register_as("desktop");
    let (mut app, _) = App::boot(None);
    let layer = |id: &str, name: &str, color: &str| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": color, "lineType": "continuous", "lineWeight": 0.5 }, "children": [] })
    };
    let line = |id: u32, a: [f64; 2], b: [f64; 2], attrs: Value| {
        json!({ "kind": "line", "id": id, "layerId": "hat", "attrs": attrs,
            "a": { "x": E + a[0], "y": N + a[1] }, "b": { "x": E + b[0], "y": N + b[1] } })
    };
    let point = |id: u32, layer: &str, p: [f64; 2], ad: &str| {
        json!({ "kind": "point", "id": id, "layerId": layer, "attrs": { "Ad": ad },
            "p": { "x": E + p[0], "y": N + p[1] } })
    };
    let doc = json!({
        "format": "kentos.document", "version": 1, "name": "Elektrik şebekesi",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "networks": [definition()] },
        "origin": { "x": E, "y": N },
        "layers": [
            layer("hat", "Hat", "#3E63DD"),
            layer("kaynak", "Kaynak", "#30A46C"),
            layer("vana", "Vana", "#E5484D"),
            layer("trafo", "Trafo", "#F76B15"),
        ],
        "activeLayer": "hat",
        "entities": [
            point(1, "kaynak", [0.0, 0.0], "Merkez A"),
            point(2, "kaynak", [400.0, 0.0], "Merkez B"),
            line(3, [0.0, 0.0], [100.0, 0.0], json!({ "uc_son": "yg" })),
            line(4, [100.0, 0.0], [200.0, 0.0], json!({ "uc_bas": "ag" })),
            line(5, [200.0, 0.0], [300.0, 0.0], json!({})),
            line(6, [400.0, 0.0], [300.0, 0.0], json!({})),
            line(7, [150.0, -70.0], [250.0, -70.0], json!({})),
            line(8, [100.0, 0.0], [100.0, 70.0], json!({ "uc_bas": "ort" })),
            point(9, "trafo", [100.0, 0.0], "T-1"),
            point(10, "vana", [300.0, 0.0], "V-1"),
        ],
        "styles": { "items": [], "categories": [] },
    });
    let doc = Document::new(
        DocumentSnapshotV1::from_json(&doc.to_string()).expect("reads"),
        None,
    )
    .expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    send(&mut app, Message::Run("view.zoomExtents"));
    app
}

/// The ids of the selected objects on a layer, ascending.
fn selected(app: &App, layer: &str) -> Vec<u32> {
    let model = &app.document.as_ref().expect("open").model;
    let mut ids: Vec<u32> = app
        .selection
        .ids()
        .iter()
        .filter_map(|s| model.get(*s))
        .filter(|e| e.base().layer_id == layer)
        .map(|e| e.base().id)
        .collect();
    ids.sort_unstable();
    ids
}

fn attr(app: &App, id: u32, name: &str) -> Option<String> {
    let model = &app.document.as_ref().expect("open").model;
    model
        .get(kentos_domain::Slot(id))
        .and_then(|e| e.base().attrs.get(name).cloned())
}

/// A trace from the first pipe reaches what the transformer's path leads to and not the pipe on its ort terminal.
fn trace_from_the_first_pipe(app: &mut App) -> Vec<u32> {
    send(app, Message::Run("tool.netTrace"));
    click_at(app, [50.0, 0.0]);
    let task = app.submit_line("");
    drive(app, task);
    selected(app, "hat")
}

#[test]
fn a_trace_goes_through_the_transformer_only_along_its_terminal_paths() {
    let mut app = electric_app();
    // Pipe 8 is on the ort terminal, which no path leads to; pipe 7 is no source's.
    assert_eq!(trace_from_the_first_pipe(&mut app), [3, 4, 5, 6]);
    // Ağlar: a path from yg to ort too; the same trace now reaches pipe 8.
    send(&mut app, Message::Run("network.manage"));
    window(&mut app, Event::DevicePaths(0, "yg, ag; yg, ort".into()));
    window(&mut app, Event::Save);
    assert_eq!(trace_from_the_first_pipe(&mut app), [3, 4, 5, 6, 8]);
}

#[test]
fn aglar_writes_the_devices_and_the_subnetworks() {
    let mut app = electric_app();
    send(&mut app, Message::Run("network.manage"));
    // The form shows what the definition says, as typed.
    window(
        &mut app,
        Event::DeviceTerminals(0, "yg: YG; ag: AG; ort".into()),
    );
    window(&mut app, Event::SubnetReach(2));
    window(&mut app, Event::SubnetAttribute("AltAg".into()));
    window(&mut app, Event::Save);
    assert!(app.networks.window.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().expect("open").model;
    let net = &doc.settings().networks[0];
    let device = &net.terminals.as_ref().expect("terminals").devices[0];
    assert_eq!(device.layer, "trafo");
    assert_eq!(device.terminals[0].name, "YG");
    assert_eq!(device.terminals[2].name, "ort");
    let subs = net.subnetworks.as_ref().expect("subnetworks");
    assert_eq!(subs.attribute, "AltAg");
    assert_eq!(subs.reach, kentos_commands::SubnetworkReach::Connected);
    // A terminal the path names that the device has not is the form's last word (the contract's).
    send(&mut app, Message::Run("network.manage"));
    window(&mut app, Event::DevicePaths(0, "yg, yok".into()));
    window(&mut app, Event::Save);
    assert!(app.networks.window.is_some(), "Kaydet waits");
}

#[test]
fn alt_aglar_says_what_each_source_reaches_and_guncelle_writes_the_names() {
    let mut app = electric_app();
    send(&mut app, Message::Run("network.subnetworks"));
    let report = app.subnetworks_report().expect("the subnetworks").clone();
    assert_eq!(report.controllers, 2);
    let rows: Vec<(String, u32, u32)> = report
        .subnetworks
        .iter()
        .map(|s| (s.name.clone(), s.edges, s.junctions))
        .collect();
    // Merkez A: pipes 3, 4, 5 and the sources, transformer and valve it reaches; Merkez B: pipe 6, its source and the valve.
    assert_eq!(
        rows,
        [("Merkez A".to_owned(), 3, 3), ("Merkez B".to_owned(), 1, 2)]
    );
    assert_eq!(
        report.orphans.count, 2,
        "pipe 7 and the pipe on the ort terminal"
    );
    assert_eq!(report.shared.count, 1, "the valve");
    assert_eq!(report.stale.missing.count, 8);
    // A row selects its objects.
    send(&mut app, Message::Subnetworks(Subnets::Pick(Pick::Orphans)));
    assert_eq!(selected(&app, "hat"), [7, 8]);
    send(&mut app, Message::Subnetworks(Subnets::Update));
    assert_eq!(attr(&app, 3, "Alt ağ").as_deref(), Some("Merkez A"));
    assert_eq!(attr(&app, 6, "Alt ağ").as_deref(), Some("Merkez B"));
    assert_eq!(
        attr(&app, 10, "Alt ağ").as_deref(),
        Some("Merkez A; Merkez B")
    );
    assert_eq!(attr(&app, 7, "Alt ağ"), None);
    // The window read again: up to date, and one undo step takes the names back.
    let report = app.subnetworks_report().expect("read again");
    assert_eq!(report.stale.missing.count, 0);
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Alt ağları güncelle"));
    assert_eq!(attr(&app, 3, "Alt ağ"), None);
}
