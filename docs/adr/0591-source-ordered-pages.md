# ADR 0591: Kaynakta sıralama ve sıralı sayfalar

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-20: Öznitelik tablosunda sağlayıcıya itilen sıralama, süzgeç ve
  toplam; imleçli sayfalar.
- **Bağlam belgesi:**
  - ADR 0581: `cad.sources.query`, kaynağa giden kutu, sütunlar ve süzgeç, anahtar sırasıyla sayfalar;
  - ADR 0546 §4: tablonun kaynağa itilen sıralaması ve sayfaları (tasarım);
  - ADR 0199 §4: Öznitelik tablosunun sıralama kuralı (türlere göre, doğal Türkçe sıra);
  - ADR 0505, 0582: PostGIS bağlı katmanı.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez. Sıra ve imleç sorunun girdisidir, çizime yazılmaz.
- **Sonraya kalan:**
  - tablonun Kaynağın tümü kipinde hücre düzenleme (bugün salt okunur; düzenleme Çalışma kopyası'nda);
  - birden çok sütunla sıralama;
  - metin sütununun kaynakta sıralanması (Türkçe doğal sırayı veren harmanlama GeoPackage'ta yok; PostgreSQL'in ICU `tr-TR`'si
    sayıları sayı gibi sıralamaz);
  - PostGIS'te sorgunun işlem anlık görüntüsüne bağlanması (bugün bir sayfa kendi okumasıdır; sayfalar arası değişiklik
    anahtar kümesiyle tekrar ya da atlama yapmaz ama yeni satırlar sonraki sayfada görünebilir).

## Bağlam

ADR 0581'den beri `cad.sources.query` bir GeoPackage ya da PostGIS tablosunu kaynağın anahtarı sırasıyla sayfa sayfa okuyor:
kutu, sütunlar ve süzgecin kaynağın denetleyebildiği kısmı kaynağa gidiyor, toplam ve istatistik soruya uyan kayıtlardan
çıkıyor. Tablonun gösterdiği sıra ise başka bir alandır: kullanıcı başlığa basınca tablo o sütuna göre sıralanır. Bugün bunu
yalnız çalışma kopyası yapabiliyor (çizimdeki nesneleri bellekte sıralayarak); milyon satırlık bir tablonun sıralı ilk sayfası
için bütün tabloyu istemciye indirmek gerekiyordu. Plan bunu ister: “Milyon satırlı tabloda sıralama değişince tamamı istemciye
inmeyebilmeli; kararlı cursor ve snapshot sayfa tekrarı/atlamasını önlemeli.”

## Karar

### 1. Kaynağın sırası: anahtar kümesi

Sağlayıcının taraması (`gpkg::Scan`, `ProviderScan`) bir sıra alır: `ScanOrder { column, descending, after }`. Satırlar
`ORDER BY sütun ASC|DESC NULLS LAST, anahtar` ile, CHUNK (1000) satırlık parçalarda gelir. Her parça bir öncekinin son
satırından sonra başlar (anahtar kümesi, `OFFSET` değil):

- son değer NULL değilse `(sütun > d OR (sütun = d AND anahtar > k) OR sütun IS NULL)` (azalan sırada `<`);
- son değer NULL ise `(sütun IS NULL AND anahtar > k)`.

NULL her iki yönde de sondadır; eşit değerler anahtarla, her iki yönde de artan. PostgreSQL'de değer metin olarak bağlanır ve
sütunun kendi türüne çevrilir (`CAST($n AS tür)`); SQLite'ta sayı sayı, metin metin bağlanır. Süzgecin gönderilen kısmı ve kutu
aynı sorgudadır.

### 2. Kim sıralar

`cad.sources.query`'nin yeni `order`'ı (`{ field, descending }`):

- **Kaynakta (`plan.order: column`):** sayı (tam ve ondalık), tarih, tarih ve saat, evet/hayır ve sağlayıcının tanıdığı öbür
  türler. Kaynak kendi değer sırasıyla sıralar; tarih `YYYY-AA-GG` metni olarak takvim sırasıdır, evet/hayır önce hayır.
- **Burada (`plan.order: here`):** metin (ve ikili) sütunlar ve çizimin katmanı. Kayıtlar okunur, süzülür, öznitelik tablosunun
  kuralıyla (`feature_table`; alanın türüne göre sayı, tarih, evet/hayır ya da doğal Türkçe sıra) sıralanır; değeri olmayan
  (ya da boş metin) sonda, eşitler yerleriyle (anahtar ya da kalıcı kimlik). Burada sıralanan soru en çok
  `MOST_ORDERED_HERE` (200 000) kayıt tutar; fazlası `order_too_large`: süzgeçle, kutuyla ya da kaynağın sıraladığı bir alanla
  sorulmalı. Metnin kaynakta sıralanması tablonun gösterdiği sırayı vermez (bkz. Sonraya kalan), o yüzden itilmez.

Sırasız soru eskisi gibi anahtar (`key`) ya da kalıcı kimlik (`uid`) sırasıyladır.

### 3. Sıralı imleç ve dosyanın damgası

Sıralı sorunun `next`'i `o:` ve son satırın yerinin ve değerinin JSON'udur: `o:[17,12.5]`, `o:[17,"2021-06-01"]`,
`o:[17,null]`, çizimde `o:["<kalıcı kimlik>","Ada 3"]`. Değer kaynakta sıralanan sütunda kaynağın ham değeridir (sayı, tarihin
metni, evet/hayır için 0 ya da 1), burada sıralananda özniteliğin metni. İmleç kanonik yazımıyla okunur (boşluksuz); başka bir
yazım, sırasız soruya sıralı imleç ya da sıralı soruya `k:`/`u:` imleci `invalid_cursor`'dır. Burada sıralamada imlecin kaydı
silinmiş olsa da sayfa onun sıradaki yerinden sürer.

Sayılan (`count`, `statistics`, `read`) imleçten sonra gelenlerdir, sırasız sorudaki gibi.

Yeni `expectedStamp`: ilk sayfanın döndürdüğü dosya damgası. Verilirse ve dosya değişmişse soru okunmaz, `source_changed`
çatışmasıdır (`path: expectedStamp`). Böylece bir dosyanın sayfaları aynı dosyanındır. Veritabanında damga yoktur; anahtar kümesi
sayfaların tekrarını ve atlanmasını önler.

### 4. Masaüstü: Kaynağın tümü

Öznitelik tablosunda bağlı katmanın kaynak satırına **Kaynağın tümü** düğmesi gelir (açıkken **Çalışma kopyası** olur). Açıkken
tablo katmanın kaynağını gösterir, `cad.sources.query` (`scope: source`) ile kendi iş parçacığında, 500 satırlık sayfalarla,
Durdur'la:

- başlığa basmak sıralar (artan, azalan, sonra anahtar); Sıra anahtar sırasına döner;
- İfade süzgeci sorunun süzgecidir;
- satırların altında **Sonraki sayfa** son sayfanın `next`'iyle ve ilk sayfanın damgasıyla sürer; dosya arada değişmişse
  söylenir ve ilk sayfa yeniden okunur;
- üst satır okunanı ve toplamı, sırayı kimin yaptığını (kaynakta sıralandı, burada sıralandı) ve süzgecin nerede denetlendiğini
  söyler;
- satırlar salt okunurdur; çalışma kopyasında olan satıra basmak nesnesini seçer, olmayan soluk gösterilir.

## Doğrulama

- `scripts/fixtures/source_order_cases.py` (GDAL'ın yazdığı `fixtures/sources/v1/sira.gpkg`: 2400 satır, metin, ondalık, tam
  sayı, tarih ve evet/hayır sütunları, NULL'lar ve boş metinler; satırlar sqlite3 ile okunur), 34 ortak durum
  (`fixtures/commands/v1/cad.sources.query.order.json`): kaynağın sırası iki yönde ve sayfa sayfa, eşitler arasında ve NULL'lar
  arasında imleç, son sayfa, tam sayı, tarih, evet/hayır, burada denetlenen süzgeçle 1000'lik parçalar, kaynağa giden süzgeç ve
  sayı, imleçten sonra sayılanlar ve istatistik, metnin doğal Türkçe sırası iki yönde, boşların arasında imleç, silinmiş kaydın
  imleci, çizimin katmanı (tam sayı, ondalık, metin alanı; eşitler kalıcı kimlikle), sekiz ret ve dosyanın damgası. Doğal sıra ve
  türlerin anahtarları `feature_table_cases.py`'den, süzgecin gönderilen kısmı `source_query_cases.py`'nin kuralıyla; KentOS
  kodu kullanılmaz.
- `crates/native/application/tests/all/sources_query.rs`:
  - rastgele değerli 2600 satırda her sütun ve yön için 7, 333, 1000 ve 1001'lik sayfaların birleşimi tek sayfalık cevaba eşit,
    çizimin aynı katmanı aynı değer sırasını verir;
  - 200 001 satırlık metin sıralaması `order_too_large`, süzgeçle ya da kaynağın sıraladığı sütunla cevaplanır.
- `crates/native/postgis/tests/all.rs`'in `orders_answer_alike_from_postgis`'i, `postgis_cases.py`'nin geçici PostGIS'inde
  (`sira` şeması): sekiz sıralı soru her sayfasıyla dosyayla aynı satırları, anahtarları, `next`'i ve toplamı verir.
- Masaüstü `features::whole_tests`: sayfalar, sıralar, süzgeç, satır seçimi, eski cevabın düşmesi; resimler
  `whole_screens` (`.run/shots/tablo-tumu-*`).
- ADR 0581'in 48 ortak durumu değişmeden geçer.
