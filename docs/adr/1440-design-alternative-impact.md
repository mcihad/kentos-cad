# 1440 — Tasarım alternatifinin ölçülebilir etkisi (`cad.scenarios.impact`)

Tarih: 2026-11-06. plan/map-cad.md MAP-10 (alt maddeler: alternatifin sınırı = tasarım katmanları ve yerine geçtiği
mevcut durum; kaynaklı, birimli karşılaştırma; değişimin hangi karardan geldiği; taban değişince alternatifin
eskimesi; tek puan yerine ölçütler ve veri eksikleri). Durum: taslak. İlgili: [ADR 0210](0210-temporal-layers-and-scenarios.md)
(senaryolar), [ADR 1420](1420-zoning-plan-typed-check.md) (imar planı profili).

## Dayanak

- **ArcGIS Urban:** plan panosunun ölçüleri bina alanlarından ve yüzeylerden hesaplanır. Yeni ölçülerde alan ölçümü,
  parsel emsali (FAR) ve bunlardan türetilen ölçüler vardır. Senaryolar aynı ölçülerle yan yana konur; ölçüler tabloya
  aktarılabilir ([Work with the dashboard](https://doc.arcgis.com/en/urban/latest/help/help-metrics-plan.htm),
  6 Kasım 2026'da okundu). Esri'nin örneğinde senaryolar parsel kaplamasına ve yeşil alan şartına göre
  karşılaştırılır; tek bir “en iyi” puanı yoktur.
- **KentOS bugün:**
  - Senaryo bir gruptur: katmanları ana katmanların yerine geçer (`replaces`, ADR 0210 §9).
  - Senaryo karşılaştırma bir katman çiftini Veri karşılaştırma penceresinde açar.
  - Alternatifin bütün ölçüsü, kararların izi ve tabanın eskimesi yoktur.
  - Harita yüzeyi senaryo göstermez (K21).

## Karar

1. **`cad.scenarios.impact` v1, salt okunur (Harita + CBS).**
   - **Kapsam:** senaryonun çiftleri (`scenario_pairs`). Alternatifin sınırı yalnız tasarım katmanları ve yerine
     geçtikleri mevcut durumdur; GIS'in zaman arayüzü Harita'ya taşınmaz.
   - **Ölçüler (birim ve yöntemle):** nesne sayısı (adet), alanların toplamı (m², düzlemsel, koordinatlardan), çizgi
     uzunluğu (m), inşaat alanı (m²: yapılaşan alanların alan × KAKS'ı; park, yeşil alan, meydan ve yol taşımaz).
   - **Kullanımlar:** kullanıma göre alanlar; alan adı ilk değer olarak `Kullanim`.
2. **Değişimin kaynağı:** kararlar anahtarlarıyla (ilk değer `KararKimlik`) eşlenir. Değişim `added`, `removed`,
   `geometry`, `decision` ya da `both` olur ve önce/sonra kullanım, alan ve inşaat alanıyla verilir. Anahtarı olmayan
   alan veri eksiği olarak bildirilir: değişimi izlenemez.
3. **Taban damgası ve eskime:**
   - **Damga:** mevcut durum katmanlarının nesnelerinin SHA-256 özeti (kimlik, katman, tür, öznitelik, köşelerin
     bitleri), ilk 8 baytı.
   - **Eskime:** önceki karşılaştırmanın damgası `since` ile verilince `stale` taban o zamandan beri değişti mi
     söyler. Alternatif kendiliğinden yeni veriye uymuş sayılmaz.
   - **Saklama:** pencere damgayı Raporu kaydet'te bu oturum için saklar. Damganın çizimde (senaryonun bilgisinde)
     saklanması `.kcad` alanı ister; sonraki iştir.
4. **Tek puan yok:** ölçüler yan yana verilir. Veri eksikleri (KAKS'ı, kullanımı ya da anahtarı olmayan alanlar) ve
   varsayımlar cevaptadır, böylece eksik veriyle kesin sonuç verilmez. Tablo, CSV ve rapor aynı sayıları kullanır.
5. **Pencere Alternatif etkisi…** (`scenario.impact`, ikon `scenarioImpact`; 7 Kasım 2026'dan beri adı **Senaryo ölçüleri…**,
   bkz. [ADR 2040](2040-impact-impervious-and-access.md) “Ad”):
   - **Yeri:** Harita'da Plan › İmar planı (Senaryo oluştur ile birlikte), CBS'de Analiz › Zaman ve senaryo. Yol ve
     Mimari sunmaz.
   - **İçerik:**
     - alternatif seçimi;
     - özet: katmanlar, damga ya da eskime, veri eksikleri, varsayımlar;
     - görünüm seçimi (Ölçüler, Kullanımlar, Değişen kararlar) ve kitin veri tablosu.
   - **Raporu kaydet:** CSV yazar ve damgayı saklar.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/scenarios_impact_cases.py` 21 durum üretir. Ölçüler rasyonel shoelace'le,
  eşleme ve damga elle (SHA-256, köşe bitleri) hesaplanır. Kabul: Alternatif A'da ticaret+konut konuta döner, park
  küçülür, yerine yeşil alan gelir, sanayi kalkar, yol eklenir. Alan −900 m², inşaat alanı −1375 m², uzunluk +65 m,
  kararlar `decision`, `geometry`, `removed`, `added`. Durumlar ayrıca şunları kapsar:
  - aynı tabanda damga güncel, taban değişince `stale`;
  - başka alan ve anahtar, boş adların ilk değere dönmesi;
  - aynı, boş ve tabansız alternatif;
  - virgüllü ve yazı KAKS, yalnız geometri, geometri ve karar birlikte, yinelenen anahtar, açık kullanımlar, retler.

  Rust ve Python geçer.
- **Masaüstü:** `impact_window_tests.rs` ölçüleri ve kararları, CSV'yi ve damgayı sınar. Taban nesnesi değişince
  `stale` olduğunu, çizime yazılmadığını ve modül işaretlerini de doğrular. Resimler
  `.run/gui-evidence/alternatif-etkisi-1440/`.

## Sonra

- Damganın çizimde saklanması (`ScenarioInfo`'ya taban damgası, `.kcad` şeması).
- Yaya erişimi (ağ analizi), arazi eğimi (yüzey), altyapıya yakınlık, geçirimsizlik ve risk katmanları: her biri
  kaynağı, birimi ve tarihiyle.
- Yerleşim ve yükseklik zarfı (plan/3d.md); karşılaştırmanın haritada gösterimi ve pafta raporu.
