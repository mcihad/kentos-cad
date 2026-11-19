# ADR 8162: Planın çıkma ve bodrum notları

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15 ("plan notlarının
  öbür konuları (çatı, bodrum, çıkmalar: ADR 8084 §6)"); çatı ADR [8160](8160-plan-roof-notes.md)'ta yapıldı.
- **Bağlam:** ADR 8084 (plan notu katmanı), 8098 (planın bina yüksekliği ve kat adedi), 8152 (çıkmalar, 41/1), 8154
  (binanın kendi ±0,00'ı), 8160 (çatı notları; 69/2'nin okunuşu).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 41/1, 69/2, 69/3, 4/1-o (bodrum kat) (RG 3.7.2017/30113).
- **Sözleşme:** `PlanNoteKey`'e `projectionMost` (Çıkma, en çok; m, 0–5) ve `basementsMost` (Bodrum kat adedi, en çok;
  0–20 tam sayı). **`.kcad`:** Mimari verisi 34 (`ARCH_WITH_PLAN_MASS`); 33 diyen dosya bu notlarla reddedilir.
  `tr.kentos.mimari-mevzuat` 0.49.0 (yeni bulgu kodu `paiy_basement_count_over`). **Web:** üretilen `PlanNoteKey.ts`
  ve katalog yenilendi; web arayüzüne dokunulmadı (TODOS.md §0.2).

## §1. Sorun

Planlar çıkmayı sınırlar ya da yasaklar ("çıkma yapılamaz", "çıkma 1,00 m'yi geçemez") ve bodrum katı sayıyla bağlar
("en çok 2 bodrum kat"). İkisi de yazılamıyordu: çıkmalar 41/1'in 1,50 m'siyle denetleniyor, bodrum sayılmıyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 41/1 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin) | Çıkma yol cephesinde yapı yaklaşma sınırından, yan ve arka bahçede bahçe mesafesinden en çok 1,50 m taşar; yan ve arka sınıra 3,00 m'den, bitişik komşuya (açık çıkma) 2,00 m'den fazla yaklaşamaz | Planın çıkma notu 1,50 m'nin yerine; 3,00 m ve 2,00 m 41/1'in kalır (K417) |
| PAİY 69/2, 69/3 | Birinci, ikinci, altıncı–dokuzuncu bölümler, geçici maddeler ve 19, 20 planla değiştirilemez; idarenin yönetmeliği planda aksine hüküm yoksa uygulanır | 41 Beşinci Bölümde (Yapılara İlişkin Hükümler): plan değiştirebilir, gevşetebilir de (K417) |
| PAİY 4/1-o | Bodrum kat: zemin katın altındaki katlar | Bodrum kat adedi bina bina, binasının ±0,00'ının altındaki asma kat olmayan katlar (K418) |
| ADR 8098 | Planın bina yüksekliği ve kat adedi notları: bina bina, en çok katlı bina | Bodrum notu da: en çok bodrumlu bina (K418) |

## §3. Karar

- **K417 — Çıkma notu:** Plan notları'nda Çıkma (en çok, m; 0: çıkma yapılamaz). `cikmalar` taşmayı (ön bahçede yapı
  yaklaşma sınırından, yan, arka ve kamusal bahçede bahçe mesafesinden) notun değeriyle ölçer; sınıra 3,00 m ve
  komşuya 2,00 m 41/1'in kalır. Bulgunun kaynağı notsa iletide "Plan notu (…)", ikisi birlikteyse ikisi; notun satırı
  "Plan notu: çıkma".
- **K418 — Bodrum notu:** Plan notları'nda Bodrum kat adedi (en çok). `bina_yuksekligi` her binanın zemin katının
  altındaki katlarını (binasının ±0,00'ının altında, asma kat olmayan) sayar; en çok bodrumlu bina notu aşarsa kaldı
  (`paiy_basement_count_over`, bulgu fazla olan en alttaki katların mahalleri); bodrumsuz binada satır kapsam dışı.
- **K419 — `.kcad`:** Mimari verisi 34: çıkma ya da bodrum notu olan dosya; 33 ya da eskisini söyleyen dosya bu
  notlarla reddedilir.

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`PlanNoteKey::{ProjectionMost, BasementsMost}`, `is_mass`, doğrulama,
  `ArchSettings::has_plan_mass`), `crates/shared/kcad/src/lib.rs`, `decode.rs` (Mimari verisi 34).
- `apps/desktop/src/shell/project/plan_notes.rs` (örnekler 1,00 ve 2; açıklama).
- `crates/shared/commands/src/core/cad_settings.rs`, `catalog/gis.rs`; üretilenler.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cikmalar.py`, `bina_yuksekligi.py`, `denetim.py`.

## §5. Sınamalar

- **Kural:** çıkmalar 14 bağımsız durum (2'si yeni: 2,50 m notu ön bahçede 2,00 m taşan balkonu geçirir; 0 m notu
  1,20 m'lik balkonu notun maddesiyle kaldırır); bina yüksekliği 14 durum (3'ü yeni: iki bodrum 2'lik notla geçer, 1'lik
  notla kalır, bodrumsuz binada 0'lık not kapsam dışı).
- **Paylaşılan komut durumları** (`cad.settings.set`, 145): iki not yazılır; 1,5 bodrum reddedilir.
- **`.kcad`** (`arch_plan_notes`): iki not Mimari verisi 34'le yazılıp okunur; 33 diyen dosya reddedilir.
- **Masaüstü:** 8152'nin konutunda 2 m'lik balkon Plan notları'na Çıkma 2,50 yazılınca 41/1 ve notun satırı geçti;
  0 m'lik notla 1,20 m'lik balkon notun maddesiyle kaldı. 8098'in evi iki bodrumla (−3,80, −7,60) Bodrum kat adedi 1
  notuyla kaldı, 2 notuyla geçti.
- **Resimler:** `.run/gui-evidence/cikma-bodrum-8162/`: Plan notları'nda çıkma ve bodrum notları, açık ve koyu.

## §6. Kalanlar

- Plan notunun binası (blok blok farklı not) okunmaz — yapıldı: ADR [8163](8163-plan-notes-per-building.md).
- Planın bodrumun kullanımı (iskân, ticaret) üzerine hükümleri ayrı konu.
