# 1096 — Dışa aktarma kaybı denetimi eğri biçimini, uzunluğu ve alanı ayrı karara bağlar

Tarih: 2026-11-20. gis-usage-report.md GIS-039. Durum: kabul edildi.

İlgili: ADR 1660 (Dışa aktarma kaybı denetimi), ADR 1880 (Destek matrisi), ADR 1071 (GIS-005, GeoPackage yayı).

## Sorun

GeoPackage gidiş-dönüşünde yedi nesne eşleşti, köşe kayması 0 m çıktı; aynı yeşil satırda uzunluk farkı 119 464 051 m,
alan farkı 1,1·10¹⁵ m² yazıyordu. Satırın rengi yalnız toleransı aşan köşe sayısına bağlıydı; uçları doğru, yayı
bozuk bir tampon geçiyor, matris eğriyi ve köşe konumunu “Korunur” gösteriyordu.

## Karar

1. **Ayrı ölçüler.** `file.exchange.check`'in sayısal cevabı köşe konumunun yanında üç ölçüyü ayrı karara bağlar ve
   her biri için aşan nesne sayısını ve en kötü nesnenin kimliğini verir (`shapeAbove/shapeUid`,
   `lengthAbove/lengthUid`, `areaAbove/areaUid`; sıfırken yazılmaz, eski cevaplar aynı kalır):
   - **Eğri biçimi:** geri okunan nesnenin yaylı kenarlarının ¼, ½ ve ¾ noktalarının özgün geometriye uzaklığı
     (`shape`); tolerans aynı mutlak tolerans. Uçları doğru, yönü ya da yarıçapı yanlış yay burada yakalanır.
   - **Uzunluk:** fark `2·n·tolerans`'ı (n: özgün nesnenin köşe sayısı) aşarsa. Gerekçe: toleransın içindeki her köşe
     iki kenarının uzunluğunu en çok iki tolerans değiştirir.
   - **Alan:** fark `çevre × tolerans`'ı aşarsa. Gerekçe: sınırı tolerans kadar oynayan bir alanın değişimi çevresi
     çarpı toleranstan büyük olamaz (birinci derece).
   Alan çizgi olarak geri okunduysa (DXF'de deliğin ayrı çizgi olması) uzunluk ve alan ölçülmez: bu, “Türü değişti”
   kaybıdır ve orada söylenir.
2. **Matris.** `EXCHANGE_ASPECTS`'e “Köşe konumu”nun ardından `shape` (Eğri biçimi), `length` (Uzunluk), `area` (Alan)
   satırları eklenir (16 satır). Aşan nesne varsa hücre `converted` ve “Toleransı aşar, n nesne: en çok …” der; yoksa
   “Korunur (en büyük fark)”. Kayıp (`lost`), yaklaşık (`approximated`, kirişe dönen eğri ve toleransı aşan köşe) ve
   sayısal sapma (`converted`) böylece ayrı görünür.
3. **Pencere.** Tek “Sayısal” satırı yerine her ölçü kendi satırında, kendi kararının rengiyle: Köşe konumu, Eğri
   biçimi, Uzunluk farkı (izin köşe başına iki tolerans), Alan farkı (izin çevre × tolerans), kot ve ölçü farkı bilgi
   olarak. Aşan ölçünün satırı en kötü nesneyi etiketi ve katmanıyla anar; Kayıplıları seç onları da seçer.

## Kanıt

- `crates/native/headless/src/exchange_check.rs` `a_wrong_arc_with_right_ends_fails_its_shape_and_area`: Ankara
  koordinatlarında yarım daire uçlu alan, yayı ters geri okununca köşeler geçer, biçim (10 m: ters yayın tepe noktası
  karenin uzun kenarına uzaklığı) ve alan (100π m²) kalır, nesnenin kimliğiyle; düz kenar yay olarak dönerse uzunluk da
  kalır; değişmeden dönen kare ve tampon her ölçüden geçer. Bağımsız değerler elle hesaplandı (π·10²).
- `apps/desktop/src/tests/exchange_check_window_tests.rs`: örnek çizimde altı biçimin hepsi (kirişe çevirenler de)
  biçim, uzunluk ve alanı tolerans içinde tutar; pencere satırları ekranda (resim `gis-039-gpkg-denetim`).
- `python/tests/test_exchange_check.py` matrisin yeni satırlarını aynı kuralla bekler (bu oturumda Python yerel modülü
  yeniden derlenmediği için koşturulmadı).
