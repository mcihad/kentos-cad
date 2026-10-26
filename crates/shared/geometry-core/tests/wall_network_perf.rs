//! Separate-process release timings; /usr/bin/time -l measures process peak RSS.
//! The known rectangular chain has 3n-3 runs and unique length 11n+3 m (n>=2).
#![allow(clippy::disallowed_methods)]
use kentos_geometry_core::{
    Vec2,
    geom::{
        arrangement::{Ring, edge_len},
        bulge::bulge_path_edges,
        wall_network,
    },
};
use std::{hint::black_box, time::Instant};
fn rooms(n: usize, separated: bool) -> Vec<Ring> {
    (0..n)
        .map(|i| {
            let x = 500000. + i as f64 * if separated { 10. } else { 4. };
            let y = 4400000.;
            Ring {
                pts: vec![
                    Vec2::new(x, y),
                    Vec2::new(x + 4., y),
                    Vec2::new(x + 4., y + 3.),
                    Vec2::new(x, y + 3.),
                ],
                bulges: None,
            }
        })
        .collect()
}
#[test]
#[ignore = "separate process release measurement; WALL_NETWORK_N and WALL_NETWORK_SEPARATED"]
fn measured_rectangular_network() {
    let n = std::env::var("WALL_NETWORK_N")
        .unwrap_or_else(|_| "100".into())
        .parse::<usize>()
        .unwrap();
    assert!(n >= 2);
    let separated = std::env::var("WALL_NETWORK_SEPARATED").as_deref() == Ok("1");
    let input = rooms(n, separated);
    let mut times = Vec::new();
    let repeats = std::env::var("WALL_NETWORK_REPEATS")
        .unwrap_or_else(|_| "5".into())
        .parse::<usize>()
        .unwrap();
    for _ in 0..repeats {
        let start = Instant::now();
        let answer = black_box(wall_network::build(black_box(&input)));
        times.push(start.elapsed().as_secs_f64() * 1000.);
        match answer {
            Err(problem) => {
                println!(
                    "NETWORK_RESULT {}",
                    serde_json::json!({"rooms":n,"separated":separated,"error":format!("{problem:?}"),"milliseconds":times})
                );
                panic!("valid rectangular input was refused: {problem:?}");
            }
            Ok(None) => assert!(separated),
            Ok(Some(network)) => {
                assert!(!separated);
                assert_eq!(network.runs.len(), 3 * n - 3);
                assert_eq!(network.shared_pieces, n - 1);
                assert!((network.shared_length - 3. * (n - 1) as f64).abs() < 1e-8);
                assert!(network.affected.iter().all(|a| *a));
                let length = network
                    .runs
                    .iter()
                    .flat_map(|r| bulge_path_edges(&r.pts, r.bulges.as_deref(), r.closed))
                    .map(|e| edge_len(&e))
                    .sum::<f64>();
                assert!((length - (11. * n as f64 + 3.)).abs() < 1e-7);
            }
        }
    }
    println!(
        "NETWORK_RESULT {}",
        serde_json::json!({"rooms":n,"separated":separated,"status":"verified","milliseconds":times})
    );
}
#[test]
fn a_valid_large_chain_does_not_allocate_source_by_edge_ownership() {
    let n = 1600;
    let network = wall_network::build(&rooms(n, false))
        .expect("valid connected rooms")
        .unwrap();
    assert_eq!(network.runs.len(), 3 * n - 3);
    assert_eq!(network.shared_pieces, n - 1);
    assert_eq!(network.shared_length, 3. * (n - 1) as f64);
}
#[test]
fn ten_thousand_disjoint_rooms_keep_original_closed_paths_without_dense_ownership() {
    assert!(
        wall_network::build(&rooms(10000, true))
            .expect("independent rooms")
            .is_none()
    );
}

#[test]
fn many_duplicate_rooms_are_refused_before_constructing_a_dense_overlap_graph() {
    let room = rooms(2, false).remove(0);
    assert_eq!(
        wall_network::build(&vec![room; 2000]).err(),
        Some(wall_network::Problem::OverlappingRooms)
    );
}
