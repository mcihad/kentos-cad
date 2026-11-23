//! Yeni proje in kentos-cad (docs/adr/9195 §2; the owner saw every menu after it): a map's project shows its module's
//! surfaces alone, and so does the screen while Yeni proje is open again over it; a CBS project made next shows its
//! own. It spans two modules, so it runs with every module registered (docs/adr/9199 §4.2).

use kentos_commands::Workspace;
use kentos_project::wizard::Step;
use crate::project::wizard::Event;
use crate::app::{App, Dialog, Message};
use crate::project::{Event as ProjectEvent, Window};

const PLAN: usize = 1;

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Project(Box::new(ProjectEvent::New(e))));
}

fn state(app: &App) -> &crate::project::wizard::State {
    match &app.project {
        Some(Window::New(s)) => s,
        _ => panic!("the wizard is not open"),
    }
}

fn open(app: &mut App) {
    // A CBS project's tables are made with the data drivers, as the program registers them at start (launch.rs).
    crate::data_layers::register();
    let _ = app.update(Message::Run("file.new"));
    assert_eq!(app.dialog, Some(Dialog::Project));
    assert_eq!(state(app).step, Step::Type);
}

/// Yeni proje in kentos-cad (docs/adr/9195 §2; the owner saw every menu after it): with nothing open the common
/// surfaces; the project made shows its module's alone, and so does the screen while Yeni proje is open again over
/// it; the next project made, of another module, shows that module's.
#[test]
fn a_new_project_shows_its_modules_surfaces_alone_and_so_does_the_next() {
    use crate::app::Panel;
    use crate::bottom::BottomTab;
    use kentos_launch::Module;
    let (mut app, _) = App::boot(None);
    assert_eq!(app.active_module(), None);
    assert_eq!(app.bottom_tabs(), [BottomTab::History, BottomTab::Works, BottomTab::Messages, BottomTab::Python]);
    // A map's project on real coordinates.
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Next);
    send(&mut app, Event::Work(PLAN));
    send(&mut app, Event::System(5254));
    send(&mut app, Event::Next);
    send(&mut app, Event::Name("Ada 7".into()));
    send(&mut app, Event::Next);
    assert_eq!(app.active_module(), Some(Module::Map));
    let map_tabs = app.bottom_tabs();
    assert!(map_tabs.contains(&BottomTab::CadInspection) && !map_tabs.contains(&BottomTab::Chart), "{map_tabs:?}");
    assert!(app.docks.contains(Panel::Sources) && !app.docks.contains(Panel::Navigator));
    // Yeni proje again: the screen behind it is still the map's.
    open(&mut app);
    assert_eq!(app.bottom_tabs(), map_tabs);
    assert!(!app.offers("tool.wall"), "no building's command over a map's project");
    // A CBS project made: its module's surfaces alone.
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    send(&mut app, Event::Province(6));
    send(&mut app, Event::Next);
    let _ = app.update(Message::Swallowed);
    assert_eq!(app.active_module(), Some(Module::Gis));
    let gis_tabs = app.bottom_tabs();
    assert!(!gis_tabs.contains(&BottomTab::CadInspection) && !gis_tabs.contains(&BottomTab::Audit), "{gis_tabs:?}");
    assert!(gis_tabs.contains(&BottomTab::Chart));
    assert!(!app.docks.contains(Panel::Blocks) && app.docks.contains(Panel::Processing));
    assert!(!app.shows_aid("view.lineWeights"));
}
