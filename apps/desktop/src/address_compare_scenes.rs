//! Adres kaydıyla karşılaştır (docs/adr/0736): the window's test and its pictures. The drawing is the shared cases'
//! (fixtures/commands/v1/cad.addresses.compare.json: ten doors on the Kapı layer, the address data Numarataj), the
//! register their CSV (fixtures/geocode/v1/register/uavt.csv). The pictures: the columns guessed and the comparison's
//! lists (Farklı yazılmış forward); after Farkları kayıttan yaz, the differences gone in one undo step.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=adres-kayit,adres-kayit-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use crate::address::Event as Address;
use crate::address::compare::Event;
use crate::app::{App, Message};
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.addresses.compare.json");

fn register() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geocode/v1/register/uavt.csv")
}

/// The shared cases' drawing opened, the view on its doors.
fn opened(app: &mut App) {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string())
        .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 500_000.0 - 5.0,
            min_y: 4_400_000.0 - 5.0,
            max_x: 500_000.0 + 105.0,
            max_y: 4_400_000.0 + 30.0,
        },
        24.0,
    );
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Address(Address::Compare(e)));
}

/// The window, the register chosen (its columns guessed) and compared.
fn compared(app: &mut App) {
    opened(app);
    let _ = app.update(Message::Run("address.compare"));
    app.address_compare_file(register());
    app.address_compare_now();
}

/// Farkları kayıttan yaz: the differences written, compared again.
fn written(app: &mut App) {
    compared(app);
    send(app, Event::Write);
    app.address_compare_now();
}

/// The comparison with its Farklı yazılmış list forward.
fn differences(app: &mut App) {
    compared(app);
    send(app, Event::List(crate::address::compare::List::Differences));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("adres-kayit", differences), ("adres-kayit-yazildi", written)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_register_is_compared_and_its_differences_written_in_one_step() {
        let (mut app, _) = App::boot(None);
        compared(&mut app);
        let a = app.address_compared().expect("compared").clone();
        // The columns guessed from the register's names: the code, the four parts and the position.
        assert_eq!((a.doors, a.coded, a.matched, a.rows), (9, 8, 7, 13));
        assert_eq!(
            (a.unregistered, a.differing, a.distant, a.undrawn, a.repeated),
            (1, 4, 1, 1, 1)
        );
        send(&mut app, Event::Write);
        app.address_compare_now();
        let doc = app.document.as_ref().expect("the drawing");
        assert_eq!(
            doc.model.undo_labels().last().map(String::as_str),
            Some("Adres kaydından yaz")
        );
        let again = app.address_compared().expect("compared again");
        assert_eq!(again.differing, 0, "{:?}", again.differences);
        // One undo step takes the four doors back.
        let _ = app.update(Message::Run("edit.undo"));
        app.address_compare_now();
        assert_eq!(app.address_compared().map(|a| a.differing), Some(4));
    }
}
