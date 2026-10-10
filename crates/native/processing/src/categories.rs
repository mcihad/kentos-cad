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

pub const CATEGORIES: [Category; 20] = [
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
        "network",
        "Ağ analizi",
        "networks",
        "En yakın tesis, maliyet matrisi ve hizmet alanları",
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
    // docs/adr/0231: the DEM's surface.
    category(
        "surface",
        "Yüzey analizi",
        "hillshade",
        "Eğim, bakı, kabartma, eğrilik, güneşlenme ve eş yükselti eğrileri",
    ),
    // docs/adr/0232: surfaces from points, densities.
    category(
        "interpolation",
        "İnterpolasyon",
        "idw",
        "Noktalardan yüzey: IDW, doğal komşu, spline, kriging, TIN",
    ),
    category(
        "density",
        "Yoğunluk",
        "kernelDensity",
        "Noktaların ve çizgilerin yoğunluğu",
    ),
    // docs/adr/0233: map algebra, masks, mosaics, statistics.
    category(
        "rasterOps",
        "Raster işlemleri",
        "rasterCalculator",
        "Hesaplayıcı, sınıflandırma, maskeyle kırpma, mozaik, yeniden örnekleme",
    ),
    category(
        "rasterStats",
        "Raster istatistiği",
        "zonalStats",
        "Bölgesel, komşuluk ve hücre istatistikleri, histogram",
    ),
    // docs/adr/0234: vectors burnt into cells, regions, lines and points out of them; scanned sheets digitized.
    category(
        "rasterVector",
        "Raster ve vektör",
        "rasterize",
        "Rasterleştirme; rasterden alan, çizgi ve nokta",
    ),
    category(
        "scannedMap",
        "Taranmış harita",
        "captureLine",
        "Çizgi yakalama, alan kapatma, eğrilere kot verme",
    ),
    // docs/adr/0235: the water's way over a DEM.
    category(
        "hydrology",
        "Hidroloji",
        "streams",
        "Çukur doldurma, akış yönü ve birikimi, havzalar, dere ağı, nemlilik indisi",
    ),
    // docs/adr/0236: how far, and how dear, every cell is from the sources.
    category(
        "distance",
        "Uzaklık ve maliyet",
        "costPath",
        "Uzaklık yüzeyi, birikimli maliyet, en düşük maliyetli yol, maliyet koridoru",
    ),
    // docs/adr/0237: criteria brought to one scale, weighed together, checked against known events.
    category(
        "suitability",
        "Uygunluk analizi",
        "weightedOverlay",
        "Bulanık üyelik ve çakıştırma, ağırlıklı toplam ve çakıştırma, AHP, ROC",
    ),
];
