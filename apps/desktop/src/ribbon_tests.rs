//! The ribbon fits the window (DESIGN.md §7.3.1): at 1100 px every tab's
//! panels fit, stepping down levels, and nothing is cut or scrolled; the
//! Görünüm tab carries the interface's own groups (appearance.rs). Also the
//! pictures of the ribbon for the owner.

use iced::Size;
use kentos_ui::theme::typography::{self, Typography};
use kentos_ui::widget::ribbon::{self, Group};

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::files_testing::app_with_drawing;

/// The groups a tab shows in the drawing's mode, as the ribbon gets them.
fn groups(app: &App, tab: &'static str) -> Vec<Group<'static, Message>> {
    let Some(spec) = app.ribbon_tabs().find(|t| t.id == tab) else {
        return Vec::new();
    };
    let mut groups: Vec<Group<'static, Message>> = spec
        .panels
        .iter()
        .filter_map(|panel| app.ribbon_group(tab, panel))
        .collect();
    if tab == "view" {
        groups.extend(app.appearance_groups());
    }
    groups
}

fn fitted(app: &App, tab: &'static str, width: f32) -> (Vec<u8>, bool) {
    let groups = groups(app, tab);
    let widths: Vec<[f32; 4]> = groups.iter().map(Group::widths).collect();
    let keep: Vec<bool> = groups.iter().map(Group::keeps).collect();
    ribbon::fit(&widths, &keep, width)
}

#[test]
fn every_tab_fits_1100_pixels_without_scrolling() {
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    typography::set(Typography::DEFAULT);
    let mut app = app_with_drawing();
    // In every work mode: each has its own ribbon (modes.rs).
    for mode in ["workspace.hybrid", "workspace.cad", "workspace.gis"] {
        let _ = app.update(Message::Run(mode));
        let tabs: Vec<&'static str> = app.ribbon_tabs().map(|t| t.id).collect();
        for tab in tabs {
            let (levels, overflow) = fitted(&app, tab, 1100.0);
            assert!(!overflow, "{mode} {tab}: does not fit 1100 px ({levels:?})");
            // Wide windows show every panel as designed.
            let (wide, _) = fitted(&app, tab, 3000.0);
            assert!(wide.iter().all(|&l| l == 0), "{mode} {tab}: {wide:?}");
        }
    }
}

#[test]
fn panels_step_down_from_the_right_and_giris_keeps_its_drawing_tools() {
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    typography::set(Typography::DEFAULT);
    let app = app_with_drawing();
    let groups = groups(&app, "home");
    assert!(groups.len() > 2);
    let widths: Vec<[f32; 4]> = groups.iter().map(Group::widths).collect();
    let keep: Vec<bool> = groups.iter().map(Group::keeps).collect();
    assert!(keep.iter().any(|&k| k), "Giriş keeps Çizim and Değiştir");
    let total: f32 = widths.iter().map(|w| w[0]).sum();
    // A little narrower than the full layout: only the rightmost panel steps down.
    let (levels, _) = ribbon::fit(&widths, &keep, total - 10.0);
    let last = levels.len() - 1;
    assert!(levels[last] > 0, "{levels:?}");
    assert!(levels[..last].iter().all(|&l| l == 0), "{levels:?}");
    // Narrow: the kept panels are the last to lose their large buttons.
    let (levels, _) = ribbon::fit(&widths, &keep, 1100.0);
    for (i, &kept) in keep.iter().enumerate() {
        if kept && levels[i] > 0 {
            assert!(
                levels.iter().zip(&keep).all(|(&l, &k)| k || l >= 2),
                "a kept panel shrank before the others showed icons: {levels:?}"
            );
        }
    }
}

#[test]
fn the_view_tab_carries_the_interface_groups_and_drops_the_webs_engine() {
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let app = app_with_drawing();
    let tab = catalog().tabs().find(|t| t.id == "view").expect("Görünüm");
    let web = tab
        .panels
        .iter()
        .filter_map(|p| app.ribbon_group("view", p))
        .count();
    assert_eq!(
        groups(&app, "view").len(),
        web + 5,
        "Tema, Çizim zemini, Yazı tipi, Eş aralıklı, Yazı boyutu"
    );
}

/// Pictures of the ribbon for the owner, in `.run/shots`:
///
/// ```text
/// cargo test -p kentos-desktop ribbon_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use kentos_ui::snapshot::Snapshot;

    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sized = |app: &mut App, name: &str, width: f32, height: f32| {
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    let picture = |app: &mut App, name: &str, width: f32| sized(app, name, width, 900.0);
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        let mut app = app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        for tab in ["home", "modify", "view"] {
            app.tab = tab;
            for width in [1440.0, 1100.0] {
                picture(&mut app, &format!("serit-{tab}-{width}{suffix}"), width);
            }
        }
    }
    // The other themes and a larger text, on the Görünüm tab.
    for (key, value, name) in [
        ("appearance.theme", serde_json::json!("night"), "gece"),
        (
            "appearance.theme",
            serde_json::json!("highContrast"),
            "karsitlik",
        ),
    ] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[(key, value)]);
        app.apply_settings();
        app.tab = "view";
        picture(&mut app, &format!("serit-view-1440-{name}"), 1440.0);
    }
    let mut app = app_with_drawing();
    let _ = app.settings.choose(&[
        ("appearance.textSize", serde_json::json!(15)),
        ("appearance.accentColor", serde_json::json!("turuncu")),
        ("appearance.typeface", serde_json::json!("jakarta")),
    ]);
    app.apply_settings();
    app.tab = "home";
    picture(&mut app, "serit-home-1440-buyuk-yazi", 1440.0);
    app.tab = "view";
    picture(&mut app, "serit-view-1440-buyuk-yazi", 1440.0);
    typography::set(Typography::DEFAULT);
    // The work modes: CAD's Ölçme tab and CBS's Giriş, the mode in the status bar.
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.cad"));
    app.tab = "map";
    picture(&mut app, "serit-cad-olcme-1440", 1440.0);
    let _ = app.update(Message::Run("workspace.gis"));
    app.tab = "home";
    picture(&mut app, "serit-cbs-giris-1440", 1440.0);
    // Every other window at a low window size: nothing cut off (the owner's rule).
    for (id, name) in [
        ("file.start", "baslangic"),
        ("help.shortcuts", "kisayollar"),
        ("help.about", "hakkinda"),
        ("cloud.signIn", "bulut-giris"),
        ("cloud.open", "bulut-projeleri"),
    ] {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run(id));
        sized(&mut app, &format!("pencere-{name}-1100x650"), 1100.0, 650.0);
    }
    let mut app = app_with_drawing();
    let _ = app.update(Message::AppMenu(crate::app_menu::Event::Toggle));
    sized(&mut app, "pencere-uygulama-menusu-1100x650", 1100.0, 650.0);
    // The long windows keep their buttons in view: the body scrolls (a low window too).
    for (id, name) in [
        ("tools.options", "ayarlar"),
        ("file.settings", "proje-ayarlari"),
        ("file.new", "yeni-proje"),
    ] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = app_with_drawing();
            let _ = app.update(Message::Run(id));
            sized(
                &mut app,
                &format!("pencere-{name}-{width}x{height}"),
                width,
                height,
            );
        }
    }
}

/// The tip over a ribbon button, and the button's menu without it
/// (DESIGN.md §7.8: a tip opens after 450 ms and closes on a click), in
/// `.run/shots/serit-ipucu-*`:
///
/// ```text
/// cargo test -p kentos-desktop ribbon_tests::tip_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn tip_screens() {
    use iced::Point;
    use kentos_ui::snapshot::{Input, Snapshot};

    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = app_with_drawing();
    app.tab = "modify";
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
        let file = out.join(format!("serit-ipucu-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    snapshot.settle(&mut app, App::view, &mut update);
    // Dizi, the split button of the Dönüştür panel.
    snapshot.input(
        &mut app,
        App::view,
        &mut update,
        Input::Move(Point::new(361.0, 62.0)),
    );
    save(&mut snapshot, &app, "1-hemen");
    std::thread::sleep(std::time::Duration::from_millis(500));
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "2-bekleyince");
    snapshot.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(361.0, 100.0)),
    );
    std::thread::sleep(std::time::Duration::from_millis(500));
    snapshot.settle(&mut app, App::view, &mut update);
    save(&mut snapshot, &app, "3-menu");
}
