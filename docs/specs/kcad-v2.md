# KCAD v2: KentOS proje dosyası (`.kcad`) bayt spesifikasyonu

- **Sürüm:** kap 2.0, belge şeması 2, KentOS CBOR profili 1.
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
- **Harita anahtarları** metindir (`non_text_key`) ve **kodlanmış anahtar baytlarına göre sözlük sırasındadır** (RFC 8949 §4.2.1). Metin anahtarlarda bu şuna eşittir: önce UTF-8 uzunluğu kısa olan, uzunluk eşitse baytça küçük olan. Örnek: `p` < `uid` < `attrs` < `layerId` < `lineWeight`.
  - Sırasız anahtar `unsorted_keys`, aynı anahtarın ikinci kez gelmesi `duplicate_key`'dir. Sıra kodlanmış baytlarla denetlendiği için iki denetim tek karşılaştırmadır.
- **Yok olan değer yazılmaz:** isteğe bağlı bir alanın değeri yoksa anahtar da yoktur. Şemalı alanlarda `null` yazılmaz (§6).
- Dosyada zaman damgası, rastgele sayı ya da yazıcıya özgü bilgi yoktur; aynı belge her kayıtta aynı baytları verir.

### 5.3 Sayılar

- **Kayan noktalı sayılar her zaman binary64'tür** (`FB` + 8 bayt, IEEE 754, büyük sonlu). Değer daha dar bir biçime kayıpsız sığsa da (`1.0`, `0.5`) daraltılmaz. Koordinatlar ve bütün şemalı float alanları bit bit korunur (`FILE-06`).
- **NaN, +∞ ve −∞ hiçbir yerde yazılamaz** (`non_finite`). Çizim geometrisi sonlu sayıdır; JSON ile çalışan istemciler de bu değerleri taşıyamaz.
- **−0 yazılabilir ve olduğu gibi korunur** (`80 00 … 00` işaret biti). Okuyucu −0'ı +0'a çevirmez; −0 ile +0 farklı dosya baytları verir.
- **Tam sayılar** şemanın verdiği aralıktadır (§6); şemalı bir float alanına tam sayı yazılamaz (`wrong_type`), şemalı bir tam sayı alanına float yazılamaz.
- **Kesin ondalık ve hisse değerleri** float olarak yazılmaz. Belge şeması 2'de böyle bir alan yoktur (öznitelikler metindir). Eklendiğinde sözleşmedeki kesin biçimleriyle yazılır: ondalık, üssüz ondalık metin (`"748.5151"`, `DecimalString`); hisse, pay ve paydası ondalık metin olan harita (`ShareValue`) (CLAUDE.md §23.1).

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

## 6. Belge şeması 2

### 6.1 Kök

Yük, üç anahtarlı bir haritadır (anahtarlar kodlanmış sırasıyla):

| Anahtar | Tür | Değer |
|---|---|---|
| `format` | metin | `"kentos.document"`; değilse `schema_format` |
| `version` | tam sayı | `2`; değilse `schema_version` |
| `document` | harita | belge (§6.2) |

`format` ve `version` sıralamada `document`'ten önce gelir: okuyucu belgenin kendisini okumadan şema sürümünü bilir.

### 6.2 Belge

Tablolar anahtarları **dosyadaki sırasıyla** verir (§5.2). “Zorunlu” sütunu boşsa alan isteğe bağlıdır ve yoksa hiç yazılmaz.

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `name` | metin | evet | projenin adı |
| `layers` | dizi: katman (§6.5) | evet | katman ağacının üst düzeyi, sırasıyla |
| `origin` | nokta | evet | yerel orijin: verinin yakınında bir çapa (GPU ona göre çalışır) |
| `styles` | harita (§6.7) | evet | projenin kendi stil kitaplığı |
| `entities` | dizi: nesne (§6.6) | evet | nesneler, **belge sırasıyla** (çizim sırası) |
| `homeView` | sınırlar | | başlangıç görünümü; yoksa bütün nesneler |
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
| tam sayı (u32, u16) | ana tür 0 | aralık dışı `bad_value` |
| numaralı metin | metin | tabloda verilen değerlerden biri; değilse `bad_value` |

Yanlış CBOR türü (float yerine tam sayı, nokta yerine harita) `wrong_type`'tır.

### 6.4 Proje ayarları

| Anahtar | Tür | Zorunlu | Değerler |
|---|---|---|---|
| `srid` | u32 | evet | EPSG kodu. Koordinat sistemi tahmin edilmez; SRID yalnız etikettir, dönüşüm değildir |
| `areaUnit` | numaralı metin | evet | `m2`, `donum`, `ha` |
| `angleUnit` | numaralı metin | evet | `grad`, `deg` |
| `plotScale` | float | evet | çizim ölçeği paydası (1:1000 → `1000.0`) |
| `workspace` | numaralı metin | | `hybrid`, `cad`, `gis`, `plan3d`, `disaster` |
| `drawingFont` | numaralı metin | | `barlow`, `arimo`, `overpass`, `quicksand`, `architects-daughter`, `courier-prime`, `plex-mono` |
| `areaDecimals` | u32 | evet | alan gösterim basamağı |
| `lengthDecimals` | u32 | evet | uzunluk gösterim basamağı |

### 6.5 Katman ağacı

**Katman** (grup ya da katman):

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `id` | metin | evet | proje içinde benzersiz kimlik; ad değişince değişmez |
| `name` | metin | evet | görünen ad |
| `type` | numaralı metin | evet | `group`, `layer` |
| `style` | katman stili | evet | |
| `locked` | bool | evet | |
| `visible` | bool | evet | |
| `children` | dizi: katman | evet | alt düğümler (katmanda boş dizi) |
| `expanded` | bool | evet | ağaçta açık mı |

**Katman stili:**

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `fill` | metin | | dolgu rengi |
| `color` | metin | evet | onaltılık renk ya da tema adı (`fg`, `fg-dim`, `ink`, `paper`) |
| `label` | etiket stili | | |
| `point` | nokta stili | | |
| `lineType` | numaralı metin | evet | `continuous`, `dashed`, `dashdot`, `dotted` |
| `renderer` | opak değer (§6.7), `null` olamaz | | stil motorunun çizicisi |
| `lineWeight` | float | evet | çıktı çizgi kalınlığı, mm |
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

### 6.6 Nesneler

- Her nesne **tek anahtarlı bir haritadır**: anahtar nesnenin türü, değer türün alanlarını taşıyan haritadır: `{"polygon": {…}}`. Tür önce okunur, alanlar türe göre denetlenir.
  - Haritada tek anahtardan farklı sayı `bad_value`, bilinmeyen tür `unknown_kind`'dır. Tanınmayan geometri sessizce atlanmaz; dosya açılmaz (`DOM-06`).
  - Tür sürümü ayrı yazılmaz: bütün türler belge şeması sürümüyle (§6.1) birlikte sürümlenir.
- **Nesnenin çalışma yuvası (yerel `u32` kimliği) yazılmaz** (`FILE-05`). Okuyucu nesnelere dosya sırasıyla 1, 2, 3, … yuvalarını verir; yuva bir sonraki açılışta değişebilir, kalıcı kimlik değişmez.

**Her türde ortak alanlar:**

| Anahtar | Tür | Zorunlu | Anlamı |
|---|---|---|---|
| `uid` | kimlik | evet | kalıcı nesne kimliği (§6.8) |
| `attrs` | harita: metin → metin | evet | öznitelikler (boş olabilir); v2'de değerler metindir |
| `color` | metin | | renk; yoksa katmana göre |
| `label` | metin | | nesnenin etiketi |
| `symbol` | metin | | katman stilinin yerine kitaplık sembolü |
| `layerId` | metin | evet | nesnenin katmanı |

**Türlere göre alanlar** (ortak alanlarla birlikte aynı haritada, sıralı):

| Tür | Alanlar |
|---|---|
| `point` | `p` nokta; `z` float (isteğe bağlı, kot) |
| `line` | `a`, `b` nokta |
| `polyline` | `pts` nokta listesi; `bulges` float dizisi (isteğe bağlı) |
| `polygon` | `pts` nokta listesi; `bulges` float dizisi (isteğe bağlı); `holes` halka dizisi (isteğe bağlı) |
| `circle` | `c` nokta; `r` float |
| `arc` | `c` nokta; `r`, `a0`, `a1` float (radyan, saat yönünün tersine `a0` → `a1`) |
| `ellipse` | `c`, `major` nokta; `ratio`, `t0`, `t1` float |
| `spline` | `pts` nokta listesi; `closed` bool |
| `xline`, `ray` | `p`, `dir` nokta |
| `text` | `p` nokta; `text` metin; `height` float (m); `rotation` float (derece, doğudan saat yönünün tersine) |
| `dimension` | `a`, `b` nokta; `offset`, `height` float; isteğe bağlı: `c` nokta, `text` metin, `angle` float, `style` numaralı metin (`aligned`, `linear`, `angular`, `radius`, `diameter`) |
| `hatch` | `ring` nokta listesi; `holes` nokta listesi dizisi (isteğe bağlı); `pattern` harita: `type` (`solid`, `lines`, `cross`), `angle` float, `spacing` float |

- **Halka** (`polygon`'un deliği): `pts` nokta listesi (zorunlu), `bulges` float dizisi (isteğe bağlı).
- **Yay değeri** (`bulges`): DXF'teki gibi `tan(θ/4)`, saat yönünün tersi artı; `bulges[i]` `pts[i] → pts[i+1]` kenarınındır, kapalı şekilde son değer kapanış kenarınındır. Yaylar, delikler, elips ve eğri parametreleri tanım olarak saklanır; ekranda çizilen üçgen ya da kısa parçalar dosyaya girmez (`FILE-08`).
- Ölçü, blok, dış başvuru, yüzey ve katı gibi yeni türler ileride şemaya ya da zorunlu bir uzantıya eklenir; eski okuyucu onları tanımadığını söyler.

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

### 6.9 Dosyaya girmeyenler

Çalışma yuvaları, seçim, geri alma ve yineleme geçmişi, kirli bayrağı, kamera, GPU tamponları, seçme ve kenet indeksleri, tarayıcı ve cihaz durumu (localStorage, IndexedDB taslakları), kullanıcı ve cihaz tercihleri dosyaya yazılmaz (`FILE-05`). Parola, belirteç ve imzalı adres yazılmaz (`SYNC-13`).

### 6.10 Belge kuralları

Dosya biçimi geçerli olsa da uygulamalar açılışta çizimin kendi kurallarını ayrıca denetler (v1'deki gibi; web `model/snapshot.ts`, masaüstü `kentos_domain::Document`):

- en az bir katman (`type: layer`) olmalı; her nesnenin `layerId`'si bir katmanın (grubun değil) kimliği olmalı;
- `srid` uygulamanın tanıdığı bir koordinat sistemi olmalı (tahmin yapılmaz);
- `polyline` ve `polygon` en az 2, delik ve `hatch` halkası en az 3, `spline` en az 2 nokta; `bulges` nokta sayısından uzun olamaz; çemberin yarıçapı pozitif;
- proje stilleri paylaşılan bir `.kstil` dosyası gibi denetlenir.

Bu kurallar dosya biçiminin değil çizimin kurallarıdır; `tools/kcad/kcad.py` onları denetlemez.

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
| `unknown_field` | şema | bilinmeyen alan |
| `missing_field` | şema | zorunlu alan yok |
| `wrong_type` | şema | alanın CBOR türü yanlış |
| `bad_value` | şema | değer geçersiz (numaralı metin, aralık, boy, nil kimlik) |
| `unknown_kind` | şema | bilinmeyen nesne türü |
| `duplicate_uid` | şema | aynı kalıcı kimlik iki nesnede |

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
- **Profil** değişirse (etiket kullanımı gibi) yeni bir `encoding` kimliği alır.
- Her değişiklik bu belgeyi, `fixtures/kcad/v2`'yi, Rust kodlayıcısını ve `tools/kcad/kcad.py`'yi birlikte günceller; eski örnek dosyalar okunmaya devam eder.

## 12. Doğrulama

- `python3 tools/kcad/kcad.py validate DOSYA` ve `inspect DOSYA`: bağımsız okuyucu (yalnız Python standart kütüphanesi).
- `cargo run -q -p kentos-kcad --bin kcad -- inspect|validate DOSYA` ve `migrate ESKİ.kcad YENİ.kcad`: Rust aracı.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`: örnek dosyaları bağımsız yazıcıyla yeniden üretir, diskle karşılaştırır ve her birini okuyucuyla `expected.json`'a göre okur.
- Rust (`crates/shared/kcad/tests`) ve tarayıcı (`apps/web/src/io/kcad.wasm.test.ts`) aynı dosyaları okur; geçerli çizimleri **bayt bayt aynı** yazar.
