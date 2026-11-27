//! The Python console's head (the CBS review's 20th item, docs/adr/9336): which Python is chosen and where from, whether
//! it holds the kentos package (looked at when the tab is shown, nothing kept running), KentOS's managed environment and
//! the user's own Python said apart, Python ortamını yapılandır shown when it holds none, Yeniden başlat open after a
//! start that failed.

use std::path::{Path, PathBuf};

use kentos_standards::PythonSource as S;

use super::{Console, Event, look_at};
use crate::app::{App, Message};
use crate::files_testing::drive;

fn console(source: S, python: &str) -> Console {
    Console {
        source: Some(source),
        python: PathBuf::from(python),
        ..Console::default()
    }
}

#[test]
fn the_head_says_where_python_comes_from_and_offers_the_way_to_set_it_up() {
    // Before the tab looked: nothing said.
    assert_eq!(Console::default().head(), None);
    // The user's own Python without kentos: said as theirs, with the way to set one up.
    let mut c = console(S::System, "python3");
    assert_eq!(c.head(), Some(("Sistemdeki python3 (kendi Python'unuz) (python3) denetleniyor…".to_owned(), false)));
    c.check = Some(Err("ModuleNotFoundError: No module named 'kentos'".to_owned()));
    let (words, setup) = c.head().expect("a head");
    assert!(setup);
    assert_eq!(
        words,
        "Sistemdeki python3 (kendi Python'unuz) kullanılıyor (python3); KentOS paketi bu Python'da yok: \
         ModuleNotFoundError: No module named 'kentos'. KentOS'un yönettiği ortamı kurun ya da kentos paketini bu \
         Python'a kurun."
    );
    // KentOS's managed environment: said apart; broken, set up again from the settings.
    let mut c = console(S::Personal, "/kullanici/python/bin/python");
    c.check = Some(Ok("Python 3.12.13 · kentos 0.1.0".to_owned()));
    assert_eq!(
        c.head(),
        Some(("KentOS'un yönettiği kişisel ortam · Python 3.12.13 · kentos 0.1.0 · ilk ifadede başlar".to_owned(), false))
    );
    c.failed = Some("KentOS oturumu açılmadı".to_owned());
    let (words, setup) = c.head().expect("a head");
    assert!(setup && words.contains("Ortamı Python ve eklentiler'den yeniden kurun."), "{words}");
    // Ready: the session's versions.
    c.failed = None;
    c.ready = Some("Python 3.12.13 · kentos 0.1.0".to_owned());
    assert_eq!(c.head(), Some(("KentOS'un yönettiği kişisel ortam · Python 3.12.13 · kentos 0.1.0".to_owned(), false)));
}

#[test]
fn a_python_that_does_not_open_is_said_and_the_checkouts_holds_kentos() {
    assert!(look_at(Path::new("/olmayan/klasor/python")).is_err_and(|why| why.starts_with("açılamadı (")));
    // This checkout's own environment (the tests' Python, found from as deep as the test's program) holds the package.
    let (python, source) = kentos_standards::interpreter_choice();
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/py/bin/python");
    if std::env::var_os("KENTOS_PYTHON").is_none() && checkout.exists() {
        assert_eq!(source, S::Checkout, "{}", python.display());
    }
    if source == S::Checkout || source == S::Env {
        let seen = look_at(&python).expect("the checkout's Python holds kentos");
        assert!(seen.starts_with("Python 3.") && seen.contains(" · kentos "), "{seen}");
    }
}

#[test]
fn the_python_tab_looks_at_the_chosen_python_once() {
    let (mut app, _) = App::boot(None);
    let task = app.update(Message::BottomTab(crate::bottom::BottomTab::Python));
    assert!(app.python.source.is_some(), "where it comes from is known at once");
    drive(&mut app, task);
    assert!(app.python.check.is_some(), "looked at");
    assert!(!app.python.started(), "nothing kept running");
    // A start that failed: Yeniden başlat looks again, the head offers the way to set Python up.
    app.python.failed = Some("KentOS oturumu açılmadı".to_owned());
    assert!(app.python.head().is_some_and(|(_, setup)| setup));
    let task = app.update(Message::Python(Event::Restart));
    assert_eq!(app.python.failed, None);
    drive(&mut app, task);
    assert!(app.python.check.is_some(), "looked at again");
}

/// The pictures (`.run/gui-evidence/kullanim-9336/`): the user's own python3 without the kentos package, said as
/// theirs with Python ortamını yapılandır; the checkout's environment looked at and ready, the example giving the
/// project's layers' names.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib python::check_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9336", "01-kendi-python", || {
        let mut app = crate::files_testing::app_with_drawing();
        app.python.source = Some(S::System);
        app.python.python = PathBuf::from("python3");
        app.python.check = Some(Err("ModuleNotFoundError: No module named 'kentos'".to_owned()));
        app.show_bottom(crate::bottom::BottomTab::Python);
        let _ = app.update(Message::BottomResized(300.0));
        app
    });
    crate::files_testing::usage_shots("9336", "02-ortam-hazir", || {
        let mut app = crate::files_testing::app_with_drawing();
        let task = app.update(Message::BottomTab(crate::bottom::BottomTab::Python));
        drive(&mut app, task);
        let _ = app.update(Message::BottomResized(300.0));
        let mut heard = None;
        super::tests::run_real(&mut app, "print([k.name for k in doc.layers()])", &mut heard);
        app
    });
}
