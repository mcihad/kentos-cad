//! Konumlu fotoğraflar (docs/adr/0225 §3; the web's `model/photoImport.ts`):
//! photos whose EXIF gives a place, moved from WGS 84 into the project's
//! system the way Koordinat dönüştür moves them (docs/adr/0167, the
//! project's datum choices taken), each a point named after its file, with
//! when it was taken, the direction it looked, its height and the camera as
//! attributes, and the photo itself as its attachment (embedded or linked,
//! as the window chose). A photo without a place, or one the system does not
//! reach, is said and left out. The height is an attribute, never an
//! elevation: EXIF does not say its datum (docs/adr/0169). The shared cases
//! are fixtures/photos/v1/import.json (scripts/fixtures/photo_import_cases.py,
//! with PROJ).

use std::collections::BTreeMap;

use kentos_commands::{
    Attachment, Entity, EntityBase, LineError, PhotoExif, PointEntity, Vec2 as Wire,
};
use kentos_geometry_core::crs::{Choice, Datum, System, Unreached, transform_in};

use crate::Vec2;
use crate::format::fixed;
use crate::second::accuracy_text;

/// A photo as the window read it: its file's name, what its EXIF says and
/// where its bytes are kept (an embedded library image or a linked file).
#[derive(Clone, Debug, PartialEq)]
pub struct Photo {
    pub name: String,
    pub exif: PhotoExif,
    pub asset: Option<String>,
    pub file: Option<String>,
}

/// A photo as the import writes it, and its place in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub index: u32,
    pub name: String,
    pub p: Vec2,
    pub attrs: BTreeMap<String, String>,
    pub attachment: Attachment,
}

/// The points to write, and the photos left out with why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub placed: Vec<Placed>,
    pub skipped: Vec<LineError>,
}

/// What is said of a photo without a place; `{name}` and `{why}` are filled in.
pub const NO_PLACE: &str = "{name}: {why} Eklenmedi.";
/// What is said of a photo the project's system does not reach.
pub const SKIPPED: &str = "{name}: projenin sistemine çevrilemedi ({why}); eklenmedi.";

/// Why a place is not reached, in a few words (the GNSS import's words).
fn why(u: Unreached) -> &'static str {
    match u {
        Unreached::Outside => "sistemin ulaştığı yerin dışında",
        Unreached::NoLink => "datumlardan birinin WGS 84'e dönüşümü yok",
        Unreached::NoGrid => "datum dönüşümünün ızgarası bu cihazda yok",
        Unreached::OutsideGrid => "datum dönüşümünün ızgarasının dışında",
    }
}

/// A file's name without its last extension (`IMG_0012.JPG` → `IMG_0012`).
pub fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(k) if k > 0 => &name[..k],
        _ => name,
    }
}

/// The photos in `to` (named `system`), in their order.
pub fn place(photos: &[Photo], to: &System, system: &str, choices: &[Choice]) -> Plan {
    let wgs84 = System::Geographic {
        datum: Datum::Wgs84,
    };
    let mut plan = Plan::default();
    for (i, photo) in photos.iter().enumerate() {
        let index = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let say = |text: &str, why: &str| LineError {
            line: index,
            message: text.replace("{name}", &photo.name).replace("{why}", why),
        };
        let e = &photo.exif;
        let (Some(lat), Some(lon)) = (e.latitude, e.longitude) else {
            let why = e.problem.as_deref().unwrap_or("Fotoğrafta GPS konumu yok.");
            plan.skipped.push(say(NO_PLACE, why));
            continue;
        };
        let moved = match transform_in(&wgs84, to, Vec2::new(lon, lat), choices) {
            Ok(moved) => moved,
            Err(u) => {
                plan.skipped.push(say(SKIPPED, why(u)));
                continue;
            }
        };
        let name = stem(&photo.name).to_owned();
        let mut attrs = BTreeMap::from([("Ad".to_owned(), name.clone())]);
        let mut put = |key: &str, value: Option<String>| {
            if let Some(v) = value {
                attrs.insert(key.to_owned(), v);
            }
        };
        put("Tarih", e.taken.clone());
        put(
            "Yön",
            e.direction
                .map(|d| format!("{}{}", fixed(d, 2), if e.magnetic { " M" } else { "" })),
        );
        put("Yükseklik", e.altitude.map(|h| fixed(h, 2)));
        put("Kamera", e.camera.clone());
        put(
            "Dönüşüm",
            Some(format!("WGS 84 → {system}: {}", accuracy_text(&moved))),
        );
        plan.placed.push(Placed {
            index,
            name,
            p: moved.point,
            attrs,
            attachment: Attachment {
                name: photo.name.clone(),
                asset: photo.asset.clone(),
                file: photo.file.clone(),
                url: None,
                note: None,
            },
        });
    }
    plan
}

/// The placed photos as the import writes them: points on the source layer
/// `""`, which the window sends to its target layer (`exchange::apply`),
/// their names as labels, the photo their attachment.
pub fn entities(placed: &[Placed]) -> Vec<Entity> {
    placed
        .iter()
        .map(|pt| {
            Entity::Point(PointEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: String::new(),
                    color: None,
                    attrs: pt.attrs.clone(),
                    label: Some(pt.name.clone()),
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: vec![pt.attachment.clone()],
                },
                p: Wire {
                    x: pt.p.x,
                    y: pt.p.y,
                },
                z: None,
                parts: None,
            })
        })
        .collect()
}
