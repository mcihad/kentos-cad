//! Birleştir's Zincir (docs/adr/0161 §2) on the shared trace's drawing
//! (fixtures/interaction/v1/join-chain.kcad): lines 1 and 2, the arc 3 and
//! the junction's lines 4 and 5 on Yol; line 6 on the locked Eski yol, at
//! line 1's western end. Expected values are worked out by hand; the web's
//! are `apps/web/src/tools/joinChain.test.ts`.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::Level;

const SCENE: &str = include_str!("../../../../../fixtures/interaction/v1/join-chain.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(SCENE);
    b.draft.snap = false;
    b
}

fn ids(b: &Bench) -> Vec<u32> {
    b.doc.entities().map(|e| e.base().id).collect()
}

#[test]
fn zincir_joins_the_chain_of_the_object_clicked_and_says_where_it_stopped() {
    let mut b = bench();
    b.start("join");
    assert!(b.type_text("Z"));
    assert_eq!(
        b.session.prompt().text(),
        "Birleştir: zincirin bir nesnesine tıklayın [uç boşluğu toleransı 0.001 m; değiştirmek için sayı yazın; Zincir (Z): açık]"
    );
    let before = b.log.len();
    b.click(-10.0, -2.5);
    assert!(!b.session.is_running(), "the tool leaves");
    assert_eq!(ids(&b), [2, 4, 5, 6]);
    let Some(Entity::Polyline(p)) = b.doc.entities().find(|e| e.base().id == 2) else {
        panic!("line 2 became the polyline");
    };
    let pts: Vec<[f64; 2]> = p.pts.iter().map(|q| rel(*q)).collect();
    let near = |a: [f64; 2], w: [f64; 2]| (a[0] - w[0]).hypot(a[1] - w[1]) < 1e-9;
    let want = [[-25.0, -5.0], [-15.0, -5.0], [-5.0, 0.0], [5.0, 0.0]];
    assert!(
        pts.len() == 4 && pts.iter().zip(want).all(|(a, w)| near(*a, w)),
        "{pts:?}"
    );
    let bulges = p.bulges.clone().unwrap_or_default();
    assert!(
        (bulges[2] + 0.414_213_562_373_095).abs() < 1e-9,
        "{bulges:?}"
    );
    assert_eq!(
        b.said(before)[b.said(before).len() - 2..],
        [
            (Level::Success, "3 nesne birleştirildi: çoklu çizgi."),
            (Level::Warn, "Zincir kilitli katmandaki bir nesnede durdu."),
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Birleştir"));
    assert_eq!(ids(&b), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn a_locked_or_lonely_object_is_said_and_another_may_be_clicked() {
    let mut b = bench();
    b.memory.join_chain = true;
    b.start("join");
    let before = b.log.len();
    // The locked old road.
    b.click(-26.5, -6.5);
    assert!(b.session.is_running(), "the tool waits for another click");
    assert_eq!(
        b.said(before),
        [(Level::Warn, "Kilitli katmandaki nesne birleştirilemez.")]
    );
    // Off: a click selects as usual.
    assert!(b.type_text("Z"));
    b.click(10.0, -2.5);
    assert_eq!(b.selected(), [4]);
}
