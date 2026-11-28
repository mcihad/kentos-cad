//! A lintel over an opening (docs/adr/8041 §3): its sizes until a tool is told and the beam over a door or a window.

use kentos_commands::Vec2;
use kentos_domain::Document;

/// A lintel's height when its tool is not told (docs/adr/8041 §3): 20 cm, over TBDY 2018 11.5.3's least 150 mm.
pub const LINTEL_HEIGHT: f64 = 0.20;
/// How far a lintel sits on the wall beyond each jamb (docs/adr/8041 §3): TBDY 2018 11.5.3's least, 200 mm.
pub const LINTEL_BEARING: f64 = 0.20;

/// A lintel over opening `o` (docs/adr/8041 §3): on its wall's axis across its jambs and `bearing` beyond each, as wide
/// as the wall is thick at its jambs, `height` high, its underside at the opening's head over its storey's floor (its
/// own, else a door's 2.10 and a window's 2.20, the joinery list's), on its wall's level. None for an opening without
/// its jambs (broken) or a host.
pub fn lintel_over(
    doc: &Document,
    o: &kentos_commands::OpeningFields,
    height: f64,
    bearing: f64,
) -> Option<kentos_commands::NewBeam> {
    let [l0, r0, l1, r1] = <[Vec2; 4]>::try_from(o.place.clone()).ok()?;
    let mid = |a: Vec2, b: Vec2| Vec2 {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    };
    let (a, b) = (mid(l0, r0), mid(l1, r1));
    let len = kentos_num::hypot(b.x - a.x, b.y - a.y);
    if len.is_nan() || len <= 0.0 {
        return None;
    }
    let u = Vec2 {
        x: (b.x - a.x) / len,
        y: (b.y - a.y) / len,
    };
    let width = kentos_num::hypot(l0.x - r0.x, l0.y - r0.y);
    let head = o.head.unwrap_or(match o.form {
        kentos_commands::OpeningForm::Window => 2.2,
        _ => 2.1,
    });
    let wall = doc.wall_of(&o.host)?;
    Some(kentos_commands::NewBeam {
        pts: vec![
            Vec2 {
                x: a.x - u.x * bearing,
                y: a.y - u.y * bearing,
            },
            Vec2 {
                x: b.x + u.x * bearing,
                y: b.y + u.y * bearing,
            },
        ],
        width,
        height,
        role: Some(kentos_commands::BeamRole::Lintel),
        base_offset: Some(head),
        arch: Some(kentos_commands::ArchInfo {
            level: wall.arch.level.clone(),
            ..Default::default()
        }),
        ..Default::default()
    })
}
