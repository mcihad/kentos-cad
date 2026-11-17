# ADR 8115: IFC model eşlemesi pencerenin iş parçacığından çıkar; büyük modelde süresi ve bütçesi

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-08'in kalanı ("model eşlemesinin
  bütçesi"), [bim-plan.md](../../plan/bim-plan.md) B13 ve §7'nin "Büyük modelde (100 daire) dışa aktarma süresi ve belleği"
  satırı ("Bütçe sıradaki iş; eşleme pencerenin iş parçacığından çıkar").
- **Bağlam:** ADR [8074](8074-ek9-from-the-written-file.md) §3 (Denetim'in EK-9 formu: modelin eşlemesi pencerenin iş
  parçacığında, dosyanın yazılıp okunması ve kural öbüründe) ve §6 (100 dairede taban: eşleme 3,1 s); ADR
  [8054](8054-phase-1-memory-and-build-bottlenecks.md) §1 (okuma kopyası).
- **Web, `.kcad`, komut, kural paketi:** değişmedi.

## §1. Ölçüm

Ölçüm testi `crates/native/headless/tests/headless/ifc_timing.rs` (release, Apple M5); ham sayılar ve ortam
[docs/perf/ifc-model-2026-11-18.md](../perf/ifc-model-2026-11-18.md). Teste iki sütun eklendi: çizimin okuma kopyası
(Denetim istenince pencerenin ödediği) ve kesit modeli (eşlemenin ilk adımı).

| 100 daire (4 202 nesne, 500 mahal tek katta) | Önce | Sonra |
|---|---|---|
| Okuma kopyası | 0,08 ms | 0,08 ms |
| Kesit modeli | 8,5 ms | 9,0 ms |
| Model eşlemesi (`arch_ifc::building`) | 3 077,5 ms | 216,0 ms |
| 20 dairede model eşlemesi | 84,5 ms | 31,7 ms |

Profil (`sample`): eşlemenin süresinin büyük çoğunluğu emsal ölçümündeydi (`Document::zoning_rooms` → `zoning_measured`):
her mahalin brüt konturu kattaki öbür bütün mahallerin eksen konturlarının birleşimiyle kesiliyordu. 500 mahalde her biri
için 499 mahalin birleşimi kuruluyordu, iş mahal sayısıyla karesel büyüyordu (20'den 100 daireye 36 kat). Aynı ölçüm
Emsal ve TAKS tablosunu, Metrekare cetvelini ve denetim olgularını da besler.

## §2. Karar

- **K219 — Komşular yeter:** bir mahalin brüt konturu yalnız eksen konturunun kutusu kendi brüt konturunun kutusuna değen
  mahallerle kesilir. Kutusu değmeyen kontur konturla kesişemez: sonuç aynıdır (sınamalar alanları yeniden yazmadan geçti),
  iş komşu sayısıyla orantılıdır.
- **K220 — Pencere beklemez:** Denetim'in standart paket çağrısı ve kalite formu çizimin okuma kopyasını (nesneler paylaşılır,
  geri alma geçmişi yok; ADR 8054 §1) alır. CAD ve bina olguları, model eşlemesi, dosyanın yazılıp okunması ve kurallar
  kuralın iş parçacığındadır. Pencerenin ödediği okuma kopyasıdır (100 dairede 0,08 ms).
- **K221 — Bütçe** (bu makinede, release): okuma kopyası ≤ 1 ms; model eşlemesi 20 dairede ≤ 50 ms, 100 dairede ≤ 300 ms;
  Denetim'in Dijital proje cevabı (eşleme, yazma, geri okuma ve olgular, form) 100 dairede ≤ 1 s (bugün ~0,63 s). Ölçüm
  testi bütçeyi aşan değişikliği gösterir; zamanlamayı sıradan sınamaya koymak güvenilmez olduğu için kapı değil.

## §3. Uygulama

- `crates/native/domain/src/zoning.rs` (`zoning_measured`: eksen konturlarının kutuları bir kez, kesen komşular kutuyla).
- `apps/desktop/src/arch/arch_audit/standards.rs` (girdi okuma kopyası; olgular ve eşleme `answer`'da),
  `apps/desktop/src/cad/tables/mod.rs` (kalite formu aynı biçimde).
- `crates/native/headless/tests/headless/ifc_timing.rs` (okuma kopyası ve kesit modeli sütunları).

## §4. Sınamalar

- Olguların sınama dosyası değişmeden geçti (`kentos-processing` `audit_facts`, 20); alanın emsal sınaması; uygulamanın emsal,
  alan ve IFC sınamaları (44); masaüstünün emsal, taş duvar, alan tablosu, merdiven altı, kat bağı, kalite formu, ruhsat
  kapısı ve IFC sınamaları (50) ve Denetim'in yolu (kullanım sınıfı, hastane, otel odası, kazan dairesi, tarih, mevzuat;
  23).
- Ölçüm testi önce ve sonra (§1).

## §5. Sıradaki adımlar

- Kalan 216 ms'nin dağılımı (kısa koşuda örnekleme tutmadı); Mimari denetimin Rust kurallarının (yangın, mevzuat) pencerede
  koşması.
- IFC-08'in öbür kalanları: son yazılan dosyanın denetimi ve bulut işi.
