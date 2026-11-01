# ADR 0807: XOX sürme ve eşit olmayan iki kanatlı sürme

- **Durum:** kabul edildi (2026-11-01).
- **İş:** plan/architecture-advanced.md §3 `ADV-03`'ün “Açık” satırındaki XOX ve eşit olmayan sürme kanatları; ADR 0777 ve
  0792'nin üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 153 (opt-in; §4). **Sözleşme:** yeni alan yok; `OpeningBuild.bothSides` üç kanatlı sürmeyi,
  `OpeningBuild.split` iki kanatlı sürmeyi de alır.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0792 üç ve dört kanatlı sürmeyi getirdi ama sektörün en sık iki biçimini açık bıraktı: ortası sabit, iki yandaki
kanatları ortaya kayan XOX (balkon ve teras kapılarında, vitrin pencerelerinde) ve kanatları eşit olmayan iki kanatlı
sürme (geniş kanadın yanına dar bir kanat). Kodda `bothSides` yalnız dört kanatta, `split` yalnız kanatlı ve cebe giren
kapıda anlamlıydı; sürmede `split` okunur, kullanılmazdı.

Başvurulan kaynaklar ADR 0792'ninkilerdir (Florida Building Code ürün onaylarındaki X/O gösterimi, Loewen'in sürme kapı
kataloğunda XOX'un standart oluşu, Archicad'in çok panelli sürme kapı nesnesi). XOX'ta dış kanatlar ortadaki sabit
kanadın önünde aynı rayda durur, ortada iki bindirme vardır: Loewen'in kesitleri bunu iki rayla gösterir.

## Karar

1. **XOX.** Üç kanatlı sürmede `build.bothSides: true`: ortadaki kanat 0. rayda sabit, iki yandaki 1. rayda ve ortaya
   kayar. Kanadın eni `w = (ışık + 2·bindirme) / 3`, başlangıçları 0, `w − b`, `2(w − b)`; bindirme yazılmazsa ADR
   0792'nin kuralıyla `ışık · 0,05 / 2`. Net geçiş bir kanadın açtığı `w − b`. Kanat düzeni “XOX”. Yalnız iki rayla:
   `tracks: 3` reddedilir (“XOX sürmenin iki yandaki kanadı ortadakinin önünde, tek raydadır; ray sayısını 2 yapın ya da
   tek yana açın.”). İki yana açılma sürmede yalnız üç ve dört kanatta (öbürlerinde “İki yana açılma üç ya da dört
   kanatlı sürmenindir …”). Planda iki yandaki kanatların okları ortaya döner.
2. **Eşit olmayan iki kanat.** İki kanatlı sürmede `build.split` birinci kanadın ışıktaki payıdır (0 ile 1 arası, uçlar
   hariç): kanatlar `pay·ışık + bindirme/2` ve `(1 − pay)·ışık + bindirme/2` (bindirme yazılmazsa `ışık · 0,05`), net
   geçiş `ışık − büyük kanat`. Pay yazılmazsa eski formül bit bit aynı kalır. Planda iki kanat `ışık · pay`'da buluşur
   (`geom::opening::two_leaf_meet`; kapı ve pencere).
3. **Araç ve pencereler.** Kapı'da üç kanatlı sürmede İ “Ortası sabit (XOX)”, iki kanatlıda Ş “Birinci kanadın payı”
   (%60 ya da 0.6, boş eşit); ikisi de `cad.openings.create`'in girdisidir (Araç = komut). Öznitelikler'de üç kanatta
   Ortası sabit (XOX), iki kanatta Birinci kanadın payı ve Kanatlar satırı; Kapı ve pencere tipleri penceresi zaten
   İki yana ve Kanat payı alanlarını sürmede gösteriyordu.
4. **Şema 153.** XOX ya da paylı iki kanatlı sürme (açıklıkta ya da projenin tipinde) şema 153 ister: önceki şemada
   XOX `bad_value`, sürmedeki pay önceden okunup kullanılmazdı. Yazıcı 153'ü yalnız bunlardan biri varken seçer
   (`needs_slide_more`, `ArchSettings::has_slide_more`). Örnekler `slide-more.kcad`, `broken/slide-more-in-schema-152.kcad`,
   `broken/multi-slide-both-ways-two-leaves.kcad`; aralık dışı `broken/schema-version-154.kcad`.

## Doğrulama

- Bağımsız başvurular KentOS kodu olmadan: `special_doors_command_cases.py` (yeni `cad.openings.create.slidemore.json`
  22 durum, `cad.openings.query.slidemore.json` 20 durum; ADR 0792'nin multislide ve özel kapı dosyaları XOX'la
  güncellendi) ve `special_doors_cases.py` (`multi-slide.json`'a 8 çizim durumu: XOX ve paylı iki kanat). Masaüstünde
  241 ortak komut durumu, çekirdeğin 30 çizim durumu, KCAD'in 234 testi ve bağımsız Python okuyucusu (1468 dosya) geçer.
- Araç: `opening_special::a_sliding_door_takes_xox_and_its_leaves_share`.
- Resimler (iki tema, 1440×900 ve 1100×650): `KENTOS_SHOTS_ONLY=surme-xox-plan,surme-xox-yakin,surme-xox-arac,
  surme-pay-arac,surme-pay-oznitelikler,surme-xox-tipler cargo test -p kentos-desktop --lib tools_screens::screens --
  --ignored --nocapture`.
- Gerçek pencerede elle denenmedi.

## Sonuçlar

- Açık: dört kanatta eşit olmayan kanatlar, XOX'ta dış kanatların farklı eni, kaldır-sür (lift-and-slide) eşiği.
- Web'e gerekenler (TODOS.md §0.2): şema 153'ün okunması, XOX ve paylı sürmenin çizimi, Kapı aracının İ ve Ş ayarları.
