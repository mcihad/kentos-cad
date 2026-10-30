//! The window under the pointer and the keys, through the real widget tree
//! (docs/adr/0803): what wakes a plate, what starts an app, where the page's
//! words lead. KentOS is “installed” at a place that does not exist, so that
//! a start ends in its sentence and no program runs.

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{Dialog, Launcher, Message, Page};
use crate::cards::Status;
use crate::manifest::{self, Folder, Origin};
use crate::registry::Registry;
use crate::resolve::{Found, Spec};
use crate::snapshot::sample;
use crate::view::view;

/// Where things are at 1440 × 900 and 13 px.
const MIMARI: Point = Point::new(432.0, 220.0);
const HARITA: Point = Point::new(820.0, 220.0);
const HARITA_START: Point = Point::new(975.0, 401.0);
const MIMARI_MENU: Point = Point::new(602.0, 357.0);
const YOL: Point = Point::new(310.0, 733.0);
const RECENT_WHOLE: Point = Point::new(975.0, 461.0);
const UPDATES_WHOLE: Point = Point::new(1366.0, 461.0);
const FIRST_RECENT: Point = Point::new(400.0, 495.0);
const SEARCH: Point = Point::new(1090.0, 58.0);
const ADD: Point = Point::new(1295.0, 58.0);
const BELL: Point = Point::new(1386.0, 58.0);
const TABLE: Point = Point::new(100.0, 500.0);
const SETTINGS: Point = Point::new(67.0, 869.0);

struct Window {
    snapshot: Snapshot,
    launcher: Launcher,
}

impl Window {
    fn new() -> Self {
        let mut snapshot =
            Snapshot::software(Size::new(1440.0, 900.0)).expect("the software renderer");
        let mut launcher = sample();
        snapshot.settle(&mut launcher, view, &mut update);
        Self { snapshot, launcher }
    }

    fn input(&mut self, input: Input) {
        self.snapshot
            .input(&mut self.launcher, view, &mut update, input);
    }

    /// Keeps a picture of the window where `KENTOS_LAUNCHER_OUT` says.
    fn keep(&mut self, name: &str) {
        if let Some(folder) = std::env::var_os("KENTOS_LAUNCHER_OUT") {
            let theme = self.launcher.theme();
            self.snapshot
                .render(view(&self.launcher), &theme)
                .save(std::path::Path::new(&folder).join(format!("{name}.png")))
                .expect("written");
        }
    }

    fn said(&self) -> Vec<String> {
        self.launcher
            .notices
            .newest_first()
            .map(|n| n.text.clone())
            .collect()
    }
}

fn update(launcher: &mut Launcher, message: Message) {
    let _ = launcher.update(message);
}

#[test]
fn the_pointer_wakes_a_plate_and_leaves_it() {
    let mut window = Window::new();
    assert_eq!(window.launcher.hot(), None, "nothing at rest");
    window.input(Input::Move(MIMARI));
    assert_eq!(window.launcher.hot(), Some("kentos-mimari"));
    window.input(Input::Move(HARITA));
    assert_eq!(window.launcher.hot(), Some("kentos-harita"));
    window.input(Input::Move(YOL));
    assert_eq!(
        window.launcher.hot(),
        Some("kentos-yol"),
        "a coming app too"
    );
    window.input(Input::Move(TABLE));
    assert_eq!(window.launcher.hot(), None);
}

#[test]
fn a_click_selects_a_plate_and_a_double_click_starts_it() {
    let mut window = Window::new();
    window.input(Input::Click(MIMARI));
    assert_eq!(window.launcher.selected.as_deref(), Some("kentos-mimari"));
    assert!(window.said().is_empty(), "one click starts nothing");
    window.input(Input::Click(MIMARI));
    let said = window.said();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("KentOS Mimari"), "{}", said[0]);
}

#[test]
fn a_plates_button_starts_its_app() {
    let mut window = Window::new();
    window.input(Input::Click(HARITA_START));
    assert_eq!(window.launcher.selected.as_deref(), Some("kentos-harita"));
    let said = window.said();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("KentOS Harita"), "{}", said[0]);
}

/// A real program through the plate's button: the plate and the status line
/// say it runs.
#[cfg(unix)]
#[test]
fn a_started_app_says_it_runs() {
    let mut window = Window::new();
    let id = "kentos-harita";
    let sleep = Spec {
        program: "/bin/sleep".into(),
        args: vec!["2".into()],
        ..Spec::default()
    };
    window
        .launcher
        .found
        .insert(id.to_owned(), Found::Program(sleep));
    window.input(Input::Click(HARITA_START));
    assert_eq!(window.said(), Vec::<String>::new());
    let app = window.launcher.registry.get(id).expect("bundled");
    assert_eq!(window.launcher.status(app), Status::Running(1));
    assert_eq!(window.launcher.processes.apps_running(), 1);
    window.keep("calisiyor");
}

/// What a user adds has no picture of its own: its tone holds the plate's
/// place, a long name is cut, a web address opens, and a few of them still
/// make whole rows.
#[test]
fn apps_a_user_adds_keep_their_place() {
    let mut window = Window::new();
    let mut registry = Registry::builtin();
    let user = |file: &str| Origin::File {
        path: std::path::PathBuf::from("/ev/uygulamalar").join(file),
        folder: Folder::User,
    };
    let read = |text: &str, file: &str| manifest::read(text, user(file)).expect("reads").manifest;
    registry.apps.insert(
        3,
        read(
            r#"{ "format": "kentos.launcher-app/1", "id": "parsel-raporu",
                 "name": "Ada ve parsel raporu hazırlama aracı, uzun adıyla",
                 "note": "Python betiği: ada ve parsel raporunu çizimden yazar, uzun açıklamasıyla",
                 "icon": "rapor", "color": "orange",
                 "launch": { "kind": "program", "program": "/bin/true" }, "order": 35 }"#,
            "parsel-raporu.json",
        ),
    );
    registry.apps.insert(
        4,
        read(
            r#"{ "format": "kentos.launcher-app/1", "id": "kurum-portali", "name": "Kurum portalı",
                 "note": "Belediyenin web portalı", "icon": "web", "color": "teal",
                 "launch": { "kind": "url", "url": "https://ornek.example" }, "order": 36 }"#,
            "kurum-portali.json",
        ),
    );
    window.launcher.set_registry(registry);
    window.launcher.selected = Some("kurum-portali".to_owned());
    window
        .snapshot
        .settle(&mut window.launcher, view, &mut update);
    let (out, coming) = window.launcher.shelves();
    assert_eq!((out.len(), coming.len()), (5, 3));
    assert_eq!(window.launcher.columns.get(), 3);
    let portal = window.launcher.registry.get("kurum-portali").expect("read");
    assert_eq!(window.launcher.status(portal), Status::Web);
    window.keep("kullanici-uygulamalari");
}

#[test]
fn a_coming_app_is_chosen_but_starts_nothing() {
    let mut window = Window::new();
    window.input(Input::Click(YOL));
    window.input(Input::Click(YOL));
    assert_eq!(window.launcher.selected.as_deref(), Some("kentos-yol"));
    assert!(window.said().is_empty());
}

#[test]
fn the_plates_menu_stays_open_under_a_pointer_that_left_the_plate() {
    let mut window = Window::new();
    window.input(Input::Move(MIMARI_MENU));
    window.input(Input::Click(MIMARI_MENU));
    // The first of its items, under the button: Başlat.
    let item = Point::new(MIMARI_MENU.x + 40.0, MIMARI_MENU.y + 28.0);
    window.input(Input::Move(item));
    assert!(window.said().is_empty(), "opening the menu starts nothing");
    window.input(Input::Click(item));
    let said = window.said();
    assert_eq!(said.len(), 1, "the item is still there: {said:?}");
    assert!(said[0].contains("KentOS Mimari"), "{}", said[0]);
}

#[test]
fn the_page_leads_to_its_pages_and_opens_a_drawing() {
    let mut window = Window::new();
    window.input(Input::Click(RECENT_WHOLE));
    assert_eq!(window.launcher.page, Page::Recent);
    window.input(Input::Click(SETTINGS));
    assert_eq!(window.launcher.page, Page::Settings);

    let mut window = Window::new();
    window.input(Input::Click(UPDATES_WHOLE));
    assert_eq!(window.launcher.page, Page::Updates);

    let mut window = Window::new();
    window.input(Input::Click(FIRST_RECENT));
    let said = window.said();
    assert_eq!(
        said.len(),
        1,
        "the drawing's program was asked for: {said:?}"
    );
}

#[test]
fn the_header_searches_adds_and_rings() {
    let mut window = Window::new();
    window.input(Input::Click(SEARCH));
    window.input(Input::Type("gis".to_owned()));
    assert_eq!(window.launcher.query, "gis");
    let (out, coming) = window.launcher.shelves();
    assert_eq!(out.len(), 1);
    assert!(coming.is_empty());

    let mut window = Window::new();
    window.input(Input::Click(ADD));
    assert!(matches!(window.launcher.dialog, Some(Dialog::Edit(_))));

    let mut window = Window::new();
    window.input(Input::Click(BELL));
    assert!(window.launcher.notices_open);
}
