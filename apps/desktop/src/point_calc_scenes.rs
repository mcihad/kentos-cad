//! The pictures of docs/adr/0188 (Nokta hesaplayıcı ekleri) in a CBS
//! project at 1:500, the shared trace's ground (`point-calc-extras.kcad`): a
//! route with a straight and a quarter-turn arc, a line, the point named 101
//! and an angle's corner and arms. Çizgi runs and the calculator over it:
//! Obje üzerinde nokta with the cursor beside the arc, Km ve sapma from
//! 1+000, Mesafe ve eğim, Açıortay, and the command line's chip with its
//! eleven constructions. The web's are `shots.mjs pointcalc`. `tools_screens`
//! takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=hesap-obje,hesap-km cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_interaction::point_calc::CalcKind;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};

/// The route (1), the line (2), the point 101 (3) and the angle's corner and arms (4–6).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    o.bulged(
        "yol",
        &[[-40.0, -10.0], [-20.0, -10.0], [0.0, -10.0]],
        &[0.0, quarter],
    );
    o.line("cizim", [-40.0, 10.0], [-20.0, 10.0]);
    let named = o.point("cizim", [20.0, 10.0], None);
    o.data(named, &[], Some("101"));
    o.point("cizim", [40.0, 10.0], None);
    o.point("cizim", [10.0, -30.0], None);
    o.point("cizim", [35.0, -30.0], None);
    o.point("cizim", [17.0, -6.0], None);
    o
}

/// The drawing in a CBS project at 1:500, Çizgi started at its first point.
fn shown(app: &mut App) {
    open(app, ground());
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_contracts::Workspace::Gis);
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "home";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 46.0,
            min_y: N - 34.0,
            max_x: E + 46.0,
            max_y: N + 26.0,
        },
        24.0,
    );
    run(app, "tool.line");
    click(app, [44.0, 22.0]);
}

fn calc(app: &mut App, kind: CalcKind) {
    let _ = app.update(Message::PointCalc(kind));
}

/// Obje üzerinde nokta on the route, from its start; the cursor beside the arc.
fn object(app: &mut App) {
    shown(app);
    calc(app, CalcKind::Object);
    click(app, [-38.0, -10.0]);
    hover(app, [-6.0, -18.0]);
}

/// Km ve sapma from 1+000; the cursor beside the arc.
fn km(app: &mut App) {
    shown(app);
    calc(app, CalcKind::Km);
    click(app, [-30.0, -10.0]);
    typed(app, "B");
    typed(app, "1+000");
    hover(app, [-14.0, -18.0]);
}

/// Mesafe ve eğim from the point 101 towards the point east of it.
fn slope(app: &mut App) {
    shown(app);
    calc(app, CalcKind::Slope);
    click(app, [20.0, 10.0]);
    click(app, [40.0, 10.0]);
    hover(app, [29.0, 4.0]);
}

/// Açıortay at the corner (10, −30); the cursor beside the bisector.
fn bisector(app: &mut App) {
    shown(app);
    calc(app, CalcKind::Bisector);
    click(app, [10.0, -30.0]);
    click(app, [35.0, -30.0]);
    click(app, [17.0, -6.0]);
    hover(app, [21.0, -14.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("hesap-obje", object),
        ("hesap-km", km),
        ("hesap-egim", slope),
        ("hesap-aciortay", bisector),
    ]
}

/// The command line's Nokta hesabı chip and its eleven constructions.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("hesap-menu", shown, |s, app| {
        press_caption(s, app, "Nokta hesabı", false, false)
    })]
}
