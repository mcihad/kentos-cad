# ADR 0664: Etikette görünür içerik örnekleri

- Durum: bu masaüstü dilimi kabul edildi (26 Ekim 2026).
- Taban: `2d396fcbf`, aynı mimari dal; ARCH-SPACE-02 / ARCH-UX-01.

## İhtiyaç ve karar

Ad, numara, net alan ve kot şablonları mevcut anlamsal etiket komutunda
vardır. Yeni kullanıcının bunları seçebilmesi için Etiket ve Hepsini
etiketle araçlarında ayrı **Hazır içerik** alanı eklenir. Serbest Şablon
alanı aynen düzenlenebilir kalır; hazır seçim yalnız bu taslağı doldurur.
Özel metin ayrı seçili değerle korunur; tanınmayan seçim sessizce silmez.

Sekiz öneri: Numara, Ad, Net alan, Mahal kimliği, Mahal ve kotlar,
Döşeme kotu, Tavan kotu, Açıklık ve boyut. Mahal ve kotlar beş satırdır;
mevcut `{number}`, `{name}`, `{netArea}`, `{floorElevation}` ve
`{ceilingElevation}` alanlarını kullanır. Eksik veri veya yanlış kaynak
türü için mevcut boş değer kuralı korunur; kaynak türü veya kat uydurulmaz.
Geçerli sayfa/standart veya yasal alan profili seçilmiş sayılmaz.

Türe göre / Tür-Koru boş taslağı seçer: yeni etiket tür varsayılanıyla,
Yenile ise mevcut içerikle devam eder. Boy, çerçeve, kol, kaynak katı,
Koru/Yenile ve hedef katman bu seçimle değiştirilmez. Klavye İçerik
seçeneği aynı geçici durumu kullanır. Seçim ve önizleme modele yazmaz;
yerleştirme veya tek toplu onay mevcut `cad.tags.create` komutudur.

Yeni komut, API alanı, kalıcı preset kimliği, dosya şeması, birim/ölçek
veya render değişikliği yoktur. Python/MCP aynı `content` metnini alır.
Web ADR0228 gereği bekler; yeni TS/TSX dosyası üretilmez.

## Kabul kapıları

- Bağımsız sabit metinlerle bilinen net/kot, özel metin ve eksik veri;
  görünür seçim/klavye, geçersiz seçim, eski taslak ve kaynak korunumu.
- Tek ve toplu etiket, kat kapsamı/Koru-Yenile, salt okunur önizleme,
  tek onay/undo/redo; aynı anlamsal komut ve tam kayıt.
- Native gerçek fare/klavye ve ham görüntüler: seçim, serbest değiştirme,
  ret/iptal, kaynak güncellemesi, elle yer ve yeni süreçte açma; dar/geniş,
  açık/koyu, normal/büyük yazı. Yalnız doğrulanan varyantlar kabul edilir.
- İlgili interaction/desktop kontrolleri ve build/clippy; önceki KCAD
  dosyaları bayt düzeyinde korunur. Tüm mimari plan kabulü sayılmaz.

## Sonuç ve sınırlar

[Kabul kaydı](../baseline/2026-10-26-architecture-tag-contents/README.md):
63 birim +708 interaction entegrasyonu, desktop build ve iki pakette
all-targets clippy geçti. Dört yeni test, sabit beklenen net/kot metinleri,
sekiz seçimin klavye çevrimi, özel/geçersiz seçim ve katlı toplu onay/geçmişi
kapsar. İlk iki koşumdaki yanlış test verisi beklentileri ayrıca saklanır.
Bu dilimde Python/MCP takımları yeniden koşulmadı; ortak komut/SDK değişmedi.

Gerçek native tek onay, undo/redo, kaynak yüksekliği 312.5cm → 3.125m,
toplu Ad yenileme, özel şablon, Tür/Koru, geçersiz token reddi, Escape ve
temiz yeni süreçte kaydet-aç doğrulandı. 34 ham ekranın yedisi tanısaldır;
15 kabul modeli tüm alanlarıyla bağımsız okuyucuda karşılaştırılır. Üç
karışmış model ayrıca saklanır ve kabul sayılmaz. Önceki1810 KCAD baytı
aynıdır. Açık13 geniş, koyu18 geniş/dar menü ve etkileşim incelendi; bütün
tema/yazı/pencere bileşimlerinin tam matrisi veya performans garantisi yoktur.
Yeni biçimlendirme farkı yok; tag_tools içindeki iki eski rustfmt farkı
tabanla aynıdır. Çoklu görünüş/pafta bağlamı ve ARCH-SPACE-02 [~] sürer.
