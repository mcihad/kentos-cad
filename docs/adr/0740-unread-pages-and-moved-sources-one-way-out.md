# ADR 0740: Okunamayan sayfa ve taşınan kaynak: okunan kalır, aynı soru kaldığı yerden sürer

- Durum: Kabul edildi (29 Ekim 2026)
- Madde: plan/gis.md CBS-40'ın dördüncü dilimi
- İlgili: [ADR 0543](0543-source-status-and-way-out.md) (Kaynak durumu ve tek iş), [ADR 0581](0581-source-query.md)
  (kaynağı sorgula), [ADR 0591](0591-source-ordered-pages.md) (sıralı sayfalar), [ADR 0632](0632-join-view-sources-and-sums.md)
  (birleşik görünümün kaynak tablosu), [ADR 0651](0651-source-state-in-card-filter-and-provider-errors.md) (kaynağın
  durumu kartta, sağlayıcının nedeni), [ADR 0506](0506-relative-sources-and-package.md) (Yeniden bağla)

## Bağlam

CBS-40'ın kalan işi: “Provider hatası, eksik sayfa, bağlantı kopması ve yeniden konumlandırma aynı akıştan
çözülebilsin.” ADR 0651 bunun Kaynak durumu'ndaki yanını yaptı; kalanı Kaynağın tümü'nde okunamayan sayfaydı.

Kod incelendiğinde bugünkü davranış şuydu:

- Kaynağın tümü'nde (ADR 0591) ilk sayfa okunamazsa tablo “Kaynakta soruya uyan kayıt yok.” diyordu: dosya taşınmış
  ya da veritabanı cevap vermiyorken kaynak boş görünüyordu. Sonraki sayfa okunamayınca neden ayakta yazıyordu ama
  düğme yine “Sonraki sayfa”ydı; hangi sayfanın okunmadığı ve ne yapılacağı söylenmiyordu. İlk sayfa için hiç düğme
  yoktu.
- İstatistik'in Kaynağın tümü'nde (ADR 0581 §6) reddedilen soru yalnız nedeniyle kalıyordu; yeniden sormanın yolu
  kapsamı değiştirip geri almaktı. Çatışma (`Conflict`) cevabı iç hata metni olarak (`Conflict { error: … }`) ekrana
  düşüyordu.
- Birleşik görünümün kaynak tablosu (ADR 0632) okunamayınca görünüm kopuk kalıyordu; yeniden okunması için çizimin
  değişmesi gerekiyordu.
- Taşınan dosyada Kaynak ▸ Yeniden bağla (ADR 0506) yalnız seçilen katmanı bağlıyordu; aynı dosyanın diğer bağlı
  katmanları ayrı ayrı bağlanmalıydı.

Üreticilerin yolu (resmî belgeler, 29 Ekim 2026'da okundu):

- QGIS 3.44 kullanıcı kılavuzu, “Working with Project Files”: veri kaynağına ulaşılamayan katman (taşınan ya da
  yeniden adlandırılan dosya, veritabanı kesintisi) projede kalır; **Handle Unavailable Layers** penceresinde yol
  elle yazılır, **Browse** ile seçilir ya da **Auto-Find** ile aranır; sonradan katmanın uyarı simgesi ya da
  **Repair Data Source…** ile onarılır; bir yol onarılınca aynı bozuk yolu paylaşan diğer katmanlar da onarılmaya
  çalışılır. <https://docs.qgis.org/3.44/en/docs/user_manual/introduction/project_files.html>
- ArcGIS Pro, “Repair broken data sources for layers and tables”: verisi bulunamayan katman kırmızı ünlemle kalır
  (taşınan, yeniden adlandırılan, silinen veri; erişilemeyen sunucu); **Set Data Source** ile onarılır; aynı taşınan
  kaynağa bakan katmanlar bir onarımla birlikte onarılır.
  <https://pro.arcgis.com/en/pro-app/latest/help/mapping/layer-properties/repair-broken-data-links.htm>

İkisinde de okunamayan kaynak bir hata kutusuyla kaybolmaz; katman kalır, neden söylenir ve tek bir onarım yolu
vardır. KentOS'ta bunun karşılığı ADR 0543'ün “durumun istediği tek iş” kuralıdır; bu ADR onu kaynağı okuyan her
yüzeye taşır.

## Karar

### 1. Ortak kural: reddin kodu ne yardım ettiğini söyler

`kentos_native_application::sources::trouble::Trouble::of(code)` kaynağın reddinin kodundan neyin yardım ettiğini
verir:

| Kod | Ne yardım eder | Neden |
|---|---|---|
| `source_unreachable`, `import_unreadable` | Yeniden dene (`Retry`) | Kaynak bağın dediği yerde ama şu an cevap vermedi: kopan bağlantı, yeniden başlayan veritabanı, başka bir programın tuttuğu ya da yazdığı dosya |
| `file_not_found`, `table_not_found`, `invalid_file_path`, `invalid_address`, `no_row_key`, `row_key_not_unique` | Yeniden bağla (`Relink`) | Kaynak bağın dediği yerde değil: taşınan ya da silinen dosya, yeniden adlandırılan ya da kaldırılan tablo, değişen adres ya da view |
| `source_changed` | İlk sayfadan (`Changed`) | Sayfalar okunurken kaynak değişti |
| `order_too_large`, `input_too_large` | Daralt (`Narrow`) | Soru bir cevabın alabileceğinden çoğunu okur; süzgeç ya da kutu |
| diğerleri | yok | Soru kendisi yanlış (olmayan alan, derlenmeyen süzgeç): başka bir soru |

Kural yalnız kodu okur; ret komutundur (`cad.sources.query`'nin kodu ve sözü), kural hiçbir şeyi değiştirmez. Kodlar
`codes.rs`'in tablosundadır; protokolün `retryable`'ı (ADR 0253 §7: yalnız çatışma) değişmez: “Yeniden dene” aynı
istek değil, durum düzelince aynı sorudur.

### 2. Komutun sözleşmesi: reddedilen sayfa hiçbir şey yazmaz, aynı soru kaldığı yerden sürer

`cad.sources.query`'nin sayfası bir kez reddedildiğinde çizim, revizyon ve bağ değişmez. Aynı soru (aynı `cursor`,
varsa aynı `expectedStamp`) kaynak geri gelince sorulduğunda sayfa kaldığı yerden sürer: sayfaların satırları
kesintisiz okumanın satırlarıdır, hiçbiri iki kez gelmez, hiçbiri atlanmaz. Bu ADR 0581 ve 0591'in imleç kuralından
doğar ve şimdi ortak durumlarla kanıtlanır:

- İmleç tablonundur, yolun değil: dosya yeni yerinde, eski imleçle sorulduğunda orada sürer; bağlı katman yeni yerine
  `cad.sources.edit`'in `relink`'iyle bağlanınca aynı soru katmandan sürer.
- Dosya önce aranır, damga sonra denetlenir: yerinde olmayan dosya `file_not_found` der (damga değil); dosya gelir de
  değişmişse `source_changed`.
- Taşınan dosyanın baytı ve zamanı aynı kalır (aynı diskte taşıma), damgası (`boy:zaman`) da; bu yüzden geri gelen
  dosyada damgalı soru da sürer.
- Veritabanı sağlayıcısı her soruda kendi bağlantısını açar ve kapatır (ADR 0505; havuz yok): sayfalar arasında kopan
  bağlantı bir sonraki soruyu etkilemez; soru sürerken kopan bağlantı o soruyu `source_unreachable` ile reddeder ve
  aynı soru yeniden sorulur.

Python ve MCP'de bunun için ayrı bir yol yoktur: aynı komut, aynı kodlar (katalogda anlamlarıyla), aynı imleç.

### 3. Masaüstünde aynı akış

`apps/desktop/src/source_trouble.rs` reddi (`Refused`: kod ve söz) ve kuralın dediği tek düğmeyi (`way_out`) verir;
kaynağı okuyan her yüzey onu kullanır:

- **Kaynağın tümü** (Öznitelik tablosu): okunamayan sayfa okunan sayfaları tabloda bırakır; ayakta “Sayfa n
  okunamadı: …” yazar. Düğme kuralındır: `Retry` için **Yeniden dene** (son soru olduğu gibi: aynı imleç, aynı damga),
  `Relink` için katmanın **Yeniden bağla…**'sı (veritabanında **Adresi denetle**, Kaynak durumu'nun sözcükleri),
  `Narrow` için düğme yok (söz süzgeci söyler). İlk sayfa okunamazsa satırların yerinde aynı söz yazar (artık “kayıt
  yok” demez). Durdur'la kesilen sayfa “Okuma durduruldu; okunan sayfalar tabloda.” der, **Sürdür** onu aynı
  imleçle yeniden sorar. Sayfalar arasında kaynak değişirse ilk sayfa yeniden okunur ve söylenir (ADR 0591, değişmedi).
- **İstatistik**'in Kaynağın tümü: ret nedeniyle ve aynı düğmeyle kalır; çatışma artık iç metin değil, sözüdür.
- **Birleşik görünüm**: kaynak tablosu okunamayınca neden yanında **Yeniden dene** görünüm okumasını yeniler. Tablo
  yerinde değilse düğme yoktur: görünümün kaynağı Birleşik görünümler… penceresinde değiştirilir.
- **Yeniden bağla…**: dosya yeni yerine bağlanınca aynı dosyaya bağlı diğer katmanlar da aynı yere bağlanır (her biri
  `cad.sources.edit`'in `relink`'i, içeriği denetlenerek; QGIS ve ArcGIS Pro'nun onarımı gibi); bağlanan her katmanın
  okunamayan sayfası ve istatistiği kendiliğinden yeniden sorulur. Yeniden bağlanamayan diğer katman olduğu gibi
  kalır ve söylenir.

## Doğrulama

- Kural: `sources::trouble`'ın testi her kodu ve kodların tablodaki türünü denetler.
- Ortak durumlar: `fixtures/commands/v1/cad.sources.query.trouble.json`, 22 durum ve 132 adım, bağımsız başvuru
  `scripts/fixtures/source_trouble_cases.py` (KentOS kodu olmadan; sayfalar ADR 0581 ve 0591'in kuralıyla sqlite3'ün
  okuduğu `sira.gpkg`'den, `source_order_cases.py` ve `source_query_cases.py`'nin işlevleriyle). Dosya taşınır,
  silinir, GeoPackage olmayan bir dosyayla ya da tabloyu taşımayan başka bir GeoPackage'la değişir, yerinde klasör
  olur; anahtar sırası, kaynakta ve burada sıra, kaynağa giden ve burada denetlenen süzgeç, istatistik, tablonun sonu,
  dosyanın kopyası, damga ve bağlı katmanın yeniden bağlanması. Koşucular (Rust ve Python) dosyayı yeni `moveFile`
  adımıyla taşır.
- PostGIS: `a_page_lost_with_its_table_or_its_server_goes_on_from_its_cursor` (`scripts/fixtures/postgis_cases.py`'nin
  geçici veritabanında): tablo yeniden adlandırılınca `table_not_found`, geri adlandırılınca sayfa sürer; sunucuya
  ulaşılamayınca `source_unreachable`, sonra aynı imleçle sürer; sayfaların anahtarları 1…2400, her biri bir kez.
- Masaüstü: Kaynağın tümü'nde taşınan dosyanın sayfası, ilk sayfanın reddi, Durdur ve Sürdür, yeniden bağlanan
  katmanın sayfası (`whole_tests.rs`); İstatistik'in reddi, Yeniden dene ve yeniden bağlama (`statistics/source_tests.rs`);
  aynı dosyanın iki katmanının birlikte yeniden bağlanması (`exchange/gpkg_tests.rs`); düğmenin kuralı
  (`source_trouble.rs`).
- Resimler: `.run/shots/tablo-tumu-*-6-sayfa-okunamadi-*`, `-7-ilk-sayfa-*`, `-8-durduruldu-*` ve
  `istatistik-kapsam-*-okunamadi-*`, 1440×900 koyu ve açık, 1100×650 koyu.

## Sonuçlar

- Kaynak okunamayınca hiçbir yüzey boş kaynak göstermez; hangi sayfanın neden okunmadığı ve tek yardım eden iş
  söylenir.
- Okunan sayfalar kaybolmaz; aynı soru kaldığı yerden sürer. Bu artık ortak durumlarla ve gerçek veritabanıyla
  kanıtlıdır.
- Taşınan dosyanın bütün katmanları bir onarımla yeni yerine bağlanır.

## Kalan

- İki referans kullanıcı grubuyla kabul (GIS uzmanı ve form kullanan belediye personeli): sahibin ve kullanıcıların
  işi; CBS-40 o zamana dek açık kalır.
- Taşınan dosyanın kendiliğinden aranması (QGIS'in Auto-Find'ı): açılışta göreli yol ve aynı içerik aranıyor
  (ADR 0506); çalışırken taşınan dosya için arama yok.
- Okunamayan sayfanın kendiliğinden yeniden denenmesi (zamanlayıcıyla) yok: kullanıcı ya da betik sorar.
