//! `cad.arch.performance` v1 (“Yangın ve performans”, docs/adr/8068; plan/bim-plan.md BIM-07): the author's
//! declarations for the fire regulation and the model, as one undo step: objects' own fire resistance classes and exit
//! roles (a roof's as an escape route) and a wall's load-bearing role (docs/adr/8090), a wall style's or a door or window
//! type's class, a material's strength and reaction to fire class (docs/adr/8091), the building's fire protection
//! class.
//! Its plan is what it would write: the objects that carry the value then. The shared cases are
//! `fixtures/commands/v1/cad.arch.performance.json` (`scripts/fixtures/arch_performance_cases.py`).
//!
//! - **Carried:** an object's own class, else its wall style's (a wall) or its type's (a door, a window); the object's
//!   own wins ([`fire_rating_of`]). A material's strength names it in the model (`Beton_C25`).
//! - **Nothing is a default:** null takes a value away; it is missing again. An empty word is refused.

use kentos_commands::{
    ArchPerformance, ArchPerformanceDone, CommandResult, Declared, Entity, OpeningForm, PerformanceOperation,
    ProjectSettings, reaction_problem, roof_class_problem, strength_problem,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Yangın ve performans";

/// Whether `e` takes a fire resistance class: a building element (a wall, a slab, a column, a beam, a roof, a railing, a
/// stair, a ramp, a door, a window or a bare opening).
pub fn fire_rated(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Wall(_)
            | Entity::Slab(_)
            | Entity::Column(_)
            | Entity::Beam(_)
            | Entity::Roof(_)
            | Entity::Railing(_)
            | Entity::Stair(_)
            | Entity::Ramp(_)
            | Entity::Opening(_)
    )
}

/// Whether `e` takes an emergency exit role: a door or a stair, or a roof as an escape route (docs/adr/8090, BYKHY
/// Ek-3/B 4-a).
pub fn exit_rated(e: &Entity) -> bool {
    match e {
        Entity::Stair(_) | Entity::Roof(_) => true,
        Entity::Opening(o) => o.opening.form == OpeningForm::Door,
        _ => false,
    }
}

/// Whether `e` takes a load-bearing role: a wall that is no curtain wall (docs/adr/8090, BYKHY Ek-3/B 2), a column
/// that is no post (docs/adr/8124: the frame's column carries unless declared not to).
pub fn load_bearing_rated(e: &Entity) -> bool {
    match e {
        Entity::Wall(w) => w.wall.curtain.is_none(),
        Entity::Column(c) => c.column.member.is_none(),
        _ => false,
    }
}

/// A wall's or a column's declared load-bearing role; none: not declared (a wall's unknown, a column's the frame's).
pub fn load_bearing_of(e: &Entity) -> Option<&kentos_commands::DeclaredBearing> {
    match e {
        Entity::Wall(w) => w.wall.arch.load_bearing.as_ref(),
        Entity::Column(c) => c.column.arch.load_bearing.as_ref(),
        _ => None,
    }
}

/// Whether `e` takes a declared exterior role (docs/adr/8124, EK-6 `IsExternal`): a wall (a curtain wall too) or a slab
/// that is no covering; a door or a window is as its wall, a roof always outside.
pub fn external_rated(e: &Entity) -> bool {
    match e {
        Entity::Wall(_) => true,
        Entity::Slab(s) => s.slab.covering.is_none(),
        _ => false,
    }
}

/// Whether `e` takes a lift shaft's vent declaration (docs/adr/8135, BYKHY 62/4): a room that is a lift shaft (an
/// Asansör, not its lobby; `room_names::shaft_of`).
pub fn shaft_vent_rated(e: &Entity) -> bool {
    match e {
        Entity::Space(s) => matches!(
            kentos_domain::room_names::shaft_of(s.space.name.as_deref(), s.space.usage.as_deref()),
            Some(kentos_domain::room_names::ShaftKind::Elevator)
        ),
        _ => false,
    }
}

/// A lift shaft room's declared vent or pressurisation (docs/adr/8135); none: not declared on this room.
pub fn shaft_vent_of(e: &Entity) -> Option<&kentos_commands::DeclaredShaftVent> {
    if !shaft_vent_rated(e) {
        return None;
    }
    e.arch_info()?.shaft_vent.as_ref()
}

/// Whether `e` takes a protected shaft declaration (docs/adr/8136, BYKHY Ek-3/B 8): a void room that is a shaft (a lift's,
/// a service shaft, an air shaft, a flue; `room_names::shaft`), not a light well, which is open to the sky: the shafts
/// whose enclosure the drawing reads (`Document::shaft_enclosures`).
pub fn protected_rated(e: &Entity) -> bool {
    use kentos_domain::room_names::{ShaftKind, shaft, shaft_of};
    match e {
        Entity::Space(s) if s.space.void => {
            let (name, usage) = (s.space.name.as_deref(), s.space.usage.as_deref());
            shaft(name, usage) && shaft_of(name, usage) != Some(ShaftKind::LightWell)
        }
        _ => false,
    }
}

/// A shaft room's declared protected role (docs/adr/8136); none: not declared on this room.
pub fn protected_of(e: &Entity) -> Option<&kentos_commands::DeclaredProtected> {
    if !protected_rated(e) {
        return None;
    }
    e.arch_info()?.protected.as_ref()
}

/// Whether `e` takes a fire stop declaration (docs/adr/8136; BYKHY 4/pp: a horizontal or vertical element): a wall or a
/// floor's or a terrace's slab, no covering; Ek-3 reads these (`Document::fire_elements`).
pub fn fire_stop_rated(e: &Entity) -> bool {
    match e {
        Entity::Wall(_) => true,
        Entity::Slab(s) => s.slab.covering.is_none() && s.slab.role != kentos_commands::SlabRole::Balcony,
        _ => false,
    }
}

/// A wall's or a slab's declared fire stop role (docs/adr/8136); none: not declared.
pub fn fire_stop_of(e: &Entity) -> Option<&kentos_commands::DeclaredFireStop> {
    if !fire_stop_rated(e) {
        return None;
    }
    e.arch_info()?.fire_stop.as_ref()
}

/// A wall's or a slab's declared exterior role; none: the drawing's.
pub fn external_of(e: &Entity) -> Option<&kentos_commands::DeclaredExternal> {
    if !external_rated(e) {
        return None;
    }
    e.arch_info().and_then(|a| a.external.as_ref())
}

/// Where an object's class comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carried {
    /// Its own.
    Own,
    /// Its wall style's or its door or window type's.
    Type,
}

/// The class an object carries: its own, else its wall style's (a wall) or its type's (an opening); none: missing.
pub fn fire_rating_of<'a>(settings: &'a ProjectSettings, e: &'a Entity) -> Option<(&'a Declared, Carried)> {
    match e {
        Entity::Opening(o) => o.opening.fire_rating.as_ref().map(|d| (d, Carried::Own)).or_else(|| {
            let id = o.opening.style.as_deref()?;
            let types = settings.arch.as_ref()?;
            let t = types.opening_types.iter().find(|t| t.id == id)?;
            t.fire_rating.as_ref().map(|d| (d, Carried::Type))
        }),
        Entity::Wall(w) => w.wall.arch.fire_rating.as_ref().map(|d| (d, Carried::Own)).or_else(|| {
            let id = w.wall.style.as_deref()?;
            let s = settings.wall_styles.iter().find(|s| s.id == id)?;
            s.fire_rating.as_ref().map(|d| (d, Carried::Type))
        }),
        other => other.arch_info()?.fire_rating.as_ref().map(|d| (d, Carried::Own)),
    }
}

/// An object's exit role: a door's, a stair's or a roof's own; none: unknown.
pub fn exit_of(e: &Entity) -> Option<&kentos_commands::DeclaredExit> {
    match e {
        Entity::Opening(o) => o.opening.exit.as_ref(),
        Entity::Stair(s) => s.stair.arch.exit.as_ref(),
        Entity::Roof(_) => e.arch_info().and_then(|a| a.exit.as_ref()),
        _ => None,
    }
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// What is written.
enum Work {
    /// The objects changed.
    Objects(Vec<(Slot, Entity)>),
    /// The project's settings after (wall styles, door and window types, materials) or the building's declaration.
    Settings(Box<ProjectSettings>),
}

/// The key `key` names in a list of (id, name): its id, else its name (case aside).
fn find<'a>(ids: impl Iterator<Item = (usize, &'a str, &'a str)> + Clone, key: &str) -> Option<usize> {
    let folded = key.to_lowercase();
    ids.clone().find(|(_, id, _)| *id == key).or_else(|| ids.clone().find(|(_, _, n)| n.to_lowercase() == folded)).map(|(i, _, _)| i)
}

fn declared(d: Option<&Declared>, what: &str, path: &str) -> Result<(), Stop> {
    match d.and_then(|d| d.problem(what)) {
        Some(why) => Err(fail(codes::INVALID_DECLARED, why, path)),
        None => Ok(()),
    }
}

/// Why `e` takes no class.
fn no_class(e: &Entity) -> String {
    format!(
        "“{}” nesnesine yangın dayanım sınıfı verilmez: sınıf duvar, döşeme, kolon, kiriş, çatı, korkuluk, merdiven, rampa, kapı ve pencerenin özelliğidir.",
        e.kind()
    )
}

fn check(doc: &Document, input: &ArchPerformance) -> Result<(Work, Vec<String>), Stop> {
    let settings = doc.settings();
    let work = match &input.operation {
        PerformanceOperation::Objects {
            uids,
            fire_rating,
            exit,
            load_bearing,
            external,
            shaft_vent,
            protected,
            fire_stop,
        } => {
            checks::uids(uids, "Yangın bilgisi verilecek nesne verilmedi.")?;
            let found = checks::named(doc, uids)?;
            for (at, _, e, _) in &found {
                if fire_rating.is_some() && !fire_rated(e) {
                    return Err(fail(codes::FIELD_NOT_FOR_KIND, no_class(e), &format!("uids[{at}]")));
                }
                if exit.is_some() && !exit_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin acil çıkış rolü olmaz: acil çıkış kapının, merdivenin ve kaçış yolu olan \
                             çatının özelliğidir.",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
                if load_bearing.is_some() && !load_bearing_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin taşıyıcılık rolü verilmez: taşıyıcılık duvarın (giydirme cephe değil) ve \
                             kolonun (dikme değil) özelliğidir.",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
                if external.is_some() && !external_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin dışarıda olduğu beyan edilmez: bu beyan duvarın ve döşemenin (kaplama \
                             değil) özelliğidir; kapı ve pencere duvarı gibidir, çatı hep dışarıdadır.",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
                if shaft_vent.is_some() && !shaft_vent_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin kuyu havalandırması beyan edilmez: bu beyan asansör kuyusu olan mahalin \
                             (adı Asansör, holü değil) özelliğidir.",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
                if protected.is_some() && !protected_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin korunumlu olduğu beyan edilmez: bu beyan şaft olan boşluk mahalin (asansör \
                             kuyusu, tesisat şaftı, hava bacası; ışıklık değil) özelliğidir.",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
                if fire_stop.is_some() && !fire_stop_rated(e) {
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!(
                            "“{}” nesnesinin yangın kesici olduğu beyan edilmez: yangın kesici duvar ya da döşemedir \
                             (balkon ve kaplama değil).",
                            e.kind()
                        ),
                        &format!("uids[{at}]"),
                    ));
                }
            }
            let given = [
                fire_rating.is_some(),
                exit.is_some(),
                load_bearing.is_some(),
                external.is_some(),
                shaft_vent.is_some(),
                protected.is_some(),
                fire_stop.is_some(),
            ];
            if !given.contains(&true) {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Ne yangın dayanım sınıfı ne acil çıkış ne taşıyıcılık ne dışarıda ne kuyu havalandırması ne \
                     korunumlu şaft ne yangın kesici verildi: fireRating, exit, loadBearing, external, shaftVent, \
                     protected ya da fireStop verin (null kaldırır)."
                        .into(),
                    "operation",
                ));
            }
            if let Some(Some(d)) = fire_rating {
                declared(Some(d), "yangın dayanım sınıfı", "operation.fireRating")?;
            }
            if let Some(Some(x)) = exit
                && let Some(why) = x.basis.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, "operation.exit.basis"));
            }
            if let Some(Some(x)) = load_bearing
                && let Some(why) = x.basis.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, "operation.loadBearing.basis"));
            }
            if let Some(Some(x)) = external
                && let Some(why) = x.basis.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, "operation.external.basis"));
            }
            if let Some(Some(x)) = shaft_vent
                && let Some((why, field)) = x.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, &format!("operation.shaftVent.{field}")));
            }
            if let Some(Some(x)) = protected
                && let Some(why) = x.basis.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, "operation.protected.basis"));
            }
            if let Some(Some(x)) = fire_stop
                && let Some(why) = x.basis.problem()
            {
                return Err(fail(codes::INVALID_DECLARED, why, "operation.fireStop.basis"));
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            let layers = doc.layers();
            if let Some((at, _, e, _)) = found.iter().find(|(_, _, e, _)| layers.is_locked(&e.base().layer_id)) {
                let id = &e.base().layer_id;
                let name = layers.get(id).map_or(id.as_str(), |n| n.name.as_str());
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!("“{name}” katmanı kilitli; üzerindeki nesnenin yangın bilgisi değişmez. Kilidi Katmanlar panelinden açın."),
                    &format!("uids[{at}]"),
                ));
            }
            let mut changes = Vec::new();
            for (_, slot, e, _) in &found {
                let mut next = (*e).clone();
                match &mut next {
                    Entity::Opening(o) => {
                        if let Some(v) = fire_rating {
                            o.opening.fire_rating.clone_from(v);
                        }
                        if let Some(v) = exit {
                            o.opening.exit.clone_from(v);
                        }
                    }
                    other => {
                        if let Some(a) = other.arch_info_mut() {
                            if let Some(v) = fire_rating {
                                a.fire_rating.clone_from(v);
                            }
                            if let Some(v) = exit {
                                a.exit.clone_from(v);
                            }
                            if let Some(v) = load_bearing {
                                a.load_bearing.clone_from(v);
                            }
                            if let Some(v) = external {
                                a.external.clone_from(v);
                            }
                            if let Some(v) = shaft_vent {
                                a.shaft_vent.clone_from(v);
                            }
                            if let Some(v) = protected {
                                a.protected.clone_from(v);
                            }
                            if let Some(v) = fire_stop {
                                a.fire_stop.clone_from(v);
                            }
                        }
                    }
                }
                if next != **e {
                    changes.push((*slot, next));
                }
            }
            // The objects named, each once, in the drawing's order: each now holds what was given.
            let mut named: Vec<(u64, String)> =
                found.iter().map(|(_, slot, _, uid)| (doc.place(*slot).unwrap_or(u64::MAX), (*uid).clone())).collect();
            named.sort();
            (Work::Objects(changes), named.into_iter().map(|(_, u)| u).collect())
        }
        PerformanceOperation::WallStyle { style, fire_rating } => {
            // The project's style, else KentOS's (a project copy of the same id then keeps the class, as the Duvar
            // stilleri window keeps a changed KentOS style).
            let list = &settings.wall_styles;
            let builtin = kentos_commands::builtin_wall_styles();
            let own = find(list.iter().enumerate().map(|(i, s)| (i, s.id.as_str(), s.name.as_str())), style);
            let kentos = own
                .is_none()
                .then(|| find(builtin.iter().enumerate().map(|(i, s)| (i, s.id.as_str(), s.name.as_str())), style))
                .flatten();
            // The project's place of the style, or KentOS's to copy.
            let source: Result<usize, usize> = match (own, kentos) {
                (Some(at), _) => Ok(at),
                (None, Some(k)) => Err(k),
                (None, None) => {
                    return Err(fail(
                        codes::STYLE_NOT_FOUND,
                        format!("Projede “{style}” kimlikli ya da adlı duvar stili yok. Duvar stilleri penceresindeki bir stili verin."),
                        "operation.style",
                    ));
                }
            };
            declared(fire_rating.as_ref(), "yangın dayanım sınıfı", "operation.fireRating")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let mut next = settings.clone();
            let at = match source {
                Ok(at) => at,
                Err(k) => {
                    next.wall_styles.push(builtin[k].clone());
                    next.wall_styles.len() - 1
                }
            };
            next.wall_styles[at].fire_rating.clone_from(fire_rating);
            let id = next.wall_styles[at].id.clone();
            let carried = carriers(doc, fire_rating.is_some(), |e| {
                matches!(e, Entity::Wall(w) if w.wall.style.as_deref() == Some(id.as_str()) && w.wall.arch.fire_rating.is_none())
            });
            (Work::Settings(Box::new(next)), carried)
        }
        PerformanceOperation::OpeningType { opening_type, fire_rating } => {
            let types = settings.arch.as_ref().map_or(&[][..], |a| a.opening_types.as_slice());
            let at = find(types.iter().enumerate().map(|(i, t)| (i, t.id.as_str(), t.name.as_str())), opening_type)
                .ok_or_else(|| {
                    fail(
                        codes::OPENING_TYPE_NOT_FOUND,
                        format!("Projede “{opening_type}” kimlikli ya da adlı kapı ve pencere tipi yok. Kapı ve pencere tipleri penceresindeki bir tipi verin."),
                        "operation.type",
                    )
                })?;
            declared(fire_rating.as_ref(), "yangın dayanım sınıfı", "operation.fireRating")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let mut next = settings.clone();
            let id = types[at].id.clone();
            if let Some(a) = next.arch.as_mut() {
                a.opening_types[at].fire_rating.clone_from(fire_rating);
            }
            let carried = carriers(doc, fire_rating.is_some(), |e| {
                matches!(e, Entity::Opening(o) if o.opening.style.as_deref() == Some(id.as_str()) && o.opening.fire_rating.is_none())
            });
            (Work::Settings(Box::new(next)), carried)
        }
        PerformanceOperation::Material { material, strength, reaction, roof_class } => {
            if strength.is_none() && reaction.is_none() && roof_class.is_none() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Ne dayanım ne yanıcılık ne dış yangın sınıfı verildi: strength, reaction ya da roofClass verin (null kaldırır).".into(),
                    "operation",
                ));
            }
            let list = settings.arch.as_ref().map_or(&[][..], |a| a.materials.as_slice());
            let at = find(list.iter().enumerate().map(|(i, m)| (i, m.id.as_str(), m.name.as_str())), material)
                .ok_or_else(|| {
                    fail(
                        codes::MATERIAL_NOT_FOUND,
                        format!("Projede “{material}” kimlikli ya da adlı malzeme yok. Malzemeler penceresindeki bir malzemeyi verin."),
                        "operation.material",
                    )
                })?;
            if let Some(Some(d)) = reaction {
                declared(Some(d), "yanıcılık sınıfı", "operation.reaction")?;
                if let Some(why) = reaction_problem(&d.value) {
                    return Err(fail(codes::INVALID_DECLARED, why, "operation.reaction.value"));
                }
            }
            if let Some(Some(d)) = roof_class {
                declared(Some(d), "dış yangın sınıfı", "operation.roofClass")?;
                if let Some(why) = roof_class_problem(&d.value) {
                    return Err(fail(codes::INVALID_DECLARED, why, "operation.roofClass.value"));
                }
            }
            if let Some(why) = strength.as_ref().and_then(|s| s.as_deref()).and_then(strength_problem) {
                return Err(fail(codes::INVALID_STRENGTH, why, "operation.strength"));
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            let mut next = settings.clone();
            if let Some(a) = next.arch.as_mut() {
                if let Some(v) = strength {
                    a.materials[at].strength.clone_from(v);
                }
                if let Some(v) = reaction {
                    a.materials[at].reaction.clone_from(v);
                }
                if let Some(v) = roof_class {
                    a.materials[at].roof_class.clone_from(v);
                }
            }
            (Work::Settings(Box::new(next)), Vec::new())
        }
        PerformanceOperation::Exits { yes, no } => {
            // Acil çıkışlar (docs/adr/8124): the suggestion the author confirmed, the author's statement each.
            let all: Vec<String> = yes.iter().chain(no).cloned().collect();
            checks::uids(&all, "Acil çıkış rolü yazılacak kapı ya da merdiven verilmedi.")?;
            if let Some((at, u)) = no.iter().enumerate().find(|(_, u)| yes.contains(u)) {
                return Err(fail(
                    codes::UID_IN_BOTH_LISTS,
                    format!("“{u}” hem evet hem hayır listesinde; her kapı ya da merdiven yalnız birinde olur."),
                    &format!("operation.no[{at}]"),
                ));
            }
            let found = checks::named(doc, &all)?;
            for (at, _, e, _) in &found {
                let door_or_stair = exit_rated(e) && !matches!(e, Entity::Roof(_));
                if !door_or_stair {
                    let place = if *at < yes.len() { format!("operation.yes[{at}]") } else { format!("operation.no[{}]", at - yes.len()) };
                    return Err(fail(
                        codes::FIELD_NOT_FOR_KIND,
                        format!("“{}” nesnesinin acil çıkış rolü olmaz: acil çıkış kapının ve merdivenin özelliğidir.", e.kind()),
                        &place,
                    ));
                }
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            let layers = doc.layers();
            if let Some((at, _, e, _)) = found.iter().find(|(_, _, e, _)| layers.is_locked(&e.base().layer_id)) {
                let id = &e.base().layer_id;
                let name = layers.get(id).map_or(id.as_str(), |n| n.name.as_str());
                let place = if *at < yes.len() { format!("operation.yes[{at}]") } else { format!("operation.no[{}]", at - yes.len()) };
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!("“{name}” katmanı kilitli; üzerindeki kapının ya da merdivenin acil çıkış rolü değişmez. Kilidi Katmanlar panelinden açın."),
                    &place,
                ));
            }
            let mut changes = Vec::new();
            for (_, slot, e, uid) in &found {
                let role = Some(kentos_commands::DeclaredExit {
                    exit: yes.contains(uid),
                    basis: kentos_commands::DeclaredBasis::Author,
                });
                let mut next = (*e).clone();
                match &mut next {
                    Entity::Opening(o) => o.opening.exit = role,
                    other => {
                        if let Some(a) = other.arch_info_mut() {
                            a.exit = role;
                        }
                    }
                }
                if next != **e {
                    changes.push((*slot, next));
                }
            }
            let mut named: Vec<(u64, String)> =
                found.iter().map(|(_, slot, _, uid)| (doc.place(*slot).unwrap_or(u64::MAX), (*uid).clone())).collect();
            named.sort();
            (Work::Objects(changes), named.into_iter().map(|(_, u)| u).collect())
        }
        PerformanceOperation::Building { fire_protection_class } => {
            declared(fire_protection_class.as_ref(), "binanın yangın koruma sınıfı", "operation.fireProtectionClass")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let mut next = settings.clone();
            let arch = next.arch.get_or_insert_with(Default::default);
            let mut p = arch.performance.clone().unwrap_or_default();
            p.fire_protection_class.clone_from(fire_protection_class);
            arch.performance = (!p.is_empty()).then_some(p);
            (Work::Settings(Box::new(next)), Vec::new())
        }
    };
    Ok(work)
}

/// The objects `of` picks (a type's, without their own class), in the drawing's order, when the type is given a class;
/// none when its class is taken away.
fn carriers(doc: &Document, given: bool, of: impl Fn(&Entity) -> bool) -> Vec<String> {
    if !given {
        return Vec::new();
    }
    doc.entities_with_uids().filter(|(_, e)| of(e)).map(|(u, _)| u.to_string()).collect()
}

pub fn validate(_cx: &ExecutionContext<'_>, input: &ArchPerformance) -> CommandResult<()> {
    match check(_cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What it would write, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ArchPerformance) -> CommandResult<ArchPerformanceDone> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document.
pub fn plan_in(doc: &Document, input: &ArchPerformance) -> CommandResult<ArchPerformanceDone> {
    match check(doc, input) {
        Ok((_, uids)) => CommandResult::Completed {
            output: ArchPerformanceDone { uids, revision: doc.revision().to_string() },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchPerformance) -> CommandResult<ArchPerformanceDone> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARCH_PERFORMANCE,
        kentos_commands::CAD_ARCH_PERFORMANCE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ArchPerformance) -> CommandResult<ArchPerformanceDone> {
    let (work, uids) = match check(cx.doc, &input) {
        Ok(prepared) => prepared,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let written = match work {
        Work::Objects(changes) => {
            if !changes.is_empty() {
                doc.update_many(changes, STEP);
            }
            Ok(())
        }
        Work::Settings(next) => {
            let next = *next;
            doc.with_style_tables(STEP, |d| {
                d.transact(STEP, |d| {
                    if d.settings().wall_styles != next.wall_styles {
                        let mut s = d.settings().clone();
                        s.wall_styles = next.wall_styles.clone();
                        d.set_settings(s);
                    }
                    let arch = next.arch.clone().unwrap_or_default();
                    d.set_opening_types(arch.opening_types.clone(), STEP)?;
                    d.set_materials(arch.materials.clone(), STEP)?;
                    d.set_building_performance(arch.performance.clone(), STEP)?;
                    Ok::<(), kentos_domain::Refusal>(())
                })
            })
        }
    };
    if let Err(e) = written {
        return CommandResult::Failed { error: error(codes::INVALID_DECLARED, e.0, Some("operation".into())) };
    }
    CommandResult::Completed {
        output: ArchPerformanceDone { uids, revision: doc.revision().to_string() },
        warnings: Vec::new(),
    }
}
