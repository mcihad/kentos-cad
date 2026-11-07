# 1036 — Yüzey raporu algoritmasını söyler

Tarih: 2026-11-07. Durum: kabul (Y101). Kapsam ROAD-TIN-02, ADR 0900 (arazi yüzeyi), ADR 0933 (aks ve boykesit
raporlarının algoritma kimliği).

## Sorun

ROAD-TIN-02, Delaunay, kısıtlı Delaunay ve kulak kırpma (ear clipping) için ayrı algoritma kimlikleri ister. Sonuç
yanlış bir algoritma adıyla yayınlanmamalıdır.

- KentOS'un yüzeyi tek bir kimlikle (`kentos.cdt/1`) kuruluyordu. Kimlik kaynakların özetine giriyordu, ama raporda
  yazmıyordu.
- Aks ve boykesit raporları kendi algoritmalarını (`kentos.alignment/1`, `kentos.profile/1`) yazıyor.

## Araştırma (7 Kasım 2026)

- **Civil 3D** (ADR 1033'ün kaynağı): TIN yüzeyi noktaların Delaunay üçgenlemesidir; kırık hat kenarı Delaunay
  özelliğini bozabilir, yani kısıtlı Delaunay'dır. Tek yüzey türü tek bir üçgenleme yoluyla yapılır.
- **KentOS'un kodu:**
  - Yüzey `kentos_road::surface::build` ile kurulur: kısıtlı Delaunay, sınır ve boşluklarla kırpma, en uzun kenarla
    dış ince üçgenleri atma.
  - Kısıt yokken aynı uygulama Delaunay üçgenlemesini verir; ayrı bir Delaunay yolu yoktur.
  - Kulak kırpma `kentos_geometry_core::triangulate`'te çokgen doldurmada kullanılır (tarama ve yüz); arazi yüzeyi onu
    kullanmaz.
- **Sonuç:** üç ayrı kimlik vermek, tek bir uygulamanın sonucunu iki ayrı adla yayınlamak olurdu. Madde tam olarak
  bunu yasaklıyor.

## Karar

- **Yüzey raporunun `algorithm`'i** (`TerrainReport`), yüzey kurulunca yazılır: `kentos.cdt/1`.
  - `road.terrains.query`, `.create`, `.edit`, `.cloud` ve `file.import.landxml`'in raporları onu verir.
  - Kurulamayan yüzeyde yazılmaz: üçgen yapılmadı.
- **Kimliğin anlamı:** sürüm aynı kalırken üçgenleme değişmez; değişirse sürüm artar, kaynakların özeti de onunla
  değişir (ADR 0900).
- **Ayrı Delaunay ve kulak kırpma kimliği verilmedi:** arazi yüzeyinde ikisinin ayrı bir uygulaması yok.

## Ölçüm

- **Bağımsız başvurular** (`road_terrain_command_cases.py`, `road_cloud_terrain_cases.py`) kurulan her yüzeyin
  raporunda `kentos.cdt/1` bekler; kurulamayanda beklemez.
- **Ortak durumlar:** `road.terrains.query` (20), `.create` (28), `.edit` (21), `.cloud` (57), `file.import.landxml`
  (26). Öbür bütün yol durumları değişmedi.
