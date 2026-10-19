//! Arrays and maps (docs/adr/0600 §3) on the language's marked JSON (`compound`). Items compare by `=`'s rule and
//! group by `arrays::Key`. An argument that should be an array or a map and is not gives an empty value.

use std::collections::HashMap;

use crate::arrays::{Key, give_array, give_map, place};
use crate::compound::{Item, MOST_ITEMS, array_items, map_items, shown};
use crate::library::Func;
use crate::scalar::{R, Scratch, V, as_text, equals, to_number};

fn contains(items: &[Item], v: V, s: &mut Scratch) -> bool {
    items.iter().any(|it| equals(it.view(), v, s))
}

/// Each item's count, by key, and the keys in the order they first appear.
fn counts(items: &[Item]) -> (Vec<(Key, usize)>, Vec<usize>) {
    let mut at: HashMap<Key, usize> = HashMap::new();
    let mut out: Vec<(Key, usize)> = Vec::new();
    let mut first: Vec<usize> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let Some(k) = Key::of(it.view()) else {
            continue;
        };
        match at.get(&k) {
            Some(&j) => out[j].1 += 1,
            None => {
                at.insert(k.clone(), out.len());
                out.push((k, 1));
                first.push(i);
            }
        }
    }
    (out, first)
}

fn median(mut xs: Vec<f64>) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(f64::total_cmp);
    let n = xs.len();
    Some(if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    })
}

/// `dizi_en_sık` (most) or `dizi_en_seyrek` (least) with its option.
fn frequent(items: &[Item], option: &str, most: bool) -> Option<Item> {
    let (counts, first) = counts(items);
    if counts.is_empty() {
        return Some(Item::Null);
    }
    let total: usize = counts.iter().map(|c| c.1).sum();
    let picked: Vec<usize> = match option {
        "real_majority" if most => counts
            .iter()
            .position(|c| c.1 * 2 > total)
            .map_or_else(Vec::new, |i| vec![i]),
        "real_minority" if !most => (0..counts.len())
            .filter(|&i| counts[i].1 * 2 < total)
            .collect(),
        "all" | "any" | "median" => {
            let edge = if most {
                counts.iter().map(|c| c.1).max()?
            } else {
                counts.iter().map(|c| c.1).min()?
            };
            (0..counts.len()).filter(|&i| counts[i].1 == edge).collect()
        }
        _ => return None,
    };
    let of = |i: usize| items[first[i]].clone();
    Some(match option {
        "any" => picked.first().map_or(Item::Null, |&i| of(i)),
        "median" => {
            let xs: Vec<f64> = picked
                .iter()
                .filter_map(|&i| to_number(items[first[i]].view()))
                .collect();
            median(xs).map_or(Item::Null, Item::Num)
        }
        "real_majority" => picked.first().map_or(Item::Null, |&i| of(i)),
        _ => {
            let mut out = String::new();
            crate::compound::push_array(&mut out, picked.iter().map(|&i| &items[first[i]]));
            Item::Nested(out)
        }
    })
}

pub(super) fn call<'a>(f: Func, args: &[V<'a>], out: &mut String, s: &mut Scratch) -> R<'a> {
    let arg = |i: usize| super::arg(args, i);
    match f {
        Func::Series => {
            let step = if args.len() > 2 {
                to_number(arg(2))
            } else {
                Some(1.0)
            };
            let (Some(a), Some(b), Some(step)) = (to_number(arg(0)), to_number(arg(1)), step)
            else {
                return R::V(V::Null);
            };
            if step == 0.0 {
                return R::V(V::Null);
            }
            let n = ((b - a) / step).floor();
            // A step that goes away from the end is empty, as QGIS has it.
            if n < 0.0 {
                return R::V(V::Null);
            }
            if n >= MOST_ITEMS as f64 {
                return R::Thrown;
            }
            let items: Vec<Item> = (0..=n as usize)
                .map(|k| Item::Num(a + step * k as f64))
                .collect();
            return give_array(&items, out);
        }
        Func::MapConcat => {
            let mut pairs: Vec<(String, Item)> = Vec::new();
            for &m in args {
                let Some(more) = map_items(m) else {
                    return R::V(V::Null);
                };
                for (k, v) in more {
                    match pairs.iter_mut().find(|(key, _)| *key == k) {
                        Some(slot) => slot.1 = v,
                        None => pairs.push((k, v)),
                    }
                }
            }
            return give_map(&pairs, out);
        }
        Func::MapPrefixKeys => {
            let Some(pairs) = map_items(arg(0)) else {
                return R::V(V::Null);
            };
            let prefix = shown(as_text(arg(1), &mut s.a)).to_owned();
            let pairs: Vec<(String, Item)> = pairs
                .into_iter()
                .map(|(k, v)| (format!("{prefix}{k}"), v))
                .collect();
            return give_map(&pairs, out);
        }
        Func::UrlEncode => {
            let Some(pairs) = map_items(arg(0)) else {
                return R::V(V::Null);
            };
            let mark = out.len();
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push('&');
                }
                percent(out, k);
                out.push('=');
                let mut tmp = String::new();
                percent(out, shown(as_text(v.view(), &mut tmp)));
            }
            return super::made(out, mark);
        }
        _ => {}
    }
    let Some(mut items) = array_items(arg(0)) else {
        return R::V(V::Null);
    };
    match f {
        Func::ArrayAll => match array_items(arg(1)) {
            Some(wanted) => R::V(V::Bool(
                wanted.iter().all(|w| contains(&items, w.view(), s)),
            )),
            None => R::V(V::Null),
        },
        Func::ArrayIntersect => match array_items(arg(1)) {
            Some(other) => R::V(V::Bool(other.iter().any(|w| contains(&items, w.view(), s)))),
            None => R::V(V::Null),
        },
        Func::ArrayCount => {
            let v = arg(1);
            R::V(V::Num(
                items.iter().filter(|it| equals(it.view(), v, s)).count() as f64,
            ))
        }
        Func::ArrayInsert => {
            let Some(at) = to_number(arg(1)) else {
                return R::V(V::Null);
            };
            let at = if at < 0.0 {
                0
            } else {
                (at.trunc() as usize).min(items.len())
            };
            items.insert(at, Item::of(arg(2)));
            give_array(&items, out)
        }
        Func::ArrayPrepend => {
            items.insert(0, Item::of(arg(1)));
            give_array(&items, out)
        }
        Func::ArrayRemoveAt => {
            let Some(at) = to_number(arg(1)) else {
                return R::V(V::Null);
            };
            if let Some(i) = place(at, items.len()) {
                items.remove(i);
            }
            give_array(&items, out)
        }
        Func::ArrayRemoveAll => {
            let v = arg(1);
            items.retain(|it| !equals(it.view(), v, s));
            give_array(&items, out)
        }
        Func::ArrayReplace => {
            if args.len() == 2 {
                let Some(map) = map_items(arg(1)) else {
                    return R::V(V::Null);
                };
                for it in &mut items {
                    let mut tmp = String::new();
                    let key = shown(as_text(it.view(), &mut tmp)).to_owned();
                    if let Some((_, v)) = map.iter().find(|(k, _)| *k == key) {
                        *it = v.clone();
                    }
                }
                return give_array(&items, out);
            }
            let after = Item::of(arg(2));
            match (array_items(arg(1)), array_items(arg(2))) {
                (Some(before), Some(after)) => {
                    if before.len() != after.len() {
                        return R::V(V::Null);
                    }
                    for it in &mut items {
                        if let Some(k) = before.iter().position(|b| equals(b.view(), it.view(), s))
                        {
                            *it = after[k].clone();
                        }
                    }
                }
                (Some(before), None) => {
                    for it in &mut items {
                        if contains(&before, it.view(), s) {
                            *it = after.clone();
                        }
                    }
                }
                (None, _) => {
                    let b = arg(1);
                    for it in &mut items {
                        if equals(it.view(), b, s) {
                            *it = after.clone();
                        }
                    }
                }
            }
            give_array(&items, out)
        }
        Func::ArrayMedian => {
            let xs: Vec<f64> = items.iter().filter_map(|it| to_number(it.view())).collect();
            R::V(median(xs).map_or(V::Null, V::Num))
        }
        Func::ArrayMajority | Func::ArrayMinority => {
            let option = if args.len() > 1 {
                as_text(arg(1), &mut s.a).trim().to_ascii_lowercase()
            } else {
                "all".into()
            };
            match frequent(&items, &option, f == Func::ArrayMajority) {
                Some(it) => crate::arrays::give(it, out),
                None => R::V(V::Null),
            }
        }
        _ => R::V(V::Null),
    }
}

/// RFC 3986's percent encoding: the unreserved characters as they are, every other byte `%XX`.
fn percent(out: &mut String, t: &str) {
    for b in t.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit(u32::from(b >> 4), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit(u32::from(b & 15), 16)
                    .unwrap_or('0')
                    .to_ascii_uppercase(),
            );
        }
    }
}
