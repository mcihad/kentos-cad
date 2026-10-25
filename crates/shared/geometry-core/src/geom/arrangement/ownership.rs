//! Ownership storage only; noding, circles, vertices and classification stay in arrangement.

pub(super) trait Counts {
    fn zero(sources: usize) -> Self;
    /// True when a new sparse slot was allocated (including a subsequently cancelled count).
    fn add(&mut self, source: usize, amount: f64) -> bool;
}
impl Counts for Vec<f64> {
    fn zero(sources: usize) -> Self {
        vec![0.; sources]
    }
    fn add(&mut self, source: usize, amount: f64) -> bool {
        self[source] += amount;
        false
    }
}

/// Source-ordered counts, retaining zero slots so cancellation has exactly the dense result.
#[derive(Debug, Default)]
pub struct SparseCounts(Vec<(usize, f64)>);
impl SparseCounts {
    pub fn nonzero(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.0.iter().copied().filter(|(_, n)| *n != 0.)
    }
}
impl Counts for SparseCounts {
    fn zero(_: usize) -> Self {
        Self::default()
    }
    fn add(&mut self, source: usize, amount: f64) -> bool {
        match self.0.binary_search_by_key(&source, |(s, _)| *s) {
            Ok(i) => {
                self.0[i].1 += amount;
                false
            }
            Err(i) => {
                self.0.insert(i, (source, amount));
                true
            }
        }
    }
}

/// Explicit allocation-count limits of the sparse topology consumer. Not a byte/RSS promise.
#[derive(Clone, Copy, Debug)]
pub struct SparseLimits {
    pub pieces: usize,
    pub ownership_slots: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget;

pub(super) trait Limits {
    type Error;
    fn check(&self, pieces: usize, ownership_slots: usize) -> Result<(), Self::Error>;
}

pub(super) struct Unlimited;
impl Limits for Unlimited {
    type Error = std::convert::Infallible;
    fn check(&self, _: usize, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl Limits for SparseLimits {
    type Error = Budget;
    fn check(&self, pieces: usize, ownership_slots: usize) -> Result<(), Self::Error> {
        if pieces > self.pieces || ownership_slots > self.ownership_slots {
            Err(Budget)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Source, build, build_sparse};
    use super::*;
    use crate::{
        Vec2,
        geom::{
            bulge::bulge_path_edges,
            overlay::{localize, origin_of},
        },
    };

    fn source(pts: Vec<Vec2>, bulges: Option<Vec<f64>>, closed: bool, cut: bool) -> Source {
        Source {
            edges: bulge_path_edges(&pts, bulges.as_deref(), closed),
            points: Some(pts),
            cut: cut.then_some(true),
        }
    }
    #[test]
    fn sparse_ownership_preserves_dense_noding_and_every_count_for_arcs_cuts_and_cancellation() {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for offset in [0., 0.5, 2., 3.9999995] {
                let pts = vec![
                    Vec2::new(x, y),
                    Vec2::new(x + 4., y),
                    Vec2::new(x + 4., y + 3.),
                    Vec2::new(x, y + 3.),
                ];
                let mut reversed = pts.clone();
                reversed.reverse();
                let circle = vec![
                    Vec2::new(x + offset + 3., y + 1.),
                    Vec2::new(x + offset - 3., y + 1.),
                ];
                let abs = vec![
                    source(pts, None, true, false),
                    source(reversed, None, true, false),
                    source(circle.clone(), Some(vec![1., 1.]), true, false),
                    source(circle, Some(vec![-1., -1.]), true, false),
                    source(
                        vec![Vec2::new(x - 5., y + 1.), Vec2::new(x + 8., y + 1.)],
                        None,
                        false,
                        true,
                    ),
                ];
                let origin = origin_of(&abs);
                let local = localize(&abs, origin);
                let dense = build(&local, &abs, origin);
                let sparse = build_sparse(
                    &local,
                    &abs,
                    origin,
                    SparseLimits {
                        pieces: 1000,
                        ownership_slots: 1000,
                    },
                )
                .unwrap();
                assert_eq!(dense.verts.pos, sparse.verts.pos);
                assert_eq!(dense.verts.input, sparse.verts.input);
                assert_eq!(dense.pieces.len(), sparse.pieces.len());
                for (a, b) in dense.pieces.iter().zip(&sparse.pieces) {
                    assert_eq!(a.edge, b.edge);
                    assert_eq!((a.from, a.to, a.cut), (b.from, b.to, b.cut));
                    let entries: Vec<_> = a
                        .delta
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(|(_, n)| *n != 0.)
                        .collect();
                    assert_eq!(entries, b.delta.nonzero().collect::<Vec<_>>());
                }
            }
        }
    }
    #[test]
    fn sparse_limits_return_no_partial_topology_and_leave_the_dense_consumer_unchanged() {
        let abs = vec![source(
            vec![
                Vec2::new(0., 0.),
                Vec2::new(4., 0.),
                Vec2::new(4., 3.),
                Vec2::new(0., 3.),
            ],
            None,
            true,
            false,
        )];
        for limits in [
            SparseLimits {
                pieces: 3,
                ownership_slots: 100,
            },
            SparseLimits {
                pieces: 100,
                ownership_slots: 3,
            },
        ] {
            assert!(matches!(
                build_sparse(&abs, &abs, Vec2::default(), limits),
                Err(Budget)
            ));
        }
        let dense = build(&abs, &abs, Vec2::default());
        assert_eq!(dense.pieces.len(), 4);
        assert!(dense.pieces.iter().all(|p| p.delta == vec![1.]));
        let sparse = build_sparse(
            &abs,
            &abs,
            Vec2::default(),
            SparseLimits {
                pieces: 4,
                ownership_slots: 4,
            },
        )
        .unwrap();
        assert_eq!(sparse.pieces.len(), 4);
    }
}
