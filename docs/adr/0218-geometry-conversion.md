# ADR 0218: Geometri dönüştürme

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-22`; ADR 0065 (çizimin Alana çevir ve Çizgiye çevir'i), ADR 0140 (Parçala, yolun parçası), ADR 0142
  (köşe kotu ve kotların taşınması), ADR 0143 ve 0174 (çok parçalı nesneler), ADR 0152 (Köşelere nokta), ADR 0180 (sayının okunuşu),
  ADR 0200 §7 (dosya parametresi), ADR 0201 (geometri işlemleri: Çıktı katmanı, girdi değişmez), ADR 0203 (hata elipsleri), ADR 0216
  (kaynak: dosya ya da katman), ADR 0217 §6 (sıralamanın kuralı).
- **Araştırma:** ArcGIS'in Feature To Point (CENTROID, INSIDE), Feature Vertices To Points (ALL, START, END, BOTH_ENDS, MID), Points To
  Line (Line Field, Sort Field, Close Line), Bearing Distance To Line, XY To Line, Table To Ellipse ve Densify'ı; QGIS'in Centroids (her
  parça için), Point on surface (GEOS'un iç noktası), Extract vertices ve Extract specific vertices, Points to path (sıra ve grup ifadesi,
  kapalı yol), Line substring, Polygons to lines, Lines to polygons, Densify by interval ve Densify by count'u.

## Bağlam

Çizimde Alana çevir, Çizgiye çevir ve Patlat nesneleri tek tek ve yerinde çevirir (ADR 0065, 0047). CBS işi toplu ister: bir katmanın
bütün parsellerinden merkez noktaları, bir yolun köşelerinden numaralı noktalar, saha noktalarından sıralı güzergâh, bir tablodaki semt ve
uzunluklardan çizgiler, hata elipslerinden elipsler, bir hattın iki km arasındaki parçası, sınırlardan çizgiler ve tersi, yeniden
izdüşümden ya da sayısallaştırmadan önce sıklaştırılmış kenarlar. Bunlar İşlemler'in işidir: girdisi kapsamla seçilir, modelde adım olur,
raporu vardır, tek geri alma adımıdır; öznitelikler sonuca taşınır.

## Karar

### 1. Kapsam

İşlemler'in yeni **Dönüştürme** kategorisinde on araç, iki platformda:

1. **Nesneden nokta** (`convert.toPoints`): ağırlık merkezi ya da iç nokta.
2. **Köşeleri noktaya çevir** (`convert.vertices`).
3. **Noktalardan çizgi** (`convert.pointsToPath`).
4. **Semt ve uzunluktan çizgi** (`convert.bearingLine`).
5. **XY'den çizgi** (`convert.xyLine`).
6. **Tablodan elips** (`convert.tableEllipse`).
7. **Çizginin parçası** (`convert.substring`).
8. **Alanları çizgiye çevir** (`convert.polygonsToLines`).
9. **Çizgileri alana çevir** (`convert.linesToPolygons`).
10. **Köşe ekle** (`convert.densify`): aralıkla ya da sayıyla.

Ortak kurallar:

- Sonuç **Çıktı katmanı**na yeni nesnelerdir (varsayılan yeni bir katman, ADR 0201'in `layer` parametresi); girdi değişmez. Girdi
  yalnız okunduğundan kilitli katmandaki nesneler de okunur (ADR 0201 gibi).
- Yeni nesne kaynağının özniteliklerini taşır; tablodan gelen nesne satırın dolu hücrelerini (sütunun adı, hücre). Aracın eklediği
  öznitelik aynı adlı kaynak özniteliğin yerine geçer.
- Hesaplar çekirdekte (`ops::convert`), iki platformda aynı. Kotlar ADR 0142'nin kuralıyla taşınır: köşede köşenin, kenarın üstündeki
  yeni köşede kenar boyunca; merkezin ve iç noktanın kotu yoktur ve kotlu girdiler söylenir (“n nesnenin kotları sonuca taşınmadı.”).
- Sayı yazan öznitelikler (uzaklık) projenin uzunluk basamağıyla, gösterim kuralıyla (`display::fixed`, ADR 0149) yazılır.
- Atlanan nesne ve satırlar nedenleriyle söylenir; tablodan okuyan araçların raporu **Satır**, **Neden** tablosudur.

**Kapsam dışı:** jeodezik çizgi ve elips (XY To Line'ın GEODESIC'i; hepsi projenin düzleminde), sınır üstünde nokta ve üç boyutlu merkez,
uçları toleransla buluşan çizgileri kapatmak (Çizgileri alana çevir'in kuralı Alana çevir'inki), çizgileri kesişimlerinden alanlara
çevirmek (Toplu alan, ADR 0151), yerinde sıklaştırma (çizimin Köşe ekle'si).

### 2. Nesneden nokta

- **Girdi** nokta, çizgi, çoklu çizgi, kapalı alan, daire ve yay; **Konum** Ağırlık merkezi ya da İç nokta; **Her parça için** (çok
  parçalı nesnenin her parçası bir nokta, **Parça** özniteliği 1, 2 …); **Çıktı** (varsayılan yeni “Merkezler”).
- **Ağırlık merkezi:** alanda alanın merkezi (delikler çıkar, yaylı kenar kesin; ifadenin `$merkez_y`'si, ADR 0100), dairede merkezi;
  çizgide uzunlukla ağırlıklı merkez: doğru parçasının ortası, yayın merkezi kapalı biçimde (çemberin merkezinden yayın orta doğrultusunda
  r·sin α / α, α yayın yarım açısı); nokta kendisi, çok noktalı nesnede noktaların ortalaması; çok parçalı alanda parçaların alanlarıyla
  ağırlıklı merkezi. Alanı sıfır olan alanın merkezi sınırının, uzunluğu sıfır olanınki ilk köşesidir. Merkez nesnenin dışında kalabilir
  (L biçimli alan, kıvrık çizgi).
- **İç nokta** her zaman nesnenin üstündedir: alanda GEOS'un iç nokta kuralı: tarama yüksekliği sınır kutusunun ortasının hemen altındaki
  ve hemen üstündeki köşe (yayda tepe noktası da) yüksekliklerinin ortasıdır; o yükseklikteki yatay doğrunun alanın içinde kalan
  aralıklarının en genişinin ortası (eşitlerde batıdaki); çok parçalı alanda parçaların en geniş aralığı (eşitlerde ilki); genişliği
  olmayan alanda sınırının yarı uzunluğundaki nokta. Çizgide yolun yarı uzunluğundaki nokta (çok parçalıda ilk parçanın); nokta kendisi;
  çok noktalı nesnede noktaların ortalamasına en yakın noktası (eşitlerde ilki). Düz kenarlı alanda bu, GEOS'un kendi iç noktasıdır
  (shapely'nin `representative_point`'i; GEOS'un kesişim formülüyle bit bit).
- Özet “n nesneden k nokta yazıldı.”

### 3. Köşeleri noktaya çevir

- **Girdi** çizgi, çoklu çizgi, kapalı alan ve yay; **Köşeler** Bütün köşeler, Başlangıç, Bitiş, İki uç ya da Orta; **Çıktı** (varsayılan
  yeni “Köşeler”).
- Bütün köşeler: dış halka, sonra delikler, çok parçalıda parça parça; kapalı halkanın ilk köşesi bir kez; yayın uçları. Her nokta
  kaynağın özniteliklerine ek **Köşe** (nesnede 1'den sıra), **Parça** (1'den), **Halka** (0 dış halka ya da çizgi, 1 … delikler),
  **Uzaklık** (halkanın ya da yolun başından yol boyunca, yaylar kesin, m) ve köşenin kotunu alır.
- Başlangıç ve Bitiş yolun (alanda dış halkanın) ilk ve son köşesidir; kapalı halkada ikisi de ilk köşedir ve İki uç onu bir kez verir.
  Orta yolun yarı uzunluğundaki noktadır (kotu kenarı boyunca); köşe olmadığından **Köşe** yazılmaz.
- Özet “n nesneden k nokta yazıldı.”

### 4. Noktalardan çizgi

- **Girdi** noktalar; **Sıra** (ifade, değer; isteğe bağlı: boşsa girdinin sırası), **Grup** (alan; isteğe bağlı: her değer bir çizgi),
  **Kapat** (ilk noktaya dönen kapalı alan), **Çıktı** (varsayılan yeni “Yollar”).
- Sıralama ADR 0217 §6'nın kuralıdır (`ops::order`): artan, boşlar sonda, eşitler girdinin sırasıyla. Çok noktalı nesnenin noktaları
  kendi sırasıyla aynı yerde. Gruplar değerlerinin girdide ilk görüldüğü sırayla (değer boşluklar atılarak; boş “(boş)”).
- Bir öncekiyle aynı yerdeki nokta atlanır (sayılır); ikiden az noktalı grup (Kapat'ta üçten az) çizgi vermez (sayılır).
- Çizginin öznitelikleri: grup alanı ve değeri, **Nokta sayısı**, Sıra yazılmışsa **Başlangıç** ve **Bitiş** (ilk ve son noktanın sıra
  değeri, ifadenin metin değeri); köşelerin kotları noktalarınki.
- Özet “n noktadan k çizgi yazıldı.” (n çok noktalı nesnelerin noktaları dahil); iletiler “n nokta bir öncekiyle aynı yerde olduğu için
  atlandı.”, “k grupta çizgi için yeterli nokta yok (en az 2).”

### 5. Semt ve uzunluktan çizgi

- **Kaynak** Dosya (CSV, TXT, Excel; ADR 0200 §7) ya da Katman (nesnelerin öznitelikleri satırdır; ADR 0216); **Doğu**, **Kuzey**,
  **Semt** ve **Uzunluk** sütunları; **Açı birimi** Grad ya da Derece (varsayılan projeninki); **Çıktı** (varsayılan yeni “Semt çizgileri”).
- Çizgi (doğu, kuzey) noktasından başlar, semt doğrultusunda (kuzeyden saat yönünde) uzunluk kadar gider: bitiş (doğu + u·sin θ,
  kuzey + u·cos θ).
- Hücreler ADR 0180'in kuralıyla okunur: isteğe bağlı işaret, rakamlar ve en çok bir ondalık nokta ya da virgül (binlik ayırıcı ve üs
  yok). Satırlar ve şekiller çekirdekte (`table::bearing_rows`; web'de bir çağrıda bütün satırlar). Boş, sayı olmayan ve sıfırdan büyük
  olmayan hücreli satır atlanır: rapor **Satır** (dosyada satır numarası, başlık 1.; katmanda “#kimlik”) ve **Neden** (““Semt” boş”,
  ““Doğu” sayı değil: abc”, ““Uzunluk” sıfırdan büyük değil”); ileti “m satır atlandı; nedenleri tabloda.” Seçilen sütun kaynakta yoksa
  çalıştırma reddedilir (Anahtarla birleştir'in sözüyle). Çizgi satırın boş olmayan hücrelerini öznitelik olarak alır.
- Özet “n satırdan k çizgi yazıldı.”

### 6. XY'den çizgi

- **Kaynak**, **Başlangıç doğu**, **Başlangıç kuzey**, **Bitiş doğu**, **Bitiş kuzey** sütunları, **Çıktı** (varsayılan yeni “Çizgiler”).
  Satırların kuralı §5'inki; başlangıcı ve bitişi aynı yerde olan satır atlanır (“Başlangıç ve bitiş aynı yerde”).

### 7. Tablodan elips

- **Kaynak**, **Doğu**, **Kuzey**, **Büyük yarı eksen**, **Küçük yarı eksen**, **Semt** (büyük eksenin doğrultusu; isteğe bağlı: seçilmezse
  ya da hücre boşsa kuzey), **Açı birimi**, **Çıktı** (varsayılan yeni “Elipsler”). Tam elips, çizimin Elips'i gibi (t0 = t1).
- Elips nesnesi (ADR 0057): merkez, büyük eksen semt doğrultusunda, oran küçük / büyük. Küçük büyükten büyükse ikisi yer değiştirir ve
  doğrultu dik açı döner; eşitse oran 1'dir. Yarı ekseni sıfır ya da eksi satır atlanır.
- Yatay ağ dengelemesinin hata elipsleri (ADR 0203: a, b ve büyük eksenin semti) doğrudan girer.

### 8. Çizginin parçası

- **Girdi** çizgi, çoklu çizgi ve yay; **Başlangıç** ve **Bitiş** (ifade, değer; varsayılan `0` ve `$uzunluk`): yolun başından yol
  boyunca metre; **Çıktı** (varsayılan yeni “Parçalar”).
- Uzaklıklar [0, uzunluk] aralığına alınır; parça yolun ADR 0140'taki kesin parçasıdır (doğru parçaları doğru, yaylar yay; çizgiden
  çizgi, yaydan yay, tek düz kenarda kalan parça çizgi). Başlangıcı bitişinden küçük olmayan, değeri sayı olmayan ve çok parçalı nesne
  yazılmaz; iletiler “n çok parçalı çizgi alınmadı.”, “n nesnede uzaklık sayı değil; yazılmadı.”, “n nesnede başlangıç bitişten küçük
  değil; yazılmadı.”
- Köşelerin kotları kaynağın kenarları boyunca (ADR 0142 kuralları 1 ve 3).

### 9. Alanları çizgiye çevir

- **Girdi** kapalı alanlar; **Çıktı** (varsayılan yeni “Sınırlar”). Her halka (dış halka, delikler; çok parçalıda parça parça) bir çoklu
  çizgi, son köşesi ilki (çizimin Çizgiye çevir'i), yaylarıyla ve kotlarıyla; öznitelik **Parça** ve **Halka** (0 dış, 1 … delikler).
- Özet “n alandan k çizgi yazıldı.”

### 10. Çizgileri alana çevir

- **Girdi** çoklu çizgiler; **Açık çizgiler** Atla ya da Kapat (son köşe ilkine düz kenarla bağlanır); **Çıktı** (varsayılan yeni
  “Alanlar”).
- Kapalılık çizimin Alana çevir'inin kuralıdır (uçlar arası en çok 1 µm; ADR 0065); yinelenen son köşe düşer, son kenarın yayı kapanış
  kenarının olur; yaylar ve kotlar kalır. Alanı sıfır olan (iki köşeli düz çizgi) ve çok parçalı çizgi alan vermez, açık çizgi Atla'da
  yazılmaz; iletiler “n açık çizgi atlandı.”, “n çizginin alanı yok; yazılmadı.”, “n çok parçalı çizgi alınmadı.”
- Özet “n çizgiden k alan yazıldı.”

### 11. Köşe ekle

- **Girdi** çizgi, çoklu çizgi ve kapalı alan; **Yöntem** Aralıkla ya da Sayıyla; **Aralık** (m), **Sayı** (kenar başına eklenecek köşe,
  1–1000); **Çıktı** (varsayılan yeni “Sıklaştırılan”).
- Aralıkla: her kenar n = ⌈uzunluk / aralık⌉ eşit parçaya bölünür (parçalar aralıktan uzun olmaz); Sayıyla: n = sayı + 1. Yaylı kenar aynı
  yayın n alt yayına bölünür: yay kalır, yeni köşeler yayın üstündedir. Delikler ve parçalar da; çizgi çoklu çizgi olur.
- Yeni köşelerin kotları kenar boyunca (ADR 0142 kuralı 3).
- Bir çalıştırma en çok 10000000 köşe ekler: sayı önce şekiller kurulmadan hesaplanır (`densify_added`), aşılırsa çalıştırma reddedilir
  (“Bu ayarla n köşe eklenirdi; bir çalıştırma en çok 10000000 köşe ekler. Aralığı büyütün ya da sayıyı küçültün.”).
- Özet “n nesneye k köşe eklendi.”; `count` çıktısı eklenen köşe sayısıdır.

### 12. Arayüz

- Pencereler tanımdan; tablolar formun altında. CBS'de Veri sekmesinde Öznitelik'in yanında Toplu çevir paneli: Nesneden nokta ve
  Köşeleri noktaya çevir görünür, öbürleri ▾'de (Düzenle 1100 pikselde dolu; onun Dönüştür'ü taşıma, döndürme ve ölçeklemedir);
  araç kutusunda Dönüştürme kategorisi; CAD'de İşlemler'den.
- **İkonlar** (sorulmadan): §Uygulama.

### 13. Performans

Bütçeler (release, bu makine; denetim, araç ve tek adım): 100 000 parselde iç nokta masaüstünde ≤ 300 ms, web'de ≤ 800 ms; bütün köşeler
(400 000 nokta) ≤ 800 / 2000 ms; 5 m aralıkla köşe ekle (800 000 köşe) ≤ 800 / 2000 ms; 100 000 noktadan 1000 grupta sıralı çizgiler
≤ 300 / 800 ms; 100 000 satırdan semt ve uzunlukla çizgi ≤ 300 / 800 ms. Web'de bir araç bir listeyi çekirdeğe tek çağrıda verir.
Büyük dosya değerinin kopyası (100 000 satırda 50 MB'lık JSON) çalıştırmanın sonunda masaüstünde ayrı bir iş parçacığında bırakılır
(çizimin iş parçacığında 270 ms sürüyordu), çalıştırma geçmişi dosyanın satırlarını tutmaz (son değerler gibi yalnız adı ve yolu), hesap
işin değerlerini ödünç alır.

## Uygulama

- **Çekirdek:** `ops::convert` dört dosyada: ortaklar ve nesnenin yolları kotlarının sırasıyla (`mod.rs`: `paths`, `Zs`, `Elevations`),
  `points` (`centroid`, `interior_point`: GEOS'un tarama doğrusu yaylarıyla; `vertex_points`, `Which`), `paths` (`substring` ADR 0140'ın
  `sub_path`'iyle ve ADR 0142'nin `carry_elevations`'ıyla, `rings_as_lines`, `line_area`, `densify`, `densify_added`), `table` (`radians`,
  `bearing_line`, `table_ellipse`, satır okuyucuları `bearing_rows`, `xy_rows`, `ellipse_rows` ve `RowFault`); web için toplu çağrılar
  `calls.rs` (`convertPoints`, `convertVertices`, `convertSubstring`, `convertRings`, `convertLineAreas`, `convertDensifyCount`,
  `convertDensify`, `convertBearingRows`, `convertXyRows`, `convertEllipseRows`).
- **Web:** `model/ops/convert.ts`, `processing/builtin/convert/` (`shared.ts`: çıktı katmanı, kotlar, kaynak ve sütun parametreleri,
  satırlar ve nedenler; `toPoints.ts`, `vertices.ts`, `pointsToPath.ts`, `tables.ts`, `paths.ts`). Kategori var olan `conversion`
  (Dönüştürme), ikonu ve açıklaması yeni.
- **Masaüstü:** `kentos-processing`'in `builtin/convert/`'u (`mod.rs`, `points.rs`, `tables.rs`, `paths.rs`); çalıştırıcıda geçmiş kaydı
  dosya satırlarını tutmaz (`kept_values`), hesap işin değerlerini ödünç alır (`Resolved::borrowed`, değerler `Cow`), büyük dosya
  satırlı değerler ayrı iş parçacığında bırakılır (`release`).
- **Şerit:** CBS'nin Veri sekmesinde Toplu çevir (`app/ribbon.ts`'in `CONVERT_COMMANDS`, `CONVERT_UNDER`; Analiz'in İşlemler menüsünde
  `conversion` yok). Envanterin `ribbonByMode`'u masaüstünün şerididir.
- **İkonlar** (sorulmadan seçildi): kategori `conversion` (kare çembere dönüşür), Nesneden nokta `featureToPoint` (alan ve ortasındaki
  nokta), Köşeleri noktaya çevir `verticesToPoints` (yolun köşelerinde noktalar), Noktalardan çizgi `pointsToPath` (noktaları birleştiren
  oklu çizgi), Semt ve uzunluktan çizgi `bearingLine` (kuzeyden açıyla çizgi), XY'den çizgi `xyLine` (iki işaretli yer arası çizgi),
  Tablodan elips `tableEllipse` (ekseniyle elips), Çizginin parçası `lineSubstring` (yolun kalın kesilmiş parçası), Alanları çizgiye çevir
  `polygonToLine` (alanın uçlu halkası), Çizgileri alana çevir `lineToPolygon` (kesikli kenarla kapanan dolu alan), Köşe ekle `densify`
  (kare köşeler arasında küçük noktalar).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/convert_cases.py` (KentOS kodu olmadan ADR'den): alanların merkezleri Green teoremiyle, yolların
  merkezleri ∫(x, y) ds / L ile, her kenar kendi parametresiyle mpmath'in 50 basamaklı integraliyle; düz kenarlı alanların iç noktası
  GEOS'un kendisinden (shapely 2.1, GEOS 3.13: `representative_point`), yaylıların ADR'nin tarama doğrusuyla 50 basamakta; yol üstündeki
  noktalar, parçalar ve sıklaştırma 50 basamakta; semt ve elipsler mpmath'le; sıralama `layer_ops_cases.py`'nin `order`'ıyla; dosyalar
  Tablo ekle'nin bağımsız okuyucusuyla (`table_file_cases.py`). 23 durum: merkezler ve iç noktalar (L, delikli, yaylı, iki parçalı, alanı
  olmayan alan; yollar; parçalar; kilitli katman), köşeler (bütün, uçlar, orta, bitiş), noktalardan çizgi (gruplu, kapalı), tablolar
  (dosyadan ve katmandan, grad ve derece, okunamayan, boş, sıfır ve aynı yer), parçalar (sabit ve ifadeli), halkalar, alanlar (Atla,
  Kapat), köşe ekle (aralık, sayı); `fixtures/processing/v1/convert.json`, `convert.kcad` ve üç tablo. Hesaplanan yaylar, kotlar, açılar
  ve parçalar dosyanın `toleranceFields`'ıyla 10 nm içinde karşılaştırılır (öbür dosyalar tam eşitlikte kalır). İki platformun
  oynatıcıları (`cases.test.ts` sayfada ve işçide, `cases.rs`'in `the_conversion_cases_do_what_they_say`'i çizimde ve okuma kopyasında)
  23'ünü geçer; çekirdeğin 9 birim testi (`ops::convert`).
- **Pencere:** masaüstünde `processing::convert_tests` (23 durum pencereden, dosyalar seçilerek); formlar ve metinler `dialog.json`'da.
- **Resimler:** masaüstünde `cargo test -p kentos-desktop processing::convert_tests::screens -- --ignored --nocapture
  --test-threads=1` (`.run/shots/islem-donustur-*`), web'de `(cd apps/web && node scripts/e2e/shots.mjs geoconvert)`; yerleşim
  `(cd apps/web && node scripts/e2e/layout.mjs --only tab-data,tab-edit,processing-convert-bearing,processing-convert-densify)`.
  Düzenle'ye konan panel 1100 pikselde şeridi taşırdı (denetim yakaladı); panel Veri'ye taşındı.
- **Süreler** (11 Ekim, bu makine, release; p50 / p95, denetim, araç ve tek adım; masaüstünde her çalıştırma tek başına):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | Nesneden nokta, 100 000 parsel, iç nokta | 200,0 / 203,0 ms | 630,1 / 656,2 ms | 300 / 800 ms |
  | Köşeleri noktaya çevir, 100 000 parsel, 400 000 nokta | 657,3 / 702,1 ms | 1892,4 / 1954,9 ms | 800 / 2000 ms |
  | Köşe ekle, 100 000 parsel, 5 m aralık, 800 000 köşe | 290,6 / 296,2 ms | 1872,9 / 1881,6 ms | 800 / 2000 ms |
  | Noktalardan çizgi, 100 000 nokta, 1000 hat, sıralı | 149,7 / 151,1 ms | 152,9 / 165,0 ms | 300 / 800 ms |
  | Semt ve uzunluktan çizgi, 100 000 satır | 237,4 / 243,4 ms | 514,8 / 607,7 ms | 300 / 800 ms |

  Masaüstünde parçaları (hazırlık, araç, uygulama): nokta 25,1 / 114,9 / 58,7 ms, köşeler 36,6 / 359,6 / 273,8 ms, köşe ekle
  25,7 / 184,3 / 69,0 ms, çizgiler 28,4 / 120,8 / 1,4 ms, semt 26,7 / 153,2 / 64,0 ms. Semt ve uzunluktan çizgi ilk ölçümde 576,8 ms'ydi
  (uygulama 518,6 ms: geçmiş kaydına dosyanın kopyası ve işin sonunda 50 MB'ın bırakılması); kayıt satırları tutmayınca, hesap
  ödünç alınca ve bırakma ayrı iş parçacığına geçince 237,4 ms. Komutlar: `cargo test --release -p kentos-processing --test cases
  convert_timing -- --ignored --nocapture`; web'de `KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; CONVERT_BENCH=1 pnpm -C apps/web exec
  vitest run scripts/perf/convert.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp`.
