# 1098 — Mesh'in hücre boyu ve sorgu cevapları çizimin plan birimiyle yazılır

Tarih: 2026-11-20. gis-usage-report.md GIS-041. Durum: kabul edildi.

İlgili: ADR 0552 (Mesh sor), ADR 0645 (çok boyutlu veri), ADR 1093 (GIS-036 birimler), ADR 1097 (GIS-040).

## Sorun

Düğümleri derece olan UGRID dosyası (quad-static.nc) WGS 84 projesine eklenirken pencere hücre boyunu “0,010 × 0,010
m” yazdı; kaynakta yatay eksenler derece, yalnız yükseklik metre. Mesh sor'un yüz alanı birimsizdi.

## Karar

1. **Mesh ekle.** Hücre boyu alanı ve önizlemedeki “Hücre boyu” çizimin plan birimini söyler: projenin kendi sistemi
   coğrafiyse “°” (dereceler yedi ondalığa kadar), değilse projenin uzunluk birimi (m, yerel projede mm/cm…). Mesh
   eklenebildiğinde dosyanın sistemi projeninkiyle aynıdır (değilse ekleme zaten reddedilir), bu yüzden plan birimi
   çizimin birimidir. Düşey veri setinin birimi (m) dosyadan geldiği gibi kalır.
2. **Mesh sor, Nokta bulutu alan/profil.** Cevaplar bir `Units` ile yazılır: coğrafi projede plan uzunlukları “°”,
   alanlar “derece²” (komut çizimin düzleminde ölçer), yükseklikler metre; projeksiyonlu projede uzunluk projenin
   biriminde, alan projenin alan biriminde (m², dönüm, ha).
3. Metre cinsinden yatay çözünürlük istenirse bu ayrı bir dönüşümdür: Mesafe ölç (ADR 1097) elipsoit üstünden ölçer;
   bu pencere değeri dönüştürmez, birimi doğru adlandırır.

## Kanıt

`apps/desktop/src/gis/rasters/tests.rs` `a_geographic_mesh_s_cells_are_in_degrees`: WGS 84 projede “Hücre boyu (°)”,
önizleme “0.01 × 0.01 °”, 21 × 21 hücre; TM projede “Hücre boyu (m)”. `area_cloud_tests`
`a_geographic_project_s_answers_are_in_degrees`: 0.0001 derece², 0.01°; projeksiyonluda m² ve m. Resim
`.run/gui-evidence/cbs-kullanim/gis-041-mesh-derece.png`.
