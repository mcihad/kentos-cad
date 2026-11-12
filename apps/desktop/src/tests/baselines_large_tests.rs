//! GNSS baz ağı (baselines_window.rs) with a network above the old dense limit (docs/adr/4560): a 26 × 26 grid of
//! stations 1 km apart (676 stations, 2 025 unknowns with the first fixed; the dense solve took at most 1 500) joined to
//! their east, north and north-east neighbours, read from its file. The window adjusts it, the χ² test passes and the
//! summary says the network's size. And its pictures.

use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::files_testing::app_with_drawing;

const W: usize = 26;

/// The network as the window's text: K (the first fixed, every thirteenth weighted by 2 cm), N, B lines.
fn network_file() -> std::path::PathBuf {
    let base = [4_121_000.0, 2_652_000.0, 4_069_000.0];
    let (east, north) = ([-0.54, 0.84, 0.0], [-0.53, -0.34, 0.78]);
    let at = |r: usize, c: usize| [0, 1, 2].map(|k| base[k] + 1000.0 * (c as f64 * east[k] + r as f64 * north[k]));
    let mut text = String::from("# 26 × 26 istasyonluk ızgara (ADR 4560)\n");
    for r in 0..W {
        for c in 0..W {
            let i = r * W + c;
            let p = at(r, c);
            if i == 0 {
                text += &format!("K;S{i};{:.4};{:.4};{:.4}\n", p[0], p[1], p[2]);
            } else if i % 13 == 0 {
                text += &format!("K;S{i};{:.4};{:.4};{:.4};0.02;0.02;0.02\n", p[0], p[1], p[2]);
            } else {
                text += &format!("N;S{i}\n");
            }
        }
    }
    let mut k = 0usize;
    for r in 0..W {
        for c in 0..W {
            for (dr, dc) in [(0, 1), (1, 0), (1, 1)] {
                if r + dr < W && c + dc < W {
                    let (a, b) = (at(r, c), at(r + dr, c + dc));
                    let kf = k as f64;
                    let noise = [0.003 * (kf * 0.7).sin(), 0.003 * (kf * 1.3).cos(), 0.006 * (kf * 0.9).sin()];
                    let d = [0, 1, 2].map(|q| b[q] - a[q] + noise[q]);
                    text += &format!(
                        "B;S{};S{};{:.4};{:.4};{:.4};9e-6;2.7e-6;0;9e-6;0;36e-6\n",
                        r * W + c,
                        (r + dr) * W + c + dc,
                        d[0],
                        d[1],
                        d[2]
                    );
                    k += 1;
                }
            }
        }
    }
    let path = std::env::temp_dir().join(format!("kentos-buyuk-ag-{}.txt", std::process::id()));
    std::fs::write(&path, text).expect("writes");
    path
}

#[test]
fn a_network_above_the_old_limit_is_adjusted() {
    let mut app = app_with_drawing();
    app.program = Some(Module::Map);
    let _ = app.update(Message::Run("survey.baselines"));
    assert_eq!(app.dialog, Some(Dialog::Baselines));
    let path = network_file();
    app.baselines_file(path.clone());
    let _ = std::fs::remove_file(&path);
    let a = app.baselines.answer.clone().expect("an answer").expect("adjusted");
    assert_eq!((a.stations.len(), a.u as usize), (W * W, 3 * (W * W - 1)));
    assert!(a.u > 1500, "above the old dense limit");
    assert_eq!(a.passed, Some(true), "the χ² test passes");
}

/// `.run/gui-evidence/seyrek-cozum-4560/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib baselines_large_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/seyrek-cozum-4560");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app_with_drawing();
        app.program = Some(Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("survey.baselines"));
        let path = network_file();
        app.baselines_file(path.clone());
        let _ = std::fs::remove_file(&path);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-buyuk-ag-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
