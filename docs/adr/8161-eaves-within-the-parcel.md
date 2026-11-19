# ADR 8161: Saçağın parsel sınırı — PAİY 42/2

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8159](8159-roof-rules-in-the-package.md) §6 ("42/2–3 (saçak parsel sınırı, çıkmadan 0,50 m)").
- **Bağlam:** ADR 8032 (vaziyet planı: parsel, kenarlar), 8152 (çıkmaların parsel kenarlarına uzaklığı), 8159 (çatı
  kuralları pakette, `roof_audit::measures`).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 42/2 (RG 3.7.2017/30113).
- **Sözleşme:** yeni komut yok. `RoofMeasure.outside`; `kentos.auditFacts` 1'in `roofs[].form`'una `outside`.
  `tr.kentos.mimari-mevzuat` 0.48.0. **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Saçağın genişliği (42/1) ölçülüyordu, parselden taşması ölçülmüyordu: parsel sınırına yakın oturan binanın 1,00 m'lik
saçağı komşu parsele taşıyordu ve denetimden geçiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 42/2 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin; değişiklik işareti yok) | "Saçaklar parsel sınırlarını aşamaz." | Saçak çizgisinin parselin dışına taşan parçası (K415) |
| ADR 8032 | Vaziyet planının parseli, köşeleri saat yönünün tersine (`site_report_for`) | Parsel vaziyet planının parselidir; vaziyet planı yoksa ölçülmez (K415) |
| ADR 8152 | Çıkmanın parsel kenarlarına en yakın uzaklığı | Saçakta yalnız taşma: dışarıdaki parçanın sınırdan en uzak köşesi (K415) |

## §3. Karar

- **K415 — Ölçü:** vaziyet planı varken her çatının saçak çizgisinden (çatının dış hattı) parseli çıkarılır; kalan
  parçanın parsel kenarlarından en uzak köşesi taşmadır (m; parselin içindeyse 0). Vaziyet planı yoksa taşma yoktur.
- **K416 — Kural:** taşan çatı kaldı (`paiy_eaves_over_parcel`: "Saçak parsel sınırını 1.00 m aşıyor — PAİY 42/2
  (1.10.2017)"; onarım: saçağı parselin içine çekin). 42/2'nin satırı yalnız vaziyet planıyla yazılır (vaziyet planı
  olmayan projede satır yok).

## §4. Uygulama

- `crates/native/application/src/arch/roof_audit.rs` (`outside`, `past`), `crates/shared/extensions/src/audit_facts/
  building.rs`, `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cati_olculeri.py`, `data/surumler.json` (42/2'nin 1.10.2017 hâli).

## §5. Sınamalar

- **Birim** (`roof_audit`): içerideki saçak 0, batıdan 1,25 m taşan saçak 1,25 (saat yönünde çizilmiş).
- **Kural** (`test_cati_olculeri.py`): 24 bağımsız durum (2'si yeni: parselin içinde geçer, 1,25 m taşan kalır).
- **Masaüstü** (`roof_rules_tests`): 8152'nin konutunda (parsel x −3–13, y −5–11) 1. katın çatısı x −4'ten başlayınca
  parselin batı kenarını 1,00 m aşar: 42/2 kaldı; x −2'den başlayınca geçti.

## §6. Kalanlar

- 42/3: çıkma varsa saçak çıkmadan en çok 0,50 m — yapıldı: ADR [8164](8164-eaves-past-projections.md).
- 40/5'in öbür çıkıntıları (merdiven evi, ışıklık, alın ve kalkan duvarları).
