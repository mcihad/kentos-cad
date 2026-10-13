# ADR 0266: Yazılan uzunlukta birim ve parantezli hesap

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-01`: mutlak ve göreli koordinat, mesafe<açı,
  doğrudan mesafe, açı kilidi, **birimli giriş ve hesap ifadesi** bütün çizim ve düzenleme araçlarında tutarlı. Kapsamı ben belirledim
  (sahibin sözü: “durmadan devam et”; CAD-DATA'dan sonra CAD-INPUT).
- **Bağlam belgesi:** ADR 0018 ve 0021 (yazılan noktanın dilbilgisi, `fixtures/point-input/v1`), 0165 §2 (yerel projenin çizim birimi),
  0166 (kilitler), 0261 (UCS), 0262 §3 (ayak ve inç işaretleri).
- **Web:** ertelendi (ADR 0228); web'in okuyucusu (`apps/web/src/tools/coordinateInput.ts`) ortak durumların yeni satırlarını geri dönüşte
  geçer; TODOS.md §0.2.

## Bağlam

Mutlak, göreli ve kutupsal nokta, doğrudan mesafe, açı kilidi (`<açı`), UCS ve ayak-inç işaretleri ortak dilbilgisindeydi. Eksik iki şey:

1. **Birimli giriş:** milimetre projesinde 2,5 m'lik bir duvarı `2500` diye yazmak gerekiyordu; `2.5m` okunmuyordu.
2. **Hesap:** `12.5+3*2` gibi bir değeri kâğıtta hesaplayıp yazmak gerekiyordu.

Ayrıca araçların kendi uzunluk alanları (yarıçap, aralık, genişlik, pah, bölme boyu …) çoğunlukla düz sayıyı okuyup çizim birimine
çeviriyordu: ayak-inç işaretli uzunluk bile o alanlarda okunmuyordu.

## Karar

### 1. Metrik birimli uzunluk

- Bir uzunluk sayının hemen ardından birimiyle yazılabilir: `mm`, `cm`, `m`, `km` (`2.5m`, `250cm`, `1mm`, `0.4km`). Birim her
  projede geçerlidir ve çizim biriminden bağımsızdır (ayak ve inç gibi, ADR 0262 §3): `2.5m` milimetre projesinde de 2,5 metredir.
- Sayıyla birim arasında boşluk yoktur (boşluk iki koordinatı ayırır); birimden sonra harf gelmez (`5mx` uzunluk değildir).
- Küçük birimler bölmeyle çevrilir (`1mm` tam 0,001 m'dir: 1 ÷ 1000), `km` çarpmayla.
- Açı birimsizdir; açıya birim yazılmaz.

### 2. Parantezli hesap

- Sayının yazıldığı her yerde parantez içinde bir toplam yazılabilir: `+`, `-`, `*`, `/`, terimin önünde işaret, iç içe parantez, arada
  boşluk (`(10+2.5)`, `(3*4)`, `-(2+3)`, `(((1+2)*(3+4)-1)/4)`). İşlem önceliği bilinen kuraldır (çarpma ve bölme önce, soldan sağa).
- Parantezin dışında işlem yoktur (`2*3` sayı değildir): `-` ve `+` sayının işareti, `,` ve `;` ayırıcı olarak kalır; böylece eski
  hiçbir yazının anlamı değişmez.
- Sıfıra bölme, sonlu olmayan sonuç, kapanmayan parantez ve 16'dan derin iç içelik sayı değildir. Parantezin içinde ondalık virgül
  `parse_number`'ın kuralıyla bir kez noktaya döner (`(1,5+2)`).
- Toplamın ardından birim gelebilir (`(2+0.5)m`); kutupsal açı da toplam olabilir (`@10<(180/2)`).
- `(` değer alanını açar (`looks_like_coordinate`, masaüstünün `value_start`'ı).

### 3. Bütün araçlarda tek kural

- Nokta yazımı zaten ortak okuyucudan geçer (`point_from_text_framed`). Araçların tek uzunluk alanları artık `plain_value` /
  `plain_length`'ten (`kentos_interaction::points`), iki uzunluklu alanları `parse_length_pair`'den, listeleri `parse_length`'ten geçer:
  çizgi, yay, daire, dilim, düzgün çokgen, dikdörtgen (köşe ve boyut), çoklu çizgi (genişlik, adım, uzunluk, yarıçap), ölçekle (başvuru
  boyu), uzat-kısalt, böl, parçala, yol boyunca dizi, dizi (aralıklar), tüm köşelere pah ve pah, buluşma, ara noktalar, Km yaz (enkesit,
  nokta), plan yolu, çizim ekleri (aralık), nokta hesaplayıcı (yan nokta, iki uzaklık, hat üzerinde, açıortay).
- Sayı olmayan alanlar (açı, adet, yüzde, alan, kâğıt yüksekliği) birim almaz; onlara birimli yazılan uzunluk karşılığı sayı gibi okunmaz.

## Sonuçlar

- Milimetre projesinde `@2.5m<0`, metre projesinde `250cm` ve `(1.2+0.3)m` aynı sonucu verir; ayak-inç ve metrik birim her uzunluk
  alanında okunur.
- Eski yazılar aynı anlamdadır (parantez ve birim harfi eskiden geçersizdi).

## Doğrulama

- Ortak durumlar `fixtures/point-input/v1/cases.json`: 22 yeni satır (metrik birim, toplam, retler); `kentos-geometry-core`'un
  `point_text` testi.
- Oturumda: `kentos-interaction` `typed_units.rs` (milimetre projesinde çizgi, daire, ayak-inçli yarıçap).
- Etkileşim izi `fixtures/interaction/v1/value-entry.json` masaüstünde üç çeşitte (ABD ve Türkçe Q klavyesi, HiDPI): `(` ile açılan alan,
  toplam, eksi fark, alana yazılan birim harfleri, Boşluk, Tab ile birimli uzunluğun kilidi (bulunan delik: Tab'ın kilidi yalnız düz sayıyı
  okuyordu, artık `parse_length`'ten), kapanmayan parantez, Esc. `KENTOS_TRACES_ONLY=value-entry cargo test -p kentos-desktop every_trace_passes`.
- Masaüstü resmi: `KENTOS_SHOTS_ONLY=giris-birim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-giris-birim-*`).
