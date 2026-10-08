//! The toolbox tree (the web's `processing/categories.ts`). Tools name a
//! category; a category may sit under a parent. Empty categories are not
//! shown, so the list can name what is coming before the tools exist.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Category {
    pub id: &'static str,
    pub label: &'static str,
    pub parent: Option<&'static str>,
    pub icon: &'static str,
    /// One line under the name in the toolbox.
    pub description: &'static str,
}

const fn category(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    description: &'static str,
) -> Category {
    Category {
        id,
        label,
        parent: None,
        icon,
        description,
    }
}

pub const CATEGORIES: [Category; 9] = [
    category(
        "points",
        "Nokta işlemleri",
        "point",
        "Nokta üretme, numaralandırma ve nokta listeleri",
    ),
    category(
        "annotation",
        "Yazı ve etiket",
        "text",
        "Ölçü, uzunluk ve öznitelik yazıları",
    ),
    category(
        "attributes",
        "Öznitelik",
        "table",
        "Öznitelik hesaplama ve düzenleme",
    ),
    category(
        "cadastre",
        "Kadastro",
        "parcel",
        "Parsel, ada ve tapu işlemleri",
    ),
    category(
        "geometry",
        "Geometri",
        "polygon",
        "Sadeleştirme, tampon, onarım ve dönüşümler",
    ),
    category(
        "analysis",
        "Analiz",
        "measure",
        "Ölçüm, istatistik ve raporlar",
    ),
    category(
        "conversion",
        "Dönüştürme",
        "explode",
        "Nesne türleri arasında dönüşüm",
    ),
    category(
        "selection",
        "Seçim",
        "select",
        "Özniteliğe ve konuma göre seçim",
    ),
    // docs/adr/0207 §7: the desktop's point cloud tools.
    category(
        "pointcloud",
        "Nokta bulutu",
        "pointCloudAdd",
        "Seyreltme, zemin, sınıflama, kırpma, karolama, raster ve sınır",
    ),
];
