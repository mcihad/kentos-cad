//! Dizi ve eşleme işlevleri (docs/adr/0214 §2.5): what `dizi`, `dizi_öğe`,
//! `eşleme_değeri` … compute on the language's arrays and maps (`compound`:
//! marked JSON text). An argument that should be an array or a map and is
//! not gives an empty value. Items compare by `=`'s rule, order by `<`'s;
//! the numbers of `dizi_topla` and its kin follow `kentos.statistics/1`
//! (an exact sum).

use std::cmp::Ordering;
use std::collections::HashSet;

use kentos_geometry_core::jsmath::{js_max, js_min};
use kentos_geometry_core::ops::statistics::{self, Dec};

use crate::compound::{self, Item, MOST_ITEMS};
use crate::js::collate::compare_tr;
use crate::js::text::{MAX_STRING_UNITS, utf16_len};
use crate::library::Func;
use crate::read::push_number_text;
use crate::scalar::{self, R, Scratch, V, as_text, to_number};

/// Text made at `mark`, unless it is past JavaScript's longest string.
fn made(out: &str, mark: usize) -> R<'static> {
    if out.len() - mark > MAX_STRING_UNITS && utf16_len(&out[mark..]) > MAX_STRING_UNITS {
        R::Thrown
    } else {
        R::Made
    }
}

/// An item as a result: numbers and true/false as they are, text written.
fn give(it: Item, out: &mut String) -> R<'static> {
    match it {
        Item::Null => R::V(V::Null),
        Item::Num(x) => R::V(V::Num(x)),
        Item::Bool(b) => R::V(V::Bool(b)),
        Item::Text(s) | Item::Nested(s) => {
            out.push_str(&s);
            R::Made
        }
    }
}

/// An array of items as a result.
fn give_array(items: &[Item], out: &mut String) -> R<'static> {
    if items.len() > MOST_ITEMS {
        return R::Thrown;
    }
    let mark = out.len();
    compound::push_array(out, items);
    made(out, mark)
}

/// A map as a result.
fn give_map(pairs: &[(String, Item)], out: &mut String) -> R<'static> {
    let mark = out.len();
    compound::push_map(out, pairs.iter().map(|(k, v)| (k.as_str(), v)));
    made(out, mark)
}

/// A position of an array (0 first, −1 last), as an index; None outside it.
fn place(i: f64, len: usize) -> Option<usize> {
    if !i.is_finite() {
        return None;
    }
    let i = i.trunc();
    let at = if i < 0.0 { len as f64 + i } else { i };
    (at >= 0.0 && at < len as f64).then_some(at as usize)
}

/// A key items are grouped and told apart by (`dizi_benzersiz`,
/// `say_benzersiz`, a layer's groups, `katmandan`): numbers (and text that
/// reads as one) by value, true/false, text as it is; empty is no key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Num(u64),
    Bool(bool),
    Text(String),
}

impl Key {
    pub fn of(v: V) -> Option<Key> {
        if scalar::is_empty(v) {
            return None;
        }
        Some(match v {
            V::Bool(b) => Key::Bool(b),
            V::Text(s) if compound::is_compound(s) => Key::Text(s.to_owned()),
            v => match to_number(v) {
                // −0 is 0.
                Some(x) => Key::Num((x + 0.0).to_bits()),
                None => match v {
                    V::Text(s) => Key::Text(s.to_owned()),
                    _ => return None,
                },
            },
        })
    }
}

/// The number an item is for the sums (`kentos.statistics/1`): a number,
/// or text the rule reads (true/false and the rest are none).
pub fn number_of(it: &Item) -> Option<Dec> {
    match it {
        // Rust writes a double's shortest digits without an exponent.
        Item::Num(x) if x.is_finite() => statistics::read_number(&format!("{x}")),
        Item::Text(t) => statistics::read_number(t),
        _ => None,
    }
}

/// The exact sum of decimals, None when it does not fit (said, never rounded).
pub fn exact_sum(nums: &[Dec]) -> Option<Dec> {
    let top = nums.iter().map(|d| d.scale).max()?;
    let mut sum = Dec { m: 0, scale: top };
    for d in nums {
        let k = 10i128.checked_pow(top - d.scale)?;
        sum.m = sum.m.checked_add(d.m.checked_mul(k)?)?;
    }
    Some(sum)
}

/// The least or the greatest of items: of the numbers when there are any,
/// else of the texts in Turkish order.
pub fn extreme(items: &[Item], greatest: bool) -> Item {
    let nums: Vec<Dec> = items.iter().filter_map(number_of).collect();
    if !nums.is_empty() {
        let pick = if greatest {
            nums.iter()
                .copied()
                .max_by(|a, b| statistics::compare(*a, *b))
        } else {
            nums.iter()
                .copied()
                .min_by(|a, b| statistics::compare(*a, *b))
        };
        return pick.map_or(Item::Null, |d| Item::Num(d.to_f64()));
    }
    let texts = items.iter().filter_map(|it| match it {
        Item::Text(t) if !t.is_empty() => Some(t.as_str()),
        _ => None,
    });
    let pick = if greatest {
        texts.max_by(|a, b| compare_tr(a, b))
    } else {
        texts.min_by(|a, b| compare_tr(a, b))
    };
    pick.map_or(Item::Null, |t| Item::Text(t.to_owned()))
}

/// The sum or the mean of the items that are numbers; None when none is.
pub fn sum_or_mean(items: &[Item], mean: bool) -> Option<f64> {
    let nums: Vec<Dec> = items.iter().filter_map(number_of).collect();
    let sum = exact_sum(&nums)?.to_f64();
    Some(if mean { sum / nums.len() as f64 } else { sum })
}

/// Where an item sorts (`dizi_sırala`): numbers (and text that reads as
/// one) first by value, then text in Turkish order, then false and true,
/// then arrays and maps by their JSON; None for an empty item (last). A
/// total order, so sorting never meets a contradiction.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Rank<'i> {
    Num(Total),
    Text(Turkish<'i>),
    Bool(bool),
    Nested(&'i str),
}

/// A number ordered totally.
struct Total(f64);

impl PartialEq for Total {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}

impl Eq for Total {}

impl PartialOrd for Total {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Total {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Text in Turkish order.
#[derive(PartialEq, Eq)]
struct Turkish<'i>(&'i str);

impl PartialOrd for Turkish<'_> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Turkish<'_> {
    fn cmp(&self, o: &Self) -> Ordering {
        compare_tr(self.0, o.0).then_with(|| self.0.cmp(o.0))
    }
}

fn sort_rank(it: &Item) -> Option<Rank<'_>> {
    Some(match it {
        Item::Null => return None,
        Item::Text(t) if t.is_empty() => return None,
        Item::Num(x) => Rank::Num(Total(*x + 0.0)),
        Item::Text(t) => match to_number(V::Text(t)) {
            Some(x) => Rank::Num(Total(x + 0.0)),
            None => Rank::Text(Turkish(t)),
        },
        Item::Bool(b) => Rank::Bool(*b),
        Item::Nested(s) => Rank::Nested(s),
    })
}

/// An item's text as `dizi_metin` writes it: the language's text of a value.
fn push_item_text(out: &mut String, it: &Item) {
    match it {
        Item::Null => {}
        Item::Num(x) => push_number_text(out, *x),
        Item::Bool(b) => out.push_str(if *b { "doğru" } else { "yanlış" }),
        Item::Text(s) => out.push_str(s),
        Item::Nested(s) => out.push_str(compound::shown(s)),
    }
}

/// An array or map function (`f` one of them) on its arguments.
pub fn call(f: Func, args: &[V], out: &mut String, s: &mut Scratch) -> R<'static> {
    let arg = |i: usize| args.get(i).copied().unwrap_or(V::Null);
    let array = || compound::array_items(arg(0));
    let map = || compound::map_items(arg(0));
    match f {
        Func::Array => {
            let items: Vec<Item> = args.iter().map(|&a| Item::of(a)).collect();
            give_array(&items, out)
        }
        Func::ArrayLength => match array() {
            Some(items) => R::V(V::Num(items.len() as f64)),
            None => R::V(V::Null),
        },
        Func::ArrayGet => {
            let (Some(items), Some(i)) = (array(), to_number(arg(1))) else {
                return R::V(V::Null);
            };
            match place(i, items.len()) {
                Some(at) => give(items.into_iter().nth(at).unwrap_or(Item::Null), out),
                None => R::V(V::Null),
            }
        }
        Func::ArrayFirst | Func::ArrayLast => {
            let Some(mut items) = array() else {
                return R::V(V::Null);
            };
            let it = if f == Func::ArrayFirst {
                (!items.is_empty()).then(|| items.swap_remove(0))
            } else {
                items.pop()
            };
            give(it.unwrap_or(Item::Null), out)
        }
        Func::ArrayContains | Func::ArrayFind => {
            let Some(items) = array() else {
                return R::V(V::Null);
            };
            let x = arg(1);
            let at = items.iter().position(|it| compound::item_equals(it, x, s));
            R::V(if f == Func::ArrayContains {
                V::Bool(at.is_some())
            } else {
                V::Num(at.map_or(-1.0, |i| i as f64))
            })
        }
        Func::ArrayAppend => {
            let Some(mut items) = array() else {
                return R::V(V::Null);
            };
            items.push(Item::of(arg(1)));
            give_array(&items, out)
        }
        Func::ArrayCat => {
            let (Some(mut items), Some(more)) = (array(), compound::array_items(arg(1))) else {
                return R::V(V::Null);
            };
            items.extend(more);
            give_array(&items, out)
        }
        Func::ArrayDistinct => {
            let Some(items) = array() else {
                return R::V(V::Null);
            };
            let mut seen: HashSet<Option<Key>> = HashSet::new();
            let kept: Vec<Item> = items
                .into_iter()
                .filter(|it| seen.insert(Key::of(it.view())))
                .collect();
            give_array(&kept, out)
        }
        Func::ArraySort => {
            let Some(mut items) = array() else {
                return R::V(V::Null);
            };
            let ascending = args.len() < 2 || scalar::truthy(arg(1));
            // Empty last either way; ties stay where they were.
            items.sort_by(|p, q| match (sort_rank(p), sort_rank(q)) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => {
                    let o = a.cmp(&b);
                    if ascending { o } else { o.reverse() }
                }
            });
            give_array(&items, out)
        }
        Func::ArrayReverse => {
            let Some(mut items) = array() else {
                return R::V(V::Null);
            };
            items.reverse();
            give_array(&items, out)
        }
        Func::ArraySlice => {
            let (Some(items), Some(from), Some(to)) =
                (array(), to_number(arg(1)), to_number(arg(2)))
            else {
                return R::V(V::Null);
            };
            let n = items.len() as f64;
            let at = |i: f64| {
                let i = i.trunc();
                if i < 0.0 { n + i } else { i }
            };
            let (a, b) = (js_max(at(from), 0.0), js_min(at(to), n - 1.0));
            let kept: &[Item] = if a <= b && a < n {
                &items[a as usize..=b as usize]
            } else {
                &[]
            };
            give_array(kept, out)
        }
        Func::ArrayToText => {
            let Some(items) = array() else {
                return R::V(V::Null);
            };
            let sep = if args.len() > 1 {
                as_text(arg(1), &mut s.a)
            } else {
                ","
            };
            let blank = as_text(arg(2), &mut s.b);
            let mark = out.len();
            for (k, it) in items.iter().enumerate() {
                if k > 0 {
                    out.push_str(sep);
                }
                match it {
                    Item::Null => out.push_str(blank),
                    it => push_item_text(out, it),
                }
            }
            made(out, mark)
        }
        Func::TextToArray => {
            if arg(0) == V::Null {
                return R::V(V::Null);
            }
            let t = as_text(arg(0), &mut s.a);
            let sep = if args.len() > 1 {
                as_text(arg(1), &mut s.b)
            } else {
                ","
            };
            let blank = (args.len() > 2 && arg(2) != V::Null).then(|| as_text(arg(2), &mut s.c));
            let item = |part: &str| match blank {
                Some(b) if part == b => Item::Null,
                _ => Item::Text(part.to_owned()),
            };
            let items: Vec<Item> = if sep.is_empty() {
                t.chars()
                    .map(|c| item(c.encode_utf8(&mut [0; 4])))
                    .collect()
            } else {
                t.split(sep).map(item).collect()
            };
            give_array(&items, out)
        }
        Func::ArraySum | Func::ArrayMean => match array() {
            Some(items) => R::V(sum_or_mean(&items, f == Func::ArrayMean).map_or(V::Null, V::Num)),
            None => R::V(V::Null),
        },
        Func::ArrayMin | Func::ArrayMax => match array() {
            Some(items) => give(extreme(&items, f == Func::ArrayMax), out),
            None => R::V(V::Null),
        },
        Func::Map => {
            if !args.len().is_multiple_of(2) {
                return R::V(V::Null);
            }
            let mut pairs: Vec<(String, Item)> = Vec::with_capacity(args.len() / 2);
            for kv in args.chunks_exact(2) {
                let key = as_text(kv[0], &mut s.a).to_owned();
                let it = Item::of(kv[1]);
                match pairs.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) => slot.1 = it,
                    None => pairs.push((key, it)),
                }
            }
            give_map(&pairs, out)
        }
        Func::MapGet | Func::MapHas => {
            let Some(pairs) = map() else {
                return R::V(V::Null);
            };
            let key = as_text(arg(1), &mut s.a);
            let found = pairs.into_iter().find(|(k, _)| k == key);
            if f == Func::MapHas {
                return R::V(V::Bool(found.is_some()));
            }
            give(found.map_or(Item::Null, |(_, it)| it), out)
        }
        Func::MapKeys | Func::MapValues => {
            let Some(pairs) = map() else {
                return R::V(V::Null);
            };
            let items: Vec<Item> = pairs
                .into_iter()
                .map(|(k, it)| {
                    if f == Func::MapKeys {
                        Item::Text(k)
                    } else {
                        it
                    }
                })
                .collect();
            give_array(&items, out)
        }
        Func::MapInsert => {
            let Some(mut pairs) = map() else {
                return R::V(V::Null);
            };
            let key = as_text(arg(1), &mut s.a).to_owned();
            let it = Item::of(arg(2));
            match pairs.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = it,
                None => pairs.push((key, it)),
            }
            give_map(&pairs, out)
        }
        Func::MapDelete => {
            let Some(mut pairs) = map() else {
                return R::V(V::Null);
            };
            let key = as_text(arg(1), &mut s.a);
            pairs.retain(|(k, _)| k != key);
            give_map(&pairs, out)
        }
        Func::FromJson => {
            if arg(0) == V::Null {
                return R::V(V::Null);
            }
            match compound::from_json(as_text(arg(0), &mut s.a)) {
                Some(it) => give(it, out),
                None => R::V(V::Null),
            }
        }
        Func::ToJson => {
            let mark = out.len();
            compound::push_value_json(out, arg(0));
            made(out, mark)
        }
        _ => R::V(V::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A function's value, as the language shows it.
    fn c(f: Func, args: &[V]) -> String {
        let mut out = String::new();
        match call(f, args, &mut out, &mut Scratch::default()) {
            R::Made => compound::shown(&out).to_owned(),
            R::V(V::Null) => "boş".into(),
            R::V(V::Num(x)) => x.to_string(),
            R::V(V::Bool(b)) => b.to_string(),
            r => format!("{r:?}"),
        }
    }

    /// An array made by `dizi`, with its mark.
    fn a(items: &[V]) -> String {
        let mut out = String::new();
        call(Func::Array, items, &mut out, &mut Scratch::default());
        out
    }

    #[test]
    fn arrays_hold_take_and_order_their_items() {
        let (n, t) = (V::Num, V::Text);
        let d = a(&[n(3.0), t("Arsa"), n(1.0), V::Null, t("12")]);
        assert_eq!(compound::shown(&d), r#"[3,"Arsa",1,null,"12"]"#);
        assert_eq!(c(Func::ArrayLength, &[t(&d)]), "5");
        assert_eq!(c(Func::ArrayLength, &[t("[1]")]), "boş");
        assert_eq!(c(Func::ArrayGet, &[t(&d), n(1.0)]), "Arsa");
        assert_eq!(c(Func::ArrayGet, &[t(&d), n(-1.0)]), "12");
        assert_eq!(c(Func::ArrayGet, &[t(&d), n(5.0)]), "boş");
        assert_eq!(c(Func::ArrayContains, &[t(&d), n(12.0)]), "true");
        assert_eq!(c(Func::ArrayFind, &[t(&d), t("1")]), "2");
        assert_eq!(c(Func::ArrayFind, &[t(&d), t("x")]), "-1");
        assert_eq!(c(Func::ArraySort, &[t(&d)]), r#"[1,3,"12","Arsa",null]"#);
        let trees = a(&[t("Çam"), t("Ceviz"), t("Dut")]);
        assert_eq!(
            c(Func::ArraySort, &[t(&trees), V::Bool(false)]),
            r#"["Dut","Çam","Ceviz"]"#
        );
        assert_eq!(
            c(Func::ArraySlice, &[t(&d), n(1.0), n(-2.0)]),
            r#"["Arsa",1,null]"#
        );
        assert_eq!(c(Func::ArraySlice, &[t(&d), n(3.0), n(1.0)]), "[]");
        let ones = a(&[n(1.0), t("1"), n(2.0), n(1.0)]);
        assert_eq!(c(Func::ArrayDistinct, &[t(&ones)]), "[1,2]");
        assert_eq!(
            c(Func::ArrayToText, &[t(&d), t("/"), t("?")]),
            "3/Arsa/1/?/12"
        );
        assert_eq!(c(Func::TextToArray, &[t("a,b,,c")]), r#"["a","b","","c"]"#);
        assert_eq!(
            c(Func::TextToArray, &[t("a-b"), t(""), t("-")]),
            r#"["a",null,"b"]"#
        );
        let tenths = a(&[n(0.1), n(0.2), t("x"), t("0,5")]);
        assert_eq!(c(Func::ArraySum, &[t(&tenths)]), "0.8");
        let three = a(&[n(1.0), n(2.0), n(4.0)]);
        assert_eq!(c(Func::ArrayMean, &[t(&three)]), (7.0f64 / 3.0).to_string());
        assert_eq!(c(Func::ArrayMax, &[t(&d)]), "12");
        let pair = a(&[t("Çam"), t("Ceviz")]);
        assert_eq!(c(Func::ArrayMin, &[t(&pair)]), "Ceviz");
        let none = a(&[t("x")]);
        assert_eq!(c(Func::ArraySum, &[t(&none)]), "boş");
    }

    #[test]
    fn maps_and_json_read_and_write() {
        let (n, t) = (V::Num, V::Text);
        let mut m = String::new();
        call(
            Func::Map,
            &[t("ad"), t("Arsa"), t("kat"), n(3.0), t("ad"), t("Bahçe")],
            &mut m,
            &mut Scratch::default(),
        );
        assert_eq!(compound::shown(&m), r#"{"ad":"Bahçe","kat":3}"#);
        assert_eq!(c(Func::MapGet, &[t(&m), t("kat")]), "3");
        assert_eq!(c(Func::MapHas, &[t(&m), t("yok")]), "false");
        assert_eq!(c(Func::MapKeys, &[t(&m)]), r#"["ad","kat"]"#);
        assert_eq!(
            c(Func::MapInsert, &[t(&m), t("x"), V::Bool(true)]),
            r#"{"ad":"Bahçe","kat":3,"x":true}"#
        );
        assert_eq!(c(Func::MapDelete, &[t(&m), t("ad")]), r#"{"kat":3}"#);
        assert_eq!(c(Func::Map, &[t("a")]), "boş");
        assert_eq!(
            c(Func::FromJson, &[t(" {\"a\": [1, 2]} ")]),
            r#"{"a":[1,2]}"#
        );
        assert_eq!(c(Func::FromJson, &[t("12.5")]), "12.5");
        assert_eq!(c(Func::FromJson, &[t("{")]), "boş");
        assert_eq!(c(Func::ToJson, &[t("A\"\n")]), r#""A\"\n""#);
        assert_eq!(c(Func::ToJson, &[V::Null]), "null");
        assert_eq!(c(Func::ToJson, &[t(&m)]), r#"{"ad":"Bahçe","kat":3}"#);
    }
}
