//! The windows of their own (docs/adr/8060): the registry, which window an event belongs to and what is remembered.
//! İşlemler's tool window taken out, brought back and closed is in gis/processing/apart_tests.rs.

use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{self, Key, Modifiers};
use iced::{Point, Size, event, window};

use super::routing::{route, secondary_key};
use super::{Event, Focus, Memory, Surface, Windows};
use crate::app::Message;

#[test]
fn the_registry_keeps_each_window_with_its_surface() {
    let mut w = Windows::default();
    let main = window::Id::unique();
    let other = window::Id::unique();
    w.set_main(main);
    assert!(w.is_main(main));
    assert!(
        !w.is_main(other),
        "with a main window, an unknown one is not it"
    );
    w.add(other, Surface::ProcessingTool);
    assert_eq!(w.surface_of(other), Some(Surface::ProcessingTool));
    assert_eq!(w.surface_of(main), None);
    assert!(!w.of(Surface::ProcessingTool).expect("listed").opened);
    assert!(w.opened(other));
    assert!(!w.opened(window::Id::unique()), "not one of these");
    assert!(w.of(Surface::ProcessingTool).expect("listed").opened);
    // A second window for the same surface takes the first one's place.
    let again = window::Id::unique();
    w.add(again, Surface::ProcessingTool);
    assert_eq!(w.surface_of(other), None);
    assert_eq!(w.of(Surface::ProcessingTool).map(|s| s.id), Some(again));
    // The keyboard goes to an opened window only; the main window always.
    w.ask_focus(Focus::Surface(Surface::ProcessingTool));
    assert_eq!(
        w.take_focus(),
        None,
        "a window being made takes it by itself"
    );
    w.opened(again);
    w.ask_focus(Focus::Surface(Surface::ProcessingTool));
    assert_eq!(w.take_focus(), Some(again));
    assert_eq!(w.take_focus(), None, "asked once");
    w.ask_focus(Focus::Main);
    assert_eq!(w.take_focus(), Some(main));
    // Away and back: said when it changes only.
    assert!(w.set_away(Surface::ProcessingTool, true));
    assert!(!w.set_away(Surface::ProcessingTool, true));
    assert!(w.set_away(Surface::ProcessingTool, false));
    // The main window closing takes every other window and opens none any more.
    assert_eq!(w.close_all(), [again]);
    assert!(w.closing());
    assert_eq!(w.of(Surface::ProcessingTool), None);
    assert_eq!(w.remove(again), None);
}

#[test]
fn without_a_daemon_every_window_but_a_secondary_one_is_the_main_one() {
    let mut w = Windows::default();
    let any = window::Id::unique();
    assert!(w.is_main(any), "tests and snapshots: one window, as before");
    let other = window::Id::unique();
    w.add(other, Surface::ProcessingTool);
    assert!(!w.is_main(other));
    assert!(w.is_main(any));
}

#[test]
fn a_secondary_window_s_events_are_its_own() {
    let main = window::Id::unique();
    let other = window::Id::unique();
    let m = Some(main);
    // The main window's keep the messages they had with one window.
    assert!(matches!(
        route(m, main, window::Event::CloseRequested),
        Some(Message::CloseRequested(id)) if id == main
    ));
    assert!(matches!(
        route(m, main, window::Event::Resized(Size::new(1200.0, 800.0))),
        Some(Message::WindowResized(s)) if s == Size::new(1200.0, 800.0)
    ));
    assert!(matches!(
        route(m, main, window::Event::Rescaled(2.0)),
        Some(Message::DeviceScale(s)) if s == 2.0
    ));
    assert!(
        route(m, main, window::Event::Moved(Point::ORIGIN)).is_none(),
        "the main window's moves are not asked for"
    );
    // A secondary window's are the windows'; its screen's scale is not the main window's.
    assert!(matches!(
        route(m, other, window::Event::CloseRequested),
        Some(Message::Windows(Event::CloseRequested(id))) if id == other
    ));
    assert!(matches!(
        route(m, other, window::Event::Resized(Size::new(900.0, 600.0))),
        Some(Message::Windows(Event::Resized(id, _))) if id == other
    ));
    assert!(matches!(
        route(m, other, window::Event::Moved(Point::new(1500.0, 40.0))),
        Some(Message::Windows(Event::Moved(id, p))) if id == other && p == Point::new(1500.0, 40.0)
    ));
    assert!(matches!(
        route(m, other, window::Event::Closed),
        Some(Message::Windows(Event::Closed(id))) if id == other
    ));
    assert!(route(m, other, window::Event::Rescaled(2.0)).is_none());
    assert!(route(m, other, window::Event::Unfocused).is_none());

    // Its keys: one no field took is its router's, never `Message::Key`; one a field took stays the field's.
    let pressed = |status| {
        secondary_key(
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Character("l".into()),
                modified_key: Key::Character("l".into()),
                physical_key: Physical::Code(Code::KeyL),
                location: keyboard::Location::Standard,
                modifiers: Modifiers::empty(),
                text: Some("l".into()),
                repeat: false,
            }),
            status,
            other,
        )
    };
    assert!(matches!(
        pressed(event::Status::Ignored),
        Some(Message::Windows(Event::Key(id, k))) if id == other && k.character() == Some('l')
    ));
    assert!(
        pressed(event::Status::Captured).is_none(),
        "typing goes to its field"
    );
    // Its modifiers and its focus stay in it: Shift held there turns nothing on in the main window.
    assert!(
        secondary_key(
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::SHIFT)),
            event::Status::Ignored,
            other,
        )
        .is_none()
    );
    assert!(
        secondary_key(
            iced::Event::Window(window::Event::Unfocused),
            event::Status::Ignored,
            other,
        )
        .is_none()
    );
}

#[test]
fn the_choice_and_the_size_are_kept_the_place_only_this_run() {
    let dir = crate::files_testing::scratch("ayri-pencereler");
    let mut memory = Memory::open(&dir);
    assert!(
        !memory.apart(Surface::ProcessingTool),
        "over the drawing at first"
    );
    memory.set_apart(Surface::ProcessingTool, true);
    memory.keep_size(Surface::ProcessingTool, Size::new(1000.0, 720.0));
    memory.keep_place(Surface::ProcessingTool, Point::new(1500.0, 40.0));
    memory.write();
    let again = Memory::open(&dir);
    assert!(again.apart(Surface::ProcessingTool));
    assert_eq!(
        again.size(Surface::ProcessingTool),
        Some(Size::new(1000.0, 720.0))
    );
    assert_eq!(
        again.place(Surface::ProcessingTool),
        None,
        "a screen gone since would hide it"
    );
    // A broken file is no choices; what is not a choice is passed over.
    std::fs::write(dir.join(super::memory::FILE_NAME), "{ bozuk").expect("writes");
    assert!(!Memory::open(&dir).apart(Surface::ProcessingTool));
    let read = super::memory::read(Some(
        r#"{"islem-araci": {"ownWindow": true, "width": "x"}, "b": 3}"#,
    ));
    assert_eq!(read.len(), 1);
    assert_eq!(read["islem-araci"].own_window, Some(true));
    assert_eq!(read["islem-araci"].size, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_3d_view_opens_apart_until_the_user_chooses_otherwise() {
    let dir = crate::files_testing::scratch("ayri-pencereler-3b");
    let mut memory = Memory::open(&dir);
    assert!(memory.apart(Surface::Scene3d), "beside the plan at first");
    assert!(!memory.apart(Surface::ProcessingTool), "a tool over the drawing at first");
    // Its window resized before any choice: the size is kept, the window still opens apart.
    memory.keep_size(Surface::Scene3d, Size::new(1300.0, 820.0));
    memory.write();
    let again = Memory::open(&dir);
    assert!(again.apart(Surface::Scene3d));
    assert_eq!(again.size(Surface::Scene3d), Some(Size::new(1300.0, 820.0)));
    let written = std::fs::read_to_string(dir.join(super::memory::FILE_NAME)).expect("written");
    assert!(!written.contains("ownWindow"), "no choice written: {written}");
    // Brought into the main window: so it stays, in the next run too.
    let mut memory = again;
    memory.set_apart(Surface::Scene3d, false);
    assert!(!Memory::open(&dir).apart(Surface::Scene3d));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_surface_fulfils_the_contract_with_a_name_of_its_own() {
    let mut names = std::collections::BTreeSet::new();
    for s in Surface::ALL {
        let spec = s.spec();
        assert!(
            !spec.kept_as().is_empty() && names.insert(spec.kept_as()),
            "{s:?}: its choices need a name of their own"
        );
        let (first, least) = spec.sizes();
        assert!(
            least.width <= first.width && least.height <= first.height,
            "{s:?}: the smallest size is not larger than the first"
        );
        assert!(
            !spec.what().is_empty() && !spec.apart_tip().is_empty() && !spec.return_tip().is_empty(),
            "{s:?}: its button's tips say what they do"
        );
    }
    // The names released stay as they are: the users' choices on their disks are kept by them.
    assert_eq!(Surface::ProcessingTool.key(), "islem-araci");
    assert_eq!(Surface::Scene3d.key(), "3b-gorunum");
}
