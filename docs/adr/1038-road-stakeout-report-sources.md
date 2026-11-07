# 1038 — Aplikasyon raporu kaynaklarını ve zamanını söyler

Tarih: 2026-11-07. Durum: kabul (Y103). Kapsam ROAD-IO-12, ADR 0908 §9 (aplikasyon cetveli ve gönderimi), ADR 1031
(sahaya giden dosya).

## Sorun

Aplikasyon cetveli (`road.stakeout.query`) ve onun yazılan çalışma kitabı noktaların hangi tip kesitin, aksın,
boykesitin, arazinin ve kübajın hangi sürümünden hesaplandığını söylemiyordu. Ne zaman üretildikleri de yazmıyordu.

Sahaya giden bir çıktının kaynağı sonradan değişirse, eldeki çıktının eski mi yeni mi olduğu anlaşılmıyordu. ROAD-IO-12
şantiye ve aplikasyon raporunda kaynakların sürümlerinin ve üretim zamanının bulunmasını istiyor.

## Araştırma (7 Kasım 2026)

- **ADR 1031'in kaynakları** (ABD karayolu idarelerinin makine kontrolü şartnameleri): sahaya verilen tasarım
  dosyasının hangi tasarımdan üretildiği ve kontrol noktaları idarece verilir. Dosya ile tasarım arasındaki bağ
  belgelenir.
- **LandXML 1.2:** kökün `date` ve `time` öznitelikleri dosyanın üretim zamanıdır. KentOS'un string yolu bunları
  zaten yazıyor; GSI, GTS-7, CSV gibi alet biçimlerinde yer yoktur.
- **KentOS'un kaynakları:**
  - Koridor, aks, boykesit, tip kesit ve dever tanımının her biri bir revizyon taşır (ADR 0908).
  - Arazi yüzeyi son kabul edildiği kaynakların özetini (`built`) taşır.
  - Kübaj hesabının revizyonu ve onayı vardır (ADR 0950).
- **Matematik:** yok.

## Karar

- **`StakeoutQueried.sources`:** her biri bugünkü hâliyle şunlar:
  - koridor;
  - aksı ve boykesiti;
  - bölgelerinin tip kesitleri, sırayla ve birer kez;
  - dever çizelgesi;
  - arazi yüzeyi ve son kabul özeti (yoksa "kabul edilmedi");
  - koridorun kübaj hesapları, sürüm ve onaylı mı.
- **Cevapta zaman yok:** sorgu her çalışmada aynı cevabı verir. Üretim zamanı yazılan dosyanındır.
- **Kaynaklar sayfası:** `road_stakeout::sources_sheet`, cetvelin yanına bir "Kaynaklar" sayfası kurar ("Üretildi" ve
  zaman, her kaynak bir satır). Aplikasyon cetvelinin "Cetveli yaz"ı onu ana sayfanın yanına cihazın yerel saatiyle
  yazar.
- **Pencere:** Aplikasyon cetveli, açıklama satırının altında kaynakları bir satırda söyler.
- **Gönderim:** `road.stakeout.send`'in LandXML'i tarih ve saati zaten yazıyor; cevabı değişmedi. Alet biçimleri
  kaynak taşımıyor.

## Ölçüm

- **Bağımsız başvuru** `road_stakeout_cases.py` (`sources`, KentOS kodu yok): kurulumun tanımlarından.
- **`road.stakeout.query`:** 26 durumun hepsi kaynakları karşılaştırıyor. Yeni durum: ana koridorun ikinci
  sürümdeki kübajı. `road.stakeout.send` durumları değişmedi.
- **Masaüstü testi:** kaynakların koridoru ve aksı; Kaynaklar sayfasının ilk satırı "Üretildi" ve zaman; tk-2'nin
  2. sürümü; arazi "kabul edilmedi".
- **Resim** `.run/gui-evidence/yol-y103/`: "Kaynaklar: koridor c-1 (1), aks g-1 (1), boykesit b-1 (1), tip kesit
  tk-1 (1), tk-2 (2), arazi t-1 (kabul edilmedi)".
