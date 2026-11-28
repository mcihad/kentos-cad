//! Which objects a DXF file can hold and how the report names them: every number of an object finite (a file holds
//! nothing else), its kind in Turkish as the app says it.

use kentos_contracts::{Entity, Vec2};

use crate::gis::Zs;

/// How the report names a kind (Turkish, as the app's ENTITY_KIND_LABEL).
pub(super) fn kind_label(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "Nokta",
        Entity::Line(_) => "Çizgi",
        Entity::Polyline(_) => "Çoklu çizgi",
        Entity::Polygon(_) => "Kapalı alan",
        Entity::Circle(_) => "Daire",
        Entity::Arc(_) => "Yay",
        Entity::Ellipse(_) => "Elips",
        Entity::Spline(_) => "Eğri",
        Entity::Nurbs(_) => "NURBS eğrisi",
        Entity::Xline(_) => "Yardımcı çizgi",
        Entity::Ray(_) => "Işın",
        Entity::Text(_) => "Yazı",
        Entity::Dimension(_) => "Ölçü",
        Entity::Hatch(_) => "Tarama",
        Entity::Insert(_) => "Blok",
        Entity::Leader(_) => "Kılavuz",
        Entity::Table(_) => "Tablo",
        Entity::Image(_) => "Resim",
        Entity::Raster(_) => "Raster",
        Entity::PointCloud(_) => "Nokta bulutu",
        Entity::Record(_) => "Kayıt",
        Entity::Wall(_) => "Duvar",
        Entity::Opening(_) => "Açıklık",
        Entity::Space(_) => "Mahal",
        Entity::Slab(_) => "Döşeme",
        Entity::Mass(_) => "Kütle",
        Entity::Grid(_) => "Aks",
        Entity::Column(_) => "Kolon",
        Entity::Stair(_) => "Merdiven",
        Entity::Ramp(_) => "Rampa",
        Entity::Beam(_) => "Kiriş",
        Entity::Roof(_) => "Çatı",
        Entity::Railing(_) => "Korkuluk",
        Entity::Tag(_) => "Etiket",
        Entity::ViewMark(_) => "Görünüm işareti",
    }
}

fn ok(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn all_ok(pts: &[Vec2]) -> bool {
    pts.iter().all(|p| ok(*p))
}

fn nums_ok(xs: &[f64]) -> bool {
    xs.iter().all(|x| x.is_finite())
}

fn zs_ok(zs: &Zs) -> bool {
    zs.iter().flatten().flatten().all(|z| z.is_finite())
}

/// Every number of an object is finite (a file can hold nothing else).
pub(super) fn finite(e: &Entity) -> bool {
    match e {
        Entity::Point(p) => {
            ok(p.p)
                && p.z.is_none_or(f64::is_finite)
                && p.parts
                    .iter()
                    .flatten()
                    .all(|q| ok(q.p) && q.z.is_none_or(f64::is_finite))
        }
        Entity::Line(l) => {
            ok(l.a) && ok(l.b) && l.za.is_none_or(f64::is_finite) && l.zb.is_none_or(f64::is_finite)
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let ring = |pts: &[Vec2], zs: &Zs, bulges: Option<&[f64]>| {
                all_ok(pts) && zs_ok(zs) && bulges.is_none_or(nums_ok)
            };
            let holes = |holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
                holes
                    .iter()
                    .flatten()
                    .all(|h| ring(&h.pts, &h.zs, h.bulges.as_deref()))
            };
            p.widths
                .iter()
                .flatten()
                .all(|[a, b]| a.is_finite() && b.is_finite())
                && ring(&p.pts, &p.zs, p.bulges.as_deref())
                && holes(&p.holes)
                && p.parts.iter().flatten().all(|part| {
                    ring(&part.pts, &part.zs, part.bulges.as_deref()) && holes(&part.holes)
                })
        }
        Entity::Circle(c) => ok(c.c) && c.r.is_finite(),
        Entity::Arc(a) => ok(a.c) && nums_ok(&[a.r, a.a0, a.a1]),
        Entity::Ellipse(e) => ok(e.c) && ok(e.major) && nums_ok(&[e.ratio, e.t0, e.t1]),
        Entity::Spline(s) => all_ok(&s.pts),
        Entity::Nurbs(n) => {
            all_ok(&n.ctrl)
                && nums_ok(&n.knots)
                && n.weights.as_deref().is_none_or(nums_ok)
                && n.fit.as_deref().is_none_or(all_ok)
                && n.tangents.iter().flatten().flatten().all(|t| ok(*t))
                && n.fit_tolerance.is_none_or(f64::is_finite)
        }
        Entity::Xline(x) | Entity::Ray(x) => ok(x.p) && ok(x.dir),
        Entity::Text(t) => ok(t.p) && nums_ok(&[t.height, t.rotation]),
        Entity::Dimension(d) => {
            ok(d.a)
                && ok(d.b)
                && d.c.is_none_or(ok)
                && nums_ok(&[d.offset, d.height])
                && d.angle.is_none_or(f64::is_finite)
        }
        Entity::Hatch(h) => {
            all_ok(&h.ring)
                && h.holes.iter().flatten().all(|r| all_ok(r))
                && nums_ok(&[h.pattern.angle, h.pattern.spacing])
                && h.pattern.scale.is_none_or(f64::is_finite)
                && h.pattern.lines.iter().flatten().all(|l| {
                    nums_ok(&[l.angle, l.origin[0], l.origin[1], l.offset[0], l.offset[1]])
                        && nums_ok(&l.dashes)
                })
        }
        Entity::Insert(i) => ok(i.p) && nums_ok(&[i.scale, i.rotation]),
        Entity::Leader(l) => all_ok(&l.pts) && nums_ok(&[l.height, l.rotation]),
        // Never written (docs/adr/0192, 0204, Kapsam dışı): said in the report.
        Entity::Image(_) | Entity::Raster(_) | Entity::PointCloud(_) | Entity::Record(_) => true,
        Entity::Wall(w) => w.wall.pts.iter().all(|p| ok(*p)) && w.wall.width.is_finite(),
        Entity::Opening(o) => o.opening.place.iter().all(|p| ok(*p)) && o.opening.width.is_finite(),
        Entity::Space(s) => s
            .space
            .parts
            .iter()
            .all(|p| all_ok(&p.outer.pts) && p.holes.iter().all(|h| all_ok(&h.pts))),
        Entity::Slab(s) => {
            s.slab
                .parts
                .iter()
                .all(|p| all_ok(&p.outer.pts) && p.holes.iter().all(|h| all_ok(&h.pts)))
                && nums_ok(&[s.slab.thickness, s.slab.offset])
        }
        // A mass's rule holds its numbers finite (docs/adr/8178).
        Entity::Mass(m) => m.mass.problem().is_none(),
        Entity::Grid(g) => all_ok(&g.grid.pts),
        Entity::Column(c) => ok(c.column.at) && nums_ok(&[c.column.width, c.column.rotation]),
        Entity::Stair(s) => all_ok(&s.stair.pts) && nums_ok(&[s.stair.width, s.stair.tread]),
        Entity::Ramp(r) => all_ok(&r.ramp.pts) && nums_ok(&[r.ramp.width, r.ramp.rise]),
        Entity::Beam(b) => {
            all_ok(&b.beam.pts) && nums_ok(&[b.beam.width, b.beam.height, b.beam.offset])
        }
        Entity::Roof(r) => {
            all_ok(&r.roof.pts)
                && r.roof.slopes.iter().flatten().all(|s| s.is_finite())
                && r.roof.offset.is_finite()
        }
        Entity::Railing(r) => {
            all_ok(&r.railing.pts)
                && nums_ok(&[r.railing.height, r.railing.width, r.railing.offset])
        }
        Entity::Tag(t) => ok(t.tag.at),
        Entity::ViewMark(m) => m.mark.points.iter().all(|p| ok(*p)),
        Entity::Table(t) => {
            ok(t.p)
                && nums_ok(&[t.height, t.rotation])
                && nums_ok(&t.rows)
                && nums_ok(&t.columns)
                && t.face.oblique.is_none_or(f64::is_finite)
        }
    }
}
