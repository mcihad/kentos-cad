//! The pictures of docs/adr/0313 (line types with patterns): Çizgi tipleri after a LIN file is loaded (its types, the
//! line it could not read said, the gas line's pattern with its text left as a gap); a parcel plan whose layers draw
//! with the file's types at the project's line type scale (a road axis, parcel borders, a gas line, a fence); the
//! same plan with the road's south edge a fence of its own pattern, chosen, its Çizgi tipi row in Öznitelikler. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=cizgitipi-pencere,cizgitipi-cizim,cizgitipi-oznitelik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::line_types_window::Event;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

/// A LIN file in metres as a surveyor's office keeps one: dashes, a text and a shape in a pattern, a line it cannot read.
const LIN: &str = "\
;; KentOS örnek çizgi tipleri, metre
*EKSEN,Eksen ____ _ ____ _
A,12,-3,2,-3
*SINIR,Sınır __ __ . __ __ .
A,6,-2,6,-2,0,-2
*GAZ,Gaz hattı ---- G ----
A,8,-2,[\"G\",STANDARD,S=1.5,X=-0.5,Y=-.75],-2
*CIT,Çit --o--o--
A,4,-1,[CIRC1,ltypeshp.shx,S=.5],-1
*KOTU,Okunmayan
B,1,-1
";

fn patterned(id: &str, name: &str, color: &str, weight: f64, pattern: &str) -> Value {
    let mut l = layer(id, name, color, weight);
    l["style"]["lineType"] = json!("dashed");
    l["style"]["linePattern"] = json!(pattern);
    l
}

/// Two parcels on a road with its axis, a gas line along it and a fence on the back.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.line("yol", [-10.0, 0.0], [130.0, 0.0]);
    o.line("kenar", [-10.0, -7.0], [130.0, -7.0]);
    o.line("kenar", [-10.0, 7.0], [130.0, 7.0]);
    o.path(
        "parsel",
        &[[0.0, 10.0], [55.0, 10.0], [58.0, 60.0], [0.0, 62.0]],
        true,
    );
    o.path(
        "parsel",
        &[[55.0, 10.0], [118.0, 10.0], [120.0, 57.0], [58.0, 60.0]],
        true,
    );
    o.path(
        "gaz",
        &[[-10.0, 4.0], [60.0, 4.0], [62.0, 9.0], [62.0, 30.0]],
        false,
    );
    o.path("cit", &[[0.0, 66.0], [58.0, 64.0], [120.0, 61.0]], false);
    o
}

/// The plan; its layers draw with the file's types when they are in the project (`loaded`); the road's south edge a
/// fence of its own pattern (`own`).
fn drawn(app: &mut App, loaded: bool, own: bool) {
    let types = kentos_formats::lin::read_lin(LIN, "ornek.lin").types;
    open_edited(app, ground(), |d| {
        d["layers"] = json!([
            layer("kenar", "Yol kenarı", "fg", 0.35),
            patterned("yol", "Yol ekseni", "#E5484D", 0.18, "EKSEN"),
            patterned("parsel", "Parsel", "#3E63DD", 0.35, "SINIR"),
            patterned("gaz", "Doğalgaz", "#F5A524", 0.35, "GAZ"),
            patterned("cit", "Çit", "#30A46C", 0.25, "CIT"),
        ]);
        d["activeLayer"] = json!("parsel");
        d["settings"]["plotScale"] = json!(500);
        if !loaded {
            for l in d["layers"].as_array_mut().into_iter().flatten() {
                if let Some(style) = l["style"].as_object_mut() {
                    style.remove("linePattern");
                }
            }
            return;
        }
        d["settings"]["lineTypes"] = serde_json::to_value(&types).expect("the types write");
        // LTSCALE: the file's 12 m dash 12 mm on paper at 1:500.
        d["settings"]["lineTypeScale"] = json!(0.5);
        if own && let Some(edge) = d["entities"].get_mut(1) {
            edge["lineType"] = json!("dashed");
            edge["linePattern"] = json!("CIT");
            edge["color"] = json!("#30A46C");
        }
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 14.0,
            min_y: N - 12.0,
            max_x: E + 134.0,
            max_y: N + 70.0,
        },
        24.0,
    );
}

/// Çizgi tipleri after the file is loaded, the gas line chosen.
fn window(app: &mut App) {
    drawn(app, false, false);
    run(app, "lineTypes.manage");
    let _ = app.update(Message::LineTypes(Event::Loaded(Some((
        "ornek.lin".into(),
        LIN.into(),
    )))));
    let _ = app.update(Message::LineTypes(Event::Choose("GAZ".into())));
}

/// The plan drawn with the file's types.
fn drawing(app: &mut App) {
    drawn(app, true, false);
    hover(app, [140.0, -14.0]);
}

/// The road's south edge chosen, a fence of its own pattern: Öznitelikler's Çizgi tipi row names it.
fn properties(app: &mut App) {
    drawn(app, true, true);
    click(app, [30.0, -7.0]);
    hover(app, [140.0, -14.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("cizgitipi-pencere", window),
        ("cizgitipi-cizim", drawing),
        ("cizgitipi-oznitelik", properties),
    ]
}
