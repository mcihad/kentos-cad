# ADR 0809: İki bileşimin karşılaştırması

- **Durum:** kabul edildi (2026-11-01).
- **İş:** plan/architecture-advanced.md §3 `ADV-05`'in “Açık” satırındaki bileşimlerin karşılaştırması; ADR 0808'in
  üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** değişmez. **Sözleşme:** `OptionsQuery.compareCombos` (`OptionsCompareCombos`),
  `OptionsQueried.combosCompared` (`CombosCompared`).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0788 bir kümenin iki seçeneğini karşılaştırır (Veri karşılaştır'ın konum kuralıyla aynı, değişen, eklenen,
kaldırılan, alan farkı). ADR 0808 kümelerin birer seçeneğinden adlı bileşimler (teklifler) getirdi; iki teklifi
karşılaştırmak için kullanıcı ayrımı olan her kümeyi tek tek bulup seçeneklerini karşılaştırıyordu, planın toplamını
da elle çıkarıyordu.

Archicad'de Design Option Combinations görünümler arasında geçilerek gözle karşılaştırılır; Revit'te de karşılaştırma
iki görünümün yan yana konmasıdır. İkisinde de iki teklifin farkını sayan bir rapor yoktur. KentOS'ta seçeneklerin
karşılaştırması zaten komuttur; iki bileşim onu kümeler üstünde yürütür.

## Karar

1. **Komut.** `cad.options.query`'nin `compareCombos`'u: `a` (karşılaştırılan) ve `b` bileşim, kimlik ya da adla
   (büyük küçük harften bağımsız); `search` ve `tolerance` seçenek karşılaştırmasınınki (yoksa 0,5 m ve 1 mm, çizim
   biriminde; eksi `invalid_input`, yolu `compareCombos.search` ya da `.tolerance`). Olmayan bileşim
   `option_combo_not_found` (`compareCombos.a`, `.b`), aynı bileşim iki kez `invalid_input` (`compareCombos.b`).
   Öbür soruların (`set`, `uid`, `compare`) retleri önce gelir.
2. **Cevap.** `combosCompared`: projenin sırasıyla her küme, iki bileşimin gösterdiği seçenek (kendi seçimi, yoksa
   birincil) aynıysa `alike`'ta, ayrıysa `sets`'te ADR 0788 §2'nin karşılaştırmasıyla (`a`'nın seçeneği `b`'ninkine
   karşı). `objectsA`, `objectsB` iki planın nesneleri (ana model ve gösterilen seçenekler); `spaceArea` ve
   `wallLength` planların toplamlarının farkı (b − a). Okuma salt okunurdur.
3. **Masaüstü.** Tasarım seçenekleri'nin Bileşimler'inde, birden çok bileşim varken seçilen bileşim bir başkasıyla
   Karşılaştır: farklı kümeler “Mutfak A → B: 2 aynı, 3 değişmiş”, aynı kümeler, planın nesneleri, alan ve duvar farkı
   tek satırda. Seçilen bileşim karşılaştırılanla aynı yapılınca ikisi yer değiştirir. Pencerenin gövdesi kayar:
   1100×650'de Bileşimler'in eklenmesiyle eylem düğmeleri pencerenin dışına düşüyordu.

## Doğrulama

- Bağımsız başvuru KentOS kodu olmadan: `scripts/fixtures/option_combo_compare_cases.py` (option_cases.py'nin
  karşılaştırması ve option_combo_cases.py'nin bileşimleriyle) `cad.options.query.compare-combos.json`'a 26 durum
  yazar: ayrı ve aynı kümeler, ters yön, adlar, kayan duvar ve donatı, arama ve tolerans, silinen mahal, eklenen üye,
  üç seçenek, uygulanan ve birincili değişen, seçeneği silinen bileşim, seçenek karşılaştırması ve küme süzgeciyle,
  tek küme, yedi ret. Masaüstünde ortak durumlar (bir beklentiyi bozarak koştuğu da sınandı), yetenek testi, Python'un
  ortak durumları.
- Masaüstü: `design_option_scenes::the_window_compares_two_combinations`; resimler (iki tema, 1440×900 ve 1100×650)
  `KENTOS_SHOTS_ONLY=bilesim-karsilastir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored
  --nocapture`.
- Gerçek pencerede elle denenmedi.

## Sonuçlar

- Açık: seçenekler arası eşleme ve birleştirme (bir seçeneğin değişikliğini öbürüne taşımak).
- Web'e gerekenler (TODOS.md §0.2): `compareCombos` ve Bileşimler'in Karşılaştır'ı.
