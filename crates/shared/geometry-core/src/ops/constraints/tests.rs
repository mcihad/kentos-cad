//! The solver's own cases; the shared cases with their independent reference are `fixtures/constraints/v1`.

use super::*;
use crate::vec2::Vec2;

fn line(ax: f64, ay: f64, bx: f64, by: f64) -> Shape {
    Shape::Line {
        a: Vec2::new(ax, ay),
        b: Vec2::new(bx, by),
    }
}

const SEG: Element = Element::Segment {
    part: 0,
    ring: 0,
    index: 0,
};
const START: Element = Element::Point(PointAt::Start);
const END: Element = Element::Point(PointAt::End);

fn r(object: usize, element: Element) -> Ref {
    Ref { object, element }
}

fn c(rule: Rule) -> Constraint {
    Constraint {
        rule,
        value: 0.0,
        reference: false,
    }
}

fn dim(rule: Rule, value: f64) -> Constraint {
    Constraint {
        rule,
        value,
        reference: false,
    }
}

fn rectangle() -> Vec<Shape> {
    vec![
        line(0.0, 0.0, 6.0, 0.0),
        line(6.0, 0.0, 6.0, 4.0),
        line(6.0, 4.0, 0.0, 4.0),
        line(0.0, 4.0, 0.0, 0.0),
    ]
}

/// The corners joined, the bottom edge horizontal, opposite edges parallel, a corner square, a corner held, and
/// the width and height.
fn rectangle_rules(width: f64) -> Vec<Constraint> {
    let mut out: Vec<Constraint> = (0..4)
        .map(|i| {
            c(Rule::Coincident {
                a: r(i, END),
                b: r((i + 1) % 4, START),
            })
        })
        .collect();
    out.push(c(Rule::Horizontal {
        a: r(0, SEG),
        b: None,
    }));
    out.push(c(Rule::Parallel {
        a: r(0, SEG),
        b: r(2, SEG),
    }));
    out.push(c(Rule::Parallel {
        a: r(1, SEG),
        b: r(3, SEG),
    }));
    out.push(c(Rule::Perpendicular {
        a: r(0, SEG),
        b: r(1, SEG),
    }));
    out.push(c(Rule::Fix { a: r(0, START) }));
    out.push(dim(
        Rule::Distance {
            a: r(0, START),
            b: r(0, END),
            axis: Axis::Aligned,
            left: false,
        },
        width,
    ));
    out.push(dim(
        Rule::Distance {
            a: r(1, START),
            b: r(1, END),
            axis: Axis::Aligned,
            left: false,
        },
        4.0,
    ));
    out
}

fn solve_of(shapes: &[Shape], constraints: &[Constraint]) -> Outcome {
    solve(
        &Problem {
            shapes,
            constraints,
            held: &[],
            fix_from: None,
        },
        &|| false,
    )
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn len(v: Vec2) -> f64 {
    crate::jsmath::js_hypot(v.x, v.y)
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn ends(shape: &Shape) -> (Vec2, Vec2) {
    match shape {
        Shape::Line { a, b } => (*a, *b),
        _ => panic!("not a line"),
    }
}

#[test]
fn a_rectangle_widened_stays_a_rectangle() {
    let shapes = rectangle();
    let out = solve_of(&shapes, &rectangle_rules(8.5));
    assert_eq!(out.status, Status::Fully, "{out:?}");
    assert_eq!(out.dof, 0);
    let l: Vec<(Vec2, Vec2)> = out.shapes.iter().map(ends).collect();
    let d = |i: usize| sub(l[i].1, l[i].0);
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(close(len(d(0)), 8.5) && close(len(d(1)), 4.0));
    assert!(
        close(cross(d(0), d(2)), 0.0) && close(cross(d(1), d(3)), 0.0),
        "opposite edges parallel"
    );
    assert!(
        close(dot(d(0), d(1)), 0.0) && close(dot(d(1), d(2)), 0.0),
        "corners square"
    );
    assert!(
        close(l[0].0.x, 0.0) && close(l[0].0.y, 0.0),
        "the held corner stays"
    );
    // The least change: the widened rectangle still runs east and north.
    assert!(l[1].0.x > 8.0 && l[2].0.y > 3.9);
    // The left edge does not move.
    assert_eq!(out.changed, vec![0, 1, 2]);
}

#[test]
fn a_conflict_writes_nothing() {
    let shapes = rectangle();
    let mut rules = rectangle_rules(6.0);
    rules.push(dim(
        Rule::Distance {
            a: r(2, START),
            b: r(2, END),
            axis: Axis::Aligned,
            left: false,
        },
        7.0,
    ));
    let out = solve_of(&shapes, &rules);
    assert_eq!(out.status, Status::Inconsistent, "{out:?}");
    assert!(out.conflicting.contains(&(rules.len() - 1)));
    assert!(out.changed.is_empty());
    assert_eq!(out.shapes, shapes);
}

#[test]
fn a_repeated_constraint_is_redundant() {
    let shapes = rectangle();
    let mut rules = rectangle_rules(6.0);
    rules.push(dim(
        Rule::Distance {
            a: r(2, START),
            b: r(2, END),
            axis: Axis::Aligned,
            left: false,
        },
        6.0,
    ));
    let out = solve_of(&shapes, &rules);
    assert_eq!(out.status, Status::Over, "{out:?}");
    assert_eq!(out.redundant, vec![rules.len() - 1]);
    assert_eq!(out.dof, 0);
}

#[test]
fn freedoms_are_counted() {
    let shapes = rectangle();
    let rules: Vec<Constraint> = rectangle_rules(6.0).into_iter().take(8).collect();
    let out = solve_of(&shapes, &rules);
    // Four lines (16), joined into a quadrilateral (8), a horizontal parallelogram (5) with a square corner (4).
    assert_eq!(out.status, Status::Under);
    assert_eq!(out.dof, 4);
    assert_eq!(out.free, vec![0, 1, 2, 3]);
    assert!(out.changed.is_empty(), "nothing to move");
}

#[test]
fn a_circle_tangent_to_a_line_and_its_radius() {
    let shapes = vec![
        line(0.0, 0.0, 10.0, 0.0),
        Shape::Circle {
            c: Vec2::new(5.0, 3.0),
            r: 2.0,
        },
    ];
    let rules = vec![
        c(Rule::Fix { a: r(0, SEG) }),
        c(Rule::Tangent {
            a: r(0, SEG),
            b: r(1, Element::Circle),
            inside: false,
            left: true,
        }),
        dim(
            Rule::Radius {
                a: r(1, Element::Circle),
            },
            2.5,
        ),
    ];
    let out = solve_of(&shapes, &rules);
    assert_eq!(out.status, Status::Under, "{out:?}");
    assert_eq!(out.dof, 1);
    let Shape::Circle {
        c: centre,
        r: radius,
    } = out.shapes[1]
    else {
        panic!()
    };
    assert!(
        (radius - 2.5).abs() < 1e-9
            && (centre.y - 2.5).abs() < 1e-9
            && (centre.x - 5.0).abs() < 1e-9
    );
}

#[test]
fn an_arc_end_joins_a_line() {
    let shapes = vec![
        line(0.0, 0.0, 4.0, 0.0),
        Shape::Arc {
            c: Vec2::new(4.0, 2.0),
            r: 2.0,
            a0: -1.4,
            a1: 1.0,
        },
    ];
    let rules = vec![
        c(Rule::Fix { a: r(0, SEG) }),
        c(Rule::Coincident {
            a: r(1, START),
            b: r(0, END),
        }),
        c(Rule::Tangent {
            a: r(0, SEG),
            b: r(1, Element::Circle),
            inside: false,
            left: true,
        }),
    ];
    let out = solve_of(&shapes, &rules);
    assert!(out.status.holds(), "{out:?}");
    let Shape::Arc {
        c: centre,
        r: radius,
        a0,
        ..
    } = out.shapes[1]
    else {
        panic!()
    };
    let start = Vec2::new(
        centre.x + radius * libm::cos(a0),
        centre.y + radius * libm::sin(a0),
    );
    assert!(len(sub(start, Vec2::new(4.0, 0.0))) < 1e-9 && (centre.y - radius).abs() < 1e-9);
}

#[test]
fn reference_measures_and_measure() {
    let shapes = rectangle();
    let width = Rule::Distance {
        a: r(0, START),
        b: r(0, END),
        axis: Axis::Aligned,
        left: false,
    };
    assert_eq!(measure(&shapes, &width), Some(6.0));
    let angle = Rule::Angle {
        a: r(0, SEG),
        b: r(1, SEG),
        reverse_a: false,
        reverse_b: false,
    };
    assert!((measure(&shapes, &angle).unwrap() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    let mut rules = rectangle_rules(7.0);
    rules.push(Constraint {
        rule: width,
        value: 0.0,
        reference: true,
    });
    let out = solve_of(&shapes, &rules);
    assert_eq!(out.status, Status::Fully);
    let (i, v) = out.measured[0];
    assert_eq!(i, rules.len() - 1);
    assert!((v - 7.0).abs() < 1e-9);
}

#[test]
fn a_missing_part_breaks_its_constraint_only() {
    let shapes = vec![
        line(0.0, 0.0, 1.0, 0.2),
        Shape::Circle {
            c: Vec2::new(0.0, 0.0),
            r: 1.0,
        },
    ];
    let rules = vec![
        c(Rule::Horizontal {
            a: r(0, SEG),
            b: None,
        }),
        c(Rule::Horizontal {
            a: r(1, SEG),
            b: None,
        }),
        c(Rule::Fix { a: r(5, START) }),
    ];
    let out = solve_of(&shapes, &rules);
    assert_eq!(out.broken, vec![1, 2]);
    assert!(out.status.holds());
    let (a, b) = ends(&out.shapes[0]);
    assert!((a.y - b.y).abs() < 1e-12);
}

#[test]
fn a_held_grip_drives_the_rest() {
    // The width dimension absent: dragging the right edge's end decides the width.
    let mut shapes = rectangle();
    let before = shapes.clone();
    if let Shape::Line { b, .. } = &mut shapes[0] {
        *b = Vec2::new(9.0, 0.0);
    }
    let rules: Vec<Constraint> = rectangle_rules(6.0)
        .into_iter()
        .filter(|c| c.rule != rectangle_rules(6.0)[9].rule)
        .collect();
    let held = [Hold {
        object: 0,
        element: Some(END),
    }];
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &rules,
            held: &held,
            fix_from: Some(&before),
        },
        &|| false,
    );
    assert!(out.status.holds(), "{out:?}");
    let (_, b) = ends(&out.shapes[1]);
    assert!((b.x - 9.0).abs() < 1e-9 && (b.y - 4.0).abs() < 1e-9);
}

#[test]
fn a_stop_cancels() {
    let shapes = rectangle();
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &rectangle_rules(9.0),
            held: &[],
            fix_from: None,
        },
        &|| true,
    );
    assert_eq!(out.status, Status::Cancelled);
    assert!(out.changed.is_empty());
}

/// `rows` × `cols` rectangles sharing their edges' corners in one group, widened by one dimension.
fn grid(rows: usize, cols: usize) -> (Vec<Shape>, Vec<Constraint>) {
    let mut shapes = Vec::new();
    let mut rules = Vec::new();
    for i in 0..rows {
        for j in 0..cols {
            let (x, y) = (j as f64 * 6.0, i as f64 * 4.0);
            let base = shapes.len();
            shapes.extend([
                line(x, y, x + 6.0, y),
                line(x + 6.0, y, x + 6.0, y + 4.0),
                line(x + 6.0, y + 4.0, x, y + 4.0),
                line(x, y + 4.0, x, y),
            ]);
            for k in 0..4 {
                rules.push(c(Rule::Coincident {
                    a: r(base + k, END),
                    b: r(base + (k + 1) % 4, START),
                }));
            }
            rules.push(c(Rule::Horizontal {
                a: r(base, SEG),
                b: None,
            }));
            rules.push(c(Rule::Parallel {
                a: r(base, SEG),
                b: r(base + 2, SEG),
            }));
            rules.push(c(Rule::Parallel {
                a: r(base + 1, SEG),
                b: r(base + 3, SEG),
            }));
            rules.push(c(Rule::Perpendicular {
                a: r(base, SEG),
                b: r(base + 1, SEG),
            }));
            let w = if i == 0 && j == 0 { 7.0 } else { 6.0 };
            rules.push(dim(
                Rule::Distance {
                    a: r(base, START),
                    b: r(base, END),
                    axis: Axis::Aligned,
                    left: false,
                },
                w,
            ));
            rules.push(dim(
                Rule::Distance {
                    a: r(base + 1, START),
                    b: r(base + 1, END),
                    axis: Axis::Aligned,
                    left: false,
                },
                4.0,
            ));
            if j > 0 {
                // Its left edge's bottom on its neighbour's right edge's bottom.
                let left = base - 4;
                rules.push(c(Rule::Coincident {
                    a: r(base, START),
                    b: r(left + 1, START),
                }));
            } else if i > 0 {
                let below = base - 4 * cols;
                rules.push(c(Rule::Coincident {
                    a: r(base, START),
                    b: r(below + 3, START),
                }));
            } else {
                rules.push(c(Rule::Fix { a: r(base, START) }));
            }
        }
    }
    (shapes, rules)
}

#[test]
fn a_grid_is_fully_constrained() {
    let (shapes, rules) = grid(3, 3);
    let out = solve_of(&shapes, &rules);
    assert_eq!(
        out.status,
        Status::Fully,
        "{:?}",
        (out.status, out.dof, &out.redundant, &out.conflicting)
    );
    // The first rectangle widened by a metre moves every one east of it.
    let (a, _) = ends(&out.shapes[4]);
    assert!((a.x - 7.0).abs() < 1e-9);
}

#[test]
#[ignore]
fn timing() {
    let free = |(shapes, rules): (Vec<Shape>, Vec<Constraint>)| {
        // Without their dimensions: every rectangle keeps its two freedoms.
        (
            shapes,
            rules
                .into_iter()
                .filter(|c| !c.rule.dimensional())
                .collect::<Vec<_>>(),
        )
    };
    type Case = (&'static str, (Vec<Shape>, Vec<Constraint>), usize);
    let cases: Vec<Case> = vec![
        ("1000 ayrı dikdörtgen", grid(1, 1), 1000),
        ("10 × 10 bağlı dikdörtgen", grid(10, 10), 1),
        ("20 × 20 bağlı dikdörtgen", grid(20, 20), 1),
        ("50 × 50 bağlı dikdörtgen", grid(50, 50), 1),
        ("20 × 20 ölçüsüz", free(grid(20, 20)), 1),
        ("50 × 50 ölçüsüz", free(grid(50, 50)), 1),
    ];
    for (label, (shapes, rules), times) in cases {
        let started = std::time::Instant::now();
        let mut out = None;
        for _ in 0..times {
            out = Some(solve_of(&shapes, &rules));
        }
        let out = out.unwrap();
        println!(
            "{label}: {:.1} ms ({:?}, {} serbestlik, {} adım, {} kısıt)",
            started.elapsed().as_secs_f64() * 1e3,
            out.status,
            out.dof,
            out.iterations,
            rules.len() * times
        );
    }
}
