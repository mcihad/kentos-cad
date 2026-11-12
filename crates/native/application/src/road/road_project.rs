//! A road project's tree (docs/adr/0911 §2; ROAD-MOD-04, 06): its sources and what its commands drew, a parent
//! before its children, each with how it stands now and why. A source's state is its own query's rule (an alignment's
//! revision a profile, a sample line set, a superelevation, a corridor or a junction was made against; a corridor's a
//! guardrail, culvert or wall was; a surface's sources since it was accepted); a drawing's, its sources' stamps and its
//! objects then and now (`road_drawings`). Deep, the surfaces and corridors are made and the approved volumes and the
//! mass haul diagrams computed: what does not build, what leaves stretches unsolved and what changed since approved.

use kentos_commands::{RoadDrawingDef, RoadNode, RoadNodeKind as N, RoadNodeState as S, RoadProjectTree, RoadSourceKind as K};
use kentos_domain::Document;
use kentos_road::sources::{self, SourceObject, SurfaceState};

use crate::checks::Stop;
use crate::road_drawings;

/// What a drawing of `command` is called in the tree.
pub(crate) fn drawing_words(command: &str) -> &'static str {
    match command {
        "road.alignments.label" => "Km yazıları",
        "road.profiles.draw" => "Boykesit çizimi",
        "road.sections.draw" => "Enkesit çizimi",
        "road.corridors.draw" => "Şevli plan",
        "road.guardrails.draw" => "Korkuluk çizimi",
        "road.culverts.draw" => "Menfez çizimi",
        "road.walls.draw" => "Duvar çizimi",
        "road.junctions.draw" => "Kavşak çizimi",
        "road.terrains.contours" => "Eş yükselti eğrileri",
        "road.terrains.sheetcontours" => "Pafta eğrileri",
        "road.terrains.zeroline" => "Sıfır hattı",
        "road.terrains.cutfill" => "Kazı ve dolgu bölgeleri",
        "cad.pipes.profile" => "Hat profili",
        _ => "Yol çizimi",
    }
}

/// What a source of `kind` is called in a sentence.
fn kind_words(kind: K) -> &'static str {
    match kind {
        K::Surface => "yüzey",
        K::Alignment => "aks",
        K::Profile => "boykesit",
        K::Template => "tip kesit",
        K::Sections => "kesit takımı",
        K::Superelevation => "dever",
        K::Corridor => "koridor",
        K::Guardrail => "otokorkuluk",
        K::Culvert => "menfez",
        K::Wall => "istinat duvarı",
        K::Junction => "kavşak",
        K::Platform => "platform",
        K::Network => "ağ",
        K::Layer => "katman",
    }
}

/// A source of `kind` named before it: “Gövde” aksı, “Tip 2” tip kesiti.
fn kind_named(kind: K) -> &'static str {
    match kind {
        K::Surface => "yüzeyi",
        K::Alignment => "aksı",
        K::Profile => "boykesiti",
        K::Template => "tip kesiti",
        K::Sections => "kesit takımı",
        K::Superelevation => "deveri",
        K::Corridor => "koridoru",
        K::Guardrail => "otokorkuluğu",
        K::Culvert => "menfezi",
        K::Wall => "istinat duvarı",
        K::Junction => "kavşağı",
        K::Platform => "platformu",
        K::Network => "ağı",
        K::Layer => "katmanı",
    }
}

/// The project's drawings that are not up to date (docs/adr/1720; MAP-20: “güncel olmayan çıktı başarıyla teslim
/// edilmesin”): stale (a source changed since), missing (an object gone), edited (changed by hand) or gone (its main
/// source gone); each with its kept record, its state's word and why. A road or map project's; none elsewhere.
pub fn outdated_drawings(doc: &Document) -> Vec<(RoadDrawingDef, &'static str, Vec<String>)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_drawings()
        .iter()
        .filter_map(|d| {
            let (state, why, _) = drawing_state(doc, d);
            outdated_word(state).map(|word| (d.clone(), word, why))
        })
        .collect()
}

/// A drawing's state in the words of a sentence when it is not up to date (stale, missing, edited, gone); none else.
pub(crate) fn outdated_word(state: S) -> Option<&'static str> {
    match state {
        S::Stale => Some("güncel değil"),
        S::Missing => Some("nesnesi eksik"),
        S::Edited => Some("elle değişti"),
        S::Gone => Some("kaynağı yok"),
        _ => None,
    }
}

/// How a drawing stands now: its state, why, and how many of its objects the drawing holds.
pub(crate) fn drawing_state(doc: &Document, d: &RoadDrawingDef) -> (S, Vec<String>, usize) {
    let mut why = Vec::new();
    let main = d.sources.first();
    let gone = main.is_none_or(|m| road_drawings::stamp_now(doc, m.kind, &m.id).is_none());
    if gone {
        let what = main.map_or("kaynağı", |m| kind_words(m.kind));
        why.push(format!("Çizildiği {what} ({}) projede yok.", main.map_or("", |m| m.id.as_str())));
    }
    let mut stale = false;
    for s in d.sources.iter().skip(usize::from(gone)) {
        match road_drawings::stamp_now(doc, s.kind, &s.id) {
            Some(now) if now == s.digest => {}
            Some(_) => {
                stale = true;
                let name = road_drawings::name_of(doc, s.kind, &s.id).unwrap_or_else(|| s.id.clone());
                why.push(format!("“{name}” {} çizildiğinden beri değişti.", kind_named(s.kind)));
            }
            None => {
                stale = true;
                why.push(format!("Dayandığı {} ({}) projede yok.", kind_words(s.kind), s.id));
            }
        }
    }
    let (digest, present) = road_drawings::objects_stamp(doc, &d.uids);
    let missing = present < d.uids.len();
    if missing {
        why.push(format!("{} nesnesinden {} tanesi çizimde yok.", d.uids.len(), d.uids.len() - present));
    }
    // A record kept before docs/adr/3560 stamped its objects with their numbers in the document: it matches that way.
    let edited = !missing && digest != d.digest && road_drawings::objects_stamp_numbered(doc, &d.uids) != d.digest;
    if edited {
        why.push("Nesneleri elle değişti; yeniden üretmek bu değişiklikleri siler.".into());
    }
    let state = if gone {
        S::Gone
    } else if edited {
        S::Edited
    } else if stale {
        S::Stale
    } else if missing {
        S::Missing
    } else {
        S::Current
    };
    (state, why, present)
}

struct Tree<'a> {
    doc: &'a Document,
    deep: bool,
    nodes: Vec<RoadNode>,
}

impl Tree<'_> {
    fn push(&mut self, kind: N, id: &str, name: &str, parent: Option<u32>, state: S, why: Vec<String>) -> u32 {
        self.nodes.push(RoadNode {
            kind,
            id: id.to_owned(),
            name: name.to_owned(),
            parent,
            state,
            why,
            command: None,
            objects: None,
        });
        (self.nodes.len() - 1) as u32
    }

    /// The drawings made from the source `kind` `id`, under `parent`.
    fn drawings(&mut self, kind: K, id: &str, parent: u32) {
        let doc = self.doc;
        for d in doc.road_drawings().iter().filter(|d| d.sources.first().is_some_and(|m| m.kind == kind && m.id == id)) {
            self.drawing(d, Some(parent));
        }
    }

    fn drawing(&mut self, d: &RoadDrawingDef, parent: Option<u32>) {
        let (state, why, present) = drawing_state(self.doc, d);
        let layer = d.input.get("layerId").and_then(serde_json::Value::as_str).unwrap_or("");
        let layer_name = self.doc.layers().get(layer).map_or(layer, |l| l.name.as_str());
        let name = format!("{} · {layer_name}", drawing_words(&d.command));
        let at = self.push(N::Drawing, &d.id, &name, parent, state, why);
        let node = &mut self.nodes[at as usize];
        node.command = Some(d.command.clone());
        node.objects = Some(u32::try_from(present).unwrap_or(u32::MAX));
    }

    fn alignment_name(&self, id: &str) -> String {
        road_drawings::name_of(self.doc, K::Alignment, id).unwrap_or_else(|| id.to_owned())
    }

    fn alignment_revision(&self, id: &str) -> Option<u32> {
        self.doc.road_alignments().iter().find(|a| a.id == id).map(|a| a.revision)
    }

    fn corridor_changed(&self, corridor: &str, then: u32) -> Option<String> {
        let c = self.doc.road_corridors().iter().find(|c| c.id == corridor)?;
        (c.revision != then).then(|| format!("“{}” koridoru değişti; koridora yeniden bağlayın.", c.name))
    }

    fn surfaces(&mut self) {
        let doc = self.doc;
        let uids: Vec<(String, &kentos_commands::Entity)> = doc.entities_with_uids().map(|(u, e)| (u.to_string(), e)).collect();
        for def in crate::road_terrains::surfaces(doc) {
            let all = crate::road_terrains::surfaces(doc);
            let g = sources::gather_among(def, all, doc.layers().nodes(), uids.iter().map(|(uid, entity)| SourceObject { uid, entity }));
            let (mut state, mut why) = match g.state(def) {
                SurfaceState::Current => (S::Current, Vec::new()),
                SurfaceState::Stale => (S::Stale, vec!["Yüzeyin kaynakları kabul edildiğinden beri değişti.".to_owned()]),
                SurfaceState::New => (S::New, vec!["Yüzey henüz kabul edilmedi.".to_owned()]),
            };
            if self.deep
                && let Err(p) = crate::road_terrains::make(doc, def).surface
            {
                state = S::Unbuildable;
                why.push(format!("Yüzey kurulamıyor: {}.", p.message));
            }
            let at = self.push(N::Surface, &def.id, &def.name, None, state, why);
            self.drawings(K::Surface, &def.id, at);
            // Its pads (docs/adr/0927): graded on it; read deep, one with open slope lines is unsolved.
            for p in doc.road_platforms().iter().filter(|p| p.surface == def.id) {
                let (mut st, mut w) = (S::Current, Vec::new());
                if self.deep {
                    match crate::road_platforms::read(doc, p) {
                        Ok(r) if r.report.open > 0 => {
                            st = S::Unsolved;
                            w.push(format!("{} şev çizgisi araziyle buluşmuyor.", r.report.open));
                        }
                        Ok(_) => {}
                        Err(stop) => {
                            st = S::Unbuildable;
                            w.push(crate::road_corridors::words(stop));
                        }
                    }
                }
                let n = self.push(N::Platform, &p.id, &p.name, Some(at), st, w);
                self.drawings(K::Platform, &p.id, n);
            }
        }
    }

    fn alignments(&mut self) {
        let doc = self.doc;
        for a in doc.road_alignments() {
            let at = self.push(N::Alignment, &a.id, &a.name, None, S::Current, Vec::new());
            self.drawings(K::Alignment, &a.id, at);
            let changed = |then: u32| {
                (a.revision != then).then(|| {
                    format!("“{}” aksı değişti ({then}. revizyona göre kurulu, aks şimdi {}.); aksa yeniden bağlayın.", a.name, a.revision)
                })
            };
            for p in doc.road_profiles().iter().filter(|p| p.alignment == a.id) {
                let why: Vec<String> = changed(p.alignment_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Profile, &p.id, &p.name, Some(at), state, why);
                self.drawings(K::Profile, &p.id, n);
            }
            for s in doc.road_sections().iter().filter(|s| s.alignment == a.id) {
                let why: Vec<String> = changed(s.alignment_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Sections, &s.id, &s.name, Some(at), state, why);
                self.drawings(K::Sections, &s.id, n);
            }
            for e in doc.road_superelevations().iter().filter(|e| e.alignment == a.id) {
                let why: Vec<String> = changed(e.alignment_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                self.push(N::Superelevation, &e.id, &e.name, Some(at), state, why);
            }
            for j in doc.road_junctions().iter().filter(|j| j.legs.first().is_some_and(|l| l.alignment == a.id)) {
                let why: Vec<String> = j
                    .legs
                    .iter()
                    .filter(|l| self.alignment_revision(&l.alignment).is_some_and(|r| r != l.alignment_revision))
                    .map(|l| format!("Kolunun aksı “{}” değişti; kavşağı yeniden bağlayın.", self.alignment_name(&l.alignment)))
                    .collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Junction, &j.id, &j.name, Some(at), state, why);
                self.drawings(K::Junction, &j.id, n);
            }
        }
    }

    fn corridors(&mut self) {
        let doc = self.doc;
        for t in doc.road_templates() {
            self.push(N::Template, &t.id, &t.name, None, S::Current, Vec::new());
        }
        for c in doc.road_corridors() {
            let mut why = Vec::new();
            let mut state = S::Current;
            if crate::road_corridors::stale(doc, c) {
                state = S::Stale;
                why.push(format!("“{}” aksı değişti; koridoru aksa yeniden bağlayın.", self.alignment_name(&c.alignment)));
            }
            if self.deep {
                let r = crate::road_corridors::report(doc, c);
                if let Some(p) = &r.problem {
                    state = S::Unbuildable;
                    why.push(format!("Koridor kurulamıyor: {p}"));
                } else if !r.unsolved.is_empty() {
                    if state == S::Current {
                        state = S::Unsolved;
                    }
                    why.push(format!("{} kesitte şev araziyle buluşmuyor ya da kesit çözülmüyor.", r.unsolved.len()));
                }
            }
            let at = self.push(N::Corridor, &c.id, &c.name, None, state, why);
            self.drawings(K::Corridor, &c.id, at);
            for v in doc.road_volumes().iter().filter(|v| v.corridor == c.id) {
                let (mut state, mut why) = match &v.approved {
                    Some(a) => (S::Approved, vec![format!("{} günü onaylandı.", a.at)]),
                    None => (S::Current, Vec::new()),
                };
                if self.deep {
                    let r = crate::road_volumes::report(doc, v);
                    if let Some(p) = &r.problem {
                        state = S::Unbuildable;
                        why.push(format!("Kübaj hesaplanamıyor: {p}"));
                    } else if r.approval.as_ref().is_some_and(|a| a.changed) {
                        state = S::Changed;
                        why.push("Kaynakları onaydan sonra değişti; fark kübaj raporunda.".into());
                    }
                }
                let n = self.push(N::Volume, &v.id, &v.name, Some(at), state, why);
                for h in doc.road_hauls().iter().filter(|h| h.volume == v.id) {
                    let (mut state, mut why) = (S::Current, Vec::new());
                    if self.deep {
                        let r = crate::road_hauls::report(doc, h);
                        if let Some(p) = &r.problem {
                            state = S::Unbuildable;
                            why.push(format!("Brükner kurulamıyor: {p}"));
                        } else if r.stale {
                            state = S::Stale;
                            why.push("Kübajı değişti.".into());
                        }
                    }
                    self.push(N::Haul, &h.id, &h.name, Some(n), state, why);
                }
            }
            for g in doc.road_guardrails().iter().filter(|g| g.corridor == c.id) {
                let why: Vec<String> = self.corridor_changed(&c.id, g.corridor_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Guardrail, &g.id, &g.name, Some(at), state, why);
                self.drawings(K::Guardrail, &g.id, n);
            }
            for u in doc.road_culverts().iter().filter(|u| u.corridor == c.id) {
                let why: Vec<String> = self.corridor_changed(&c.id, u.corridor_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Culvert, &u.id, &u.name, Some(at), state, why);
                self.drawings(K::Culvert, &u.id, n);
            }
            for w in doc.road_walls().iter().filter(|w| w.corridor == c.id) {
                let why: Vec<String> = self.corridor_changed(&c.id, w.corridor_revision).into_iter().collect();
                let state = if why.is_empty() { S::Current } else { S::Stale };
                let n = self.push(N::Wall, &w.id, &w.name, Some(at), state, why);
                self.drawings(K::Wall, &w.id, n);
            }
        }
    }

    fn sheets_and_pay(&mut self) {
        let doc = self.doc;
        for s in doc.road_sheet_sets() {
            let mut why = Vec::new();
            if self.alignment_revision(&s.alignment).is_some_and(|r| r != s.alignment_revision) {
                why.push(format!("“{}” aksı paftalar yapıldığından beri değişti.", self.alignment_name(&s.alignment)));
            }
            if let Some(p) = s.profile.as_deref()
                && let Some(def) = doc.road_profiles().iter().find(|x| x.id == p)
                && def.revision != s.profile_revision
            {
                why.push(format!("“{}” boykesiti paftalar yapıldığından beri değişti.", def.name));
            }
            if let Some(k) = s.sections.as_deref()
                && let Some(def) = doc.road_sections().iter().find(|x| x.id == k)
                && def.revision != s.sections_revision
            {
                why.push(format!("“{}” kesit takımı paftalar yapıldığından beri değişti.", def.name));
            }
            let state = if why.is_empty() { S::Current } else { S::Stale };
            self.push(N::SheetSet, &s.id, &s.name, None, state, why);
        }
        if let Some(book) = doc.road_pay() {
            let (mut state, mut why) = (S::Current, Vec::new());
            if self.deep
                && let Ok((q, _)) = crate::road_pay::answer(doc, None)
                && q.rows.iter().any(|r| r.lines.iter().any(|l| l.stale))
            {
                state = S::Stale;
                why.push("Metrajın kaynaklarından biri eskidi.".into());
            }
            let name = format!("Poz defteri · {} {}", book.source, book.version);
            self.push(N::Pay, "pay", &name, None, state, why);
        }
    }

    /// The utility networks a pipe run profile was drawn from (docs/adr/3560), each with its drawings under it; a network
    /// is a source, its own state always current.
    fn networks(&mut self) {
        let doc = self.doc;
        for n in &doc.settings().networks {
            if doc.road_drawings().iter().any(|d| d.sources.first().is_some_and(|m| m.kind == K::Network && m.id == n.id)) {
                let at = self.push(N::Network, &n.id, &n.name, None, S::Current, Vec::new());
                self.drawings(K::Network, &n.id, at);
            }
        }
    }

    /// The drawings whose main source is gone: top rows at the end.
    fn orphans(&mut self) {
        let doc = self.doc;
        for d in doc.road_drawings() {
            let placed = self.nodes.iter().any(|n| n.kind == N::Drawing && n.id == d.id);
            if !placed {
                self.drawing(d, None);
            }
        }
    }
}

/// Refuses a drawing that is neither a road project's nor a map project's: a map project's tree holds its surfaces,
/// pads and what was drawn from them (docs/adr/1720).
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if !matches!(doc.settings().cad_work, Some(kentos_commands::CadWork::Road | kentos_commands::CadWork::Map)) {
        return Err(crate::road_terrains::fail(
            crate::codes::NOT_A_ROAD_PROJECT,
            "Proje ağacı yol ve harita projesinde (KentOS Yol, KentOS Harita) olur; bu çizim ikisinden biri değil.".into(),
            "",
        ));
    }
    Ok(())
}

/// The project's tree now.
pub(crate) fn tree(doc: &Document, deep: bool) -> Result<RoadProjectTree, Stop> {
    road_project(doc)?;
    let mut t = Tree { doc, deep, nodes: Vec::new() };
    t.surfaces();
    t.alignments();
    t.corridors();
    t.sheets_and_pay();
    t.networks();
    t.orphans();
    let count = |f: &dyn Fn(&RoadNode) -> bool| u32::try_from(t.nodes.iter().filter(|n| f(n)).count()).unwrap_or(u32::MAX);
    let stale = count(&|n| n.state == S::Stale);
    let problems = count(&|n| {
        matches!(n.state, S::Unbuildable | S::Unsolved | S::Changed | S::Missing | S::Edited | S::Gone)
    });
    let rebuildable = count(&|n| n.kind == N::Drawing && matches!(n.state, S::Stale | S::Missing));
    Ok(RoadProjectTree {
        nodes: t.nodes,
        stale,
        problems,
        rebuildable,
        deep,
        auto_rebuild: doc.road_auto_rebuild(),
        regional: !doc.road_rebuild_whole(),
        revision: doc.revision().to_string(),
    })
}
