# ADR 0293: Ötele'de delikli alan, eğriler, yaklaşık sonuç ve raporlar

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-07`: “Offset; line/arc/polyline/ellipse/NURBS/region için
  destek ve yaklaşık sonuç politikası belli olsun; yarıçap çökmesi ve self-intersection raporlansın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0246 (anlamsal Ötele komutu), 0047 (Ötele), 0143 (çok parçalı alan), 0149 §5.3 (eğrinin kirişleri, 0,1 mm),
  0259 (NURBS).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. Web'in donmuş çağrı kayıtları (`offsetEntity`) değişmedi: eğri desteği yalnız komutun yolunda.

## Bağlam

`cad.entities.offset` çizgi, çoklu çizgi, alan, daire, yay, elips ve yardımcı çizgileri ötelüyordu. Üç açık vardı: delikli alan ötelenince
delikleri **sessizce düşüyordu**; içe genişliğinden çok ötelenen alan tersine dönmüş bir halka olarak yazılıyordu; fit-point eğrisi ve NURBS
ötelenemiyordu. Elipsin kopyası yaklaşıktı ama bu söylenmiyordu, kendini kesen kopya da fark edilmiyordu.

## Karar

1. **Desteklenenler ve sonucun kesinliği:**

   | Kaynak | Kopya | Kesinlik |
   | --- | --- | --- |
   | çizgi, çoklu çizgi (düz ve yaylı), alan, kapalı çoklu çizgi | aynı tür | kesin (kenarlar kaydırılır, köşeler kesişimleri; yaylar aynı merkezle) |
   | daire, yay | aynı tür | kesin (yarıçap ± mesafe) |
   | delikli alan, çok parçalı alan | alan | kesin; halka noktanın yanına, delikler öbür yana kayar |
   | elips, eliptik yay | alan ya da çoklu çizgi (512 adım bir tur) | yaklaşık: kiriş en çok a·h²/8 uzaklaşır (a büyük yarı eksen, h adım) |
   | fit-point eğrisi, NURBS | çoklu çizgi (açık ya da kapalı) | yaklaşık: eğrinin 0,1 mm'lik kirişleri ötelenir (ADR 0149 §5.3) |
   | yardımcı çizgi, ışın | aynı tür | kesin |

   Yaklaşık kopya yazılır ve `approximate_offset` uyarısı ne kadar yaklaşık olduğunu milimetreyle söyler.
2. **Delikli alan:** nokta alanın içindeyse (bir deliğin içinde değil) alan küçülür: halka içe, delikler dışa; değilse alan büyür: halka
   dışa, delikler içe ve kapanan delik düşer. Halkalar birbirinden ayrı kalırsa kopya onlarla kurulur; değerse fark işlemiyle birleştirilir
   (büyüyen delik halkayı yutarsa ret). Çok parçalı alanın parçalarındaki delikler de aynı kuralla gider.
3. **Çökme:** bir yay parçası, daire ya da yay yarıçapını, elips en küçük eğrilik yarıçapını aşan mesafeyle, ya da kapalı halka genişliğinden
   çok içe ötelenince (bir kenarı tersine döner ya da alanı biter) kopya yapılmaz; kod `radius_collapse` (ret ya da İki yana'da öbür kopya
   yapıldıysa uyarı). Öbür başarısızlıklar `offset_failed` kalır.
4. **Kendini kesme:** kopyanın komşu olmayan kenarları kesişiyorsa kopya yazılır ve `self_intersecting` uyarısı kaç yerde kestiğini söyler
   (4 000 kenardan uzun yol denetlenmez). Kopyayı budamak kullanıcınındır.
5. **Çekirdek:** `ops::offset::offset_noted` (kopya ve notu: `approximate`, `crossings`), `collapses`, `crossings`; `offset_entity` web'in
   kayıtlarıyla aynı kalır (eğri desteği `offset_noted`'da).

## Sonuçlar

- Delikli bir parselin içe ötelenmesi artık deliklerini korur; daha önce kayıp sessizdi.
- Ajan ve Python, sonucun kesin mi yaklaşık mı olduğunu ve kendini kesip kesmediğini uyarılardan okur.

## Doğrulama

- `python3 scripts/fixtures/offset_command_cases.py --check` (36 durum: delikli alanın içe ve dışa ötelenmesi, kapanan delik, yutulan
  halka, tersine dönen kare, iki yerde kendini kesen yol; yarıçap çökmesi `radius_collapse`); `cargo test -p kentos-native-application
  --test all`; `cargo test -p kentos-geometry-core --test all offset_notes` (eğrinin kopyasının eğriye uzaklığı, delik, çökme, kesişim);
  web'in donmuş çağrı kayıtları (`every_call_fixture_matches_the_typescript_reference`) değişmeden geçer.
