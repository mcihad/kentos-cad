# ADR 0184: Tablo

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-21`'in ardından `CAD-22`; sahibin 5 Ekim
  kararıyla madde tek parçada biter. Sahibin 6 Ekim eki: “tablolarda çerçeve de olabilmelidir” (Kalın çerçeve, §2). Ayrıntılar bu
  ADR'nin varsayılanlarıdır. Adlar AutoCAD'in Türkçe arayüzünden (“Tablo”, “Tablo ekle”); Netcad'in Excel Dosyası Yükle ve
  Koordine Özet'i, ArcGIS'in tablo çerçevesi ve QGIS'in öznitelik tablosu öğesi örnektir.
- **Bağlam belgesi:** TODOS.md `CAD-22`; ADR 0183 (yazının yüzü ve stili), ADR 0182 (çok satırlı yazı), ADR 0144 (bloklar), ADR 0149
  (gösterim kuralı), ADR 0178 (veride arama), ADR 0025 (`.kcad` v2), ADR 0165 (proje türleri: CAD arayüzü).

## Bağlam

Çizimlerde koordinat ve alan çizelgeleri, parsel listeleri ve notlar tablo olarak durur. Bugüne dek bunlar çizgilerle ve tek tek
yazılarla elle kuruluyordu: bir değer değişince bütün tablo yeniden çizilir, satır eklemek her şeyi kaydırır, Excel'deki liste
çizime ancak yazı yazı aktarılır. DXF'te AutoCAD'in tablosu (ACAD_TABLE) karmaşık bir nesnedir; KentOS onu okumuyordu.

## Karar

### 1. Tablo bir nesnedir

Tablo (`table`), sol üst köşesinden (`p`) asılı, onunla dönen (`rotation`, derece) satırlar ve sütunlardır. Alanları:

- `height` hücrelerin yazı yüksekliği (m); `rows` satırların yüksekliği yukarıdan aşağı, `columns` sütunların genişliği soldan
  sağa (m); `cells` satır satır sözler, her biri tek satır (boş söz boş hücredir).
- `merges` birleşik alanlar (`row`, `col`, `rows`, `cols`): alanın sözü sol üst hücresindedir, öbür hücreleri boştur; alanlar
  tablonun içinde, ayrı ve birden çok hücredir.
- `aligns` sütun başına `left`, `center`, `right` (yokluğu hepsi sol); `header` ilk satır başlıktır: kalın ve ortalı.
- `grid` çizgiler: yokluğu hepsi, `outer` yalnız dış çizgi, `rows` dış çizgi ve satırlar arası, `none` hiçbiri.
- `frame` **Kalın çerçeve**: dış çizgi bu genişlikte (m), tablonun içine doğru dolu bir bant olur; `grid` `none` iken çizilmez.
- Yüz: yazınınki (`textStyle`, `font`, `bold`, `italic`, `oblique`; ADR 0183 §2). Hücreler dik çizilir; yazı tipi yoksa projeninkiyle.
- `source` satırların geldiği yer: `coordinates`, `areas`, `attributes` (nesnelerin kalıcı kimlikleri, sırasıyla) ya da `file`
  (dosyanın adı ve sayfası). Tabloyu güncelle (§6) onu izler.

Sınırlar: en çok 10 000 satır, 100 sütun, 100 000 hücre; hücre en çok 1000 harf, satır sonu ve denetim karakteri yok; boylar
sonlu, sıfırdan büyük ve en çok 10⁶ m; kaynak en az 1, en çok 100 000 nesne. Kurallar sözleşmenin `TableShape::problem`'idir;
komutlar onu `invalid_table` ile, `.kcad` okuyucusu `bad_value` ile reddeder. Tablo yalnız çizimin nesnesidir: blok tablo içermez
(`table_in_block`).

### 2. Çizim

- **Yerleşim** (çekirdek `geom::table`): tablonun eksenlerinde x satırlar boyunca, y sütunlar boyunca aşağı; nokta
  `p + u·x − v·y`. İki hücre arasındaki kenar, bir birleşik alan ikisini de tutmuyorsa çizilir; bir ızgara çizgisinde buluşan
  kenarlar tek çizgidir. Kalın çerçeveli tabloda dış çizgi yerine bandın dört dolu şeridi çizilir (üst ve alt bütün en, yanlar
  aralarında). Hücrenin sözü kenardan yarım yükseklik içeride başlar (sol), ortadadır ya da yarım yükseklik içeride biter (sağ);
  taban çizgisi hücrenin ortasının 0,35 yükseklik altındadır; birleşik alanın sözü bütün kutusundadır.
- Çizgiler ve band nesnenin rengi ve kalınlığıyla (katmana göre), sözler nesnenin renginde, tablonun yüzünde çizilir. Depo tabloyu
  çizgi (ve band varsa dolgu) kaydı ve hücre başına bir yazı kaydı (`LABEL_CELL`) olarak tutar; seçme, kenet ve kapsam ondan.
- **Tutamaçlar:** sol üst köşe tabloyu taşır; her sütunun sağ kenarı o sütunun genişliğini değiştirir (en az yüksekliğin yüzde
  biri; çerçeveyi sığdırmayan genişlik reddedilir).
- Patlat tabloyu çizgilerine, bandının dolgularına ve hücre yazılarına açar. Taşı, döndür, ölçekle tabloyu (ölçekte boylarını ve
  çerçevesini) dönüştürür.

### 3. Tablo ekle

Pencere (CAD projesinde Açıklama › Tablo; komut `table.insert`, `TABLO`, `TB`):

- **Kaynak:** Boş tablo (satır ve sütun sayısı, 1–100; sütunlar 8 yazı yüksekliği genişliğinde), Dosyadan (§4), Koordinat
  çizelgesi, Alan çizelgesi, Öznitelik tablosu. Çizelgelerin nesneleri pencere açılırken seçili olanlardır (tablolar dışında) ya da
  Sahneden seç ile seçilir; çizimin sırasıyla.
- **Görünüş:** Yazı stili (CAD projesinde; stilin yüzü ve sabit yüksekliği), Yazı yüksekliği kâğıtta mm (2,5; çizim ölçeğiyle
  metreye), Çizgiler (Tümü, Dış, Satırlar, Yok), Kalın çerçeve ve kâğıtta kalınlığı (0,7 mm). Seçimler uygulama açık kaldıkça kalır.
- **Önizleme** ilk 8 satır ve 6 sütun; özet satır ve sütun sayısıyla tablonun boyunu söyler; tek satıra indirilen hücreleri sayar.
- **Yerleştir:** tablo imleçten sol üst köşesiyle asılı (çizgileri kesikli, bandı hafif dolu, sözleri soluk) gelir; tık ya da
  yazılan nokta onu etkin katmana `cad.entities.create` ile tek adımda (“Tablo”) yazar ve seçer.
- **Çizelgeler** (çekirdek `ops::table`, iki platformda aynı): sayılar projenin birimi ve basamaklarıyla (ADR 0149'un gösterim
  kuralı).
  - Koordinat: noktaların ve çizgi, çoklu çizgi ve alan köşelerinin yerleri, 1 µm içinde aynı olan bir kez; adı noktanın etiketi
    (çok noktalı nesnede “ad (2)”), adsızlar 1'den, etiketlerde olmayan numaralarla; sütunlar Nokta, doğu ve kuzey (CAD'de X, Y;
    CBS'de Y, X), bir kot varsa Z.
  - Alan: alanlar, daireler ve tam elipsler; Ad (etiket), Alan (birimiyle), Çevre; birden çoksa Toplam satırı.
  - Öznitelik: etiketi ya da özniteliği olan her nesne; bir etiket varsa Ad, sonra özniteliklerin adları kod sırasıyla.
- Sütunun sözlerinin hepsi sayıysa (işaretli, nokta ya da virgül ondalıklı) sütun sağa dayalıdır. Satır ve sütun boyları sözlere
  göre: satır 2 yazı yüksekliği, sütun sözünün eni artı bir yükseklik, en az 2 yükseklik.

### 4. Dosyalar

- **Excel (.xlsx):** çalışma kitabının her sayfası (adıyla); paylaşılan ve satır içi metinler, sayılar yazıldıkları gibi, mantıksal
  değerler DOĞRU ve YANLIŞ; 10 000 satırdan ve 100 sütundan ötesi okunmaz ve söylenir. Eski Excel (.xls) okunmaz: “XLSX ya da CSV
  olarak kaydedin”.
- **CSV ve TXT:** kodlama içerikten (UTF-8, UTF-16, Windows-1254); ayırıcı ilk 50 kaydın en düzenli olanı (sekme, noktalı virgül,
  virgül, dikey çizgi; en az iki sütun), yoksa boşluklar, yoksa satır başına bir hücre; tırnaklı alanlar CSV gibi.
- 64 MB'tan büyük dosya okunmaz. İlk satır başlık seçeneği; sondaki boş satırlar ve sütunlar düşer, kısa satırlar tamamlanır,
  satır sonlu hücre tek satır olur.

### 5. Tabloyu düzenle

Pencere (komut `table.edit`; tabloya çift tık): hücreler elektronik tablo gibi (sütun harfleri, satır numaraları, başlık kalın,
birleşik alan tek hücre), etkin hücrenin adresi ve sözü üstte.

- Ok tuşları, Tab, Shift ile alan; Enter ya da F2 ve çift tık hücreyi yazar, bir harf yazmaya başlar, Delete seçilenleri boşaltır;
  Ctrl+Z ve Ctrl+Y pencerenin içinde geri alır.
- Araçlar: üstüne ve altına satır, soluna ve sağına sütun ekle (seçilen kadar), satırları ve sütunları sil (biri kalır), hücreleri
  birleştir ve ayır, sütunu sola, ortaya ve sağa dayama, başlık satırı, çizgiler, kalın çerçeve, Yazıya sığdır; seçilen sütunların
  genişliği, satırların yüksekliği ve çerçevenin kâğıttaki kalınlığı yazılır; Kaynaktan ayır.
- Düzenlemelerin kuralları çekirdekte (`ops::table_edit`): eklenen satır, yerindeki satır kadar yüksektir; birleşik alan araya
  giren satırla büyür, silinen satırlarla küçülür, üst satırı silinen alanın sözü kalan ilk satırına geçer; birleştirmede içteki
  alanlar emilir, kesilen alan reddedilir, sözler sırayla boşlukla birleşir; sözü uzayan sütun genişler.
- Kaydet tabloyu `cad.entities.edit`'in `table` işlemiyle tek adımda (“Tablo”) yazar; kaydedilmemiş değişiklikle kapatmak sorar.
  Kilitli katmandaki tablo açılmaz.

### 6. Komutlar, Öznitelikler ve Tabloyu güncelle

- `cad.entities.create` tabloyu yazar (işlem `table`, “Tablo”); `cad.entities.edit`'in `table` ve `tableUpdate` (“Tabloyu
  güncelle”) işlemleri yalnız tabloyu tabloya çevirir (`not_a_table`); `properties` Öznitelikler'in satırlarını yazar.
  `cad.blocks.define` ve `cad.blocks.edit`'in yeniden tanımlaması tabloyu reddeder (`table_in_block`).
- **Öznitelikler:** Yazı stili (CAD), satır ve sütun sayısı, yazı yüksekliği ve açı (yazılır), genişlik ve derinlik, kaynak, konum.
- **Tabloyu güncelle** (`table.update`): seçili tabloların (seçim yoksa kaynağı olan bütün tabloların) kaynağı yeniden okunur:
  çizelge nesnelerinin çizimde kalanlarından (eksikler söylenir), dosyadan gelen tablo dosyası her ad için bir kez yeniden
  seçilerek. Kural (`ops::table_edit::refresh`): kalan satır yüksekliğini, kalan sütun eski ve yeni sözlerinin istediğinden
  genişini korur; sütun sayısı aynıysa hizalar kalır; içteki hücreleri boş kalan birleşik alan kalır; biçim tablonundur. Kilitli
  katmandaki tablo atlanır ve söylenir; değişmeyen yazılmaz; hepsi tek adımda.

### 7. DXF

- **Yazma:** tablo kendi adsız bloğunun (`*Un`) INSERT'idir: blokta çizgiler LINE, band dört SOLID, hücreler TEXT (sözün yazı
  tipiyle; ortalı ve sağa dayalı hizalı); KENTOS verisi tablonun kendi alanlarını taşır. Başka programlar tabloyu blok olarak
  gösterir; rapor bunu söyler. Veri AutoCAD'in nesne sınırını (16 KB) aşarsa tablo yalnız çizgi ve yazılarıyla yazılır.
- **Okuma:** KENTOS verisi olan INSERT tablo olur: başka programda taşınmış ya da döndürülmüşse INSERT'in yeri ve dönüşü, eşit
  ölçeklenmişse boyları ölçekle; aynalanmış, eğik ya da dizi yerleştirme çizgi ve yazılarıyla alınır. Nesnelerden gelen tablonun
  kaynağı düşer (okunan nesnelerin kimlikleri yenidir), dosyanın kaynağı kalır. AutoCAD'in ACAD_TABLE nesnesi bugünkü gibi
  bloğundan alınır.

### 8. `.kcad` şema 22

`table` türü yalnız şema 22'de ve yalnız belgede (bkz. `docs/specs/kcad-v2.md` §6.1, §6.6); tipli sütunlarda tür 15
(`FORMATS_VERSION` 32). Bağımsız Python okuyucu ve yazıcı, `tables.kcad` ve 14 bozuk örnek.

### 9. Arayüz

Tablo komutları CAD projesinin arayüzündedir (Açıklama › Tablo; CBS gizler). İkonlar: `tableInsert`, `tableEdit`,
`tableUpdate`; kaynakların (`tableFile`, `tableCoordinates`, `tableAreas`, `tableAttributes`) ve düzenleyicinin araçlarının her biri
ayrı.

## Kapsam dışı

Hücre başına renk, yazı tipi, hiza ve kenarlık; çok satırlı hücre ve sarma; formül; sıralama; Excel'e canlı bağ (dosya güncellemede
yeniden seçilir); DXF'in ACAD_TABLE nesnesini tablo olarak okuma ve yazma; sayfa düzenindeki tablo (`CAD-07`); Veri karşılaştırma ve
nesne şablonlarında tablo.

## Uygulama

Tek parçada (6 Ekim): sözleşme (`table.rs`: `TableEntity`, `CellRange`, `TableAlign`, `TableGrid`, `TableSource`,
`TableShape::problem`, `TableFileRead`; `EntityGeometry::Table`, `CreateOperation::Table`, `EditOperation::Table`/`TableUpdate`;
`invalid_table`, `not_a_table`, `table_in_block`; katalog, TS tipleri ve Python SDK'sı yeniden üretildi); çekirdek (`geom::table`
yerleşimi, tutamaçları ve sığdırma; `ops::table` çizelgeleri, dosya satırları, boylar; `ops::table_edit` düzenlemeler ve güncelleme;
depo kayıtları `LINE`/`MIXED`, `LABEL_CELL`, paket türü 18; Patlat, dönüşümler, seçme, kenet, kapsam, veride arama); biçimler
(`table_file` XLSX ve CSV/TXT okuyucusu, DXF yazma ve okuma; okuyucunun kestiği sayfa iki platformda kendi sözüyle reddedilir:
`kentos_interaction::table::sheet_cells`, `model/tables.ts` `sheetCells`); `.kcad` şema 22 (kodek, sütunlar, `FORMATS_VERSION` 32, bağımsız Python
okuyucu ve yazıcı, örnek ve bozuk dosyalar, belge); iki platformda çizim (stilli çizgiler ve band, hücre yazıları; paftanın
etiketleri), Tablo ekle (masaüstü `tables/insert.rs`, web `ui/table/TableInsertDialog.ts`), yerleştirme aracı
(`kentos_interaction::table_place`, `tools/tablePlaceTool.ts`), Tabloyu düzenle (`tables/editor.rs`, `ui/table/TableEditor.ts`),
Tabloyu güncelle (`tables/update.rs`, `app/tables.ts`), Öznitelikler'in satırları ve Yazı stili satırının tabloya da uygulanması,
çift tık, CAD şeridinin Açıklama'sında Tablo paneli (CBS gizler), ikonlar; ortak iz `table.json` (oynatıcılara `table`
beklentisi ve ok tuşları), envanter.

## Doğrulama

- `python3 scripts/fixtures/table_cases.py --check` (yerleşim, çerçeve, tutamaçlar, boylar, çizelgeler, dosya satırları,
  düzenlemeler, güncelleme; mpmath ve yazı tiplerinin ölçülmüş ilerlemeleriyle); çekirdek `ops::table`, `ops::table_edit`; web
  `model/ops/table.test.ts`.
- `python3 scripts/fixtures/table_file_cases.py --check` (Python'un zipfile ve xml.etree'siyle yazılmış örnek dosyalar);
  `cargo test -p kentos-formats --test all table_file`; web `io/formats.wasm.test.ts`.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`; `cargo test -p kentos-kcad`; web `io/kcad.wasm.test.ts`.
- `python3 scripts/fixtures/dxf_write_reference.py --check` (`tables.dxf`: adsız bloklar, çizgiler, band, hücre yazıları, KENTOS
  verisi); `cargo test -p kentos-formats --test dxf_write tables`; web `io/dxf.wasm.test.ts`.
- `python3 scripts/fixtures/create_command_cases.py --check`, `edit_command_cases.py --check`, `blocks_command_cases.py --check`;
  iki platformun komut testleri; ortak depo kayıtları (`fixtures/store-records/v1`); stilli çizgiler (`kentos-native-style`
  `table_lines`).
- Ortak iz `table.json` iki platformda üç türde.
- Resimler: `(cd apps/web && node scripts/e2e/shots.mjs tables)`, `KENTOS_SHOTS_ONLY=tablo-cizim,tablo-ekle-alan,tablo-ekle-koordinat,tablo-ekle-oznitelik,tablo-ekle-bos,tablo-yerlestir,tablo-duzenle,tablo-oznitelikler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-tablo-*`).
