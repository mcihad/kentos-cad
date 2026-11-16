//! The building's IFC (docs/adr/8047): `cad.arch.ifc` v1 and the adapter from KentOS's document to the IFC
//! core. It gives `kentos_ifc::arch` the section model (`sections::model`), the drawing's objects with their ids, the
//! wall styles' names, the rooms' net areas and the project's architecture settings; every IFC rule (classes, names,
//! properties, geometry, what is left out) is in the kentos-ifc crate. The command writes the file whole or not at
//! all; the drawing does not change. The shared cases are `fixtures/commands/v1/cad.arch.ifc.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, ExportIfcBuilding, IfcBuildingExported, IfcView};
use kentos_commands::{SiteEdgeKind, VariableValue};
use kentos_domain::Document;
use kentos_ifc::arch::{Gap, Input, Particulars, Why, Written};
use kentos_ifc::building::Building;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::exchange_files::{fail, now, stamp_of, write_file};
use crate::sections::{self, Left};

/// The drawing's blocks that are building elements (docs/adr/8070 §5): each insert whose block's card says what element it
/// is, with its outline in plan: the box of what its block draws with the insert's values, placed as the insert is
/// (moved, turned, scaled, mirrored); a lift's rated capacity from its `KISI` and `YUK` attributes, a number each.
fn fixtures(doc: &Document) -> Vec<kentos_ifc::arch::Fixture> {
    let mut out = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let kentos_commands::Entity::Insert(i) = e else { continue };
        let Some(def) = doc.block(i.block) else { continue };
        let Some(element) = def.card.as_ref().and_then(|c| c.element.clone()) else {
            continue;
        };
        let Some(outline) = sections::fixture_outline(doc, i) else {
            continue;
        };
        let ring: Vec<[f64; 2]> = outline.iter().map(|q| [q.x, q.y]).collect();
        let number = |tag: &str| -> Option<f64> {
            def.attribute_value(i, tag)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .and_then(|v| v.replace(',', ".").parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v > 0.0)
        };
        out.push(kentos_ifc::arch::Fixture {
            id: uid.as_u128(),
            uid: uid.to_string(),
            name: def.name.clone(),
            element,
            level: i.arch.level.clone(),
            ring,
            people: number("KISI").filter(|v| v.fract() == 0.0).map(|v| v as i64),
            weight: number("YUK"),
        });
    }
    out
}

/// Each storey's emsal areas (docs/adr/8069): the rooms as the ratios count them (`zoning_rooms`), by storey and by how
/// they count, their contours together; an arc of a contour as its chord. The part of a storey's balconies past their
/// units' 20 % goes with its counted-in area.
fn emsal_areas(doc: &Document) -> Vec<kentos_ifc::arch::EmsalArea> {
    use kentos_commands::EmsalCount;
    let rooms = doc.zoning_rooms();
    let mut keys: Vec<(Option<String>, u8)> = Vec::new();
    let rank = |c: EmsalCount| match c {
        EmsalCount::Included => 0u8,
        EmsalCount::Exempt => 1,
        EmsalCount::Outside => 2,
    };
    for r in &rooms {
        let k = (r.level.clone(), rank(r.count));
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    keys.into_iter()
        .map(|(level, rank_of)| {
            let count = rooms.iter().find(|r| r.level == level && rank(r.count) == rank_of).map_or(EmsalCount::Included, |r| r.count);
            let shapes: Vec<kentos_geometry_core::geom::arrangement::Area> = rooms
                .iter()
                .filter(|r| r.level == level && r.count == count)
                .flat_map(|r| r.shape.iter().cloned())
                .collect();
            let union = kentos_geometry_core::geom::region::union_areas(&shapes);
            let ring = |r: &kentos_geometry_core::geom::arrangement::Ring| r.pts.iter().map(|p| [p.x, p.y]).collect::<Vec<[f64; 2]>>();
            let rings = union.iter().flat_map(|a| std::iter::once(ring(&a.outer)).chain(a.holes.iter().map(ring))).collect();
            let moved = if count == EmsalCount::Included {
                rooms.iter().filter(|r| r.level == level).map(|r| r.moved).sum()
            } else {
                0.0
            };
            kentos_ifc::arch::EmsalArea { level, count, rings, moved }
        })
        .collect()
}

/// The building program's units with their rooms (docs/adr/8069), in the program's order.
fn unit_groups(doc: &Document) -> Vec<kentos_ifc::arch::UnitGroup> {
    let Some(arch) = doc.settings().arch.as_ref() else { return Vec::new() };
    arch.program
        .units
        .iter()
        .map(|u| kentos_ifc::arch::UnitGroup {
            id: None,
            number: u.number.clone(),
            name: u.name.clone(),
            kind: u.kind,
            rooms: doc
                .entities_with_uids()
                .filter(|(_, e)| matches!(e, kentos_commands::Entity::Space(s) if s.space.unit.as_deref() == Some(u.id.as_str())))
                .map(|(id, _)| id.as_u128())
                .collect(),
        })
        .collect()
}

/// What a file of one of the program's buildings holds (docs/adr/8072 §2): its storeys (a shared level where it counts)
/// and the objects on them; an opening whose wall it holds; a grid on no level (every building's).
struct Scope {
    building: kentos_commands::ProgramBuilding,
    levels: Vec<String>,
    several: bool,
}

impl Scope {
    fn of(doc: &Document, id: &str) -> Option<Scope> {
        let program = &doc.settings().arch.as_ref()?.program;
        let building = program.buildings.iter().find(|b| b.id == id)?.clone();
        let levels = doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.iter().map(|l| l.id.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|l| program.building_of_level(l).is_some_and(|b| b.id == building.id))
            .collect();
        Some(Scope { building, levels, several: program.buildings.len() > 1 })
    }

    /// Whether the file holds the object: an architectural one on one of its storeys; a grid on no level; an opening
    /// whose wall it holds; anything else (no IFC element) as it is.
    fn holds(&self, doc: &Document, e: &kentos_commands::Entity) -> Option<bool> {
        use kentos_commands::Entity;
        match e {
            Entity::Opening(o) => {
                let host = kentos_domain::Uuid::parse_str(&o.opening.host).ok()?;
                let wall = doc.get(doc.slot_of(host)?)?;
                Some(kentos_ifc::arch::level_of(wall).is_some_and(|l| self.levels.iter().any(|x| x == l)))
            }
            Entity::Grid(g) => Some(g.grid.arch.level.as_deref().is_none_or(|l| self.levels.iter().any(|x| x == l))),
            _ => match kentos_ifc::arch::level_of(e) {
                Some(l) => Some(self.levels.iter().any(|x| x == l)),
                None if matches!(
                    e,
                    Entity::Wall(_)
                        | Entity::Slab(_)
                        | Entity::Column(_)
                        | Entity::Beam(_)
                        | Entity::Roof(_)
                        | Entity::Railing(_)
                        | Entity::Stair(_)
                        | Entity::Ramp(_)
                        | Entity::Space(_)
                ) =>
                {
                    // On no storey: the only building's, no one's beside others.
                    Some(!self.several)
                }
                None => None,
            },
        }
    }
}

/// The building and what was written, with the curved objects written as chords: one whose arcs the model could not
/// take back (a wall's, a slab's and a curved stair's are written as arcs, docs/adr/8071 §1, §9).
fn made(doc: &Document, stamp: &str, application: &str) -> (Building, Written, usize) {
    let (b, w, c, _) = made_for(doc, stamp, application, None);
    (b, w, c)
}

/// [`made`] of one of the program's buildings (docs/adr/8072 §2), and how many architectural objects of the project's
/// no building holds (on no storey, or a shared one no building is chosen for) when it has several.
fn made_for(doc: &Document, stamp: &str, application: &str, scope: Option<&Scope>) -> (Building, Written, usize, usize) {
    let model = sections::model(doc);
    let curved = model
        .left
        .iter()
        .filter(|(why, owner)| {
            *why == Left::Curved
                && owner.as_ref().is_some_and(|o| {
                    !model
                        .owners
                        .iter()
                        .zip(&model.exact)
                        .any(|(w, e)| w.as_deref() == Some(o.as_str()) && e.is_some())
                })
        })
        .count();
    // One building's file: its storeys' objects only (docs/adr/8072 §2).
    let mut unplaced = 0;
    let mut kept: Vec<String> = Vec::new();
    let entities: Vec<(u128, String, &kentos_commands::Entity)> = doc
        .entities_with_uids()
        .filter(|(u, e)| match scope.map(|s| s.holds(doc, e)) {
            None | Some(None) | Some(Some(true)) => {
                kept.push(u.to_string());
                true
            }
            Some(Some(false)) => {
                let program = doc.settings().arch.as_ref().map(|a| &a.program);
                let level = kentos_ifc::arch::level_of(e);
                let no_one = match (program, level) {
                    (Some(p), Some(l)) => p.building_of_level(l).is_none(),
                    (_, None) => !matches!(e, kentos_commands::Entity::Opening(_) | kentos_commands::Entity::Grid(_)),
                    _ => false,
                };
                if no_one {
                    unplaced += 1;
                }
                false
            }
        })
        .map(|(u, e)| (u.as_u128(), u.to_string(), e))
        .collect();
    let in_scope = |owner: &Option<String>| scope.is_none() || owner.as_ref().is_some_and(|o| kept.contains(o));
    let gaps: Vec<Gap> = model
        .gaps
        .iter()
        .filter(|g| in_scope(&g.owner))
        .map(|g| Gap {
            ring: g.ring.clone(),
            bulges: g.bulges.clone(),
            sill: g.sill,
            head: g.head,
            form: g.form,
            owner: g.owner.clone(),
        })
        .collect();
    // Each solid's part (docs/adr/8070 §3, §7), as the IFC model reads it.
    let parts: Vec<Option<kentos_ifc::arch::Part>> = model
        .parts
        .iter()
        .map(|p| {
            p.map(|p| match p {
                sections::Part::Roof(k) => kentos_ifc::arch::Part::Roof(k),
                sections::Part::Mullion(k) => kentos_ifc::arch::Part::Mullion(k),
                sections::Part::Transom(k) => kentos_ifc::arch::Part::Transom(k),
                sections::Part::Panel => kentos_ifc::arch::Part::Panel,
            })
        })
        .collect();
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    let mut net_area = |room: &kentos_commands::SpaceFields| search.net_area(room);
    let wall_style_name = |style: Option<&str>| doc.wall_style_name(style);
    let (b, written) = kentos_ifc::arch::building(Input {
        title: doc.name(),
        stamp,
        application,
        levels: model
            .levels
            .iter()
            .filter(|(id, ..)| scope.is_none_or(|s| s.levels.contains(id)))
            .cloned()
            .collect(),
        solids: &model.solids,
        owners: &model.owners,
        parts: &parts,
        exact: &model.exact,
        gaps,
        slab_cuts: model
            .slab_cuts
            .iter()
            .filter(|c| in_scope(&c.owner))
            .filter_map(|c| {
                Some(kentos_ifc::arch::SlabCut {
                    owner: c.owner.clone()?,
                    source: c.source.clone(),
                    ring: c.ring.iter().map(|p| [p.x, p.y]).collect(),
                    bulges: c.bulges.clone(),
                })
            })
            .collect(),
        entities,
        wall_style_name: &wall_style_name,
        wall_styles: &doc.settings().wall_styles,
        net_area: &mut net_area,
        arch: doc.settings().arch.as_ref(),
        particulars: particulars(doc),
        crs: crs(doc),
        emsal: emsal_areas(doc)
            .into_iter()
            .filter(|z| scope.is_none_or(|s| z.level.as_ref().is_some_and(|l| s.levels.contains(l))))
            .collect(),
        groups: unit_groups(doc)
            .into_iter()
            .filter(|g| scope.is_none_or(|s| unit_building(doc, &g.number).as_deref() == Some(s.building.id.as_str())))
            .collect(),
        fixtures: fixtures(doc)
            .into_iter()
            .filter(|f| scope.is_none_or(|s| f.level.as_ref().is_some_and(|l| s.levels.contains(l))))
            .collect(),
        coverings: doc
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                kentos_commands::Entity::Space(s) => Some((u.to_string(), doc.room_coverings(&s.space))),
                _ => None,
            })
            .collect(),
        // Each room's floor and ceiling (docs/adr/8040 §3, 8071 §7); with none known, the roofs on its storey
        // (docs/adr/8071 §11).
        rooms: doc
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                kentos_commands::Entity::Space(s) => doc.room_height(&s.space).map(|h| {
                    (
                        u.to_string(),
                        kentos_ifc::arch::RoomLevels {
                            floor: h.floor,
                            ceiling: h.ceiling,
                            known: h.ceiling_known(),
                            suspended: h.ceiling_from == kentos_domain::CeilingFrom::Suspended,
                            roof: if h.ceiling_known() {
                                Vec::new()
                            } else {
                                undersides(doc, s.space.arch.level.as_deref())
                            },
                        },
                    )
                }),
                _ => None,
            })
            .collect(),
        building: scope.map(|s| kentos_ifc::arch::BuildingOf {
            id: s.building.id.clone(),
            name: s.building.name.clone(),
            code: s.building.code.clone(),
            usage: s.building.usage.clone(),
            zero: s.building.zero.unwrap_or(0.0),
            several: s.several,
        }),
    });
    (b, written, curved, unplaced)
}

/// The program's building a unit belongs to, by the unit's number.
fn unit_building(doc: &Document, number: &str) -> Option<String> {
    doc.settings().arch.as_ref()?.program.units.iter().find(|u| u.number == number)?.building.clone()
}

/// The faces of the roofs standing on a storey (docs/adr/8071 §11), each its outline in plan and its underside's plane
/// over ±0.00: its top's plane less its build-up, upright as the section model stacks it (one build-up its thickness;
/// layers across the face, each × √(1 + slope²)); a flat roof one face over its eaves.
fn undersides(doc: &Document, level: Option<&str>) -> Vec<kentos_ifc::under_roof::UnderFace> {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::roof as core;
    let Some(level) = level else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_, e) in doc.entities_with_uids() {
        let kentos_commands::Entity::Roof(r) = e else {
            continue;
        };
        if r.roof.arch.level.as_deref() != Some(level) {
            continue;
        }
        let Shape::Roof { pts, roof: spec } = crate::geometry::roof_shape(r, doc.settings()) else {
            continue;
        };
        let depth = |dx: f64, dy: f64| {
            if r.roof.layers.is_empty() {
                r.roof.thickness_m()
            } else {
                r.roof.layers.iter().map(|l| l.thickness).sum::<f64>() * (1.0 + dx * dx + dy * dy).sqrt()
            }
        };
        let ring = |q: &[kentos_geometry_core::Vec2]| q.iter().map(|p| [p.x, p.y]).collect::<Vec<[f64; 2]>>();
        match core::face_planes(&pts, &spec) {
            Ok(faces) if !faces.is_empty() => {
                for (f, (c, dx, dy)) in faces {
                    out.push((ring(&f.ring), (c - depth(dx, dy), dx, dy)));
                }
            }
            Ok(_) => out.push((ring(&pts), (spec.base - depth(0.0, 0.0), 0.0, 0.0))),
            Err(_) => {}
        }
    }
    out
}

/// The drawing's künye (the permit set's particulars, docs/adr/8037) and the measures its title block reads, for the
/// IFC's EK-6 and EK-7 words: a value not given, or given as "—" (the künye's default), is none. The areas are İmar
/// durumu's (yapı inşaat alanı: all the storeys'; taban alanı), the use the building program's, the gardens' distances
/// the Vaziyet planı's, each kind's least edge (a public area's edge an ön bahçe, PAİY 23/1-a).
pub fn particulars(doc: &Document) -> Particulars {
    let fold = |s: &str| kentos_sheet::expr::fold(s);
    let vars = &doc.settings().variables;
    let get = |name: &str| -> Option<String> {
        let v = vars.iter().find(|v| fold(&v.name) == fold(name))?;
        let t = match &v.value {
            VariableValue::Text(t) => t.trim().to_owned(),
            VariableValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => {
                format!("{}", *n as i64)
            }
            VariableValue::Number(n) => format!("{n}"),
            VariableValue::Null | VariableValue::Bool(_) => return None,
        };
        (!t.is_empty() && t != "—" && t != "-").then_some(t)
    };
    let facts = doc.title_facts();
    let fact = |name: &str| {
        facts
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.clone())
    };
    let zoning = doc.zoning_report();
    let mut setbacks: [Option<f64>; 3] = [None; 3];
    if let Some(site) = doc.site_report() {
        for e in &site.edges {
            let k = match e.kind {
                SiteEdgeKind::Front | SiteEdgeKind::Public => 0,
                SiteEdgeKind::Side => 1,
                SiteEdgeKind::Back => 2,
            };
            setbacks[k] = Some(setbacks[k].map_or(e.required, |v: f64| v.min(e.required)));
        }
    }
    Particulars {
        work: get("proje_adi")
            .or_else(|| Some(doc.name().trim().to_owned()).filter(|n| !n.is_empty())),
        project_code: get("proje_kodu"),
        building_code: get("bina_kodu"),
        revision: get("revizyon_no"),
        province: get("il"),
        district: get("ilce"),
        neighbourhood: get("mahalle"),
        block: get("ada"),
        parcel: get("parsel"),
        title_deed: get("tapu_kaydi"),
        architect: get("mimar"),
        architect_title: get("mimar_unvan"),
        registration: get("sicil_no"),
        office: get("buro_adi"),
        office_tax_number: get("vergi_no"),
        office_address: get("buro_adres"),
        occupancy: fact("kullanim_amaci"),
        gross_area: zoning
            .as_ref()
            .map(|r| r.levels.iter().map(|l| l.all()).sum::<f64>()),
        footprint: zoning.as_ref().map(|r| r.footprint),
        setbacks,
        // The künye's building kind (Ruhsat takımı › Dijital teslim kapsamı) as the construction action.
        construction: get("yapi_turu").map(|k| match k.as_str() {
            "yeni" => "Yeni yapı".to_owned(),
            "ilave" => "İlave".to_owned(),
            "tadilat" => "Tadilat".to_owned(),
            _ => k,
        }),
    }
}

/// The project's system as the IFC says it (EK-7 Tablo 7.6–7.7, `kentos_ifc::crs`): the drawing's coordinates are the
/// system's, so the map conversion moves them by nothing in plan and by ±0.00's absolute elevation (Vaziyet planı) in
/// height; its heights' datum the project's (an orthometric system by its EPSG name, ellipsoidal heights ITRF96's, EK-7's
/// note). None for a drawing in local coordinates.
pub fn crs(doc: &Document) -> Option<kentos_ifc::crs::Crs> {
    let settings = doc.settings();
    let own = kentos_project::systems::own(settings)?;
    let name = if own.code.starts_with("EPSG:") {
        own.code.clone()
    } else {
        own.name.clone()
    };
    let vertical = settings.heights.as_ref().and_then(|h| match h.kind {
        kentos_commands::HeightKind::Orthometric => h
            .vertical()
            .map(|v| format!("{} (EPSG:{})", v.name, v.srid)),
        kentos_commands::HeightKind::Ellipsoidal => Some("ITRF96".to_owned()),
        kentos_commands::HeightKind::Unknown => None,
    });
    let datum = settings
        .arch
        .as_ref()
        .and_then(|a| a.site.as_ref())
        .and_then(|s| s.datum)
        .unwrap_or(0.0);
    Some(kentos_ifc::crs::projected(
        &name,
        Some(own.title.clone()),
        vertical,
        [0.0, 0.0, datum],
    ))
}

/// The model file's name the save dialog offers (EK-2 Tablo 2.1): `260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc`, or the
/// drawing's name while the künye has no project code.
pub fn file_name(doc: &Document) -> String {
    format!("{}.ifc", particulars(doc).file_stem(doc.name()))
}

/// The model as `cad.arch.ifc` writes it, what it left out, and the künye it read: for Mimari denetim's Dijital proje
/// findings (`digital_audit`, EK-9).
pub fn model(doc: &Document) -> (Building, Written, Particulars) {
    let (b, written, _) = made(doc, "", "KentOS Mimari");
    (b, written, particulars(doc))
}

/// EK-9's model quality control form as a table's rows (`kentos_ifc::quality::form_rows`): its header, each group's
/// title and its questions answered from the model `cad.arch.ifc` writes now.
pub fn form_rows(doc: &Document) -> Vec<Vec<String>> {
    let (b, written, k) = model(doc);
    kentos_ifc::quality::form_rows(&kentos_ifc::quality::ek9(&b, &written, &k))
}

/// The building to write: `stamp` its time (`YYYY-MM-DDThh:mm:ss`), `application` KentOS's name.
pub fn building(doc: &Document, stamp: &str, application: &str) -> (Building, Written) {
    let (b, written, _) = made(doc, stamp, application);
    (b, written)
}

/// One of the program's buildings to write (docs/adr/8072 §2): its storeys and their objects, at its ±0.00; with how
/// many of the project's architectural objects no building holds. None for a building the program does not have.
pub fn building_of(doc: &Document, id: &str, stamp: &str, application: &str) -> Option<(Building, Written, usize)> {
    let scope = Scope::of(doc, id)?;
    let (b, written, _, unplaced) = made_for(doc, stamp, application, Some(&scope));
    Some((b, written, unplaced))
}

/// The model file's name of one of the program's buildings (EK-2 Tablo 2.1): its code in the building code's place;
/// without the künye's project code the drawing's name with the building's code (else its id) after it.
pub fn file_name_of(doc: &Document, id: &str) -> String {
    let mut k = particulars(doc);
    let code = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.program.buildings.iter().find(|b| b.id == id))
        .and_then(|b| b.code.clone());
    if k.project_code.is_none() {
        return format!("{}_{}.ifc", doc.name(), code.unwrap_or_else(|| id.to_owned()));
    }
    if code.is_some() {
        k.building_code = code;
    }
    format!("{}.ifc", k.file_stem(doc.name()))
}

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_IFC, words, path)
}

fn losses(message: String) -> CommandWarning {
    CommandWarning {
        code: codes::EXPORT_LOSSES.into(),
        message,
        path: None,
    }
}

/// What was left out for `why`, its kinds in this order with how many ("2 duvar, 1 kapı"); empty: nothing.
fn counted(written: &Written, why: Why) -> String {
    const ORDER: [&str; 13] = [
        "duvar",
        "döşeme",
        "kolon",
        "kiriş",
        "çatı",
        "korkuluk",
        "merdiven",
        "rampa",
        "kapı",
        "pencere",
        "açıklık",
        "mahal",
        "aks",
    ];
    ORDER
        .iter()
        .filter_map(|what| {
            let n = written
                .left
                .iter()
                .filter(|l| l.what == *what && l.why == why)
                .count();
            (n > 0).then(|| format!("{n} {what}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct Planned {
    path: PathBuf,
    out: IfcBuildingExported,
    text: String,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ExportIfcBuilding) -> Result<Planned, Stop> {
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin.".into(),
            "path",
        ));
    }
    if !text.to_ascii_lowercase().ends_with(".ifc") {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{text}” bir .ifc dosyası değil."),
            "path",
        ));
    }
    let path = PathBuf::from(text);
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” klasörü yok; dosya var olan bir klasöre yazılır.",
                folder.display()
            ),
            "path",
        ));
    }
    if path.exists() && !input.overwrite {
        return Err(fail(
            codes::FILE_EXISTS,
            format!(
                "{} var; üzerine yazmak için overwrite verin.",
                path.display()
            ),
            "path",
        ));
    }
    let stamp = match input.stamp.as_deref() {
        None => {
            let (d, t) = now();
            format!("{d}T{t}")
        }
        Some(s) => {
            let (d, t) = stamp_of(s).ok_or_else(|| {
                bad(
                    "stamp",
                    format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın."),
                )
            })?;
            format!("{d}T{t}")
        }
    };
    let application = format!("KentOS Mimari {}", env!("CARGO_PKG_VERSION"));
    // One of the program's buildings (docs/adr/8072 §2), else the project as one.
    let scope = match input.building.as_deref() {
        Some(id) => Some(Scope::of(doc, id).ok_or_else(|| {
            bad("building", format!("“{id}” binası bina programında yok; programdaki bir binanın kimliğini verin."))
        })?),
        None => None,
    };
    let (mut b, written, curved, unplaced) = made_for(doc, &stamp, &application, scope.as_ref());
    // Its bodies whole, or already cut (docs/adr/8071 §4).
    b.view = match input.view {
        IfcView::DesignTransfer => kentos_ifc::building::View::DesignTransfer,
        IfcView::Reference => kentos_ifc::building::View::Reference,
    };
    let held = written.walls
        + written.slabs
        + written.columns
        + written.beams
        + written.roofs
        + written.railings
        + written.stairs
        + written.ramps
        + written.chimneys
        + written.fixtures
        + written.curtain_walls
        + written.spaces;
    if held == 0 {
        return Err(bad(
            "",
            "Yazılacak bina yok: çizimde gövdesi olan duvar, döşeme, kolon, kiriş, çatı, korkuluk, merdiven, rampa ya da \
             mahal yok."
                .into(),
        ));
    }
    let mut warnings = Vec::new();
    for (why, because) in [
        (Why::NoBody, "kesit modeli gövdesini kuramadı"),
        (Why::NoWall, "duvarı yazılmadı"),
        (Why::NoArea, "alanı yok"),
        (Why::OneWay, "katında hep bir yönde; IFC'nin aks ızgarası iki yönde aks ister"),
    ] {
        let list = counted(&written, why);
        if !list.is_empty() {
            warnings.push(losses(format!("{list} IFC'ye yazılmadı: {because}.")));
        }
    }
    if curved > 0 {
        warnings.push(losses(format!(
            "{curved} yayı alınamayan eğri nesne IFC'ye düz parçalarla yazıldı."
        )));
    }
    if unplaced > 0 {
        warnings.push(losses(format!(
            "{unplaced} nesnenin binası belli değil (katı yok ya da ortak katın sayıldığı bina seçilmedi); hiçbir binanın \
             dosyasına yazılmadı. Nesneye kat verin ya da Bina programı › Binalar'da ortak katın binasını seçin."
        )));
    }
    // Every body a closed, simple solid (docs/adr/8071 §3): a body that is not one is said, with the first thing wrong.
    let open = kentos_ifc::closed::open_bodies(&b);
    if !open.is_empty() {
        let shown: Vec<String> = open.iter().take(5).map(|(name, why)| format!("{name}: {why}")).collect();
        let more = if open.len() > 5 { format!(" ve {} nesne daha", open.len() - 5) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::IFC_OPEN_BODY.into(),
            message: format!(
                "{} nesnenin gövdesi kapalı bir katı değil: {}{more}. Nesneyi düzeltip yeniden aktarın.",
                open.len(),
                shown.join("; ")
            ),
            path: None,
        });
    }
    let text = kentos_ifc::building::write(&b);
    let sha256 = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let n = |x: usize| x as u32;
    let out = IfcBuildingExported {
        path: path.display().to_string(),
        schema: "IFC4X3_ADD2".into(),
        view: input.view,
        storeys: n(written.storeys),
        walls: n(written.walls),
        doors: n(written.doors),
        windows: n(written.windows),
        openings: n(written.openings),
        spaces: n(written.spaces),
        slabs: n(written.slabs),
        columns: n(written.columns),
        beams: n(written.beams),
        roofs: n(written.roofs),
        railings: n(written.railings),
        stairs: n(written.stairs),
        // The elements of docs/adr/8070 (§9).
        ramps: n(written.ramps),
        chimneys: n(written.chimneys),
        curtain_walls: n(written.curtain_walls),
        members: n(written.members),
        fixtures: n(written.fixtures),
        grid_axes: n(written.grid_axes),
        coverings: n(written.coverings),
        bytes: text.len() as u64,
        sha256,
    };
    Ok(Planned {
        path,
        out,
        text,
        warnings,
    })
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ExportIfcBuilding) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ExportIfcBuilding,
) -> CommandResult<IfcBuildingExported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ExportIfcBuilding,
) -> CommandResult<IfcBuildingExported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&p.path, &p.text) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("IFC yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed {
        output: p.out,
        warnings: p.warnings,
    }
}
