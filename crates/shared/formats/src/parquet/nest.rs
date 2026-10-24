//! A leaf column's repetition and definition levels as records of nested lists (Dremel's assembly; docs/adr/0628 §1):
//! for each repeated node on its path, the lists it makes (Arrow's offsets) and whether each is there, and for each
//! element of the deepest one (or each record of a column that repeats nowhere) the index of its value.

/// The lists a column's levels make.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Nested {
    /// `offsets[k]`: for each list of the `k`-th repeated node, its first element; one more at the end. The lists of
    /// the first are one per record; the lists of the next are one per element of the one before.
    pub offsets: Vec<Vec<usize>>,
    /// `present[k]`: whether each list of the `k`-th repeated node is there (false: null).
    pub present: Vec<Vec<bool>>,
    /// For each element of the deepest list (each record when nothing repeats): its value's index; none for null.
    pub values: Vec<Option<usize>>,
    /// Each of those elements' definition level (a struct holding the value is null below its own level).
    pub defs: Vec<u16>,
    pub records: usize,
}

/// Assembles `def` and `rep` (empty when the column's levels are all zero) of a column whose definition level is
/// `max_def` and whose repeated nodes have definition levels `repeated`, root first.
pub fn nest(
    def: &[u16],
    rep: &[u16],
    slots: usize,
    max_def: u16,
    repeated: &[u16],
) -> Result<Nested, String> {
    let levels = repeated.len();
    let mut out = Nested {
        offsets: vec![Vec::new(); levels],
        present: vec![Vec::new(); levels],
        values: Vec::new(),
        defs: Vec::new(),
        records: 0,
    };
    let mut count = vec![0usize; levels];
    let mut value = 0usize;
    for i in 0..slots {
        let d = if def.is_empty() { max_def } else { def[i] };
        let r = if rep.is_empty() {
            0
        } else {
            usize::from(rep[i])
        };
        if d > max_def || r > levels {
            return Err("Parquet düzeyi sütunun en büyük düzeyini aşıyor".into());
        }
        if r == 0 {
            out.records += 1;
        } else if i == 0 {
            return Err("Parquet sütunu bir kaydın ortasında başlıyor".into());
        }
        if levels == 0 {
            out.defs.push(d);
            out.values.push(if d == max_def {
                value += 1;
                Some(value - 1)
            } else {
                None
            });
            continue;
        }
        for k in 1..=levels {
            if k < r {
                continue;
            }
            if k > r {
                // A new list at this level, when the element holding it is there.
                if k > 1 && d < repeated[k - 2] {
                    break;
                }
                out.offsets[k - 1].push(count[k - 1]);
                out.present[k - 1].push(d + 1 >= repeated[k - 1]);
            }
            if d < repeated[k - 1] {
                break;
            }
            count[k - 1] += 1;
            if k == levels {
                out.defs.push(d);
                out.values.push(if d == max_def {
                    value += 1;
                    Some(value - 1)
                } else {
                    None
                });
            }
        }
    }
    for (offsets, n) in out.offsets.iter_mut().zip(&count) {
        offsets.push(*n);
    }
    Ok(out)
}

impl Nested {
    /// The elements of list `i` of level `k`.
    pub fn range(&self, k: usize, i: usize) -> std::ops::Range<usize> {
        self.offsets[k][i]..self.offsets[k][i + 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_column_with_nulls_and_empty_lists() {
        // optional group a (LIST) { repeated group list { optional int32 element } }: max_def 3, the list's 2.
        // Records: [1, null, 2], null, [], [3].
        let def = [3, 2, 3, 0, 1, 3];
        let rep = [0, 1, 1, 0, 0, 0];
        let n = nest(&def, &rep, 6, 3, &[2]).unwrap();
        assert_eq!(n.records, 4);
        assert_eq!(n.offsets[0], [0, 3, 3, 3, 4]);
        assert_eq!(n.present[0], [true, false, true, true]);
        assert_eq!(n.values, [Some(0), None, Some(1), Some(2)]);
    }

    #[test]
    fn polygons_of_rings_of_points() {
        // A polygon column: list (rings) of list (points), points required: def 2 at a point, rep 0/1/2.
        // Records: two rings of 2 and 1 points; then an empty polygon.
        let def = [2, 2, 2, 0];
        let rep = [0, 2, 1, 0];
        let n = nest(&def, &rep, 4, 2, &[1, 2]).unwrap();
        assert_eq!(n.records, 2);
        assert_eq!(n.offsets[0], [0, 2, 2]);
        assert_eq!(n.offsets[1], [0, 2, 3]);
        assert_eq!(n.values, [Some(0), Some(1), Some(2)]);
    }
}
