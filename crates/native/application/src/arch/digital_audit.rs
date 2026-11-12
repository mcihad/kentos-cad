//! Dijital proje denetimi (docs/adr/8047 §9; Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında
//! Yönetmelik, RG 5.8.2026/33331): the model quality control form's (Md. 15, EK-9 Tablo 9.1) questions the drawing
//! fails, read from the IFC model KentOS writes for it (`kentos_ifc::quality`, the same `cad.arch.ifc` writes). A
//! finding names the form's question and the article:
//!
//! - **2, Proje bilgileri (Md. 13, EK-7):** the künye's words the model's project, architect, office, address and parcel
//!   take, missing: `digital_project_info_missing`.
//! - **3, Proje ve dosya isimlendirmesi (Md. 6, EK-1):** no project code from the administration (none, or the künye's
//!   000000): `digital_project_code_missing`.
//! - **4, Mahal no ve mahal ismi (Md. 10):** rooms without a number or a name, each one: `digital_space_unnamed`.
//! - **6, Koordinat sistemi (Md. 13, EK-7 Tablo 7.6–7.7):** a drawing in local coordinates, a system that is no TUREF
//!   zone, or no ±0.00 elevation: `digital_system_wrong`.
//! - **12, Mahal:** no room in the model: `digital_no_spaces`.
//! - **21, PDF ve IFC uyumu (Md. 9):** objects of the drawing the model does not hold (no body, no wall, no area, a
//!   ramp): `digital_model_loss`, those objects its own.
//!
//! The form's other questions the model answers by how it is made, or the architect answers (`kentos_ifc::quality`).
//! Question 9 (EK-6's fire ratings, fire exits, concrete classes) is no finding: the drawing cannot hold them yet. A
//! drawing without walls is no building: nothing is said.

use kentos_commands::{Entity, codes};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::Vec2 as V;
use kentos_ifc::quality::{self, Answer};

use crate::regulation_audit::{Article, Said, Source};

const GAZETTE: &str = "5.8.2026/33331";
/// Md. 13: the project's attributes (EK-7).
pub const INFO_ARTICLE: Article = Article {
    source: Source::Dijital,
    article: "13",
    gazette: GAZETTE,
};
/// Md. 6: the files' names (EK-1, EK-2).
pub const NAMING_ARTICLE: Article = Article {
    source: Source::Dijital,
    article: "6",
    gazette: GAZETTE,
};
/// Md. 10: the rooms' numbers and names.
pub const ROOMS_ARTICLE: Article = Article {
    source: Source::Dijital,
    article: "10",
    gazette: GAZETTE,
};
/// Md. 15: the model quality control form (EK-9).
pub const FORM_ARTICLE: Article = Article {
    source: Source::Dijital,
    article: "15",
    gazette: GAZETTE,
};
/// Md. 9: the model and its sheets alike.
pub const CONSISTENCY_ARTICLE: Article = Article {
    source: Source::Dijital,
    article: "9",
    gazette: GAZETTE,
};

/// The findings, each pushed with its objects and its place.
pub(crate) fn findings(doc: &Document, push: &mut impl FnMut(Said, Vec<Slot>, V)) {
    let Some(building) = doc.entities().find_map(|e| match e {
        Entity::Wall(w) => w.wall.pts.first().map(|p| V::new(p.x, p.y)),
        _ => None,
    }) else {
        return;
    };
    let (b, written, k) = crate::arch_ifc::model(doc);
    let rows = quality::ek9(&b, &written, &k);
    let slots_of = |ids: &[String]| -> Vec<Slot> {
        ids.iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.slot_of(u))
            .collect()
    };
    let said = |code: &'static str, message: String, repair: &str, article: Article| Said {
        code,
        message,
        repair: repair.to_owned(),
        rule: article.named(),
        binding: None,
    };
    for row in &rows {
        let Answer::No(why) = &row.answer else {
            continue;
        };
        match row.no {
            2 => push(
                said(
                    codes::DIGITAL_PROJECT_INFO_MISSING,
                    format!(
                        "IFC modelinin proje bilgileri eksik: {} (EK-7; EK-9 {}) — {}",
                        quality::project_info_missing(&k).join(", "),
                        row.no,
                        INFO_ARTICLE.short()
                    ),
                    "Mimari › Proje › Ruhsat takımı'nın künyesinde doldurun: mimar ve Mimarlar Odası sicil no, büronun \
                     adı, vergi kimlik no ve adresi, il, ilçe, mahalle, ada ve parsel.",
                    INFO_ARTICLE,
                ),
                Vec::new(),
                building,
            ),
            3 => push(
                said(
                    codes::DIGITAL_PROJECT_CODE_MISSING,
                    format!(
                        "Projenin kodu yok: model ve dosyalar EK-2'ye göre adlanamıyor (EK-1; EK-9 {}) — {}",
                        row.no,
                        NAMING_ARTICLE.short()
                    ),
                    "Proje kodunu idareden alın (EK-1: başvuru yılının iki hanesi ve dört haneli sıra no) ve Ruhsat \
                     takımı'nın künyesinde Proje kodu'na yazın.",
                    NAMING_ARTICLE,
                ),
                Vec::new(),
                building,
            ),
            4 => {
                let slots = slots_of(&written.unnamed);
                let at = slots
                    .first()
                    .and_then(|s| doc.get(*s))
                    .and_then(|e| match e {
                        Entity::Space(s) => s.space.parts.first().and_then(|p| p.outer.pts.first()),
                        _ => None,
                    })
                    .map_or(building, |p| V::new(p.x, p.y));
                push(
                    said(
                        codes::DIGITAL_SPACE_UNNAMED,
                        format!(
                            "{} mahalin numarası ya da adı yok (EK-9 {}) — {}",
                            written.unnamed.len(),
                            row.no,
                            ROOMS_ARTICLE.short()
                        ),
                        "Mahalleri seçip Öznitelikler'de numarasını ve adını verin; seçili mahalin sekmesindeki Mahal numarala \
                         sırayla numaralar.",
                        ROOMS_ARTICLE,
                    ),
                    slots,
                    at,
                );
            }
            6 => push(
                said(
                    codes::DIGITAL_SYSTEM_WRONG,
                    format!(
                        "IFC modelinin koordinat sistemi eksik, {why} (EK-7 Tablo 7.6–7.7; EK-9 {}) — {}",
                        row.no,
                        INFO_ARTICLE.short()
                    ),
                    "Dosya › Proje ayarları'nda koordinat sistemi olarak binanın TUREF 3° dilimini (EPSG:5253–5259; \
                     Sivas TM33, EPSG:5255) seçin; ±0.00'ın mutlak kotunu Mimari › Parsel ve bahçe mesafeleri'nde \
                     (Vaziyet planı) verin.",
                    INFO_ARTICLE,
                ),
                Vec::new(),
                building,
            ),
            12 => push(
                said(
                    codes::DIGITAL_NO_SPACES,
                    format!(
                        "Mimari modelde mahal yok (EK-9 {}) — {}",
                        row.no,
                        FORM_ARTICLE.short()
                    ),
                    "Mimari › Mahal ile odaları mahal olarak çizin; IFC'de her biri bir IfcSpace olur.",
                    FORM_ARTICLE,
                ),
                Vec::new(),
                building,
            ),
            21 => {
                let ids: Vec<String> = written.left.iter().map(|l| l.id.clone()).collect();
                let slots = slots_of(&ids);
                push(
                    said(
                        codes::DIGITAL_MODEL_LOSS,
                        format!(
                            "Çizimin nesneleri IFC modeline girmiyor: {} (EK-9 {}) — {}",
                            quality::left_words(&written),
                            row.no,
                            CONSISTENCY_ARTICLE.short()
                        ),
                        "Gövdesi kurulamayan nesneyi (yüksekliği olmayan duvar …) düzeltin, açıklığı bir duvara bağlayın; \
                         rampa henüz IFC'ye aktarılmıyor, paftada kalır.",
                        CONSISTENCY_ARTICLE,
                    ),
                    slots,
                    building,
                );
            }
            _ => {}
        }
    }
}
