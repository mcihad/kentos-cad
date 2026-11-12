//! The pictures of docs/adr/0311 (layers: freeze, isolation, states' files, finding and translating): an office's
//! architectural layers with a frozen layer and a frozen group (their marks in Katmanlar, their objects not drawn);
//! Katman ara with a pattern (`a-*`) and with Süzgeç ▾'s Dondurulmuşlar; Katmanları çevir with two rules and its
//! preview; Katman durumları with Dışa aktar… and İçe aktar…. `tools_screens` takes them in both themes at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=katman-dondur,katman-ara,katman-suzgec,katman-cevir,katman-durumlari cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

fn styled(id: &str, name: &str, color: &str, weight: f64, more: Value) -> Value {
    let mut l = layer(id, name, color, weight);
    if let (Some(l), Some(more)) = (l.as_object_mut(), more.as_object()) {
        for (k, v) in more {
            l.insert(k.clone(), v.clone());
        }
    }
    l
}

/// A small plan on an office's layers: the walls, a door, windows (frozen), furniture, an axis (locked), an old
/// group (frozen) with its walls, and an imported layer named another way.
fn ground() -> Objects {
    let mut o = Objects::new();
    let room = [[0.0, 0.0], [12.0, 0.0], [12.0, 8.0], [0.0, 8.0]];
    o.path("duvar", &room, true);
    o.path(
        "duvar",
        &[[0.25, 0.25], [11.75, 0.25], [11.75, 7.75], [0.25, 7.75]],
        true,
    );
    o.line("kapi", [5.0, 0.25], [5.0, 1.25]);
    o.arc("kapi", [5.0, 0.25], 1.0, 0.0, std::f64::consts::FRAC_PI_2);
    o.line("pencere", [8.0, 8.0], [10.5, 8.0]);
    o.path(
        "mobilya",
        &[[1.0, 4.5], [3.0, 4.5], [3.0, 7.0], [1.0, 7.0]],
        true,
    );
    o.circle("mobilya", [8.0, 4.0], 0.9);
    o.line("aks", [-1.5, 4.0], [13.5, 4.0]);
    o.line("eski-duvar", [0.0, 10.0], [12.0, 10.0]);
    o.line("wall", [14.0, 0.0], [14.0, 8.0]);
    o
}

fn tree(d: &mut Value) {
    d["layers"] = json!([
        {
            "id": "mimari", "name": "Mimari", "type": "group", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
            "children": [
                styled("duvar", "A-DUVAR", "#E5484D", 0.5, json!({})),
                styled("kapi", "A-KAPI", "#C8A165", 0.18, json!({})),
                styled("pencere", "A-PENCERE", "#4AA3DF", 0.18, json!({ "frozen": true })),
                styled("mobilya", "A-MOBİLYA", "#8A94A6", 0.13, json!({ "plot": false })),
                styled("aks", "A-AKS", "#E06C75", 0.18, json!({ "locked": true })),
            ]
        },
        {
            "id": "eski", "name": "Eski proje", "type": "group", "visible": true, "locked": false, "expanded": true,
            "frozen": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
            "children": [ styled("eski-duvar", "duvar_eski", "#8C9AAA", 0.18, json!({})) ]
        },
        styled("wall", "Wall", "#30A46C", 0.35, json!({})),
        styled("bos", "Boş katman", "fg", 0.18, json!({})),
    ]);
    d["activeLayer"] = json!("duvar");
}

fn drawn(app: &mut App) {
    open_edited(app, ground(), tree);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.0,
            min_y: N - 3.0,
            max_x: E + 17.0,
            max_y: N + 12.0,
        },
        24.0,
    );
    hover(app, [16.0, -2.0]);
}

/// The tree with its marks: the frozen layer and group faint, a snowflake beside them.
fn frozen(app: &mut App) {
    drawn(app);
}

/// Katman ara with a pattern: the layers whose names start with A-, Türkçe folded.
fn pattern(app: &mut App) {
    drawn(app);
    let _ = app.update(Message::LayerSearch("a-*".to_owned()));
}

/// Süzgeç ▾'s Dondurulmuşlar: the frozen layers alone (by themselves or their groups).
fn props(app: &mut App) {
    drawn(app);
    let _ = app.update(Message::LayerProps(crate::layer_tree::Prop::Frozen));
}

/// Katmanları çevir: the old walls and the imported Wall to A-DUVAR, the empty layer to a new A-NOT; the preview.
fn translate(app: &mut App) {
    drawn(app);
    run(app, "layer.translate");
    use crate::layer_translate::Event;
    let send = |app: &mut App, e: Event| {
        let _ = app.update(Message::LayerTranslate(e));
    };
    send(app, Event::From(0, "duvar*,wall*".into()));
    send(app, Event::To(0, "A-DUVAR".into()));
    send(app, Event::Add);
    send(app, Event::From(1, "Boş*".into()));
    send(app, Event::To(1, "A-NOT".into()));
    send(app, Event::Color(1, "#56B6C2".into()));
    send(app, Event::Weight(1, "0,18".into()));
}

/// Katman durumları with a state saved: Dışa aktar… and İçe aktar… beside the state's buttons.
fn states(app: &mut App) {
    drawn(app);
    let parts = kentos_domain::layer_states::LayerStateParts {
        locks: true,
        styles: true,
    };
    app.save_layer_state("Pafta 1:50", parts);
    run(app, "layer.states");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("katman-dondur", frozen),
        ("katman-ara", pattern),
        ("katman-suzgec", props),
        ("katman-cevir", translate),
        ("katman-durumlari", states),
    ]
}
