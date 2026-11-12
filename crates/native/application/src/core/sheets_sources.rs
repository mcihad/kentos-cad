//! A sheet page's sources (docs/adr/1320 §2): the objects its viewports draw or its tables read, by persistent id, and
//! the content digests a publication records of them and of the project parts every page draws with, so
//! `file.publish.status` can say what changed since a delivery and `file.publish.sheets` with `previous` can tell the
//! pages to make again from the ones to carry. Read-only; the frame tests are `cad.sheets.set`'s (`sheets_set`).

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{Entity, Vec2, ViewTarget, encoding::sha256_hex};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::arrange::object_bounds;
use kentos_sheet::kinds::{ItemKind, TableSource};
use kentos_sheet::model::SheetBook;

use crate::geometry::{drawing_font, shape};
use crate::sheets_set;

/// The object digest's name and version, written beside the digests.
pub const OBJECT_DIGEST: &str = "kentos.object-digest/1";

/// A JSON value's digest: SHA-256 of its text with every object's keys sorted and no whitespace. `serde_json`'s map is
/// ordered by key (no `preserve_order`), so the text is the canonical one an independent reader rebuilds with
/// `json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False)`.
fn value_digest(v: &serde_json::Value) -> String {
    sha256_hex(&serde_json::to_vec(v).unwrap_or_default())
}

/// An object's content digest ([`OBJECT_DIGEST`]): its snapshot form without its slot `id`, which a reload may renumber
/// (the persistent id names the object).
pub fn object_digest(e: &Entity) -> String {
    let mut v = serde_json::to_value(e).unwrap_or_default();
    if let Some(m) = v.as_object_mut() {
        m.remove("id");
    }
    value_digest(&v)
}

/// The digests of what every page draws with besides its objects: the project settings, layers, styles, block
/// definitions and the sheet book without its sheets (masters, templates, profiles, page setups), by name.
pub fn part_digests(doc: &Document, book: Option<&SheetBook>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut put = |name: &str, v: serde_json::Value| {
        out.insert(name.to_owned(), value_digest(&v));
    };
    put(
        "settings",
        serde_json::to_value(doc.settings()).unwrap_or_default(),
    );
    put(
        "layers",
        serde_json::to_value(doc.layers().nodes()).unwrap_or_default(),
    );
    put(
        "styles",
        serde_json::to_value(doc.styles()).unwrap_or_default(),
    );
    put(
        "blocks",
        serde_json::to_value(doc.blocks().iter().map(|b| &**b).collect::<Vec<_>>())
            .unwrap_or_default(),
    );
    let mut rest = serde_json::to_value(book.cloned().unwrap_or_default()).unwrap_or_default();
    if let Some(m) = rest.as_object_mut() {
        m.remove("sheets");
    }
    put("book", rest);
    out
}

/// A sheet's own record's digest (its items, page, drawing information), none when the book has no such sheet.
pub fn sheet_digest(book: &SheetBook, id: &str) -> Option<String> {
    book.sheet(id)
        .map(|s| value_digest(&serde_json::to_value(s).unwrap_or_default()))
}

/// Every object once, in drawing order: its persistent id, box, slot and itself.
type Listed<'a> = Vec<(String, Bounds, Slot, &'a Entity)>;

fn listed(doc: &Document) -> Listed<'_> {
    let blocks = crate::blocks::drawing_blocks(doc);
    let font = drawing_font(doc.settings().drawing_font);
    doc.entities_with_uids()
        .filter_map(|(u, e)| {
            let slot = doc.slot_of(u)?;
            Some((u.to_string(), object_bounds(&shape(e), &blocks, font), slot, e))
        })
        .collect()
}

/// The objects a page of `sheet` draws or reads, by their place in `objects` (ADR 1320 §2's rule): each map's ground
/// met and printed, the model tables it shows read, the view marks pointing at it, a paper attribute table's layer.
/// On an atlas page (`atlas`, docs/adr/3600) each map's ground is the one the page draws it at, and a map the page
/// does not draw shows nothing; on a sheet's page an atlas map, like an unplaced one, has no ground. None when a
/// ground is not known.
fn sheet_uses(
    doc: &Document,
    book: &SheetBook,
    sheet: &kentos_sheet::model::Sheet,
    objects: &Listed<'_>,
    atlas: Option<&[(String, [f64; 4])]>,
) -> Option<HashSet<usize>> {
    let named: Vec<(usize, Slot, &Entity, &String)> = objects.iter().enumerate().map(|(i, (u, _, s, e))| (i, *s, *e, u)).collect();
    let index: std::collections::HashMap<&str, usize> = objects.iter().enumerate().map(|(i, (u, ..))| (u.as_str(), i)).collect();
    let views = doc.settings().drawn_views();
    let mut uses: HashSet<usize> = HashSet::new();
    for item in &sheet.items {
        match &item.kind {
            ItemKind::Map(_) => {
                let ground = match atlas {
                    Some(page) => match page.iter().find(|(id, _)| *id == item.id) {
                        Some((_, b)) => *b,
                        None => continue,
                    },
                    None => kentos_sheet::model_scope::Viewport::of_with_views(item, &views).ok().and_then(|v| {
                        let corners: Vec<Vec2> =
                            v.transform().ground_corners(&v.content).map(|p| Vec2 { x: p[0], y: p[1] }).to_vec();
                        sheets_set::corners_box(&corners)
                    })?,
                };
                let draws = sheets_set::drawn_by(doc, &sheet.id, &item.id, Some(book));
                let target = ViewTarget::sheet(&sheet.id, &item.id);
                for (i, (u, b, _, e)) in objects.iter().enumerate() {
                    if sheets_set::meets(b, &ground) && draws(u, b) {
                        uses.insert(i);
                        if matches!(e, Entity::Table(_)) {
                            for r in sheets_set::reads_of(doc, e, &named) {
                                if let Some(j) = index.get(r.as_str()) {
                                    uses.insert(*j);
                                }
                            }
                        }
                    }
                    if matches!(e, Entity::ViewMark(m) if m.mark.target.as_ref() == Some(&target)) {
                        uses.insert(i);
                    }
                }
            }
            ItemKind::Table(t) => {
                if let TableSource::Layer(l) = &t.source {
                    for (i, (.., e)) in objects.iter().enumerate() {
                        if e.base().layer_id == l.layer {
                            uses.insert(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Some(uses)
}

/// The sources of each of `chosen`'s pages, in drawing order: the objects whose box meets a viewport's ground and that
/// the viewport prints (its level, phase, categories, layer state; `cad.sheets.set`'s `shows`), the objects the model
/// tables it shows read (`table`), the view marks pointing at it (`marked`), and every object on a layer a paper
/// attribute table lists. A box test lists more objects than a page draws, never fewer. None for a page with a
/// viewport whose ground is not known (an atlas viewport, an unplaced one): its sources cannot be listed.
pub fn page_sources(
    doc: &Document,
    chosen: &[String],
) -> Result<Vec<(String, Option<Vec<String>>)>, (String, String)> {
    let Some(kept) = doc.sheets() else {
        return Ok(chosen.iter().map(|id| (id.clone(), Some(Vec::new()))).collect());
    };
    let book = kentos_sheet::project::read_project_sheets(kept).map_err(|e| ("book_unreadable".to_owned(), e.message))?.0;
    // One pass a viewport over the objects, never an object against every other (cad.sheets.set's list is for a few
    // changed objects).
    let objects = listed(doc);
    let mut out = Vec::with_capacity(chosen.len());
    for id in chosen {
        let Some(sheet) = book.sheet(id) else {
            out.push((id.clone(), Some(Vec::new())));
            continue;
        };
        let listed = sheet_uses(doc, &book, sheet, &objects, None).map(|uses| {
            let mut order: Vec<usize> = uses.into_iter().collect();
            order.sort_unstable();
            order.into_iter().map(|i| objects[i].0.clone()).collect()
        });
        out.push((id.clone(), listed));
    }
    Ok(out)
}

/// The sources of each page of `sheet`'s atlas (docs/adr/3600; ADR 1320 §2's rule a page at a time): each page is its
/// own object (`feature`, first) and the maps it draws with the ground each shows on the project's ground (its atlas
/// maps where it puts them, an overview where it follows them, the maps that stand still); the objects those grounds
/// meet that the map prints, the tables and marks as on a sheet's page, in drawing order after the page's object.
pub fn atlas_page_sources(
    doc: &Document,
    sheet: &str,
    pages: &[(Vec<(String, [f64; 4])>, String)],
) -> Result<Vec<Option<Vec<String>>>, (String, String)> {
    let Some(kept) = doc.sheets() else {
        return Ok(pages.iter().map(|_| None).collect());
    };
    let book = kentos_sheet::project::read_project_sheets(kept).map_err(|e| ("book_unreadable".to_owned(), e.message))?.0;
    let Some(sheet) = book.sheet(sheet) else {
        return Ok(pages.iter().map(|_| None).collect());
    };
    let objects = listed(doc);
    Ok(pages
        .iter()
        .map(|(grounds, feature)| {
            sheet_uses(doc, &book, sheet, &objects, Some(grounds)).map(|uses| {
                let mut order: Vec<usize> = uses.into_iter().collect();
                order.sort_unstable();
                std::iter::once(feature.clone())
                    .chain(order.into_iter().map(|i| objects[i].0.clone()).filter(|u| u != feature))
                    .collect()
            })
        })
        .collect())
}

/// The digests of the objects named by `uids` that are in the drawing, by persistent id.
pub fn object_digests<'a>(
    doc: &Document,
    uids: impl IntoIterator<Item = &'a String>,
) -> BTreeMap<String, String> {
    uids.into_iter()
        .filter_map(|u| {
            let slot = kentos_domain::Uuid::parse_str(u)
                .ok()
                .and_then(|x| doc.slot_of(x))?;
            Some((u.clone(), object_digest(doc.get(slot)?)))
        })
        .collect()
}
