//! Maske and the drawing order tools (docs/adr/0310): the mask drawn through `cad.polygon.create`, an area made a
//! mask through `cad.entities.set` (Nesne), the frame's option, a mask picked by its boundary only, and Nesnenin
//! üstüne / altına through `cad.entities.order`. The drawing is the traces' `objects.kcad`: lines 2 and 3 cross
//! east of the origin, the area 4 is on Parsel, line 6 on a locked layer.

use crate::common::{Bench, E, N};
use kentos_contracts::{Entity, Wipeout};
use kentos_domain::Slot;
use kentos_interaction::{Level, Vec2};

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b
}

fn mask_of(e: &Entity) -> Option<Wipeout> {
    e.base().look.wipeout
}

fn order_of(b: &Bench, slot: u32) -> Option<i32> {
    b.doc.get(Slot(slot)).and_then(|e| e.base().look.order)
}

fn draw(b: &mut Bench, pts: &[[f64; 2]]) {
    for p in pts {
        b.click(p[0], p[1]);
    }
    b.confirm();
}

#[test]
fn maske_draws_a_framed_mask_then_a_frameless_one() {
    let mut b = bench();
    b.start("wipeout");
    assert_eq!(
        b.session.prompt().text(),
        "Maske: maskenin ilk köşesini belirtin [Çerçeve (Ç): açık / Nesne (N)]"
    );
    let options = b.options();
    assert!(
        options.contains(&"Ç") && options.contains(&"N"),
        "{options:?}"
    );
    draw(
        &mut b,
        &[[2.0, 6.0], [14.0, 6.0], [14.0, 14.0], [2.0, 14.0]],
    );
    let mask = b.newest().clone();
    assert!(matches!(mask, Entity::Polygon(_)));
    assert_eq!(mask_of(&mask), Some(Wipeout { frame: true }));
    assert_eq!(mask.base().layer_id, "cizim", "the active layer");
    assert_eq!(b.last_text(), Some("Maske eklendi."));
    assert_eq!(b.last_level(), Some(Level::Success));
    // Çerçeve off: the next mask is frameless; the option is kept for the session.
    assert!(b.type_text("Ç"));
    draw(&mut b, &[[16.0, 6.0], [22.0, 6.0], [19.0, 12.0]]);
    assert_eq!(mask_of(b.newest()), Some(Wipeout { frame: false }));
    assert!(b.memory.wipeout_no_frame);
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

#[test]
fn a_mask_is_picked_by_its_boundary_and_what_it_covers_stays_reachable() {
    let mut b = bench();
    b.start("wipeout");
    draw(
        &mut b,
        &[[2.0, 6.0], [14.0, 6.0], [14.0, 14.0], [2.0, 14.0]],
    );
    let mask = Slot(b.newest().base().id);
    b.spatial.sync(&b.doc);
    let tol = 0.5;
    // Inside the mask, on line 3 (from 0,16 to 16,4: at x 8, y 10): the line, not the mask.
    assert_eq!(
        b.spatial.pick(Vec2::new(E + 8.0, N + 10.0), tol),
        Some(Slot(3))
    );
    // Inside, on nothing: nothing.
    assert_eq!(b.spatial.pick(Vec2::new(E + 5.0, N + 9.5), tol), None);
    // On its boundary: the mask.
    assert_eq!(
        b.spatial.pick(Vec2::new(E + 2.0, N + 12.5), tol),
        Some(mask)
    );
    // Made an ordinary area again, its inside picks it.
    let uid = b.doc.uid(mask).expect("its id").to_string();
    let input = kentos_contracts::EntitiesSetProperties {
        uids: vec![uid],
        layer_id: None,
        color: None,
        line_weight: None,
        line_type: None,
        line_type_scale: None,
        transparency: None,
        wipeout: Some(None),
        symbol: None,
        attrs: None,
        label: None,
        attachments: None,
        unlink: false,
        operation: kentos_contracts::PropertiesOperation::Wipeout,
        expected_revision: None,
    };
    let done = kentos_native_application::set::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut b.doc),
        input,
    );
    assert!(matches!(
        done,
        kentos_contracts::CommandResult::Completed { .. }
    ));
    b.spatial.sync(&b.doc);
    assert_eq!(b.spatial.pick(Vec2::new(E + 5.0, N + 9.5), tol), Some(mask));
}

#[test]
fn nesne_makes_the_area_clicked_a_mask() {
    let mut b = bench();
    b.start("wipeout");
    assert!(b.type_text("N"));
    assert_eq!(
        b.session.prompt().text(),
        "Maske: maske yapılacak kapalı alana tıklayın [Çerçeve (Ç): açık / Nesne (N): açık]"
    );
    // A line is no area: said, nothing written.
    let revision = b.doc.revision();
    b.click(-8.0, -12.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), revision);
    // Inside the area 4: it becomes a mask, one step “Maske”.
    b.click(-18.0, 9.0);
    assert_eq!(
        b.doc.get(Slot(4)).and_then(mask_of),
        Some(Wipeout { frame: true })
    );
    assert_eq!(b.last_text(), Some("Alan maske oldu."));
    assert_eq!(b.doc.undo().as_deref(), Some("Maske"));
    assert_eq!(b.doc.get(Slot(4)).and_then(mask_of), None);
}

#[test]
fn nesnenin_ustune_puts_the_selection_right_above_the_object_clicked() {
    let mut b = bench();
    // Çizim draws 1, 2, 3 (all order 0): 1 above 2 gives 2, 1, 3.
    b.selection.set([Slot(1)]);
    b.start("drawOrderAbove");
    assert_eq!(
        b.session.prompt().text(),
        "Nesnenin üstüne: seçili nesnelerin üstüne konacağı nesneye tıklayın"
    );
    b.click(12.0, 10.0);
    assert_eq!(
        (order_of(&b, 1), order_of(&b, 2), order_of(&b, 3)),
        (Some(1), None, Some(2))
    );
    assert_eq!(
        b.last_text(),
        Some("Nesnenin üstüne: 2 nesnenin çizim sırası değişti.")
    );
    assert!(!b.session.is_running(), "the tool leaves");
    assert_eq!(b.doc.undo().as_deref(), Some("Nesnenin üstüne"));
    // Another layer's object as the reference: refused, said, the tool waits.
    b.selection.set([Slot(1)]);
    b.start("drawOrderUnder");
    b.click(-24.0, 9.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .is_some_and(|t| t.contains("çizim sırası katmanın içindedir"))
    );
    assert!(b.session.is_running());
    assert_eq!(order_of(&b, 1), None);
}
