//! Toplama (docs/adr/0214 §2.6): an aggregate over the objects of a layer,
//! by group. Numbers follow `kentos.statistics/1` (ADR 0200 §4): the sum is
//! exact, the mean is the exact sum over the count, the median of an even
//! count the exact mean of the two middle ones, the standard deviation the
//! sample's (n − 1) in double precision in the layer's order; the least and
//! the greatest are of the numbers, else of the texts in Turkish order.

use std::collections::{HashMap, HashSet};

use kentos_geometry_core::ops::statistics::{self, Dec};

use super::{Table, WorldCall};
use crate::arrays::{self, Key, extreme, number_of, sum_or_mean};
use crate::compound::{self, Item};
use crate::library::{Func, find_function};
use crate::read::push_number_text;
use crate::scalar::is_empty;

/// An aggregate's operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregate {
    Sum,
    Mean,
    Count,
    CountDistinct,
    Min,
    Max,
    Median,
    StdDev,
    Concat,
    Array,
}

impl Aggregate {
    /// The names `katman_toplamı` takes, for its message.
    pub const NAMES: &'static str = "topla, ortalama, say, say_benzersiz, en_düşük, en_yüksek, orta_değer, std_sapma, değerleri_birleştir ya da değerleri_dizi";

    /// The operation of an aggregate function.
    pub fn of(f: Func) -> Option<Aggregate> {
        Some(match f {
            Func::Sum => Aggregate::Sum,
            Func::Mean => Aggregate::Mean,
            Func::Count => Aggregate::Count,
            Func::CountDistinct => Aggregate::CountDistinct,
            Func::Minimum => Aggregate::Min,
            Func::Maximum => Aggregate::Max,
            Func::Median => Aggregate::Median,
            Func::StdDev => Aggregate::StdDev,
            Func::ConcatValues => Aggregate::Concat,
            Func::ArrayAgg => Aggregate::Array,
            _ => return None,
        })
    }

    /// The operation `katman_toplamı` names (an aggregate function's name or alias).
    pub fn from_name(name: &str) -> Option<Aggregate> {
        find_function(crate::js::text::trim(name)).and_then(|d| Aggregate::of(d.func))
    }
}

/// The separator `değerleri_birleştir` writes when none is given.
pub const SEPARATOR: &str = ", ";

/// The median of decimals: the middle one, or the exact mean of the two.
fn median(mut nums: Vec<Dec>) -> Option<f64> {
    if nums.is_empty() {
        return None;
    }
    nums.sort_by(|a, b| statistics::compare(*a, *b));
    let n = nums.len();
    if n % 2 == 1 {
        return Some(nums[n / 2].to_f64());
    }
    let (a, b) = (nums[n / 2 - 1], nums[n / 2]);
    Some(match arrays::exact_sum(&[a, b]) {
        Some(sum) => sum
            .m
            .checked_mul(5)
            .map(|m| Dec {
                m,
                scale: sum.scale + 1,
            })
            .map_or((a.to_f64() + b.to_f64()) / 2.0, Dec::to_f64),
        None => (a.to_f64() + b.to_f64()) / 2.0,
    })
}

/// The sample standard deviation of the numbers, two passes in their order.
fn std_dev(nums: &[Dec]) -> Option<f64> {
    if nums.len() < 2 {
        return None;
    }
    let xs: Vec<f64> = nums.iter().map(|d| d.to_f64()).collect();
    let n = xs.len() as f64;
    let mean = xs.iter().fold(0.0, |s, x| s + x) / n;
    let ss = xs.iter().fold(0.0, |s, x| s + (x - mean) * (x - mean));
    Some((ss / (n - 1.0)).sqrt())
}

/// An item's text as the joined values write it.
fn push_text(out: &mut String, it: &Item) {
    match it {
        Item::Null => {}
        Item::Num(x) => push_number_text(out, *x),
        Item::Bool(b) => out.push_str(if *b { "doğru" } else { "yanlış" }),
        Item::Text(s) => out.push_str(s),
        Item::Nested(s) => out.push_str(compound::shown(s)),
    }
}

/// One aggregate over items in the layer's order.
pub fn over(op: Aggregate, items: &[&Item], separator: &str) -> Item {
    let given = || items.iter().filter(|it| !is_empty(it.view()));
    let nums = || -> Vec<Dec> { items.iter().filter_map(|it| number_of(it)).collect() };
    let num = |x: Option<f64>| x.map_or(Item::Null, Item::Num);
    match op {
        Aggregate::Sum | Aggregate::Mean => {
            let owned: Vec<Item> = items.iter().map(|it| (*it).clone()).collect();
            num(sum_or_mean(&owned, op == Aggregate::Mean))
        }
        Aggregate::Count => Item::Num(given().count() as f64),
        Aggregate::CountDistinct => {
            let keys: HashSet<Key> = given().filter_map(|it| Key::of(it.view())).collect();
            Item::Num(keys.len() as f64)
        }
        Aggregate::Min | Aggregate::Max => {
            let owned: Vec<Item> = items.iter().map(|it| (*it).clone()).collect();
            extreme(&owned, op == Aggregate::Max)
        }
        Aggregate::Median => num(median(nums())),
        Aggregate::StdDev => num(std_dev(&nums())),
        Aggregate::Concat => {
            let mut out = String::new();
            for (k, it) in given().enumerate() {
                if k > 0 {
                    out.push_str(separator);
                }
                push_text(&mut out, it);
            }
            if given().next().is_none() {
                Item::Null
            } else {
                Item::Text(out)
            }
        }
        Aggregate::Array => {
            let mut out = String::new();
            compound::push_array(&mut out, items.iter().copied());
            Item::Nested(out)
        }
    }
}

/// `değerleri_birleştir`'s separator: its own, else ", " (`katman_toplamı`'s text is its operation).
pub(super) fn separator(c: &WorldCall) -> &str {
    match (c.func, c.text.as_deref()) {
        (Func::ConcatValues, Some(t)) => t,
        _ => SEPARATOR,
    }
}

/// An aggregate's result per group over a call's table: every object that
/// meets the condition, grouped by key when `groups` is given.
pub(super) fn results(
    op: Aggregate,
    t: &Table,
    groups: Option<&[Option<Key>]>,
    c: &WorldCall,
) -> HashMap<Option<Key>, Item> {
    let separator = separator(c);
    let kept = |k: usize| t.mask.as_ref().is_none_or(|m| m[k]);
    let mut buckets: HashMap<Option<Key>, Vec<&Item>> = HashMap::new();
    match groups {
        Some(keys) => {
            for (k, it) in t.values.iter().enumerate() {
                if kept(k) {
                    buckets
                        .entry(keys.get(k).cloned().flatten())
                        .or_default()
                        .push(it);
                }
            }
        }
        None => {
            let all: Vec<&Item> = t
                .values
                .iter()
                .enumerate()
                .filter(|&(k, _)| kept(k))
                .map(|(_, it)| it)
                .collect();
            buckets.insert(None, all);
        }
    }
    buckets
        .into_iter()
        .map(|(key, list)| (key, over(op, &list, separator)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_keep_the_statistics_rule() {
        let items = [
            Item::Num(0.1),
            Item::Num(0.2),
            Item::Text("x".into()),
            Item::Null,
            Item::Text("0,5".into()),
            Item::Num(0.2),
        ];
        let refs: Vec<&Item> = items.iter().collect();
        let n = |op| over(op, &refs, ", ");
        assert_eq!(n(Aggregate::Sum), Item::Num(1.0));
        assert_eq!(n(Aggregate::Count), Item::Num(5.0));
        assert_eq!(n(Aggregate::CountDistinct), Item::Num(4.0));
        assert_eq!(n(Aggregate::Mean), Item::Num(0.25));
        assert_eq!(n(Aggregate::Median), Item::Num(0.2));
        assert_eq!(n(Aggregate::Max), Item::Num(0.5));
        assert_eq!(
            n(Aggregate::Concat),
            Item::Text("0.1, 0.2, x, 0,5, 0.2".into())
        );
        assert_eq!(
            n(Aggregate::Array),
            Item::Nested(format!(
                "{}[0.1,0.2,\"x\",null,\"0,5\",0.2]",
                compound::ARRAY
            ))
        );
        let even = [Item::Num(1.0), Item::Num(2.0)];
        let refs: Vec<&Item> = even.iter().collect();
        assert_eq!(over(Aggregate::Median, &refs, ""), Item::Num(1.5));
        assert_eq!(over(Aggregate::StdDev, &refs, ""), Item::Num(0.5f64.sqrt()));
        assert_eq!(over(Aggregate::Sum, &[], ""), Item::Null);
        assert_eq!(over(Aggregate::Count, &[], ""), Item::Num(0.0));
        assert_eq!(Aggregate::from_name("SUM"), Some(Aggregate::Sum));
        assert_eq!(
            Aggregate::from_name("değerleri_dizi"),
            Some(Aggregate::Array)
        );
        assert_eq!(Aggregate::from_name("yuvarla"), None);
    }
}
