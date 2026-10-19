# ADR 0600: İfade motorunun genişletilmesi — plan ve ilk işlevler

- **Durum:** kabul edildi (2026-10-19). Sahibin isteği (19 Ekim):
  - “qgis expression ifadelerini komple incele, bizimkini incele, bizimkini geliştir, editörünü geliştir, flow kısmını geliştir”;
  - “Hepsinin birebir olmasına gerek yok ihtiyacımıza göre, ayrıca ilk önceliğimiz performans”.
- **Numara:** ifade motorunun işleri 0600'den başlar (05xx CBS planındadır).
- **Bağlam belgesi:** ADR 0100 (ifade motoru, sütun motoru, İfade oluşturucu), 0101 (akış), 0214 (tarih, `@` değerleri, düzenli
  ifade, dizi ve eşleme, toplama, mekânsal ilişki), 0228 (web'e ara).
- **Web:** ertelendi (ADR 0228). Çekirdek ortaktır; web'in işlev listesi geri dönüşte çekirdekten gelir (TODOS.md §0.2).

## Bağlam

QGIS 4.2'nin ifade motoru 414 işlev bildirir (`QgsExpression.Functions()`, makinedeki QGIS'ten okundu). KentOS'un dili 113 işlev
bildirir; bunların 100'ü QGIS'teki bir işlevin karşılığıdır (QGIS adı takma ad). Eksiklerin kümeleri:

| Küme | QGIS'te olup KentOS'ta olmayan |
|---|---|
| Geometri | 143 (geometri türünde değer: `buffer($geometry, 5)`, `area(…)`, `x(…)` …) |
| Renk | 24 |
| Diziler | 16 (`array_foreach`, `array_filter`, `generate_series` …) |
| Kayıt ve öznitelikler | 15 (`attribute`, `attributes`, `uuid`, `represent_value` …) |
| Dönüşümler | 14 (`to_bool`, `to_decimal`, `to_dms`, `to_time`, base64, özetler …) |
| Metin | 13 (`ltrim`, `repeat`, `reverse`, `title`, `format`, `unaccent` …) |
| Toplama | 12 (`q1`, `q3`, `iqr`, `majority` …) |
| Tarih ve saat | 11 |
| Matematik | 7 (`rand`, `randf`, `clamp`, `scale_linear` …) |
| Genel | 7 (`with_variable`, `var`, `eval` …) |
| Eşlemeler | 7 |
| Bulanık eşleme | 4 (`levenshtein` …) |
| Koşullar | 2 (`nullif`, `try`) |
| Dosya, katman, raster, pafta, sensör, manyetik model, form | geri kalanı |

Düzenleyicide işlevlerin yardımı ve tamamlaması vardır. Akışta düğümler küçük ve sıkışıktır. İç içe ifade (QGIS'in `@element`'i)
yoktur.

## Karar

### 1. İlke

- QGIS birebir kopyalanmaz. Harita, kadastro, imar ve CAD işinin gerçekten kullandığı işlevler alınır, Türkçe adla; anlamı aynıysa
  QGIS adı takma addır.
- Performans önce gelir:
  - yeni işlevler sütun motorunda (ADR 0100) partiler hâlinde çalışır;
  - nesneye bağlı olmayanlar derlemede sabite katlanır;
  - aynı iş bir yazmaçta yapılır;
  - metin çağıranın tamponuna yazılır, nesne başına ayırma yapılmaz.
- Her yeni işlevin anlamı bağımsız başvuruyla denetlenir: anlamı QGIS'inkiyle aynı olanlar makinedeki QGIS 4.2'nin kendi
  motoruyla, ötekiler ADR'nin kuralıyla.

### 2. Dilimler

1. **Bu ADR.** Nesneye bakmayan yeni işlevler (aşağıda) ve rastgele sayıların değişken çağrı kuralı.
2. **ADR 0601: iç içe ifadeler.**
   - `dizi_her` (`array_foreach`), `dizi_süz` (`array_filter`): dizinin her öğesinde değerlendirilen ifade, `@öğe` (`@element`),
     `@sıra` (`@counter`);
   - `değişkenle` (`with_variable`);
   - `dene` (`try`): hatası boş ya da ikinci değer olan ifade.
3. **ADR 0602: toplamanın ekleri:** `q1`, `q3`, `iqr`, `range`, `majority`, `minority`, `count_missing`, `min_length`, `max_length`,
   `concatenate_unique`.
4. **ADR 0603: İfade oluşturucu.**
5. **ADR 0604: Akış.**
6. **Sonraya:**
   - geometri türünde değer ve geometri işlevleri: bugünkü `$` değerleri ve mekânsal işlevler işin çoğunu görür; değer türü
     sütun motorunu büyütür, ayrı karar;
   - renk işlevleri: stil motorunun renk alanları ifadeyle verildiğinde.

### 3. Bu dilimin işlevleri

| Küme | Türkçe ad (QGIS adı) |
|---|---|
| Dönüşümler | `mantıksal` (`to_bool`), `ondalık_derece` (`to_decimal`), `derece_dakika_saniye` (`to_dms`), `derece_dakika` (`to_dm`), `base64_kodla` (`to_base64`), `base64_çöz` (`from_base64`), `özet` (`hash`), `md5`, `sha256` |
| Rastgele | `rastgele` (`rand`), `rastgele_ondalık` (`randf`), `uuid` |
| Matematik | `sınırla` (`clamp`), `ölçekle_doğrusal` (`scale_linear`), `ölçekle_üstel` (`scale_exponential`) |
| Metin | `soldan_kırp` (`ltrim`), `sağdan_kırp` (`rtrim`), `yinele` (`repeat`), `ters_çevir` (`reverse`), `başlık` (`title`), `geçme_sayısı` (`substr_count`), `ayraçla_birleştir` (`concat_ws`), `biçimle` (`format`), `karakter` (`char`), `karakter_kodu` (`ascii`), `aksansız` (`unaccent`), `satırlara_böl` (`wordwrap`), `levenshtein`, `hamming` (`hamming_distance`), `ortak_parça` (`longest_common_substring`) |
| Koşullar | `eşitse_boş` (`nullif`) |
| Diziler | `dizi_hepsi` (`array_all`), `dizi_say` (`array_count`), `dizi_araya` (`array_insert`), `dizi_başa` (`array_prepend`), `dizi_çıkar` (`array_remove_at`), `dizi_hepsini_çıkar` (`array_remove_all`), `dizi_değiştir` (`array_replace`), `dizi_ortak` (`array_intersect`), `dizi_en_sık` (`array_majority`), `dizi_en_seyrek` (`array_minority`), `dizi_ortanca` (`array_median`), `seri` (`generate_series`) |
| Eşlemeler | `eşleme_birleştir` (`map_concat`), `eşleme_önek` (`map_prefix_keys`), `adres_kodla` (`url_encode`) |
| Tarih ve saat | `epok` (`epoch`), `epoktan` (`datetime_from_epoch`), `saat_yap` (`make_time`, `to_time`) |

Anlam kuralları:

- **Ondalık ayırıcı.** `derece_dakika_saniye` ve `derece_dakika` QGIS gibi `°`, `′`, `″` yazar. Ondalık ayırıcısı dilin kuralıyla
  noktadır (QGIS yerel ayarı kullanır); `ondalık_derece` ikisini de okur.
- **Rastgele sayılar.**
  - `rastgele(en_az, en_çok[, tohum])` tam sayı, `rastgele_ondalık(…)` ondalık verir.
  - Tohumla belirlidir: aynı tohum aynı sayıyı verir. Üreteç SplitMix64'tür (ADR 0220'nin üreteci); QGIS'in Mersenne Twister'ıyla
    aynı sayılar beklenmez, yalnız aralık ve belirlilik.
  - Tohumsuz çağrı değişkendir: derlemede sabite katlanmaz, iki çağrısı aynı yazmaçta birleşmez, her nesnede yeni sayıdır.
  - `uuid` hep değişkendir.
- **Diğer anlamlar.**
  - `özet` MD5, SHA-1, SHA-224, SHA-256, SHA-384 ve SHA-512 bilir. SHA'lar çalışma alanının saf `sha1` ve `sha2`
    crate'lerinden gelir (zaten bağımlılıktı); MD5, Base64 ve UUID'nin yazımı burada yazıldı. Yeni bağımlılık yoktur.
  - `base64_çöz` metin verir; UTF-8 olmayan bayt boştur.
  - `biçimle`: `%1` … `%9`.
  - `aksansız` Türkçe harfleri de sadeleştirir (`ş`→`s`, `İ`→`I`, `ı`→`i`).
  - `satırlara_böl`: artı uzunluk en çok, eksi uzunluk en az.
  - `seri` en çok 1 000 000 öğe verir.

### 4. QGIS'ten bilerek ayrılanlar

Çapraz denetimde ortaya çıktı; her biri fixture'da gerekçesiyle durur (`rule`, `qgisSays`):

- **`mantıksal` (`to_bool`):** QGIS boş olmayan her metni doğru sayar (`to_bool('false')` doğrudur). KentOS `false`, `no`, `off`,
  `0`, `yanlış`, `hayır` sözcüklerini yanlış okur.
- **Türkçe harfler:**
  - `başlık` Türkçe büyük ve küçük harfle çalışır (`istanbul` → `İstanbul`, `KIZILAY` → `Kızılay`);
  - `aksansız` `ı`'yı `i` yapar.
- **Diziler:**
  - `dizi_araya` eksi yerde başa, uzunluğu aşan yerde sona ekler (QGIS iki durumda da çöker);
  - `dizi_en_sık` ve `dizi_en_seyrek` eşit sıklıktaki öğeleri ilk geçtikleri sırayla verir (QGIS'te karma tablonun sırası).
- **`base64_çöz`** metin verir (QGIS bayt dizisi verir).
- **`biçimle`** k'ıncı değeri kalıptaki k'ıncı küçük yer numarasına yazar, tek geçişte. QGIS'in zincirli `arg`'ından tek farkı: bir
  değerin kendi içindeki `%n`'i olduğu gibi kalır.

### 5. Performans

100 000 nesnede, bütün değerler okunarak (release, `measures_docs_adr_0600`; p50):

| İfade | Süre |
|---|---|
| `sınırla(0, $alan / 10, 50)` | 1,7 ms |
| `ölçekle_doğrusal($alan, 0, 1000, 1, 10)` | 2,2 ms |
| `rastgele(1, 100)` | 2,8 ms |
| `ayraçla_birleştir('/', 104, Parsel)` | 5,2 ms |
| `levenshtein(Nitelik, 'Arsa')` | 7,3 ms |
| `aksansız('Şişli ' \|\| Nitelik)` | 9,8 ms |
| `biçimle('%1 ada %2 parsel', 104, Parsel)` | 14 ms |
| `başlık(Nitelik \|\| ' parseli')` | 15 ms |
| `derece_dakika_saniye($alan / 25, 'y', 2)` | 16 ms |
| `dizi_say(dizi(Nitelik, 'Arsa', 'Tarla'), 'Arsa')` | 32 ms (dizinin JSON'u, ADR 0214'ün taşıması) |

## Doğrulama

- `scripts/fixtures/expression_qgis_cases.py`, `fixtures/expression/v2/qgis.json`'ı yazar:
  - her ifade QGIS adlarıyla yazılır;
  - QGIS 4.2'nin motoru (`/usr/share/qgis/python`) değerlendirir ve sonucu dilin değerine çevrilir (sayı, metin, doğru/yanlış, boş,
    dizi, eşleme);
  - QGIS'ten ayrılan anlamlar durumda gerekçesiyle elle yazılır (ondalık ayırıcı, rastgele sayıların değerleri).
- `crates/shared/expression/tests/qgis.rs` aynı ifadeleri sütun motoruyla ve tek nesne yoluyla değerlendirir; ikisinin sonucu da
  durumla aynıdır. 150 durumun 123'ünde değer QGIS 4.2'nindir.
- Tohumsuz rastgele çağrılar: iki çağrı ayrı sayı, iki nesne ayrı sayı, değerler aralıkta, `uuid` sürüm 4 (aynı test).
- İfade oluşturucunun yardım örnekleri derlenir ve yazdıkları sonucu verir (`editor::catalog`'un testi); `builder.json` yeni
  işlevlerle yeniden yazıldı (yalnız eklemeler).
- Performans: yeni işlevlerin 100 000 nesnede partili süreleri `perf`'te (ADR 0100'ün ölçüm çatısı).
