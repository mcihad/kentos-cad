//! The command names a Netcad user types (docs/adr/0141, the “Netcad adları”
//! table and the new commands' own): each starts its command from the command
//! line, and no other command answers to it. Test code only.

use iced::{Point, Rectangle, Size};

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::files_testing::{app_with_drawing as sample, last_said};
use crate::tools_scenes::{far_ground, open, run};
use crate::viewport::Event;

/// Names of tools, and the tool each starts (the session's id for it).
const TOOL_NAMES: [(&str, &str); 14] = [
    ("ALANSOR", "area"),
    ("CETVEL", "measure"),
    ("XYZSOR", "crsQuery"),
    ("KUTU", "rectangle"),
    ("AYRISTIR", "explode"),
    ("KOSEYUVARLAT", "fillet"),
    ("COKLUDOGRU", "polyline"),
    ("BICIMBOYA", "matchProperties"),
    ("OBJEBOL", "split"),
    ("PENCEREBUYUT", "zoomWindow"),
    ("PRIZMA", "stationOffset"),
    ("CITLESEC", "selectFence"),
    ("DAIRESEC", "selectCircle"),
    ("ICEREN", "selectContaining"),
];

/// Names of view commands: they change what the drawing area shows, or say something.
const VIEW_NAMES: [&str; 5] = ["LIMITBUL", "ONCEKIPENCERE", "ZP", "ZN", "KAPSAM"];

fn app() -> App {
    let mut app = sample();
    let area = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0));
    let _ = app.update(Message::Viewport(Event::Resized(area)));
    app
}

/// A name typed in the command line and sent.
fn type_name(app: &mut App, name: &str) {
    let _ = app.submit_line(name);
}

#[test]
fn each_name_starts_its_tool() {
    for (name, tool) in TOOL_NAMES {
        let mut app = app();
        type_name(&mut app, name);
        assert!(app.session.is_running(), "{name} starts a tool");
        assert_eq!(app.session.tool_id(), tool, "{name}");
        // Typed as a person types it, in lower case too.
        let mut app = self::app();
        type_name(&mut app, &name.to_lowercase());
        assert_eq!(app.session.tool_id(), tool, "{name}, in lower case");
    }
}

#[test]
fn the_view_names_move_through_the_history_and_show_everything() {
    let mut app = app();
    let start = app.viewpoint();
    run(&mut app, "view.zoomIn");
    let zoomed = app.viewpoint();
    assert_ne!(zoomed, start);
    // Önceki pencere, by its Netcad name and by its short one.
    type_name(&mut app, "ONCEKIPENCERE");
    assert_eq!(app.viewpoint(), start, "ONCEKIPENCERE");
    type_name(&mut app, "ZN");
    assert_eq!(app.viewpoint(), zoomed, "ZN");
    type_name(&mut app, "ZP");
    assert_eq!(app.viewpoint(), start, "ZP");
    // Limitleri bul: everything on the screen, as Tümünü göster does.
    let mut all = self::app();
    run(&mut all, "view.zoomExtents");
    let everything = all.viewpoint();
    type_name(&mut app, "LIMITBUL");
    assert_eq!(app.viewpoint(), everything, "LIMITBUL");
}

#[test]
fn kapsam_names_the_extent_check() {
    let mut app = app();
    open(&mut app, far_ground());
    type_name(&mut app, "KAPSAM");
    assert_eq!(
        last_said(&app),
        "2 nesne çizimin geri kalanından çok uzakta; seçildi."
    );
    assert_eq!(app.selection.len(), 2);
}

/// A name that two commands answer to would start whichever comes first.
#[test]
fn no_other_command_answers_to_a_netcad_name() {
    let names = TOOL_NAMES.iter().map(|(name, _)| *name).chain(VIEW_NAMES);
    for name in names {
        let owners: Vec<&str> = catalog()
            .commands()
            .iter()
            .filter(|c| c.aliases.iter().any(|a| a.eq_ignore_ascii_case(name)))
            .map(|c| c.id)
            .collect();
        assert_eq!(owners.len(), 1, "{name} is answered by {owners:?}");
    }
}

/// A tool's method by its own typed name (docs/adr/0147 §7): Ölçülendirme
/// starts with the method's style, as its ribbon menu starts it; the tool's
/// own names start it as it was.
#[test]
fn a_method_s_name_starts_its_tool_with_the_method() {
    for (name, step) in [
        ("DOR", "koordinat ölçüsünün noktasını belirtin"),
        ("dimordinate", "koordinat ölçüsünün noktasını belirtin"),
        ("KOORDINATOLCU", "koordinat ölçüsünün noktasını belirtin"),
        ("DAR", "yay uzunluğu ölçülecek yaya tıklayın"),
        ("YAYUZUNLUGU", "yay uzunluğu ölçülecek yaya tıklayın"),
        ("DIMLIN", "doğrusal ölçünün (ΔY / ΔX) ilk noktasını belirtin"),
        ("DIMRAD", "yarıçapı ölçülecek daireye ya da yaya tıklayın"),
        ("DAL", "hizalı ölçünün ilk noktasını belirtin"),
        ("DJO", "kırıklı yarıçapı ölçülecek daireye ya da yaya tıklayın"),
        ("semt", "semt ölçüsünün başlangıcını gösterin"),
        ("EGIM", "eğim ölçüsünün birinci noktasını gösterin"),
    ] {
        let mut app = app();
        type_name(&mut app, name);
        assert!(app.session.is_running(), "{name} starts a tool");
        assert_eq!(app.session.tool_id(), "dimension", "{name}");
        let prompt = app.session.prompt().text();
        assert!(prompt.starts_with(&format!("Ölçü: {step}")), "{name}: {prompt}");
    }
    // Every method's name answers to its method alone (one ribbon's splits).
    let methods = catalog()
        .tabs_in(kentos_contracts::Workspace::Cad)
        .flat_map(|tab| tab.panels.iter())
        .flat_map(|panel| panel.items.iter())
        .filter_map(|item| match item {
            crate::catalog::Item::Split { entries, .. } => Some(entries.iter()),
            _ => None,
        })
        .flatten()
        .filter(|e| !e.aliases.is_empty())
        .count();
    assert!(methods >= 10, "Ölçülendirme's ten methods have names: {methods}");
}
