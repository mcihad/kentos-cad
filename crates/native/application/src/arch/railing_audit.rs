//! Korkuluğun mevzuat denetimi (docs/adr/8045 §4): Planlı Alanlar İmar Yönetmeliği 38 on the drawing's railings, its
//! slabs' voids and its balconies and terraces. A finding says the measure, the limit, the article and its gazette.
//!
//! - **Yükseklik (38/1):** "döşeme kotundan itibaren en az 1.10 metre yüksekliğe kadar": a railing lower; offered: 1.10.
//! - **Boşluk (38/4):** "en fazla 0.10 metre çapında geçişe izin verilir": a balustered railing's clear gap wider;
//!   offered: PAİY's 0.10 (its own gap taken away).
//! - **Galeri boşluğu (38/1):** a slab's void's sides a railing guards (`railing_voids`: free of walls, open where a stair
//!   from below arrives) run 0.10 m or more with no railing along them (none of its storey's within half its top rail and
//!   5 cm, nor the slab's own edge). Drawn with Korkuluk's Boşluktan; offered no fix: the railing's layer is the tool's.
//! - **Balkon ve teras (38/1):** a balcony's or a terrace's free sides with no railing along them and no edge of its own;
//!   offered: its edge a railing (`cad.slabs.edit`), along every free side as Döşeme draws it.
//!
//! Only stretches of 0.10 m and more count: PAİY 38/4's 0.10 m passes through no shorter one.

use kentos_commands::{
    Entity, RAILING_GAP, RAILING_HEIGHT, RailingEntity, SlabFields, SlabRole, codes,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use serde_json::json;

use crate::regulation_audit::{Article, Fix, RAILING_ARTICLE, Said, metres};

/// PAİY 38/4: a railing's openings.
pub const GAP_ARTICLE: Article = Article::paiy("38/4", "3.7.2017/30113");

/// The codes the railings' rules find, in their order (a missing railing is PAİY's `paiy_railing_missing`, said with the
/// stairs' and ramps').
pub const CODES: [&str; 2] = [codes::PAIY_RAILING_LOW, codes::PAIY_RAILING_GAP];

/// How far a railing's line may stand from the side it guards past half its top rail, metres.
const ALONG: f64 = 0.05;
/// How finely a side is walked to see which of it a railing guards, metres.
const STEP: f64 = 0.05;

/// A railing's line as stretches, round to its first point when closed.
fn stretches(r: &RailingEntity) -> Vec<(V, V)> {
    let p: Vec<V> = r.railing.pts.iter().map(|q| V::new(q.x, q.y)).collect();
    let mut out: Vec<(V, V)> = p.windows(2).map(|w| (w[0], w[1])).collect();
    if r.railing.closed
        && let (Some(&a), Some(&b)) = (p.last(), p.first())
    {
        out.push((a, b));
    }
    out
}

fn distance(p: V, (a, b): (V, V)) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - a.x - dx * t).hypot(p.y - a.y - dy * t)
}

/// How much of `sides` no railing of `level` guards, metres: walked every [`STEP`], a point guarded when a railing's line
/// runs within half its top rail and [`ALONG`] of it.
fn unguarded(doc: &Document, level: Option<&str>, sides: &[(Vec<V>, bool)]) -> f64 {
    let guards: Vec<(f64, Vec<(V, V)>)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Railing(r) if level.is_none() || r.railing.arch.level.as_deref() == level => {
                Some((r.railing.width / 2.0 + ALONG, stretches(r)))
            }
            _ => None,
        })
        .collect();
    let guarded = |p: V| {
        guards
            .iter()
            .any(|(near, lines)| lines.iter().any(|s| distance(p, *s) <= *near))
    };
    let mut out = 0.0;
    for (line, closed) in sides {
        let mut pts = line.clone();
        if *closed && let Some(&first) = line.first() {
            pts.push(first);
        }
        for w in pts.windows(2) {
            let l = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
            let n = (l / STEP).ceil().max(1.0) as usize;
            for k in 0..n {
                let t = (k as f64 + 0.5) / n as f64;
                let p = V::new(
                    w[0].x + (w[1].x - w[0].x) * t,
                    w[0].y + (w[1].y - w[0].y) * t,
                );
                if !guarded(p) {
                    out += l / n as f64;
                }
            }
        }
    }
    out
}

/// A ring's middle, its corners' mean.
fn middle(pts: &[kentos_commands::Vec2]) -> V {
    let n = pts.len().max(1) as f64;
    V::new(
        pts.iter().map(|p| p.x).sum::<f64>() / n,
        pts.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

/// The railings' findings (docs/adr/8045 §4), each said through `push(said, slots, at, fix)`: the railing or the slab is
/// the first slot, a fix its command's operation on it.
pub(crate) fn findings(doc: &Document, push: &mut impl FnMut(Said, Vec<Slot>, V, Option<Fix>)) {
    for e in doc.entities() {
        let Entity::Railing(r) = e else { continue };
        let x = &r.railing;
        let slot = Slot(r.base.id);
        let at = x.pts.first().map_or(V::new(0.0, 0.0), |p| V::new(p.x, p.y));
        if x.height < RAILING_HEIGHT - 1e-9 {
            push(
                Said {
                    code: codes::PAIY_RAILING_LOW,
                    message: format!(
                        "Korkuluk {} < {} — {}",
                        metres(x.height),
                        metres(RAILING_HEIGHT),
                        RAILING_ARTICLE.short()
                    ),
                    repair: "Korkuluğu döşemesinden en az 1,10 m yükseltin.".into(),
                    rule: RAILING_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                at,
                Some(Fix {
                    label: "Yüksekliği 1,10 m yap".into(),
                    operation: json!({"kind": "set", "height": RAILING_HEIGHT}),
                    command: None,
                }),
            );
        }
        if let Some(gap) = x.baluster_gap()
            && gap > RAILING_GAP + 1e-9
        {
            push(
                Said {
                    code: codes::PAIY_RAILING_GAP,
                    message: format!(
                        "Korkuluğun çubuk aralığı {} > {} — {}",
                        metres(gap),
                        metres(RAILING_GAP),
                        GAP_ARTICLE.short()
                    ),
                    repair: "Çubukları sıklaştırın: aralarından 0,10 m çaplı bir cisim geçmesin."
                        .into(),
                    rule: GAP_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                at,
                Some(Fix {
                    label: "Aralığı 0,10 m yap".into(),
                    operation: json!({"kind": "set", "gap": null}),
                    command: None,
                }),
            );
        }
    }
    for e in doc.entities() {
        let Entity::Slab(s) = e else { continue };
        let x: &SlabFields = &s.slab;
        let slot = Slot(s.base.id);
        let level = x.arch.level.as_deref();
        // Its own edge, a railing or a parapet, guards every free side it has, its voids' too (docs/adr/0722 §4).
        if x.edge.is_some() {
            continue;
        }
        // Its voids (38/1: "galeri boşluğu").
        for hole in x.parts.iter().flat_map(|p| &p.holes) {
            let sides = crate::railing_voids::void_sides(doc, x, hole, level);
            let open = unguarded(doc, level, &sides);
            if open < RAILING_GAP {
                continue;
            }
            push(
                Said {
                    code: codes::PAIY_RAILING_MISSING,
                    message: format!(
                        "Döşeme boşluğunun {}'lik kenarında korkuluk yok — {}",
                        metres(open),
                        RAILING_ARTICLE.short()
                    ),
                    repair: "Korkuluk aracında Çizgi'yi Boşluktan yapıp boşluğun içine tıklayın."
                        .into(),
                    rule: RAILING_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                middle(&hole.pts),
                None,
            );
        }
        // A balcony's or a terrace's free sides (38/1: "balkon, teras").
        let whose = match x.role {
            SlabRole::Balcony => "Balkonun",
            SlabRole::Terrace => "Terasın",
            SlabRole::Floor => continue,
        };
        let sides: Vec<(Vec<V>, bool)> = doc
            .slab_runs(x)
            .into_iter()
            .map(|run| {
                let mut pts: Vec<V> = run.pts.iter().map(|p| V::new(p.x, p.y)).collect();
                let closed = pts.len() > 3 && pts.first() == pts.last();
                if closed {
                    pts.pop();
                }
                (pts, closed)
            })
            .collect();
        let open = unguarded(doc, level, &sides);
        if open < RAILING_GAP {
            continue;
        }
        let at = x
            .parts
            .first()
            .map_or(V::new(0.0, 0.0), |p| middle(&p.outer.pts));
        push(
            Said {
                code: codes::PAIY_RAILING_MISSING,
                message: format!(
                    "{whose} {}'lik serbest kenarında korkuluk yok — {}",
                    metres(open),
                    RAILING_ARTICLE.short()
                ),
                repair: "Serbest kenarlarına korkuluk ya da parapet koyun.".into(),
                rule: RAILING_ARTICLE.named(),
                binding: None,
            },
            vec![slot],
            at,
            Some(Fix {
                label: "Serbest kenarlarına korkuluk koy".into(),
                operation: json!({"kind": "set", "edge": {"kind": "railing"}}),
                command: Some("cad.slabs.edit"),
            }),
        );
    }
}
