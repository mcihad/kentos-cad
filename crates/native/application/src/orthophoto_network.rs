//! `cad.orthophoto.network` v1 and `cad.orthophoto.footprint` v1 (plan/map-cad.md MAP-12, docs/adr/5260): the mosaic
//! elements of two or more orthophotos, each place going to the covering image whose footprint's centre is nearest
//! (`kentos_geometry_core::ops::seam_network::network`), with the junctions where three or more meet; and a raster's
//! footprint from its pixels that are not NoData (`outline`, or `hull`). Each writes its areas in one undo step; the
//! layer is made in the same step when none is given and the drawing has none. The shared cases are
//! `fixtures/commands/v1/cad.orthophoto.network.json` and `cad.orthophoto.footprint.json`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    AreaPart, CommandResult, Entity, EntityBase, FOOTPRINT_LAYER, FOOTPRINT_LAYER_NAME, MOST_FOOTPRINT_PIXELS,
    MOST_NETWORK_IMAGES, OrthoNetworkElement, OrthoNetworkJunction, OrthoNetworkPart, OrthophotoFootprint,
    OrthophotoFootprintBuilt, OrthophotoNetwork, OrthophotoNetworkBuilt, PathEntity, RingGeometry, SEAMLINE_LAYER,
    SEAMLINE_LAYER_NAME, Vec2,
};
use kentos_domain::{Document, NewLayer};
use kentos_geometry_core::ops::seam_network::{self as net, Pt};
use kentos_geometry_core::ops::seamlines::{SeamFault, convex_footprint};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::orthophoto_seamlines::{footprint, raster};

/// The undo steps' names.
pub const NETWORK_STEP: &str = "Mozaik elemanları ağı";
pub const FOOTPRINT_STEP: &str = "Ayak izi";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn bad(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_INPUT, message.to_owned(), path)
}

/// The layer written on: given (there, not locked) or the default one, made when the drawing has none.
fn layer_of(doc: &Document, asked: Option<&str>, default: &str, what: &str) -> Result<(String, bool), Stop> {
    let layer = asked.map(str::trim).unwrap_or(default).to_owned();
    match doc.layers().get(&layer) {
        Some(node) if doc.layers().is_locked(&layer) => Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” katmanı kilitli; {what} bu katmana yazılır. Kilidi Katmanlar panelinden açın.", node.name),
            "layer",
        )),
        Some(_) => Ok((layer, false)),
        None if asked.is_some() => Err(fail(codes::LAYER_NOT_FOUND, format!("“{layer}” katmanı çizimde yok."), "layer")),
        None => Ok((layer, true)),
    }
}

/// An area of `parts` on `layer` with `attrs`: the first part the area's ring, the others its parts.
fn area(layer: &str, parts: &[OrthoNetworkPart], attrs: BTreeMap<String, String>) -> Entity {
    let pts = |r: &[Pt]| r.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect::<Vec<_>>();
    let holes = |p: &OrthoNetworkPart| {
        (!p.holes.is_empty()).then(|| {
            p.holes
                .iter()
                .map(|h| RingGeometry { pts: pts(h), bulges: None, zs: None })
                .collect::<Vec<_>>()
        })
    };
    let rest: Vec<AreaPart> = parts[1..]
        .iter()
        .map(|p| AreaPart { pts: pts(&p.ring), bulges: None, holes: holes(p), zs: None, ms: None })
        .collect();
    Entity::Polygon(PathEntity {
        space_separator: None,
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.to_owned(),
            color: None,
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        pts: pts(&parts[0].ring),
        bulges: None,
        holes: holes(&parts[0]),
        zs: None,
        ms: None,
        parts: (!rest.is_empty()).then_some(rest),
        closed: false,
        widths: None,
    })
}

/// Writes `entities` (and the layer when it is to be made) as one undo step; their ids.
fn written(
    cx: &mut ExecutionContext<'_>,
    step: &str,
    entities: Vec<Entity>,
    opened: Option<(&str, &str)>,
) -> Result<Vec<String>, Stop> {
    let group = cx.doc.begin_group(step);
    if let Some((id, name)) = opened {
        let layer = NewLayer { id: Some(id.to_owned()), ..NewLayer::layer(name) };
        if let Err(refusal) = cx.doc.add_layer(layer, None, false) {
            cx.doc.cancel_group(group);
            return Err(Stop::failed(error(codes::LAYER_NOT_FOUND, refusal.to_string(), Some("layer".into()))));
        }
    }
    let Ok(slots) = cx.doc.add_many(entities, step) else {
        cx.doc.cancel_group(group);
        return Err(Stop::failed(error(codes::INVALID_INPUT, "Çizimde yeni nesneye yer kalmadı.".into(), None)));
    };
    cx.doc.end_group(group);
    Ok(slots.iter().filter_map(|s| cx.doc.uid(*s).map(|u| u.to_string())).collect())
}

fn parts_of(e: &net::Element) -> Vec<OrthoNetworkPart> {
    e.parts.iter().map(|p| OrthoNetworkPart { ring: p.ring.clone(), holes: p.holes.clone() }).collect()
}

pub mod network {
    //! `cad.orthophoto.network` v1's handlers.
    use super::*;

    /// The answer and the areas to write.
    fn run(doc: &Document, input: &OrthophotoNetwork) -> Result<(OrthophotoNetworkBuilt, Vec<Entity>), Stop> {
        let n = input.images.len();
        if !(2..=MOST_NETWORK_IMAGES).contains(&n) {
            return Err(bad(&format!("Ağ 2 ile {MOST_NETWORK_IMAGES} arasında ortofotoyla kurulur; {n} verildi."), "images"));
        }
        let mut seen = HashSet::new();
        let mut rasters = Vec::new();
        for (k, uid) in input.images.iter().enumerate() {
            let path = format!("images[{k}]");
            let (id, r) = raster(doc, uid, &path)?;
            if !seen.insert(id) {
                return Err(bad("Aynı ortofoto iki kez verildi; her görüntü ağda bir kez yer alır.", &path));
            }
            rasters.push((id, r));
        }
        if !input.footprints.is_empty() && input.footprints.len() != n {
            return Err(bad(
                &format!("Ayak izleri görüntülerle aynı sırada, her görüntüye bir tane verilir ({n} görüntü, {} ayak izi).", input.footprints.len()),
                "footprints",
            ));
        }
        let (layer, opened) = layer_of(doc, input.layer.as_deref(), SEAMLINE_LAYER, "mozaik elemanları")?;
        let given = !input.footprints.is_empty();
        let mut rings = Vec::new();
        for k in 0..n {
            let path = if given { format!("footprints[{k}]") } else { format!("images[{k}]") };
            let raw: Vec<Pt> = if given {
                footprint(doc, &input.footprints[k], &path)?
            } else {
                rasters[k].1.raster.corners().iter().map(|&(x, y)| [x, y]).collect()
            };
            let what = if given { "ayak izi" } else { "görüntünün sınırı" };
            let ring = convex_footprint(&raw, k).map_err(|f| match f {
                SeamFault::NotConvex(_) => bad(
                    &format!("{}. {what} dışbükey değil; ağ dışbükey ayak izleriyle kurulur. İçbükey ayak izini parçalara bölün ya da Ayak izi'nin zarfını kullanın.", k + 1),
                    &path,
                ),
                _ => bad(&format!("{}. {what} alan kaplamıyor (üçten az köşe ya da sıfır alan).", k + 1), &path),
            })?;
            rings.push(ring);
        }
        let size = rings.iter().flatten().fold(1.0_f64, |m, p| m.max(p[0].abs()).max(p[1].abs()));
        let built = net::network(&rings, size * 1e-12 + 1e-9);
        let ids: Vec<String> = rasters.iter().map(|(id, _)| id.to_string()).collect();
        if let Some(k) = built.elements.iter().position(|e| e.parts.is_empty()) {
            return Err(bad(
                &format!(
                    "{}. görüntünün ayak izinin her yeri başka bir görüntünün merkezine daha yakın; mozaikte ona yer kalmıyor. Onu ağdan çıkarın.",
                    k + 1
                ),
                &format!("images[{k}]"),
            ));
        }
        let elements: Vec<OrthoNetworkElement> = built
            .elements
            .iter()
            .enumerate()
            .map(|(k, e)| OrthoNetworkElement {
                image: ids[k].clone(),
                parts: parts_of(e),
                area: e.area,
                centre: net::centroid(&rings[k]),
                uids: Vec::new(),
            })
            .collect();
        let junctions: Vec<OrthoNetworkJunction> = built
            .junctions
            .iter()
            .map(|(at, who)| OrthoNetworkJunction { at: *at, images: who.iter().map(|&k| ids[k].clone()).collect() })
            .collect();
        let covered: f64 = elements.iter().map(|e| e.area).sum();
        let mut notes = vec![format!(
            "{} görüntünün mozaik elemanları: her yer, onu kaplayan görüntülerden ayak izinin merkezi en yakın olanın; kaplanan alan {:.2} m².",
            n, covered
        )];
        notes.push(match junctions.len() {
            0 => "Üç ya da daha çok elemanın buluştuğu düğüm yok.".to_owned(),
            k => format!("{k} düğümde üç ya da daha çok eleman buluşuyor (üçlü düğüm)."),
        });
        if elements.iter().any(|e| e.parts.len() > 1) {
            notes.push("Bir elemanın birden çok parçası var: görüntü mozaikte ayrık yerlerde kullanılır.".into());
        }
        notes.push("Birleşme çizgileri elemanların ortak kenarlarıdır. BÖHHBÜY birleşme çizgisinin yerine kural koymaz; kayıklık m. 67(6) ile ayrıca denetlenir (Ortofoto karşılaştırması).".into());
        let entities = elements
            .iter()
            .flat_map(|e| {
                let mut attrs = BTreeMap::new();
                attrs.insert("Görüntü".to_owned(), e.image.clone());
                attrs.insert("Yöntem".to_owned(), "merkeze en yakın".to_owned());
                std::iter::once(area(&layer, &e.parts, attrs))
            })
            .collect();
        let out = OrthophotoNetworkBuilt { elements, junctions, covered, layer, opened, notes, revision: doc.revision().to_string() };
        Ok((out, entities))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &OrthophotoNetwork) -> CommandResult<()> {
        match run(cx.doc, input) {
            Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &OrthophotoNetwork) -> CommandResult<OrthophotoNetworkBuilt> {
        match run(cx.doc, input) {
            Ok((out, _)) => CommandResult::Completed { output: out, warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    fn write(cx: &mut ExecutionContext<'_>, input: OrthophotoNetwork) -> CommandResult<OrthophotoNetworkBuilt> {
        if let Err(stop) = checks::revision(cx.doc, input.expected_revision.as_deref()) {
            return stop.into();
        }
        let (mut out, entities) = match run(cx.doc, &input) {
            Ok(done) => done,
            Err(stop) => return stop.into(),
        };
        let made = out.opened.then_some((SEAMLINE_LAYER, SEAMLINE_LAYER_NAME));
        match written(cx, NETWORK_STEP, entities, made) {
            Ok(uids) => {
                for (e, uid) in out.elements.iter_mut().zip(uids) {
                    e.uids = vec![uid];
                }
            }
            Err(stop) => return stop.into(),
        }
        out.revision = cx.doc.revision().to_string();
        CommandResult::Completed { output: out, warnings: Vec::new() }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: OrthophotoNetwork) -> CommandResult<OrthophotoNetworkBuilt> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_ORTHOPHOTO_NETWORK,
            kentos_commands::CAD_ORTHOPHOTO_NETWORK_VERSION,
            input,
            write,
        )
    }
}

pub mod outline {
    //! `cad.orthophoto.footprint` v1's handlers.
    use super::*;

    /// Whether the point (column, row) is inside the ring of pixel corners (ray to the east, crossings counted).
    fn inside(ring: &[[i64; 2]], p: [f64; 2]) -> bool {
        let mut inside = false;
        let n = ring.len();
        for k in 0..n {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            let (ay, by) = (a[1] as f64, b[1] as f64);
            if (ay > p[1]) != (by > p[1]) {
                let x = a[0] as f64 + (p[1] - ay) / (by - ay) * (b[0] - a[0]) as f64;
                if p[0] < x {
                    inside = !inside;
                }
            }
        }
        inside
    }

    fn run(doc: &Document, input: &OrthophotoFootprint) -> Result<(OrthophotoFootprintBuilt, Entity), Stop> {
        let (id, r) = raster(doc, &input.raster, "raster")?;
        let (layer, opened) = layer_of(doc, input.layer.as_deref(), FOOTPRINT_LAYER, "ayak izi")?;
        let (w, h) = (r.raster.width, r.raster.height);
        let pixels = u64::from(w) * u64::from(h);
        if pixels > MOST_FOOTPRINT_PIXELS {
            return Err(bad(
                &format!("Raster {pixels} piksel; ayak izi en çok {MOST_FOOTPRINT_PIXELS} pikselden çıkarılır. Daha küçük bir kopyasını kullanın."),
                "raster",
            ));
        }
        let mut o = crate::orthophoto_seams::opened(doc, r, input.folder.as_deref(), "raster")?;
        let nodata = o.reader.nodata;
        let region = o
            .region(0, 0, w, h)
            .map_err(|e| fail(codes::IMPORT_UNREADABLE, format!("Rasterin pikselleri okunamadı: {e}"), "raster"))?
            .ok_or_else(|| fail(codes::IMPORT_UNREADABLE, "Rasterin pikselleri okunamadı.".into(), "raster"))?;
        let (wu, hu) = (w as usize, h as usize);
        let mut valid = vec![false; wu * hu];
        for j in 0..h {
            for i in 0..w {
                valid[j as usize * wu + i as usize] = (0..region.bands).any(|b| {
                    let v = region.at(i, j, b);
                    !v.is_nan() && nodata.is_none_or(|n| v != n)
                });
            }
        }
        let count = valid.iter().filter(|v| **v).count() as u64;
        if count == 0 {
            return Err(bad("Rasterin her pikseli NoData; ayak izi yok.", "raster"));
        }
        let ground = |p: [i64; 2]| -> Pt {
            let (x, y) = r.raster.at(p[0] as f64, p[1] as f64);
            [x, y]
        };
        let size = r.raster.corners().iter().fold(1.0_f64, |m, p| m.max(p.0.abs()).max(p.1.abs()));
        let tol = size * 1e-12 + 1e-9;
        let parts: Vec<OrthoNetworkPart> = if input.hull {
            let ring = net::hull(&valid, wu, hu).into_iter().map(ground).collect::<Vec<_>>();
            vec![OrthoNetworkPart { ring: net::tidy(&ring, true, tol), holes: Vec::new() }]
        } else {
            let rings = net::outline(&valid, wu, hu);
            let outers: Vec<&Vec<[i64; 2]>> = rings.iter().filter(|r| !r.1).map(|r| &r.0).collect();
            let area2 = |r: &[[i64; 2]]| -> i64 {
                (0..r.len()).map(|k| r[k][0] * r[(k + 1) % r.len()][1] - r[(k + 1) % r.len()][0] * r[k][1]).sum::<i64>().abs()
            };
            let mut holes_of: Vec<Vec<&Vec<[i64; 2]>>> = vec![Vec::new(); outers.len()];
            for (hole, _) in rings.iter().filter(|r| r.1) {
                // The invalid pixel left of the hole's first edge (rows down: left of (dx, dy) is (dy, −dx)).
                let (a, b) = (hole[0], hole[1 % hole.len()]);
                let d = [(b[0] - a[0]).signum(), (b[1] - a[1]).signum()];
                let at = [a[0] as f64 + 0.5 * d[0] as f64 + 0.5 * d[1] as f64, a[1] as f64 + 0.5 * d[1] as f64 - 0.5 * d[0] as f64];
                if let Some(k) = (0..outers.len()).filter(|&k| inside(outers[k], at)).min_by_key(|&k| area2(outers[k])) {
                    holes_of[k].push(hole);
                }
            }
            let mut parts: Vec<OrthoNetworkPart> = outers
                .iter()
                .zip(holes_of)
                .map(|(o, hs)| OrthoNetworkPart {
                    ring: net::tidy(&o.iter().copied().map(ground).collect::<Vec<_>>(), true, tol),
                    holes: hs.iter().map(|h| net::tidy(&h.iter().copied().map(ground).collect::<Vec<_>>(), false, tol)).collect(),
                })
                .collect();
            for p in &mut parts {
                p.holes.sort_by(|a, b| a[0][1].total_cmp(&b[0][1]).then(a[0][0].total_cmp(&b[0][0])));
            }
            parts.sort_by(|a, b| a.ring[0][1].total_cmp(&b.ring[0][1]).then(a.ring[0][0].total_cmp(&b.ring[0][0])));
            parts
        };
        let o0 = parts[0].ring[0];
        let rel = |r: &[Pt]| net::signed2(&r.iter().map(|q| [q[0] - o0[0], q[1] - o0[1]]).collect::<Vec<_>>()) / 2.0;
        let area_m2: f64 = parts.iter().map(|p| rel(&p.ring) + p.holes.iter().map(|h| rel(h)).sum::<f64>()).sum();
        let mut notes = vec![if input.hull {
            format!("Ayak izi: NoData olmayan {count} pikselin dışbükey zarfı, {area_m2:.2} m²; birleşme çizgisi ve ağ bunu ayak izi olarak alır.")
        } else {
            format!(
                "Ayak izi: NoData olmayan {count} pikselin dış sınırı, piksellerin kenarlarından; {} parça, {} delik, {area_m2:.2} m².",
                parts.len(),
                parts.iter().map(|p| p.holes.len()).sum::<usize>()
            )
        }];
        notes.push(match nodata {
            Some(v) => format!("NoData değeri {v}: bir piksel, bantlarından biri bu değerde değilse geçerlidir. Köşeden değen pikseller ayrı parçadır."),
            None => "Rasterin NoData değeri yok: her piksel geçerli, ayak izi rasterin sınırıdır.".into(),
        });
        let mut attrs = BTreeMap::new();
        attrs.insert("Görüntü".to_owned(), id.to_string());
        attrs.insert("Yöntem".to_owned(), if input.hull { "dışbükey zarf" } else { "piksel sınırı" }.to_owned());
        let entity = area(&layer, &parts, attrs);
        let out = OrthophotoFootprintBuilt {
            parts,
            area: area_m2,
            pixels,
            valid: count,
            nodata,
            hull: input.hull,
            layer,
            opened,
            uid: None,
            notes,
            revision: doc.revision().to_string(),
        };
        Ok((out, entity))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &OrthophotoFootprint) -> CommandResult<()> {
        match run(cx.doc, input) {
            Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &OrthophotoFootprint) -> CommandResult<OrthophotoFootprintBuilt> {
        match run(cx.doc, input) {
            Ok((out, _)) => CommandResult::Completed { output: out, warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    fn write(cx: &mut ExecutionContext<'_>, input: OrthophotoFootprint) -> CommandResult<OrthophotoFootprintBuilt> {
        if let Err(stop) = checks::revision(cx.doc, input.expected_revision.as_deref()) {
            return stop.into();
        }
        let (mut out, entity) = match run(cx.doc, &input) {
            Ok(done) => done,
            Err(stop) => return stop.into(),
        };
        let made = out.opened.then_some((FOOTPRINT_LAYER, FOOTPRINT_LAYER_NAME));
        match written(cx, FOOTPRINT_STEP, vec![entity], made) {
            Ok(uids) => out.uid = uids.into_iter().next(),
            Err(stop) => return stop.into(),
        }
        out.revision = cx.doc.revision().to_string();
        CommandResult::Completed { output: out, warnings: Vec::new() }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: OrthophotoFootprint) -> CommandResult<OrthophotoFootprintBuilt> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT,
            kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT_VERSION,
            input,
            write,
        )
    }
}
