# 5300 — Datum dönüşümünün kullanım alanı çokgeni ve haritası; atlamalı trigonometrik nivelman

Tarih: 2026-11-12. plan/map-cad.md MAP-02, on birinci tur. Durum: taslak. MAP-02'nin sıradaki işi (ADR 4900'ün “Sonra”sı):
- datum dönüşümlerinin alan çokgenleri ve koordinat sistemi penceresinde harita;
- atlamalı (ortadan) trigonometrik nivelman.

İlgili:
- [ADR 4900](4900-crs-area-polygons-and-measured-refraction.md): sistemlerin EPSG çokgenleri;
- [ADR 3700](3700-coordinate-transform-point-by-point-areas.md): noktanın kendi kararı;
- [ADR 0169](0169-field-data.md) §3: karne indirgemesi, yer eğriliği ve kırılma;
- [ADR 0203](0203-network-adjustment.md) §5: kot ağı, trigonometrik ağırlık;
- [ADR 4100](4100-traverse-trigonometric-heights.md), [4500](4500-traverse-reciprocal-zenith-angles.md): poligonda
  trigonometrik kotlar.

## Dayanak

12 Kasım 2026'da okundu; not `.run/research/map-cad/uygulama/MAP-02.11.md`.

- **EPSG kayıt sitesi** (`apps.epsg.org` API'si):
  - KentOS'un dört datum işleminin (1783, 1784, 5260, 5261) kullanımı EPSG alanı **1237** “Turkey”dir (karada ve denizde,
    alan revizyonu 2026-07-21);
  - 1237'nin çokgeni (845 köşe) `area-polygons.json`'dakiyle (v13.106) köşe köşe aynıdır;
  - PROJ yalnız kutuyu tutar. Kutu Burgaz'ı (Bulgaristan) ve Halep'i içine alır, çokgen almaz.
- **BÖHHBÜY:**
  - m. 28(1)(f): poligon yükseklik farkları geometrik ya da karşılıklı trigonometrik nivelmanla; düşey açı ±10 cc ya da
    daha iyi; iki farkın farkı en çok 3 cm;
  - m. 29(1): ortometrik yükseklik, yüksekliği bilinen noktalara dayalı geometrik ya da trigonometrik nivelmanla.

  Atlamalı (ortadan) yöntem için ayrı kural ve tolerans yok.
- **Ders notları ve çalışmalar** (OMÜ 2023, KTÜN, ÇOMÜ; UNB TR 132/142; Mahun):
  - ortadan yöntemde alet iki noktanın arasındadır, alet yüksekliği düşer;
  - dengeli nişanda eğrilik ve kırılma büyük ölçüde götürülür;
  - kalan kırılma en büyük hatadır;
  - 200 m'ye dek ±2,7 mm bildirilmiş.

## Karar

1. **Datum dönüşümünün alanı çokgendir:**
   - ortak çekirdeğin işlem kaydı alanın EPSG kodunu taşır (`Operation.extent`, 1237);
   - `kentos_project::crs::operation_region` taşınan çokgeni, yoksa kutuyu verir;
   - aynı kural `cad.crs.transform`'un nokta notlarında (`outside_operation_area`), `cad.layers.reproject`'in önizlemesinde
     ve masaüstünün Koordinat dönüştür'ünde uygulanır;
   - kaydın `area.extent`'i (yeni, isteğe bağlı) hangi alanın çokgeninin okunduğunu söyler.
2. **Kullanım alanı haritası** (Proje ayarları › Koordinat sistemi, seçicinin altında):
   - Türkiye'nin alanı (1237) zemin, il merkezleri noktalarla;
   - sistemin EPSG alanı dolu (çokgeni ya da kutusu);
   - ikinci sistem başka datumdaysa datum dönüşümünün alanı kesikli;
   - altında iki satır: hangi EPSG alanı, çokgen mi kutu mu.

   Harita yalnız gösterir; karar dönüşümlerin notudur.
3. **Atlamalı trigonometrik nivelman** (`cad.survey.adjust`, kot ağı, `leapfrog`):
   - **Kurulum:** geri ve ileri nokta; her nişanın eğik uzunluğu, başucu açısı (açı birimiyle, 0 tam yukarı) ve işaret
     yüksekliği.
   - **Kot farkı:** her nişanın `S·cos z + (1 − k)·D²/2R − t`'si (D = S·sin z, projenin k'sı, R 6 371 000 m, ADR 0169
     §3); ileri nişanınkinden geri nişanınki çıkar. Alet yüksekliği düşer.
   - **Uzunluk:** iki yatay uzunluğun toplamı.
   - **Hesap:** ortak çekirdeğin `survey::fieldbook::leapfrog`'u.
   - **Dengeleme:** kurulumlar kot farklarının ardından dengelenir. Çıktının `leapfrog`'u her kurulumun farkını ve
     uzunluğunu verir.
   - **Retler:**
     - sonlu olmayan sayı `not_finite` (yolu `work.leapfrog[i].back.distance` …);
     - sıfır ya da eksi uzunluk, 0 ile yarım tur arasında olmayan başucu açısı `survey_refused` (yolu
       `work.leapfrog[i]`, hangi nişan olduğu söylenir);
     - geometrik nivelmanda kurulum `invalid_input` (`work.method`).
4. **Masaüstü** (yeni şerit düğmesi yok):
   - **Kot ağı dengelemesi:** Trigonometrik seçiliyken **Atlamalı nivelman** tablosu açılır (Geri | İleri | Geri S, z, t |
     İleri S, z, t); altında her kurulumun kot farkı. Çizime yaz komutla yazar.
   - **Proje ayarları › Koordinat sistemi:** **Kullanım alanı** haritası.
5. **Kapsam:** CAD + CBS (koordinat zinciri ortaktır, ADR 4900 gibi).

## Sınırlar

- Datum işlemlerinin alanı için yalnız 1237'nin çokgeni taşınır. Projenin kendi tanımı ve ızgaralı yollarda alan yok.
- Haritada il sınırı yok; il merkezleri yalnız yönelim içindir.
- Atlamalı yöntem için resmî tolerans yok:
  - dengelemenin model testi ve m0 söylenir;
  - m. 28'in 3 cm'si karşılıklı nivelmanındır, atlamalıya uygulanmaz.
- Kurulumun karşılıklı-eşzamanlı birleşimi (aynı kurulumda iki yönlü) ve kırılmanın kurulumdan kestirimi yok.
- Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvurular:**
  - **`cad.survey.adjust`** (`survey_adjust_command_cases.py`, mpmath 50 basamak; dengeleme başvurunun kendi
    `levelling`'i): 30'dan 41 duruma, 11 yeni:
    - iki reper arasında üç kurulum (4 mm kapanma, N1 ve N2 yazılır, geri alınır);
    - kot farklarıyla birlikte;
    - derece birimi;
    - 8 ret.
  - **`cad.crs.transform`** (`crs_vertical_cases.py`, QGIS'in pyproj'u; alan `crs_area_test.extent_of`'la): Burgaz kutuda
    ama çokgende değil, yeni durum (34). Kayıtların `area.extent`'i 1237. Zincir ve geoit durumları yeniden yazıldı.
  - **Bozma denemesi:** bir kurulumun farkı 0,1 mm ve Burgaz'ın notu değiştirilince koşucu tam o ikisini yakalar.
- **Çekirdek:** `survey::tests::a_leapfrog_setup…` (eşit nişanda k düşer, uzun nişanda kalır, retler); `crs::operation`
  testi (extent).
- **Proje:** `a_datum_operation_reads_its_extent_s_polygon` (Burgaz kutuda, çokgende değil; Ankara içeride; aynı datumda
  alan yok).
- **Python** (`python/tests/test_leapfrog_operation_area.py`): testin kendi hesabıyla kurulumun farkı ve N1'in kotu; testin
  kendi çift-tek kuralıyla Burgaz. Python ortak durumları geçer.
- **MCP** (`crates/native/mcp/tests/leapfrog_operation_area.rs`).
- **Masaüstü** (`calc::network::leapfrog_tests`):
  - tablo komutun farklarını verir, Çizime yaz kotları yazar;
  - Geometrik'te kurulumlar girmez;
  - harita 1525 ve 1237'yi okur.

  Çapraz denetimler 152 test. Resimler `.run/gui-evidence/atlamali-nivelman-5300/`.

## Sonra

- Kurulumdan kırılma kestirimi; karşılıklı-eşzamanlı ve atlamalının birleşimi.
- Haritada il sınırları (kaynak: HGM/TKGM idari sınır verisi).
- Zamana bağlı Helmert'in resmî parametreleri (sahip/kurum).
- Web (TODOS §0.2).
