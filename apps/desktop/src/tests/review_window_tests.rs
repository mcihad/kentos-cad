//! İnceleme notları: the chosen note follows the filter; a note the filter hides is neither detailed nor acted on.

use super::*;
use crate::files_testing::app_with_drawing;

fn app_with_notes() -> App {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("review.notes"));
    for text in ["Birinci", "İkinci"] {
        let _ = app.update(Message::ReviewNotes(Event::Text(text.into())));
        let _ = app.update(Message::ReviewNotes(Event::Add));
    }
    app
}

fn status_of(app: &App, number: u32) -> ReviewStatus {
    app.review_listed().notes.iter().find(|n| n.note.number == number).expect("the note").note.status
}

/// UR-054: a resolved note chosen, the Açık filter hides it: no details, and its state is not changed from there.
#[test]
fn a_note_the_filter_hides_is_not_acted_on() {
    let mut app = app_with_notes();
    // The second note (chosen as added) is resolved; the first stays open.
    let _ = app.update(Message::ReviewNotes(Event::Status(Status(ReviewStatus::Resolved))));
    assert_eq!(status_of(&app, 2), ReviewStatus::Resolved);
    // Filtering to Açık moves the choice to the shown open note.
    let _ = app.update(Message::ReviewNotes(Event::Filter(Filter::Open)));
    let first = app.review_listed().notes.iter().find(|n| n.note.number == 1).unwrap().note.id.clone();
    assert_eq!(app.review_window.as_ref().unwrap().chosen.as_deref(), Some(first.as_str()));
    // Resolving it takes its row out of the table: its details go and a further change does nothing.
    let _ = app.update(Message::ReviewNotes(Event::Status(Status(ReviewStatus::Resolved))));
    assert_eq!(status_of(&app, 1), ReviewStatus::Resolved);
    assert_eq!(app.review_visible_chosen(), None);
    let _ = app.update(Message::ReviewNotes(Event::Status(Status(ReviewStatus::Open))));
    assert_eq!(status_of(&app, 1), ReviewStatus::Resolved, "a hidden note is not changed");
    let _ = app.update(Message::ReviewNotes(Event::Remove));
    assert_eq!(app.review_listed().notes.len(), 2, "a hidden note is not removed");
    // An empty filter chooses nothing.
    let _ = app.update(Message::ReviewNotes(Event::Filter(Filter::Answered)));
    assert_eq!(app.review_window.as_ref().unwrap().chosen, None);
    // Tümü shows them again, the choice the first shown.
    let _ = app.update(Message::ReviewNotes(Event::Filter(Filter::All)));
    assert!(app.review_visible_chosen().is_some());
}

/// UR-054's pictures: the Açık filter, empty after the chosen note was resolved, `.run/gui-evidence/kullanim-054/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    crate::files_testing::usage_shots("054", "acik-suzgec-gizli-not", || {
        let mut app = app_with_notes();
        let _ = app.update(Message::ReviewNotes(Event::Filter(Filter::Open)));
        let _ = app.update(Message::ReviewNotes(Event::Status(Status(ReviewStatus::Resolved))));
        app
    });
}
