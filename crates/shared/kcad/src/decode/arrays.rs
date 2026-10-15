//! The associative arrays (document schema 53, docs/specs/kcad-v2.md §6.14,
//! docs/adr/0300): read as the writer writes them; the contract's rules
//! (`arrays_problem`) and the items' inserts are checked once the blocks and
//! the objects are known (`check`).

use kentos_contracts::{
    ArrayFault, ArrayId, ArrayItem, ArrayLayout, AssocArray, BlockId, EntityId, arrays_problem,
};

use super::{id16, list, map, named, point, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};

/// The most places a layout's count says (the contract's largest).
const COUNT_MAX: u64 = kentos_contracts::ARRAY_PLACES_MAX as u64;

fn layout(r: &mut Reader<'_>) -> Result<ArrayLayout, KcadError> {
    #[derive(Clone, Copy)]
    enum Kind {
        Grid,
        Polar,
        Line,
        Path,
    }
    let (mut kind, mut rows, mut cols, mut dx, mut dy) = (None, None, None, None, None);
    let (mut center, mut count, mut fill, mut rotate, mut fit) = (None, None, None, None, false);
    let (mut path, mut spacing, mut align) = (None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                kind = Some(named(
                    r,
                    &[
                        ("grid", Kind::Grid),
                        ("polar", Kind::Polar),
                        ("line", Kind::Line),
                        ("path", Kind::Path),
                    ],
                )?)
            }
            "rows" => rows = Some(r.uint(COUNT_MAX)? as u32),
            "cols" => cols = Some(r.uint(COUNT_MAX)? as u32),
            "count" => count = Some(r.uint(COUNT_MAX)? as u32),
            "dx" => dx = Some(r.float()?),
            "dy" => dy = Some(r.float()?),
            "fill" => fill = Some(r.float()?),
            "spacing" => spacing = Some(r.float()?),
            "center" => center = Some(point(r)?),
            "rotate" => rotate = Some(r.bool()?),
            "align" => align = Some(r.bool()?),
            "fit" => {
                if !r.bool()? {
                    return Err(r.fail(Code::BadValue, "fit yalnız doğruyken yazılır"));
                }
                fit = true;
            }
            "path" => path = Some(EntityId(id16(r)?).to_text()),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    // Only the kind's own fields: another kind's field is the writer's mistake.
    let extra = |r: &mut Reader<'_>, given: bool, what: &str| -> Result<(), KcadError> {
        if given {
            return Err(r.fail(Code::BadValue, &format!("{what} bu yerleşimin alanı değil")));
        }
        Ok(())
    };
    Ok(match required(r, kind, "kind")? {
        Kind::Grid => {
            extra(
                r,
                center.is_some() || count.is_some() || fill.is_some(),
                "center/count/fill",
            )?;
            extra(
                r,
                rotate.is_some() || fit || path.is_some(),
                "rotate/fit/path",
            )?;
            extra(r, spacing.is_some() || align.is_some(), "spacing/align")?;
            ArrayLayout::Grid {
                rows: required(r, rows, "rows")?,
                cols: required(r, cols, "cols")?,
                dx: required(r, dx, "dx")?,
                dy: required(r, dy, "dy")?,
            }
        }
        Kind::Polar => {
            extra(
                r,
                rows.is_some() || cols.is_some() || dx.is_some(),
                "rows/cols/dx",
            )?;
            extra(r, dy.is_some() || fit || path.is_some(), "dy/fit/path")?;
            extra(r, spacing.is_some() || align.is_some(), "spacing/align")?;
            ArrayLayout::Polar {
                center: required(r, center, "center")?,
                count: required(r, count, "count")?,
                fill: required(r, fill, "fill")?,
                rotate: required(r, rotate, "rotate")?,
            }
        }
        Kind::Line => {
            extra(
                r,
                rows.is_some() || cols.is_some() || center.is_some(),
                "rows/cols/center",
            )?;
            extra(
                r,
                fill.is_some() || rotate.is_some() || path.is_some(),
                "fill/rotate/path",
            )?;
            extra(r, spacing.is_some() || align.is_some(), "spacing/align")?;
            ArrayLayout::Line {
                count: required(r, count, "count")?,
                dx: required(r, dx, "dx")?,
                dy: required(r, dy, "dy")?,
                fit,
            }
        }
        Kind::Path => {
            extra(
                r,
                rows.is_some() || cols.is_some() || center.is_some(),
                "rows/cols/center",
            )?;
            extra(
                r,
                fill.is_some() || rotate.is_some() || fit,
                "fill/rotate/fit",
            )?;
            extra(r, dx.is_some() || dy.is_some(), "dx/dy")?;
            ArrayLayout::Path {
                path: required(r, path, "path")?,
                count: required(r, count, "count")?,
                spacing,
                align: required(r, align, "align")?,
            }
        }
    })
}

fn item(r: &mut Reader<'_>) -> Result<ArrayItem, KcadError> {
    let mut out = ArrayItem::default();
    map(r, |r, key| {
        match key {
            "uid" => out.uid = Some(EntityId(id16(r)?)),
            "kept" => {
                if !r.bool()? {
                    return Err(r.fail(Code::BadValue, "kept yalnız doğruyken yazılır"));
                }
                out.kept = true;
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(out)
}

fn array(r: &mut Reader<'_>) -> Result<AssocArray, KcadError> {
    let (mut p, mut id, mut block, mut items, mut scale) = (None, None, None, None, None);
    let (mut layout_, mut mirror, mut layer_id, mut rotation) = (None, false, None, None);
    map(r, |r, key| {
        match key {
            "p" => p = Some(point(r)?),
            "id" => id = Some(ArrayId(id16(r)?)),
            "block" => block = Some(BlockId(id16(r)?)),
            "items" => items = Some(list(r, |r, _| item(r))?),
            "scale" => scale = Some(r.float()?),
            "layout" => layout_ = Some(layout(r)?),
            "mirror" => {
                if !r.bool()? {
                    return Err(r.fail(Code::BadValue, "mirror yalnız doğruyken yazılır"));
                }
                mirror = true;
            }
            "layerId" => layer_id = Some(text(r)?),
            "rotation" => rotation = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(AssocArray {
        id: required(r, id, "id")?,
        block: required(r, block, "block")?,
        layer_id: required(r, layer_id, "layerId")?,
        p: required(r, p, "p")?,
        scale: required(r, scale, "scale")?,
        rotation: required(r, rotation, "rotation")?,
        mirror,
        layout: required(r, layout_, "layout")?,
        items: required(r, items, "items")?,
    })
}

pub(super) fn arrays(r: &mut Reader<'_>) -> Result<Vec<AssocArray>, KcadError> {
    let at = r.position();
    let out = list(r, |r, _| array(r))?;
    if out.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş dizi listesi yazılmaz"));
    }
    Ok(out)
}

/// The rules once the blocks and the objects are known: the contract's, and every item's insert an object of
/// the drawing.
pub(super) fn check(
    r: &mut Reader<'_>,
    list: &[AssocArray],
    blocks: &[BlockId],
    uids: &[EntityId],
) -> Result<(), KcadError> {
    let fail = |r: &mut Reader<'_>, path: &[Seg<'static>], why: &str| {
        r.push(Seg::Name("arrays"));
        for seg in path {
            r.push(*seg);
        }
        let e = r.fail(Code::BadValue, why);
        for _ in 0..=path.len() {
            r.pop();
        }
        e
    };
    if let Some((at, why)) = arrays_problem(list, blocks) {
        let path = match at {
            ArrayFault::Array(i) => vec![Seg::Index(i)],
            ArrayFault::Item(i, k) => vec![Seg::Index(i), Seg::Name("items"), Seg::Index(k)],
        };
        return Err(fail(r, &path, why));
    }
    let known: std::collections::HashSet<&EntityId> = uids.iter().collect();
    for (i, a) in list.iter().enumerate() {
        for (k, item) in a.items.iter().enumerate() {
            if item.uid.is_some_and(|u| !known.contains(&u)) {
                let path = [
                    Seg::Index(i),
                    Seg::Name("items"),
                    Seg::Index(k),
                    Seg::Name("uid"),
                ];
                return Err(fail(r, &path, "öğenin yerleştirmesi çizimde yok"));
            }
        }
    }
    Ok(())
}
