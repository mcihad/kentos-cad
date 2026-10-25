//! Reproducible release comparison: rebuilt seed queries vs one immutable index.
//! cargo run -q --release -p kentos-geometry-core --example space_queries
//! Measures geometry only; not UI latency, GPU time, or end-to-end command time.
use kentos_geometry_core::geom::{
    space, space_index::BoundaryIndex, wall::Spec, wall_join::WallIn,
};
use kentos_geometry_core::vec2::Vec2;
use std::{hint::black_box, time::Instant};

fn percentile(values: &mut [f64], p: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() - 1) as f64 * p).ceil() as usize]
}
fn main() {
    let mut rows = Vec::new();
    for n in [4, 12, 20] {
        let extent = n as f64 * 4.;
        let (x, y) = (500000., 4400000.);
        let mut walls = vec![(
            vec![
                Vec2::new(x, y),
                Vec2::new(x + extent, y),
                Vec2::new(x + extent, y + extent),
                Vec2::new(x, y + extent),
            ],
            Spec {
                width: 0.2,
                closed: true,
                ..Default::default()
            },
        )];
        for i in 1..n {
            let p = i as f64 * 4.;
            walls.push((
                vec![Vec2::new(x + p, y), Vec2::new(x + p, y + extent)],
                Spec {
                    width: 0.2,
                    ..Default::default()
                },
            ));
            walls.push((
                vec![Vec2::new(x, y + p), Vec2::new(x + extent, y + p)],
                Spec {
                    width: 0.2,
                    ..Default::default()
                },
            ));
        }
        let ins: Vec<_> = walls
            .iter()
            .map(|(pts, spec)| WallIn {
                pts,
                bulges: None,
                spec,
            })
            .collect();
        let seeds: Vec<_> = (0..n)
            .flat_map(|r| {
                (0..n).map(move |c| Vec2::new(x + c as f64 * 4. + 2., y + r as f64 * 4. + 2.))
            })
            .collect();
        let legacy = || {
            seeds
                .iter()
                .map(|&s| space::found(&ins, s, space::Method::Net).unwrap())
                .collect::<Vec<_>>()
        };
        let indexed = || {
            let index = BoundaryIndex::for_queries(&ins);
            seeds
                .iter()
                .map(|&s| index.found(s, space::Method::Net).unwrap())
                .collect::<Vec<_>>()
        };
        let expected = legacy();
        assert_eq!(indexed(), expected);
        assert!((space::area(&expected) - n as f64 * n as f64 * 3.8 * 3.8).abs() < 1e-5);
        let mut old = Vec::new();
        let mut new = Vec::new();
        for i in 0..9 {
            // Alternate order to reduce fixed warm-up/order bias. Construction
            // of every index is inside the measured call in both paths.
            for fast in if i % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let start = Instant::now();
                let answer = if fast { indexed() } else { legacy() };
                black_box(&answer);
                let elapsed = start.elapsed().as_secs_f64() * 1000.;
                assert_eq!(answer, expected);
                if fast {
                    new.push(elapsed)
                } else {
                    old.push(elapsed)
                }
            }
        }
        rows.push(serde_json::json!({"rooms":seeds.len(),"walls":walls.len(),"origin":[x,y],"samples":9,
            "legacyMs":old,"indexedMs":new,"legacyMedianMs":percentile(&mut old,0.5),"legacyP95Ms":percentile(&mut old,0.95),
            "indexedMedianMs":percentile(&mut new,0.5),"indexedP95Ms":percentile(&mut new,0.95),"allBoundariesExactEqual":true}));
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
