//! Analytic independent room counts/sizes, including the wall bands that must
//! never be offered as rooms. These expectations are elementary geometry.
use kentos_geometry_core::geom::{
    region, space, space_index::BoundaryIndex, wall::Spec, wall_join::WallIn,
};
use kentos_geometry_core::vec2::Vec2;

type Wall = (Vec<Vec2>, Option<Vec<f64>>, Spec);
fn wall(pts: &[(f64, f64)], closed: bool) -> Wall {
    (
        pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        None,
        Spec {
            width: 0.2,
            closed,
            ..Default::default()
        },
    )
}
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Wall {
    wall(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], true)
}
fn inputs(walls: &[Wall]) -> Vec<WallIn<'_>> {
    walls
        .iter()
        .map(|(pts, b, spec)| WallIn {
            pts,
            bulges: b.as_deref(),
            spec,
        })
        .collect()
}
fn areas(index: &BoundaryIndex, method: space::Method) -> Vec<f64> {
    let mut result: Vec<_> = index
        .rooms(method)
        .unwrap()
        .iter()
        .map(|r| {
            assert!(
                region::inside_area(&r.net, r.seed),
                "analytic probe not inside net face"
            );
            let repeated = index.found(r.seed, method).unwrap();
            assert_eq!(repeated, r.boundary, "enumeration and seed lookup disagree");
            region::net_area(&r.boundary)
        })
        .collect();
    result.sort_by(f64::total_cmp);
    result
}
fn equal(got: &[f64], expected: &[f64]) {
    assert_eq!(
        got.len(),
        expected.len(),
        "got {got:?}, expected {expected:?}"
    );
    for (g, e) in got.iter().zip(expected) {
        assert!((g - e).abs() < 1e-6, "{g} != {e}");
    }
}

#[test]
fn strict_room_seed_rejects_wall_material_without_changing_legacy_seed_semantics() {
    for (x,y) in [(0.,0.),(500000.,4400000.)] {
        let walls=vec![rectangle(x,y,10.,8.),wall(&[(x+5.,y),(x+5.,y+8.)],false)];
        let index=BoundaryIndex::new(&inputs(&walls));
        let legacy=BoundaryIndex::for_queries(&inputs(&walls));
        for method in [space::Method::Net,space::Method::Centerline,space::Method::Gross] {
            let material=Vec2::new(x+5.,y+4.);
            assert!(index.found(material,method).is_some());
            assert_eq!(index.found(material,method),legacy.found(material,method));
            assert!(index.room_at(material,method).unwrap().is_none());
            let free=Vec2::new(x+2.,y+6.);
            assert_eq!(index.room_at(free,method).unwrap(),index.found(free,method));
            assert!(legacy.room_at(free,method).is_err());
        }
    }
}

#[test]
fn bounded_voids_exclude_closed_wall_material_and_keep_methods_and_legacy_queries() {
    for (x, y) in [(0., 0.), (500000., 4400000.)] {
        for reversed in [false, true] {
            let mut walls = vec![rectangle(x, y, 6., 4.), rectangle(x + 10., y, 8., 5.)];
            if reversed {
                for (pts, _, _) in &mut walls {
                    pts.reverse();
                }
            }
            let ins = inputs(&walls);
            let index = BoundaryIndex::new(&ins);
            equal(&areas(&index, space::Method::Net), &[5.8 * 3.8, 7.8 * 4.8]);
            equal(&areas(&index, space::Method::Centerline), &[24., 40.]);
            equal(
                &areas(&index, space::Method::Gross),
                &[6.2 * 4.2, 8.2 * 5.2],
            );
            for seed in [
                Vec2::new(x + 2., y + 2.),
                Vec2::new(x + 12., y + 2.),
                Vec2::new(x + 3., y),
                Vec2::new(x - 3., y),
            ] {
                for method in [
                    space::Method::Net,
                    space::Method::Centerline,
                    space::Method::Gross,
                ] {
                    assert_eq!(index.found(seed, method), space::found(&ins, seed, method));
                }
            }
        }
    }
}

#[test]
fn joined_partition_cross_and_open_corridor_have_measured_free_faces() {
    let mut walls = vec![
        rectangle(0., 0., 10., 8.),
        wall(&[(5., 0.), (5., 8.)], false),
    ];
    equal(
        &areas(&BoundaryIndex::new(&inputs(&walls)), space::Method::Net),
        &[4.8 * 7.8, 4.8 * 7.8],
    );
    walls.push(wall(&[(0., 4.), (10., 4.)], false));
    equal(
        &areas(&BoundaryIndex::new(&inputs(&walls)), space::Method::Net),
        &[4.8 * 3.8; 4],
    );
    let corridor = vec![wall(&[(0., 0.), (0., 8.), (10., 8.), (10., 0.)], false)];
    equal(
        &areas(&BoundaryIndex::new(&inputs(&corridor)), space::Method::Net),
        &[],
    );
}

#[test]
fn nested_closed_walls_make_a_courtyard_and_annulus_without_two_material_bands() {
    let walls = vec![rectangle(0., 0., 12., 10.), rectangle(4., 3., 4., 4.)];
    let index = BoundaryIndex::new(&inputs(&walls));
    let mut expected = vec![3.8 * 3.8, 11.8 * 9.8 - 4.2 * 4.2];
    expected.sort_by(f64::total_cmp);
    equal(&areas(&index, space::Method::Net), &expected);
}

#[test]
fn circular_void_remains_analytic_at_far_coordinates() {
    for (x, y) in [(0., 0.), (500000., 4400000.)] {
        let walls = vec![(
            vec![Vec2::new(x - 5., y), Vec2::new(x + 5., y)],
            Some(vec![1., 1.]),
            Spec {
                width: 0.2,
                closed: true,
                ..Default::default()
            },
        )];
        let index = BoundaryIndex::new(&inputs(&walls));
        equal(
            &areas(&index, space::Method::Net),
            &[std::f64::consts::PI * 4.9 * 4.9],
        );
        let rooms = index.rooms(space::Method::Net).unwrap();
        assert!(
            rooms[0]
                .boundary
                .outer
                .bulges
                .as_ref()
                .unwrap()
                .iter()
                .any(|b| *b != 0.)
        );
    }
}

#[test]
fn a_four_hundred_room_grid_has_no_material_cells_and_reuses_one_network() {
    let mut walls = vec![rectangle(0., 0., 80., 80.)];
    for i in 1..20 {
        let p = i as f64 * 4.;
        walls.push(wall(&[(p, 0.), (p, 80.)], false));
        walls.push(wall(&[(0., p), (80., p)], false));
    }
    let index = BoundaryIndex::new(&inputs(&walls));
    let result = areas(&index, space::Method::Net);
    assert_eq!(result.len(), 400);
    for area in result {
        assert!((area - 3.8 * 3.8).abs() < 1e-7);
    }
}

#[test]
fn empty_network_and_invalid_curved_wall_do_not_invent_rooms() {
    assert!(
        BoundaryIndex::new(&[])
            .rooms(space::Method::Net)
            .unwrap()
            .is_empty()
    );
    let walls = vec![(
        vec![Vec2::new(-1., 0.), Vec2::new(1., 0.)],
        Some(vec![1., 1.]),
        Spec {
            width: 4.,
            closed: true,
            ..Default::default()
        },
    )];
    assert!(
        BoundaryIndex::new(&inputs(&walls))
            .rooms(space::Method::Net)
            .is_err()
    );
    let query = BoundaryIndex::for_queries(&[]);
    assert!(query.found(Vec2::new(0., 0.), space::Method::Net).is_none());
    assert!(query.rooms(space::Method::Net).is_err());
}

fn separator(a: (f64,f64), b: (f64,f64)) -> kentos_geometry_core::entity::Shape {
    kentos_geometry_core::entity::Shape::Line { a:Vec2::new(a.0,a.1), b:Vec2::new(b.0,b.1) }
}

#[test]
fn virtual_partition_preserves_total_area_in_every_method_and_direction() {
    for (x,y) in [(0.,0.),(500000.,4400000.)] {
        for reversed in [false,true] {
            let walls=vec![rectangle(x,y,10.,8.)];
            let ends=if reversed {((x+5.,y+10.),(x+5.,y-2.))} else {((x+5.,y-2.),(x+5.,y+10.))};
            let sep=vec![separator(ends.0,ends.1)];
            let index=BoundaryIndex::new_with_separators(&inputs(&walls),&sep);
            let old=BoundaryIndex::new(&inputs(&walls));
            for (method,total) in [(space::Method::Net,76.44),(space::Method::Centerline,80.),(space::Method::Gross,83.64)] {
                equal(&areas(&index,method),&[total/2.;2]);
                equal(&areas(&old,method),&[total]);
                let query=BoundaryIndex::for_queries_with_separators(&inputs(&walls),&sep);
                assert_eq!(query.found(Vec2::new(x+2.,y+6.),method),index.found(Vec2::new(x+2.,y+6.),method));
                assert!(index.room_at(Vec2::new(x,y+4.),method).unwrap().is_none(),"physical material became a room");
            }
        }
    }
}

#[test]
fn dangling_virtual_edge_does_not_divide_or_remove_free_area() {
    let walls=vec![rectangle(0.,0.,10.,8.)];
    for sep in [vec![separator((5.,2.),(5.,6.))],vec![separator((5.,0.),(5.,6.))]] {
        equal(&areas(&BoundaryIndex::new_with_separators(&inputs(&walls),&sep),space::Method::Net),&[76.44]);
    }
}

#[test]
fn virtual_cross_t_junction_and_duplicate_edges_keep_measured_faces() {
    let walls=vec![rectangle(0.,0.,10.,8.)];
    let vertical=separator((5.,0.),(5.,8.));
    let cross=vec![vertical.clone(),separator((0.,4.),(10.,4.))];
    equal(&areas(&BoundaryIndex::new_with_separators(&inputs(&walls),&cross),space::Method::Net),&[19.11;4]);
    let tee=vec![vertical.clone(),separator((0.,4.),(5.,4.))];
    equal(&areas(&BoundaryIndex::new_with_separators(&inputs(&walls),&tee),space::Method::Net),&[19.11,19.11,38.22]);
    let duplicate=vec![vertical.clone(),vertical,separator((5.,8.),(5.,0.))];
    equal(&areas(&BoundaryIndex::new_with_separators(&inputs(&walls),&duplicate),space::Method::Net),&[38.22;2]);
}

#[test]
fn closed_virtual_island_is_free_area_inside_and_outside() {
    let walls=vec![rectangle(0.,0.,10.,8.)];
    let sep=vec![separator((2.,2.),(5.,2.)),separator((5.,2.),(5.,4.)),separator((5.,4.),(2.,4.)),separator((2.,4.),(2.,2.))];
    let index=BoundaryIndex::new_with_separators(&inputs(&walls),&sep);
    equal(&areas(&index,space::Method::Net),&[6.,70.44]);
    for method in [space::Method::Net,space::Method::Centerline,space::Method::Gross] {
        equal(&areas(&BoundaryIndex::new_with_separators(&[],&sep),method),&[6.]);
    }
}

#[test]
fn circular_room_split_keeps_analytic_bulges_and_half_circle_areas() {
    for (x,y) in [(0.,0.),(500000.,4400000.)] {
        let walls=vec![(vec![Vec2::new(x-5.,y),Vec2::new(x+5.,y)],Some(vec![1.,1.]),Spec {width:0.2,closed:true,..Default::default()})];
        let sep=vec![separator((x-6.,y),(x+6.,y))];
        let index=BoundaryIndex::new_with_separators(&inputs(&walls),&sep);
        for (method,radius) in [(space::Method::Net,4.9),(space::Method::Centerline,5.),(space::Method::Gross,5.1)] {
            equal(&areas(&index,method),&[std::f64::consts::PI*radius*radius/2.;2]);
            for room in index.rooms(method).unwrap() {
                assert!(room.boundary.outer.bulges.as_ref().unwrap().iter().any(|b|*b!=0.));
            }
        }
    }
}

#[test]
fn empty_separator_list_is_exactly_the_existing_network() {
    let walls=vec![rectangle(500000.,4400000.,10.,8.),wall(&[(500005.,4400000.),(500005.,4400008.)],false)];
    let old=BoundaryIndex::new(&inputs(&walls));let new=BoundaryIndex::new_with_separators(&inputs(&walls),&[]);
    for method in [space::Method::Net,space::Method::Centerline,space::Method::Gross] {
        for seed in [Vec2::new(500002.,4400006.),Vec2::new(500008.,4400006.),Vec2::new(500005.,4400006.)] {
            assert_eq!(old.found(seed,method),new.found(seed,method));
            assert_eq!(old.room_at(seed,method),new.room_at(seed,method));
        }
    }
}
