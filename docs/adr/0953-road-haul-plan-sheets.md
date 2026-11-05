# 0953 — Taşıma planının çalışma kitabı

Tarih: 2026-11-05. Durum: kabul (Y47). Kapsam ROAD-HAUL-12.

## Sorun

Taşıma planı (ADR 0924, 0925, 0944) yalnız pencerede okunuyordu. Hakedişe, yükleniciye ya da şantiyeye giden belge
için elle kopyalamak gerekiyordu. ROAD-HAUL-12: "CSV/XLSX/report çıktısı approved quantity/haul revisions ve aktarım
birimlerini taşısın."

## Karar

`road_hauls_plan::sheets(doc, plan)` planı ADR 0223'ün yazıcısıyla (XLSX, ODS, CSV) sayfalara döker. Taşıma planı
penceresinde "Taşıma planı [biçim] Planı yaz…" satırı vardır.

- **Özet:** Brükner ve sürümü; kübaj, sürümü ve onayı (n. onay · gün, ya da onaysız; ADR 0950); hacimlerin ne olduğu
  (kübajın düzeltilmiş hacmi: yarma gevşek, dolgu sıkışık); amaç ve durum; yarma ve dolgu fazlası, taşınan, moment,
  ortalama uzaklık, karşılanmayan ve yerleşmeyen. Her biri birimiyle: m³, m³·m, m ve para birimi. Serbest taşıma ve
  maliyet kalemleri ile uygunsuz yarma yalnız plan onları taşıyorsa yazılır.
- **Taşımalar:** her parçanın km'leri ve uzunluğu.
  - Engel varsa dolaşma ve taşıma boyu: uzunluk artı parçadaki engellerin dolaşması ([baş, son)).
  - Uygunsuz malzeme varsa uygun ve uygunsuz akış ayrı, işaretli (+ ileri).
  - Yoksa yön ve hacim.

  Moment Σ (|uygun| + |uygunsuz|) × taşıma boyudur. Net akış tek başına iki malzeme zıt yöne gittiğinde momenti eksik
  verir: ortak durumda 9,5 milyona karşı gerçek moment 12,06 milyon m³·m. Bu yüzden ikisi ayrı yazılır.
- **Alanlar:** ödünç ve döküm alanları, km, kapasite, kullanılan, uygunsuz.
- **Engeller:** km, dolaşma ya da kapalı, geçen hacim.

CSV yazıcısı yalnız ilk sayfayı (Özet) yazar (ADR 0223). Bütün plan için XLSX ya da ODS seçilmelidir.

## Ölçüm

- Pencere bekçisi `the_plan_sheets_carry_the_plan` ("Uygunsuz yarma ve engel birlikte" durumu): dört sayfa; özetin
  momenti planınki ve birimi m³·m; kübajın sürümü yazılı. Taşımalar sayfasından Σ (|uygun| + |uygunsuz|) × taşıma
  boyu, planın momentini satırların yarım milimetre ve yarım litre yuvarlamasının sınırı içinde verir (12062964 ile
  12062998,8; sınır satır satır hesaplanır). Engelin geçen hacmi planınkidir.
- Dolaşması olmayan parçada boş toplam −0 dönüyor ve sayfaya "-0.0" yazılıyordu; +0'dan katlanır.

## Açık

- PDF rapor.
- Taşıma planının onayı ve sürümü (ROAD-HAUL-08).
- CSV'de her sayfa ayrı dosya.
