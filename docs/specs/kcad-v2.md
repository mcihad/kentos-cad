# KCAD v2: KentOS proje dosyası (`.kcad`) bayt spesifikasyonu

- **Sürüm:** kap 2.0, belge şeması 2 (yazıcı gerektiğinde 3–18 yazar, §6.1), KentOS CBOR profili 1.
- **Durum:** kabul edildi (2026-09-26, [ADR 0025](../adr/0025-kcad-v2-encoding.md)). Yön [ADR 0011](../adr/0011-kcad-binary-snapshot.md)'den, kimlikler [ADR 0014](../adr/0014-persistent-entity-identity.md)'ten gelir.
- **Kapsam:** TODOS.md `FILE-01..08`, `FILE-12`, `FILE-22`, `FILE-23`.
- **Başvuru uygulamaları:** Rust kodlayıcı ve çözücü `crates/shared/kcad` (`kentos-kcad`); tarayıcıda aynı kod `crates/wasm/formats-wasm` ile; bağımsız Python okuyucusu `tools/kcad/kcad.py`; bayt düzeyinde örnekler `fixtures/kcad/v2`.

Bu belgede **-meli/-malı** zorunluluk, **-abilir** serbestlik bildirir. Bayt değerleri onaltılıktır (`0x89` ya da yalnız `89`).

## 1. Kapsam

- `.kcad` bir projenin **tek bir anlık görüntüsüdür**: kaydetme ve yükleme içindir. İçinde veritabanı, SQL, kalıcı arama ya da seçim indeksi, işlem günlüğü ya da tile düzeni yoktur (ADR 0011).
- Açık çizim bellekte düzenlenir. Kaydetme o anki sürümün baytlarını üretir; açma baytları yeniden çizime çevirir.
- **Uzantı** `.kcad`. **MIME türü** `application/octet-stream`: kayıtlı özel bir tür yoktur, kayıtlıymış gibi sunulmaz (`FILE-13`).
- Eski biçim (v1) JSON metindir (`kentos.document` sürüm 1) ve okunmaya devam eder. İki biçim uzantıdan değil **içerikten** ayrılır (§8).

## 2. Genel düzen

```
bayt 0                      başlık (sabit 36 bayt + zorunlu uzantı listesi)
bayt headerLength           yük: tek bir CBOR öğesi, payloadLength bayt
bayt headerLength + payloadLength
                            SHA-256 özeti, 32 bayt
dosya sonu
```

- Dosyanın boyu **tam olarak** `headerLength + payloadLength + 32` bayttır. Eksiği (`truncated`) de fazlası (`trailing_data`) da hatadır.
- Başlıktaki tam sayılar **küçük sonludur** (little-endian). CBOR kendi tanımı gereği **büyük sonludur** (RFC 8949). Bayt sırası sabittir, dosyada bayt sırası işareti yoktur.

## 3. Başlık

| Konum | Boy | Alan | Tür | 2.0 yazıcısının değeri |
|---|---|---|---|---|
| 0 | 9 | `magic` | bayt | `89 4B 43 41 44 0D 0A 1A 0A` |
| 9 | 1 | `major` | u8 | `2` |
| 10 | 1 | `minor` | u8 | `0` |
| 11 | 1 | `minReaderMinor` | u8 | `0` |
| 12 | 2 | `headerLength` | u16 | `36` + uzantı listesinin boyu |
| 14 | 1 | `encoding` | u8 | `1` |
| 15 | 1 | `codec` | u8 | `0` |
| 16 | 8 | `payloadLength` | u64 | yükün diskteki bayt sayısı |
| 24 | 8 | `decodedLength` | u64 | kodek çözülünce yükün bayt sayısı; kodek `0`'da `payloadLength` |
| 32 | 2 | `flags` | u16 | `0` |
| 34 | 2 | `extensionCount` | u16 | `0` |
| 36 | … | `extensions` | liste | `extensionCount` kayıt (§3.6) |

### 3.1 İmza

`89 4B 43 41 44 0D 0A 1A 0A`, yani `\x89KCAD\r\n\x1a\n` (PNG'nin düzeni):

- `89`: 7 bitlik aktarımı ve metin dosyası sanılmayı yakalar; JSON hiçbir zaman bu baytla başlamaz.
- `KCAD`: dosyayı gözle tanıtır.
- `0D 0A` ve sondaki `0A`: dosya metin olarak aktarılıp satır sonları çevrilirse imza bozulur.
- `1A`: eski DOS `type` komutunda dökümü durdurur.

İlk beş bayt (`89 4B 43 41 44`) tutup imzanın kalanı tutmuyorsa dosya bir KCAD'dir ama aktarımda bozulmuştur: `damaged_signature`.

### 3.2 Sürümler

- **`major`**: kap düzeninin ana sürümü. Okuyucu yalnız bildiği ana sürümü okur; 2.0 okuyucusu yalnız `2`'yi. Başka bir değer `unsupported_version`'dır.
- **`minor`**: dosyayı yazan yazıcının küçük sürümü. Bilgi içindir; okuyucu buna bakarak dosyayı reddetmez.
- **`minReaderMinor`**: dosyayı doğru okumak için gereken en küçük okuyucu sürümü `major.minReaderMinor`'dır. Yazıcı bunu dosyada **gerçekten kullandığı** en yeni özelliğe göre yazar: 2.3 yazıcısı yalnız 2.0 özelliklerini kullandıysa `0` yazar ve 2.0 okuyucusu dosyayı açar (`fixtures/kcad/v2/readable-minor.kcad`).
  - Okuyucunun küçük sürümü `minReaderMinor`'dan küçükse dosya **açılmaz**: `newer_version`, “KentOS'u güncelleyin”.
  - `minor ≥ minReaderMinor` olmalıdır; değilse `bad_header`.
- **Belge şeması sürümü** ayrıdır ve yükün içindedir (§6.1). Kap ile şema birbirinden bağımsız değişebilir (`FILE-02`).
- **En az yazıcı sürümü ayrı bir alan değildir.** 2.0 okuyucusu anlamadığı hiçbir içeriği açmaz (§3.6, §6); bu yüzden açabildiği dosyayı kayıpsız yeniden yazar. Eski bir istemci yeni bir anlık görüntüyü veri kaybederek yazamaz (`SYNC-14`). Atlanabilir (isteğe bağlı) içerik tanımlanırsa en az yazıcı sürümü o küçük sürümde başlığa eklenir.

### 3.3 Kodlama ve sıkıştırma kimlikleri

| `encoding` | Anlamı |
|---|---|
| `1` | KentOS CBOR profili 1 (§5) ile yazılmış belge şeması (§6) |
| başka | tanımsız: `unknown_encoding` |

| `codec` | Anlamı |
|---|---|
| `0` | yok: yük diskte olduğu gibi durur, `decodedLength = payloadLength` olmalı |
| başka | tanımsız: `unknown_codec` |

- 2.0'da sıkıştırma yoktur (`FILE-10`, `FILE-11`). Alan ayrılmıştır: yeni bir kodek yeni bir küçük sürümle ve `minReaderMinor` yükseltilerek gelir. Kodek çözülmeden önce `decodedLength` sınıra (§3.4) göre denetlenir; SHA-256 diskteki (sıkıştırılmış) baytları kapsar, bozuk veri çözücüye hiç verilmez.

### 3.4 Uzunluklar ve sınırlar

- `headerLength ≥ 36` olmalı. Uzantı listesi başlığı tam doldurur (§3.6).
- `payloadLength ≤ 2³⁰` (1 073 741 824 bayt) ve `decodedLength ≤ 2³⁰` olmalı; değilse `too_large`. Bir okuyucu platformunun belleğine göre daha küçük bir sınır uygulayabilir; o zaman sınırı ve nedenini söyler.

### 3.5 Bayraklar

`flags` 2.0'da `0` olmalıdır; başka bir değer `bad_header`'dır. Bir bayrak ancak yeni bir küçük sürümle, `minReaderMinor` yükseltilerek tanımlanabilir.

### 3.6 Zorunlu uzantılar

- `extensions`, `extensionCount` tane kayıttan oluşur: 1 bayt ad uzunluğu `n` (1…64), ardından `n` bayt ASCII ad; ad yalnız `a–z 0–9 . _ -` içerir (`kentos.blocks` gibi).
- En çok 64 kayıt olabilir. Kayıtlar tam olarak `headerLength`'e kadar sürer. Sözdizimi bozuksa `bad_header`.
- Listedeki her ad **zorunludur**: okuyucu o uzantıyı bilmiyorsa dosyayı **açmaz** ve adını söyler (`unknown_extension`). Salt okunur açılış yapılmaz: zorunlu uzantı içeriğin anlamını değiştirir (yeni nesne türü, şifreleme); bilmeden açmak çizimi eksik ya da yanlış gösterir (TODOS.md `DOM-06`: sessiz düşürme yok). Kararın gerekçesi ADR 0025'tedir.
- 2.0'da tanımlı uzantı yoktur; 2.0 yazıcısı liste yazmaz.

### 3.7 Bütünlük

- Dosyanın son 32 baytı, kendisinden önceki **bütün baytların** (başlık ve yük) SHA-256 özetidir (FIPS 180-4).
- Özet **bozulmayı** yakalar: disk hatası, yarım kopya, aktarımda değişen bayt. **İmza değildir**; dosyayı kimin yazdığını kanıtlamaz, kasıtlı değişikliğe karşı koruma vermez (`FILE-12`). İmza ya da şifreleme ileride ayrı bir zorunlu uzantı olur.

## 4. Okuma sırası

Okuyucu şu sırayla denetler ve ilk tutmayan denetimin hata kodunu (§9) verir:

1. Dosya boşsa `empty`.
2. İlk 9 bayt imza değilse: ilk 5 bayt `89 4B 43 41 44` ise (dosya imzanın başından kısaysa `truncated`, değilse `damaged_signature`); başka her durumda `not_kcad`.
3. Dosya 36 bayttan kısaysa `truncated`.
4. `major ≠ 2` ise `unsupported_version`.
5. `minReaderMinor` okuyucunun küçük sürümünden büyükse `newer_version`.
6. `headerLength < 36` ise `bad_header`.
7. `payloadLength` ya da `decodedLength` 2³⁰'dan büyükse `too_large`.
8. Dosya boyu `headerLength + payloadLength + 32`'den küçükse `truncated`, büyükse `trailing_data`.
9. SHA-256 tutmuyorsa `hash_mismatch`.
10. `minor < minReaderMinor`, `flags ≠ 0`, `extensionCount > 64` ya da uzantı listesi bozuksa `bad_header`.
11. `encoding ≠ 1` ise `unknown_encoding`; `codec ≠ 0` ise `unknown_codec`; `decodedLength ≠ payloadLength` ise `bad_header`.
12. Uzantı listesi boş değilse `unknown_extension` (2.0 hiçbir uzantıyı bilmez).
13. Yük §5'e göre çözülür ve §6'ya göre denetlenir.

- Özet (9) başlığın alanları yorumlanmadan önce denetlenir; bozuk bir başlık yanıltıcı bir hataya yol açmaz.
- Aynı dosyada birden çok kural bozuksa 13. adımda **hangisinin** bildirileceği tanımlı değildir; okuyucular yalnız dosyayı reddetmekte anlaşır. Örnek dosyaların her biri tek bir kuralı bozar.
- Okuyucu dosyayı **ya bütünüyle açar ya hiç açmaz**: yarım okunmuş bir çizim açık çizimin yerine geçmez (`FILE-20`).

## 5. KentOS CBOR profili 1

Yük, RFC 8949 CBOR'unun bu bölümle daraltılmış bir alt kümesidir. Profil **belirlenimlidir**: aynı belge (aynı alan değerleri, aynı nesne sırası) her yazıcıda **aynı baytlara** kodlanır (RFC 8949 §4.2.1 “core deterministic encoding” ve aşağıdaki ek kurallar). Okuyucu profilin her kuralını denetler; profile uymayan bir yük geçerli CBOR olsa da reddedilir.

### 5.1 İzin verilen türler

| Ana tür | Anlamı | Profilde |
|---|---|---|
| 0 | işaretsiz tam sayı | var; en kısa biçim |
| 1 | negatif tam sayı | var; en kısa biçim |
| 2 | bayt dizgisi | yalnız şemanın bayt istediği yerde (kimlikler, özet); belirli uzunluk |
| 3 | metin dizgisi | var; UTF-8; belirli uzunluk |
| 4 | dizi | var; belirli uzunluk |
| 5 | harita | var; belirli uzunluk; yalnız metin anahtar; sıralı; yinelenmez |
| 6 | etiket | **yok** (`tag`) |
| 7 | basit ve kayan noktalı | yalnız `F4` false, `F5` true, `F6` null ve `FB` binary64 |

- Ek bilgi 28–30 ayrılmıştır, yerinde olmayan `FF` (kırma) ve 0, 1, 6, 7 ana türlerinde 31 iyi biçimli değildir: `malformed`.
- 2–5 ana türlerinde belirsiz uzunluk (ek bilgi 31) yasaktır: `indefinite_length`.
- `F7` (undefined), `E0`–`F3` ve `F8` basit değerleri yasaktır: `simple_value`.
- `F9` (float16) ve `FA` (float32) yasaktır: `narrow_float`.

### 5.2 Belirlenimli kodlama

- **En kısa tam sayı ve uzunluk:** bir öğenin argümanı (tam sayının değeri ya da dizgi, dizi, harita uzunluğu) 0–23 ise ilk bayta gömülür; 24–255 bir, 256–65 535 iki, 65 536–2³²−1 dört, daha büyüğü sekiz ek baytla yazılır. Daha uzun biçim `non_shortest`'tir.
- **Belirli uzunluk:** her dizgi, dizi ve harita uzunluğunu önden yazar.
- **Harita anahtarları** metindir (`non_text_key`) ve **kodlanmış anahtar baytlarına göre sözlük sırasındadır** (RFC 8949 §4.2.1). Metin anahtarlarda bu şuna eşittir: önce UTF-8 uzunluğu kısa olan, uzunluk eşitse baytça küçük olan. Örnek: `p` < `zs` < `uid` < `attrs` < `layerId` < `lineWeight`.
  - Sırasız anahtar `unsorted_keys`, aynı anahtarın ikinci kez gelmesi `duplicate_key`'dir. Sıra kodlanmış baytlarla denetlendiği için iki denetim tek karşılaştırmadır.
- **Yok olan değer yazılmaz:** isteğe bağlı bir alanın değeri yoksa anahtar da yoktur. Şemalı alanlarda `null` yazılmaz (§6); tek istisna köşe kotlarının listesidir: `zs` köşe başına bir öğe taşır, kotsuz köşe `null`'dır (§6.6).
- Dosyada zaman damgası, rastgele sayı ya da yazıcıya özgü bilgi yoktur; aynı belge her kayıtta aynı baytları verir.

### 5.3 Sayılar

- **Kayan noktalı sayılar her zaman binary64'tür** (`FB` + 8 bayt, IEEE 754, büyük sonlu). Değer daha dar bir biçime kayıpsız sığsa da (`1.0`, `0.5`) daraltılmaz. Koordinatlar ve bütün şemalı float alanları bit bit korunur (`FILE-06`).
- **NaN, +∞ ve −∞ hiçbir yerde yazılamaz** (`non_finite`). Çizim geometrisi sonlu sayıdır; JSON ile çalışan istemciler de bu değerleri taşıyamaz.
- **−0 yazılabilir ve olduğu gibi korunur** (`80 00 … 00` işaret biti). Okuyucu −0'ı +0'a çevirmez; −0 ile +0 farklı dosya baytları verir.
- **Tam sayılar** şemanın verdiği aralıktadır (§6); şemalı bir float alanına tam sayı yazılamaz (`wrong_type`), şemalı bir tam sayı alanına float yazılamaz.
- **Kesin ondalık ve hisse değerleri** float olarak yazılmaz. Belge şemalarında (2, 3, 4) böyle bir alan yoktur (öznitelikler metindir). Eklendiğinde sözleşmedeki kesin biçimleriyle yazılır: ondalık, üssüz ondalık metin (`"748.5151"`, `DecimalString`); hisse, pay ve paydası ondalık metin olan harita (`ShareValue`) (CLAUDE.md §23.1).

### 5.4 Metin

Metin dizgileri geçerli UTF-8 olmalıdır: aşırı uzun biçim, vekil (surrogate) kod noktası ve yarım karakter yasaktır (`invalid_utf8`). Metin normalleştirilmez; bayt bayt korunur.

### 5.5 Sınırlar

| Sınır | Değer | Aşılırsa |
|---|---|---|
| Yük ve çözülmüş yük | 2³⁰ bayt | `too_large` |
| İç içe dizi ve harita derinliği (en dıştaki harita 1) | 64 | `too_deep` |
| Bir dizinin öğe ya da bir haritanın çift sayısı | 2²⁴ (16 777 216) | `too_long` |
| Bir metin ya da bayt dizgisinin boyu | 2²⁴ bayt | `too_long` |

- Önce sınır, sonra yükte kalan bayt denetlenir: `n` öğeli bir dizi için en az `n`, `n` çiftli bir harita için en az `2n`, `n` baytlık bir dizgi için `n` bayt kalmalıdır; kalmıyorsa `cbor_truncated`. Okuyucu bu yüzden dosyanın beyan ettiği uzunluğa göre hiçbir zaman dosyanın kendisinden büyük bellek ayırmaz.
- Yük tek bir öğedir; öğeden sonra bayt kalırsa `cbor_trailing`, öğe yükün sonunu aşarsa `cbor_truncated`.

### 5.6 Etiket yok

Profil hiçbir CBOR etiketi kullanmaz. ADR 0014 kimlikler için UUID etiketini (37) önermişti; kullanılmadı:

- Şema her alanın ne olduğunu zaten söyler: `uid` her zaman 16 baytlık UUID'dir, etiket yeni bilgi taşımaz.
- Etiket her nesneye 2 bayt ekler ve profile “bu etiket nerede geçerli” kuralını getirir.
- Genel CBOR araçları etiketsiz dosyayı da çözer; kimlik 16 baytlık bayt dizgisi olarak görünür.

## 6. Belge şeması 2–8

### 6.1 Kök

Yük, üç anahtarlı bir haritadır (anahtarlar kodlanmış sırasıyla):

| Anahtar | Tür | Değer |
|---|---|---|
| `format` | metin | `"kentos.document"`; değilse `schema_format` |
| `version` | tam sayı | `2`'den `64`'e bir sayı; değilse `schema_version` (`fixtures/kcad/v2/broken/schema-version-65.kcad`) |
| `document` | harita | belge (§6.2) |

`format` ve `version` sıralamada `document`'ten önce gelir: okuyucu belgenin kendisini okumadan şema sürümünü bilir.

**Şema 3**, şema 2'nin kendisi ve nesnenin kendi çizgi kalınlığıdır (`lineWeight`, §6.6; ADR 0139). Yazıcı `3`'ü **yalnız bir nesnenin kendi kalınlığı varken** yazar; başka her çizim şema 2'dir ve eskisiyle bayt bayt aynıdır, bu yüzden şema 2 okuyucusu onu açmaya devam eder. Şema 2 yükünde `lineWeight` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/line-weight-in-schema-2.kcad`).

**Şema 4**, şema 3'ün kendisi ve köşe kotlarıdır (çizgide `za`, `zb`; çoklu çizgide, alanda ve alanın deliğinde `zs`; §6.6; ADR 0142). Yazıcı `4`'ü **yalnız bir nesnede köşe kotu alanı varken** yazar: bir çizginin bir ucunun kotu, bir çoklu çizginin, alanın ya da alanın bir deliğinin `zs`'i (hepsi `null` olan bir `zs` de alandır). Başka her çizim şema 2 ya da 3'tür ve eskisiyle bayt bayt aynıdır, bu yüzden eski okuyucular kotsuz çizimi açmaya devam eder. Şema 2 ve 3 yükünde kot alanları bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/elevation-in-schema-2.kcad`, `elevation-in-schema-3.kcad`). Şema 4 şema 3'ü kapsar: nesnenin kendi kalınlığı orada da yazılır.

**Şema 5**, şema 4'ün kendisi ve çok parçalı alandır (`polygon`'un `parts`'ı, §6.6; ADR 0143). Yazıcı `5`'i **yalnız bir alanın `parts` alanı varken** yazar (boş bir `parts` da alandır). Başka her çizim şema 2, 3 ya da 4'tür ve eskisiyle bayt bayt aynıdır. Şema 2, 3 ve 4 yükünde `parts` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/parts-in-schema-4.kcad`): eski okuyucu çok parçalı alanı sessizce tek parçaya indirmez, dosyayı açmaz. Şema 5 şema 4'ü kapsar: köşe kotu ve nesne kalınlığı orada da yazılır.

**Şema 6**, şema 5'in kendisi ve bloklardır: belgenin `blocks` alanı (§6.2, §6.9) ve `insert` nesne türü (§6.6; ADR 0144). Yazıcı `6`'yı **yalnız çizimde bir blok tanımı varken** yazar; tanımsız bir yerleştirme zaten yazılamaz (`unknown_block`). Başka her çizim şema 2–5'tir ve eskisiyle bayt bayt aynıdır. Şema 2–5 yükünde `blocks` bilinmeyen alan (`unknown_field`, `fixtures/kcad/v2/broken/blocks-in-schema-5.kcad`), `insert` bilinmeyen türdür (`unknown_kind`, `insert-in-schema-5.kcad`): eski okuyucu blokları sessizce düşürmez, dosyayı açmaz. Şema 6 şema 5'i kapsar: parçalı alan, köşe kotu ve nesne kalınlığı orada da, tanımların nesnelerinde de yazılır.

**Şema 7**, şema 6'nın kendisi ve yazı ekleridir: yazının `align`, `widthFactor` ve `mask`'ı (§6.6), öznitelik tanımının `align` ve `widthFactor`'ı (§6.9; ADR 0145). Yazıcı `7`'yi **yalnız bir yazıda ya da öznitelik tanımında bu alanlardan biri varken** yazar, belgenin ya da bir blok tanımının yazısında. Başka her çizim şema 2–6'dır ve eskisiyle bayt bayt aynıdır. Şema 2–6 yükünde bu alanlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/text-align-in-schema-6.kcad`, `attribute-align-in-schema-6.kcad`): eski okuyucu hizalı yazıyı sessizce sol alt noktasına taşımaz, dosyayı açmaz. Şema 7 şema 6'yı kapsar.

**Şema 8**, şema 7'nin kendisi ve kılavuzdur: `leader` nesne türü (§6.6; ADR 0146). Yazıcı `8`'i **yalnız çizimde ya da bir blok tanımında kılavuz varken** yazar. Başka her çizim şema 2–7'dir ve eskisiyle bayt bayt aynıdır. Şema 2–7 yükünde `leader` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/leader-in-schema-7.kcad`): eski okuyucu kılavuzu sessizce düşürmez, dosyayı açmaz. Şema 8 şema 7'yi kapsar.

**Şema 9**, şema 8'in kendisi ve yeni ölçülerdir: ölçünün (`dimension`) beş yeni türü (`ordinate`, `arcLength`, `jogged`, `azimuth`, `slope`), zemini (`mask`) ve eğim ölçüsünün iki kotu (`za`, `zb`; §6.6; ADR 0147). Yazıcı `9`'u **yalnız belgenin ya da bir blok tanımının bir ölçüsünde bunlardan biri varken** yazar. Başka her çizim şema 2–8'dir ve eskisiyle bayt bayt aynıdır. Şema 2–8 yükünde yeni türler bilinmeyen değerdir (`bad_value`, `fixtures/kcad/v2/broken/dimension-ordinate-in-schema-8.kcad`), yeni alanlar bilinmeyen alandır (`unknown_field`, `dimension-mask-in-schema-8.kcad`): eski okuyucu yeni ölçüyü sessizce başka bir ölçü olarak çizmez, dosyayı açmaz. Şema 9 şema 8'i kapsar.

**Şema 10**, şema 9'un kendisi ve katmanın kendi kenetidir: katman düğümünün `snap` alanı (§6.5; ADR 0163 §4). Yazıcı `10`'u **yalnız bir katmanın `snap`'ı varken** yazar. Başka her çizim şema 2–9'dur ve eskisiyle bayt bayt aynıdır. Şema 2–9 yükünde `snap` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/layer-snap-in-schema-9.kcad`): eski okuyucu, yazarının kenedini kapattığı katmana sessizce kenetlenmez, dosyayı açmaz. Şema 10 şema 9'u kapsar.

**Şema 11**, şema 10'un kendisi ve yerel projenin çizim birimidir: proje ayarlarının `drawingUnit` alanı (§6.4; ADR 0165 §2). Yazıcı `11`'i **yalnız ayarlar bir birim adlandırırken** yazar (uygulamalar metreyi yazmaz). Başka her çizim şema 2–10'dur ve eskisiyle bayt bayt aynıdır. Şema 2–10 yükünde `drawingUnit` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/drawing-unit-in-schema-10.kcad`): eski okuyucu milimetreyle yazılıp okunan bir çizimin sayılarını sessizce metre diye göstermez, dosyayı açmaz. Geometri her şemada metrededir; birim yalnız yazılan ve gösterilen sayıların birimidir. Şema 11 şema 10'u kapsar.

**Şema 12**, şema 11'in kendisi ve projenin ikinci koordinat sistemidir: proje ayarlarının `secondSrid` alanı (§6.4; ADR 0167 §1). Yazıcı `12`'yi **yalnız projenin ikinci sistemi varken** yazar. Başka her çizim şema 2–11'dir ve eskisiyle bayt bayt aynıdır. Şema 2–11 yükünde `secondSrid` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/second-srid-in-schema-11.kcad`): eski okuyucu dosyayı açıp bir sonraki kayıtta ikinci sistemi sessizce düşürmez, dosyayı açmaz. Şema 12 şema 11'i kapsar.

**Şema 13**, şema 12'nin kendisi ve projenin kendi koordinat sistemleriyle datum seçimleridir: proje ayarlarının `customCrs`, `secondCustomCrs` ve `datumTransforms` alanları (§6.4, §6.4.1; ADR 0168). Yazıcı `13`'ü **yalnız bu alanlardan biri varken** yazar. Başka her çizim şema 2–12'dir ve eskisiyle bayt bayt aynıdır. Şema 2–12 yükünde bu alanlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/custom-crs-in-schema-12.kcad`): eski okuyucu kendi sistemi olan bir projeyi koordinat sistemi yokmuş gibi açmaz. Şema 13 şema 12'yi kapsar.

**Şema 14**, şema 13'ün kendisi ve projenin ölçme ayarlarıdır: proje ayarlarının `survey` alanı (§6.4, §6.4.2; ADR 0169 §3). Yazıcı `14`'ü **yalnız projenin ölçme ayarı varken** yazar. Başka her çizim şema 2–13'tür ve eskisiyle bayt bayt aynıdır. Şema 2–13 yükünde `survey` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/survey-in-schema-13.kcad`): eski okuyucu karneyi başka bir refraksiyon katsayısıyla ve toleranssız indirgemez, dosyayı açmaz. Şema 14 şema 13'ü kapsar.

**Şema 15**, şema 14'ün kendisi ve ölçme ayarlarının poligon toleranslarıdır: `survey`'in `twoWay`, `traverseAngle` ve `traverseCoord`'u (§6.4.2; ADR 0169 §3). Yazıcı `15`'i **yalnız bunlardan biri varken** yazar. Başka her çizim şema 2–14'tür ve eskisiyle bayt bayt aynıdır. Şema 14 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/survey-traverse-in-schema-14.kcad`): eski okuyucu poligonu toleranssız denetlemez, dosyayı açmaz. Şema 15 şema 14'ü kapsar.

**Şema 16**, şema 15'in kendisi ve ölçme ayarlarının zeminidir: `survey`'in `groundHeight` ve `reduceToGrid`'i (§6.4.2; ADR 0171 §2, §4). Yazıcı `16`'yı **yalnız bunlardan biri varken** yazar. Başka her çizim şema 2–15'tir ve eskisiyle bayt bayt aynıdır. Şema 15 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/survey-ground-in-schema-15.kcad`): eski okuyucu zemin değerlerini başka bir yükseklikte vermez, ölçülen uzunlukları düzleme indirmeden bırakmaz, dosyayı açmaz. Şema 16 şema 15'i kapsar. Örnek dosya `survey-ground.kcad`.

**Şema 17**, şema 16'nın kendisi ve çok parçalı çoklu çizgiyle çok noktalı nesnedir: `polyline`'ın ve `point`'in `parts`'ı (§6.6; ADR 0174). Yazıcı `17`'yi **yalnız belgenin ya da bir blok tanımının bir çoklu çizgisinin ya da noktasının `parts` alanı varken** yazar. Başka her çizim şema 2–16'dır ve eskisiyle bayt bayt aynıdır. Şema 2–16 yükünde çoklu çizginin ve noktanın `parts`'ı bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/polyline-parts-in-schema-16.kcad`, `point-parts-in-schema-16.kcad`): eski okuyucu çok parçalı çizgiyi sessizce ilk parçasına indirmez, dosyayı açmaz. Şema 17 şema 16'yı kapsar. Örnek dosya `multi-part-lines.kcad`.

**Şema 18**, şema 17'nin kendisi ve bir nesnenin etiketini yazan yazıdır: `text`'in `labelOf`'u ve `labelScale`'i (§6.6; ADR 0175 §4). Yazıcı `18`'i **yalnız belgenin bir yazısında bu alanlar varken** yazar. Başka her çizim şema 2–17'dir ve eskisiyle bayt bayt aynıdır. Şema 2–17 yükünde yazının `labelOf`'u ve `labelScale`'i bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/text-label-of-in-schema-17.kcad`): eski okuyucu bağlı yazıyı sessizce bağsız açmaz, dosyayı açmaz. Bir blok tanımının yazısında iki alan her şemada bilinmeyen alandır (`block-text-label-of.kcad`): tanımın nesnelerinin kalıcı kimliği yoktur. Şema 18 şema 17'yi kapsar. Örnek dosya `linked-texts.kcad`.

**Şema 19**, şema 18'in kendisi ve projenin adlı katman durumlarıdır: proje ayarlarının `layerStates`'i (§6.4, §6.4.3; ADR 0177 §4). Yazıcı `19`'u **yalnız projenin katman durumu varken** yazar. Başka her çizim şema 2–18'dir ve eskisiyle bayt bayt aynıdır. Şema 2–18 yükünde `layerStates` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/layer-states-in-schema-18.kcad`): eski okuyucu durumları açıp bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 19 şema 18'i kapsar. Örnek dosya `layer-states.kcad`.

**Şema 20**, şema 19'un kendisi ve çok satırlı yazıdır: `text`'in `boxWidth`'i, `lineSpacing`'i ve `runs`'ı (§6.6; ADR 0182 §1). Yazıcı `20`'yi **yalnız belgenin ya da bir blok tanımının bir yazısında bu alanlardan biri varken** yazar. Başka her çizim şema 2–19'dur ve eskisiyle bayt bayt aynıdır; satır sonu (`\n`) taşıyan metin her şemada metindir. Şema 2–19 yükünde üç alan bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/paragraph-in-schema-19.kcad`): eski okuyucu yazıyı biçimsiz açıp bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 20 şema 19'u kapsar. Örnek dosya `paragraphs.kcad`.

**Şema 21**, şema 20'nin kendisi ve yazı ve ölçü stilleridir: proje ayarlarının `textStyles`'ı ve `dimensionStyles`'ı (§6.4, §6.4.4), `text`'in `textStyle`, `font`, `bold`, `italic`, `oblique`'i ve `dimension`'ın `dimStyle`, `arrow`, `arrowSize`, `extOffset`, `extBeyond`, `textGap`, `textPlace`, `decimals`, `unit`, `prefix`, `suffix`, `font`'u (§6.6; ADR 0183). Yazıcı `21`'i **yalnız projenin bir stili ya da belgenin veya bir blok tanımının bir yazısında ya da ölçüsünde bu alanlardan biri varken** yazar. Başka her çizim şema 2–20'dir ve eskisiyle bayt bayt aynıdır. Şema 2–20 yükünde bu alanlar bilinmeyen alandır (`unknown_field`; `fixtures/kcad/v2/broken/style-table-in-schema-20.kcad`, `style-face-in-schema-20.kcad`, `style-look-in-schema-20.kcad`): eski okuyucu stilleri ve görünüşleri düşürüp bir sonraki kayıtta sessizce silmez, dosyayı açmaz. Şema 21 şema 20'yi kapsar. Örnek dosya `styles.kcad`.

**Şema 22**, şema 21'in kendisi ve tablodur: `table` nesne türü (§6.6; ADR 0184). Tablo yalnız belgenin nesnesidir: şema 22'de bir blok tanımında `bad_value`'dur (`fixtures/kcad/v2/broken/table-in-block.kcad`). Yazıcı `22`'yi **yalnız çizimde tablo varken** yazar. Başka her çizim şema 2–21'dir ve eskisiyle bayt bayt aynıdır. Şema 2–21 yükünde `table` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/table-in-schema-21.kcad`): eski okuyucu tabloyu sessizce düşürmez, dosyayı açmaz. Şema 22 şema 21'i kapsar. Örnek dosya `tables.kcad`.

**Şema 23**, şema 22'nin kendisi ve tarama ekleridir (ADR 0186): taramanın `pattern`'inde `type` `pattern` ve `gradient`, `name`, `scale`, `lines` ve `gradient` alanları; taramanın `assoc`'u (§6.6). Desen ve degrade belgede ve blok tanımında olabilir; `assoc` yalnız belgenin taramasındadır (blok tanımının nesnelerinin kalıcı kimliği yoktur): şema 23'te blok tanımında bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/hatch-assoc-in-block.kcad`). Yazıcı `23`'ü **yalnız deseni çizgi aileli ya da degrade olan (ya da onların bir alanı olan) ya da nesnelere bağlı bir tarama varken** yazar, belgede ya da bir blok tanımında. Başka her çizim şema 2–22'dir ve eskisiyle bayt bayt aynıdır. Şema 2–22 yükünde `pattern` ve `gradient` türleri bilinmeyen değer (`bad_value`, `broken/hatch-pattern-in-schema-22.kcad`), yeni alanlar bilinmeyen alandır (`unknown_field`, `broken/hatch-name-in-schema-22.kcad`, `broken/hatch-assoc-in-schema-22.kcad`): eski okuyucu başka bir desen çizmez, dosyayı açmaz. Şema 23 şema 22'yi kapsar. Örnek dosya `hatches.kcad`.

**Şema 24**, şema 23'ün kendisi ve resimdir: `image` nesne türü (§6.6; ADR 0192). Resim yalnız belgenin nesnesidir: şema 24'te bir blok tanımında `bad_value`'dur (`fixtures/kcad/v2/broken/image-in-block.kcad`). Yazıcı `24`'ü **yalnız çizimde resim varken** yazar. Başka her çizim şema 2–23'tür ve eskisiyle bayt bayt aynıdır. Şema 2–23 yükünde `image` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/image-in-schema-23.kcad`): eski okuyucu resmi sessizce düşürmez, dosyayı açmaz. Şema 24 şema 23'ü kapsar. Gömülü resmin baytları projenin stillerinde (§6.7) opak bir öğedir; dosya biçimi onları yorumlamaz. Örnek dosya `images.kcad`.

**Şema 25**, şema 24'ün kendisi ve eğri boyunca yazıdır: yazının `path` alanı (§6.6; ADR 0196), belgede ve blok tanımında. Yazıcı `25`'i **yalnız bir yazının eğrisi varken** yazar. Başka her çizim şema 2–24'tür ve eskisiyle bayt bayt aynıdır. Şema 2–24 yükünde `path` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/text-path-in-schema-24.kcad`): eski okuyucu yazıyı sessizce düzleştirmez, dosyayı açmaz. Şema 25 şema 24'ü kapsar. Örnek dosya `text-paths.kcad`.

**Şema 26**, şema 25'in kendisi ve katmanın alanlarıdır: katman düğümünün `fields` alanı (§6.5; ADR 0199 §1). Yazıcı `26`'yı **yalnız bir katmanın alanı varken** yazar. Başka her çizim şema 2–25'tir ve eskisiyle bayt bayt aynıdır. Şema 2–25 yükünde `fields` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/layer-fields-in-schema-25.kcad`): eski okuyucu katmanın şemasını bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 26 şema 25'i kapsar. Örnek dosya `layer-fields.kcad`.

**Şema 27**, şema 26'nın kendisi ve projenin topoloji kurallarıdır: proje ayarlarının `topology` alanı (§6.4, §6.4.5; ADR 0202 §7). Yazıcı `27`'yi **yalnız projenin topoloji ayarı varken** yazar. Başka her çizim şema 2–26'dır ve eskisiyle bayt bayt aynıdır. Şema 2–26 yükünde `topology` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/topology-in-schema-26.kcad`): eski okuyucu kuralları ve istisnaları bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 27 şema 26'yı kapsar. Örnek dosya `topology.kcad`.

**Şema 28**, şema 27'nin kendisi ve ölçme ayarlarının önsel doğruluklarıdır: `survey`'in `sigmaDirection`, `sigmaDistance`, `sigmaPpm`, `sigmaCentering`, `sigmaZenith` ve `sigmaLevelling`'i (§6.4.2; ADR 0203 §1). Yazıcı `28`'i **yalnız bunlardan biri varken** yazar. Başka her çizim şema 2–27'dir ve eskisiyle bayt bayt aynıdır. Şema 27 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/survey-sigma-in-schema-27.kcad`): eski okuyucu ağ dengelemesini başka ağırlıklarla yapmaz, dosyayı açmaz. Şema 28 şema 27'yi kapsar. Örnek dosya `survey-sigmas.kcad`.

**Şema 29**, şema 28'in kendisi ve rasterdir: `raster` nesne türü (§6.6; ADR 0204 §2). Raster yalnız belgenin nesnesidir: şema 29'da bir blok tanımında `bad_value`'dur (`fixtures/kcad/v2/broken/raster-in-block.kcad`). Yazıcı `29`'u **yalnız çizimde raster varken** yazar. Başka her çizim şema 2–28'dir ve eskisiyle bayt bayt aynıdır. Şema 2–28 yükünde `raster` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/raster-in-schema-28.kcad`): eski okuyucu rasteri sessizce düşürmez, dosyayı açmaz. Şema 29 şema 28'i kapsar. Rasterin pikselleri dosyaya yazılmaz; gömülü rasterin baytları projenin stillerinde (§6.7) opak bir öğedir. Örnek dosya `rasters.kcad`.

**Şema 30**, şema 29'un kendisi ve yazı yükseklikleri, ölçü çizgileri ve kılavuz oklarıdır (ADR 0205): proje ayarlarının `annotation` alanı (§6.4, §6.4.6), ölçü stillerinin ve `dimension`'ın `dimLineColor`, `dimLineWeight`, `dimLineType`, `extColor`, `extWeight`, `extLineType`, `textColor`'ı (§6.4.4, §6.6), `leader`'ın `arrowSize`'ı ve `arrow`'un AutoCAD'den gelen on değeri (§6.6). Yazıcı `30`'u **yalnız projenin yazı yüksekliği ya da bir ölçü stilinin çizgisi varken ya da belgenin veya bir blok tanımının bir ölçüsünde çizgi alanı, bir kılavuzunda ok boyu ya da bu on oktan biri varken** yazar. Başka her çizim şema 2–29'dur ve eskisiyle bayt bayt aynıdır. Şema 2–29 yükünde bu alanlar bilinmeyen alandır (`unknown_field`; `fixtures/kcad/v2/broken/annotation-in-schema-29.kcad`, `dimension-lines-in-schema-29.kcad`, `dimension-style-lines-in-schema-29.kcad`, `leader-arrow-size-in-schema-29.kcad`), on ok bilinmeyen değerdir (`bad_value`, `leader-arrow-closed-in-schema-29.kcad`): eski okuyucu yükseklikleri, çizgileri ve okları bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 30 şema 29'u kapsar. Örnek dosya `annotation.kcad`.

**Şema 31**, şema 30'un kendisi ve nokta bulutlarıdır (ADR 0207): `pointcloud` nesne türü (§6.6) ve rasterin adresi, `url` (§6.6). Nokta bulutu yalnız belgenin nesnesidir: şema 31'de bir blok tanımında `bad_value`'dur (`fixtures/kcad/v2/broken/pointcloud-in-block.kcad`). Yazıcı `31`'i **yalnız çizimde nokta bulutu ya da adresten okunan raster varken** yazar. Başka her çizim şema 2–30'dur ve eskisiyle bayt bayt aynıdır. Şema 2–30 yükünde `pointcloud` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/pointcloud-in-schema-30.kcad`), rasterin `url`'si bilinmeyen alandır (`unknown_field`, `raster-url-in-schema-30.kcad`): eski okuyucu bulutu ya da adresi sessizce düşürmez, dosyayı açmaz. Şema 31 şema 30'u kapsar. Bulutun noktaları dosyaya yazılmaz; gömülü bulutun baytları projenin stillerinde (§6.7) opak bir öğedir. Örnek dosya `pointclouds.kcad`.

**Şema 32**, şema 31'in kendisi ve harita servisleridir (ADR 0208): katmanın `service`'i (katman bir harita servisinden çizilir: XYZ, WMS, WMTS, OGC API Tiles, ArcGIS, Google, vektör karolar) ve `feed`'i (katmanın nesneleri bir servisten ya da adresten alındı: WFS, OGC API Features, ArcGIS, GeoJSON) (§6.5), proje ayarlarının `connections`'ı (servislerin kimlik doğrulaması, sırrı olmadan; §6.4.7). Yazıcı `32`'yi **yalnız servisten çizilen ya da nesneleri bir kaynaktan alınan katman ya da projenin bağlantısı varken** yazar. Başka her çizim şema 2–31'dir ve eskisiyle bayt bayt aynıdır. Şema 2–31 yükünde üçü de bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/service-in-schema-31.kcad`, `feed-in-schema-31.kcad`, `connections-in-schema-31.kcad`): eski okuyucu servis katmanını boş bir katman sanıp bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 32 şema 31'i kapsar. Karolar, servisin cevapları ve sırlar dosyaya yazılmaz (§6.10). Örnek dosya `services.kcad`.

**Şema 33**, şema 32'nin kendisi ve projenin ağlarıdır (ADR 0209): proje ayarlarının `networks`'ü (§6.4, §6.4.8): hangi çizgi katmanlarının kenar, hangi nokta katmanlarının düğüm olduğu, bağlanma kuralı ve toleransı, yön, maliyetler ve kısıtlar. Yazıcı `33`'ü **yalnız projenin ağı varken** yazar. Başka her çizim şema 2–32'dir ve eskisiyle bayt bayt aynıdır. Şema 2–32 yükünde `networks` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/networks-in-schema-32.kcad`): eski okuyucu ağ tanımlarını bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 33 şema 32'yi kapsar. Ağın grafı dosyaya yazılmaz: her analiz çizimin nesnelerinden kurar. Örnek dosya `networks.kcad`.

**Şema 34**, şema 33'ün kendisi, zamansal katmanlar ve senaryolardır (ADR 0210): katman düğümünün `time`'ı (katmanın nesnelerinin başlangıç ve bitiş öznitelikleri, kimlik alanı, Birikimli), grubun `scenario`'su (grup bir senaryodur) ve senaryo katmanının `replaces`'i (yerine geçtiği ana katman) (§6.5). Yazıcı `34`'ü **yalnız bir düğümde bunlardan biri varken** yazar. Başka her çizim şema 2–33'tür ve eskisiyle bayt bayt aynıdır. Şema 2–33 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/time-in-schema-33.kcad`). Şema 34 şema 33'ü kapsar. Nesnelerin zamanı özniteliklerinde kalır; dosya biçimi değerleri okumaz ve denetlemez. Örnek dosya `scenarios.kcad`.

**Şema 35**, şema 34'ün kendisi ve katman süzgecidir (ADR 0211): katman düğümünün `filter`'ı (katmanın yalnız bir ifadeye ya da nesne listesine uyan nesneleri gösterilir, seçilir ve işlemlere girer) (§6.5). Yazıcı `35`'i **yalnız bir katmanın süzgeci varken** yazar. Başka her çizim şema 2–34'tür ve eskisiyle bayt bayt aynıdır. Şema 2–34 yükünde bu anahtar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/filter-in-schema-34.kcad`). Şema 35 şema 34'ü kapsar. Süzgeç görünümdür: dosya bütün nesneleri yazar; ifadenin derlenmesi dosya biçiminin kuralı değildir. Örnek dosya `filters.kcad`.

**Şema 36**, şema 35'in kendisi ve etiket motorudur (ADR 0212): etiket stilinin motor anahtarları (metnin ifadesi, rengi, yerleşim kipleri, hale, zemin, gölge, çağrı çizgisi, yığma, kısaltma, küçültme, öncelik, çakışma, yinelenenler), katman stilinin `labels`'ı (katmanın etiketlemesi: tek etiket, kurallı sınıflar ya da etiketsiz; engel) (§6.5) ve nesnenin `labelPins`'i (elle taşınmış, döndürülmüş ya da gizlenmiş etiketleri; §6.6). Yazıcı `36`'yı **yalnız bunlardan biri varken** yazar; motor anahtarı olmayan etiket stili eskisiyle bayt bayt aynıdır. Başka her çizim şema 2–35'tir. Şema 2–35 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/label-engine-in-schema-35.kcad`, `labels-in-schema-35.kcad`, `label-pins-in-schema-35.kcad`): eski okuyucu etiketlemeyi sessizce bugünkü etikete indirmez, dosyayı açmaz. Şema 36 şema 35'i kapsar. Etiketlerin yerleşimi dosyaya yazılmaz (çizimin işidir); yalnız iğneler yazılır. İfadelerin derlenmesi dosya biçiminin kuralı değildir: komutlar derler. Örnek dosya `labels.kcad`.

**Şema 37**, şema 36'nın kendisi ve projenin değişkenleridir (ADR 0214 §2.3): proje ayarlarının `variables`'ı (§6.4, §6.4.9): ifade dilinin `@ad` değerleri (iş numarası, müşteri, katsayı …), adları, etiketleri, türleri ve değerleriyle. Yazıcı `37`'yi **yalnız projenin değişkeni varken** yazar. Başka her çizim şema 2–36'dır ve eskisiyle bayt bayt aynıdır. Şema 2–36 yükünde `variables` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/variables-in-schema-36.kcad`): eski okuyucu değişkenleri bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 37 şema 36'yı kapsar. Yerleşik değişkenler (projenin adı, koordinat sistemi, ölçeği, tarih, kullanıcı) dosyaya yazılmaz: çağıran verir. Örnek dosya `variables.kcad`.

**Şema 38**, şema 37'nin kendisi ve çok boyutlu rasterlerdir (ADR 0243): rasterin `dataset`'i (gösterdiği NetCDF değişkeni ve dilimi; §6.6) ve görünüşünün `edges`'i (mesh'in çizgilerinin rengi; §6.6). Yazıcı `38`'i **yalnız veri setli ya da ağ çizgili bir raster varken** yazar. Başka her çizim şema 2–37'dir ve eskisiyle bayt bayt aynıdır. Şema 2–37 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/dataset-in-schema-37.kcad`, `edges-in-schema-37.kcad`): eski okuyucu veri setini sessizce düşürüp dosyanın ilk bandını göstermez, dosyayı açmaz. Şema 38 şema 37'yi kapsar. Değişkenin değerleri dosyaya yazılmaz; yalnız adı, dilimi ve boyutların değerleri yazılır. Örnek dosya `multidim.kcad`.

**Şema 39**, şema 38'in kendisi ve köşe ölçüleridir (M; ADR 0224 §1): çizginin `ma` ve `mb`'si, çoklu çizginin ve parçalarının `ms`'i (§6.6). Yazıcı `39`'u **yalnız çizimin ya da bir blok tanımının bir nesnesinde ölçü alanı varken** yazar (hepsi `null` olan bir `ms` de alandır). Başka her çizim şema 2–38'dir ve eskisiyle bayt bayt aynıdır. Şema 2–38 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/measures-in-schema-38.kcad`): eski okuyucu rotanın ölçülerini sessizce düşürmez, dosyayı açmaz. Alanın ölçüsü yoktur: alanda `ms` her şemada bilinmeyen alandır (`broken/measures-in-polygon.kcad`). Şema 39 şema 38'i kapsar. Örnek dosya `measures.kcad`.

**Şema 40**, şema 39'un kendisi ve nesne ekleridir (ADR 0225): nesnenin `attachments`'ı (fotoğrafları ve belgeleri; §6.6), katmanın `actions`'ı (nesnelerinin açtığı adresler ve dosyalar) ve `card`'ı (Bilgi aracının gösterdiği kart; §6.5). Yazıcı `40`'ı **yalnız çizimin bir nesnesinin eki ya da bir katmanın eylemi veya kartı varken** yazar (içi boş kart da alandır). Başka her çizim şema 2–39'dur ve eskisiyle bayt bayt aynıdır. Şema 2–39 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/attachments-in-schema-39.kcad`, `actions-in-schema-39.kcad`, `card-in-schema-39.kcad`): eski okuyucu ekleri sessizce düşürmez, dosyayı açmaz. Şema 40 şema 39'u kapsar. Gömülü fotoğrafın baytları proje stillerinin (§6.7) resim öğesindedir, ek yalnız kimliğini yazar; bağlı dosyanın içeriği dosyaya girmez. Şablonların (`{…}` arası ifadeler) derlenmesi dosya biçiminin kuralı değildir: komutlar derler. Örnek dosya `attachments.kcad`.

**Şema 41**, şema 40'ın kendisi ve katman çizimidir (ADR 0226): katmanın `compose`'u (saydamlığı, karışım kipi ve maskesi; §6.5). Yazıcı `41`'i **yalnız bir katmanın katman çizimi varken** yazar. Başka her çizim şema 2–40'tır ve eskisiyle bayt bayt aynıdır. Şema 2–40 yükünde `compose` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/compose-in-schema-40.kcad`): eski okuyucu katman çizimini sessizce düşürmez, dosyayı açmaz. Şema 41 şema 40'ı kapsar. Sembol katmanlarının geometri üreticisi ve çizim efektleri sembolün JSON'undadır (proje stillerinde ve işleyicide opak), şemayı değiştirmez. Örnek dosya `compose.kcad`.

**Şema 42**, şema 41'in kendisi ve projenin adres verisidir (ADR 0229 §2): proje ayarlarının `addressLocators`'ı (§6.4, §6.4.10): kapı noktalarının, yolların ve mahalle, ilçe ve il alanlarının katmanları ve alanları, en düşük eşleşme puanı ve Konumdan adres'in uzaklığı. Yazıcı `42`'yi **yalnız projenin adres verisi varken** yazar. Başka her çizim şema 2–41'dir ve eskisiyle bayt bayt aynıdır. Şema 2–41 yükünde `addressLocators` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/addresses-in-schema-41.kcad`): eski okuyucu adres verisini bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 42 şema 41'i kapsar. Adres arama dizini dosyaya yazılmaz: her arama çizimin nesnelerinden kurar; çevrimiçi sağlayıcıların ayarları ve anahtarları cihazdadır, dosyaya girmez. Örnek dosya `addresses.kcad`.

**Şema 43**, şema 42'nin kendisi ve ilişkili ölçüdür (ADR 0249 §4): ölçünün `refs`'i (§6.6), `a`, `b` ve `c` noktalarının izlediği nesne yerleri. `refs` yalnız belgenin ölçüsündedir (blok tanımının nesnelerinin kalıcı kimliği yoktur): şema 43'te blok tanımında bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/dimension-refs-in-block.kcad`). Yazıcı `43`'ü **yalnız bir ölçünün başvurusu varken** yazar. Başka her çizim şema 2–42'dir ve eskisiyle bayt bayt aynıdır. Şema 2–42 yükünde `refs` bilinmeyen alandır (`unknown_field`, `broken/dimension-refs-in-schema-42.kcad`): eski okuyucu ölçünün bağlarını bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 43 şema 42'yi kapsar. Örnek dosya `dimension-refs.kcad`.

**Şema 44**, şema 43'ün kendisi ve projenin pafta kitabıdır (ADR 0251): belgenin `sheets`'i (§6.12), pafta çekirdeğinin kitabı (paftalar, ana sayfalar, değişkenler, resimlerin bilgileri) ve resimlerin baytları. Yazıcı `44`'ü **yalnız projenin pafta kitabı varken** yazar. Başka her çizim şema 2–43'tür ve eskisiyle bayt bayt aynıdır. Şema 2–43 yükünde `sheets` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/sheets-in-schema-43.kcad`): eski okuyucu paftaları bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 44 şema 43'ü kapsar. Örnek dosya `sheets.kcad`.

**Şema 45**, şema 44'ün kendisi ve istek defteridir (ADR 0253 §6): belgenin `requests`'i (§6.13), çizimin yürüttüğü isteklerin son 256'sı: kimliği, çağrının parmak izi, komut ve sürümü, sonraki sürüm, durumu ve o zamanki cevabı. Aynı istek yeniden gelince iki kez uygulanmaz; “uygulandı, kaydedildi, cevap gitmeden süreç çöktü” durumunda açılan dosya isteği bilir. Yazıcı `45`'i **yalnız çizim bir istek yürüttüyse** yazar. Başka her çizim şema 2–44'tür ve eskisiyle bayt bayt aynıdır. Şema 2–44 yükünde `requests` bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/requests-in-schema-44.kcad`): eski okuyucu defteri bir sonraki kayıtta sessizce düşürmez, dosyayı açmaz. Şema 45 şema 44'ü kapsar. Örnek dosya `requests.kcad`.

**Şema 46**, şema 45'in kendisi ve kapalı ve genişlikli çoklu çizgidir (ADR 0258): çoklu çizginin `closed`'ı (son köşesi ilkine bir kenarla bağlanan kapalı çizgi; alan değildir) ve `widths`'i (her köşeden çıkan kenarın çizim uzunluğundaki başlangıç ve bitiş genişliği; kâğıdın çizgi kalınlığı değildir; §6.6). Yazıcı `46`'yı **yalnız çizimin ya da bir blok tanımının bir çoklu çizgisi kapalı ya da genişlikliyken** yazar. Başka her çizim şema 2–45'tir ve eskisiyle bayt bayt aynıdır. Şema 2–45 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/polyline-closed-in-schema-45.kcad`, `polyline-widths-in-schema-45.kcad`): eski okuyucu kapalı çizgiyi açık, genişlikli çizgiyi ince açmaz, dosyayı açmaz. Alanın ikisi de yoktur: alanda her şemada bilinmeyen alandır (`broken/polygon-closed.kcad`, `polygon-widths.kcad`). Şema 46 şema 45'i kapsar. Örnek dosya `polyline-extras.kcad`.

**Şema 47**, şema 46'nın kendisi ve NURBS eğrisidir (ADR 0259): `nurbs` nesne türü (§6.6). Yazıcı `47`'yi **yalnız çizimde ya da bir blok tanımında NURBS eğrisi varken** yazar. Başka her çizim şema 2–46'dır ve eskisiyle bayt bayt aynıdır. Şema 2–46 yükünde `nurbs` bilinmeyen türdür (`unknown_kind`, `fixtures/kcad/v2/broken/nurbs-in-schema-46.kcad`): eski okuyucu eğriyi sessizce düşürmez, dosyayı açmaz. Şema 47 şema 46'yı kapsar. Örnek dosya `nurbs.kcad`.

**Şema 48**, şema 47'nin kendisi ve nesnenin görünüşüdür (ADR 0260): nesnenin `lineType`, `lineTypeScale`, `transparency`'si, çizimin nesnesinin `order`'ı, bir blok tanımının nesnesinin `byLayer`'ı (§6.6) ve katmanın `plot`'u (§6.5). Yazıcı `48`'i **yalnız bunlardan biri varken** yazar. Başka her çizim şema 2–47'dir ve eskisiyle bayt bayt aynıdır. Şema 2–47 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/look-in-schema-47.kcad`): eski okuyucu kesik çizgiyi sürekli, saydam nesneyi donuk, yazdırılmayan katmanı yazdırılır açmaz, dosyayı açmaz. Şema 48 şema 47'yi kapsar. Örnek dosya `look.kcad`.

**Şema 49**, şema 48'in kendisi ve kullanıcı koordinat sistemidir (ADR 0261): ayarların `ucs`'i ve `namedUcs`'i (§6.4.11). Yazıcı `49`'u **yalnız geçerli UCS dünya değilken ya da adlı UCS varken** yazar. Başka her çizim şema 2–48'dir ve eskisiyle bayt bayt aynıdır. Şema 2–48 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `fixtures/kcad/v2/broken/ucs-in-schema-48.kcad`): eski okuyucu döndürülmüş bir eksenle kaydedilmiş çizimi sessizce dünyada açmaz, dosyayı açmaz. Şema 49 şema 48'i kapsar. Örnek dosya `ucs.kcad`.

**Şema 50**, şema 49'un kendisi ve ayak ile inçtir (ADR 0262): ayarların `drawingUnit`'inin `in` ve `ft` değerleri ve `lengthStyle` (`architectural`, `engineering`, `fractional`; ondalık yazılmaz, yalnız `in` ya da `ft` birimli projede; değilse `bad_value`: `broken/length-style-decimal.kcad`, `length-style-metric.kcad`). Yazıcı `50`'yi **yalnız birim inç ya da ayakken ya da biçim varken** yazar. Başka her çizim şema 2–49'dur ve eskisiyle bayt bayt aynıdır. Şema 2–49 yükünde `in` bilinmeyen birimdir (`bad_value`, `broken/drawing-unit-in-in-schema-49.kcad`), `lengthStyle` bilinmeyen alandır (`unknown_field`, `broken/length-style-in-schema-49.kcad`): eski okuyucu inç çizimini metrede açmaz. Şema 50 şema 49'u kapsar. Örnek dosya `imperial.kcad`.

**Şema 51**, şema 50'nin kendisi ve eğri üzerindeki yer ile blok içi alt nesnedir (ADR 0263): ölçünün başvurusunun `along` yeri (kenar ya da eğri boyunca, `t` ondalığıyla) ve `path`'i (yerleştirmenin içindeki parçanın yolu; §6.6). Yazıcı `51`'i **yalnız bir başvurunun `along` yeri ya da yolu varken** yazar. Başka her çizim şema 2–50'dir ve eskisiyle bayt bayt aynıdır. Şema 2–50 yükünde `t` ve `path` bilinmeyen alandır (`unknown_field`, `broken/dimension-refs-along-in-schema-50.kcad`, `dimension-refs-path-in-schema-50.kcad`): eski okuyucu noktanın izlediği yeri sessizce düşürmez. Şema 51 şema 50'yi kapsar. Örnek dosya `subref-extras.kcad`.

**Şema 52**, şema 51'in kendisi ve adlı görünümlerdir (ADR 0285): ayarların `namedViews`'i (§6.4.12). Yazıcı `52`'yi **yalnız projede adlı görünüm varken** yazar. Başka her çizim şema 2–51'dir ve eskisiyle bayt bayt aynıdır. Şema 2–51 yükünde `namedViews` bilinmeyen alandır (`unknown_field`, `broken/views-in-schema-51.kcad`): eski okuyucu görünümleri sessizce düşürmez. Şema 52 şema 51'i kapsar. Örnek dosya `views.kcad`.

**Şema 53**, şema 52'nin kendisi ve ilişkili dizilerdir (ADR 0300): belgenin `arrays`'i (§6.14). Yazıcı `53`'ü **yalnız çizimde ilişkili dizi varken** yazar. Başka her çizim şema 2–52'dir ve eskisiyle bayt bayt aynıdır. Şema 2–52 yükünde `arrays` bilinmeyen alandır (`unknown_field`, `broken/arrays-in-schema-52.kcad`): eski okuyucu diziyi sessizce bağımsız yerleştirmelere çevirmez, dosyayı açmaz. Şema 53 şema 52'yi kapsar. Örnek dosya `arrays.kcad`.

**Şema 55**, şema 54'ün kendisi ve dondurmadır (ADR 0311 §1, §3): katmanın ya da grubun `frozen`'ı (§6.5; yalnız `true`) ve katman durumu düğümünün `frozen` ve `plot`'u (§6.4.3). Yazıcı `55`'i **yalnız bir düğüm dondurulmuşken ya da bir durum dondurmayı ve yazdırmayı saklarken** yazar. Başka her çizim şema 2–54'tür ve eskisiyle bayt bayt aynıdır. Şema 2–54 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `broken/frozen-in-schema-54.kcad`, `broken/layer-states-frozen-in-schema-54.kcad`): eski okuyucu dondurulmuş katmanı çizip seçmez, dosyayı açmaz. Şema 55 şema 54'ü kapsar. Örnek dosya `frozen.kcad`.

**Şema 56**, şema 55'in kendisi ve desenli çizgi tipleridir (ADR 0313 §1): ayarlarda `lineTypes` ve `lineTypeScale` (§6.4), katman stilinde ve katman durumunun stilinde `linePattern` (§6.5, §6.4.3), nesnede ve blok nesnesinde `linePattern` (§6.6), sütunlarda görünüşün desen biti. Yazıcı `56`'yı **yalnız projede çizgi tipi ya da çizgi tipi ölçeği varken ya da bir katman, durum ya da nesne bir deseni anarken** yazar. Başka her çizim şema 2–55'tir ve eskisiyle bayt bayt aynıdır. Şema 2–55 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `broken/patterns-in-schema-55.kcad`, `broken/line-type-scale-in-schema-55.kcad`). Şema 56 şema 55'i kapsar. Örnek dosya `patterns.kcad`.

**Şema 54**, şema 53'ün kendisi ve alanın maskesidir (ADR 0310): alanın (`polygon`) `wipeout`'u (§6.6), AutoCAD'in WIPEOUT'u. Yazıcı `54`'ü **yalnız çizimin ya da bir blok tanımının bir alanı maskeyken** yazar. Başka her çizim şema 2–53'tür ve eskisiyle bayt bayt aynıdır. Şema 2–53 yükünde `wipeout` bilinmeyen alandır (`unknown_field`, `broken/wipeouts-in-schema-53.kcad`): eski okuyucu maskeyi sıradan alan gibi açmaz, dosyayı açmaz. Şema 54 şema 53'ü kapsar. Örnek dosya `wipeouts.kcad`.

**Şema 60**, şema 59'un kendisi ve ölçünün başvurularının kuralıdır (ADR 0320 §1): ölçünün `refs`'inde `rule` (`curve`: yarıçap, çap, kırıklı yarıçap, yay uzunluğu ve yaydan açının merkezi eğrisinden; `edges`: kenarlardan açının köşesi iki kenarın kesişiminden), sütunlarda başvuru maskesinin 3. ve 4. bitleri. Yazıcı `60`'ı **yalnız bir ölçünün kuralı varken** yazar. Başka her çizim şema 2–59'dur ve eskisiyle bayt bayt aynıdır. Şema 2–59 yükünde `rule` bilinmeyen alandır (`unknown_field`, `broken/dimension-rule-in-schema-59.kcad`): eski okuyucu yarıçapın merkezini ya da açının köşesini geride bırakmaz, dosyayı açmaz. Şema 60 şema 59'u kapsar. Örnek dosya `dimension-rules.kcad`.

**Şema 61**, şema 60'ın kendisi ve ölçünün seçenekleridir (ADR 0321): ölçünün ve ölçü stilinin `options`'ı (`fit`, `round`, `zeros`, `lengthStyle`, `alt`, `tolerance`, `breakSize`), ölçünün `textShift`'i (`along`, `across`) ve `breaks`'i (`line`: `dimension`, `first`, `second`; `t` 0–1, en çok 64); sözleşmenin JSON'u gibi, varsayılanlar yazılmadan, kanonik anahtar sırasıyla; sütunlarda ölçünün 29. biti ve bir JSON metni. Yazıcı `61`'i **yalnız bunlardan biri varken** yazar. Başka her çizim şema 2–60'tır ve eskisiyle bayt bayt aynıdır. Şema 2–60 yükünde bu anahtarlar bilinmeyen alandır (`unknown_field`, `broken/dimension-options-in-schema-60.kcad`, `broken/dimension-text-shift-in-schema-60.kcad`); boş seçenekler, sıfır artış, alt değerli simetrik tolerans ve 1'i aşan `t` `bad_value`'dur. Şema 61 şema 60'ı kapsar. Örnek dosya `dimension-options.kcad`.

**Şema 62**, şema 61'in kendisi ve ölçünün işaret türleridir (ADR 0322 §1): `style`'ın `centerMark`, `centerLine` ve `elevation`'ı (kotun `za`'sı zorunlu, `zb`'si yok) ve başvuruların `rule`'ünün `pair`'i; sütunlarda türler 10–12, kural maskesinin 3. ve 4. bitleri birlikte. Yazıcı `62`'yi **yalnız bunlardan biri varken** yazar. Şema 2–61 yükünde bu türler ve kural bilinmeyen değerdir (`bad_value`, `broken/dimension-center-mark-in-schema-61.kcad`, `broken/dimension-rule-pair-in-schema-61.kcad`); kotsuz kot `missing_field`, `zb`'li kot `bad_value`'dur. Örnek dosya `dimension-marks.kcad`.

**Şema 63**, şema 62'nin kendisi ve stilden ayrılan alanlardır (ADR 0322 §3): yazının ve ölçünün `overrides`'ı (alan adlarının listesi, JSON'u gibi; yazının `font`, `bold`, `italic`, `oblique`, `widthFactor`, `height`; ölçünün görünüş alanları ve `height`); boş liste yazılmaz; sütunlarda yazının 14. seçeneği ve ölçünün “more” JSON'u. Yazıcı `63`'ü **yalnız böyle bir liste varken** yazar. Şema 2–62 yükünde `overrides` bilinmeyen alandır (`unknown_field`, `broken/dimension-overrides-in-schema-62.kcad`); boş liste, türün alanı olmayan ad ve iki kez geçen ad `bad_value`'dur. Örnek dosya `overrides.kcad`.

**Şema 64**, şema 63'ün kendisi ve ölçekli açıklamalardır (ADR 0323): yazının, ölçünün ve kılavuzun `annotative`'i, JSON'u gibi bir harita: `at` (nesnenin kendi geometrisinin ölçeği, 1:N'nin N'si, sıfırdan büyük, en çok 1 000 000) ve isteğe bağlı `scales` (diğer ölçekler küçükten büyüğe, birer kez, `at` olmadan, en çok 32; boş liste yazılmaz); her öğe `scale` ve o ölçeğin ayrı değerleri: yazı ve kılavuzda `point` (`{x, y}`) ve `rotation`, ölçüde `offset` ve `textShift` (`{along, across}`). Eğri boyunca ve nesneye bağlı yazı ölçekli olmaz. Sütunlarda yazının 15. seçeneği, kılavuzun 4. seçeneği (JSON metni) ve ölçünün “more” JSON'u. Yazıcı `64`'ü **yalnız ölçekli bir nesne varken** yazar. Şema 2–63 yükünde `annotative` bilinmeyen alandır (`unknown_field`, `broken/annotative-in-schema-63.kcad`); sıfır ölçek, yinelenen kendi ölçeği, sırasız ölçekler ve türün almadığı değer `bad_value`'dur (`annotative-scale-zero`, `-own-repeated`, `-not-ascending`, `-dimension-point`). Örnek dosya `annotative.kcad`.

**Şema 65**, şema 64'ün kendisi, çok satırlı yazının düzeni ve kesridir (ADR 0324): yazının `layout`'u, JSON'u gibi bir harita: `indent` (ilk satırın diğerlerinden girintisi, yazının yüksekliği cinsinden; eksi asılı girinti), `leftIndent` (paragrafların ilk satırları dışındaki satırların kutunun solundan girintisi, sıfır ya da pozitif), `tabs` (sekme durakları, yükseklik cinsinden, sıfırdan büyük, küçükten büyüğe, birer kez, en çok 32) ve `columns` (`count` 2–16, `gap` yükseklik cinsinden sıfır ya da pozitif, `height` metre, sıfırdan büyük; yalnız kutulu yazıda). Sıfır girinti yazılmaz, boş düzen yazılmaz; ilk satır kutunun solundan geri gidemez (`leftIndent + indent ≥ 0`), sayılar en çok 1000 yükseklik. Biçim diliminin `script`'i `stack` de olur (yığılmış kesir): dilimin harfleri `/`, `#` ya da `^` içerir. Sütunlarda yazının 16. seçeneği (düzenin JSON metni), dilimin 8 ve 16 bayrakları birlikte. Yazıcı `65`'i **yalnız düzenli ya da kesirli bir yazı varken** yazar. Şema 2–64 yükünde `layout` bilinmeyen alan (`broken/text-layout-in-schema-64.kcad`), `stack` bilinmeyen değerdir (`text-stack-in-schema-64`, `bad_value`); boş düzen, eksi sol girinti, kutudan taşan ilk satır, sırasız sekmeler, kutusuz ya da tek sütun ve ayıraçsız kesir `bad_value`'dur (`text-layout-empty`, `-left-negative`, `-hangs-out`, `-tabs-descending`, `-columns-boxless`, `-columns-one`, `text-stack-without-mark`). Örnek dosya `paragraph-layout.kcad`.

**Yeni şema eklemenin kuralı** (ADR 0265 §2): her yeni şema `n` için (1) yazıcı `n`'yi yalnız yeni özellik varken yazar; (2) `n`'de yazılmış geçerli bir örnek, bağımsız Python yazıcısıyla; (3) özelliğin `n − 1`'de yazılmış bozuk örneği (`broken/…-in-schema-<n−1>.kcad`) ve `broken/schema-version-<n+1>.kcad`; (4) önceki örneklerin hepsi okunmaya devam eder; (5) `tools/kcad/kcad.py` aynı kuralı okur; (6) örnek kodekten, tipli sütunlardan, bulutun kaynağından ve Python'un tipli katmanından geçer. Bir şema numarası yeniden kullanılmaz. `kentos-kcad`'in `schemas` testi (1)–(3)'ü zorlar.

Yazıcı, çizimin taşıdığını tutan **en eski** şemayı yazar: düzenli ya da kesirli yazısı olan çizim 65, ölçekli açıklaması olan çizim 64, stilden ayrılan alanı olan çizim 63, çizgi tipi, çizgi tipi ölçeği ya da desen anan katmanı, durumu ya da nesnesi olan çizim 56, dondurulmuş düğümü ya da dondurmayı saklayan katman durumu olan çizim 55, maskeli alanı olan çizim 54, ilişkili dizisi olan çizim 53, adlı görünümü olan proje 52, bir ölçüsünün başvurusu eğri üzerinde ya da blok içinde olan çizim 51, inç ya da ayak birimli ya da uzunluk biçimli çizim 50, geçerli UCS'i dünya olmayan ya da adlı UCS'i olan çizim 49, bir nesnesinin kendi görünüşü (çizgi tipi, ölçeği, saydamlık, çizim sırası, katmana göre parçalar) ya da yazdırılmayan katmanı olan çizim 48, NURBS eğrisi olan çizim 47, kapalı ya da genişlikli çoklu çizgisi olan çizim 46, istek yürütmüş çizim 45, pafta kitabı olan proje 44, başvurulu ölçüsü olan çizim 43, adres verisi olan proje 42, katman çizimi olan çizim 41, eki, eylemi ya da bilgi kartı olan çizim 40, köşe ölçüsü olan çizim 39, veri setli ya da ağ çizgili rasteri olan çizim 38, değişkeni olan proje 37, etiket motorunun bir alanı (etiket stilinin motor anahtarı, katmanın etiketlemesi ya da bir nesnenin etiket iğnesi) olan çizim 36, süzgeçli katmanı olan çizim 35, zaman ayarı, senaryosu ya da senaryo katmanı olan çizim 34, ağı olan proje 33, servis katmanı, servisten alınan katmanı ya da bağlantısı olan çizim 32, nokta bulutu ya da adresten okunan rasteri olan çizim 31, yazı yüksekliği, ölçü çizgisi, kılavuz ok boyu ya da yeni kılavuz oku olan çizim 30, rasteri olan çizim 29, ölçme ayarlarında önsel doğruluk olan çizim 28, topoloji ayarı olan çizim 27, alanı olan katmanı olan çizim 26, eğri boyunca yazısı olan çizim 25, resmi olan çizim 24, deseni çizgi aileli ya da degrade olan ya da nesnelere bağlı taraması olan çizim 23, tablosu olan çizim 22, projede yazı ya da ölçü stili olan ya da bir yazısının yüzü ya da bir ölçüsünün görünüşü olan çizim 21, kutusu, satır aralığı ya da biçim dilimi olan yazısı olan çizim 20, katman durumu olan çizim 19, bir nesnenin etiketini yazan yazısı olan çizim 18, çok parçalı çoklu çizgisi ya da çok noktalı nesnesi olan çizim 17, ölçme ayarlarında zemin (ortalama yükseklik ya da projeksiyona indirme) olan çizim 16, poligon toleransı olan çizim 15, ölçme ayarı olan çizim 14, projenin kendi sistemi, ikinci sistemin tanımı ya da datum seçimi olan çizim 13, ikinci koordinat sistemi olan çizim 12, olmayıp çizim birimi adlandıran 11, kendi keneti olan bir katmanı olan 10, olmayıp yeni ölçüsü olan 9, olmayıp kılavuz olan 8, olmayıp yazı eki olan 7, olmayıp blok tanımı olan 6, bloksuz olup çok parçalı alanı olan 5, parçalı alanı olmayıp kotu olan 4, kotu olmayıp nesne kalınlığı olan 3, hiçbiri olmayan 2. Okunan çizimin bellekteki biçimi (`DocumentSnapshotV2`) her şemada aynıdır; şema dosyanın neyi taşıdığını söyler.

### 6.2 Belge

Tablolar anahtarları **dosyadaki sırasıyla** verir (§5.2). “Zorunlu” sütunu boşsa alan isteğe bağlıdır ve yoksa hiç yazılmaz.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `name` | metin | evet | projenin adı |
| `arrays` | dizi: ilişkili dizi (§6.14) | | yalnız şema 53: ilişkili diziler, yapıldıkları sırayla; boş dizi yazılmaz (`bad_value`) |
| `blocks` | dizi: blok tanımı (§6.9) | | yalnız şema 6: blok tanımları, sırasıyla; boş dizi yazılmaz (`bad_value`) |
| `layers` | dizi: katman (§6.5) | evet | katman ağacının üst düzeyi, sırasıyla |
| `origin` | nokta | evet | yerel orijin: verinin yakınında bir çapa (GPU ona göre çalışır) |
| `sheets` | harita (§6.12) | | yalnız şema 44: projenin pafta kitabı ve resimlerinin baytları |
| `styles` | harita (§6.7) | evet | projenin kendi stil kitaplığı |
| `entities` | dizi: nesne (§6.6) | evet | nesneler, **belge sırasıyla** (çizim sırası) |
| `homeView` | sınırlar | | başlangıç görünümü; yoksa bütün nesneler |
| `requests` | dizi: istek (§6.13) | | yalnız şema 45: çizimin yürüttüğü istekler, sırasıyla; boş dizi yazılmaz (`bad_value`) |
| `settings` | harita (§6.4) | evet | proje ayarları |
| `projectId` | kimlik | | projenin kalıcı kimliği (§6.8) |
| `activeLayer` | metin | evet | etkin katmanın kimliği |
| `migratedFrom` | harita (§6.8) | | v1 dosyasından göçün kaynağı |

Bilinmeyen bir anahtar `unknown_field`, eksik zorunlu anahtar `missing_field`'dır. Bu kural şemanın **bütün** haritaları için geçerlidir (opak kısımlar hariç, §6.7).

### 6.3 Ortak biçimler

| Ad | CBOR | Kural |
|---|---|---|
| nokta | dizi: 2 float | `[x, y]`: `x` doğu (Y, sağa), `y` kuzey (X, yukarı), dünya birimiyle (metre); 2'den farklı öğe `bad_value` |
| nokta listesi | dizi: nokta | |
| sınırlar | dizi: 4 float | `[minX, minY, maxX, maxY]` |
| kimlik | bayt dizgisi: 16 | RFC 9562 UUID'nin 16 baytı, ağ sırasıyla; boş (nil, 16 sıfır) olamaz; boy yanlışsa `bad_value` |
| özet | bayt dizgisi: 32 | SHA-256 |
| float | `FB` binary64 | sonlu (§5.3) |
| kot listesi | dizi: float ya da `null` | yalnız şema 4 ve 5 (§6.6): köşe başına bir öğe, sonlu float (metre) ya da kotsuz köşe için `null`; başka türde öğe `wrong_type` |
| tam sayı (u32, u16) | ana tür 0 | aralık dışı `bad_value` |
| numaralı metin | metin | tabloda verilen değerlerden biri; değilse `bad_value` |

Yanlış CBOR türü (float yerine tam sayı, nokta yerine harita) `wrong_type`'tır.

### 6.4 Proje ayarları

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `srid` | u32 | evet | EPSG kodu; 0 yerel sistemdir (koordinat sistemi yok, ADR 0165 §2). Koordinat sistemi tahmin edilmez; SRID yalnız etikettir, dönüşüm değildir |
| `survey` | harita (§6.4.2) | | şema 14'te: projenin ölçme ayarları (ADR 0169 §3) |
| `areaUnit` | numaralı metin | evet | `m2`, `donum`, `ha` |
| `networks` | dizi (§6.4.8) | | şema 33'te: projenin ağları, tanımlandıkları sırayla (ADR 0209 §2); boş dizi yazılmaz |
| `topology` | harita (§6.4.5) | | şema 27'de: projenin topoloji kuralları, toleransı ve istisnaları (ADR 0202 §7) |
| `annotation` | harita (§6.4.6) | | şema 30'da: projenin yazı yükseklikleri, kâğıtta mm (ADR 0205 §1) |
| `connections` | dizi (§6.4.7) | | şema 32'de: projenin harita servislerine bağlantıları, sırları olmadan (ADR 0208 §2); boş dizi yazılmaz |
| `angleUnit` | numaralı metin | evet | `grad`, `deg` |
| `customCrs` | harita (§6.4.1) | | şema 13'te: projenin kendi sistemi bir tanımsa o (ADR 0168 §1); o zaman `srid` 0'dır, değilse `bad_value` (`broken/custom-crs-with-srid.kcad`). Böyle projenin ikinci sistemi olabilir (`secondSrid`), çizim birimi metredir |
| `lineTypes` | dizi | | şema 56'da: projenin desenli çizgi tipleri, penceredeki sırasıyla (ADR 0313 §1). Her öğe `LineTypeDef`'in kanonik haritasıdır: `name` (boş olamaz, büyük küçük harf ve kenar boşlukları gözetmeden eşsiz: `broken/line-types-same-name.kcad`), isteğe bağlı `description` ve `source`, `unit` (`paper` yazılmaz, `drawing`), `elements` (1–12 öğe; her biri `length` float ve karmaşık öğede `text` {`text`, `style`, yer} ya da `shape` {`name`, `file`, yer}; yer `scale` (1 yazılmaz), `rotation`, `absolute`, `upright`, `x`, `y`, sıfır ve `false` yazılmaz). Boş dizi yazılmaz (`broken/line-types-empty.kcad`) |
| `plotScale` | float | evet | çizim ölçeği paydası (1:1000 → `1000.0`) |
| `variables` | dizi (§6.4.9) | | şema 37'de: projenin `@` değişkenleri, penceredeki sırasıyla (ADR 0214 §2.3); boş dizi yazılmaz |
| `workspace` | numaralı metin | | projenin türü: `cad`, `gis`, `plan3d`, `disaster`; eski dosyaların `hybrid`'i (kalkan Hibrit modu) okunur ve olduğu gibi yazılır, alanın yokluğu gibi türü sorulmamış proje demektir (ADR 0165 §1) |
| `drawingFont` | numaralı metin | | `barlow`, `arimo`, `overpass`, `quicksand`, `architects-daughter`, `courier-prime`, `plex-mono` |
| `secondSrid` | u32 | | şema 12'de: projenin ikinci koordinat sisteminin EPSG kodu; koordinatları projeninkilerin yanında gösterilir, çizim dönüştürülmez (ADR 0167 §1). 0 olamaz, `srid` ile aynı olamaz, koordinat sistemi olmayan projede (`srid` 0, `customCrs` yok) bulunmaz; değilse `bad_value` (`broken/second-srid-zero.kcad`, `broken/second-srid-same.kcad`, `broken/second-srid-local.kcad`). Okuyucu sistemi tanımasa da alanı korur; değerleri gösterilmez |
| `drawingUnit` | numaralı metin | | şema 11'de: yerel projenin çizim birimi, `mm`, `cm`, `m` (yokluğu metre); şema 50'de `in` ve `ft` de (ADR 0262). Uzunluklar ve koordinatlar bu birimle yazılır ve gösterilir; geometri metrede saklanır. Koordinat sistemi olan projede birim sistemindir (ADR 0165 §2) |
| `layerStates` | dizi (§6.4.3) | | şema 19'da: projenin adlı katman durumları, menüdeki sırasıyla (ADR 0177 §4) |
| `textStyles` | dizi (§6.4.4) | | şema 21'de: projenin adlı yazı stilleri, penceredeki sırasıyla (ADR 0183 §1, §2) |
| `dimensionStyles` | dizi (§6.4.4) | | şema 21'de: projenin adlı ölçü stilleri, penceredeki sırasıyla (ADR 0183 §1, §3) |
| `areaDecimals` | u32 | evet | alan gösterim basamağı |
| `lineTypeScale` | float | | şema 56'da: projenin çizgi tipi ölçeği (LTSCALE), 0'dan büyük, en çok 1000; 1 yazılmaz (`broken/line-type-scale-one.kcad`), yoksa 1 (ADR 0313 §4) |
| `lengthDecimals` | u32 | evet | uzunluk gösterim basamağı |
| `addressLocators` | dizi (§6.4.10) | | şema 42'de: projenin adres verileri, tanımlandıkları sırayla (ADR 0229 §2); boş dizi yazılmaz |
| `lengthStyle` | numaralı metin | | şema 50'de: `architectural`, `engineering`, `fractional`; yalnız `drawingUnit` `in` ya da `ft` iken (ADR 0262 §2) |
| `ucs` | harita (§6.4.11) | | şema 49'da: geçerli kullanıcı koordinat sistemi (ADR 0261 §1); yoksa dünya, dünya yazılmaz |
| `namedUcs` | dizi (§6.4.11) | | şema 49'da: adlı UCS'ler, kaydedildikleri sırayla; boş dizi yazılmaz |
| `namedViews` | dizi (§6.4.12) | | şema 52'de: adlı görünümler, kaydedildikleri sırayla; boş dizi yazılmaz |
| `datumTransforms` | dizi (§6.4.1) | | şema 13'te: projenin datum seçimleri, kayıttaki her datum çifti için en çok biri (ADR 0168 §3); değilse `bad_value` (`broken/datum-transform-twice.kcad`) |
| `secondCustomCrs` | harita (§6.4.1) | | şema 13'te: ikinci sistem bir tanımsa o (ADR 0168 §1); `secondSrid` ile birlikte bulunmaz, projenin bir sistemi olmalıdır (`srid` ya da `customCrs`); değilse `bad_value` (`broken/second-custom-with-second-srid.kcad`, `broken/second-custom-without-system.kcad`) |

#### 6.4.1 Koordinat sistemi tanımları ve datum seçimleri

Şema 13'te (ADR 0168). Bir **tanım** haritadır: `name` (boş olmayan metin) ve `system` (harita). Sistemin anahtarları, `kind`'ına göre:

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `kind` | numaralı metin | evet | `tm`, `geographic`, `local` |
| `datum` | numaralı metin | | `tm` ve `geographic`'te: kayıttaki datum, `TUREF`, `ED50`, `WGS84`; |
| `customDatum` | harita | | ya da projenin datumu (aşağıda); ikisinden tam biri |
| `scaleFactor` | float | `tm` | 0'dan büyük |
| `falseEasting`, `falseNorthing` | float | `tm` | m |
| `centralMeridian` | float | `tm` | derece, −180 ile 180 arası |
| `latitudeOfOrigin` | float | | `tm`'de derece, −90 ile 90 arası; yokluğu 0 |
| `base` | harita | `local` | `srid` (u32, kayıttaki projeksiyonlu bir sistem, 0 olamaz) ya da `definition` (yalnız `tm` olan bir tanım); tam biri |
| `plane` | harita | `local` | `kind` `similarity` (`east`, `north`, `rotation` derece, saat yönünün tersine, `scale` 0'dan büyük) ya da `affine` (`a` … `f`, `a·e − b·d` 0 olamaz): bu sistemden tabana, taban x = a·x + b·y + c, taban y = d·x + e·y + f |

Bir sistemde türünün olmayan anahtar `unknown_field`'dır (`broken/custom-crs-field-of-other-kind.kcad`).

**Projenin datumu** haritadır: `name` (metin), `toWgs84` (isteğe bağlı yedi parametre; yoksa datum kendi içinde kalır) ve `ellipsoid` (`name`, `semiMajor` 0'dan büyük, `inverseFlattening` 1'den büyük). **Yedi parametre** (`toWgs84`, `helmert`) haritadır: `scale` (float, ppm), isteğe bağlı `accuracy` (float, m, 0 ya da büyük), `rotation` (3 float, ″), `convention` (`positionVector`, `coordinateFrame`), `translation` (3 float, m).

Bir **datum seçimi** haritadır: `to` ve `from` (kayıttaki iki ayrı datum), `name` (metin) ve ya `helmert` (yedi parametre) ya `grid` (harita: `id` küçük harfli 64 onaltılık rakamla dosyanın SHA-256'sı, `file` metin, `size` u64 0'dan büyük, isteğe bağlı `accuracy`); tam biri. Izgaranın kendisi dosyada değildir, cihazın kitaplığındadır (ADR 0168 §4).

Bütün sayılar sonludur. Bu kurallardan biri tutmazsa `bad_value` (`broken/custom-crs-two-datums.kcad`, `custom-crs-flat-ellipsoid.kcad`, `custom-crs-folded-plane.kcad`, `custom-crs-local-base.kcad`, `custom-crs-rotation-two.kcad`, `datum-transform-same.kcad`, `datum-transform-both.kcad`, `datum-transform-grid-id.kcad`). Örnek dosyalar `custom-crs.kcad` (yerel sistem, kayıttaki taban, iki datum seçimi), `custom-second-crs.kcad` (ikinci sistemin tanımı: afin, projenin datumlu TM tabanı) ve `custom-geographic.kcad` (WGS 84'e bağı olmayan datumla coğrafi sistem).


#### 6.4.2 Ölçme ayarları

Şema 14'te (ADR 0169 §3). Harita; anahtarların hepsi isteğe bağlıdır ama en az biri bulunur (boş harita `bad_value`, `broken/survey-empty.kcad`); `reduceToGrid` bool, ötekiler float'tır; başka anahtar `unknown_field`'dır (`broken/survey-unknown-key.kcad`):

| Anahtar | Değer |
|---|---|
| `index` | indeks hatası toleransı, radyan; 0'dan büyük |
| `faceHz` | iki durum yatay açı farkı toleransı, radyan; 0'dan büyük (`broken/survey-tolerance-zero.kcad`) |
| `faceSlope` | iki durum eğik uzunluk farkı toleransı, m; 0'dan büyük |
| `refraction` | trigonometrik kot farkının refraksiyon katsayısı k, −1 ile 1 arası (`broken/survey-refraction-range.kcad`); yokluğu 0.13 |
| `twoWay` | şema 15'te: poligon kenarının iki yönden yatay uzunluk farkı toleransı, m; 0'dan büyük (`broken/survey-two-way-zero.kcad`) |
| `traverseAngle` | şema 15'te: poligonun açı kapanması toleransı, radyan; 0'dan büyük |
| `traverseCoord` | şema 15'te: poligonun koordinat kapanması (fs) toleransı, m; 0'dan büyük |
| `groundHeight` | şema 16'da: projenin ortalama elipsoit yüksekliği, m; −500 ile 9000 arası (`broken/survey-ground-range.kcad`); Mesafe ölç ve Alan hesapla'nın zemin değerleri (ADR 0171 §2) |
| `reduceToGrid` | şema 16'da: bool; Hesap pencereleri ölçülen uzunlukları düzleme indirir, Aplikasyon düzlemdekini zemine çevirir (ADR 0171 §4); `true` yalnız `groundHeight` varken (`broken/survey-reduce-without-height.kcad`), bool değilse `wrong_type` (`broken/survey-reduce-not-bool.kcad`); uygulamalar `false`'u yazmaz, okuyucu yazılmış `false`'u da okur |
| `sigmaDirection` | şema 28'de: doğrultu ölçüsünün önsel standart sapması, radyan; 0'dan büyük (ADR 0203 §1) |
| `sigmaDistance` | şema 28'de: kenarın sabit payı, m; 0'dan büyük (`broken/survey-sigma-zero.kcad`) |
| `sigmaPpm` | şema 28'de: kenarın uzunlukla artan payı, milyonda; 0'dan küçük değil (`broken/survey-sigma-ppm-negative.kcad`) |
| `sigmaCentering` | şema 28'de: alet ve hedefin her birinin merkezlemesi, m; 0'dan küçük değil |
| `sigmaZenith` | şema 28'de: başucu açısının önsel standart sapması, radyan; 0'dan büyük |
| `sigmaLevelling` | şema 28'de: geometrik nivelmanın kilometre başına standart sapması, m; 0'dan büyük |

Yokluğu denetlenmeyen toleranstır: farklar gösterilir, karşılaştırılmaz. Uygulamalar varsayılan k'yı (0.13) yazmaz; okuyucu yazılmış 0.13'ü de okur.

#### 6.4.3 Katman durumları

Şema 19'da (ADR 0177 §4). Dizi; boş dizi yazılmaz (alan yazılmaz). Her öğe bir haritadır, anahtarları (kodlanmış sırasıyla) `id` < `name` < `nodes`:

| Anahtar | Tür | Değerler |
|---|---|---|
| `id` | metin | durumun kimliği; boş olamaz, projede bir kez (`broken/layer-states-empty-id.kcad`, `broken/layer-states-same-id.kcad`) |
| `name` | metin | menüdeki adı; uçlarındaki boşluklar dışında boş olamaz, öyle karşılaştırılınca projede bir kez (`broken/layer-states-empty-name.kcad`, `broken/layer-states-same-name.kcad`) |
| `nodes` | dizi | kaydedildiği andaki ağacın sırasıyla düğümler |

Düğüm bir haritadır, anahtarları `node` < `plot` < `style` < `frozen` < `locked` < `visible`:

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `node` | metin | evet | katmanın ya da grubun kimliği (§6.5); boş olamaz, bir durumda bir kez (`broken/layer-states-same-node.kcad`). Ağaçta olmayan düğüm (sonradan silinmiş) geçerlidir: uygulanınca atlanır |
| `style` | harita (§6.5'in katman stili) | | durum stille kaydedildiyse katmanın stili; grubun yoktur |
| `locked` | bool | | durum kilitlerle kaydedildiyse düğümün kendi kilidi |
| `frozen` | bool | | şema 55'te: düğümün kendi dondurulması; şema 55'ten beri her durum saklar, yoksa uygulama dondurmaya dokunmaz (ADR 0311 §3) |
| `plot` | bool | | şema 55'te: düğümün kendi yazdırılması; yoksa uygulama yazdırmaya dokunmaz |
| `visible` | bool | evet | düğümün kendi görünürlüğü (`broken/layer-states-without-visible.kcad`, bool değilse `wrong_type`: `broken/layer-states-visible-not-bool.kcad`) |

#### 6.4.4 Yazı ve ölçü stilleri

Şema 21'de (ADR 0183). İki dizi; boş dizi yazılmaz (alan yazılmaz). Her öğe bir haritadır; `false` bayrak ve olmayan değer yazılmaz. Liste bütün olarak denetlenir (`bad_value`): kimlik boş olamaz ve listede bir kez geçer (`broken/style-table-id-empty.kcad`); ad uçlarında boşluksuz, boş değil, en çok 64 harf, satır sonu ve denetim karakteri olmadan ve büyük küçük harf ayırmadan listede bir kezdir (`broken/style-table-name-padded.kcad`, `broken/style-table-name-twice.kcad`); “Standart” (stilsiz yazı ve ölçü) ayrılmıştır (`broken/style-table-standart.kcad`). Stilin kimliği nesnelerin `textStyle`'ı ve `dimStyle`'ıdır; adı değişebilir.

**Yazı stili**, anahtarları kodlanmış sırasıyla `id` < `bold` < `font` < `name` < `height` < `italic` < `oblique` < `fontFile` < `widthFactor`:

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `id`, `name` | metin | evet | kimlik ve ad (yukarıda) |
| `font` | numaralı metin | evet | yedi çizim yazı tipinden biri (§6.4'ün `drawingFont`'u gibi; `broken/style-table-font-missing.kcad`) |
| `bold`, `italic` | bool | | yalnız `true` yazılır |
| `oblique` | float | | harflerin yatıklığı, derece: −85 ile 85 arası, 0 değil |
| `height` | float | | kâğıtta mm: sıfırdan büyük, en çok 1000 (`broken/style-table-height-zero.kcad`); yokluğu yüksekliği aracın bırakır |
| `widthFactor` | float | | 0'dan büyük, en çok 100, 1 değil |
| `fontFile` | metin | | DXF'ten gelen yazı tipi dosyası (boş değil); DXF'e aynen yazılır |

**Ölçü stili**, anahtarları kodlanmış sırasıyla `id` < `font` < `name` < `unit` < `arrow` < `height` < `prefix` < `suffix` < `textGap` < `decimals` < `extColor` < `arrowSize` < `extBeyond` < `extOffset` < `extWeight` < `textColor` < `textPlace` < `dimLineType` < `extLineType` < `dimLineColor` < `dimLineWeight` (çizgilerinki şema 30'da):

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `id`, `name` | metin | evet | kimlik ve ad (yukarıda) |
| `height` | float | evet | değerin kâğıttaki yüksekliği, mm: sıfırdan büyük, en çok 1000 |
| `arrow` | numaralı metin | | `closed` (dolu ok), `open` (açık ok), `dot` (nokta), `none` (yok); yokluğu çentiktir |
| `arrowSize`, `extOffset`, `extBeyond`, `textGap` | float | | mm: ok boyu sıfırdan, öbürleri 0'dan büyük ya da eşit; en çok 1000 ve değer yüksekliğinin 100 katı (`broken/style-table-ratio-too-big.kcad`) |
| `textPlace` | numaralı metin | | `centre`: değer çizginin ortasında; yokluğu üstünde |
| `decimals` | u32 | | en çok 8; yokluğu projenin uzunluk basamakları |
| `unit` | numaralı metin | | `m`, `cm`, `mm`; yokluğu projenin birimi |
| `prefix`, `suffix` | metin | | en çok 32 harf, boş değil, satır sonu ve denetim karakteri yok |
| `font` | numaralı metin | | değerin yazı tipi; yokluğu projenin |
| `dimLineColor`, `extColor`, `textColor` | metin | | şema 30'da: ölçü çizgisinin, uzatma çizgilerinin ve değerin rengi, `#RRGGBB` (`broken/dimension-style-text-colour-short.kcad`); yokluğu nesnenin rengi |
| `dimLineWeight`, `extWeight` | float | | şema 30'da: ölçü çizgisinin ve uzatma çizgilerinin kâğıttaki kalınlığı, mm, 0 ile 100 arası (`broken/dimension-style-line-weight-too-big.kcad`); yokluğu nesnenin kalınlığı |
| `dimLineType`, `extLineType` | numaralı metin | | şema 30'da: `continuous`, `dashed`, `dashdot`, `dotted`; yokluğu sürekli |

Bilinmeyen anahtar `unknown_field`'dır (`broken/style-table-unknown-field.kcad`).

#### 6.4.5 Topoloji kuralları

Şema 27'de (ADR 0202). Harita, anahtarları kodlanmış sırasıyla `rules` < `tolerance` < `exceptions`; üçü de isteğe bağlıdır ama harita boş olamaz (`broken/topology-empty.kcad`). Bütünü denetlenir (`bad_value`).

| Anahtar | Tür | Değerler |
|---|---|---|
| `rules` | dizi | kurallar, denetlenecekleri ve listelenecekleri sırayla; boş dizi yazılmaz |
| `tolerance` | float | metre, 0,000001 ile 1 arası (`broken/topology-tolerance-range.kcad`); yokluğu 0,001 |
| `exceptions` | dizi | bilerek bırakılan bulgular; boş dizi yazılmaz |

**Kural** haritadır, anahtarları `id` < `kind` < `layer` < `other` < `value`:

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `id` | metin | evet | boş olamaz, projede bir kez (`broken/topology-rule-same-id.kcad`); istisnalar onu anar |
| `kind` | numaralı metin | evet | `mustNotOverlap`, `mustNotHaveGaps`, `mustNotHaveSlivers`, `mustNotHaveDuplicates`, `mustNotHaveDangles`, `mustNotHaveShortEdges`, `mustNotHaveSmallAngles`, `mustBeValid`, `mustNotHaveMissingVertices` (katman içi); `mustNotOverlapWith`, `mustBeCoveredBy`, `boundaryMustBeCoveredBy`, `mustBeOnEndOf` (katmanlar arası); başkası `bad_value` (`broken/topology-rule-unknown-kind.kcad`) |
| `layer` | metin | evet | katmanın kimliği (§6.5), boş olamaz. Projede olmayan katman geçerlidir: kural denetlenmez |
| `other` | metin | | katmanlar arası kuralda zorunlu, başkasında yazılamaz (`broken/topology-rule-without-other.kcad`, `broken/topology-rule-other-on-one-layer.kcad`); boş ya da `layer` ile aynı olamaz (`broken/topology-rule-other-itself.kcad`) |
| `value` | float | | yalnız değer alan türde (`mustNotHaveSlivers`, `mustNotHaveShortEdges` metre; `mustNotHaveSmallAngles` radyan, dik açıdan küçük; `broken/topology-rule-value-not-taken.kcad`, `broken/topology-rule-angle-too-large.kcad`); sıfırdan büyük; yokluğu türün varsayılanı (0,1 m, 0,05 m, 5°) |

**İstisna** haritadır, anahtarları `at` < `rule` < `objects`:

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `at` | nokta | evet | bulgunun yeri |
| `rule` | metin | evet | kuralın kimliği; listede olmayan kural `bad_value` (`broken/topology-exception-unknown-rule.kcad`) |
| `objects` | dizi: kimlik | evet | bulgunun nesnelerinin kalıcı kimlikleri (§6.3), bulgunun sırasıyla; boş olamaz. Çizimde olmayan nesnenin kimliği geçerlidir: istisna bir bulguya uymaz |

Bilinmeyen anahtar `unknown_field`'dır.

#### 6.4.6 Yazı yükseklikleri

Şema 30'da (ADR 0205 §1). Harita, anahtarları kodlanmış sırasıyla `text` < `table` < `leader` < `measure` < `station` < `dimension` < `coordinate`; hepsi isteğe bağlı float'tır, kâğıtta mm: sıfırdan büyük, en çok 100 (`broken/annotation-zero.kcad`, `broken/annotation-too-tall.kcad`). Harita boş olamaz (`broken/annotation-empty.kcad`); bilinmeyen anahtar `unknown_field` (`broken/annotation-unknown-kind.kcad`), tam sayı `wrong_type`'tır (`broken/annotation-int.kcad`). Bir türün yüksekliği yoksa varsayılanıdır: `text` (Yazı), `leader` (Kılavuz), `dimension` (Ölçü) ve `table` (Tablo) 2,5, `coordinate` (Koordinat yazısı), `station` (Km yazısı) ve `measure` (Kenar ve köşe yazıları) 2. Çizimdeki yükseklik `mm / 1000 × plotScale`'dir; nesneler kendi yüksekliklerini metrede taşır, bu alan yalnız yeni nesnelerin ve izlemenin kuralıdır (ADR 0205 §2, §3).

#### 6.4.7 Bağlantılar

Şema 32'de (ADR 0208 §2). Bir bağlantı, bir sunucunun kimlik doğrulamasıdır: hangi kökene (şema, makine ve kapı) gidildiğinde nasıl doğrulanılır. **Sır dosyaya yazılmaz** (anahtar, parola, belirteç, istemci sırrı): sırlar cihazındır (masaüstünde `baglantilar.json`, web'de tarayıcının deposu); dosyayı açan kendi sırrını girer. Bilinmeyen anahtar (`password` dahil) `unknown_field`'dır (`broken/connection-secret.kcad`).

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | metin | evet | projede bir kez (`broken/connection-duplicate.kcad`); 1–64 harf, rakam, tire ya da alt çizgi; katmanlar bununla anar |
| `auth` | numaralı metin | evet | `none`, `query` (adresin parametresi), `header` (istek başlığı), `basic` (kullanıcı adı ve parola), `bearer` (belirteç), `arcgis` (ArcGIS belirteci), `oauth2` (istemci kimliğiyle belirteç), `google` (Google Map Tiles API anahtarı ve oturumu) |
| `name` | metin | evet | görünen ad, 1–128 harf |
| `names` | dizi: metin | | `query` ve `header`'da zorunlu: parametrenin ya da başlığın adları (1–8; harf, rakam, tire, nokta, alt çizgi); başkalarında yazılmaz (`broken/connection-basic-names.kcad`); boş dizi yazılmaz |
| `scope` | metin | | yalnız `oauth2`'de: istenen kapsam (`broken/connection-scope-not-oauth2.kcad`) |
| `origin` | metin | evet | küçük harfle `http(s)://makine[:kapı]`, yolsuz, varsayılan kapı yazılmaz (`broken/connection-origin-path.kcad`, `broken/connection-origin-upper.kcad`); sır yalnız bu kökene gider |
| `tokenUrl` | metin | | `arcgis` ve `oauth2`'de belirtecin alındığı adres; `oauth2`'de zorunlu (`broken/connection-oauth2-no-token-url.kcad`) |

Liste en çok 256 bağlantıdır; boş liste yazılmaz (`broken/connections-empty.kcad`). Kurallar sözleşmenindir (`kentos_contracts::service::connections_problem`); kıran liste `bad_value`'dur.

#### 6.4.8 Ağlar

Şema 33'te (ADR 0209 §2). Bir ağ, çizimin katmanlarından her analizde kurulan grafın tanımıdır; grafın kendisi dosyaya yazılmaz. Harita, anahtarları kodlanmış sırasıyla:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | metin | evet | 1–40 küçük harf, rakam ya da tire; projede bir kez (`broken/network-duplicate-id.kcad`, `broken/network-bad-id.kcad`) |
| `kind` | numaralı metin | evet | `road` (Yol ağı), `utility` (Şebeke); bilinmeyen değer `bad_value` (`broken/network-unknown-kind.kcad`) |
| `name` | metin | evet | kırpılmış, 1–80 karakter, projede bir kez (Türkçe büyük küçük harf ayrımı olmadan; `broken/network-duplicate-name.kcad`, `broken/network-name-untrimmed.kcad`) |
| `costs` | dizi | | uzunluktan başka maliyetler, en çok 8; boş dizi yazılmaz (`broken/network-costs-empty.kcad`). Her biri `name` (kırpılmış, 1–40 karakter, bir kez, `Uzunluk` olamaz: `broken/network-cost-length-name.kcad`, `broken/network-cost-twice.kcad`), `kind` (`speed`: süre, dakika = uzunluk / hız; `field`: kenarın bütününün maliyeti), `field` (okunan öznitelik), `unit` (yalnız `field`'da, en çok 12 karakter; boş yazılmaz: `broken/network-cost-speed-unit.kcad`), `speed` (yalnız `speed`'de ve zorunlu: alanın okunamadığı yerde km/sa, 0 < h ≤ 1000; `broken/network-cost-speed-zero.kcad`, `broken/network-cost-field-speed.kcad`) |
| `edges` | dizi | evet | kenar katmanları, 1–16: `layer` (katmanın kimliği) ve isteğe bağlı `filter` (ifade); aynı katman aynı süzgeçle bir kez (`broken/network-no-edges.kcad`, `broken/network-edge-twice.kcad`) |
| `closed` | metin | | kapalı kenarların ifadesi (ADR 0100), boş olmayan, en çok 1000 karakter (`broken/network-closed-blank.kcad`) |
| `connect` | numaralı metin | evet | `ends` (uçlarda; bir uç başka bir kenarın içine değiyorsa orada), `vertices` (bütün köşelerde de) |
| `direction` | harita | evet | `kind`: `both`, `digitized` ya da `field`; yalnız `field`'da `field` (zorunlu) ve `forward`, `backward`, `closed` (metin dizileri; boşu yazılmaz, en az biri bulunur; her biri en çok 16 değer, değer en çok 40 karakter, bir değer listelerde bir kez: `broken/network-direction-no-values.kcad`, `broken/network-direction-value-twice.kcad`, `broken/network-direction-list-empty.kcad`). Başka türde alan ya da değer `bad_value`'dur (`broken/network-direction-field-on-both.kcad`). Sözleşmenin JSON biçimi `field` yönünde üç diziyi de, boşunu da yazar |
| `junctions` | dizi | | düğüm katmanları, en çok 16; boş dizi yazılmaz (`broken/network-junctions-empty.kcad`). Her biri `layer`, `role` (`junction`, `source`, `valve`; `broken/network-junction-role.kcad`), isteğe bağlı `filter` ve `closed` (ifadeler) |
| `tolerance` | float | evet | metre, 0,0001 ile 10 arasında (`broken/network-tolerance.kcad`) |

Bilinmeyen anahtar `unknown_field`'dır (`broken/network-unknown-field.kcad`). Liste en çok 32 ağdır; boş liste yazılmaz (`broken/networks-empty.kcad`). Kurallar sözleşmenindir (`kentos_contracts::network::networks_problem`); kıran liste `bad_value`'dur. Ağın adlandırdığı katmanın çizimde olması kural değildir: ağ kurulurken söylenir.

#### 6.4.9 Değişkenler

Şema 37'de (ADR 0214 §2.3). Projenin kendi `@ad` değerleri; ifade dili onları derlenirken sabit olarak okur, paftanın `@` değerleri de projeninkileri bunlardan alır. Harita, anahtarları kodlanmış sırasıyla:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `kind` | numaralı metin | evet | `text`, `number`, `bool`, `date`; bilinmeyen değer `bad_value` (`broken/variable-unknown-kind.kcad`), yokluğu `missing_field` (`broken/variable-no-kind.kcad`) |
| `name` | metin | evet | `@`'siz ad: bir harf ya da `_`, ardından harf, rakam ve `_`; en çok 64 karakter (`broken/variable-bad-name.kcad`). Adlar Türkçe harf ve büyük küçük harf ayrımı olmadan karşılaştırılır (`Proje_Adı` = `proje_adi`); projede bir kez (`broken/variable-duplicate.kcad`), yerleşik değişkenlerin adı olamaz: `proje_adi`, `koordinat_sistemi`, `epsg`, `olcek`, `tarih`, `simdi`, `katman_adi`, `katman`, `kullanici` (`broken/variable-builtin-name.kcad`) |
| `label` | metin | | pencerede adın yanında görünen; boş yazılmaz |
| `value` | null, bool, float ya da metin | | türünün değeri: `number` float, `bool` true/false, `text` en çok 4000 karakter metin, `date` takvimde bir günün `YYYY-AA-GG` metni (`broken/variable-number-text.kcad`, `broken/variable-bad-date.kcad`); yokluğu (ya da null) değeri henüz verilmemiş değişkendir, ifadede boştur |

Bilinmeyen anahtar `unknown_field`'dır (`broken/variable-unknown-field.kcad`). Liste en çok 200 değişkendir; boş liste yazılmaz (`broken/variables-empty.kcad`). Kurallar sözleşmenindir (`kentos_contracts::variables::variables_problem`); kıran liste `bad_value`'dur.

#### 6.4.10 Adres verisi

Şema 42'de (ADR 0229 §2). Bir adres verisi, adres aramasının her aramada çizimin katmanlarından kurduğu dizinin tanımıdır; dizinin kendisi dosyaya yazılmaz. Harita, anahtarları kodlanmış sırasıyla:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | metin | evet | 1–40 küçük harf, rakam ya da tire; projede bir kez (`broken/address-duplicate-id.kcad`, `broken/address-bad-id.kcad`) |
| `name` | metin | evet | kırpılmış, 1–80 karakter, projede bir kez (Türkçe büyük küçük harf ayrımı olmadan; `broken/address-duplicate-name.kcad`, `broken/address-name-untrimmed.kcad`) |
| `areas` | dizi | | bölge katmanları, her düzeyden en çok biri; boş dizi yazılmaz (`broken/address-areas-empty.kcad`). Her biri `layer`, `level` (`neighbourhood`, `district`, `province`; bilinmeyen değer `bad_value`: `broken/address-area-unknown-level.kcad`; bir düzey iki kez `bad_value`: `broken/address-area-level-twice.kcad`), `name` (alanın adını veren öznitelik) ve yalnız mahalle düzeyinde isteğe bağlı `district` (`broken/address-district-area-district.kcad`) |
| `doors` | harita | | kapı noktaları: `layer`, `street` ve `door` (zorunlu; `broken/address-door-without-street.kcad`), isteğe bağlı `neighbourhood`, `district`, `province`, `postcode`, `building`, `code` |
| `reach` | float | evet | Konumdan adres'in uzaklığı, metre: 0'dan büyük, en çok 10 000 (`broken/address-reach-zero.kcad`, `broken/address-reach-far.kcad`) |
| `streets` | harita | | yol orta hatları: `layer`, `name` (zorunlu), isteğe bağlı `neighbourhood`, `district` |
| `minScore` | u32 | evet | en düşük eşleşme puanı, 50–100 (`broken/address-min-score.kcad`); yokluğu `missing_field` (`broken/address-no-min-score.kcad`) |

Alanların değerleri özniteliklerin adlarıdır: boş olamaz (boş yazılmaz: `broken/address-door-field-empty.kcad`), kırpılmış, en çok 64 karakter ve denetim karaktersiz olmalıdır (`broken/address-door-field-untrimmed.kcad`). Kapı, yol ve bölgeden en az biri bulunur (`broken/address-no-source.kcad`). Bilinmeyen anahtar `unknown_field`'dır (`broken/address-unknown-field.kcad`). Liste en çok 16 tanımdır; boş liste yazılmaz (`broken/addresses-empty.kcad`). Kurallar sözleşmenindir (`kentos_contracts::address::address_locators_problem`); kıran liste `bad_value`'dur. Tanımın adlandırdığı katmanın çizimde olması kural değildir: dizin kurulurken söylenir.

#### 6.4.11 Kullanıcı koordinat sistemi

Şema 49'da (ADR 0261 §1). UCS dünyada bir başlangıç ve x ekseninin doğudan saat yönünün tersine açısıdır; ölçeği ve aynalaması
yoktur. `ucs` bir haritadır, anahtarları `angle` < `origin`: `origin` nokta, her iki sayısı da mutlak değerce en çok 10 000 km
(`broken/ucs-far.kcad`); `angle` float, radyan, 0 ile 2π arasında, 2π hariç (`broken/ucs-angle-tau.kcad`). Dünya UCS'i (başlangıç
(0, 0), açı 0) ayar olarak yazılmaz (`broken/ucs-world.kcad`). `namedUcs`'in her öğesi aynı haritaya `name` (metin) eklenmiş
hâlidir, anahtarları `name` < `angle` < `origin`: ad kırpılmış, 1–64 karakter, denetim karaktersiz (`broken/named-ucs-blank.kcad`),
dizide tekil (`broken/named-ucs-twice.kcad`; büyük küçük harf ayrıdır), en çok 100 öğe; boş dizi yazılmaz
(`broken/named-ucs-empty.kcad`), bilinmeyen anahtar `unknown_field`'dır (`broken/named-ucs-unknown-field.kcad`). Bunların hepsi
`bad_value`'dur. Nesnelerin koordinatları dünyadadır; UCS onları değiştirmez. Örnek dosya `ucs.kcad`.

#### 6.4.12 Adlı görünümler

Şema 52'de (ADR 0285 §1). Adlı görünüm kameranın baktığı yerdir: alanın ortasındaki dünya noktası, ekranın x ve y'si boyunca
gösterilen dünya eni ve boyu ve dönüşü (ekranın x ekseninin doğudan saat yönünün tersine açısı). `namedViews`'in her öğesi bir
haritadır, anahtarları `name` < `twist` < `width` < `center` < `height`, hepsi zorunlu (`broken/named-views-no-twist.kcad`,
`missing_field`): `name` kırpılmış, 1–64 karakter, denetim karaktersiz (`broken/named-views-blank.kcad`), dizide tekil
(`broken/named-views-twice.kcad`; büyük küçük harf ayrıdır); `center` nokta, her iki sayısı da mutlak değerce en çok 10 000 km
(`broken/named-views-far.kcad`); `width` ve `height` float, 0'dan büyük ve en çok 20 000 km (`broken/named-views-width-zero.kcad`);
`twist` float, radyan, 0 ile 2π arasında, 2π hariç (`broken/named-views-twist-tau.kcad`). En çok 100 öğe; boş dizi yazılmaz
(`broken/named-views-empty.kcad`); bilinmeyen anahtar `unknown_field`'dır (`broken/named-views-unknown-field.kcad`). Öbürleri
`bad_value`'dur. Görünüme gitmek kameranındır, çizim değişmez. Örnek dosya `views.kcad`.

### 6.5 Katman ağacı

**Katman** (grup ya da katman):

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | metin | evet | proje içinde benzersiz kimlik; ad değişince değişmez |
| `card` | bilgi kartı | | şema 40'ta: Bilgi aracının nesnelerde gösterdiği (aşağıda); yalnız katmanda, grupta ve servisten çizilen katmanda `bad_value` |
| `feed` | veri kaynağı | | şema 32'de: katmanın nesnelerinin alındığı servis ya da adres (aşağıda); yalnız katmanda |
| `name` | metin | evet | görünen ad |
| `snap` | katman keneti | | şema 10'da: katmanın kendi keneti (aşağıda); yalnız katmanda, grupta `bad_value` |
| `time` | zaman ayarı | | şema 34'te: katmanın nesnelerinin zamanı hangi özniteliklerde (aşağıda); yalnız katmanda, grupta ve servisten çizilen katmanda `bad_value` |
| `type` | numaralı metin | evet | `group`, `layer` |
| `style` | katman stili | evet | |
| `fields` | dizi: alan | | şema 26'da: katmanın nesnelerinin özniteliklerinin şeması (aşağıda); boş değil; yalnız katmanda, grupta `bad_value` |
| `filter` | süzgeç | | şema 35'te: katmanın süzgeci (aşağıda); yalnız katmanda, grupta ve servisten çizilen katmanda `bad_value` |
| `actions` | dizi: eylem | | şema 40'ta: nesnelerin açtığı adresler ve dosyalar (aşağıda); boş değil; yalnız katmanda, grupta ve servisten çizilen katmanda `bad_value` |
| `compose` | katman çizimi | | şema 41'de: katmanın çizime nasıl girdiği (aşağıda); yalnız katmanda, grupta `bad_value` |
| `frozen` | bool | | şema 55'te: yalnız `true` (`false` `bad_value`: `broken/frozen-false.kcad`); katman ya da grup altındakilerle dondurulmuştur: çizilmez, seçilmez, kenetlenmez, kapsama sayılmaz; etkin katman dondurulmaz (ADR 0311 §1) |
| `locked` | bool | evet | |
| `service` | servis | | şema 32'de: katman bu harita servisinden çizilir (aşağıda); yalnız katmanda; böyle katman nesne tutmaz |
| `visible` | bool | evet | |
| `plot` | bool | | şema 48'de: yalnız `false` (`true` `bad_value`: `broken/plot-true.kcad`); katman ya da grup paftada ve yazdırmada çizilmez, altındakilerle (ADR 0260 §4) |
| `children` | dizi: katman | evet | alt düğümler (katmanda boş dizi) |
| `expanded` | bool | evet | ağaçta açık mı |
| `replaces` | metin | | şema 34'te: senaryo katmanının yerine geçtiği ana katmanın kimliği (aşağıda); yalnız bir senaryo grubunun içindeki katmanda |
| `scenario` | senaryo | | şema 34'te: grup bir senaryodur (aşağıda); yalnız grupta, katmanda `bad_value` |

**Zaman ayarı** (şema 34; ADR 0210 §2): adlar kırpılmış, 1–64 karakter; `end` `start`'tan farklı.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `end` | metin | | bitişin özniteliği; yoksa nesneler anlıktır |
| `key` | metin | | kimlik alanı: bir nesnenin sürümlerini birbirine bağlayan öznitelik |
| `start` | metin | evet | başlangıcın (anlık katmanda anın) özniteliği |
| `cumulative` | bool | | yalnız `true`: Birikimli, nesne başlangıcından sonra hep görünür |

Kurala uymayan ayar, `cumulative: false` ve bilinmeyen anahtar reddedilir (`fixtures/kcad/v2/broken/time-*.kcad`).

**Senaryo** (şema 34; ADR 0210 §9): harita; `note` metni (isteğe bağlı; kırpılmış, 1–500 karakter). **Ağacın kuralları:** senaryo grubu başka bir senaryo grubunun içinde olmaz; `replaces` yalnız bir senaryo grubunun içindeki katmanda olur, boş olmaz ve katmanın kendisini göstermez; gösterdiği düğüm ağaçta varsa senaryo grubu dışında bir katmandır (ana katman) ve bir senaryo içinde bir ana katmanın yerine en çok bir katman geçer. Ağaçta olmayan bir düğümü gösteren `replaces` kural dışı değildir (`fixtures/kcad/v2/broken/scenario-*.kcad`, `replaces-*.kcad`).

**Süzgeç** (şema 35; ADR 0211 §2): harita; ikisinden en az biri (`broken/filter-empty.kcad`).

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `objects` | dizi: dizgi | | nesnelerin kalıcı kimlikleri (16 baytlık dizgi, §6.8): yalnız bunlar geçer; 1–100 000 kimlik, her biri bir kez, sıfır kimlik yok |
| `expression` | metin | | İfadeyle seç'in dilinde koşul (ADR 0100): doğru olan nesne geçer; kırpılmış, 1–10 000 karakter |

İkisi birden varsa nesne ikisinden de geçmelidir. Boş liste yazılmaz (`broken/filter-objects-empty.kcad`); kimliği 16 bayt olmayan (`broken/filter-object-short.kcad`), sıfır (`broken/filter-object-nil.kcad`) ya da iki kez geçen liste (`broken/filter-objects-twice.kcad`), boş ya da baştan veya sondan boşluklu (`broken/filter-expression-blank.kcad`) ve 10 000 karakterden uzun ifade (`broken/filter-expression-long.kcad`), grubun (`broken/filter-on-group.kcad`) ve servisten çizilen katmanın süzgeci (`broken/filter-on-service-layer.kcad`) ve bilinmeyen anahtar (`broken/filter-unknown-field.kcad`) reddedilir. Çizimde olmayan bir nesnenin kimliği kural dışı değildir (nesne silinmiş olabilir). İfadenin derlenmesi okuyucunun kuralı değildir: derlenmeyen ifadeyi uygulama okur ve hiçbir nesneyi geçirmez, komutlar yazmaz.

**Eylem** (şema 40; ADR 0225 §4): harita; `name` (metin, evet: menüdeki ve karttaki ad; kırpılmış, 1–100 karakter, denetim karaktersiz, katmanda büyük küçük harften bağımsız bir kez) ve `target` (metin, evet: `http`, `https` ya da `mailto` adresi ya da dosya yolu; şablon; boş değil, en çok 4 096 karakter, denetim karaktersiz). Katmanda en çok 32 eylem bulunur; sıraları anlamlıdır ve korunur. Boş dizi (`broken/actions-empty.kcad`), aynı adlı iki eylem (`broken/action-name-twice.kcad`), boş hedef (`broken/action-target-blank.kcad`), grubun (`broken/actions-on-group.kcad`) ve servisten çizilen katmanın eylemi (`broken/actions-on-service-layer.kcad`) `bad_value`, hedefsiz eylem `missing_field`'dır (`broken/action-without-target.kcad`). Anahtarların kodlanmış sırası `name` < `target`'tır.

**Bilgi kartı** (şema 40; ADR 0225 §5): harita; hepsi isteğe bağlı.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `text` | metin | | alanların altındaki satırlar; şablon; boş değil, en çok 10 000 karakter, satır sonu dışında denetim karaktersiz |
| `title` | metin | | kartın başlığı; şablon; yoksa nesnenin türü ve etiketi; boş değil, en çok 1 000 karakter, tek satır |
| `fields` | dizi: metin | | gösterilen öznitelikler, sırasıyla; yoksa nesnenin bütün öznitelikleri; boş değil, en çok 200 ad, her biri bir kez, boş ad yok |
| `actions` | bool | | yalnız `false`: katmanın eylemleri kartta gösterilmez |
| `attachments` | bool | | yalnız `false`: nesnenin ekleri kartta gösterilmez |

Açık düğme yazılmaz (`broken/card-switch-on.kcad`); boş alan listesi (`broken/card-fields-empty.kcad`), iki kez geçen alan (`broken/card-fields-twice.kcad`), satır sonlu başlık (`broken/card-title-line-break.kcad`) ve grubun kartı (`broken/card-on-group.kcad`) `bad_value`, bilinmeyen anahtar `unknown_field`'dır (`broken/card-unknown-field.kcad`). Alanların katmanda bulunması denetlenmez. **Şablon**, metnin `{` ile `}` arasındaki her parçası nesne üstünde değerlendirilen bir ifade (İfadeyle seç'in dili), `{{` ve `}}` düz ayraç olan metindir (ADR 0225 §6); okuyucu şablonu bölmez ve derlemez.

**Katman çizimi** (şema 41; ADR 0226 §3, §4): harita; anahtarların hepsi isteğe bağlı, ama en az biri yazılır. Yalnız bir şey söyleyen değer yazılır: tam opak katmanın saydamlığı, normal kip ve kapalı maske kaynağı yazılmaz (kodlanmış baytlar biriciktir).

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `mask` | maske | | bu katmanın etiketlerinin ve işaretlerinin altında başka katmanlar boşalır (aşağıda) |
| `blend` | numaralı metin | | karışım kipi: `multiply`, `screen`, `overlay`, `darken`, `lighten`, `colorDodge`, `colorBurn`, `hardLight`, `softLight`, `difference`, `exclusion`, `addition`; `normal` yazılmaz (`broken/compose-blend-normal.kcad`), bilinmeyen kip `bad_value` (`broken/compose-blend-unknown.kcad`) |
| `opacity` | float | | saydamlığın tersi, 0 ≤ o < 1; 1 yazılmaz (`broken/compose-opacity-one.kcad`), aralık dışı `bad_value` (`broken/compose-opacity-above-one.kcad`) |

**Maske**: harita; `labels` ve `markers` (bool: katmanın yerleşen etiketleri, çizilen işaretleri; yalnız `true` yazılır, en az biri), `size` (float, evet: şekillerin büyüdüğü ve köşelerinin yuvarlandığı boy, ekranda mm, 0–10) ve `targets` (dizi: metin, evet: maskelenen katmanların kimlikleri; 1–64, her biri bir kez, boş değil). Boş harita (`broken/compose-empty.kcad`) ve grubun katman çizimi (`broken/compose-on-group.kcad`) `bad_value`, bilinmeyen anahtar `unknown_field`'dır (`broken/compose-unknown-field.kcad`). Kaynaksız maske (`broken/mask-without-source.kcad`), `false` yazılan kaynak (`broken/mask-switch-off.kcad`), 10 mm'den büyük boy (`broken/mask-size-above-ten.kcad`), boş hedef listesi (`broken/mask-targets-empty.kcad`) ve iki kez geçen hedef (`broken/mask-target-twice.kcad`) `bad_value`, hedefsiz maske `missing_field`'dır (`broken/mask-without-targets.kcad`). Maskenin hedeflerinin ağaçta bulunması denetlenmez: silinen, birleştirilen ya da başka çizimden gelen katmanın kimliği kalabilir; çizilirken ağaçta olmayan, grup olan ya da katmanın kendisi olan hedef atlanır (komutlar yazarken hedefleri denetler). Anahtarların kodlanmış sırası `mask` < `blend` < `opacity`, maskeninki `size` < `labels` < `markers` < `targets`'tır.

**Katman keneti** (şema 10; ADR 0163 §4): iki biçimden **tam biri**.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `off` | bool | | yalnız `true`: katmanın nesnelerine kenetlenilmez |
| `kinds` | dizi: numaralı metin | | yalnız bu türlerle kenetlenilir (genel türlerle kesişimi): `endpoint` (çeyrek noktalarını da getirir), `midpoint`, `center`, `node`, `intersection`, `perpendicular`, `tangent`, `nearest`, `centroid`, `extension`, `parallel`, `grid` |

İkisi birden, ikisi de yok, `off: false`, boş, yinelenen ya da bilinmeyen tür `bad_value`'dur (`fixtures/kcad/v2/broken/layer-snap-*.kcad`). Kesişim kenedi iki nesneden birinin katmanında kesişim türü açıksa çalışır.

**Alan** (şema 26; ADR 0199 §1): katmanın nesnelerinin bir özniteliği; adı özniteliğin anahtarıdır. Değerler nesnelerin `attrs`'ında metin olarak kalır; dosya biçimi değerleri alanlarla denetlemez (kurala uymayan değer okunur ve korunur).

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `max` | metin | | sayı alanının en büyük değeri, türünün tek biçiminde |
| `min` | metin | | sayı alanının en küçük değeri, türünün tek biçiminde |
| `kind` | numaralı metin | evet | `text`, `integer`, `decimal`, `date`, `boolean` |
| `name` | metin | evet | özniteliğin anahtarı |
| `alias` | metin | | tabloda ve formda gösterilen ad |
| `scale` | u32 | | ondalık sayının en çok kesir basamağı (0–15) |
| `length` | u32 | | metnin en çok karakteri (1–10 000) |
| `values` | dizi: harita | | değer listesi: her biri `code` ve `label` metni (ikisi de zorunlu) |
| `default` | metin | | yeni nesnenin değeri, alanın kurallarına uyan tek biçimde |
| `required` | bool | | yalnız `true`: nesne alanı boş bırakamaz |

Anahtarlar kodlanmış sırasıyladır. Tek biçimler ve alan listesinin kuralları ADR 0199 §1'dedir (`kentos_contracts::fields`): ad boş değil, başında ya da sonunda boşluk yok, en çok 64 karakter, denetim karakteri yok, Türkçe katlanarak bir kez; takma ad boş değil, en çok 64 karakter; uzunluk yalnız metinde, ondalık basamak yalnız ondalıkta, aralık yalnız sayılarda (uçları tek biçimde, en az en çoktan büyük değil), değer listesi yalnız metin ve sayılarda (boş değil, kodlar boş değil ve sayılarda tek biçimde, kodlar ve katlanmış etiketler bir kez); varsayılan alanın kurallarına uyan tek biçim. Kuralı kıran liste, boş liste, `required: false`, bilinmeyen tür ve negatif sayı `bad_value`'dur; bilinmeyen anahtar `unknown_field`, türü ya da etiketi olmayan `missing_field`'dır (`fixtures/kcad/v2/broken/layer-fields-*.kcad`).

**Servis** (şema 32; ADR 0208 §2): katmanın çizildiği harita servisi. Anahtarlar kodlanmış sırasıyladır; bayraklar yalnız `true` iken, diziler yalnız boş değilken yazılır (`broken/service-yflip-false.kcad`, `broken/service-layers-empty.kcad`, `broken/service-params-empty.kcad`).

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `url` | metin | evet | servisin adresi (XYZ'de karo şablonu: `{z}`, `{x}`, `{y}` ya da `{-y}`, ya da `{quadkey}`; `{s}` varsa `subdomains`); `http://` ya da `https://`, en çok 4096 harf; Google'da boş (adres oturumdan) |
| `bbox` | 4 ondalık | | servisin kapsamı WGS 84 derecesinde: batı, güney, doğu, kuzey (−180 ≤ batı ≤ doğu ≤ 180, −90 ≤ güney ≤ kuzey ≤ 90); yeteneklerden, “Servisin kapsamına yakınlaştır” için |
| `grid` | karo ızgarası | | WMTS, OGC API Tiles ve önbellekli ArcGIS'in ızgarası: `srid` u32 (0 değil) ve `matrices` (1–40 matris: `id` metin, `x0`, `y0`, `resolution` float, `tileWidth`, `tileHeight` 1–4096, `matrixWidth`, `matrixHeight` 1–2⁴⁰) |
| `kind` | numaralı metin | evet | `xyz`, `wms`, `wmts`, `ogcTiles`, `arcgis`, `google`, `vector` |
| `srid` | u32 | | istenen sistem (WMS'te zorunlu; 0 değil) |
| `style` | metin | | WMS ve WMTS'te stil, vektörde stilin adresi, Google'da harita türü (`roadmap`, `satellite`, `terrain`, `hybrid`) |
| `yFlip` | bool | | yalnız `true`: TMS gibi satırlar güneyden sayılır |
| `format` | metin | | istenen resim türü (`image/png`) |
| `layers` | dizi: metin | | WMS'te en az biri, WMTS'te tek biri; en çok 64, her biri 1–256 harf |
| `params` | dizi: harita | | ek parametreler: `name` (harf, rakam, tire, nokta, alt çizgi) ve `value` (en çok 1024 harf); en çok 32 |
| `preset` | metin | | hazır altlığın adı (`osm-standard`) |
| `dynamic` | bool | | yalnız `true`: ArcGIS'in ızgarasız `export`'u |
| `maxZoom` | u32 | | en büyük kat, en çok 30 |
| `minZoom` | u32 | | en küçük kat, `maxZoom`'dan büyük değil |
| `opacity` | float | | 0,1–1; yokluğu 1 |
| `version` | metin | | WMS'te `1.1.1` ya da `1.3.0` |
| `template` | metin | | OGC API Tiles'ın ve WMTS'in REST karo şablonu |
| `tileSize` | u32 | | karonun pikseli, 64–4096 |
| `matrixSet` | metin | | WMTS'in matris kümesinin adı (WMTS'te zorunlu) |
| `connection` | metin | | projenin bağlantılarından birinin `id`'si (Google'da zorunlu) |
| `subdomains` | dizi: metin | | `{s}`'in değerleri, en çok 16 |
| `attribution` | metin | | servisin kaynak notu, en çok 512 harf |
| `transparent` | bool | | yalnız `true`: saydam zemin istenir |

Kurallar sözleşmenindir (`kentos_contracts::service::ServiceLayer::problem`); kıran servis `bad_value`'dur (`fixtures/kcad/v2/broken/service-*.kcad`). Servis katmanında nesne bulunmaz (`broken/service-holds-object.kcad`).

**Veri kaynağı** (şema 32; ADR 0208 §10): katmanın nesnelerinin alındığı yer; nesneler katmanda kalır, kaynak yalnız yeniden almanın tarifidir.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `key` | metin | | nesneleri eşleyen özniteliğin adı |
| `url` | metin | evet | servisin ya da dosyanın adresi, `http://` ya da `https://` |
| `bbox` | 4 float | | istenen alan, kaynağın sisteminde: en küçük doğu, kuzey, en büyük doğu, kuzey |
| `kind` | numaralı metin | evet | `wfs`, `ogcFeatures`, `arcgis`, `geojson` |
| `name` | metin | | WFS'in türü, OGC API'nin koleksiyonu, ArcGIS'in katmanı (üçünde zorunlu) |
| `srid` | u32 | | istenen sistem (0 değil) |
| `limit` | u64 | | en çok nesne, 1–500 000 |
| `filter` | metin | | süzgeç (CQL ya da `where`), en çok 8192 harf |
| `fetched` | metin | | son alınış, RFC 3339 |
| `version` | metin | | yalnız WFS'te: `1.0.0`, `1.1.0`, `2.0.0` |
| `connection` | metin | | projenin bağlantılarından birinin `id`'si |

Bir katmanda servis ve kaynak birlikte bulunmaz (`broken/service-and-feed.kcad`); ikisi de grupta `bad_value`'dur (`broken/service-on-group.kcad`, `broken/feed-on-group.kcad`). Andıkları bağlantı projenin olmalıdır (`broken/service-unknown-connection.kcad`, `broken/feed-unknown-connection.kcad`).

**Katman stili:**

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `fill` | metin | | dolgu rengi |
| `color` | metin | evet | onaltılık renk ya da tema adı (`fg`, `fg-dim`, `ink`, `paper`) |
| `label` | etiket stili | | |
| `labels` | etiketleme | | yalnız şema 36 ve sonrası: katmanın etiketlemesi (aşağıda); grupta `bad_value` (`broken/labels-on-group.kcad`) |
| `point` | nokta stili | | |
| `lineType` | numaralı metin | evet | `continuous`, `dashed`, `dashdot`, `dotted` |
| `renderer` | opak değer (§6.7), `null` olamaz | | stil motorunun çizicisi |
| `lineWeight` | float | evet | çıktı çizgi kalınlığı, mm |
| `linePattern` | metin | | şema 56'da: katmanın desenli çizgi tipinin adı (`lineTypes`'tan); `lineType` en yakın yerleşik tiptir (ADR 0313 §1) |
| `pickInterior` | bool | | içinden seçilebilir mi |

**Nokta stili:** `size` (float, evet; CSS px çapı), `symbol` (numaralı metin, evet: `ring`, `cross`, `triangle`).

**Etiket stili:**

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `ink` | numaralı metin | | `fg`, `fg-dim`, `label` |
| `grow` | float | | |
| `size` | float | evet | CSS px |
| `weight` | u16 | | 400, 500, 600 |
| `maxSize` | float | | |
| `maxScale` | float | | |
| `minScale` | float | | |
| `template` | metin | | |
| `placement` | numaralı metin | evet | `center`, `corner`, `beside`, `along` |
| `minFeaturePx` | float | | |

**Etiket stilinin motor anahtarları** (yalnız şema 36 ve sonrası; ADR 0212 §2; hepsi isteğe bağlı, yoklukları bugünkü davranış):

| Anahtar | Tür | Değerler |
|---|---|---|
| `text` | metin | metnin ifadesi (İfadeyle seç'in dili); varsa `template`'in yerine |
| `color` | metin | yazının rengi: `#rrggbb`, `#rrggbbaa` ya da `fg`, `fg-dim`, `label`, `ink`, `paper` |
| `italic` | bool | |
| `align` | numaralı metin | `left`, `center`, `right` (çok satırda) |
| `point` | numaralı metin | `around`, `center` |
| `line` | numaralı metin | `parallel`, `curved`, `horizontal`, `contour` |
| `area` | numaralı metin | `horizontal`, `free`, `perimeter`, `boundary`, `parcel`, `corner` |
| `position` | numaralı metin | `on`, `above`, `below`, `sides` |
| `distance` | float | px, 0–500 |
| `repeat` | float | px, 20–100 000 |
| `maxAngle` | float | derece, 5–90 |
| `curved`, `mergeLines`, `inside`, `outside` | bool | |
| `halo` | harita | `width` (float, evet; px, 0–10), `color` |
| `background` | harita | `shape` (evet: `rect`, `round`, `ellipse`), `fill`, `stroke`, `padding` (px, 0–50) |
| `shadow` | harita | `dx`, `dy` (float, evet; px, ±50), `color`, `opacity` (0–1) |
| `callout` | harita | `kind` (evet: `straight`, `manhattan`), `color`, `width` (px, 0,1–10), `minLength` (px, 0–1000) |
| `stack` | harita | `mode` (evet: `ifNeeded`, `always`), `chars` (u32, evet; 2–500), `at` (bölme karakterleri, 1–20 harf) |
| `abbreviate` | harita | `always` (bool), `words` (evet: 1–500 harita, her biri `word` ve `short`, 1–100 harf, boşluksuz; sözcük bir kez) |
| `shrink` | float | 0,5–1 |
| `priority` | u8 | 0–10 |
| `overlap` | numaralı metin | `never`, `ifNeeded`, `always` |
| `duplicates` | float | px, 1–10 000 |

Motor anahtarı olan stil bütün kurallarıyla denetlenir (`kentos_contracts::labels::style_problem`; boy 1–200, kalınlık 100–900, renkler ve aralıklar yukarıdaki gibi); kıran stil `bad_value`'dur (`broken/label-size-out.kcad`, `broken/label-color-bad.kcad`, `broken/label-halo-wide.kcad`, `broken/label-shrink-out.kcad`, `broken/label-priority-high.kcad`, `broken/label-stack-chars.kcad`, `broken/label-abbreviate-empty.kcad`, `broken/label-abbreviate-twice.kcad`, `broken/label-text-blank.kcad`). Haritalarda bilinmeyen anahtar `unknown_field`'dır (`broken/label-unknown-engine-field.kcad`).

**Etiketleme** (yalnız şema 36 ve sonrası; ADR 0212 §2):

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `mode` | numaralı metin | evet | `single` (tek etiket: stilin `label`'ı ya da türün varsayılanı), `rules` (kurallı), `off` (etiketsiz) (`broken/labels-mode-unknown.kcad`) |
| `classes` | dizi: sınıf | | kurallı etiketlemenin sıralı sınıfları, 1–64; yalnız `rules`'ta (`broken/labels-rules-without-classes.kcad`, `broken/labels-rules-empty.kcad`, `broken/labels-classes-in-single.kcad`) |
| `obstacle` | harita | | katmanın nesneleri öbür etiketlere engel: `weight` (u8, evet; 1–10; `broken/labels-obstacle-weight.kcad`), `kind` (`interior` ya da `boundary`; yokluğu `interior`) |

**Sınıf:** `name` (metin, evet; 1–100 harf, başında ve sonunda boşluk yok, katmanda bir kez: `broken/labels-class-name-blank.kcad`, `broken/labels-class-twice.kcad`), `when` (metin; koşulun ifadesi, 1–10 000 harf, boşluklu sınırsız: `broken/labels-class-when-blank.kcad`), `style` (etiket stili, evet). Başka anahtar `unknown_field`'dır (`broken/labels-unknown-field.kcad`).

### 6.6 Nesneler

- Her nesne **tek anahtarlı bir haritadır**: anahtar nesnenin türü, değer türün alanlarını taşıyan haritadır: `{"polygon": {…}}`. Tür önce okunur, alanlar türe göre denetlenir.
  - Haritada tek anahtardan farklı sayı `bad_value`, bilinmeyen tür `unknown_kind`'dır. Tanınmayan geometri sessizce atlanmaz; dosya açılmaz (`DOM-06`).
  - Tür sürümü ayrı yazılmaz: bütün türler belge şeması sürümüyle (§6.1) birlikte sürümlenir.
- **Nesnenin çalışma yuvası (yerel `u32` kimliği) yazılmaz** (`FILE-05`). Okuyucu nesnelere dosya sırasıyla 1, 2, 3, … yuvalarını verir; yuva bir sonraki açılışta değişebilir, kalıcı kimlik değişmez.

**Her türde ortak alanlar:**

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `uid` | kimlik | evet | kalıcı nesne kimliği (§6.8); bir blok tanımının nesnesinde yoktur, orada bilinmeyen alandır (§6.9) |
| `attrs` | harita: metin → metin | evet | öznitelikler (boş olabilir); v2'de değerler metindir |
| `color` | metin | | renk; yoksa katmana göre |
| `label` | metin | | nesnenin etiketi |
| `symbol` | metin | | katman stilinin yerine kitaplık sembolü |
| `layerId` | metin | evet | nesnenin katmanı |
| `lineWeight` | float | | yalnız şema 3 ve sonrası: nesnenin kendi çizgi kalınlığı, kağıtta mm, `0` en ince çizgi; `0`…`100` dışı `bad_value`; yoksa katmana göre |
| `labelPins` | dizi: iğne | | yalnız şema 36 ve sonrası ve yalnız belgenin nesnesinde: elle sabitlenmiş, döndürülmüş ya da gizlenmiş etiketleri (aşağıda) |
| `attachments` | dizi: ek | | yalnız şema 40 ve sonrası ve yalnız belgenin nesnesinde: fotoğrafları ve belgeleri (aşağıda) |
| `lineType` | numaralı metin | | yalnız şema 48 ve sonrası: nesnenin kendi çizgi tipi, `continuous`, `dashed`, `dashdot`, `dotted` (başkası `bad_value`: `broken/look-line-type-unknown.kcad`); yoksa kaynağına göre (ADR 0260 §1) |
| `lineTypeScale` | float | | yalnız şema 48 ve sonrası: desenin çarpanı, 0'dan büyük, en çok 1000 (`broken/look-scale-zero.kcad`); yoksa 1 |
| `linePattern` | metin | | yalnız şema 56 ve sonrası: nesnenin kendi desenli çizgi tipinin adı; `lineType` en yakın yerleşik tipi taşır. Blok nesnesinde de (ADR 0313 §1, §6) |
| `transparency` | float | | yalnız şema 48 ve sonrası: 0 ile 0,9 arası pay (`broken/look-transparency-high.kcad`); yoksa kaynağına göre |
| `order` | tam sayı | | yalnız şema 48 ve sonrası ve yalnız belgenin nesnesinde: katmanının içindeki çizim sırası, −2³¹ ile 2³¹ − 1 arası (kayan noktalı sayı `wrong_type`: `broken/look-order-float.kcad`); yoksa 0 |
| `byLayer` | dizi: numaralı metin | | yalnız şema 48 ve sonrası ve yalnız bir blok tanımının nesnesinde (belgenin nesnesinde bilinmeyen alan, `unknown_field`: `broken/look-by-layer-in-drawing.kcad`): değersiz olup yerleştirmenin yerine kendi katmanını izleyen parçalar, `color` < `lineType` < `lineWeight` < `transparency` sırasıyla, birer kez, değeri olan parça hiç |
| `wipeout` | harita: maske | | yalnız şema 54 ve sonrası ve yalnız alanda (`polygon`; başka türde `bad_value`: `broken/wipeout-on-a-line.kcad`): alan çizim zemininin rengiyle dolar ve kendinden önce çizilenleri örter. Tek anahtarı `frame` (zorunlu, true ya da false): sınırı çizilir mi. Başka anahtar bilinmeyen alandır (`unknown_field`) |

**Etiket iğnesi** (şema 36; ADR 0212 §2): harita; `class` (metin: kurallı katmanın sınıfının adı, 1–100 harf; yokluğu nesnenin ilk etiketi), `at` (nokta: etiketin ortasının nesnenin çapasından uzaklığı, metre; çapa nesnenin etiket noktasıdır), `rotation` (float: derece, saat yönünün tersine; yalnız `at` ile), `hidden` (bool: yalnız `true`). İğne ya bir yerdir ya gizlidir (`broken/label-pin-nothing.kcad`); `rotation` `at`'sız (`broken/label-pin-rotation-alone.kcad`), `hidden: false` (`broken/label-pin-hidden-false.kcad`), aynı sınıfın iki iğnesi (`broken/label-pin-class-twice.kcad`), 10 000 km'den uzak yer (`broken/label-pin-far.kcad`), boş dizi (`broken/label-pins-empty.kcad`) `bad_value`, bilinmeyen anahtar `unknown_field`'dır (`broken/label-pin-unknown-field.kcad`). Dizide en çok 64 iğne bulunur. Bir blok tanımının nesnesinde iğne olmaz (`broken/label-pins-in-block.kcad`). Sınıfın adının katmanda bulunması denetlenmez: sınıfı kalmamış iğne hiçbir şey yapmaz. Anahtarların kodlanmış sırası `at` < `class` < `hidden` < `rotation`'dır.

**Ek** (şema 40; ADR 0225 §1): harita; `name` (metin, evet: görünen ad; boş değil, en çok 256 karakter, denetim karaktersiz) ve **tam bir** kaynak: `asset` (metin: proje stillerinde gömülü resmin öğe kimliği, §6.7), `file` (metin: bağlı dosyanın yolu, mutlak ya da projenin klasörüne göre) ya da `url` (metin: `http://` ya da `https://` adresi, boşluksuz); kaynağın metni boş değil, en çok 4 096 karakter (öğe kimliği 256), denetim karaktersiz; `note` (metin, isteğe bağlı: en çok 1 000 karakter, satır sonu dışında denetim karaktersiz). Dizide en çok 64 ek bulunur; sıraları anlamlıdır ve korunur. Boş dizi (`broken/attachments-empty.kcad`), iki kaynak (`broken/attachment-two-sources.kcad`), kaynaksız ek (`broken/attachment-no-source.kcad`), boş ad (`broken/attachment-name-blank.kcad`), `http` ya da `https` olmayan adres (`broken/attachment-ftp.kcad`) ve sekmeli not (`broken/attachment-note-control.kcad`) `bad_value`, adsız ek `missing_field` (`broken/attachment-without-name.kcad`), bilinmeyen anahtar `unknown_field`'dır (`broken/attachment-unknown-field.kcad`). Bir blok tanımının nesnesinde ek olmaz (`broken/attachments-in-block.kcad`). Öğe kimliğinin proje stillerinde bulunması ve bağlı dosyanın varlığı denetlenmez (dosya başka bilgisayarda olabilir; komutlar gömülü fotoğrafın öğesini denetler). Anahtarların kodlanmış sırası `url` < `file` < `name` < `note` < `asset`'tir.

**Türlere göre alanlar** (ortak alanlarla birlikte aynı haritada, sıralı):

| Tür | Alanlar |
|---|---|
| `point` | `p` nokta; `z` float (isteğe bağlı, kot); `parts` nokta dizisi (isteğe bağlı; yalnız şema 17 ve sonrası) |
| `line` | `a`, `b` nokta; `za`, `zb` float (isteğe bağlı; yalnız şema 4 ve sonrası: uçların kotu); `ma`, `mb` float (isteğe bağlı; yalnız şema 39 ve sonrası: uçların ölçüsü) |
| `polyline` | `pts` nokta listesi; `bulges` float dizisi (isteğe bağlı); `zs` kot listesi (isteğe bağlı; yalnız şema 4 ve sonrası); `ms` ölçü listesi (isteğe bağlı; yalnız şema 39 ve sonrası); `parts` parça dizisi (isteğe bağlı; yalnız şema 17 ve sonrası); `closed` yalnız `true` ve `widths` genişlik listesi (isteğe bağlı; yalnız şema 46 ve sonrası) |
| `polygon` | `pts` nokta listesi; `bulges` float dizisi (isteğe bağlı); `holes` halka dizisi (isteğe bağlı); `zs` kot listesi (isteğe bağlı; yalnız şema 4 ve sonrası); `parts` parça dizisi (isteğe bağlı; yalnız şema 5 ve sonrası) |
| `circle` | `c` nokta; `r` float |
| `arc` | `c` nokta; `r`, `a0`, `a1` float (radyan, saat yönünün tersine `a0` → `a1`) |
| `ellipse` | `c`, `major` nokta; `ratio`, `t0`, `t1` float |
| `spline` | `pts` nokta listesi; `closed` bool |
| `nurbs` | yalnız şema 47 (ADR 0259): `degree` u32 (1–11); `ctrl` nokta listesi; `knots` float dizisi; isteğe bağlı: `weights` float dizisi, `closed` ve `periodic` yalnız `true`, `fit` nokta listesi, `tangents` iki öğeli dizi (nokta ya da null), `fitTolerance` float (aşağıda) |
| `xline`, `ray` | `p`, `dir` nokta |
| `text` | `p` nokta; `text` metin; `height` float (m); `rotation` float (derece, doğudan saat yönünün tersine); yalnız şema 7 ve sonrası: `align` numaralı metin, `widthFactor` float, `mask` bool (isteğe bağlı; aşağıda); yalnız şema 18 ve sonrası ve yalnız belgenin yazısında: `labelOf` kimlik, `labelScale` float (isteğe bağlı, birlikte; aşağıda); yalnız şema 20 ve sonrası: `boxWidth`, `lineSpacing` float, `runs` dizi (isteğe bağlı; aşağıda); yalnız şema 21 ve sonrası: `textStyle` metin, `font` numaralı metin, `bold`, `italic` bool, `oblique` float (isteğe bağlı; aşağıda); yalnız şema 25 ve sonrası: `path` harita (isteğe bağlı; aşağıda) |
| `dimension` | `a`, `b` nokta; `offset`, `height` float; isteğe bağlı: `c` nokta, `text` metin, `angle` float, `style` numaralı metin (`aligned`, `linear`, `angular`, `radius`, `diameter`; şema 9'da `ordinate`, `arcLength`, `jogged`, `azimuth`, `slope`), şema 9'da `mask` bool, `za`, `zb` float (aşağıda); şema 21'de `dimStyle` metin, `arrow`, `textPlace`, `unit`, `font` numaralı metin, `arrowSize`, `extOffset`, `extBeyond`, `textGap` float, `decimals` u32, `prefix`, `suffix` metin; şema 30'da `dimLineColor`, `extColor`, `textColor` metin, `dimLineWeight`, `extWeight` float, `dimLineType`, `extLineType` numaralı metin (aşağıda); şema 43'te ve yalnız belgede isteğe bağlı `refs` harita (aşağıda) |
| `hatch` | `ring` nokta listesi; `holes` nokta listesi dizisi (isteğe bağlı); `pattern` harita: `type` (`solid`, `lines`, `cross`; şema 23'te `pattern`, `gradient`), `angle` float, `spacing` float; şema 23'te isteğe bağlı `name` metin, `scale` float, `lines` aile dizisi, `gradient` harita (aşağıda); şema 23'te ve yalnız belgede isteğe bağlı `assoc` harita (aşağıda) |
| `insert` | yalnız şema 6 (§6.9): `block` kimlik (tanımın `id`'si); `p` nokta; `scale` float (pozitif; değilse `bad_value`); `rotation` float (radyan, doğudan saat yönünün tersine); `mirror` bool (isteğe bağlı; yalnız `true` yazılır, `false` `bad_value`) |
| `leader` | yalnız şema 8 (ADR 0146): `pts` nokta listesi (en az iki); `height` float (m, pozitif); `rotation` float (derece, doğudan saat yönünün tersine); isteğe bağlı: `text` metin, `arrow` numaralı metin, `mask` bool, şema 30'da `arrowSize` float (aşağıda) |
| `image` | yalnız şema 24 ve yalnız belgede (ADR 0192): `p` nokta (sol alt köşe); `width`, `height` float (m); `rotation` float (radyan, doğudan saat yönünün tersine, `p` çevresinde); isteğe bağlı: `mirror` bool, `asset` metin, `file` metin, `clip` nokta listesi, `opacity` float (aşağıda) |
| `raster` | yalnız şema 29 ve yalnız belgede (ADR 0204 §2): `affine` 6 float; `width`, `height`, `bands` u32; `sample` numaralı metin; `srid` u32; `style` harita; isteğe bağlı: `asset` metin, `file` metin, şema 31'de `url` metin, `opacity` float, şema 38'de `dataset` harita (aşağıda) |
| `pointcloud` | yalnız şema 31 ve yalnız belgede (ADR 0207 §3): `sources` dosya dizisi; `bounds` 6 float; `count` u64; `srid` u32; `style` harita; isteğe bağlı: `opacity` float (aşağıda) |
| `table` | yalnız şema 22 ve yalnız belgede (ADR 0184): `p` nokta (sol üst köşe); `rotation` float (derece, doğudan saat yönünün tersine); `height` float (hücrelerin yazı yüksekliği, m); `rows`, `columns` float dizisi (satırların yüksekliği yukarıdan aşağı, sütunların genişliği soldan sağa, m); `cells` metin dizisi dizisi (satır satır); isteğe bağlı: `merges` birleşik alan dizisi, `aligns` numaralı metin dizisi, `header` bool, `grid` numaralı metin, `frame` float, `textStyle` metin, `font` numaralı metin, `bold`, `italic` bool, `oblique` float, `source` harita (aşağıda) |

- **Halka** (`polygon`'un ya da parçanın deliği): `pts` nokta listesi (zorunlu), `bulges` float dizisi (isteğe bağlı), `zs` kot listesi (isteğe bağlı; yalnız şema 4 ve sonrası). Halkanın anahtarları kodlanmış sırasıyla `zs` < `pts` < `bulges`'tir.
- **Parça** (şema 5; ADR 0143): çok parçalı alanın ilk parçasının ötesindeki bir parçası. Alanın kendi `pts`, `bulges`, `holes` ve `zs`'i ilk parçadır; `parts` öbürlerini sırasıyla tutar. Parça haritası: `pts` nokta listesi (zorunlu), `bulges` float dizisi, `holes` halka dizisi, `zs` kot listesi (isteğe bağlı); alanlarının kuralları alanın kendi alanlarınınkiyle aynıdır. Anahtarları kodlanmış sırasıyla `zs` < `pts` < `holes` < `bulges`'tir.
  - Parçaların sırası anlamlıdır ve korunur. Okuyucu parçaların örtüşmesini denetlemez, deliklerin dış halkanın içinde olmasını denetlemediği gibi.
  - Yazıcı `parts`'ı verildiği gibi yazar, boş olanı da. Tek parçalı alanda alanı hiç üretmemek üreticinin işidir.
  - Şema 5–16'da yalnız `polygon` parça alır; `polyline`'da `parts` bilinmeyen alandır (`unknown_field`).
- **Çoklu çizginin parçası** (şema 17; ADR 0174): çok parçalı çoklu çizginin ilk parçasının ötesindeki bir parçası, alanın parçasıyla aynı haritadır: `pts` nokta listesi (zorunlu, en az 2 nokta; azı `bad_value`), `bulges` float dizisi, `zs` kot listesi (isteğe bağlı), şema 39'da `ms` ölçü listesi (isteğe bağlı; ADR 0224; anahtarları kodlanmış sırasıyla `ms` < `zs` < `pts` < `bulges`). `holes` yasaktır (`bad_value`: çizginin deliği olmaz). Çoklu çizginin kendi `pts`, `bulges` ve `zs`'i ilk parçadır; parçalar açıktır, sırası anlamlıdır ve korunur, birbirine değmeleri denetlenmez.
- **Noktanın parçası** (şema 17; ADR 0174): çok noktalı nesnenin ilk noktasının ötesindeki bir noktası: `p` nokta (zorunlu), `z` float (isteğe bağlı, kot). Anahtarları kodlanmış sırasıyla `p` < `z`'dir. Noktanın kendi `p` ve `z`'si ilk noktadır; öznitelikleri ve etiketi bütün noktalarındır.
- **Bağlı yazı** (şema 18; ADR 0175 §4): `labelOf` yazının etiketini yazdığı nesnenin kalıcı kimliğidir (§6.8'in biçiminde 16 bayt, sıfır değil; değilse `bad_value`), `labelScale` etiketin yazıldığı ölçeğin paydasıdır (1:N; sonlu ve sıfırdan büyük; NaN ve sonsuz `non_finite`, sıfır ve eksi `bad_value`). İkisi birlikte yazılır; yalnız biri `bad_value`'dur. Kimliğin çizimde bir nesneyi adlandırması denetlenmez: nesnesi olmayan bağ hiçbir şeyi izlemez. Anahtarların kodlanmış sırası `labelOf` < `labelScale`'dir.
- **Çok satırlı yazı** (şema 20; ADR 0182 §1): metnin satır sonları (`\n`) her şemada metnindir. `boxWidth` satırların sözcük sözcük sarıldığı genişliktir (metre; sonlu ve sıfırdan büyük; NaN ve sonsuz `non_finite`, sıfır ve eksi `bad_value`). `lineSpacing` satırların taban çizgileri arasının 5/3 yükseklik başına çarpanıdır (DXF'in 44'ü; 0,25 ile 4 arası, dışı `bad_value`; 1, alanın yokluğuyla aynı anlamdadır ve KentOS'un komutları onu yazmaz). `runs` harflerin biçimleridir: boş olmayan dizi (boş dizi `bad_value`; dilimsiz yazıda alan yazılmaz), her öğe bir harita, anahtarları kodlanmış sırasıyla `end` < `bold` < `color` < `start` < `italic` < `script` < `underline`:
  - `start`, `end` tam sayı (zorunlu, 32 bit): dilimin ilk harfi ve sonuncusunun ötesi, metnin Unicode karakter sırasıyla (kod noktası; UTF-16 değil); `start < end ≤` harf sayısı, değilse `bad_value`.
  - `bold`, `italic`, `underline` bool: yalnız `true` yazılır (`false` `bad_value`); `script` numaralı metin `super` (üst simge) ya da `sub` (alt simge); `color` metin (nesnenin rengi gibi: `#RRGGBB` ya da tema adı; boş `bad_value`). En az biri yazılır: biçimsiz dilim `bad_value`'dur.
  - Dilimler sıralı ve ayrıdır (öncekiyle örtüşen ya da ondan önce başlayan `bad_value`); aynı biçimdeki bitişik iki dilim tek dilimdir (ikisi `bad_value`): biçimlerin tek yazılışı vardır.
- **Yazının yüzü** (şema 21; ADR 0183 §2): `textStyle` izlediği stilin kimliğidir (§6.4.4; boş `bad_value`: `broken/style-face-style-empty.kcad`; stil tablosunda olmayan kimlik geçerlidir ve stilsiz sayılır). `font` yazının çizildiği yazı tipidir (bilinmeyen değer `bad_value`: `broken/style-face-font-unknown.kcad`); yazı tipi olan yazı dik çizilir, olmayan bugünkü gibi projenin yazı tipiyle. `bold`, `italic` yalnız `true` yazılır (`false` `bad_value`: `broken/style-face-bold-false.kcad`), `oblique` −85 ile 85 arası ve 0 değildir (`broken/style-face-oblique-steep.kcad`, `broken/style-face-oblique-zero.kcad`); üçü yazı tipi olmadan `bad_value`'dur (`broken/style-face-bold-without-font.kcad`). Yazının yüksekliği ve genişlik çarpanı kendi alanlarıdır; stil onları uygulanınca verir.
- **Ölçünün görünüşü** (şema 21; ADR 0183 §3): `dimStyle` izlediği stilin kimliğidir (boş `bad_value`). `arrow` `closed`, `open`, `dot`, `none`'dır, yokluğu çentiktir (bilinmeyen `bad_value`: `broken/style-look-arrow-unknown.kcad`). `arrowSize`, `extOffset`, `extBeyond`, `textGap` ölçünün `height`'ının katıdır (ok boyu sıfırdan büyük: `broken/style-look-arrow-size-zero.kcad`; öbürleri 0 ya da büyük: `broken/style-look-gap-negative.kcad`; en çok 100: `broken/style-look-gap-too-wide.kcad`); yoklukları bugünkü boylardır (çentik 0,6, ok ve nokta 1, boşluk ve aşma 0,5, değer 0,35). `textPlace` yalnız `centre` (`broken/style-look-place-unknown.kcad`), `decimals` en çok 8 (`broken/style-look-decimals-nine.kcad`), `unit` `m`, `cm`, `mm` (`broken/style-look-unit-unknown.kcad`), `prefix` ve `suffix` boş değil, en çok 32 harf, satır sonu ve denetim karakteri yok (`broken/style-look-prefix-empty.kcad`, `broken/style-look-suffix-line-break.kcad`), `font` değerin yazı tipidir.
- **Ölçünün çizgileri** (şema 30; ADR 0205 §6): `dimLineColor`, `extColor`, `textColor` ölçü çizgisinin (okları ve çentikleriyle), uzatma çizgilerinin ve değerin rengidir, `#RRGGBB` (`broken/dimension-line-colour-not-hex.kcad`); yoklukları nesnenin rengidir. `dimLineWeight`, `extWeight` kâğıttaki kalınlıktır, mm, 0 ile 100 arası (`broken/dimension-ext-weight-negative.kcad`); 0 kılcaldır, yoklukları nesnenin kalınlığıdır. `dimLineType`, `extLineType` `continuous`, `dashed`, `dashdot`, `dotted`'dır (`broken/dimension-line-type-unknown.kcad`); yoklukları sürekli çizgidir. Ölçü stilinden gelirler ve stil değişince görünüşün öbür alanları gibi izlerler (ADR 0183 §4).
- **Yay değeri** (`bulges`): DXF'teki gibi `tan(θ/4)`, saat yönünün tersi artı; `bulges[i]` `pts[i] → pts[i+1]` kenarınındır, kapalı şekilde son değer kapanış kenarınındır. Yaylar, delikler, elips ve eğri parametreleri tanım olarak saklanır; ekranda çizilen üçgen ya da kısa parçalar dosyaya girmez (`FILE-08`).
- **Köşe kotları** (şema 4; ADR 0142):
  - **Anlamı:** metre, projenin düşey datumunda; sonlu bir float, eksi olabilir (deniz altı, kazı), aralık sınırı yoktur. Hangi yükseklik olduğu (ortometrik, elipsoidal) proje ayarının işidir, dosya biçiminin değil.
  - **Çizgi:** `za` başlangıcın, `zb` bitişin kotudur, her biri kendi başına isteğe bağlıdır. Kotsuz ucun anahtarı hiç yazılmaz (§5.2); `null` yazılmaz ve okunmaz (`wrong_type`).
  - **Çoklu çizgi, alan ve delik:** `zs`, `pts` ile **aynı uzunlukta**, köşe başına bir öğedir: sonlu float ya da `null`. `null` kotsuz köşedir; 0 değildir, “kot yok” ayrı bir durumdur. Opak kısımlar (§6.7) dışında şemalı bir alanda yazılan tek `null` budur. Uzunluğu `pts`'ten farklı bir `zs` `bad_value`'dur (yolu `…/zs`).
  - **Okuma sırası:** `zs` anahtarı `pts`'ten önce geldiği için okuyucu uzunluğu haritanın sonunda, iki alanı da okuyunca denetler.
  - **Yazıcı** `zs`'i verildiği gibi yazar, hepsi `null` olan da: yazılan baytlar okununca çizimle aynı çıkmalıdır. Hiçbir köşenin kotu yoksa alanı hiç üretmemek üreticinin işidir (ADR 0142).
  - **Sonlu olmayan kot** (NaN, ±∞) hiçbir float gibi yazılamaz ve okunamaz: `non_finite` (§5.3). Kotun yerinde tam sayı ya da başka tür `wrong_type`'tır.
  - **Şema:** kot alanları şema 4'te gelir (§6.1). Nokta kendi `z`'sini önceki şemalardan beri taşır; daire, yay, elips, eğri, yardımcı çizgi, ışın, yazı, ölçü ve tarama kot almaz.
- **Köşe ölçüleri** (M, şema 39; ADR 0224 §1): rotanın köşelerindeki doğrusal referans değeridir (km, metre ya da kurumun kendi birimi; birim rotanındır, dosya söylemez). Kotların kurallarıyla aynıdır: çizginin `ma` başlangıcın, `mb` bitişin ölçüsüdür, ölçüsüz ucun anahtarı yazılmaz (`null` `wrong_type`: `broken/line-measure-null.kcad`); çoklu çizginin ve parçasının `ms`'i `pts` ile aynı uzunlukta, köşe başına sonlu float ya da `null`'dır (farklı uzunluk `bad_value`: `broken/measure-wrong-length.kcad`, `broken/part-measure-wrong-length.kcad`). Değer eksi olabilir, köşeden köşeye artması ya da azalması gerekmez (kalibrasyon atlaması, ters yönlü rota). `ms` anahtarı `zs`'ten ve `pts`'ten önce gelir. Ölçü yalnız çizgide ve çoklu çizgidedir: alanda, nokta, yay ve öbür türlerde bilinmeyen alandır.
- **NURBS eğrisi** (`nurbs`, şema 47; ADR 0259 §2): kaynak `degree`, `ctrl`, `knots` ve `weights`'tir; sayılar yazıldığı gibi saklanır. Kural `nurbs_problem`'dir (sözleşme): derece 1–11 (`broken/nurbs-degree-12.kcad`), en az derece + 1 denetim noktası, `knots` denetim noktası + derece + 1 tane (`broken/nurbs-knot-count.kcad`), sonlu (`non_finite`: `broken/nurbs-knot-nan.kcad`), azalmayan (`broken/nurbs-knots-decreasing.kcad`), ilk ve son aralığı boş olmayan, bir iç düğüm en çok derece kez (`broken/nurbs-inner-knot.kcad`); `weights` denetim noktası kadar (`broken/nurbs-weight-count.kcad`), her biri sıfırdan büyük (`broken/nurbs-weight-zero.kcad`). `closed` ve `periodic` yalnız `true` yazılır (`broken/nurbs-closed-false.kcad`); `tangents` tam iki öğedir (`broken/nurbs-tangents-three.kcad`). `fit`, `tangents` ve `fitTolerance` bilgidir, çizilen eğri denetim noktalarınındır. Anahtarların kodlanmış sırası `fit` < `ctrl` < `knots` < `closed` < `degree` < `weights` < `periodic` < `tangents` < `fitTolerance`'tır.
- **Görünüş** (şema 48; ADR 0260): `lineType`, `lineTypeScale` ve `transparency` her nesnede, `order` yalnız belgenin nesnesinde, `byLayer` yalnız bir blok tanımının nesnesinde bulunur. Değer yokluğu nesnenin yerine göre okunur: belgede katmanın, blokta yerleştirmenin değeri; `byLayer`'daki parça blokta da nesnenin kendi katmanınındır. Ortak alanların kodlanmış sırası `order` < `byLayer` < `lineType` < `transparency` < `lineTypeScale`'dir. **Maske** (şema 54; ADR 0310): `wipeout` belgenin ve blok tanımının alanında bulunur.
- **Kapalı ve genişlikli çoklu çizgi** (şema 46; ADR 0258): `closed` yalnız `true` yazılır (`false` `bad_value`: `broken/polyline-closed-false.kcad`); kapalı çizginin kapanan kenarının bükümü `bulges`'ın sonuncusudur, en az 2 köşesi olur. `widths` köşe başına bir çift, `[başlangıç, bitiş]` iki float'tır, çizim uzunluğunda (metre; yerel projenin birimi okumada ve yazmada); `pts` ile aynı uzunlukta (farklı `bad_value`: `broken/polyline-widths-count.kcad`), çift iki sayıdır (`broken/polyline-widths-three.kcad`), sayılar sonlu (`non_finite`: `broken/polyline-widths-nan.kcad`) ve sıfır ya da büyüktür (`bad_value`: `broken/polyline-widths-negative.kcad`). Açık çizginin son köşesinden kenar çıkmaz: son çift `[0, 0]`'dır (`broken/polyline-widths-open-last.kcad`); hepsi sıfır olan liste yazılmaz (`broken/polyline-widths-all-zero.kcad`). Kapalı ya da genişlikli çizgi tek parçalıdır: `parts` ile birlikte `bad_value` (`broken/polyline-closed-parts.kcad`). Genişlik kâğıdın çizgi kalınlığı (`lineWeight`) değildir; ikisi ayrı alandır. Anahtarların kodlanmış sırası `ms` < `zs` < `pts` < `parts` < `bulges` < `closed` < `widths`'tir.
- **Yerleştirme** (`insert`, şema 6; ADR 0144): tanımın nesneleri, tanımın taban noktasından `p`'ye taşınmış, `mirror` ise tanımın x ekseninde aynalanmış, `scale` ile ölçeklenmiş ve `rotation` kadar döndürülmüş hâliyle çizilir. Dönüşüm benzerliktir; şekiller türlerini korur. Anahtarları kodlanmış sırasıyla `p` < `block` < `scale` < `mirror` < `rotation`'dır (ortak alanlarla birlikte sıralanır). Tanımın öznitelik tanımlarının değerleri yerleştirmenin `attrs`'ındadır.
- **Eğri boyunca yazı** (şema 25; ADR 0196 §1): `path` yazının harflerinin üstünde durduğu eğridir, yazının kendi çerçevesinde (başlangıcı `p`, x ekseni `rotation` doğrultusunda, y ekseni ona dik ve solda; metre): `pts` nokta listesi `p`'den sonraki köşelerdir (ilk köşe `p`'nin kendisidir, yazılmaz), isteğe bağlı `bulges` float listesi her kenarın kavisidir (kenar i, köşe i'den i + 1'e; DXF'in tan(θ/4)'ü, saat yönünün tersi artı; yokluğu bütün kenarlar düz). Başka alan `unknown_field` (`broken/text-path-unknown-field.kcad`), `pts`'siz harita `missing_field`'dır (`broken/text-path-without-pts.kcad`). `bad_value`: köşesi yok (`broken/text-path-empty.kcad`), kavis sayısı köşe sayısından farklı (`broken/text-path-bulges-count.kcad`), bütün köşeleri `p`'de (uzunluğu sıfır; `broken/text-path-zero-length.kcad`), yazı tek satır değil: satır sonu, `boxWidth` ya da `lineSpacing` (`broken/text-path-line-break.kcad`, `broken/text-path-box.kcad`), yazı bir nesneye bağlı (`labelOf`, `broken/text-path-linked.kcad`). Sonlu olmayan float `non_finite`'tir. Harflerin eğri üstündeki yeri çizimin işidir, dosyanınki değil.
- **Yazı ekleri** (şema 7; ADR 0145):
  - **`align`**: `p`'nin yazının hangi noktası olduğu. Değerler: `baselineCenter`, `baselineRight`, `bottomLeft`, `bottomCenter`, `bottomRight`, `middleLeft`, `middleCenter`, `middleRight`, `topLeft`, `topCenter`, `topRight`. Yatayda sol, orta, sağ; düşeyde taban çizgisi, alt (taban çizgisinin 0,2 yükseklik altı), orta (yarım yükseklik üstü), üst (bir yükseklik üstü). Alan yoksa `p` taban çizgisinin soludur; bu bir değer değildir, `baselineLeft` bilinmeyen değerdir (`bad_value`). Başka bilinmeyen değer de `bad_value`'dur.
  - **`widthFactor`**: harflerin eni bununla çarpılır, yükseklik değişmez. `0`'dan büyük, en çok `100`; değilse `bad_value`, sonlu değilse `non_finite`. Alan yoksa 1'dir; yazıcı verileni yazar, 1'i üretmemek üreticinin işidir.
  - **`mask`**: yazının kutusu yazıdan önce çizim alanının zemin rengiyle doldurulur. Yalnız `true` yazılır; `false` `bad_value`'dur.
  - Yazının anahtarları kodlanmış sırasıyla `p` < `mask` < `text` < `align` < `height` < `rotation` < `widthFactor`'dır (ortak alanlarla birlikte sıralanır).
- **Kılavuz** (`leader`, şema 8; ADR 0146): ok başı, kırık çizgi, kol ve not tek nesnedir.
  - **`pts`**: köşeler; ilki okun ucu, sonuncusu kolun başladığı yer. İkiden az köşe `bad_value`'dur.
  - **`text`**: tek satırlık not. Yoksa kılavuz yalnız oktur: kol ve not yoktur. Boş metin `bad_value`'dur; notsuzluk alanın yokluğudur.
  - **`height`**: notun yüksekliği, metre. Ok başı ve kol bundan ölçülür (ADR 0146 §2). 0 ya da eksi `bad_value`, sonlu değilse `non_finite`.
  - **`rotation`**: notun ve kolun doğrultusu, derece (yazınınki gibi).
  - **`arrow`**: ok başı. Değerler: `open` (açık ok), `dot` (dolu nokta), `none` (ok başı yok); şema 30'da AutoCAD'in öbürleri (ADR 0205 §7): `closed` (boş üçgen), `open30` (ince açık ok), `open90` (dik açık ok), `dotSmall` (küçük nokta), `dotBlank` (boş nokta), `oblique` (eğik çizgi), `archTick` (mimari çentik), `boxFilled` (dolu kare), `boxBlank` (boş kare), `datumFilled` (dayanak üçgeni). Alan yoksa dolu üçgendir; bu bir değer değildir, `filled` bilinmeyen değerdir (`bad_value`, `broken/leader-arrow-unknown.kcad`).
  - **`arrowSize`** (şema 30; ADR 0205 §7): ok başının boyu, notun `height`'ının katı, 0,1 ile 10 arası (`broken/leader-arrow-size-too-small.kcad`, `broken/leader-arrow-size-too-big.kcad`); yokluğu 1'dir.
  - **`mask`**: notun kutusu nottan önce çizim alanının zemin rengiyle doldurulur (yazınınki gibi). Yalnız `true` yazılır; `false` `bad_value`'dur.
  - Kılavuzun anahtarları kodlanmış sırasıyla `pts` < `mask` < `text` < `arrow` < `height` < `rotation` < `arrowSize`'dır (ortak alanlarla birlikte sıralanır). Kılavuz kot almaz.
- **Yeni ölçüler** (`dimension`, şema 9; ADR 0147). Alanların türe göre anlamı ADR 0147 §1'in tablosundadır:
  - **`ordinate`** (koordinat): `a` nokta, `b` çizginin ucu; `angle` 0 (Y, noktanın doğusu) ya da 90 (X, kuzeyi), yoksa 0. Başka bir `angle` `bad_value`'dur.
  - **`arcLength`** (yay uzunluğu): `c` yayın merkezi, `a`'dan `b`'ye saat yönünün tersine yay; `offset` ölçü yayının yaydan uzaklığı. `c` yoksa `missing_field`.
  - **`jogged`** (kırıklı yarıçap): `a` gerçek merkez, `b` yayın üstünde, `c` çizginin başladığı gösterilen merkez, `offset` kırığın `c`'den uzaklığı. `c` yoksa `missing_field`.
  - **`azimuth`** (semt): `a`'dan `b`'ye doğrultu; `offset` yazının kenardan işaretli uzaklığı.
  - **`slope`** (eğim): `a` ile `b` arasındaki eğim; `za` ve `zb` iki noktanın kotu, metre, ikisi de zorunlu (`missing_field`). Başka bir türde `za` ya da `zb` `bad_value`'dur.
  - **`mask`**: ölçünün değeri çizim alanının zemin rengiyle doldurulan kutusunun üstünde çizilir, her türde. Yalnız `true` yazılır; `false` `bad_value`'dur.
  - Ölçünün anahtarları kodlanmış sırasıyla `a` < `b` < `c` < `za` < `zb` < `mask` < `refs` < `text` < `angle` < `style` < `height` < `offset`'tir (ortak alanlarla birlikte sıralanır).
- **Ölçünün başvuruları** (`refs`, şema 43; ADR 0249 §1, §4): ölçünün `a`, `b` ve `c` noktalarının izlediği nesne yerleri; başvurusu olmayan nokta serbesttir. Yalnız belgenin ölçüsünde (`broken/dimension-refs-in-block.kcad`).
  - **`refs`** harita: isteğe bağlı `a`, `b`, `c`, her biri bir başvuru; şema 60'ta isteğe bağlı `rule` numaralı metin (`curve`, `edges`; ADR 0320 §1: yer anılmayan noktaların nasıl izlediği; başka ad `bad_value`, `broken/dimension-rule-unknown.kcad`; şema 59 ve öncesinde `unknown_field`, `broken/dimension-rule-in-schema-59.kcad`). Başvurusu olmayan noktanın anahtarı yazılmaz; hiç başvuru yoksa alanın kendisi yazılmaz, boş harita `bad_value`'dur (`broken/dimension-refs-empty.kcad`). Başka anahtar `unknown_field`'dır (`broken/dimension-refs-unknown-field.kcad`). Anahtarların sırası `a` < `b` < `c` < `rule`.
  - **Başvuru** harita: `uid` nesnenin kalıcı kimliği (§6.8'in biçiminde, 16 bayt; sıfır kimlik `bad_value`: `broken/dimension-refs-zero-uid.kcad`; kimliğin çizimde bir nesneyi adlandırması denetlenmez), `place` yer haritası, isteğe bağlı `broken` metin: başvurunun koptuğu neden, `object_gone`, `kind_changed`, `no_such_place`, `vertex_gone`, `ambiguous`'tan, şema 51'de `piece_gone`'dan biri (başkası `bad_value`: `broken/dimension-refs-unknown-broken.kcad`); kopmamış başvuruda yazılmaz. Şema 51'de isteğe bağlı `path` işaretsiz tam sayı dizisi: `uid` bir yerleştirmeyken yerin bulunduğu parçanın yolu, her sayı o düzeydeki tanımın `entities`'indeki sıra (ADR 0263 §3); 1 ile 16 öğe (boş `bad_value`: `broken/dimension-refs-path-empty.kcad`; 16'dan uzun `bad_value`: `dimension-refs-path-deep.kcad`), nesnenin kendisi için yazılmaz. Kopan noktanın koordinatları ölçünün `a`, `b`, `c`'sinde kalır. Anahtarların sırası `uid` < `path` < `place` < `broken`.
  - **Yer** harita: `kind` metin ve türün sayıları, işaretsiz tam sayı, en çok 10 000 000 (`SUBREF_INDEX_LIMIT`; değilse `bad_value`): `node` (nokta; `part`: 0 noktanın kendisi, `k` `k`'ıncı ek noktası), `start`, `end` (çizginin, yayın, elips yayının başı ve sonu), `center` (dairenin, yayın, elipsin merkezi) sayısız; `vertex` (çoklu çizginin, alanın köşesi) ve `mid` (kenarın ortası; çizgide üçü de 0) `part`, `ring` (0 dış halka, `h` `h`'ıncı delik), `index`; şema 51'de `along` (kenar ya da eğri boyunca yer; ADR 0263 §1) `part`, `ring`, `index` ve `t` float, 0 ile 1 arasında (değilse `bad_value`: `broken/dimension-refs-along-t-above-one.kcad`). Bilinmeyen tür `bad_value`'dur (`broken/dimension-refs-unknown-kind.kcad`); türün almadığı bir sayı `unknown_field`, aldığı sayının yokluğu `missing_field`'dır. Anahtarların sırası `t` < `kind` < `part` < `ring` < `index`.
- **Tablo** (`table`, şema 22; ADR 0184): satırlar ve sütunlar, sol üst köşesinden asılı, onunla döner; hücreler tek satırdır.
  - **Boyutlar:** `height`, her satırın yüksekliği ve her sütunun genişliği sonlu, sıfırdan büyük ve en çok 10⁶ m'dir; en az 1, en çok 10 000 satır, en az 1, en çok 100 sütun, en çok 100 000 hücre (değilse `bad_value`; sonlu olmayan float `non_finite`).
  - **`cells`:** `rows` kadar satır, her satırda `columns` kadar metin (`broken/table-cells-short.kcad`); boş metin boş hücredir. Hücrede satır sonu ya da denetim karakteri (Unicode `Cc`) yoktur, en çok 1000 harf (`broken/table-cell-line-break.kcad`). Bunlar `bad_value`'dur.
  - **`merges`:** birleşik alanlar: her öğe `row`, `col`, `rows`, `cols` tam sayılı (32 bit) bir harita; alan tablonun içindedir ve birden çok hücredir (`broken/table-merge-outside.kcad`), öncekilerle örtüşmez; sol üst hücresi dışındaki hücreleri boştur: alanın yazısı sol üst hücresindedir (`broken/table-merge-hides-words.kcad`). Boş dizi yazılmaz (`broken/table-merges-empty.kcad`). Hepsi `bad_value`.
  - **`aligns`:** sütun başına `left`, `center` ya da `right`; sütun sayısı kadar (`broken/table-aligns-short.kcad`). Yokluğu hepsinin sola dayalıdır.
  - **`header`:** ilk satır başlıktır: kalın ve ortalı çizilir. Yalnız `true` yazılır (`false` `bad_value`: `broken/table-header-false.kcad`).
  - **`grid`:** çizilen çizgiler: `outer` yalnız dış çizgi, `rows` dış çizgi ve satırlar arası, `none` hiçbiri. Yokluğu hepsidir; bu bir değer değildir, `all` bilinmeyen değerdir (`broken/table-grid-all.kcad`). Bir birleşik alanın içindeki çizgiler çizilmez.
  - **`frame`:** kalın çerçeve: dış çizgi bu genişlikte, tablonun içine doğru dolu bir bant olarak çizilir (m). Sıfırdan büyük ve tablonun eninin ve boyunun yarısından küçüktür (`broken/table-frame-too-wide.kcad`); `grid` `none` iken çizilmez.
  - **Yüz:** `textStyle`, `font`, `bold`, `italic`, `oblique` yazınınkilerdir (şema 21'in kuralları; kalın, eğik ve yatık yazı tipiyle olur: `broken/table-bold-without-font.kcad`). Hücreler dik çizilir; yazı tipi yoksa projeninkiyle.
  - **`source`:** satırların geldiği yer (Tabloyu güncelle): `kind` `coordinates`, `areas` ya da `attributes` ise `objects` en az 1, en çok 100 000 kimliklik dizi (§6.8'in biçiminde; sıfır kimlik `bad_value`: `broken/table-source-nil.kcad`; kimliklerin çizimde bir nesneyi adlandırması denetlenmez); `file` ise `name` boş olmayan metin ve isteğe bağlı `sheet` (boş değil). Bir türün öbürünün alanları `bad_value`'dur (`broken/table-source-file-objects.kcad`). Anahtarların kodlanmış sırası `kind` < `name` < `sheet` < `objects`'tir.
  - Tablonun anahtarları kodlanmış sırasıyla `p` < `bold` < `font` < `grid` < `rows` < `cells` < `frame` < `aligns` < `header` < `height` < `italic` < `merges` < `source` < `columns` < `oblique` < `rotation` < `textStyle`'dır (ortak alanlarla birlikte sıralanır); birleşik alanınki `col` < `row` < `cols` < `rows`'tur. Tablo kot almaz.
- **Tarama ekleri** (`hatch`, şema 23; ADR 0186). `pattern`'in alanları türüne göredir; ilk üç tür (`solid`, `lines`, `cross`) yalnız `type`, `angle` ve `spacing` alır ve eskisi gibi okunur:
  - **`pattern`** (çizgi aileli desen): `name` boş olmayan (yalnız boşluk değil), en çok 64 harflik metin (kitaplığın ya da dosyanın adı: `ANSI31`); `scale` desen biriminin metresi, sıfırdan büyük, en çok 10⁶; `lines` en az 1, en çok 64 aile. Aile: `angle` float (derece, desenin dönüşünden önce), `origin` ve `offset` nokta biçiminde iki float (`[x, y]`; `offset` bir sonraki çizginin çizgi boyunca kayması ve çizgiye dik uzaklığı, çizginin kendi doğrultusunda; dik uzaklık 0 olamaz: `broken/hatch-pattern-lines-apart-zero.kcad`), isteğe bağlı `dashes` float dizisi (artı çizilir, eksi boşluk, 0 nokta; en çok 16, hepsi 0 olamaz; boş dizi yazılmaz: `broken/hatch-pattern-dashes-empty.kcad`; yokluğu bütün çizgidir). Sayıların mutlak değeri en çok 10⁶'dır. `angle` desenin dönüşüdür; `spacing` 1 yazılır ve okunmaz. Ailenin anahtarları kodlanmış sırasıyla `angle` < `dashes` < `offset` < `origin`'dir.
  - **`gradient`** (degrade): `gradient` harita zorunludur: `shape` (`linear`, `cylinder`, `spherical`), `color2` `#RRGGBB` metin (`broken/hatch-gradient-colour-name.kcad`), `inverted` bool, yalnız `true` yazılır (`false` `bad_value`: `broken/hatch-gradient-inverted-false.kcad`). İlk renk taramanın rengidir; `angle` degradenin doğrultusudur; `spacing` 1 yazılır ve okunmaz. Anahtarların kodlanmış sırası `shape` < `color2` < `inverted`'dır.
  - Bir türün öbürünün alanları `bad_value`'dur (`broken/hatch-lines-with-name.kcad`); bu denetimler ürün komutlarınınkidir (`HatchPattern::problem`). Desenin anahtarları kodlanmış sırasıyla `name` < `type` < `angle` < `lines` < `scale` < `spacing` < `gradient`'tir.
  - **`assoc`** (taramanın izlediği nesneler, yalnız belgede): `outer` kapalı nesnenin kalıcı kimliği (§6.8'in biçiminde), isteğe bağlı `islands` ve `cutouts` kimlik dizileri (adalar; boş bırakılan yazılar ve yerleştirmeler; boş dizi yazılmaz: `broken/hatch-assoc-islands-empty.kcad`), `seed` nokta (tohum, sonlu). Bir kimlik iki kez geçmez (`broken/hatch-assoc-twice.kcad`), en çok 100 000 nesne; sıfır kimlik `bad_value`'dur; kimliklerin çizimde bir nesneyi adlandırması denetlenmez. Anahtarların kodlanmış sırası `seed` < `outer` < `cutouts` < `islands`'dır.
  - Taramanın anahtarları kodlanmış sırasıyla `ring` < `assoc` < `holes` < `pattern`'dir (ortak alanlarla birlikte sıralanır).
- **Resim** (`image`, şema 24; ADR 0192): bir PNG ya da JPEG, sol alt köşesinden, genişliği ve yüksekliğiyle yerleşir, köşesi çevresinde döner.
  - **Boyut:** `width` ve `height` sıfırdan büyük, en çok 10⁷ m'dir (`broken/image-width-zero.kcad`); sonlu olmayan float `non_finite`'tir.
  - **Kaynak:** ya `asset` (projenin stillerindeki PNG ya da JPEG öğenin kimliği: **gömülü**) ya `file` (dosyanın yolu, mutlak ya da çizim dosyasının klasörüne göre: **bağlı**), ikisinden yalnız biri; ikisi birden (`broken/image-two-sources.kcad`) ya da hiçbiri (`broken/image-no-source.kcad`) `bad_value`'dur, boş ya da yalnız boşluk olan metin yok sayılır. `file` en çok 4096 harftir, denetim karakteri (Unicode `Cc`) içermez (`broken/image-file-line-break.kcad`). `asset`'in stillerde bir öğeyi adlandırması denetlenmez (stiller opaktır, §6.7): bulunamayan resim yerine açık gri çizilir.
  - **`mirror`:** resim kendi x ekseninde ters (Aynala'nın sonucu, blok yerleştirmesinin aynası gibi); yalnız `true` yazılır (`false` `bad_value`: `broken/image-mirror-false.kcad`).
  - **`clip`:** kırpma sınırı, resmin kendi kesirleriyle (sol alt 0,0, sağ üst 1,1; aynalı resimde de resmin kendisininkiyle): en az 3, en çok 10 000 köşe, her köşe 0 ile 1 arasında (`broken/image-clip-outside.kcad`, `broken/image-clip-two-corners.kcad`). Yokluğu bütün resimdir. Okuyucu halkanın yönünü ve kendini kesmesini denetlemez; KentOS'un araçları onu saat yönünün tersine, en alttaki (eşitse en soldaki) köşesinden yazar.
  - **`opacity`:** donukluk 0,1 ile 1 arası (`broken/image-opacity-low.kcad`); yokluğu 1'dir.
  - Bunlar ürün komutlarının denetimleridir (`ImageFields::problem`). Resmin anahtarları kodlanmış sırasıyla `p` < `clip` < `file` < `asset` < `width` < `height` < `mirror` < `opacity` < `rotation`'dır (ortak alanlarla birlikte sıralanır). Resim kot almaz.
- **Raster** (`raster`, şema 29; ADR 0204): bir ortofoto, taranmış pafta ya da yükseklik modeli; pikselleri dosyasında, yalnız yeri ve görünüşü belgede.
  - **`affine`:** altı float `[x₀, a, b, y₀, c, d]`: sütun i ve satır j (piksel köşeleri; sol üst 0, 0) x = x₀ + a·i + b·j, y = y₀ + c·i + d·j'dedir (GDAL'ın geotransform sırası). Altıdan başka sayı (`broken/raster-affine-five.kcad`) ve tersinmeyen dönüşüm (a·d − b·c sıfır, `broken/raster-affine-flat.kcad`) `bad_value`'dur.
  - **Boyut:** `width`, `height` 1 ile 4 000 000 piksel (`broken/raster-width-zero.kcad`), `bands` 1 ile 255 arası; `sample` `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32` ya da `f64` (`broken/raster-sample-unknown.kcad`). `srid` dosyanın sistemidir (0: dosya söylemiyordu, kullanıcı projeninkini onayladı).
  - **Kaynak:** resminki gibi ya `asset` (projenin stillerindeki öğe: **gömülü**) ya `file` (dosyanın yolu: **bağlı**), şema 31'de ya da `url` (HTTP ya da HTTPS adresi, parça parça okunur: **adres**; ADR 0207 §1), yalnız biri (`broken/raster-two-sources.kcad`, `broken/raster-no-source.kcad`, `broken/raster-url-and-file.kcad`); `file` ve `url` en çok 4096 harf, denetim karakteri içermez; `url` `http://` ya da `https://` ile başlar (`broken/raster-url-ftp.kcad`). `asset`'in bir öğeyi adlandırması denetlenmez.
  - **`style`** (görünüş, ADR 0204 §4): `render` numaralı metin (`rgb`, `gray`, `palette`, `ramp`, `hillshade`, `rampShade`) ve `bands` u32 dizisi (1'den; `rgb` üç ya da dört, öbürleri bir: `broken/raster-rgb-two-bands.kcad`; rasterin bantlarından biri: `broken/raster-band-missing.kcad`) zorunlu; isteğe bağlı `stretch` (`minMax`, `percent`, `manual`; gerdirmesizlik alanın yokluğudur, `none` yazılmaz: `broken/raster-stretch-none.kcad`), `min`, `max` float (`manual`'da ikisi birden, en küçük en büyükten küçük: `broken/raster-manual-upside-down.kcad`), `ramp` metin (`Gri`, `Arazi`, `Spektral`, `Viridis`, `Mavi-kırmızı`, `Sıcaklık`: `broken/raster-ramp-unknown.kcad`), `invert` bool (yalnız `true`: `broken/raster-invert-false.kcad`), `azimuth` (0–360), `altitude` (0–90), `zFactor` (0'dan büyük) float (`broken/raster-light-below.kcad`), `nodata` float, `resampling` (`nearest`; çift doğrusal alanın yokluğudur, `bilinear` yazılmaz: `broken/raster-bilinear-written.kcad`), şema 38'de `edges` metin (mesh'in çizgilerinin rengi `#RRGGBB`: `broken/edges-not-a-colour.kcad`; yalnız mesh'i gösteren rasterde: `broken/edges-without-mesh.kcad`). Görünüşün anahtarları kodlanmış sırasıyla `max` < `min` < `ramp` < `bands` < `edges` < `invert` < `nodata` < `render` < `azimuth` < `stretch` < `zFactor` < `altitude` < `resampling`'dir.
  - **`opacity`:** donukluk 0,1 ile 1 arası (`broken/raster-opacity-low.kcad`); yokluğu 1'dir.
  - **`dataset`** (şema 37; ADR 0243 §6): rasterin gösterdiği NetCDF değişkeni ve dilimi; harita: `variable` metin zorunlu (değişkenin adı; vektörde x bileşeni), isteğe bağlı `vector` metin (y bileşeni: büyüklük gösterilir), `mesh` metin (UGRID ağının topoloji değişkeni; yokluğu CF ızgarası), `dims` dizi (dilim boyutları dosyadaki sırasıyla; boş liste yazılmaz: `broken/dataset-dims-empty.kcad`), `followTime` bool (yalnız `true`: `broken/dataset-follow-false.kcad`; zaman boyutu ister: `broken/dataset-follow-without-time.kcad`). Boyut haritadır: `name` metin, `index` u32 (gösterilen değer; değerlerin içinde: `broken/dataset-index-out.kcad`), `values` float dizisi (1–100 000 sonlu değer; zamanda ana zamanlar, 1970'ten milisaniye), isteğe bağlı `time` bool (yalnız `true`; en çok bir boyutta: `broken/dataset-two-times.kcad`; değerleri azalmaz: `broken/dataset-time-falling.kcad`) ve `units` metin. Adlar 1–256 harf, denetim karakteri içermez (`broken/dataset-variable-empty.kcad`); en çok 8 boyut. Veri setli rasterin bandı 1'dir (`broken/dataset-three-bands.kcad`). Anahtarların kodlanmış sırası `dims` < `mesh` < `vector` < `variable` < `followTime`, boyutunkiler `name` < `time` < `index` < `units` < `values`'dir.
  - Bunlar ürün komutlarının denetimleridir (`RasterFields::problem`, `RasterStyle::problem`). Rasterin anahtarları kodlanmış sırasıyla `url` < `file` < `srid` < `asset` < `bands` < `style` < `width` < `affine` < `height` < `sample` < `opacity`'dir (ortak alanlarla birlikte sıralanır). Raster kot almaz.
- **Nokta bulutu** (`pointcloud`, şema 31; ADR 0207): bir ya da birden çok LAS, LAZ, COPC ya da XYZ dosyası (birden çoksa **sanal bulut**); noktaları dosyalarında, belgede yalnız nerede oldukları, ne tuttukları ve nasıl gösterildikleri.
  - **`sources`:** 1 ile 4096 dosya (`broken/pointcloud-no-files.kcad`). Her dosya bir haritadır: kaynağı ya `asset` (projenin stillerindeki öğe: **gömülü**) ya `file` (yolu, mutlak ya da çizimin klasörüne göre: **bağlı**) ya `url` (HTTP ya da HTTPS adresi: **adres**), yalnız biri (`broken/pointcloud-two-sources.kcad`, `broken/pointcloud-no-source.kcad`); `file` ve `url` en çok 4096 harf, denetim karakteri içermez, `url` `http://` ya da `https://` ile başlar (`broken/pointcloud-url-ftp.kcad`). `format` numaralı metin (`las`, `laz`, `copc`, `xyz`; `broken/pointcloud-format-unknown.kcad`), `count` u64 (başlığın dediği nokta sayısı), `bounds` 6 float `[x₁, y₁, z₁, x₂, y₂, z₂]` (başlığın dediği kapsam; her eksende en küçük en büyükten büyük değil: `broken/pointcloud-bounds-upside-down.kcad`; altıdan başka sayı: `broken/pointcloud-bounds-five.kcad`). Dosyanın anahtarları kodlanmış sırasıyla `url` < `file` < `asset` < `count` < `bounds` < `format`'tır.
  - **`bounds`, `count`:** dosyaların birlikte kapsamı ve nokta sayısı; sayı dosyalarınkinin toplamıdır (`broken/pointcloud-count-not-sum.kcad`). `srid` dosyaların sistemidir (0: dosyalar söylemiyordu, kullanıcı projeninkini onayladı).
  - **`style`** (görünüş, ADR 0207 §5): `render` numaralı metin (`rgb`, `classification`, `elevation`, `intensity`, `returns`, `single`) ve `size` float (0,5 ile 32: `broken/pointcloud-size-zero.kcad`) zorunlu; isteğe bağlı `ramp` metin (rasterin rampalarından biri; yokluğu `Arazi`), `invert` bool (yalnız `true`: `broken/pointcloud-invert-false.kcad`), `min`, `max` float (ikisi birden, en küçük en büyükten küçük; `elevation` ve `intensity`'de zorunlu: `broken/pointcloud-ramp-without-range.kcad`), `hidden` u8 dizisi (çizilmeyen sınıflar, küçükten büyüğe ve birer kez: `broken/pointcloud-hidden-unsorted.kcad`; boş liste yazılmaz: `broken/pointcloud-hidden-empty.kcad`), `rgb8` bool (renkler 16 bitlik alanlarda 0–255 ise; yalnız `true`), `sizeUnit` (`m`: boy zeminde metre; piksel alanın yokluğudur, `px` yazılmaz: `broken/pointcloud-px-written.kcad`), `shape` (`square`; yuvarlak alanın yokluğudur, `round` yazılmaz: `broken/pointcloud-round-written.kcad`). Görünüşün anahtarları kodlanmış sırasıyla `max` < `min` < `ramp` < `rgb8` < `size` < `shape` < `hidden` < `invert` < `render` < `sizeUnit`'tir.
  - **`opacity`:** donukluk 0,1 ile 1 arası (`broken/pointcloud-opacity-low.kcad`); yokluğu 1'dir.
  - Bunlar ürün komutlarının denetimleridir (`PointCloudFields::problem`, `PointCloudStyle::problem`). Bulutun anahtarları kodlanmış sırasıyla `srid` < `count` < `style` < `bounds` < `opacity` < `sources`'tır (ortak alanlarla birlikte sıralanır). Bulut kot almaz; taşınmaz, döndürülmez, ölçeklenmez (noktaları dosyasındadır).
- Dış başvuru, yüzey ve katı gibi yeni türler ileride şemaya ya da zorunlu bir uzantıya eklenir; eski okuyucu onları tanımadığını söyler.

### 6.7 Proje stilleri ve opak değerler

`styles` harita: `items` (dizi, evet) ve `categories` (dizi, evet). Öğeleri ve katman stilinin `renderer`'ı **opak değerlerdir**: stil motorunun kendi yapısıdır, dosya biçimi onları yorumlamaz, olduğu gibi taşır (sözleşme ADR 0002). Opak değer JSON'la gösterilebilen bir CBOR değeridir:

| JSON | CBOR |
|---|---|
| `null` | `F6` |
| `true`, `false` | `F5`, `F4` |
| tam sayı (−2⁶³ … 2⁶⁴−1) | ana tür 0 ya da 1; aralık dışı `bad_value` |
| diğer sayılar | `FB` binary64 |
| metin | ana tür 3 |
| dizi | ana tür 4 |
| nesne | ana tür 5; anahtarlar metin ve kodlanmış sırasıyla |

- Opak kısımda **tam sayı ile float ayrıdır**: `2` ve `2.0` farklı değerlerdir ve farklı baytlara kodlanır. Bayt dizgisi opak kısımda bulunamaz (`wrong_type`).
- Opak kısımlar da profilin bütün kurallarına (sıra, derinlik, sınırlar) uyar.

### 6.8 Kimlikler ve göç kaynağı

- **`uid`**: her nesnenin kalıcı kimliği (ADR 0014). Yeni nesnede UUIDv7, v1'den göçte UUIDv5; okuyucu sürüme bakmaz. Bir dosyada **benzersizdir** (`duplicate_uid`) ve boş olamaz. Kaydetme ve açma kimliği değiştirmez; düzenleme korur, yeni nesne yeni kimlik alır.
- **`projectId`**: projenin kalıcı kimliği, varsa. v1 dosyasından göçte `UUIDv5(ad_alanı, "project")` (ADR 0014), v2 dosyasından okunanda dosyadaki değer. 2.0'da yeni bir proje kimlik almaz.
- **`migratedFrom`**: çizim bir v1 dosyasından açılıp v2 olarak kaydedildiyse göçün kaynağı. Sonraki kayıtlarda da korunur; kimliklerin nereden geldiğini söyler.

| Anahtar | Tür | Değer |
|---|---|---|
| `format` | metin | `"kentos.document"` |
| `version` | tam sayı | `1` |
| `sourceSha256` | özet | v1 dosyasının kanonik metninin SHA-256'sı (ADR 0014, dilim 2): v1 kimliklerinin ad alanı bundan türer |

2.0'da tanımlı tek göç v1'den olduğu için başka `format` ve `version` değerleri `bad_value`'dur. Nesnelerin eski yerel kimlikleri ayrıca yazılmaz: aynı v1 dosyasından türetilen kimlikler zaten dosyadaki `uid`'lerdir.

### 6.9 Bloklar

Şema 6'da belgenin `blocks` alanı blok tanımlarının dizisidir (ADR 0144). Her tanım bir haritadır (anahtarlar kodlanmış sırasıyla):

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | kimlik | evet | tanımın kalıcı kimliği; yerleştirmeler onu gösterir |
| `base` | nokta | evet | taban noktası, tanımın kendi koordinatlarında |
| `name` | metin | evet | tanımın adı |
| `entities` | dizi: nesne (§6.6) | evet | tanımın nesneleri, sırasıyla; `uid`'leri yoktur |
| `attributes` | dizi: öznitelik tanımı | | yerleştirmenin yazı olarak gösterdiği öznitelikler; boş dizi yazılmaz (`bad_value`) |
| `description` | metin | | açıklama |
| `units` | metin | | yalnız şema 57 ve sonrası: çizildiği birim, `mm`, `cm`, `m`, `in`, `ft` ya da `unitless` (başka söz `bad_value`, `broken/block-units-unknown.kcad`); birimli blok gerçek boyutludur, birimsiz bloğun sayıları çizim birimidir (ADR 0315 §4) |

**Yerleştirmenin y ölçeği:** şema 57'de `insert` nesnesi `yScale` float taşıyabilir: pozitif (`broken/insert-y-scale-zero.kcad`) ve `scale`'den farklı; eşitse yazılmaz (`broken/insert-y-scale-equal.kcad`). Yoksa `scale` iki eksende geçerlidir; varsa parçalar afinle yerleşir (ADR 0315 §3). Şema 56 ve öncesinde `yScale` ve `units` `unknown_field`'dır.

**Öznitelik seçenekleri, yerleştirmenin yerleri ve bloğun kartı** (şema 58, ADR 0317; şema 57 ve öncesinde `unknown_field`: `broken/attribute-options-in-schema-57.kcad`, `broken/block-cards-in-schema-57.kcad`):

- Öznitelik tanımının bayrakları `invisible`, `constant`, `verify`, `preset`, `movable`, `multiline` yalnız `true` olarak yazılır; `false` `bad_value`'dur (`broken/attribute-option-false.kcad`). Sabit öznitelik `verify` ve `preset` taşımaz (`broken/attribute-constant-verify.kcad`). `boxWidth` float (m, pozitif) yalnız çok satırlıdadır (`broken/attribute-box-not-multiline.kcad`). `field` metin: boş olmayan alan şablonu (`{NO}/{TUR}`). Anahtarlar haritanın kanonik sırasındadır.
- `insert` nesnesi `attributePlaces` taşıyabilir: etiketten noktaya harita, tanımın koordinatlarında. Boş harita yazılmaz (`broken/insert-places-empty.kcad`); her etiket yerleştirdiği tanımın taşınabilir bir özniteliğidir (`broken/insert-place-not-movable.kcad`).
- Tanım `card` haritası taşıyabilir: `category` metin (`/` ile yol, boş bölümsüz), `tags` metin dizisi (boş değil; her biri Türkçe katlanarak bir kez, `broken/block-card-tag-twice.kcad`), `version` tamsayı (1 ve üstü, `broken/block-card-version-zero.kcad`). Örnek `block-cards.kcad`.

**Dinamik blok** (şema 59, ADR 0318; şema 58 ve öncesinde `unknown_field`: `broken/dynamic-blocks-in-schema-58.kcad`):

- Tanım `dynamic` haritası taşıyabilir: sözleşmenin `DynamicBlock`'unun JSON'u (noktalar `{x, y}` haritası, sayılar float, yerel kimlikler tam sayı; anahtarlar kanonik sırada). Bilinmeyen alan `unknown_field`'dır (`broken/dynamic-unknown-field.kcad`); varsayılan değerinde bir alan (boş liste, serbest kural) ya da float yerine tam sayı yazılmaz (`bad_value`). Kuralları sözleşmenin `dynamic_problem`'idir: eylemin parametresine uymayan türü (`broken/dynamic-action-unfit.kcad`), blokta olmayan nesne (`broken/dynamic-piece-unknown.kcad`), kurala uymayan varsayılan (`broken/dynamic-default-off-rule.kcad`), iki kez geçen ad (`broken/dynamic-name-twice.kcad`), tablonun kurala uymayan değeri (`broken/dynamic-lookup-off-rule.kcad`) `bad_value`'dur. Örnek `dynamic-blocks.kcad`.
- `insert` nesnesi `parameters` taşıyabilir: parametrenin adından değere harita (`true`/`false`, float, metin ya da `[x, y]`), yalnız varsayılandan farklılar. Boş harita yazılmaz (`broken/parameters-empty.kcad`); her ad yerleştirdiği dinamik tanımın tablo olmayan bir parametresidir ve değeri onun kuralına uyar (`broken/parameters-unknown.kcad`, `broken/parameters-off-rule.kcad`, `broken/parameters-not-dynamic.kcad`).

**Öznitelik tanımı** haritası: `p` nokta (evet), `tag` metin (evet), `align` numaralı metin (yalnız şema 7 ve sonrası), `value` metin (varsayılan değer), `height` float (evet, m), `prompt` metin (Blok ekle'nin sorusu), `rotation` float (evet, derece, bir yazınınki gibi), `widthFactor` float (yalnız şema 7 ve sonrası). Anahtarları bu sıradadır. `align` ve `widthFactor`'ın anlamı ve kuralları yazınınkidir (§6.6, yazı ekleri).

- **Tanımın nesneleri** belgenin nesneleriyle aynı biçimdedir (§6.6), yalnız `uid` taşımazlar: kimlikleri tanımın içinde yereldir. Okuyucu onlara tanımın içinde 1, 2, 3, … yuvalarını verir. Katmanları saklanır ama ağaçta bulunmaları gerekmez. Boş katman (`""`) bloğun katmanıdır: yerleştirmenin katmanında çizilir; ağaçta olan bir katmandaki nesne o katman görünürken görünür (ADR 0315 §2).
- **Kurallar** (okuyucu ve yazıcı aynı sırayla denetler; web ve masaüstü belgeleri de aynı kurallarla, `kentos_contracts::blocks`):
  1. Tanımlar sırasıyla: adı boş ya da yalnız boşluk olamaz (`bad_value`); kimliği (`duplicate_block`) ve adı (`duplicate_block`, Türkçe büyük/küçük harf ayrımı yapılmadan: I → ı, İ → i, öbür harfler Unicode'un küçük harfiyle) bir kezdir; öznitelik etiketi boş olamaz ve tanımda bir kezdir (`bad_value`).
  2. Tanımların nesnelerindeki yerleştirmeler bilinen bir tanımı gösterir (`unknown_block`); bir yerleştirme listede kendisinden sonra gelen bir tanımı da gösterebilir.
  3. Hiçbir tanım kendini doğrudan ya da başka tanımlar yoluyla içeremez (`block_cycle`); iç içelik en çok 16 düzeydir, içinde yerleştirme olmayan tanım bir düzeydir (`block_too_deep`). Derinlik önce aranır ve yığın 16'yı geçmez: 16'dan uzun bir döngü `block_too_deep` olarak söylenir. Bir tanım iç içe yerleştirmeleriyle (öznitelik tanımları dahil) bir milyondan çok parçaya açılamaz (`block_too_large`, ADR 0316 §4); sayı sığdan derine, doymalı toplamla hesaplanır.
  4. Belgenin kendi nesnelerindeki her yerleştirme bir tanımı gösterir (`unknown_block`).
- Hatanın yolu yerleştirmenin `…/insert/block` alanını ya da tanımın kendisini (`document/blocks/<sıra>`) gösterir.

### 6.10 Dosyaya girmeyenler

Çalışma yuvaları, seçim, geri alma ve yineleme geçmişi, kirli bayrağı, kamera, GPU tamponları, seçme ve kenet indeksleri, tarayıcı ve cihaz durumu (localStorage, IndexedDB taslakları), kullanıcı ve cihaz tercihleri dosyaya yazılmaz (`FILE-05`). Parola, belirteç ve imzalı adres yazılmaz (`SYNC-13`); servislerin bağlantılarının sırları da (§6.4.7). Servislerin karoları ve cevapları dosyaya yazılmaz; önbellekleri cihazındır (ADR 0208 §7).

### 6.11 Belge kuralları

Dosya biçimi geçerli olsa da uygulamalar açılışta çizimin kendi kurallarını ayrıca denetler (v1'deki gibi; web `model/snapshot.ts`, masaüstü `kentos_domain::Document`):

- en az bir katman (`type: layer`) olmalı; her nesnenin `layerId`'si bir katmanın (grubun değil) kimliği olmalı;
- `srid` uygulamanın tanıdığı bir koordinat sistemi ya da yerel sistem (0) olmalı (tahmin yapılmaz);
- `polyline`, `polygon` ve parça en az 2, delik ve `hatch` halkası en az 3, `spline` en az 2 nokta; `bulges` nokta sayısından uzun olamaz; çemberin yarıçapı pozitif;
- proje stilleri paylaşılan bir `.kstil` dosyası gibi denetlenir;
- pafta kitabı bir `.kpafta`'nın kitabı gibi pafta çekirdeğiyle okunur ve denetlenir (`kentos_sheet::validate::read_book`); okunamayan kitap açılışta söylenir, çizim açılır, kitap bir sonraki kayıtta olduğu gibi yazılır (§6.12).

Bu kurallar dosya biçiminin değil çizimin kurallarıdır; `tools/kcad/kcad.py` onları denetlemez.

### 6.12 Pafta kitabı

Şema 44 (ADR 0251). `sheets` harita:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `book` | opak harita (§6.7) | evet | pafta çekirdeğinin kitabı (`kentos.sheet/1`): paftalar, ana sayfalar, öğeler, değişkenler ve resimlerin bilgileri (`assets`: SHA-256, tür, ad, boy), kimlikleriyle; harita değilse `wrong_type` (`broken/sheets-book-not-a-map.kcad`) |
| `pictures` | dizi: resim | | kitabın resimlerinin baytları; resim yoksa yazılmaz, boş dizi `bad_value` (`broken/sheets-pictures-empty.kcad`) |

Resim harita:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `bytes` | bayt dizgisi | evet | dosyanın baytları (PNG, JPEG, SVG); en çok 2²⁴ bayt (§5.5) |
| `sha256` | özet | evet | `bytes`'ın SHA-256'sı; uymazsa `bad_value` (`broken/sheets-picture-digest.kcad`), 32 bayt değilse `bad_value` (`broken/sheets-picture-sha256-length.kcad`) |

- Aynı SHA-256 iki kez verilemez (`bad_value`, `broken/sheets-picture-twice.kcad`).
- Kitap dosya biçimine opaktır: dosya biçimi onu yorumlamaz, olduğu gibi taşır; tam sayı ile float ayrıdır (§6.7). Kitabın kendi kuralları (kimliklerin benzersizliği, öğelerin türleri, resimlerin bilgileri) pafta çekirdeğinindir (§6.11). Kitabın andığı ama `pictures`'ta olmayan resim paftanın ön denetiminde söylenir; `pictures`'ta olup kitabın anmadığı resim yazıcının işi değildir, yazan uygulama onu yazmaz.
- Sözleşmenin JSON'unda (`DocumentSnapshotV2.sheets`, `ProjectSheets`) `bytes` base64 metindir (RFC 4648, dolgulu), `sha256` 64 küçük onaltılık hanedir.
- Pafta kitabının geri alma geçmişi dosyaya girmez (§6.10). Pafta şablonları (cihazın ve bulutun kitaplığı) projenin değildir, dosyaya girmez; paftaya kullanılmış şablonun sayfası ve resimleri kitapta zaten vardır.

### 6.13 İstek defteri

Şema 45 (ADR 0253 §6). `requests` dizi: istek, en eskisi önce, en çok 256 (fazlası `bad_value`); boş dizi yazılmaz (`bad_value`, `broken/requests-empty.kcad`). İstek harita:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `answer` | metin | | o zamanki cevabın JSON'u (`CallAnswer`), en çok 65 536 bayt; daha büyüğü yazılmaz (`bad_value`) |
| `status` | metin | evet | `done` ya da `undone`: yazdığı çizimde mi, geri mi alındı; başkası `bad_value` (`broken/requests-status.kcad`) |
| `command` | metin | evet | komutun kimliği (`cad.entities.offset`); boş olamaz |
| `version` | tam sayı | evet | komutun sürümü, 1'den (`broken/requests-version-zero.kcad`) |
| `revision` | metin | evet | yazdıktan sonra çizimin sürümü, ondalık rakamlar (`broken/requests-revision.kcad`); oturumundur, açılan çizimin sürümüyle karşılaştırılmaz |
| `requestId` | metin | evet | isteğin kimliği: 1–128 karakter, yalnız ASCII harf, rakam ve `. _ : -` (`broken/requests-id.kcad`); aynı kimlik iki kez yazılamaz (`broken/requests-twice.kcad`) |
| `fingerprint` | bayt dizgisi | evet | çağrının parmak izi (komut, sürüm, biçim, girdi ve beklentilerin kanonik JSON'unun SHA-256'sının ilk 16 baytı); 16 bayt değilse `bad_value` (`broken/requests-fingerprint-length.kcad`) |

- Sözleşmenin JSON'unda (`DocumentSnapshotV2.requests`, `RequestRecord`) `fingerprint` 32 küçük onaltılık hanedir.
- Defter dosyanın bir parçasıdır, geri alma geçmişi değildir: oturumun kaydı (`kentos.calls/1`) ve geri alma adımları dosyaya girmez (§6.10). Veritabanı projesi defter tutmaz; sunucunun kendi idempotency'si vardır (ADR 0026).

### 6.14 İlişkili diziler

Şema 53 (ADR 0300). `arrays` dizi: ilişkili dizi, yapıldıkları sırayla. İlişkili dizi harita:

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `p` | nokta | evet | ilk yer: bloğun taban noktasının gittiği yer |
| `id` | kimlik | evet | dizinin kalıcı kimliği; iki dizide aynı kimlik `bad_value` |
| `block` | kimlik | evet | kaynak: bütün öğelerin yerleştirdiği blok tanımı; çizimde tanımlı değilse `bad_value` (`broken/arrays-block-unknown.kcad`) |
| `items` | dizi: öğe | evet | yer başına bir öğe, yerleşimin sırasıyla (ızgarada satır satır); sayısı yerlerinkinden başkaysa `bad_value` (`broken/arrays-items-short.kcad`) |
| `scale` | float | evet | ilk yerin ölçeği, sıfırdan büyük (`broken/arrays-scale-zero.kcad`) |
| `layout` | harita | evet | yerleşim (aşağıda) |
| `mirror` | bool | | ilk yer aynalıysa `true`; değilse yazılmaz |
| `layerId` | metin | evet | öğelerin katmanı; boş olamaz |
| `rotation` | float | evet | ilk yerin dönüşü, radyan |

Öğe harita: `uid` (kimlik; o yerdeki yerleştirmenin kalıcı kimliği, yoksa öğe elle silinmiştir) ve `kept` (bool, yalnız `true` yazılır: öğe elle değiştirilmiştir, yerleşim değişince yerinde kalır). Bir yerleştirme iki öğede olamaz (`broken/arrays-item-twice.kcad`); öğenin yerleştirmesi çizimin nesnelerinden biri olmalıdır (`broken/arrays-item-missing.kcad`).

Yerleşim harita, `kind`'ı ve yalnız kendi alanlarıyla (başkasının alanı `bad_value`):

| `kind` | Alanlar |
|---|---|
| `grid` | `dx`, `dy` (float), `cols`, `rows` (tam sayı): `rows` × `cols` yer |
| `polar` | `fill` (float, radyan), `count` (tam sayı), `center` (nokta), `rotate` (bool) |
| `line` | `dx`, `dy` (float), `fit` (bool, yalnız `true`), `count` (tam sayı) |
| `path` | `path` (kimlik: yolun kalıcı kimliği), `align` (bool), `count` (tam sayı), `spacing` (float, sıfırdan büyük; yoksa yerler yola eşit dağılır) |

- Yer sayısı 2 ile 10 000 arasındadır; sayılar sonludur.
- Sözleşmenin JSON'unda (`DocumentSnapshotV2.arrays`, `AssocArray`) kimlikler küçük harfli, tireli UUID metnidir; yolun kimliği de.
- Öğeler sıradan yerleştirmelerdir (§6.6); dizi onların üstünde bir kayıttır. Yerlerin hesabı Dizi komutununkidir (`array_transforms`, ADR 0047).

## 7. v1'den göç

- v1 dosyası (JSON, `kentos.document` sürüm 1) açılınca nesnelerin kalıcı kimlikleri içerikten türetilir (ADR 0014): aynı dosya her platformda aynı kimlikleri verir.
- Çizim v2 olarak kaydedilince kimlikler `uid` olarak, projenin türetilen kimliği `projectId` olarak, kaynak `migratedFrom` olarak yazılır. Web ve masaüstü aynı v1 dosyasını **aynı v2 baytlarına** çevirir (`fixtures/kcad/v2/migrated.kcad`).
- Özgün v1 dosyası göç doğrulanmadan ezilmez. Uygulamaların Kaydet ve Farklı kaydet davranışı ADR 0025'tedir.

## 8. Tür koklama

Dosyanın türü uzantısından değil içeriğinden anlaşılır (`FILE-03`):

| Sonuç | Kural | Ne olur |
|---|---|---|
| `kcad` | ilk 9 bayt imza | v2 okuyucusu (§4) |
| `kcad-damaged` | ilk 5 bayt `89 4B 43 41 44`, imzanın kalanı değil | v2 okuyucusu `damaged_signature` ya da `truncated` der |
| `json` | isteğe bağlı UTF-8 BOM (`EF BB BF`) ve JSON boşluklarından (`20 09 0A 0D`) sonra ilk bayt `{` | v1 okuyucusu; `format` ve `version` orada denetlenir |
| `empty` | BOM ve boşluktan başka bir şey yok | “dosya boş” |
| `foreign` | başka her şey | “KentOS çizim dosyası değil”; DXF, NCN gibi dosyalar İçe aktar'dan açılır |

## 9. Hata kodları

Okuyucular kodu sabit tutar (programlar ve örnek dosyalar için); ileti Türkçedir, nedeni ve çözümü söyler (CLAUDE.md §8).

| Kod | Aşama | Anlamı |
|---|---|---|
| `empty` | kap | dosya boş |
| `not_kcad` | kap | KCAD imzası yok |
| `damaged_signature` | kap | imza bozuk (metin aktarımı) |
| `truncated` | kap | dosya ya da başlık eksik |
| `trailing_data` | kap | dosyanın sonunda fazla bayt |
| `unsupported_version` | kap | ana sürüm 2 değil |
| `newer_version` | kap | daha yeni bir okuyucu gerekiyor |
| `bad_header` | kap | başlık alanı geçersiz |
| `unknown_encoding` | kap | yük kodlaması tanımsız |
| `unknown_codec` | kap | sıkıştırma kodeki tanımsız |
| `unknown_extension` | kap | zorunlu uzantı bilinmiyor |
| `too_large` | kap | yük sınırı aşılıyor |
| `hash_mismatch` | kap | SHA-256 tutmuyor |
| `malformed` | CBOR | iyi biçimli değil (ayrılmış ek bilgi, yersiz kırma) |
| `indefinite_length` | CBOR | belirsiz uzunluk |
| `non_shortest` | CBOR | tam sayı ya da uzunluk en kısa biçimde değil |
| `narrow_float` | CBOR | float16 ya da float32 |
| `non_finite` | CBOR | NaN ya da sonsuz |
| `simple_value` | CBOR | izin verilmeyen basit değer |
| `tag` | CBOR | etiket |
| `invalid_utf8` | CBOR | metin UTF-8 değil |
| `non_text_key` | CBOR | harita anahtarı metin değil |
| `unsorted_keys` | CBOR | anahtarlar sırasında değil |
| `duplicate_key` | CBOR | anahtar yinelenmiş |
| `too_deep` | CBOR | derinlik sınırı |
| `too_long` | CBOR | uzunluk sınırı |
| `cbor_truncated` | CBOR | öğe yükün sonunu aşıyor |
| `cbor_trailing` | CBOR | yükte öğeden sonra bayt var |
| `schema_format` | şema | yük bir KentOS çizimi değil |
| `schema_version` | şema | belge şeması sürümü okunamıyor |
| `unknown_field` | şema | bilinmeyen alan (eski şemada yeni alan dahil) |
| `missing_field` | şema | zorunlu alan yok |
| `wrong_type` | şema | alanın CBOR türü yanlış |
| `bad_value` | şema | değer geçersiz (numaralı metin, aralık, boy, nil kimlik, kot sayısı) |
| `unknown_kind` | şema | bilinmeyen nesne türü |
| `duplicate_uid` | şema | aynı kalıcı kimlik iki nesnede |
| `duplicate_block` | şema | aynı kimlik ya da ad iki blok tanımında (§6.9) |
| `unknown_block` | şema | yerleştirme tanımlı olmayan bir bloğu gösteriyor |
| `block_cycle` | şema | blok kendini doğrudan ya da dolaylı içeriyor |
| `block_too_deep` | şema | bloklar 16 düzeyden derin iç içe |
| `block_too_large` | şema | bir blok iç içe yerleştirmeleriyle bir milyondan çok parçaya açılır |

## 10. Örnek: en küçük çizim

`fixtures/kcad/v2/minimal.kcad` (490 bayt): tek katman, tek nokta. Kaynağı `minimal.json`.

**Başlık** (36 bayt):

```
00  89 4B 43 41 44 0D 0A 1A 0A      imza
09  02                              major 2
0A  00                              minor 0
0B  00                              minReaderMinor 0
0C  24 00                           headerLength 36
0E  01                              encoding 1
0F  00                              codec 0 (yok)
10  A6 01 00 00 00 00 00 00         payloadLength 422
18  A6 01 00 00 00 00 00 00         decodedLength 422
20  00 00                           flags 0
22  00 00                           extensionCount 0
```

**Yük** (422 bayt, 0x24'ten):

```
A3                                          harita, 3 çift
  66 "format"   6F "kentos.document"
  67 "version"  02
  68 "document" A7                          harita, 7 çift (isteğe bağlı alan yok)
    64 "name"     64 42 6F C5 9F            "Boş" (UTF-8, 4 bayt)
    66 "layers"   81                        dizi, 1
      A8                                    katman: 8 çift
        62 "id"   61 "0"
        64 "name" 61 "0"
        64 "type" 65 "layer"
        65 "style" A3
          65 "color"      62 "fg"
          68 "lineType"   6A "continuous"
          6A "lineWeight" FB 3F D0 00 00 00 00 00 00      0.25
        66 "locked"   F4
        67 "visible"  F5
        68 "children" 80
        68 "expanded" F5
    66 "origin"   82 FB 41 1E 84 80 00 00 00 00           500000.0
                     FB 41 50 C8 E0 00 00 00 00           4400000.0
    66 "styles"   A2  65 "items" 80  6A "categories" 80
    68 "entities" 81
      A1 65 "point" A4                      tek anahtarlı harita: tür "point"
        61 "p"       82 FB 41 1E 84 85 00 00 00 00        500001.25
                        FB 41 50 C8 E0 A0 00 00 00        4400002.5
        63 "uid"     50 01 92 F5 A0 7C 3E 7D 4A 9B 1E 4C 2F 8A 6D 3E 10
        65 "attrs"   A1 62 "Ad" 62 "P1"
        67 "layerId" 61 "0"
    68 "settings" A6
      64 "srid"           19 14 88          5256
      68 "areaUnit"       62 "m2"
      69 "angleUnit"      64 "grad"
      69 "plotScale"      FB 40 8F 40 00 00 00 00 00    1000.0
      6C "areaDecimals"   02
      6E "lengthDecimals" 03
    6B "activeLayer" 61 "0"
```

**Özet** (son 32 bayt): `72 27 D1 06 01 EF 2A A1 F8 6F 39 3A 9E DA 89 8F 92 F5 4D 6A E8 0E 9D 50 F0 C0 94 17 2A 70 8E CC`, baştaki 458 baytın SHA-256'sı.

Anahtar sırasına dikkat: `name` (4) < `layers` (6) < `origin` < `styles` < `entities` (8) < `settings` < `activeLayer` (11); nesnede `p` (1) < `uid` (3) < `attrs` (5) < `layerId` (7).

## 11. Sürüm kuralları

- **Ana sürüm** yalnız kap düzeni uyumsuz değişince artar (başlık alanlarının yeri, bütünlük yöntemi).
- **Küçük sürüm** yeni bir kodek, bayrak, uzantı ya da isteğe bağlı başlık alanı getirir. Yazıcı `minReaderMinor`'ı yalnız o yeni şeyi gerçekten kullandığında yükseltir.
- **Belge şeması sürümü** şema uyumsuz değişince artar (alan anlamı, zorunluluk). Yeni bir nesne türü ya da alan da şema sürümünü ya da bir zorunlu uzantıyı gerektirir: 2.0 okuyucusu bilmediği alanı ve türü reddeder.
  - Yazıcı şema sürümünü, `minReaderMinor` gibi, dosyada **gerçekten kullandığı** en yeni alana göre yazar: köşe kotu, nesne kalınlığı ve parçalı alanı olmayan çizim şema 2'dir, yalnız nesne kalınlığı olan 3, köşe kotu olan 4, çok parçalı alanı olan 5 (§6.1).
- **Profil** değişirse (etiket kullanımı gibi) yeni bir `encoding` kimliği alır.
- Her değişiklik bu belgeyi, `fixtures/kcad/v2`'yi, Rust kodlayıcısını ve `tools/kcad/kcad.py`'yi birlikte günceller; eski örnek dosyalar okunmaya devam eder.

## 12. Doğrulama

- `python3 tools/kcad/kcad.py validate DOSYA` ve `inspect DOSYA`: bağımsız okuyucu (yalnız Python standart kütüphanesi).
- `cargo run -q -p kentos-kcad --bin kcad -- inspect|validate DOSYA` ve `migrate ESKİ.kcad YENİ.kcad`: Rust aracı.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`: örnek dosyaları bağımsız yazıcıyla yeniden üretir, diskle karşılaştırır ve her birini okuyucuyla `expected.json`'a göre okur.
- Rust (`crates/shared/kcad/tests`) ve tarayıcı (`apps/web/src/io/kcad.wasm.test.ts`) aynı dosyaları okur; geçerli çizimleri **bayt bayt aynı** yazar.
