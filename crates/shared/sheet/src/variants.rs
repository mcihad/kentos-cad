//! Layout variants (design §3.2a). Constraints carry a layout from one size
//! of paper to another; they cannot carry it from one orientation to the
//! other (a strip down the right of a landscape sheet is a band along the
//! bottom of a portrait one). So a sheet, and a master page, keeps a layout
//! per kind of paper.
//!
//! - `Item.frame` is always the frame on the current paper; drawing reads
//!   only that.
//! - When the paper changes ([`settle`]) the first variant whose condition
//!   the paper meets is put in force, or the base layout when none does.
//!   Each item comes from the layout that names it, moved from that layout's
//!   reference paper by its constraints; an item a variant does not name
//!   comes from the base layout.
//! - While a variant is in force the base layout waits in `base_layout`, and
//!   edits ([`write_through`]) are recorded in the variant too, moved back to
//!   its reference paper by the same constraints: a fix made on portrait never
//!   disturbs landscape.
//!
//! A reference paper takes the margins of the paper it is carried to: a
//! layout keeps its distances from the margins, whatever they were.

use std::collections::{BTreeMap, BTreeSet};

use crate::layout;
use crate::model::{BaseLayout, Item, LayoutVariant, Page, Sheet, VariantFrame};
use crate::units::SizeUm;

/// The layout state of a sheet or a master page, borrowed.
pub struct LayoutMut<'a> {
    pub items: &'a mut Vec<Item>,
    pub variants: &'a mut Vec<LayoutVariant>,
    pub active: &'a mut Option<String>,
    pub base: &'a mut Option<BaseLayout>,
}

impl<'a> LayoutMut<'a> {
    pub fn of_sheet(s: &'a mut Sheet) -> LayoutMut<'a> {
        LayoutMut {
            items: &mut s.items,
            variants: &mut s.variants,
            active: &mut s.active_variant,
            base: &mut s.base_layout,
        }
    }

    pub fn of_master(m: &'a mut crate::model::Master) -> LayoutMut<'a> {
        LayoutMut {
            items: &mut m.items,
            variants: &mut m.variants,
            active: &mut m.active_variant,
            base: &mut m.base_layout,
        }
    }
}

/// The variant a paper calls for: the first whose condition it meets.
pub fn choose<'a>(variants: &'a [LayoutVariant], page: &Page) -> Option<&'a LayoutVariant> {
    variants.iter().find(|v| v.when.matches(page))
}

/// The variant a paper would put in force on a sheet or master page (none: its base layout).
pub fn variant_for<'a>(
    book: &'a crate::model::SheetBook,
    owner: &crate::model::Owner,
    page: &Page,
) -> crate::error::Result<Option<&'a LayoutVariant>> {
    let variants = match owner.kind {
        crate::model::OwnerKind::Sheet => book.sheet(&owner.id).map(|s| &s.variants),
        crate::model::OwnerKind::Master => book.master(&owner.id).map(|m| &m.variants),
    }
    .ok_or_else(|| {
        crate::error::SheetError::new("unknown_sheet", format!("“{}” kitapta yok.", owner.id))
    })?;
    Ok(choose(variants, page))
}

/// Every item's place as it is: the record a new variant or the base layout keeps.
pub fn record(items: &[Item]) -> Vec<VariantFrame> {
    items
        .iter()
        .map(|it| VariantFrame {
            item: it.id.clone(),
            frame: it.frame,
            rotation: it.rotation,
            constraints: it.constraints,
            hidden: it.hidden,
        })
        .collect()
}

/// A layout's reference paper, with the margins of the paper it is carried to.
fn reference(size: SizeUm, to: &Page) -> Page {
    Page { size, ..to.clone() }
}

fn put(it: &mut Item, f: &VariantFrame) {
    it.frame = f.frame;
    it.rotation = f.rotation;
    it.constraints = f.constraints;
    it.hidden = f.hidden;
}

fn depth(items: &[Item], i: usize) -> usize {
    let mut d = 0;
    let mut up = items[i].group.as_deref();
    while let Some(g) = up {
        d += 1;
        if d > items.len() {
            break;
        }
        up = items
            .iter()
            .find(|x| x.id == g)
            .and_then(|x| x.group.as_deref());
    }
    d
}

/// The items as a layout's record places them on `to`, and which those are:
/// the named ones from its reference paper by their recorded constraints,
/// with the unnamed items of a named group carried along with it (the others
/// are left where they were and must not be read).
fn placed(
    items: &[Item],
    frames: &[VariantFrame],
    size: SizeUm,
    to: &Page,
) -> (Vec<Item>, BTreeSet<String>) {
    let mut copy = items.to_vec();
    let mut done: BTreeSet<String> = BTreeSet::new();
    for f in frames {
        if let Some(it) = copy.iter_mut().find(|i| i.id == f.item) {
            put(it, f);
            done.insert(f.item.clone());
        }
    }
    // Parents first: an unnamed item inside a placed group keeps its place in the group.
    let mut order: Vec<usize> = (0..copy.len()).collect();
    order.sort_by_key(|&i| (depth(&copy, i), i));
    for i in order {
        if done.contains(&copy[i].id) {
            continue;
        }
        let Some(g) = copy[i].group.clone() else {
            continue;
        };
        if !done.contains(&g) {
            continue;
        }
        let (Some(old), Some(new)) = (
            items.iter().find(|x| x.id == g).map(|x| x.frame),
            copy.iter().find(|x| x.id == g).map(|x| x.frame),
        ) else {
            continue;
        };
        copy[i].frame = layout::relayout_frame(&copy[i].frame, &copy[i].constraints, &old, &new);
        done.insert(copy[i].id.clone());
    }
    crate::validate::group_frames(&mut copy);
    layout::relayout(&mut copy, &reference(size, to), to);
    (copy, done)
}

/// Puts the items on `to` (they are on `from`), with `target` in force (none:
/// the base layout). The caller sets the page.
pub fn switch_to(l: &mut LayoutMut<'_>, target: Option<&str>, from: &Page, to: &Page) {
    let target = target.filter(|id| l.variants.iter().any(|v| v.id == *id));
    if l.active.as_deref() == target {
        layout::relayout(l.items, from, to);
        return;
    }
    // The base layout: set aside now, or kept since a variant came into force.
    let base = match l.base.take() {
        Some(b) if l.active.is_some() => b,
        _ => BaseLayout {
            reference: from.size,
            frames: record(l.items),
        },
    };
    let mut now = l.items.clone();
    layout::relayout(&mut now, from, to);
    let (from_base, in_base) = placed(l.items, &base.frames, base.reference, to);
    let from_variant = target
        .and_then(|id| l.variants.iter().find(|v| v.id == id))
        .map(|v| placed(l.items, &v.frames, v.reference, to));
    for (i, it) in l.items.iter_mut().enumerate() {
        let src = match &from_variant {
            Some((fv, named)) if named.contains(&it.id) => &fv[i],
            _ if in_base.contains(&it.id) => &from_base[i],
            _ => &now[i],
        };
        let f = VariantFrame {
            item: String::new(),
            frame: src.frame,
            rotation: src.rotation,
            constraints: src.constraints,
            hidden: src.hidden,
        };
        put(it, &f);
    }
    crate::validate::group_frames(l.items);
    *l.active = target.map(str::to_owned);
    *l.base = target.map(|_| base);
}

/// Puts the items on `to` with the layout `to` calls for.
pub fn settle(l: &mut LayoutMut<'_>, from: &Page, to: &Page) {
    let target = choose(l.variants, to).map(|v| v.id.clone());
    switch_to(l, target.as_deref(), from, to);
}

/// Items a master page draws on a sheet's paper: as its layout for that paper places them.
pub fn arranged(master: &crate::model::Master, to: &Page) -> Vec<Item> {
    let mut m = master.clone();
    let page = m.page.clone();
    settle(&mut LayoutMut::of_master(&mut m), &page, to);
    m.items
}

/// Layout records of items that are gone are dropped.
pub fn purge(l: &mut LayoutMut<'_>) {
    let ids: std::collections::BTreeSet<&str> = l.items.iter().map(|i| i.id.as_str()).collect();
    for v in l.variants.iter_mut() {
        v.frames.retain(|f| ids.contains(f.item.as_str()));
    }
    if let Some(b) = l.base.as_mut() {
        b.frames.retain(|f| ids.contains(f.item.as_str()));
    }
}

/// After an edit on `page`: what changed (or is new) is recorded in the
/// variant in force, moved back to its reference paper; records of removed
/// items are dropped.
pub fn write_through(l: &mut LayoutMut<'_>, before: &[Item], page: &Page) {
    if let Some(id) = l.active.clone()
        && let Some(v) = l.variants.iter_mut().find(|v| v.id == id)
    {
        let changed: Vec<usize> = l
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| match before.iter().find(|b| b.id == it.id) {
                None => true,
                Some(b) => {
                    b.frame != it.frame
                        || b.rotation != it.rotation
                        || b.constraints != it.constraints
                        || b.hidden != it.hidden
                }
            })
            .map(|(i, _)| i)
            .collect();
        if !changed.is_empty() {
            let back = layout::relayout_items(l.items, page, &reference(v.reference, page));
            for i in changed {
                let it = &l.items[i];
                let f = VariantFrame {
                    item: it.id.clone(),
                    frame: back[i],
                    rotation: it.rotation,
                    constraints: it.constraints,
                    hidden: it.hidden,
                };
                match v.frames.iter_mut().find(|x| x.item == it.id) {
                    Some(x) => *x = f,
                    None => v.frames.push(f),
                }
            }
        }
    }
    purge(l);
}

/// Item ids renamed in the layout records (a copied sheet, a template used).
pub fn remap(
    variants: &mut [LayoutVariant],
    base: &mut Option<BaseLayout>,
    ids: &BTreeMap<String, String>,
) {
    let re = |frames: &mut Vec<VariantFrame>| {
        for f in frames {
            if let Some(n) = ids.get(&f.item) {
                f.item = n.clone();
            }
        }
    };
    for v in variants {
        re(&mut v.frames);
    }
    if let Some(b) = base {
        re(&mut b.frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{GroupItem, ItemKind, RectShape, ShapeItem, ShapeKind};
    use crate::model::{
        ConstraintBox, Constraints, HConstraint, Orientation, Paper, VConstraint, VariantCondition,
    };
    use crate::units::{Margins, RectUm};

    fn page(o: Orientation, w: i32, h: i32) -> Page {
        Page {
            paper: Paper::Custom,
            orientation: o,
            size: SizeUm {
                width: w,
                height: h,
            },
            margins: Margins {
                top: 10_000,
                right: 10_000,
                bottom: 10_000,
                left: 10_000,
            },
            background: None,
        }
    }

    fn shape(id: &str, f: RectUm, c: Constraints) -> Item {
        let mut it = Item::new(
            id,
            id,
            f,
            ItemKind::Shape(ShapeItem {
                shape: ShapeKind::Rect(RectShape { radius: 0 }),
                stroke: None,
                fill: None,
            }),
        );
        it.constraints = c;
        it
    }

    fn frame(id: &str, f: RectUm, c: Constraints) -> VariantFrame {
        VariantFrame {
            item: id.into(),
            frame: f,
            rotation: 0,
            constraints: c,
            hidden: false,
        }
    }

    const STRETCH: Constraints = Constraints::new(HConstraint::LeftRight, VConstraint::TopBottom);
    const RIGHT: Constraints = Constraints::new(HConstraint::Right, VConstraint::TopBottom);
    const BOTTOM: Constraints = Constraints::new(HConstraint::LeftRight, VConstraint::Bottom);

    /// A landscape sheet (400 × 280) with a map and a strip on the right, and a portrait variant
    /// (200 × 280) with the strip along the bottom.
    fn sheet() -> (Vec<Item>, Vec<LayoutVariant>) {
        let items = vec![
            shape(
                "harita",
                RectUm::new(10_000, 10_000, 290_000, 260_000),
                STRETCH,
            ),
            shape(
                "serit",
                RectUm::new(300_000, 10_000, 90_000, 260_000),
                RIGHT,
            ),
        ];
        let variants = vec![LayoutVariant {
            id: "dikey".into(),
            name: "Dikey".into(),
            when: VariantCondition {
                orientation: Orientation::Portrait,
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
            },
            reference: SizeUm {
                width: 200_000,
                height: 280_000,
            },
            frames: vec![
                frame(
                    "harita",
                    RectUm::new(10_000, 10_000, 180_000, 180_000),
                    STRETCH,
                ),
                frame(
                    "serit",
                    RectUm::new(10_000, 190_000, 180_000, 80_000),
                    BOTTOM,
                ),
            ],
        }];
        (items, variants)
    }

    #[test]
    fn a_portrait_paper_puts_the_portrait_layout_in_force_and_back() {
        let (mut items, mut variants) = sheet();
        let (mut active, mut base) = (None, None);
        let land = page(Orientation::Landscape, 400_000, 280_000);
        // A4-like portrait, larger than the reference: the strip keeps its height at the bottom, the map stretches.
        let port = page(Orientation::Portrait, 210_000, 297_000);
        let original = items.clone();
        let mut l = LayoutMut {
            items: &mut items,
            variants: &mut variants,
            active: &mut active,
            base: &mut base,
        };
        settle(&mut l, &land, &port);
        assert_eq!(l.active.as_deref(), Some("dikey"));
        // harita: both edges kept from the margins: 10..200 across, 10..207 down (297 − 10 − (280 − 10 − 190)).
        assert_eq!(
            l.items[0].frame,
            RectUm::new(10_000, 10_000, 190_000, 197_000)
        );
        // serit: bottom kept (10 from the margin), height 80: top 297 − 10 − 80 = 207.
        assert_eq!(
            l.items[1].frame,
            RectUm::new(10_000, 207_000, 190_000, 80_000)
        );
        assert_eq!(l.items[1].constraints, BOTTOM);
        assert!(l.base.is_some());
        settle(&mut l, &port, &land);
        assert_eq!(*l.active, None);
        assert_eq!(*l.base, None);
        assert_eq!(*l.items, original, "the base layout comes back exactly");
    }

    #[test]
    fn an_edit_on_portrait_is_kept_by_portrait_and_does_not_touch_landscape() {
        let (mut items, mut variants) = sheet();
        let (mut active, mut base) = (None, None);
        let land = page(Orientation::Landscape, 400_000, 280_000);
        let port = page(Orientation::Portrait, 200_000, 280_000);
        let original = items.clone();
        let mut l = LayoutMut {
            items: &mut items,
            variants: &mut variants,
            active: &mut active,
            base: &mut base,
        };
        settle(&mut l, &land, &port);
        let before = l.items.clone();
        l.items[1].frame = RectUm::new(10_000, 200_000, 180_000, 70_000);
        write_through(&mut l, &before, &port);
        assert_eq!(
            l.variants[0].frames[1].frame,
            RectUm::new(10_000, 200_000, 180_000, 70_000)
        );
        settle(&mut l, &port, &land);
        assert_eq!(*l.items, original);
        settle(&mut l, &land, &port);
        assert_eq!(
            l.items[1].frame,
            RectUm::new(10_000, 200_000, 180_000, 70_000)
        );
    }

    #[test]
    fn items_a_variant_does_not_name_come_from_the_base_layout() {
        let (mut items, mut variants) = sheet();
        items.push(shape(
            "logo",
            RectUm::new(320_000, 20_000, 30_000, 30_000),
            Constraints::new(HConstraint::Right, VConstraint::Top),
        ));
        let (mut active, mut base) = (None, None);
        let land = page(Orientation::Landscape, 400_000, 280_000);
        let port = page(Orientation::Portrait, 200_000, 280_000);
        let mut l = LayoutMut {
            items: &mut items,
            variants: &mut variants,
            active: &mut active,
            base: &mut base,
        };
        settle(&mut l, &land, &port);
        // From the base layout by its own constraints: 40 from the margin's right edge on
        // landscape (390 − 350), so 190 − 40 − 30 = 120 on portrait.
        assert_eq!(
            l.items[2].frame,
            RectUm::new(120_000, 20_000, 30_000, 30_000)
        );
    }

    #[test]
    fn a_group_follows_the_constraints_its_layout_gives_it() {
        let mut items = vec![
            shape(
                "a",
                RectUm::new(300_000, 10_000, 90_000, 100_000),
                Constraints {
                    h: HConstraint::Scale,
                    v: VConstraint::Scale,
                    relative_to: ConstraintBox::Group,
                },
            ),
            shape(
                "b",
                RectUm::new(300_000, 110_000, 90_000, 160_000),
                Constraints {
                    h: HConstraint::Scale,
                    v: VConstraint::Scale,
                    relative_to: ConstraintBox::Group,
                },
            ),
        ];
        items[0].group = Some("g".into());
        items[1].group = Some("g".into());
        let mut g = Item::new(
            "g",
            "g",
            RectUm::new(300_000, 10_000, 90_000, 260_000),
            ItemKind::Group(GroupItem {}),
        );
        g.constraints = RIGHT;
        items.push(g);
        let variants = vec![LayoutVariant {
            id: "dikey".into(),
            name: "Dikey".into(),
            when: VariantCondition {
                orientation: Orientation::Portrait,
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
            },
            reference: SizeUm {
                width: 200_000,
                height: 280_000,
            },
            frames: vec![frame(
                "g",
                RectUm::new(10_000, 190_000, 180_000, 80_000),
                BOTTOM,
            )],
        }];
        let (mut active, mut base, mut variants) = (None, None, variants);
        let mut l = LayoutMut {
            items: &mut items,
            variants: &mut variants,
            active: &mut active,
            base: &mut base,
        };
        let land = page(Orientation::Landscape, 400_000, 280_000);
        let port = page(Orientation::Portrait, 200_000, 280_000);
        settle(&mut l, &land, &port);
        // The group is a band along the bottom; its children keep their proportions inside it.
        assert_eq!(
            l.items[2].frame,
            RectUm::new(10_000, 190_000, 180_000, 80_000)
        );
        // a: the top 100/260 of the group, the full width.
        let a = l.items[0].frame;
        assert_eq!((a.left, a.top, a.width), (10_000, 190_000, 180_000));
        assert_eq!(a.height, 30_769);
    }
}
