# ADR 8160: Planın çatı notları — mahya, çatı eğimi, saçak

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8159](8159-roof-rules-in-the-package.md) §6 ("Planın çatı notları"), [8084](8084-plan-note-layer.md) §6.
- **Bağlam:** ADR 8084 (plan notu katmanı), 8098 ve 8158 (plan notunun konuları, Mimari verisi 16 ve 32), 8159 (çatı
  kuralları pakette: `cati_olculeri`).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 40/2, 40/4, 42/1, 69/2, 69/3 (RG 3.7.2017/30113); Sivas İmar
  Yönetmeliği 40/5-g (RG 10.11.2022/32009).
- **Sözleşme:** `PlanNoteKey`'e `ridgeMost` (Mahya yüksekliği, en çok; m, 0–20), `roofSlopeMost` (Çatı eğimi, en çok;
  %, 0–200) ve `eavesMost` (Saçak genişliği, en çok; m, 0–5). **`.kcad`:** Mimari verisi 33 (`ARCH_WITH_PLAN_ROOF`);
  32 diyen dosya bu notlarla reddedilir. `tr.kentos.mimari-mevzuat` 0.47.0 (yeni bulgu kodu `paiy_roof_slope_steep`).
  **Web:** üretilen `PlanNoteKey.ts` ve katalog yenilendi; web arayüzüne dokunulmadı (TODOS.md §0.2).

## §1. Sorun

Çatı kuralları pakete taşınınca (ADR 8159) planın çatı hükümleri okunabilir oldu, ama yazılamıyordu: "çatı eğimi %33'ü
geçemez", "saçak 1,20 m" diyen planın binası PAİY'nin ve Sivas'ın değerleriyle denetleniyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 42/1 | "Uygulama imar planında belirlenmemiş ise 1.00 metreyi geçmeyen saçak yapılabilir" | Planın saçak notu 1,00 m'nin yerine (K412) |
| PAİY 40/2 | Çatı eğimleri malzeme, yöre ve iklimle "ilgili idarenin tasvibi ile tayin edilir" | PAİY eğim sınırı koymaz; planın eğim notu Sivas'sız da okunur (K413) |
| PAİY 40/4 | Mahya yüksekliği 5,00 m'yi geçmez | Planın mahya notu 5,00 m'nin yerine (K412) |
| PAİY 69/2 | Birinci, ikinci, altıncı–dokuzuncu bölümler, geçici maddeler ve 19, 20 planla değiştirilemez | 40 ve 42 Beşinci Bölümde (Yapılara İlişkin Hükümler): plan değiştirebilir, gevşetebilir de (K412) |
| PAİY 69/3 | İdarenin imar yönetmeliği planda aksine hüküm yoksa uygulanır | Planın notu Sivas 40/5-g'nin de önünde (K412) |

## §3. Karar

- **K412 — Üç konu:** Plan notları'nda Mahya yüksekliği (en çok, m), Çatı eğimi (en çok, %), Saçak genişliği (en çok,
  m); formun örnekleri 3,50, 33, 0,80; açıklama yazılmayınca 5,00 m ve 1,00 m olduğunu söyler. Not yönetmeliklerin
  değerinin önüne geçer (sıkı ya da gevşek): `katman.bulgu`'nun plan notu yolu; yönetmeliklerin değeri farklıysa
  iletide yanında yazılır.
- **K413 — Satırlar ve eğim:** notun satırı ("Plan notu: mahya yüksekliği", "… çatı eğimi", "… saçak genişliği")
  sonucunu söyler; notun bağladığı ölçünün maddesi kapsam dışıdır (40/4, 42/1; Sivas 40/5-g mahya da eğim de plandaysa).
  Eğim notu Sivas seçili değilse de okunur: aşan kenar `paiy_roof_slope_steep`, düzeltmesi dik kenarları notun
  sınırına indirir ("Eğimi planın %33 sınırına indir").
- **K414 — `.kcad`:** Mimari verisi 33: çatı notu olan dosya; 32 ya da eskisini söyleyen dosya bu notlarla reddedilir.

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`PlanNoteKey::{RidgeMost, RoofSlopeMost, EavesMost}`, `is_roof`,
  `PLAN_ROOF_SLOPE_MAX`, doğrulama, `ArchSettings::has_plan_roof`).
- `crates/shared/kcad/src/lib.rs`, `decode.rs` (Mimari verisi 33).
- `apps/desktop/src/shell/project/plan_notes.rs` (örnekler, açıklama).
- `crates/shared/commands/src/core/cad_settings.rs`, `catalog/gis.rs`; üretilenler (`PlanNoteKey.ts`,
  `commandCatalog.json`, `python/kentos/cad`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cati_olculeri.py`, `denetim.py`.

## §5. Sınamalar

- **Kural** (`test_cati_olculeri.py`): 22 bağımsız durum (6'sı yeni: mahya notu 3,50 m 3,80'i kaldırır, 6,00 m 5,20'yi
  geçirir; eğim notu %33 düzeltmesiyle, Sivas'ta %45 notu %42'yi geçirir; saçak notu 1,20 m geçirir, 0,80 m kaldırır).
- **Paylaşılan komut durumları** (`cad.settings.set`, 143): üç not yazılır; 6 m'lik saçak notu reddedilir.
- **`.kcad`** (`arch_plan_notes`): üç not Mimari verisi 33'le yazılıp okunur; 32 diyen dosya reddedilir.
- **Masaüstü** (`roof_rules_tests`): 8159'un çatısında Plan notları'na saçak 1,20 ve eğim %33 yazılınca saçak bulgusu
  gider (notun satırı geçti, 42/1 kapsam dışı), eğim notun maddesiyle kalır; Düzelt bütün kenarları %33'e indirir.
- **Resimler:** `.run/gui-evidence/cati-notlari-8160/`: Plan notları'nda üç çatı notu, açık ve koyu.

## §6. Kalanlar

- Plan notunun binası (blok blok farklı çatı hükmü) okunmaz — yapıldı: ADR [8163](8163-plan-notes-per-building.md).
- 40/2'nin idarenin tasvibiyle belirlenen eğimi plan notu değilse yazılmaz.
