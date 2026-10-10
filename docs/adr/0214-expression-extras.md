# ADR 0214: İfade dili ekleri

- **Durum:** kabul edildi (2026-10-10). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”); düzenli
  ifade için `regex-lite` sahibin onayıyla (10 Ekim: “regex-lite (Önerilen)”). İki platform (masaüstü ve web), bulut (`.kcad` şeması);
  ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-18`, `OUT-01`; ADR 0100 (ifade motoru: sütun motoru, tek nesne yolu, şema, dil ekleri, İfade
  oluşturucu), ADR 0101 (akış), ADR 0164 (paftanın `@` değişkenleri), ADR 0200 (Konuma göre seç'in ilişkileri, `kentos.statistics/1`),
  ADR 0201 (örtüşme işlemleri), ADR 0210 (zaman çekirdeği: tarihin okunuşu, yazılışı, takvim).
- **Araştırma:** QGIS'in işlev listesi (Date and Time: `make_date`, `to_date`, `year`…`second`, `week`, `day_of_week`, `format_date`,
  aralık aritmetiği; Aggregates: `aggregate`, `sum`, `mean`, `count`, `minimum`, `maximum`, `count_distinct`, `median`, `stdev`,
  `concatenate`, `array_agg`, `group_by` ve `filter` adlı argümanlar; Geometry: `overlay_intersects`, `overlay_contains`,
  `overlay_within`, `overlay_nearest` ve ifade argümanıyla dizi dönüşü, `get_feature`, `attribute`; String: `regexp_match`,
  `regexp_replace`, `regexp_substr`, `regexp_matches`; Arrays: `array`, `array_length`, `array_get` (0'dan, eksi sondan),
  `array_first`, `array_last`, `array_contains`, `array_find`, `array_append`, `array_cat`, `array_distinct`, `array_sort`,
  `array_reverse`, `array_slice`, `array_to_string`, `string_to_array`, `array_sum`, `array_mean`, `array_min`, `array_max`; Maps:
  `map`, `map_get`, `map_akeys`, `map_avals`, `map_exist`, `map_insert`, `map_delete`, `from_json`, `to_json`; Variables: proje,
  katman, `@now`). ArcGIS Pro'nun Arcade'i (Date, DateAdd, DateDiff, Text biçimleri; FeatureSetByName, Intersects, Count, Sum;
  Regex yok). Netcad'in Tablo Kolonu Değer İfadeleri (tarih ve metin işlevleri; katman ötesi sınırlı).

## Bağlam

İfade dili (ADR 0100) bugün bir nesnenin kendi değerlerini bilir: öznitelikleri, tipli alanları, `$` geometri değerlerini; sayı, metin,
doğru/yanlış ve boş değerleri; matematik, metin ve koşul işlevlerini. Coğrafi işin sık ifadeleri bunun ötesindedir: bir tarihten yıl,
iki tarih arası gün; aynı katmandaki toplamın payı (`$alan / topla($alan)`); bir parselin kesiştiği sit alanı, en yakın durağın adı,
mahallesinin adı; bir metinden kalıpla parça; bir ilişkideki değerlerin listesi; projenin iş numarası. QGIS bunları işlevlerle,
ArcGIS Arcade'le verir.

## Karar

### 1. Kapsam

Dile yedi küme eklenir; hepsi Türkçe adla, QGIS'teki anlamı aynıysa QGIS adı takma adla:

| Küme | Ne verir | Çağıran |
|---|---|---|
| Tarih ve saat | tarih yapma ve okuma, parçaları, ekleme, fark, biçimleme, şimdi | her yerde |
| Değişkenler | `@ad`: yerleşik (proje adı, koordinat sistemi, ölçek, tarih, şimdi, kullanıcı, katman) ve projenin kendi değişkenleri | İşlemler, İfade oluşturucu, Öznitelik tablosunun süzgeci, pafta; `@katman_adi` her yerde |
| Düzenli ifade | eşleşme, yer, parça, gruplar, değiştirme (`regex-lite`) | her yerde |
| Dizi ve eşleme | yapma, öğe, arama, ekleme, birleştirme, sıralama, dilim, metne ve metinden, toplam; anahtarlı değerler, JSON | her yerde |
| Toplama | aynı katmanın nesneleri üstünden (grup ve koşulla) ve başka katman üstünden | İşlemler, İfade oluşturucu |
| Mekânsal ilişki | başka katmanın nesneleriyle kesişme, kapsama, içinde kalma, merkezin içinde olma, uzaklıkta olma; sayı, değerler, uzaklık, en yakının değeri, kesişim alanı ve uzunluğu | İşlemler, İfade oluşturucu |
| Başka katmandan değer | anahtarla eşleşen ilk nesnenin değeri | İşlemler, İfade oluşturucu |

- **Projenin değişkenleri** projede saklanır (`ProjectSettings.variables`, `.kcad` şema 37): ad, etiket, tür (metin, sayı, doğru/yanlış,
  tarih), değer; paftanın değişken biçimiyle aynı. Proje ayarları'nda Değişkenler bölümü (iki platform, §4); pafta onları kitabın proje
  değişkenlerinden sonra okur (`OUT-01`'in açığı).
- **“Başka katmana bakan” işlevler** (toplama, mekânsal ilişki, başka katmandan değer) çağıranın çizimin katmanlarını vermesini ister
  (§3). İşlemler'in bütün araçları (İfadeyle seç, Öznitelik hesapla, ifade alanı olan her araç) ve onların İfade oluşturucu'su verir.
  Başka yerde (katman süzgeci, stil kuralları, etiketler, ağın maliyetleri, raster hesaplayıcı, pafta, Öznitelik tablosunun süzgeci) bu
  işlevler **derlenmez**: “Bu işlev başka katmanlara bakar; burada kullanılamaz.”, yeri işlevin adı. Kural tek yerdedir: çekirdeğin
  `compile_with`'i şemanın `world`'ü demedikçe reddeder; ev sahipleri ayrıca denetlemez (süzgeç, stil ve etiketler başka katman
  değişince yeniden kurulmaz; bunlar ayrı iştir). İfade oluşturucu oralarda bu işlevleri ağaçta ve tamamlamada göstermez.
- **`@` değerleri** bir değerlendirmenin değerleridir; okundukları yer istenince değerlendirilen ifadelerdir: İşlemler, İfade oluşturucu,
  Öznitelik tablosunun süzgeci ve pafta. Çizimin kalıcı ifadeleri (stil kuralları, etiketler, katman süzgeci, ağın maliyetleri) yalnız
  `@katman_adi`'yi (`$katman`) okur: çizim kurulurken değerlendirilip tutulurlar; bir değişken Proje ayarları'nda ya da saat nesnelere
  dokunulmadan değişince eski kalırlardı. Oralarda öbür `@ad`'lar boştur ve İfade oluşturucu uyarır: “Bu alanda yalnız @katman_adi ve
  @katman okunur; “@ad” boş olur.” (TODOS.md `GIS-18`'in açık kalanı: kalıcı ifadelerde projenin değişkenleri, değişince yeniden kurmayla).
- **Kapsam dışı:** geometri türünde değer (QGIS'in `$geometry`, `buffer()` dönüşü; mekânsal işlevler katman adı ve uzaklık alır,
  geometri değil); ilişkiler (`relation_aggregate`: KentOS'ta ilişki modeli yok); katmanın kullanıcı tanımlı değişkenleri; saat dilimi
  ayarı (tarihler dilimsizdir, §2.1); `@üst` (toplamanın koşulunda geçerli nesne: mekânsal işlevler bu işi görür); dizi içinde dizi ve
  eşleme içinde eşleme işlevleri (JSON'dan okunur ve yazılır, öğeleri metin olarak görünür).

### 2. Tanımlar

#### 2.1 Değerler

- **Tarih** ISO metnidir: `YYYY-AA-GG`; **tarih ve saat** `YYYY-AA-GGTSS:DD:ss`, milisaniyesi varsa `.mmm` (zaman çekirdeğinin `write`'ı,
  ADR 0210 §3). Okunuş zaman çekirdeğinin `read`'idir: `YYYY-AA-GG`, `GG.AA.YYYY`, `T` ya da boşlukla saat, isteğe bağlı dilim (dilim
  verilirse UTC'ye çevrilir). Tarihler dilimsiz duvar saatidir. Metin olarak karşılaştırılır (ISO sırası zaman sırasıdır); okunamayan
  metin tarih işlevlerinde boştur.
- **Dizi** ve **eşleme** dilin dördüncü ve beşinci değer biçimidir; içeride JSON metnidir, başında görünmeyen bir işaret karakteriyle
  (U+E000 dizi, U+E001 eşleme): yazmaçlar ve sütunlar değişmez. Görünüşü (alana yazılınca, metne çevrilince, birleştirilince) JSON'dur:
  `["Arsa","Bahçe"]`, `{"ad":"Arsa","kat":3}`; sayılar JavaScript'in yazdığı gibi, metinler JSON kaçışlarıyla, doğru/yanlış `true`/
  `false`, boş `null`. Eşitlik iki JSON metninin eşitliğidir; boş olmayan dizi ve eşleme doğrudur. Eşlemenin anahtarları metindir,
  ekleniş sırasıyla durur. Bir dizinin öğesi dizi ya da eşleme olabilir (JSON'dan), ama işlevler onu metni olarak görür.

#### 2.2 Tarih ve saat

| İşlev | Takma ad | Ne verir |
|---|---|---|
| `tarih(yıl, ay, gün)`, `tarih(metin)` | `make_date`, `to_date` | Tarih; ay ve gün takvimde yoksa boş; metinden okunanın tarih kısmı |
| `tarih_saat(yıl, ay, gün, saat, dakika, saniye)`, `tarih_saat(metin)` | `make_datetime`, `to_datetime` | Tarih ve saat (saniye kesirli olabilir, milisaniyeye yuvarlanır) |
| `şimdi()`, `bugün()` | `now` | `@şimdi`, `@tarih`: çağıranın verdiği an (bir değerlendirme boyunca sabit) |
| `yıl(t)`, `ay(t)`, `gün(t)`, `saat(t)`, `dakika(t)`, `saniye(t)` | `year` … `second` | Parçalar (saniye kesirli) |
| `hafta(t)` | `week` | ISO 8601 hafta numarası (1–53) |
| `haftanın_günü(t)` | — | ISO: Pazartesi 1 … Pazar 7 (QGIS'in `day_of_week`'i Pazar 0'dır: takma ad yok) |
| `yılın_günü(t)` | — | 1–366 |
| `tarih_ekle(t, n, birim)` | — | `n` birim eklenir; ay ve yıl takvimle (ayın sonunu aşan gün o ayın son günü, ADR 0210 §5'in `position`'ı); tarihe gün, hafta, ay ya da yıl eklenince tarih kalır, daha küçük birimle tarih ve saat olur |
| `tarih_farkı(a, b, birim)` | — | `b − a`: gün, hafta, saat, dakika, saniye kesirli; ay ve yıl takvimle tam sayı (`b`'nin günü ve saati `a`'nınkinden önceyse bir eksik), yıl = ay ÷ 12'nin tam kısmı (sıfıra doğru) |
| `tarih_biçimle(t, biçim)` | — | Biçimde `YYYY` `YY` yıl, `AAAA` ay adı, `AAA` kısa ay adı, `AA` `A` ay, `GGGG` gün adı, `GGG` kısa gün adı, `GG` `G` gün, `SS` `S` saat, `DD` `D` dakika, `ss` `s` saniye; tek tırnak içi olduğu gibi (QGIS'in Qt biçimi farklıdır: takma ad yok) |

Birimler: `yıl`, `ay`, `hafta`, `gün`, `saat`, `dakika`, `saniye` (Türkçe harfsiz ve İngilizce `year`, `month`, `week`, `day`, `hour`,
`minute`, `second` de; çoğul `s`'li İngilizce de). Bilinmeyen birim boştur. Ay adları Ocak … Aralık, kısaları Oca … Ara; gün adları
Pazartesi … Pazar, kısaları Pzt, Sal, Çar, Per, Cum, Cmt, Paz.

#### 2.3 Değişkenler

`@ad` yazılır; ad harf, rakam ve `_`'dir, Türkçe harfsiz ve büyük/küçük harf duyarsız aranır (`@Proje_Adı` = `@proje_adi`; paftanın
kuralı, ADR 0164). Sıra: projenin değişkenleri, sonra yerleşikler:

| Değişken | Değer |
|---|---|
| `@proje_adi` | Çizimin adı |
| `@koordinat_sistemi`, `@epsg` | Sistemin adı (“TUREF / TM36”), kodu (yoksa boş) |
| `@olcek` | Çizim ölçeğinin paydası |
| `@tarih`, `@simdi` | Bugün (tarih), şimdi (tarih ve saat): çağıranın saati, değerlendirme boyunca sabit |
| `@katman_adi`, `@katman` | Değerlendirilen nesnenin katmanının adı (`$katman` ile aynı; her yerde) |
| `@kullanici` | Oturumdaki kullanıcının adı (yoksa boş) |

Değişkenler derlenirken sabittir (katlanır); ev sahibi onları her değerlendirmede yeniden verir (saat cihazın yerel saati, değerlendirme
boyunca aynı; kullanıcı oturumdakinin adı: web'de ve masaüstünde bulut hesabının görünen adı). Bilinmeyen değişken boştur ve İfade
oluşturucu onu uyarıyla işaretler (yazılana yakın bir değişken varsa “… mi yazılacaktı?”). Değişkenin türü metin, sayı, doğru/yanlış ya da
tarihtir (tarih ISO metin). `şimdi()` ve `bugün()` `@simdi` ve `@tarih`'i okur. Proje değişkeninin adı yerleşiklerden biri olamaz.

#### 2.4 Düzenli ifade

`regex-lite`'ın sözdizimi (Rust `regex`'inki): `.` `*` `+` `?` `{n,m}` `[…]` `^` `$` `|` `(…)` `(?:…)`, `(?i)` büyük/küçük harf
duyarsız. `\w`, `\d`, `\s` ve harf duyarsızlığı yalnız ASCII'dedir; Türkçe harfler doğrudan yazılarak eşleşir (`[çğıöşüÇĞİÖŞÜ]`).

| İşlev | Takma ad | Ne verir |
|---|---|---|
| `eşleşir(metin, kalıp)` | — | Doğru/yanlış |
| `düzenli_bul(metin, kalıp)` | `regexp_match` | İlk eşleşmenin yeri, 1'den (UTF-16 birimi, `bul` gibi); yoksa 0 |
| `düzenli_parça(metin, kalıp)` | `regexp_substr` | Kalıbın ilk grubu varsa onun, yoksa bütün eşleşmenin metni; yoksa boş |
| `düzenli_gruplar(metin, kalıp)` | `regexp_matches` | İlk eşleşmenin gruplarının dizisi (eşleşmeyen grup boş metin); yoksa boş |
| `düzenli_değiştir(metin, kalıp, yerine)` | `regexp_replace` | Her eşleşme değiştirilir; `yerine`'de `$1`, `${ad}` gruplardır |

Sabit kalıp derlenirken bir kez okunur; okunamayan sabit kalıp derleme hatasıdır (“Düzenli ifade okunamadı (…).”). Değerden gelen
kalıp partide okunur, son okunan 16'sı saklanır; okunamayanı boştur. Kalıp en çok 1 000 karakter; `regex-lite`'ın boyut (1 MB) ve iç içe
geçme (100) sınırı aşılırsa okunmaz. Bir kalıp eşleşmesinin hangi baytla başlayabileceğini açıkça söylüyorsa (ilk öğesi bir harf, kaçışlı
bir işaret, `\d` ya da olumsuz olmayan ASCII bir sınıf, kendisi ve çevresindeki gruplar isteğe bağlı değil; seçenek `|`, bayrak, çapa
yok) arama metnin o baytlardan ilkinden başlar, hiçbiri yoksa eşleşme yoktur; öbür kalıplar baştan aranır. Grupların yeri kalıpla
saklanır, nesne başına ayrılmaz.

#### 2.5 Dizi ve eşleme

| İşlev | Takma ad | Ne verir |
|---|---|---|
| `dizi(a, b, …)` | `array` | Dizi |
| `dizi_uzunluğu(d)` | `array_length` | Öğe sayısı |
| `dizi_öğe(d, i)` | `array_get` | `i`'inci öğe, 0'dan; eksi sondan (-1 son); yoksa boş |
| `dizi_ilk(d)`, `dizi_son(d)` | `array_first`, `array_last` | İlk ve son öğe |
| `dizi_içerir(d, x)` | `array_contains` | `=`'nin kuralıyla |
| `dizi_bul(d, x)` | `array_find` | İlk eşit öğenin yeri, 0'dan; yoksa -1 |
| `dizi_ekle(d, x)`, `dizi_birleştir(d, e)` | `array_append`, `array_cat` | Sonuna öğe, iki dizi |
| `dizi_benzersiz(d)` | `array_distinct` | Eşitlerin ilki kalır |
| `dizi_sırala(d [, artan])` | `array_sort` | Tam bir sıra: sayılar (sayı okunan metinler de) değerce, sonra metinler Türkçe sırayla, sonra yanlış ve doğru, sonra diziler ve eşlemeler JSON'larıyla; boşlar sonda (azalan sırada da); kararlı |
| `dizi_ters(d)`, `dizi_dilim(d, baş, son)` | `array_reverse`, `array_slice` | Ters; `baş`'tan `son`'a (ikisi dahil, 0'dan, eksi sondan) |
| `dizi_metin(d [, ayraç [, boş_yerine]])` | `array_to_string` | Öğelerin metinleri ayraçla (varsayılan `,`) |
| `metin_dizi(metin [, ayraç [, boş_metin]])` | `string_to_array` | Ayraçla bölünmüş metinler (varsayılan `,`) |
| `dizi_topla(d)`, `dizi_ortalama(d)`, `dizi_en_düşük(d)`, `dizi_en_yüksek(d)` | `array_sum`, `array_mean`, `array_min`, `array_max` | Sayı olan öğelerin; `kentos.statistics/1`'in kuralı (kesin toplam); en düşük ve en yüksek sayı yoksa metinlerin (Türkçe sırayla) |
| `eşleme(a1, d1, a2, d2, …)` | `map` | Eşleme |
| `eşleme_değeri(e, a)`, `eşleme_içerir(e, a)` | `map_get`, `map_exist` | Anahtarın değeri; var mı |
| `eşleme_anahtarları(e)`, `eşleme_değerleri(e)` | `map_akeys`, `map_avals` | Dizi |
| `eşleme_ekle(e, a, d)`, `eşleme_sil(e, a)` | `map_insert`, `map_delete` | Yeni eşleme |
| `json_oku(metin)`, `json_yaz(x)` | `from_json`, `to_json` | JSON'dan değer (dizi, eşleme, sayı, metin, doğru/yanlış); değerin JSON'u |

Dizi ya da eşleme beklenen yerde başka değer boştur (`dizi_uzunluğu('a')` boş). Bir dizi en çok 1 000 000 öğe, metni JavaScript'in en
uzun metnini aşamaz (aşan ifadeyi boşaltır, ADR 0100'ün kuralı).

#### 2.6 Toplama

Aynı katman: `topla(ifade [, grup [, koşul]])`, `ortalama`, `say` (boş olmayan değerler), `say_benzersiz`, `en_düşük`, `en_yüksek`,
`orta_değer`, `std_sapma`, `değerleri_birleştir(ifade, ayraç [, grup [, koşul]])`, `değerleri_dizi(ifade [, grup [, koşul]])`; takma
adlar `sum`, `mean` ve `avg`, `count`, `count_distinct`, `minimum`, `maximum`, `median`, `stdev` ve `stddev`, `concatenate`, `array_agg`.
En düşük ve en yüksek `en_düşük`, `en_yüksek`'tir: `en_az` ve `en_çok` dilde zaten `min` ve `max`'ın takma adlarıdır (ADR 0100).

- Nesneleri değerlendirilen nesnenin **katmanının bütün nesneleridir** (yalnız seçilenler değil), belgedeki sırasıyla. `koşul` doğru
  olanlar girer; `grup` verilirse yalnız grup değeri değerlendirilen nesneninkine eşit (`=`'nin kuralı) olanlar.
- Sayıların kuralı Özet istatistik'inkidir (`kentos.statistics/1`, ADR 0200): toplam kesin, ortalama kesin toplamın sayıya bölümü,
  orta değer çift sayıda iki ortadakinin ortalaması, std_sapma örneklem sapması (n − 1). Sayı olmayan değerler yalnız `say`'a, `en_az`
  ve `en_çok` sayı yoksa metinlere (Türkçe sırayla), birleştirme ve diziye girer. Hiç değer yoksa sonuç boştur (`say` 0).
- Başka katman: `katman_toplamı(katman, işlem, ifade [, koşul])` (`aggregate`); `işlem` yukarıdakilerin adı ya da takma adı
  (`'topla'`, `'sum'` …; bilinmeyen işlem derleme hatasıdır), birleştirmede ayraç `, `.
- **Katman** sabit bir metindir (“Katman adı sabit bir metin olmalı”): ağaçta o adı taşıyan ilk katman (Türkçe harfsiz, büyük/küçük harf
  duyarsız). Katman yoksa çağrı boştur. Katmanın bütün nesneleri okunur (görünürlüğü ve seçimi gözetmeden), katmanın süzgecinin dışarıda
  bıraktıkları dışında (ADR 0211 §1: İşlemler'in girdilerinde olduğu gibi).
- **Hesap bir kez:** her toplama çağrısı değerlendirmenin başında katman başına (grupla: grup değeri başına) bir kez hesaplanır, sonra
  nesne başına okunur.

#### 2.7 Mekânsal ilişki

Değerlendirilen nesne ile `katman`'ın nesneleri arasında; `uzaklık` (m) verilirse nesne o kadar büyütülmüş gibi (“uzaklıkta”, GIS-02'nin
kuralı), `koşul` öbür katmanın nesnelerine uygulanır:

| İşlev | Takma ad | Ne verir |
|---|---|---|
| `kesişir(katman [, uzaklık [, koşul]])` | `overlay_intersects` | Kesişen (uzaklıkta) bir nesne var mı |
| `kapsar(katman [, koşul])` | `overlay_contains` | Nesne öbürlerinden birini bütünüyle kapsıyor mu |
| `içinde_kalır(katman [, koşul])` | `overlay_within` | Nesne öbürlerinden birinin bütünüyle içinde mi |
| `merkezi_içinde(katman [, koşul])` | — | Nesnenin merkezi (`$merkez`) öbürlerinden birinin içinde mi |
| `kesişen_sayısı(katman [, uzaklık [, koşul]])` | — | Kaç nesne |
| `kesişenler(katman, ifade [, uzaklık [, koşul]])` | — | Kesişenlerin `ifade` değerleri, katmandaki sırayla, dizi |
| `uzaklık(katman [, koşul])` | — | En yakın nesneye uzaklık (m; kesişiyorsa 0) |
| `en_yakın(katman, ifade [, koşul])` | `overlay_nearest` | En yakın nesnenin `ifade` değeri (eşitlikte katmandaki sıra) |
| `kesişim_alanı(katman [, koşul])` | — | Nesneyle kesişen alanların kesişimlerinin alanları toplamı (m²; öbür katmanın örtüşen alanları ikişer sayılır) |
| `kesişim_uzunluğu(katman [, koşul])` | — | Nesnenin (çizgi ya da alanın sınırı) öbür katmanın alanları içindeki uzunluğu toplamı (m) |

İlişkiler Konuma göre seç'inkilerdir (ADR 0200: çekirdeğin `relate`'i; yaylar kesin, alan kenarları dahil); kesişim alanı ve uzunluğu
örtüşme işlemlerinindir (ADR 0201). Nesne kendi katmanına bakıyorsa kendisi sayılmaz. Geometrisi ilişkiye girmeyen nesne (yazı, blok …)
için ilişkiler yanlış, sayılar 0, `kesişenler` boş dizidir. Adaylar deponun uzamsal dizininden gelir; en yakını arayan kutu bir
nesne bulunana ya da katmanı kaplayana dek dört katına büyür. QGIS'in `overlay_*` dizi döndürür; bizimki doğru/yanlıştır (`kesişenler`
diziyi verir).

#### 2.8 Başka katmandan değer

`katmandan(katman, ifade, alan, değer)`: `katman`'ın `alan` değeri `değer`'e eşit (`=`) ilk nesnesinin `ifade` değeri; yoksa boş. QGIS'te
`attribute(get_feature(katman, alan, değer), ifade)`. Anahtar tablosu katman ve alan başına bir kez kurulur.

### 3. Değerlendirme

- **Derleme:** yeni işlevler `library`'nin tablosunda (ad, takma adlar, argüman sayısı, imza, açıklama, örnekler, grup: Tarih ve saat,
  Diziler ve eşlemeler, Toplama, Mekânsal; Düzenli ifade Metin'de); hesabı saf olanlar `functions::call`'dan ailelerine gider (`dates`,
  `patterns`, `arrays`), iki yol (sütun motoru ve tek nesne yolu) aynı kodla. `@ad` sözcükleyicide değişkendir; değeri `Schema`'nın
  değişkenlerinden derlenirken sabittir (`resolve`). Sabit kalıp derlenir.
- **Başka katmana bakanlar** derlenirken ayrı bir listeye girer (`Expr::world`, `WorldCall`): işlev, katman adı, sabit metin (işlem, alan,
  ayraç), iç ifadeler (değer, grup, koşul) ayrı programlar olarak; argümanların rolleri `world::roles`'ta (katman, metin, değer, grup,
  koşul, geçerli nesnenin değeri). Bir çağrı başkasının içinde olamaz. Çağıran katmanları verir:

  ```rust
  pub struct World<'a> {
      /// Ağacın katmanları: adı, nesnelerinin kimlikleri ve nesneleri.
      pub layers: Vec<WorldLayer<'a>>,
      /// Şekillerin deposu (ilişkiler için).
      pub store: Option<&'a Store>,
  }
  pub trait LayerObjects {
      /// Katmanın nesneleri, bir iç ifadenin sütun motoruna okuduğu gibi.
      fn source<'s>(&'s self, e: &'s Expr) -> Box<dyn Source<'s> + 's>;
  }
  ```

  Motor bir değerlendirmede bir `Session` tutar: toplamlar, anahtar tabloları ve koşulun geçirdikleri çağrı ve katman başına bir kez
  hesaplanır (iç ifade öbür katmanın nesneleri üstünde sütun motoruyla); geçerli nesnenin şekli son 1 024'ü saklanarak; sonra nesne
  başına okunur. İki yol aynı oturumu kullanır.
- **Çağıranlar:** web'de İşlemler ve İfade oluşturucu deponun değerlendirmesine (`GeometryStore.evaluateExpressionIn`) bağlamı (`@`
  değerleri, izin) ve ifadenin baktığı katmanların tek bir tablosunu (`ExprWorld`) verir; ilişkiler deponun şekilleriyle çekirdekte.
  İş iş parçacığında koşunca katmanlar işin kopyasından, süzgecin dışarıda bıraktıkları işle (`RunJob.leftOut`) gelir. Masaüstünde
  İşlemler sütun motoruna geçti (bugün tek nesne yolundaydı: `EntityObjects`, `evaluate_in`) ve katmanları belgeden ve geometri
  deposundan kurar (`runner::world_layers`); İfade oluşturucu'nun önizlemesi de başka katmana bakan ifadeyi aynı yolla, o nesneye dek
  nesnelerle değerlendirir.

### 4. Arayüz

- **İfade oluşturucu** (iki platform): yeni gruplar ağaçta: Tarih ve saat, Diziler ve eşlemeler, Toplama, Mekânsal; Değişkenler'in
  altında projenin değişkenleri ve yerleşikler (açıklamaları ve değerleriyle yardımda); `@` yazınca değişkenler tamamlanır; bilinmeyen
  değişken uyarıdır; önizleme değerleri (dizi ve eşleme JSON olarak). Başka katmana bakan işlevler yalnız İşlemler'in oluşturucusunda
  görünür; öbürlerinde yazılınca hata. Başka katmanın nesnelerinde değerlendirilen argümandaki ad (`en_yakın('Yol', Ad)`'ın `Ad`'ı)
  girdi nesnelerinin alanlarıyla denetlenmez. Masaüstünün ağacında `@` öğelerinin işareti `@`'dir.
- **Proje ayarları › Değişkenler** (iki platform): bir tablo (Ad, Etiket, Tür, Değer; satırda yukarı, aşağı, sil; Değişken ekle), adın
  ve değerin sorunu satırın altında, varken Kaydet bekler; Tür'ün listesi Metin, Sayı, Doğru/yanlış, Tarih; doğru/yanlış bir anahtardır.
  Altında hazır değişkenler o anki değerleriyle. Kurallar ortak: `kentos_project::variable_form`, `model/variableForm.ts` (§2.3; ad
  kırpılır ve baştaki bir `@` düşer; sayı Hesap pencerelerinin okuduğu gibi, `1,5` da; tarih `YYYY-AA-GG` ya da `GG.AA.YYYY`; yeni satır
  `degisken1`, `degisken2` …; tür değişince doğru/yanlışa geçen değer `yanlış` ya da `doğru`, ondan çıkan boş). Kaydet tek adım (proje
  ayarı); “Bu bölümü varsayılana döndür” değişkenleri kaldırır.
- **Pafta** çizimin değişkenlerini kitabın proje değişkenlerinden sonra okur (`RenderInputs.project.variables`).
- **İkonlar** (sorulmadan seçildi): Değişkenler bölümünün `projectVariables`'ı (süslü parantezler arasında `@`), iki platformda.

### 5. Performans

Saf işlevler partide sütun sütun; tarih okuma zaman çekirdeğinin hızlı yolu; sabit kalıp bir kez derlenir, değerden gelen kalıplar son
16'sı saklanarak. Toplama ve anahtar tabloları değerlendirme başına bir kez; mekânsal ilişkiler deponun uzamsal diziniyle nesne başına
(aday kutular, sonra kesin sınama); düzenli ifade eşleşmenin başlayabileceği ilk bayttan aranır (§2.4). Bütçeler (release, bu makine): 10⁵ nesnede `yıl(Tarih)` ≤ 15 ms, `düzenli_parça(Ad, '(\d+)')`
≤ 40 ms, `dizi_uzunluğu(metin_dizi(Etiketler))` ≤ 40 ms; 10⁴ parselde `$alan / topla($alan, Mahalle)` ≤ 15 ms, `kesişir('Sit alanı')`
(500 sit alanı) ≤ 60 ms, `en_yakın('Durak', Ad)` (2 000 durak) ≤ 80 ms, `katmandan('Mahalle', Ad, 'Kod', MahalleKodu)` ≤ 10 ms; web'de
aynı işler ≤ 2,5 katı.

## Uygulama

- **İfade çekirdeği** (`crates/shared/expression`): değer biçimleri `compound.rs` (dizi ve eşleme: işaretli JSON metni, `shown`, `Item`),
  işlevler `dates.rs` (zaman çekirdeğinin `read`/`write`'ı, takvimle ay ve yıl, ISO hafta, Türkçe biçim), `patterns.rs` (`regex-lite`;
  `Kept`: son 16 kalıp ve grupların yeri; eşleşmenin başlayabileceği baytlar `first_bytes`), `arrays.rs` (diziler, eşlemeler, JSON,
  `Key`, kesin toplam, tam sıra); tablo `library.rs` ve `library/extras.rs` (`Family`, gruplar); sözcükleyici `@`, ayrıştırıcının rolleri
  ve denetimleri (`check_args`: sabit katman ve metin, işlem adı, sabit kalıp, eşlemenin çiftleri), `resolve.rs` (`@`'lerin değerleri,
  başka katmana bakan çağrıların listesi, iç ifadeler); `world.rs` (`World`, `WorldLayer`, `LayerObjects`, `Session`), `world/roles.rs`,
  `world/aggregate.rs` (`kentos.statistics/1`), `world/spatial.rs` (Konuma göre seç'in ilişkileri, en yakın, örtüşmeler); `program`,
  `exec`, `kernels`, `walk`'ta `World` düğümü; `host.rs` (`Variable`, `Schema.variables`, `Schema.world`, `evaluate_objects_in`),
  `rows.rs` (`WorldTable`, `TableLayer`, `evaluate_rows_on_in`); `compile_with` başka katmana bakanı şema izin vermedikçe reddeder
  (`WORLD_REFUSED`); `api.rs` (`exprCompileIn`, şema olarak alanlar, değişkenler ve izin); İfade oluşturucu'nun hizmetleri
  (`editor::lex` `@`, `catalog` Değişkenler grubu ve izne göre işlevler, `complete` `@` tamamlaması, `check` reddin yeri, bilinmeyen `@`'ler,
  başka katmanın argümanındaki adlar, `flow`).
- **Sözleşme ve dosya:** `kentos_contracts::variables` (`ProjectVariable`, `VariableKind`, `VariableValue`, `BUILTIN_VARIABLES`, kurallar),
  `ProjectSettings.variables`; `.kcad` şema 37 (`FORMATS_VERSION` 47; `decode/variables.rs`, `encode/variables.rs`; anahtarlar kind, name,
  label, value), `docs/specs/kcad-v2.md` §6.4.9, bağımsız Python okuyucusu `tools/kcad/kcad.py`, örnekler `fixtures/kcad/v2/variables.*` ve
  on bozuk dosya.
- **Proje:** `kentos_project::variables` (yerleşik değerler), `variable_form` (Değişkenler'in formu); web `model/projectVariables.ts`
  (`settingsSource`, `documentSource`, `expressionVariables`), `model/variableForm.ts`, `app/expressionVariables.ts` (oturumdaki kişiyle).
- **İşlemler:** masaüstü `kentos_processing::expression` (`EntityObjects`, `View`, `Evaluation`, `evaluate_in`), `runner.rs` (`world_layers`,
  önizleme ve çalıştırmada katmanlar, süzgecin dışarıda bıraktıkları dışında), `types.rs` (`RunContext.world`), araçlar `calculate_field`,
  `select_by_expression`, parametre denetimi izinle; web `processing/job.ts` (`RunJob.variables`, `RunJob.leftOut`, `jobLayers`),
  `runner.ts` (`variables()`, `layers()`), `features.ts` (`ProcessingHost.variables`), `app/processing.ts`.
- **WASM:** `exprEvaluateIn`, `ExprWorld`, `GeometryStore.evaluateExpressionIn`; web `wasm/core.ts`, `model/expression/expression.ts`
  (`ExprContext`, `ExprLayers`, `worldTable`), `model/expression/layers.ts`, `builderObjects.ts`.
- **Arayüz:** İfade oluşturucu web'de `ui/expression/` (`ExprSchema` ile `CodeEditor`, `BuilderTree`, `FlowMode`; `builderApi`'nin
  `variables` ve `world`'ü), masaüstünde `expression/mod.rs` (`attribute_fields`, `expression_variables`, `variables_of`), `objects.rs`
  (başka katmana bakan önizleme), `view.rs`; Öznitelik tablosunun süzgeci iki platformda `@` değerleriyle; Proje ayarları › Değişkenler web
  `ui/settings/ProjectSettingsDialog.ts` (`variablesSection`, `styles/settings.css`'in `.svars`'ı), masaüstü `project/variables.rs` ve
  `project/settings.rs`; ikon `ui/icons.ts`'in `projectVariables`'ı (masaüstüne envanterle).
- **Pafta:** `kentos_sheet::display::ProjectInfo.variables`, `sheet_scope` kitabınkilerden sonra; web `app/sheet/inputs.ts`, masaüstü
  `sheets.rs` ve `sheet_inputs::sheet_variable`.
- **Python:** `kentos.cad.types` (`ProjectVariable`, `VariableKind`, `VariableValue`); üretici `scripts/python/sdk.py` birkaç JSON türünün
  birleşimini (`anyOf`) adlı tür olarak yazar.
- **Bağımsız başvurular:** `scripts/fixtures/expression_extras.py` (`fixtures/expression/v2/extras.json`, 190 kaynak), `spatial_query_cases.py`
  (`fixtures/processing/v1/queries.json`'un altı ifade durumu), `kcad_v2_reference.py` (değişkenler), `variable_form_cases.py`
  (`fixtures/project/v1/variable-form.json`: okumalar, yazım, yeni satır, tür değişimi, yerleşik değerler).
- **Resimler:** masaüstü `project::variables::tests::screens` (`degiskenler-*`), `expression::tests::world_screens` (`ifade-degiskenler-*`);
  web `shots.mjs variables` (`degiskenler-*`, `ifade-degiskenler`).
- **Süre ölçümleri:** `cargo test --release -p kentos-expression --test perf measures_the_additions -- --ignored --nocapture`, web
  `apps/web/scripts/perf/expressionExtras.test.ts`.


## Doğrulama

10 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `expression_extras.py` (`extras.json`: 190 kaynak, beş katmanlı
  bir dünyada her biri altı parselde; mekânsal değerler 1e−9 göreli), `spatial_query_cases.py` (`queries.json`'un altı ifade durumu:
  Öznitelik hesapla ve İfadeyle seç ile toplama, kesişen sayısı, en yakın, başka katmandan değer), `kcad_v2_reference.py` (527 dosya:
  `variables.kcad`, on bozuk dosya, şema 38'in reddi), `variable_form_cases.py` (7 okuma durumunda 34 satır, yazım, yeni satır, tür
  değişimi, üç projede yerleşik değerler), `expression_language.py` ve `expression_geometry.py` (değişmedi, geçer).
- **Platformlar:** `extras.json` çekirdekte (tek nesne yolu ve dünya oturumu, `tests/language.rs`) ve web'de (WASM, sayfanın tablosu ve
  katmanların tek tablosuyla, `model/expression/extras.test.ts`) aynı değerleri verir; `builder.json` (215 durum; 15'i yeni: `@`
  sözcükleri, tamamlama, yardım, ret, bilinmeyen `@`, değişkenin okunmadığı alan, başka katmanın argümanındaki adlar) iki platformda;
  `variable-form.json` iki platformda; `queries.json`'un ifade durumları masaüstünde ve web'de (sayfada ve işçi yolunda); katman süzgecinin
  başka katmana bakan işlevden düşürdükleri iki platformda aynı sınamayla (hepsi 2, 2, 1, 1; yalnız çamlar 1, 1, 0, 0). Paftada çizimin
  değişkenleri kitabınkilerden sonra (`kentos-sheet`, 363 + 51). Python `kentos.cad` (61), MCP (11). Resimler iki platformda 1440×900 ve
  1100×650'de, iki temada: masaüstü `degiskenler-*` ve `ifade-degiskenler-*`, web `shots.mjs variables`; `e2e:layout`'a
  `project-settings-variables` eklendi.
- **Bulunan ve düzeltilen:** başvurunun iki hatası (eksi sıfır yıl farkı, eşleşmeyen `düzenli_değiştir`) ve çekirdeğin iki hatası
  (`katman_toplamı`'nın ayracı, boş grupta `say`). İfade oluşturucu başka katmanın argümanındaki adı (`en_yakın('Yol', Ad)`) girdi
  nesnelerinin alanıyla denetleyip yanlış uyarıyordu: argümanların rolleriyle atlanır. Başka katmana bakan işlevler katman süzgecinin
  dışarıda bıraktıklarını okuyordu (girdilerin kuralına aykırı): iki platformda düşer, web'de işle (`RunJob.leftOut`). Değeri olmayan
  değişken dosyada yazılmıyor, bellekte `null` yazılıyordu: sözleşme boş değeri yazmaz. Başka katmana bakan işlevlerin reddi her ev
  sahibinde ayrıydı (`compile_local`): tek kural `compile_with`'e taşındı, eksik kalan yerler (raster hesaplayıcı, pafta, ağ) kendiliğinden
  reddeder. Masaüstünün İfade oluşturucu'su başka katmana bakan ifadeyi tek nesne yolunda boş gösteriyordu: sütun motoruyla değerlendirir.
  Pafta testlerinin sahte ayarlarında `variables` yoktu. Clippy'nin bulguları (JavaScript'in `max`'ı, `is_multiple_of`, türetilebilen
  `Default`, büyük `enum` kolu) düzeltildi.
- **Takım:** `pnpm typecheck`; `pnpm test` (352 dosya, 4861 test geçti, 26 atlandı); `pnpm rust:test` (3056 geçti, 39 yok sayıldı; clippy
  ve bağımlılık yönü temiz; clippy düzeltmelerinden sonra `kentos-expression` yeniden: 75); `pnpm rust:test:desktop` (1279 geçti, 196 yok
  sayıldı; clippy temiz); `pnpm py:test` (61); `cargo test -p kentos-mcp` (11); `pnpm e2e:interaction` (142 iz × 3 varyant); `pnpm e2e`
  (209 denetim); `KENTOS_E2E_DB=scratch pnpm e2e:cloud` (108 denetim; proje ayarı sunucudan geçer); `pnpm build` (çekirdek WASM
  4 711,25 kB, gzip 1 699,26 kB; Proje ayarları penceresi 42,4 kB); `pnpm inventory:check`; `e2e:layout`'ta Proje ayarları'nın Genel,
  Birimler ve Değişkenler'i (12 görünüm, sorunsuz). Gerçek GPU sınaması (`KENTOS_GPU_TESTS`) ve `pnpm e2e:visual` çalıştırılmadı: çizim
  hattına dokunulmadı.
- **Süreler** (release; masaüstü `measures_the_additions`'ın, web `scripts/perf/expressionExtras.test.ts`'in gönderilen WASM'la 20 koşusunun
  p50'si; ölçerken masaüstü uygulaması açıktı):

  | İş | Masaüstü | Bütçe | Web | Web bütçesi |
  |---|---|---|---|---|
  | `yıl(Tarih)`, 10⁵ nesne | 6,5 ms | 15 | 17,1 ms | 37,5 |
  | `düzenli_parça(Ad, '(\d+)')`, 10⁵ nesne | 27,3 ms | 40 | 56,3 ms | 100 |
  | `dizi_uzunluğu(metin_dizi(Etiketler))`, 10⁵ nesne | 23,0 ms | 40 | 37,3 ms | 100 |
  | `$alan / topla($alan, Mahalle)`, 10⁴ parsel | 4,3 ms | 15 | 9,2 ms | 37,5 |
  | `kesişir('Sit alanı')`, 10⁴ parsel, 500 sit | 24,1 ms | 60 | 27,7 ms | 150 |
  | `en_yakın('Durak', Ad)`, 10⁴ parsel, 2 000 durak | 33,7 ms | 80 | 39,9 ms | 200 |
  | `katmandan('Mahalle', Ad, 'Kod', MahalleKodu)`, 10⁴ parsel | 1,8 ms | 10 | 4,0 ms | 25 |

  İlk ölçümde `düzenli_parça` 46,6 ms'ydi (bütçe 40): motorun payı 4,5 ms, kalanı `regex-lite`'ın taraması (nesne başına ~350 ns). Grupların
  yeri kalıpla saklandı (44,3 ms); önce düz arama sonra grupları yalnız eşleşmeden aramak daha yavaştı (62,7 ms: `regex-lite`'ta düz arama
  da aynı makineyi koşturur) ve bırakıldı; eşleşmenin başlayabileceği ilk bayttan arama (§2.4; 4 000 rastgele kalıpta süzgeçsiz aramayla
  aynı sonuç) 27,3 ms verdi. Maddenin bütünü çekirdek WASM'ı (`--profile wasm`) 4 757 090 bayttan 5 054 655 bayta büyüttü (+297 565, %6,3;
  brotli +72 709 bayt, 1 194 483).
