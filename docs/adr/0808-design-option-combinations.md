# ADR 0808: Tasarım seçeneklerinin bileşimleri

- **Durum:** kabul edildi (2026-11-01).
- **İş:** plan/architecture-advanced.md §3 `ADV-05`'in “Açık” satırındaki seçenek bileşimi; ADR 0788'in üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 154 (opt-in; §5). **Sözleşme:** `ArchSettings.optionCombos`, `ViewRecipe.combo`,
  `cad.options.edit`'in üç işlemi, `cad.options.query`'nin `combos`'u.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0788 tasarım seçeneklerini getirdi: bir kümenin seçeneklerinden biri birincildir, görünüş tarifi küme küme başka
seçeneği gösterebilir. İki ya da daha çok küme olunca (mutfak A/B, banyo A/B) bir teklif kümelerin birer seçeneğidir
ve teklifler arasında geçmek her kümede ayrı ayrı birincil seçmektir; aynı teklifi gösteren her görünümün tarifinde
seçimler tek tek yinelenir.

Archicad bunu Design Option Combinations ile çözer: her kümenin bir seçeneğinden oluşan adlı bileşim, görünümler
bileşimi gösterir, Design Options Manager'da bileşim etkin yapılır. Revit'te tasarım seçenekleri görünüm başına küme
küme seçilir; bileşim kavramı yoktur, aynı seçim her görünümde yinelenir. KentOS Archicad'in modelini alır, seçimi
saklanmış veriden değil komutla ve görünümün tarifinden yürütür.

## Karar

1. **Bileşim.** Projenin `arch.optionCombos`'u: en çok 50 bileşim; her biri kimlik (küçük harf, rakam, tire; 1–64),
   ad (boş olmayan, en çok 120 karakter, büyük küçük harften bağımsız tek) ve seçimler (`{set, option}`, her küme bir
   kez, küme ve seçenek projenin). Bileşimin anmadığı küme birincilini gösterir (`combo_choices`).
2. **Komutlar.** `cad.options.edit`'e üç işlem, hepsi tek geri alma adımı “Tasarım seçeneği”:
   - `saveCombo` (Bileşimi kaydet): `name`, isteğe bağlı `id` ve `choices` (küme ve seçenek kimlik ya da adla).
     Seçimsiz kayıt kümelerin şimdiki birincillerini tutar. Aynı adda (ya da verilen kimlikte) bileşim varsa seçimleri
     (ve adı) değişir; aynısı `nothing_to_set`. Yeni bileşimin kimliği addan, küme kimliğinin kuralıyla (harfi kalmayan
     ad “bilesim”), alınmışsa numaralanır. Kümesi olmayan proje `option_set_not_found`; bilinmeyen küme ya da seçenek
     yoluyla (`operation.choices[i].set`, `.option`), iki kez seçilen küme `invalid_input`.
   - `applyCombo` (Bileşimi uygula): bileşimin seçenekleri kümelerinin birincili olur; hepsi zaten birincilse
     `nothing_to_set`.
   - `deleteCombo` (Bileşimi sil): bir görünümün tarifi gösteriyorsa `option_combo_in_use` (görünümler adlarıyla).
   Bilinmeyen bileşim `option_combo_not_found`. Kabul et ya da Seçeneği sil kalkan küme ya da seçeneğin seçimini
   bileşimlerden düşürür ve `option_combo_dropped` uyarısıyla adlarını söyler; o kümelerde birincil gösterilir.
3. **Görünüş tarifi.** Tarifin `combo`'su bileşimin kimliği: tarifin kendi seçmediği kümelerde bileşimin seçimleri
   gösterilir (`ProjectSettings::drawn_views`: pafta, başsız kitap ve PDF aynı yoldan). Olmayan bileşim
   `option_combo_not_found` (`edit.recipe.combo`), kimlik olmayan değer `invalid_recipe`. Görünüş şablonu bileşim
   taşımaz; şablona bağlı tarif kendi bileşimini tutar (seçenekler gibi görünümündür).
4. **Okuma.** `cad.options.query`'nin `combos`'u, sorunun kümesi ya da nesnesinden bağımsız bütün bileşimler: seçimleri
   her küme için (kendi seçimi ya da birincil, projenin sırasıyla), şimdiki plan olup olmadığı (her seçim kümesinin
   birincili) ve gösterdiği planın nesne sayısı, mahal alanı ve duvar boyu.
5. **Şema 154.** Bileşimi ya da bileşimli tarifi olan proje şema 154 ister; önceki şemalarda `unknown_field`. Yazıcı
   154'ü yalnız biri varken seçer. Örnek `option-combos.kcad` (mutfak ve banyo kümeleri, iki teklif, Teklif 1'i
   gösteren görünüm), önceki şema `broken/option-combos-in-schema-153.kcad`, bozuklar
   `broken/option-combos-option-unknown.kcad` ve `broken/option-combos-set-twice.kcad`; aralık dışı
   `broken/schema-version-155.kcad`.
6. **Masaüstü.** Tasarım seçenekleri penceresinde Bileşimler: bileşimler seçimleri, şimdiki olup olmadığı, nesne
   sayısı ve mahal alanıyla; ad alanı, Birincillerden kaydet, Bileşimi uygula (şimdiki olmayanda), Bileşimi sil.
   Görünümler'de tarifin Bileşim seçicisi (Yok ya da projenin bileşimleri) ve tarifin özetinde bileşimin adı.

## Doğrulama

- Bağımsız başvuru KentOS kodu olmadan: `scripts/fixtures/option_combo_cases.py` (option_cases.py'nin çizimi ve
  kurallarıyla) `cad.options.edit.combos.json` 33 durum (görünüş tarifi adımlarıyla) ve
  `cad.options.query.combos.json` 21 durum yazar. Masaüstünde `KENTOS_CASES_ONLY=cad.options,cad.views` 238 ortak
  durum, yetenek testi, KCAD'in 234 testi ve bağımsız Python okuyucusu geçer.
- Başsız: `design_options::proposals_combine_the_sets_and_a_view_draws_one` (iki küme, iki teklif, tarifle haritanın
  gösterdiği, uygulama ve geri alma, kullanılan bileşimin silinmemesi, KCAD, kabulde düşen seçim).
- Masaüstü: `design_option_scenes::the_window_saves_applies_and_deletes_combinations`; resimler (iki tema, 1440×900 ve
  1100×650) `KENTOS_SHOTS_ONLY=bilesim-pencere,bilesim-uygulandi,bilesim-tarif cargo test -p kentos-desktop --lib
  tools_screens::screens -- --ignored --nocapture`.
- Yolda bulunan: ADR 0799'un `cad.elevations.edit`'i `revision_conflict`'i anıyor ama hiçbir durum vermiyordu
  (yetenek testi düşüyordu); `room_elevation_cases.py`'ye durum eklendi.
- Gerçek pencerede elle denenmedi.

## Sonuçlar

- Açık: bileşimlerin karşılaştırması (ADR 0809 kapattı), seçenekler arası eşleme ve birleştirme.
- Web'e gerekenler (TODOS.md §0.2): şema 154'ün okunması, bileşimlerin komutları, tarifin bileşimi.
