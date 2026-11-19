# ADR 8164: Çıkmadan saçak — PAİY 42/3

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8161](8161-eaves-within-the-parcel.md) §6 ("42/3: çıkma varsa saçak çıkmadan en çok 0,50 m").
- **Bağlam:** ADR 8152 (çıkmalar: zemin katın üstündeki katlarda mahallerin taban alanı dışındaki parçaları), 8159
  (çatı kuralları pakette, `roof_audit::measures`), 8160 (planın saçak notu), 8161 (saçağın parsel sınırı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 42/1, 42/3, 4/1 (çıkma) (RG 3.7.2017/30113; mevzuat.gov.tr'nin
  1.7.2026'ya kadar işlenmiş birleştirilmiş metni); Sivas İmar Yönetmeliği 42/3 (RG 10.11.2022/32009; aynı hüküm).
- **Sözleşme:** yeni komut yok. `RoofMeasure.over_projection`, `Document::projection_parts` (vaziyet planından bağımsız
  çıkma parçaları); `kentos.auditFacts` 1'in `roofs[].form`'una `overProjection`. `tr.kentos.mimari-mevzuat` 0.51.0
  (yeni bulgu kodu `paiy_eaves_past_projection`). **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Saçağın genişliği yalnız duvarlarından çizilen çatıda ve duvardan ölçülüyordu (42/1, 1,00 m). Çıkması olan binada
yönetmelik saçağı çıkmanın kendisinden ölçer ve daha dar tutar; bir balkonun önünde 0,80 m taşan saçak denetimden
geçiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 42/3 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin; değişiklik işareti yok) | "Binada çıkma yapılması halinde saçak genişliği çıkmadan itibaren 0.50 metreyi aşamaz." | Çıkmanın ön yüzünden saçak çizgisine en çok 0,50 m (K424) |
| PAİY 42/1 | Planda belirlenmemişse saçak 1,00 m'yi geçmez | Planın saçak notu 42/1'in yerine geçer (8160); 42/3 çıkmanın önünü söyler, notla bağlanmaz (K425) |
| Sivas İmar Yönetmeliği 42/3 (alomaliye.com'daki metin) | Aynı cümle, 0,50 m | Değer aynı: ayrı satır yazılmaz (42/1'deki gibi) |
| ADR 8152 | Çıkma: zemin katın üstündeki bir katta mahalin taban alanının dışındaki parçası (dış kontur) | Ölçü bu parçalardan; vaziyet planı gerekmez (K424) |
| Web araması (19 Kasım 2026) | Fıkranın ölçme biçimini açıklayan idare kılavuzu bulunamadı | Okuma KentOS'undur ve kararda yazılı: yalnız ön yüz (K424) |

## §3. Karar

- **K424 — Ölçü:** çatının altındaki her çıkmanın ön yüzü: çıkmanın dışa bakan kenarı (ötesi taban alanı ya da aynı
  kattaki başka çıkma değil) ve içeri doğru gidince çıkma taban alanına varır (çıkmanın yanları taban alanı boyunca
  uzanır, ön yüz değildir). Ön yüzün iki ucundan 1 cm içeride ve ortasında, dışa dik doğrultuda saçak çizgisine (çatının
  dış hattı) uzaklık; yalnız çatının altındaki noktalar. Çatının bütün çıkmalarındaki en büyüğü `overProjection` (m);
  ön yüzü çatının altında çıkma yoksa yok. Çatının katının üstündeki kattaki çıkma onun altında değildir; katsız çatı
  bütün çıkmaları okur. Vaziyet planı gerekmez (`Document::projection_parts`).
- **K425 — Kural:** aşan çatı kaldı (`paiy_eaves_past_projection`: "Saçağın çıkmadan genişliği 0.80 m > 0.50 m —
  PAİY 42/3 (1.10.2017)"; onarım: saçağı çıkmanın ön yüzünden en çok 0,50 m taşacak biçimde daraltın). 42/3'ün satırı
  yalnız çatının altında çıkma varken yazılır. Planın saçak notu 42/3'ü bağlamaz. Düzeltme önerilmez: saçak çatının
  dış hattıdır, çatının nasıl daraltılacağı mimarındır.

## §4. Uygulama

- `crates/native/domain/src/projections.rs` (`ProjectionPart`, `projection_parts`; `projections_for` onları okur).
- `crates/native/application/src/arch/roof_audit.rs` (`over_projection`, `past_projections`, `first_hit`).
- `crates/shared/extensions/src/audit_facts/building.rs`, `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cati_olculeri.py`, `data/surumler.json` (42/3'ün 1.10.2017
  hâli).

## §5. Sınamalar

- **Birim** (`roof_audit::eaves_past_a_projection`): 10 × 8 m taban alanının güneyinde 6 m genişliğinde, 1,20 m
  derinliğinde balkon; saçak taban alanından 2,00 m güneyde: balkonun önünden 0,80 m (yanları sayılmaz); çatı ve balkon
  saat yönünde çizilince de 0,80; saçak balkonun önünde bitince yok.
- **Kural** (`test_cati_olculeri.py`): 27 bağımsız durum (2'si yeni, 42/3'ten elle: 0,80 m kalır, 0,40 m geçer).
- **Masaüstü** (`roof_rules_tests`): 8152'nin konutunda 1,20 m'lik balkonun üstündeki 1. kat çatısı y −2,00'den başlayınca
  42/3 kaldı (0,80 m), y −1,60'tan başlayınca geçti (0,40 m).
- **Resimler:** `.run/gui-evidence/cikma-sacak-8164/`: Denetim'de 42/3'ün bulgusu, açık ve koyu.

## §6. Kalanlar

- 40/5'in öbür çıkıntıları (merdiven evi, ışıklık, alın ve kalkan duvarları).
- Çıkmanın çatıya göre ayrı çatısı (çıkmanın kendi saçağı) çatının altındaki çıkma gibi okunur; ayrı bir hükmü yok.
