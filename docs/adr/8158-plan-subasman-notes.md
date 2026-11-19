# ADR 8158: Planın subasman kotu hükmü — PAİY 4/1-yyy'nin aksine hükmü

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8153](8153-plinth-level-4-1-yyy.md) §6 ("İmar planının subasman hükmü (plan notunun konusu olarak)"),
  [8084](8084-plan-note-layer.md) §6 (plan notlarının öbür konuları).
- **Bağlam:** ADR 8084 (plan notu katmanı: planın değeri yönetmeliklerin önüne geçer, 69/3), 8098 (planın bina yüksekliği
  ve kat adedi notları, Mimari verisi 16), 8153 (subasman kotu, binanın kendi ±0,00'ı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-yyy (RG 3.7.2017/30113), 69/3.
- **Sözleşme:** `PlanNoteKey`'e `plinthMost` (Subasman kotu, en çok) ve `plinthLeast` (Subasman kotu, en az);
  `cad.settings.set`'in `arch.planNotes`'u onları alır (−10 ile +10 m, en az en çoğu aşmaz). **`.kcad`:** Mimari verisi
  32 (`ARCH_WITH_PLAN_PLINTH`): subasman notu taşıyan dosya `modules.arch = 32` yazar; 31 diyen reddedilir.
  `tr.kentos.mimari-mevzuat` 0.45.0. **Web:** üretilen `PlanNoteKey.ts` ve komut kataloğu yenilendi; web arayüzüne
  dokunulmadı (TODOS.md §0.2).

## §1. Sorun

4/1-yyy zemin katın taban döşemesini ±0,00 ile +1,20 arasında tutar, "İmar planlarında aksine bir hüküm bulunmaması
halinde". Planın aksine hükmü yazılamıyordu: eğimli bölgede subasmanı +2,40'a kadar bırakan planın binası 4/1-yyy'den
kalıyor, bulgunun onarımı "uzman onayıyla işaretleyin" diyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-yyy (1.7.2026'ya kadar işlenmiş birleştirilmiş metin) | Subasman kotu zemin kat taban döşemesi üst kotudur; "İmar planlarında aksine bir hüküm bulunmaması halinde, ±0.00 kotunun altına düşemez ve +1.20 kotunun üzerine çıkamaz" | Planın en çok ve en az notu iki ucun yerine geçer (K406, K407) |
| Bozüyük Belediyesi uygulama imar planı notları (5.9.2025) | 56. not: "Subasman kotunun 2.40m olarak düzenleneceği yapılarda binanın kot alacağı noktayı belirlemede belediye yetkilidir"; 62. not: Hmaks/Yençok adalarında yükseklik subasman ile saçak arasından | Planlar subasmanı +1,20'den farklı yazar: not değeri planı okuyan yazar (K406) |
| ADR 8084, 8098 | Plan notu: konusu, değeri (m), kaynağı; KentOS notun sözünü okumaz | Aynı biçim; iki yeni konu (K406) |

## §3. Karar

- **K406 — İki konu:** Plan notları'nda "Subasman kotu (en çok)" (`plinthMost`) ve "Subasman kotu (en az)"
  (`plinthLeast`): zemin katın taban döşemesinin binanın ±0,00'ından en çok ve en az yüksekliği, m; ±0,00'ın altı
  eksiyle. Değer −10 ile +10 m arasında; ikisi de yazılmışsa en az en çoğu aşamaz (alan: en azın değeri). Formun örneği
  1,50 ve −0,50; açıklaması yazılmayınca +1,20 ve ±0,00 olduğunu söyler.
- **K407 — Kural:** `subasman` planın notunu 4/1-yyy'nin ucunun yerine okur: notu aşan zemin kat kaldı (bulgunun maddesi
  plan notu, iletide "planın notu (4/1-yyy'nin aksine hükmü)"); notun satırı ("Plan notu: subasman kotu (en çok)")
  sonucunu söyler. İki uç da plandaysa 4/1-yyy kapsam dışı; biri plandaysa 4/1-yyy öbür ucu okur ve iletisi yalnız o ucu
  söyler. Notsuz projede değişmedi.
- **K408 — `.kcad`:** Mimari verisi 32: subasman notu olan dosya; 31 ya da daha eskisini söyleyen dosya bu notlarla
  reddedilir (ileti: "planın subasman kotu notları mimari verisi sürüm 32'den önce yok").

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`PlanNoteKey::{PlinthMost, PlinthLeast}`, `is_plinth`, `PLAN_PLINTH_MAX`,
  doğrulama, `ArchSettings::has_plan_plinth`).
- `crates/shared/kcad/src/lib.rs` (`ARCH_WITH_PLAN_PLINTH`, `uses_plan_plinth`, `arch_version`, `MODULE_VERSIONS`),
  `decode.rs`.
- `apps/desktop/src/shell/project/plan_notes.rs` (örnek, açıklama).
- `crates/shared/commands/src/core/cad_settings.rs`, `catalog/gis.rs` (belge); üretilenler: `PlanNoteKey.ts`,
  `commandCatalog.json`, `python/kentos/cad/types.py`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/subasman.py`.

## §5. Sınamalar

- **Kural** (`test_subasman.py`): 14 bağımsız durum (6'sı yeni: en çok +2,40 notuyla +1,50 geçer, +2,60 notun
  maddesiyle kalır; en az −0,50 notuyla −0,30 geçer, −0,80 kalır; iki notla +2,00 geçer, 4/1-yyy kapsam dışı; yalnız en
  çok notunda −0,30 4/1-yyy'nin en az ucuyla kalır).
- **Paylaşılan komut durumları** (`cad.settings.set`, 141): iki not yazılır; en azı en çoğundan yüksek reddedilir
  (`invalid_arch`, `arch.planNotes.1.value`).
- **`.kcad`** (`arch_plan_notes`): iki not Mimari verisi 32'yle yazılıp okunur; 31 diyen dosya reddedilir.
- **Masaüstü** (`plan_limits_tests`): 8098'in evi 1,50 m yükseltilince 4/1-yyy kalır ("+1.50 m, en çok +1.20 m"); Plan
  notları'na "Subasman kotu (en çok) 2,40" yazılınca olgularda `plinthMost` 2,4, 4/1-yyy ve notun satırı geçti; en azı
  1, en çoğu 0,50 notlarla Kaydet bekler, −0,50'yle kaydeder.
- **Resimler:** `.run/gui-evidence/subasman-plan-8158/`: Plan notları'nda iki subasman notu, açık ve koyu.

## §6. Kalanlar

- Plan notunun binası: plan notu bütün projenin; blok blok farklı subasman (kademeli yerleşim) okunmaz.
- Kot alınan noktanın kendisi (10–14: yoldan, tabii zeminden kotlandırma): binanın ±0,00'ı projenin yazdığıdır.
