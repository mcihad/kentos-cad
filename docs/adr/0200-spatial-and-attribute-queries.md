# ADR 0200: Mekânsal ve öznitelik sorgusu

- **Durum:** kabul edildi (2026-10-07). Kapsam sahibin kararlarıdır (7 Ekim): Konuma göre seç, Bilgi al, Özet istatistik ve Anahtarla
  birleştir; hepsi İşlemler aracı; ilişkiler (1-N, N-M) ve ilişki formu sonraya (`DOM-09`–`DOM-11`). Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-02` (ve araştırma notu), ADR 0084 (İşlem araçları), ADR 0100 (ifade dili ve `$merkez_y`,
  `$merkez_x`), ADR 0199 (katman alanları), ADR 0184 §4 (tablo dosyası okuyucusu), ADR 0149 (gösterim kuralı).

## Bağlam

İfadeyle seç ve Öznitelik hesapla bir nesnenin kendi özniteliklerine ve geometrisine bakar. Bir katmanın nesnelerini başka bir katmanın
nesnelerine göre seçmenin, bir alanın içindekilerden (ağaç sayısı, yapıların taban alanı toplamı) ya da bir nesneyi çevreleyen alandan
(yapının parsel numarası) bilgi almanın, bir alanın grup grup istatistiğinin ve başka bir katmandan ya da CSV/Excel dosyasından ortak
anahtarla öznitelik aktarmanın yolu yoktur. Netcad'de Konumsal Seçim, İçindekinden ve Çevreleyenden Bilgi Al ve Veri Aktar; ArcGIS'te
Select Layer By Location, Spatial Join, Summary Statistics ve Join Field; QGIS'te Select by location, Join attributes by location,
Statistics by categories ve Join attributes by field value vardır.

## Karar

Beş yeni İşlemler aracı iki platformda, ADR 0084'ün çalıştırıcısıyla (tek geri alma adımı, kilitli katmandaki nesneye yazmama, arka
planda çalışma, modellerde kullanılma): hesap ortak çekirdekte, araçların tanımı web'de `processing/builtin/`, masaüstünde
`kentos-processing`'de.

### 1. Nesnenin sorgudaki geometrisi ve ilişkiler

- Bir nesne sorguda şunlardır: **alanları** (kapalı alan ve parçaları delikleriyle, daire, tam elips, kapalı eğri; çekirdeğin
  `areas_of_entity`'si: yaylar kesin, elips ve eğri 1 mm içinde), **kenarları** (çizgi, çoklu çizgi, yay, elips ve eğri yayı, alanların
  sınırları; `entity_edges`) ve **noktaları** (nokta ve çok noktalı nesnenin noktaları; yazı, blok yerleştirmesi, tablo, resim ve ölçünün
  yerleşim noktası). Sonsuz doğru ve ışın sorguya girmez.
- **Kesişen**: ortak bir noktaları vardır: kenarlar kesişir ya da değer, bir nokta öbürünün kenarında ya da alanının içindedir, ya da
  biri öbürünün alanının içindedir.
- **İçeren** (yalnız alanı olan nesne): öbür nesnenin her noktası bu nesnenin alanının içinde ya da sınırındadır (kenarları sınırla
  kesiştikleri yerlerden bölünür, her parçanın ortası içeride ya da sınırda); öbürünün alanı bu nesnenin bir deliğini içine almaz.
- **İçinde kalan**: öbürü onu içerir.
- **Ayrık**: kesişmez.
- **Uzaklıkta**: aralarındaki en kısa uzaklık verilen uzaklıktan büyük değildir (kesişenlerin uzaklığı 0).
- **Merkezi içinde**: nesnenin merkezi öbürünün alanının içinde ya da sınırındadır. Merkez ifadelerdeki `$merkez_y`, `$merkez_x`'tir
  (`shape_centroid`): alanın ağırlık merkezi, öbürlerinin yerleşim noktası.
- “Sınırda” ve “değer” 1 mm içindedir (ölçü ve kenet kararlarından ayrı, açık bir tolerans; CLAUDE.md §23.3). Bir nesne kendisiyle
  karşılaştırılmaz.
- Çekirdek: `ops::spatial_query` (ilişki ve merkez) ve deponun `relate_pairs`'i (kutularla adaylar, sonra kesin karar): girdi ve başvuru
  listelerinden ilişkiyi sağlayan çiftler, girdi sonra başvuru sırasıyla.

### 2. Konuma göre seç (`selection.byLocation`)

- **Seçilecek nesneler** (Tümü, Görünen, Katman, Seçili), **İlişki** (yukarıdaki altısı), **Başvuru nesneleri** (Seçili, Katman,
  Görünen, Tümü), **Uzaklık** (Uzaklıkta'da, m), **Seçim biçimi** (Yeni seçim, Seçime ekle, Seçimden çıkar, Seçim içinde ara; İfadeyle
  seç'inki). Bir girdi nesnesi başvurulardan biriyle ilişkiyi sağlıyorsa seçilir (Ayrık: hiçbiriyle kesişmiyorsa). Çizimi değiştirmez.
- Başvuru yoksa çalışmaz, İşlemler'in boş girdi kuralıyla, sorun Başvuru nesneleri'nin altında: ““Başvuru nesneleri”: seçili
  nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin.” Sonsuz doğru ve ışın girdi de başvuru da olmaz
  (alanın notu: “Yardımcı çizgi, ışın alınmaz.”).

### 3. Bilgi al

- **İçindekinden bilgi al** (`attributes.fromInside`): **Hedef alanlar** (alanı olan nesneler; yazar), **Kaynak nesneler**, **İlişki**
  (İçinde kalan, Kesişen, Merkezi içinde: kaynağın hedefe göre), **İstatistik** (Sayı, Toplam, Ortalama, En az, En çok, İlk değer),
  **Alan** (kaynağın; Sayı'da yok), **Yazılacak alan** (hedefin). Her hedefe ilişkiyi sağlayan kaynakların değerinden yazılır: Sayı
  tam sayı (hiçbiri yoksa 0); öbürleri hiçbiri yoksa boş (öznitelik kaldırılır); İlk değer çizim sırasıyla ilk dolu değer, olduğu gibi.
- **Çevreleyenden bilgi al** (`attributes.fromEnclosing`): **Hedef nesneler** (yazar), **Kaynak alanlar**, **Alan** (kaynağın),
  **Yazılacak alan** (boşsa kaynağın alanının adı). Hedefin merkezini içeren kaynak alanın değeri yazılır; birden çok alan içeriyorsa çizim
  sırasıyla ilki (sayılır ve söylenir), hiçbiri içermiyorsa hedef değişmez (sayılır ve söylenir).
- Değer yazmak katmanın alanlarının kuralıyladır (ADR 0199 §1): hedefin katmanında yazılacak alan varsa değer onun tek biçimine çevrilir,
  uymayan değer bütün çalıştırmayı reddeder (neden ve nesneyle). Bu kural İşlemler'in bütün öznitelik yazmalarına (Öznitelik hesapla da)
  uygulanır: çalıştırıcı değişiklik kümesini uygulamadan önce denetler.

### 4. Sayıların kuralı

- Değer sayı olarak okunur: katmanın alanı tam ya da ondalık sayıysa tek biçimi; değilse metin ondalık sayının kuralıyla (`.` ya da `,`
  ayırıcı, üs yok; ADR 0199 §1). Okunamayan değer atlanır, sayılır ve söylenir.
- Toplam, En az ve En çok kesindir (ondalık metinler üzerinde, yuvarlama yok; `ops::statistics`). Ortalama toplamın sayıya bölümüdür,
  girdilerin en çok kesir basamağından iki fazla basamağa yarım çifte yuvarlanır; yazılacak alan ondalık sayıysa onun basamağına, tam
  sayıysa birlere yarım çifte yuvarlanır. Standart sapma (örneklem, n − 1) çift duyarlıkta hesaplanır ve ortalamanın basamağıyla
  yazılır; tek değerde boştur. Kural sürümlüdür: `kentos.statistics/1`.

### 5. Özet istatistik (`statistics.summary`)

- **Nesneler**, **Alan**, **Grupla** (isteğe bağlı alan). Çıktı bir tablodur: Grup (gruplanınca), Nesne (gruptaki nesne sayısı), Değer
  (sayı okunan değer sayısı), Toplam, Ortalama, En az, En çok, Std. sapma; gruplar grup metninin doğal sırasıyla, boş grup “(boş)” en
  sonda; gruplanınca son satır “Toplam”. Pencere tabloyu gösterir: Panoya kopyala (sekmeyle ayrılmış) ve CSV olarak kaydet.
  Çizimi değiştirmez.

### 6. Anahtarla birleştir (`attributes.joinByField`)

- **Hedef nesneler** (yazar), **Hedef anahtar** (hedefin alanı), **Kaynak** (Katman ya da Dosya), **Kaynak nesneler** ya da **Dosya**
  (CSV, TXT, XLSX: Tablo ekle'nin okuyucusu, ADR 0184 §4; ilk sayfa, ilk satır başlık), **Kaynak anahtar** (boşsa hedef anahtarla aynı
  ad), **Aktarılacak alanlar** (virgülle; boşsa anahtardan başka hepsi), **Önek**, **Var olan değerler** (Üzerine yaz, Yalnız boşlara).
- Anahtarlar başında ve sonundaki boşluk atılarak karşılaştırılır; iki taraf da sayı okunuyorsa sayı olarak (`007` ile `7` aynı),
  değilse metin olarak tam. Kaynakta aynı anahtar birden çok kez varsa ilki alınır ve söylenir. Eşleşmeyen hedefler ve kullanılmayan
  kaynak satırları sayılır ve söylenir. Değerler §3'ün kuralıyla yazılır.

### 7. İşlemler altyapısına iki ek

- **Dosya parametresi** (`file`): pencerede Dosya seç… düğmesi ve seçilen dosyanın adı; uzantıları parametre söyler. Değeri dosyanın adı
  ve içeriğidir; son değerlerde yalnız adı saklanır (masaüstünde yolu): dosya yeniden açılan pencerede yeniden seçilir (masaüstünde
  yolundan okunur). Modelin girdisi olamaz.
- **Tablo çıktısı** (`table`): sütunlar ve satırlar (metin). Pencere çalıştırmadan sonra gösterir; modelde sonraki adıma geçmez.
- **Alan parametresinin kaynağı** bir dosya da olabilir: alan listesi dosyanın sütunlarıdır (“n satır”); `of` bir liste ise görünen ilk
  kaynak okunur (Kaynak: Katman ya da Dosya). Birden çok ad alan alan (`multiple`, Aktarılacak alanlar) listede adları işaretler; adlar
  virgülle yazılır. Okunan bir alanın notu “n nesnede var.”dır, yazılanınki gibi “değeri değişir.” demez.
- **Araç reddedebilir** (`refused`): ileti olduğu gibi, çalıştırma `error`, çizim değişmez (dosyada anahtar sütunu yok: “Kaynakta “Ada”
  alanı yok; alanları: Parsel, Malik, Hisse.”).

## Uygulama

- Çekirdek: `ops::spatial_query` (`Relation`, `geometry_of`, `intersects`, `contains`, `distance`, `center_in`, `relate`), deponun
  `relate_pairs`'i (`store/processing.rs`), `ops::statistics` (`kentos.statistics/1`: `Dec`, `read_number`, `figures`, `statistic`,
  `summarize`, `join_key`, `join_plan`; web'e `statisticMany` (grup başına ölçek), `summarizeValues`, `joinPlan`); WASM'da deponun
  `relatePairs`'i.
- Web: araçlar `processing/builtin/` (`selectByLocation.ts`, `infoFromInside.ts`, `infoFromEnclosing.ts`, `summaryStatistics.ts`,
  `joinByField.ts`, ortak yazma kuralı `attributeWrites.ts`), çalıştırıcının denetimi `processing/writeCheck.ts`, dosya parametresi ve
  tablo çıktısı `types.ts`, `parameters.ts`, `runner.ts` (dosyanın özeti), `ui/processing/` (form, görünüş, `paramFields.ts`'in dosya
  alanı, `ToolDialog.ts`'in sonuç tablosu), dosyayı okuyan `app/processing.ts`'in `chooseFile`'ı.
- Masaüstü: `kentos-processing`'de araçlar `builtin/` (`select_by_location.rs`, `info_from_inside.rs`, `info_from_enclosing.rs`,
  `summary_statistics.rs`, `join_by_field.rs`, `queries.rs`), denetim `writes.rs`, `ParamKind::File`, `OutputKind::Table`,
  `RunResult::refused`, `RunContext::field`; pencerede dosya (`fields.rs`, `dialog::file_value`, yoldan yeniden okuma) ve sonuç tablosu
  (`window.rs`, Panoya kopyala ve CSV olarak kaydet).
- KentOS UI'ın parçalı seçimi genişliği doldururken web'in `flex: 1`'iyle dizilir: parçalar eşit paylaşır, hiçbiri yazısından dar
  olmaz (`segmented::fill_widths`); “İçinde kalan / Kesişen / Merkezi içinde” iki satıra kırılmaz.
- İkonlar sahibin seçimleri (7 Ekim): `selectLocation`, `infoInside`, `infoEnclosing`, `statsSummary`, `joinField`. Araçlar İşlemler
  araç kutusunda ve CBS şeridinin Analiz sekmesinde kategorileriyle (Seçim, Öznitelik, Analiz).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/spatial_query_cases.py` (ADR'den, KentOS kodu olmadan; düz kenarlarda kesirlerle, dairede kesin
  uzaklıkla): ilişkiler, merkezler, sayıların kuralı ve anahtar eşleme `fixtures/spatial-query/v1/cases.json`'da; araçların çalıştırmaları
  İşlemler'in ortak durum biçiminde `fixtures/processing/v1/queries.json`'da, çizimi `queries.kcad` ve dosyası `malikler.csv`. İki
  platform ikisini de geçer: çekirdek `tests/all/spatial_query.rs`, web `processing/cases.test.ts` (sayfada ve işçinin yolundan),
  masaüstü `kentos-processing`'in `tests/cases.rs`'i (burada ve çizimin okuma kopyasında); dosya değerleri iki platformda Tablo ekle'nin
  okuyucusuyla okunur.
- Pencere: `fixtures/processing/v1/dialog.json`'un yeni formları ve oturumları (`by-location`, `summary`, `join-file`: Uzaklık'ın
  görünmesi, boş başvurunun sorunu, sonuç tablosu ve kalkması, yeniden seçilmesi istenen dosya, dosyanın sütunları, işaretlenen adlar);
  masaüstünde `processing::query_tests` (tablo, dosya ve son değerleri, iki ret).
- Resimler iki platformda 1440×900 ve 1100×650, açık ve koyu: web `(cd apps/web && node scripts/e2e/shots.mjs queries)`, masaüstü
  `cargo test -p kentos-desktop processing::query_tests::screens -- --ignored --nocapture` (`.run/shots/islem-sorgu-*`).
