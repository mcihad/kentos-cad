# ADR 0223: Tablo alışverişi: XLSX ve ODS, Tablodan nokta, tabloları dışa aktarma

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez, biçim sözleşmesi `FORMATS_VERSION` 50; ikonlar sorulmadan seçilir; ilke
  “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-27` (“Excel (XLSX) ve ODS okuma ve yazma, XY sütunlu tablodan nokta katmanı, öznitelik tablosunu
  Excel'e verme”); ADR 0184 §4 (Tablo ekle'nin dosya okuyucusu: CSV, TXT, XLSX), ADR 0200 §7 (İşlemler'in dosya parametresi, tablo
  çıktısı), ADR 0218 §5–§7 (tablolardan çizgi ve elips), ADR 0199 §4 (Öznitelik tablosu), ADR 0167 ve 0168 (dönüşüm, datum seçimleri),
  ADR 0180 (sayının okunuşu), ADR 0048 (koordinat listesi), ADR 0222 §2 (dönüşüm açıkça sorulur).
- **Araştırma:** ECMA-376 (SpreadsheetML: paylaşılan dizgiler, `cellXfs`, yerleşik sayı biçimleri, `_xHHHH_` kaçışı, 1900 ve 1904
  tarih sistemleri, donmuş bölme, `_xlnm._FilterDatabase`); OpenDocument 1.3'ün hesap tablosu (tekrar sayıları, `covered-table-cell`,
  `office:value-type`, `text:s`, `table:database-range`, `settings.xml`'in `HorizontalSplitMode`'u); openpyxl'in `is_date_format`'ı ve
  tarih çevirisi; LibreOffice Calc'ın ODS okuyuşu (paragrafın boşlukları ODF'in belge kuralıyla katlanmaz; başsız LibreOffice görünüm
  ayarlarını okumaz); pandas'ın `odf` okuyucusu; Excel'in sınırları (1 048 576 satır, 16 384 sütun, 15 anlamlı basamak, sayfa adının
  31 karakteri ve yasak işaretleri); ArcGIS'in Excel To Table, Table To Excel, XY Table To Point'i; QGIS'in Export to spreadsheet ve
  Create points layer from table'ı; Netcad'in Excel Dosyası Yükle'si.

## Bağlam

KentOS XLSX'i yalnız Tablo ekle ve İşlemler'in dosya parametresi için okuyordu: ODS'yi okumuyor, tarih biçimli hücreyi gün sayısı olarak
veriyor, bütün sayfayı roxmltree ağacı olarak kuruyordu (çeyrek milyon satır yüz megabaytlık XML, ağacı bunun on katı). İşlemler'in
dosya parametresi Tablo ekle'nin 10 000 satırlık sınırıyla okuyor ve uzun dosyayı **sessizce kısaltıyordu**. Hiçbir tablo XLSX ya da
ODS olarak yazılmıyordu: Öznitelik tablosu dışa aktarılamıyor, İşlemler'in tabloları yalnız CSV olarak kaydediliyordu. Koordinatlı bir
tablodan nokta yazmanın yolu yalnız metin koordinat listesiydi (ADR 0048): Excel'deki nokta listesi, boylam ve enlem, katmanın
özniteliklerindeki koordinatlar alınamıyordu.

## Karar

### 1. Okuma

`formats::table_file` (Tablo ekle, Tabloyu güncelle ve İşlemler'in dosya parametresi) üç tür dosya okur: metin tablosu (CSV, TXT;
ADR 0184 §4'ün kuralı değişmez), Excel çalışma kitabı (XLSX) ve OpenDocument hesap tablosu (ODS). Hücre metindir; hiçbir şey
hesaplanmaz.

- **Akış:** çalışma kitabının büyük parçaları (sayfalar, paylaşılan dizgiler, `content.xml`) yeni `formats::xml_scan` ile etiket etiket
  okunur: yalnız açık öğeler ve kapsamdaki ad alanları tutulur, ad alanları URI'leriyle eşlenir, okuyucu sınıra varınca durur. XML'in
  kuralları korunur (beş varlık ve karakter başvuruları, CR LF'nin LF'si, CDATA, belge türü bildirimi reddi, kapanmayan öğe, ikinci kök,
  bildirilmemiş önek, yinelenen öznitelik, `DEPTH`'ten derin öğe). Küçük parçalar (çalışma kitabı, ilişkileri, biçemler) ağaç olarak.
- **XLSX:** sayfalar `xl/workbook.xml`'in sırasıyla, adlarıyla, ilişkilerinden (göreli ya da kökten); hücre başvurusunun yerinde, yoksa
  satırındaki öncekinin ardında. Sözcükleri: paylaşılan ya da kendi dizgisi (fonetik `rPh` dışında), formülün dizgi sonucu, `_xHHHH_`
  kaçışları adlandırdıkları karakter olarak (`_x005F_` bir alt çizgi); evet/hayır DOĞRU ya da YANLIŞ; **tarih ya da saat biçimli sayı
  ISO metniyle**: biçimin tarih olup olmadığı openpyxl'in kuralıyla (ilk bölüm, tırnaklı metin ve köşeli parçalar dışında; `[h]` gibi
  geçen süre biçimi sayı kalır), yerleşik biçimler 14–22 ve 45–47; Excel'in 1900 sistemi (60'ın altında bir gün sonra: Excel'in
  olmayan 29 Şubat 1900'ü) ya da çalışma kitabının `date1904`'ü; tarih YYYY-AA-GG, saat SS:DD:ss, ikisi YYYY-AA-GGTSS:DD:ss (saniyeye
  yuvarlanır; 23:59:59,5 ertesi günün 00:00:00'ı); eksi ya da 9999-12-31'den sonraki değer yazıldığı gibi.
- **ODS:** `content.xml`'in tabloları sırasıyla, adlarıyla. Satır ve hücre tekrar sayıları: boş olan yalnız yeri ilerletir (bir milyon kez
  tekrarlanan son satır bir şey tutmaz), dolu olan sınırlar içinde o kadar yazılır; başlık satırları, satır grupları ve listeleri olağan
  satırdır; örtülü hücre (`covered-table-cell`) boştur. Sözcükleri `office:value-type`'a göre: float, percentage, currency
  `office:value` yazıldığı gibi; date `office:date-value` (saniyesi yuvarlanır); time `PT12H30M00S` → 12:30:00; boolean DOĞRU ya da
  YANLIŞ; string ya da türsüz hücre `office:string-value`, yoksa paragraflarının metni (satır başına bir paragraf; boşlukları LibreOffice
  Calc'ın okuduğu gibi yazıldığı gibi; `text:s` sayısınca boşluk, en çok 10 000; `text:tab`, `text:line-break`; notlar, çizimler ve alt
  tablolar dışarıda).
- **Sınırlar** (`read_with(bytes, Limits)`): Tablo ekle 10 001 satır ve 101 sütun (tablonun en çoğunun bir fazlası, kuralı nedenini
  söylesin diye); İşlemler'in dosya parametresi **250 000 satır ve 500 sütun**, sütun adlarının satırı dışında. Sınırı aşan sayfa
  bırakılır ve nedeni söylenir (“… sayfası 250 000 satırdan uzun; İşlemler en çok 250 000 satır alır.”), dosyanın hiçbir sayfası sığmazsa
  dosya reddedilir; **sessiz kısaltma yoktur**. 64 MB'tan büyük dosya, eski Excel (.xls, OLE2), çalışma kitabı olmayan arşiv ve hesap
  tablosu olmayan OpenDocument reddedilir; hiçbir girdide çökme olmaz.
- **İşlemler'in dosya alanı:** dosya seçilince ayrı iş parçacığında (masaüstünde engelleyici görevde, web'de biçim işçisinde) okunur.
  Birden çok sayfalı çalışma kitabında **Sayfa** listesi çıkar (ilk sayfa seçili); dosyanın değeri sayfaların satırlarını ve
  bırakılanları taşır, son değerler (`islemler.json`, `kentos.processing.v1`) yalnız adı saklar. Dosya alanı olan araçlar (Anahtarla
  birleştir, Veri aktar, XY'den çizgi, Semt ve uzunluktan çizgi, Tablodan elips, Tablodan nokta) ve Tablo ekle ODS'yi de kabul eder.

### 2. Yazma

`formats::workbook::write` sayfaları (ad, adlı ve türlü sütunlar, sözcük satırları) XLSX, ODS ya da CSV olarak yazar. Sözcük sütununun
türüyle yazılır; boş sözcük boş hücredir:

- **number:** en çok 15 anlamlı basamaklı ondalık sayı metni (işaret, rakamlar, bir nokta ve rakamlar) sayı; sütunun basamağı
  verilmişse o kadar basamak gösterilir (0, 0.00 …); öbürü metin.
- **date:** takvimin bir günü (YYYY-AA-GG) tarih; XLSX'te 1 Mart 1900'den (Excel'in gün sayımı o günlerde bir gün kayık), öncesi metin;
  gösterimi okuyanın yerel tarih biçimi (Türkçede GG.AA.YYYY).
- **boolean:** true ve false evet/hayır; öbürü metin.
- **auto** (türü olmayan sütun, İşlemler'in tabloları): düz sayı (baştaki sıfırı, noktadan sonraki sondaki sıfırı olmayan, -0
  olmayan, en çok 15 anlamlı basamaklı) sayı, öbürü metin; “007” gibi bir kod metin kalır.
- **text:** metin. XLSX'te paylaşılan dizgi, XML'de geçmeyen karakterler ve metnin kendi `_xHHHH_`'si Excel'in kaçışıyla, sözcükler
  yazıldığı gibi geri gelir; ODS'de satırlar ayrı paragraf, boşluk dizileri `text:s`, sekme `text:tab`, CR LF ve CR LF olur, denetim
  karakterleri yazılmaz (ODF tutamaz).

Sütununun dediğinden başka yazılan sözcükler (uzun sayı, sayı olmayan, tarih olmayan, erken tarih, evet/hayır olmayan, ODS'nin
yazamadığı denetim karakteri ve CR) raporda sayılarıyla söylenir. Sayfa adı Excel'in kuralıyla iki biçimde de: 1–31 karakter,
`: \ / ? * [ ]` ve denetim karakteri yerine `_`, uçlarındaki kesme işaretleri yok, boş ad `SayfaN`, `History` `History_`, büyük
küçük harfe bakmadan alınmış ad `(2)`, `(3)` …; değişen ad söylenir. Sütun adları satırı kalın ve donmuş, tablonun üstünde süzgeç
düğmeleri (XLSX `autoFilter` ve `_xlnm._FilterDatabase`, ODS `table:database-range`); sütun genişliği en geniş sözcüğünün (çok satırlıda
en uzun satırının) ve adının 3 fazlasının 2 fazlası, 8 ile 60 karakter arası. XLSX ve ODS sayfası en çok 1 048 576 satır (adlar
dahil) ve 16 384 sütun alır; büyüğü reddedilir ve CSV önerilir. Aynı girdi aynı baytları verir (dosyalar 1 Ocak 1980 tarihli).

- **XLSX:** içerik türleri, ilişkiler, çalışma kitabı, biçemler (kalın yazı, tarih biçimi 14, basamaklar `0.00…`), paylaşılan dizgiler,
  sayfa başına bir çalışma sayfası; belge özellikleri yazılmaz.
- **ODS:** `mimetype` ilk ve sıkıştırmasız (`zip::Pack::Stored`), `META-INF/manifest.xml`, `content.xml` (hücre ve sütun biçemleri,
  tablolar, süzgeç aralıkları), `styles.xml`, `settings.xml` (donmuş satır). Sayının hücresi değerini ve metnini, tarih ISO değerini ve
  GG.AA.YYYY metnini, evet/hayır DOĞRU ya da YANLIŞ metnini taşır (LibreOffice'in yazdığı gibi; yalnız değeri okuyanlar da metni
  okuyanlar da aynı şeyi görür).
- **CSV:** Katman listesi'ninki (Excel'in Türkçesi): bayt sırası işareti, noktalı virgülle ayrılmış alanlar (noktalı virgül, tırnak ya
  da satır sonu taşıyan alan tırnaklı), CR LF; her sözcük yazıldığı gibi.
- **Sıkıştırma:** sayfalar, paylaşılan dizgiler ve `content.xml` hafif düzeyde (`zip::Pack::Fast`, düzey 3), küçük parçalar olağan
  düzeyde (6) sıkıştırılır: 36 MB'lık bir sayfa 811 ms yerine 235 ms'de, %10 kadar büyük paketlenir (§8).

### 3. Öznitelik tablosunu dışa aktar

Alt panelin Tablo sekmesinde **Dışa aktar ▾** (Excel çalışma kitabı (.xlsx), OpenDocument hesap tablosu (.ods), CSV (.csv)): görünen
satırlar, görünen sırayla (Ara, Göster, İfade süzgeci, sıralama). Sayfanın adı katmanın adı; sütunları Tür (metin), katmanın alanları
**adlarıyla** (takma adlarıyla değil, geri okunabilsin diye) ve alan adı olmayan anahtarlar doğal sırayla (metin). Alanın sütunu türüyle
yazılır: tam sayı basamaksız sayı, ondalık sayı alanın basamağıyla (verilmemişse yazıldığı gibi), tarih tarih, evet/hayır evet/hayır,
metin metin; değer listeli alan metin (etiketleri). Hücre: Tür tablonun adlandırdığı tür; alanın kurallarına uyan değer kanonik metni
(değer listesinin etiketi), uymayan değer yazıldığı gibi, alan adı olmayan anahtar yazıldığı gibi. Kural iki platformda tektir:
masaüstünde `kentos_interaction::feature_table::workbook_sheet`, web'de `model/featureTable.ts`'in `featureSheet`'i.

### 4. İşlemler'in tabloları

Sonuç tablosunun “CSV olarak kaydet”i **Kaydet ▾** olur (aynı üç biçim): sayfa aracın adıyla, sütunlar türsüz (`auto`): düz sayı sayı,
öbürü metin.

### 5. Tablodan nokta

İşlemler'in Dönüştürme kategorisinde **Tablodan nokta** (`convert.tablePoints`; takma adlar TABLODANNOKTA, XYNOKTA, EXCELNOKTA; CBS'de
Veri › Toplu çevir ▾'da tablolardan çizgi ve elipsten sonra):

- **Kaynak:** dosya (CSV, TXT, XLSX, ODS; ilk satırı sütun adları, çalışma kitabında seçilen sayfa) ya da nesneler (öznitelikleri
  satırdır; ADR 0218 §5'in kuralı). Kaynakta olmayan sütun çalıştırmayı reddeder: “Kaynakta “Kot” alanı yok; alanları: Ad, Y, X, Z, Kod.”
- **Sütunlar:** Doğu ve Kuzey zorunlu (coğrafi sistemde boylam ve enlem), Kot ve Ad isteğe bağlı.
- **Koordinat sistemi:** Projenin sistemi (varsayılan) ya da kayıttaki bir sistem (yerel olanlar dışında). Projeninkinden başkaysa
  noktalar projenin datum seçimleriyle `crs::transform_in` ile projenin sistemine çevrilir (ADR 0222 §2: dönüşüm açıkça seçilir);
  kot olduğu gibi kalır. Projenin sistemi yoksa çevirme reddedilir, nedeni ve yolu söylenir.
- **Satırın okunuşu** (çekirdeğin `ops::convert::table::point_rows`'u): sayı ADR 0180'in kuralıyla (işaret, rakamlar, bir nokta ya da
  virgül); coğrafi sistemde ayrıca derece, dakika ve saniye (`crs::parse_angle`: boşluk ya da ° ′ ″, sonda yarıküre harfi K, G, D, B ya
  da N, S, E, W), boylam ±180 ve enlem ±90 içinde. Boş, sayı olmayan, sınır dışı ya da dönüşümün ulaşamadığı hücreli satır atlanır;
  sonuç tablosu Satır ve Neden (“Y” boş, “X” sayı değil: kuzey, “Boylam” coğrafi sınırın dışında (boylam ±180, enlem ±90): 190,
  Projenin sistemine çevrilemedi (izdüşümün dışında; datumlar arasında yol yok; datum seçiminin ızgarası bu cihazda yok; ızgaranın
  dışında)), uyarı “K satır atlandı; nedenleri tabloda.”
- **Sonuç:** yeni katmanda (Noktalar, dönüştürme araçlarının görünüşü) satırların sırasıyla noktalar; satırın dolu hücreleri sütun
  adlarıyla öznitelik, Ad sütunu verilmişse Ad özniteliği onun değeri (nokta `#ad` ile bulunur); tek geri alma adımı “Tablodan nokta”;
  “N satırdan M nokta yazıldı.”

### 6. Koordinat listesi ve çalışma kitabı

Koordinat listesi içe aktar (ve Noktalar'ın İçe aktar'ı) metin dosyası okur. ZIP arşivi (XLSX, ODS) ya da eski Excel dosyası seçilince
(`coords::read` ilk baytlarından tanır) hiçbir satır okunmaz, pencere kodlamayı “metin değil” diye gösterir ve yalnız şunu söyler:
“Bu dosya metin değil: bir çalışma kitabı (Excel, ODS) ya da arşiv. Koordinat listesi NCN, TXT ya da CSV dosyasıdır; Excel ya da ODS
tablosundaki noktaları İşlemler › Tablodan nokta ile alın.” Satırı olmayan dosyada genel “ayırıcıyı ve sütunları denetleyin” uyarısı
artık çıkmaz (nedeni ipucundadır). Metin “PK” ile başlasa da olağan okunur.

### 7. Simgeler

Sorulmadan seçildi: `exportXlsx` dışa aktarılan dosya sayfasında X, `exportOds` dosya sayfasında O, `exportCsv` dosya sayfasında
ayırıcılar arasında değerler; `tablePoints` küçük bir tablo ve köşesinden çıkan üç dolu nokta (satırlar noktaya dönüşür).

### 8. Başarım

Okuma akıştır (§1): çeyrek milyon satırlık sayfa ağaç olmadan, sınıra varınca durarak okunur; tarayıcının biçim işçisinde de bellek
sayfanın kendisiyle sınırlıdır. Tarayıcının etiket ayrımı baytın ardındaki işaretle yapılır, bitiş etiketi bayt aramasıyla bulunur,
öneklerin çözümleri bağlamalar değişene dek küçük bir önbellektedir (`resolved`); XLSX hücresinin türü küçük bir sayımdır. Yazma
hücreleri parça parça ekler (sözcük karakter karakter değil, kaçacak bayta dek dilim dilim; sayılar biçimlendirme makinesi olmadan;
sütunun biçemi bir kez); büyük parçalar hafif düzeyde sıkıştırılır. Okuma ve yazma masaüstünde ayrı iş parçacığında, web'de biçim
işçisinde; dosya diyalogu yazmayı beklemez.

## Uygulama

- **Çekirdek** `kentos-formats`: `xml_scan.rs` (yeni), `table_file/` (`mod.rs`: `Limits`, `read_with`, `Grid`; `xlsx.rs`; `ods.rs`;
  `dates.rs`), `workbook/` (`mod.rs`: hücre kuralı, sayfa adları, genişlikler, CSV; `xlsx.rs`; `ods.rs`), `zip.rs`'in `Pack` ve
  `write_with`'i, `coords.rs`'in `NOT_TEXT`'i. Geometri çekirdeğinde `ops::convert::table::point_rows`, `RowFault::OutOfRange` ve
  `Unreached`, işlem `convertPointRows`. Sözleşme `WorkbookWriteInput`, `WorkbookFormat`, `WorkbookSheet`, `WorkbookColumn`,
  `CellKind`; `FORMATS_VERSION` 50.
- **WASM** `formats-wasm`: `readTableFile(bytes, processing)`, `writeWorkbook(input)`.
- **İşlemler:** masaüstü `kentos-processing`'in `builtin/convert/tables.rs`'i (`table_points`), dosya parametresinin değeri
  (`parameters.rs`'in `stored_value`'su sayfaları ve satırları saklamaz), ODS kabul eden araçlar; web `processing/builtin/convert/`
  (`tables.ts`, `shared.ts`), `model/ops/convert.ts`, `processing/types.ts`'in `FileValue`'su (`sheet`, `sheets`, `leftOut`),
  `processing/parameters.ts`.
- **Masaüstü:** `processing/dialog.rs`'in `file_value`'su, `processing/mod.rs`'in `FileRead`, `Sheet` ve `SaveTableAs` olayları,
  `processing/fields.rs`'in Sayfa listesi, `processing/window.rs`'in Kaydet ▾'si; `table_write.rs` (biçimler, menü, ayrı iş
  parçacığında yazma, `Message::TableWritten`); `features/`'in Dışa aktar ▾'si; `tables/`'ın ODS'si; `exchange/coord_import.rs`'in
  özeti; komut `catalog.rs` ve `ported.json`'da; testler ve resimler `processing/table_points_tests.rs`, `exchange/tests.rs`.
- **Web:** `ui/processing/paramFields.ts`'in Sayfa listesi, `ToolDialog.ts`'in Kaydet ▾'si ve bırakılan sayfa uyarıları,
  `app/processing.ts`'in `fileValue`'su; `ui/bottom/featureExport.ts` (biçimler, menü, yazma), `ui/bottom/FeatureTable.ts`'in Dışa
  aktar ▾'si, `model/featureTable.ts`'in `featureSheet`'i; `ui/table/TableInsertDialog.ts`, `app/tables.ts`; `ui/io/CoordImportDialog.ts`'in
  özeti; `io/protocol.ts`, `io/client.ts`, `io/formatsWorker.ts`; simgeler `ui/icons.ts`; şerit `app/ribbon.ts`'in `CONVERT_UNDER`'ı;
  resimler `shots.mjs tableexchange`.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `scripts/fixtures/table_file_cases.py` (`fixtures/table/v1/files`, 17 dosya, 21 durum): metin tabloları, zipfile ve elle yazılmış XML'le kurulan
    XLSX'ler (tarihler, 1904 sistemi, kaçışlar, ilişkiler) ve ODS (tekrarlar, örtülü hücreler, türler, paragraflar), LibreOffice'in
    kendi yazdığı `libre.xlsx` ve `libre.ods`, eski Excel başı, başka arşiv, OpenDocument metni; kural ADR'den, Python'un zipfile ve
    xml.etree'siyle. `--check` her XLSX'i openpyxl'le, her ODS'yi LibreOffice'le (XLSX'e çevrilip openpyxl'le) okuyup kuralınkiyle
    karşılaştırır: **0 fark** (bilinen farklar nedenleriyle listede: LibreOffice'in Mart 1900 öncesi gün sayımı, süreyi günün saati
    olarak yazması, değeri olmayan sayıyı 0 okuması, 9999'dan sonraki tarihler). Sınırlar ayrıca (`cok-satir.csv`, İşlemler'in sınırı).
  - `scripts/fixtures/workbook_cases.py` (`fixtures/table/v1/written`, 5 durum): yazıcının dosyaları tablo okuyucusunun kuralıyla geri
    okunur; her XLSX openpyxl'le (hücre türleri ve değerleri, kalın ve donmuş başlık, süzgeç, sayı biçimleri, sütun genişlikleri), her
    ODS LibreOffice'le açılır (donmuş satır başsız LibreOffice görünüm ayarlarını okumadığından denetlenmez); CSV betiğin kendi
    yazışıyla. Yazıcının baytları iki platformda kaydedilenlerle aynı (`KENTOS_WRITE_WORKBOOK=1` yeniden yazar); sıkıştırma düzeyi
    değişince kaydedilenler yeniden yazıldı, içlerindeki XML bayt bayt aynı kaldı.
  - `scripts/fixtures/table_points_cases.py` (`fixtures/processing/v1/table-points.json`, `.kcad`, `nokta-duzlem.csv`,
    `nokta-wgs84.ods`; 4 durum): düzlem tablosu, iki sayfalı ODS'de WGS 84 boylam ve enlemi (ondalık, virgüllü, DMS, yarıküre harfi,
    sınır dışı, sayı olmayan), katmanın öznitelikleri, olmayan sütun; noktalar pyproj'la (EPSG:4326 → EPSG:32636).
  - `scripts/fixtures/feature_export_cases.py` (`fixtures/feature-table/v1/export.json`, 2 durum): Dışa aktar'ın sayfası katman
    alanlarının kuralıyla (`layer_field_cases.py`).
  - `convert_cases.py` ADR 0218'in durumlarını değişmeden geçer.
- **Ortak durumlar** iki platformda: Tablodan nokta masaüstünde çekirdek, işlem ve pencere düzeyinde (`the_table_point_cases_do_what_they_say`,
  `processing::table_points_tests`: dosya pencerenin okuduğu gibi, ODS'nin ikinci sayfası seçilince o okunur), web'de istemci ve işçide
  (`processing/cases.test.ts`); tablo dosyaları ve yazılan çalışma kitapları masaüstünde `tests/all/table_file.rs`, `workbook.rs`, web'de
  `io/formats.wasm.test.ts`; Dışa aktar'ın sayfası `tests/all/feature_export.rs` ve `model/featureExport.test.ts`; koordinat listesinin
  reddi `coords::a_workbook_is_no_coordinate_list`, `exchange::tests::a_workbook_in_the_coordinate_list_window_points_to_tablodan_nokta`
  ve web'in biçim testi; pencerenin ortak dosyası `fixtures/processing/v1/dialog.json` yeniden kaydedildi (Kaydet, ODS, yeni araç).
- **Süreler** (release, bu makine; masaüstü `cargo test --release -p kentos-formats --test all workbook_timing -- --ignored
  --nocapture` ve `cargo test --release -p kentos-processing --test cases table_points_timing -- --ignored --nocapture`; web
  `WORKBOOK_BENCH=1 … scripts/perf/workbook.test.ts` ve `CONVERT_BENCH=1 … scripts/perf/convert.test.ts`, gönderilen WASM'la; yazma ve
  okuma üç koşunun en iyisi, Tablodan nokta bütün çalıştırmanın p50'si):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | XLSX yaz (100 000 satır, 10 sütun; 5,8 MB) | 452 ms | 588 ms | 700 / 900 ms |
  | ODS yaz (4,9 MB) | 654 ms | 781 ms | 900 / 1100 ms |
  | CSV yaz (8,2 MB) | 49 ms | 119 ms | 150 / 250 ms |
  | XLSX oku (250 000 satır, 10 sütun; 14,5 MB; İşlemler'in sınırıyla) | 1476 ms | 1503 ms | 2000 / 2200 ms |
  | ODS oku (12,3 MB) | 1580 ms | 2171 ms | 2200 / 3000 ms |
  | Tablodan nokta, 100 000 satır, projenin düzleminde | 240 ms | 513 ms | 300 / 800 ms |
  | Tablodan nokta, 100 000 satır, WGS 84'ten projenin sistemine | 283 ms | 545 ms | 400 / 900 ms |

  Web'in süreleri işçinin yaptığı gibi girdinin ve sonucun JSON'unu da içerir. İlk ölçümde XLSX yazma 1093 ms, ODS 944 ms sürdü: sürenin
  çoğu düzey 6'daki sıkıştırma (36 MB'lık sayfada 811 ms) ve hücre başına ayırmalardı (ODS'de her satır için karakter dizisi, her metin
  için iki kopya); hafif düzey ve dilim dilim yazma ikisini de indirdi. Okumada 250 000 satırlık XLSX'in 93 MB'lık sayfası 281 ms'de açılır,
  etiket taraması 1,3 kez hızlandı (847 ms'den 736 ms'ye); ODS'nin 298 MB'lık `content.xml`'inin açılması 714 ms'dir.
- **Resimler:** masaüstü `.run/shots/islem-tablo-*` (28: şerit, Tablodan nokta'nın penceresi, Sayfa listesi, Kaydet ▾, noktalar
  çizimde, Öznitelik tablosunun Dışa aktar ▾'si, koordinat listesinde çalışma kitabı), web `apps/web/scripts/e2e/out/shots/tableexchange`
  (28); 1440 × 900 ve 1100 × 650'de, iki temada.

## Sonuçlar

XLSX ve ODS iki yönde; İşlemler'in dosyaları 250 000 satıra kadar ve sınırı aşan söylenerek; tablolar ve Öznitelik tablosu türleriyle
dışa aktarılır; koordinatlı her tablodan (dosya ya da katman, düzlem ya da coğrafi) nokta yazılır. Kapsam dışı: eski Excel (.xls) ve
XLSB; formüllerin hesaplanması (kaydedilmiş sonuçları okunur); birleştirilmiş hücrelerin yayılması (yalnız sol üst hücre değer taşır);
çok sayfalı dışa aktarma (yazıcı birden çok sayfa alır, arayüz bir sayfa yazar); şifreli çalışma kitapları; Excel'e canlı bağlantı.
