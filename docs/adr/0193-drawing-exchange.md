# ADR 0193: Çizimler arası alışveriş

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-30`'un ardından `CAD-31`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler AutoCAD'in WBLOCK'u (Blok
  yaz), DesignCenter'ı ve dosyadan INSERT'i, Netcad'in Seçilen Objeleri Kaydet, Başka Projeden Özellik Ekle ve Ekle'sidir.
- **Bağlam belgesi:** TODOS.md `CAD-31`; ilgili `CAD-10`, `DOM-08`. ADR 0025 (`.kcad`), ADR 0144 (blok), ADR 0177 (Kullanılmayanları
  temizle: kullanılanların kuralı), ADR 0183 (yazı ve ölçü stilleri, içe aktarmanın birleştirmesi), ADR 0175 (bağlı yazı), ADR 0186
  (ilişkili tarama), ADR 0184 (tablonun kaynağı), ADR 0192 (resim).

## Bağlam

Bugün çizimler arasında yalnız oturumun panosu var (Kes, Kopyala, Yapıştır, Özgün koordinatlara yapıştır): nesneler gider ama katmanları,
blokları, stilleri ve kitaplığı ancak nesnelerle ve eksik gider; bir çizimin bir parçasını ayrı dosya yapmanın, başka bir projenin
katmanlarını ve stillerini almanın, bir çizimi blok olarak yerleştirmenin yolu yok.

## Karar

### 1. Seçilenleri dosyaya kaydet

Komut `file.saveSelection` (Dosya; CAD'de Ekle › Alışveriş, CBS'de Veri › Dosya alışverişi): seçili nesneler yeni bir `.kcad` olur, açık çizim
değişmez. Yeni çizim (“seçimin çizimi”):

- **Nesneler:** seçilenler, çizimin sırasıyla; kalıcı kimlikleri (`uid`) korunur, yuvaları 1'den yeniden.
- **Katmanlar:** ağaç, nesnelerin ve kalan blok tanımlarının nesnelerinin katmanlarına ve onların üst gruplarına budanır; kalan her düğüm
  kimliği, adı, stili, görünürlüğü, kilidi ve açıklığıyla. Etkin katman açık çizimdeki kaldıysa odur, değilse ağacın ilk katmanı.
- **Bloklar:** nesnelerin yerleştirdiği tanımlar ve onların yerleştirdikleri (iç içe), çizimin sırasıyla.
- **Kitaplık:** kalan nesnelerin, blok nesnelerinin ve katmanların kullandığı öğeler (Kullanılmayanları temizle'nin kullanılanlar
  kuralı, ADR 0177): nesnelerin sembolleri (`symbol`), katman stillerinin adlandırdıkları (`ref`), bu sembollerin ve katman stillerinin
  görüntüleri (`asset`), resim nesnelerinin görüntüleri; şablonlar alınmaz; kategoriler olduğu gibi.
- **Ayarlar:** projenin ayarları olduğu gibi (koordinat sistemi, birimler, ölçek, yazı ve ölçü stilleri, ölçme); katman durumlarının kalan
  katmanlara ait girdileri (girdisi kalmayan durum düşer).
- **Bağlar:** kaydedilmeyen nesneye bağlı yazının bağı (`labelOf`, `labelScale`), kaydedilmeyen nesneyi izleyen taramanın ilişkisi
  (`assoc`) düşer; tablonun kaynağının kaydedilmeyen nesneleri çıkar, nesnesi kalmayan kaynak düşer.
- Adı dosyanın adıdır. Seçim yoksa komut seçim ister (söyler).

### 2. Başka çizimden al

Komut `file.takeFrom` (“Başka çizimden al…”; Dosya, CAD'de Ekle › Alışveriş, CBS'de Veri › Dosya alışverişi): bir `.kcad` seçilir; pencere
onun alınabileceklerini bölüm bölüm işaret kutularıyla gösterir: **Katmanlar** (yollarıyla), **Bloklar**, **Yazı stilleri**, **Ölçü
stilleri**, **Kitaplık** (semboller, görüntüler, şablonlar), **Katman durumları**, **Proje ayarları** (birimler ve ondalıklar, açı
birimi, çizim ölçeği, çizim yazı tipi, ölçme ayarları; koordinat sistemi alınmaz, pencere söyler). Proje ayarları dışında hepsi işaretli
başlar (projenin birimlerini değiştirmek bilerek yapılır); bu çizimde aynı adı taşıyanlar satırda “bu çizimde var” diye belirtilir. **Aynı
adlı olanlar:** Atla (varsayılan) ya da Değiştir.

- Sıra: proje ayarları, katmanlar, yazı ve ölçü stilleri, kitaplık, bloklar, katman durumları; her bölümde öbür çizimin sırasıyla.
- Yeni düğümün, bloğun, stilin, öğenin ve katman durumunun kimliği öbür çizimdekidir; bu çizimde varsa düğümün ve katman durumunun
  kimliği `-2`, `-3` … ekiyle boş olana çıkar, blok ve stil kimliği (UUID) yeniden üretilir.
- Katmanlar yollarıyla (gruplar ve ad; adlar Türkçe harfler katlanıp büyük küçük harf ayrılmadan, baştaki ve sondaki boşluklar atılarak
  karşılaştırılır): olmayan yol, eksik gruplarıyla açılır (öbür çizimdeki görünürlük, kilit, stil ve katmanın kendi keneti ile, ebeveyninin
  sonuna); olan katmanda Değiştir stilini (bütün `style`'ı) alır; aynı yolda türü başka (grup ya da katman) olan düğüm hep atlanır.
- Bloklar adıyla (blokların ad kuralı, ADR 0144): olmayan eklenir (yerleştirdiği bloklar da, aynı kuralla); olanında Değiştir tanımını
  (nesneleri, taban noktası ve açıklaması) yeniden tanımlar, kimliği kalır, yerleştirmeler onu izler; Atla'da öbür çizimin o bloğa
  yerleştirmeleri bizimkine bağlanır. Blok nesnelerinin katmanı aynı yollu katmanımız, yoksa bloğunki (boş); yazı ve ölçü stilleri adıyla
  bizimkine, bizde yoksa alan düşer (nesne kendi görünüşüyle kalır); sembolleri bizde yoksa sembol ve görüntüleri de alınır.
- Yazı ve ölçü stilleri adıyla (büyük küçük harf ayrımsız): olmayan eklenir; olanında Değiştir değerlerini alır (kimliğimiz kalır,
  stildeki nesneler izler; ADR 0183).
- Kitaplık öğeleri kimliğiyle: olmayan sona eklenir; olanında Değiştir öğenin yerine geçer. Sembolle birlikte kullandığı görüntüler
  (`asset`) de alınır, aynı kuralla.
- Katman durumları adıyla: girdileri katman yollarıyla bizim katmanlarımıza; karşılığı olmayan girdi düşer.
- Katmanlar ve bloklar tek geri alma adımıdır (“Başka çizimden al”); stiller, kitaplık, katman durumları ve proje ayarları ayarlardır, geri
  alma adımı değildir (ADR 0092, 0183 §1); ileti ikisini ayrı söyler. Bloklar, yerleştirdikleri bloklar çizimde olunca eklenir ya da yeniden
  tanımlanır (bir tanım yalnız çizimdeki blokları yerleştirebilir, ADR 0144); bir blok kuralı reddederse hiçbir şey alınmaz.
- Katman ağacını değiştirme yetkisi olmayan veritabanı projesinde (ADR 0078) ağaca düğüm ekleyen ya da düğümün stilini değiştiren alma
  reddedilir, nedeni söylenir; yalnız stilleri, kitaplığı ve blokları alan iş yapılır.

### 3. Dosyadan blok ekle

Komut `block.insertFile` (“Dosyadan blok ekle…”; Blok panelinde): bir `.kcad` seçilir; çizimin bütün nesneleri bir blok tanımı olur ve
Blok ekle bu blokla başlar (yerleştirme, ölçek, dönüş, aynalama Blok ekle'nin).

- Ad dosyanın adıdır; çizimde varsa blokların ad kuralıyla sayı alır (`Ad (2)`); dosyanın blokları da (iç içe olanlar, yalnız
  yerleştirilenler) aynı kuralla.
- Taban noktası alınan nesnelerin kapsamının (Tümünü göster'in ölçtüğü) sol alt köşesidir; nesneler yerlerinde kalır.
- Dosyanın blokları gelir (adları çizimde varsa sayı alarak), yazı ve ölçü stilleri adıyla birleşir (içe aktarmanın kuralı, ADR 0183 §7),
  nesnelerin kullandığı kitaplık öğeleri eklenir (kimliği olan atlanır).
- Nesnelerin katmanı aynı yollu katmanımızdır; yoksa yol, gruplarıyla, aynı adımda açılır (AutoCAD'in yaptığı gibi; kimlikler §2'nin
  kuralıyla). Bağlı yazının bağı ve taramanın ilişkisi düşer (blok nesnesinin kalıcı kimliği yoktur).
- Blok resim ve tablo almaz (ADR 0184, 0192): resimler ve tablolar alınmaz, sayılarıyla söylenir. Boş çizim söylenir.
- Tanım ve açılan katmanlar tek adımdır (“Dosyadan blok ekle”); yerleştirme Blok ekle'nin kendi adımıdır. Katman açması gereken iş,
  ağacı değiştirme yetkisi olmayan veritabanı projesinde reddedilir (§2).

### 4. Ortak kurallar

Üç işin kuralları belge düzeyindedir (çizimlerin anlık görüntüleri üzerinde, hesap yok): masaüstünde `kentos_domain::exchange`, web'de
`model/exchange.ts`; ortak durumlar `fixtures/exchange/v1/cases.json`, bağımsız Python başvurusu `scripts/fixtures/exchange_cases.py`.
Kapsamın sol alt köşesi çekirdeğin sınırlarıdır. Dosyalar okunurken `.kcad`'in kuralları (ADR 0025) uygulanır; okunamayan dosya nedeniyle
söylenir. Öbür çizimin koordinat sistemi başkaysa bu söylenir (dönüşüm yoktur; Veri karşılaştır'ın kuralı, ADR 0179).

## Uygulama

- **Kurallar:** masaüstünde `kentos_domain::exchange` (`selection`, `take`, `file_block`; sözleşmenin JSON'u üzerinde `selection_json`,
  `take_json`, `file_block_json`), web'de `model/exchange.ts` (`selectionDrawing`, `takeFrom`, `fileBlock`); ikisi aynı adımları aynı sırayla
  yapar. Yeni katmanın kendi keneti için masaüstünün `NewLayer`'ına `snap` eklendi (web'in `LayerInit`'i taşıyordu).
- **Uygulama:** masaüstünde `apps/desktop/src/drawing_exchange.rs` (dosya seçimi, Başka çizimden al penceresi, sonuçların çizime yazılması;
  seçimin dosyası `saving::write_watched` ile kendi iş parçacığında yazılır, geri okunup doğrulanır), web'de `app/drawingExchange.ts` ve
  `ui/io/TakeFromDialog.ts` (dosya `.kcad` kodeğiyle biçim işçisinde yazılır).
- **Komutlar:** `file.saveSelection` (Seçilenleri dosyaya kaydet…; takma adlar `WBLOCK`, `BLOKYAZ` …), `file.takeFrom` (Başka çizimden al…;
  `ADCENTER` …), `block.insertFile` (Dosyadan blok ekle…; `INSERTFILE` …). Dosya menüsünde; CAD şeridinde Ekle › Alışveriş (ilk ikisi) ve
  Blok paneli (üçüncüsü), CBS şeridinde Veri › Dosya alışverişi. İkonları sahip yokken önerilen seçeneklerdir.
- İz oynatıcılarında dosya `openFile` ile verilir; web'in oynatıcısı dosya okuyan komutun aracını bekler.

## Doğrulama

- Ortak durumlar `fixtures/exchange/v1/cases.json` (`scripts/fixtures/exchange_cases.py`, KentOS kodu olmadan yazılmış bağımsız başvuru;
  `--check`): üç seçim (iç içe blok, katman ağacının budanması, kitaplığın kullanılanları, bağların düşmesi, tablonun kaynağı), üç alma
  (Atla, Değiştir, yalnız birkaç seçim; kimliklerin `-2` eki, katlanan adlar, türü başka düğüm, katman durumunun yolları), dosyadan blok
  (ad sayısı, iç içe bloklar, sol alt köşe, resim ve tablo sayısı). Masaüstü `crates/native/domain/tests/all/exchange.rs`, web
  `model/exchange.wasm.test.ts` aynı durumları geçer.
- Ortak izler `take-from.json` ve `block-insert-file.json` (`exchange.kcad`, `altyapi-projesi.kcad`) iki platformda; resimler `kentos-cad
  kullan take-from`, `kentos-cad kullan block-insert-file` ve web'in `e2e:use`'u.
- Seçimin dosyası masaüstünde yazılıp geri açılarak denetlenir (`drawing_exchange` testleri).

## Kapsam dışı

Dış başvuru (xref: bağlı, güncellenen çizim), çoklu belge sekmeleri (`DOM-08`), nesnelerin başka çizimden alınması (panonun ve içe
aktarmanın işi), DXF'ten blok ekleme, koordinat sisteminin alınması ve dönüşüm, katman tanım dosyası (`.kstil`'in işi).
