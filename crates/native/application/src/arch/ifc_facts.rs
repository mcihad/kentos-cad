//! The facts of a building's IFC file as the model quality control form reads them (docs/adr/8074 §3, 8075 §2): the
//! model `cad.arch.ifc` writes, its file read back by the reading face (`kentos_ifc::read::file_facts`), with what only
//! the source knows: the objects the file is expected to hold (`expected`: persistent id, GlobalId, class;
//! `kentos_ifc::expected`), what the export left out and why it wrote no system (`export`), whether the file is one
//! written in memory or the one delivered, and where the building is in the drawing (`at`, for a finding that names no
//! object). The standards package's form (`dijital_proje.form`) decides on them; the hosts ask it (the desktop's audit,
//! table and Ruhsat takımı in memory; the sheet publication on the file it delivers).
//!
//! A drawing without walls is no building and has no facts.

use kentos_commands::Entity;
use kentos_domain::{Document, Uuid};
use kentos_ifc::arch::{Why, Written};
use kentos_ifc::building::Building;
use serde_json::{Value, json};

/// The model to write, taken from the drawing (on the window's thread where there is one); written, read back and
/// measured anywhere ([`Planned::facts`], [`Planned::facts_of`]).
pub struct Planned {
    building: Building,
    written: Written,
    at: [f64; 2],
}

/// The model `cad.arch.ifc` would write now, its header's time `stamp` and `application`: the whole project's, or one
/// of the program's buildings (`building`, docs/adr/8072 §2). None for a drawing without walls or a building the
/// program does not have.
pub fn plan(
    doc: &Document,
    building: Option<&str>,
    stamp: &str,
    application: &str,
) -> Option<Planned> {
    let at = doc.entities().find_map(|e| match e {
        Entity::Wall(w) => w.wall.pts.first().map(|p| [p.x, p.y]),
        _ => None,
    })?;
    let (building, written) = match building {
        Some(id) => {
            let (b, w, _) = crate::arch_ifc::building_of(doc, id, stamp, application)?;
            (b, w)
        }
        None => crate::arch_ifc::building(doc, stamp, application),
    };
    Some(Planned {
        building,
        written,
        at,
    })
}

impl Planned {
    /// The file's name (EK-2 Tablo 2.1, else the drawing's), with its extension.
    pub fn file_name(&self) -> String {
        format!("{}.ifc", self.building.file)
    }

    /// Where the building is in the drawing: its first wall's first point (a finding that names no object is shown
    /// there).
    pub fn at(&self) -> [f64; 2] {
        self.at
    }

    /// The file's text.
    pub fn text(&self) -> String {
        kentos_ifc::building::write(&self.building)
    }

    /// The facts of the file written in memory (not a delivered file: the form leaves the delivered version to the
    /// author, EK-9/18).
    pub fn facts(self) -> Result<Value, String> {
        let text = self.text();
        self.facts_of(text.as_bytes(), true)
    }

    /// The facts of the file's `bytes` read back (from memory, or from the folder it is delivered from), the source's
    /// objects beside them; `memory` says the file is not the one delivered.
    pub fn facts_of(&self, bytes: &[u8], memory: bool) -> Result<Value, String> {
        let b = &self.building;
        let limits = kentos_ifc::read::Limits::default();
        let mut facts = kentos_ifc::read::file_facts(bytes, &self.file_name(), &limits, &|| false).map_err(|e| {
            format!("Yazılan IFC modeli geri okunamadı: {e}. Bu KentOS'un hatasıdır; çizimi bildirin.")
        })?;
        facts["expected"] = kentos_ifc::expected::of(b)
            .iter()
            .map(|e| json!({ "uid": Uuid::from_u128(e.object).to_string(), "globalId": e.global_id, "class": e.class }))
            .collect();
        let left: Vec<Value> = self
            .written
            .left
            .iter()
            .map(|l| json!({ "uid": l.id, "what": l.what, "why": why(l.why) }))
            .collect();
        facts["export"] = json!({ "left": left, "crsWhy": b.crs_why, "northWhy": b.north_why, "memory": memory });
        facts["at"] = json!(self.at);
        Ok(facts)
    }
}

/// Why the export left an object out, as its warning says it.
fn why(w: Why) -> &'static str {
    match w {
        Why::NoBody => "kesit modeli gövdesini kuramadı",
        Why::NoWall => "duvarı yazılmadı",
        Why::NoArea => "alanı yok",
        Why::OneWay => "katında hep bir yönde; IFC'nin aks ızgarası iki yönde aks ister",
    }
}
