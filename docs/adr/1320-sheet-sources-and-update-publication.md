# ADR 1320: Paftanın kaynak dökümü ve önceki teslimden güncelleme yayını

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/map-cad.md `MAP-18` (“Harita anotasyonu, pafta ve teslimi tek
  revizyondan üret”) maddesinin ilk dilimi: “mühendislik nesnesi güncellenince etkilenen pafta/tablolar listelensin; son
  yayındaki kaynak revizyonu ile güncel model farkı görülsün; eski paftaya yeni tarih basarak güncelmiş gibi yayınlama”
  ve “yalnız değişen sayfaların yeniden üretimi ve teslim paketi manifesti mevcut atlas/job altyapısına bağlansın”.
- **Bağlam belgesi:** ADR 0615 (sıralı, atomik yayın), 0617 (kaynaklı teslim), 0619 (sabit kaynak pini), 1090 (sayfa
  içerik özeti, yayın durumu, devam ettirilebilir yayın; `cad.sheets.set`'in `shows`/`table`/`marked` nedenleri).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler §7'de.

## Bağlam

ADR 1090'dan sonra bir teslim klasörünün `yayin.json`'u her sayfanın bağlamsız içerik özetini taşıyor;
`file.publish.status` bu özeti çizimin şimdiki hâliyle karşılaştırıp paftayı `current`/`stale`/`fileChanged`… diye
söylüyor. MAP-18'in istediğine göre eksik olanlar:

1. **Neden eskidi:** durum yalnız “değişti” diyordu. Hangi nesnenin değiştiği, sayfaya hangisinin girip hangisinin
   çıktığı, yoksa paftanın kendi düzeninin ya da katman, stil, şablon gibi bütün sayfaların okuduğu bir parçanın mı
   değiştiği görülmüyordu. Nesnelerin kendi revizyonu yok (EntityBase sürüm taşımaz); belge revizyonu bütün çizimin.
2. **Güncelleme yayını:** eskiyen paftaları yeniden yayınlamanın tek yolu yeni bir klasöre ya bütün seti (değişmeyen
   paftalar da bugünün tarihiyle yeniden basılır: “eski paftaya yeni tarih”) ya da yalnız eskiyenleri (eksik bir teslim)
   yayınlamaktı. Masaüstü penceresi “yalnız onları yeniden yayınlayın” diyordu ama bütün bir güncel teslim
   kurmanın yolu yoktu.

## Araştırma

- **ArcGIS Pro harita serisi** (Export a map series, [pro.arcgis.com/…/export-options.htm](https://pro.arcgis.com/en/pro-app/latest/help/layouts/export-options.htm)):
  dışa aktarım bütün sayfalar, şimdiki sayfa, seçili sayfalar, seçili dizin nesnelerinin sayfaları ya da sayfa aralığıdır;
  tek çok sayfalı PDF ya da sayfa başına PDF. Hangi sayfanın değiştiğini ürün söylemez; alt kümeyi kullanıcı seçer.
- **QGIS atlas** (QgsLayoutAtlas, [api.qgis.org/…/classQgsLayoutAtlas.html](https://api.qgis.org/api/classQgsLayoutAtlas.html)):
  kapsama katmanı bir ifadeyle süzülür (`setFilterFeatures`, `setFilterExpression`), `updateFeatures` süzgeci yeniden
  uygular. Değişen sayfayı bulmak yine kullanıcının ifadesidir.
- **AutoCAD Map 3D harita kitabı** (About Map Books, [help.autodesk.com/…/MAP3D-Use](https://help.autodesk.com/cloudhelp/2027/ENU/MAP3D-Use/files/GUID-E74970C7-BD42-4B22-9E82-1F4D51B29BB0.htm)):
  harita değişince harita kitabı yeniden üretilir; kitap bir AutoCAD sayfa setine bağlıdır, bağı koparsa “Rebuild Sheet
  Set”. Değişmeyen sayfayı koruyan bir yol anlatılmıyor.
- **Revizyon:** ISO 7200 başlık bloğunun revizyon göstergesi alanı ve ISO 19650-2'nin revizyon kodu (P01, C02…;
  NBS ve Autodesk Docs özetleri: [support.thenbs.com](https://support.thenbs.com/support/solutions/articles/7000055103),
  [Autodesk Docs Revision Codes](https://help.autodesk.com/cloudhelp/ENG/Docs-Files/files/file-naming-standard/Revision_Codes.html)):
  değişen bir belge yeni revizyonla yayınlanır; eski revizyon yerinde kalır (yerine geçilmiş, “superseded”). Bir
  tarihin ya da revizyonun değişmeyen sayfaya basılması o sayfayı yeni revizyon gibi gösterir.

**KentOS kararı:** değişen sayfayı kullanıcı seçmez, sayfa özetinden ve kaynak dökümünden ürün bulur; güncelleme
önceki teslimin yerine yazılmaz, yeni klasöre kurulur (önceki teslim “yerine geçilmiş” olarak kalır); değişmeyen sayfa
yeniden basılmaz, baytları ve tarihiyle taşınır; değişen sayfanın revizyonu ilerletilmediyse söylenir.

## Karar

### 1. Kapsam

`file.publish.sheets` (v1, geriye uyumlu yeni alanlar) ve `file.publish.status` (v1, yeni alanlar). Yeni komut yok,
`.kcad` şeması değişmez; kayıt teslim klasörünün `yayin.json`'undadır.

### 2. Sayfanın kaynak dökümü

Her yayın `yayin.json`'a `sources` yazar:

- `digest`: `kentos.object-digest/1`.
- `pages[]`: her paftanın `objects`'i, kalıcı kimlikleriyle çizim sırasında. Bir nesne sayfanın kaynağıdır, eğer:
  görünüm penceresinin zeminine kutusu değiyor ve pencere onu basıyorsa (kat, evre, kategori, katman durumu;
  `cad.sheets.set`'in `shows`'u), pencerenin gösterdiği bir model tablosu onu okuyorsa (`table`), paftaya bakan bir
  görünüm işaretiyse (`marked`) ya da paftadaki bir kâğıt öznitelik tablosunun katmanındaysa. Kutu sınaması fazlasını
  listeleyebilir, eksiğini listelemez. Zemini bilinmeyen bir penceresi olan sayfanın (atlas, yerleşmemiş) dökümü
  `null`'dır: listelenemez.
- `objects`: listelenen her nesnenin içerik özeti: nesnenin anlık görüntü JSON'u, yuva kimliği (`id`, yeniden açılışta
  değişebilir) çıkarılarak, anahtarlar sıralı, boşluksuz, UTF-8; SHA-256. Bağımsız okuyucu
  `json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False)` ile aynı metni kurar (sayılar en kısa
  gidiş-dönüş yazımıyla; üslü yazımda Python ile serde farklı yazar, bu yüzden başvuru durumları üslü sayı içermez).
- `sheets`: her paftanın kendi kaydının (öğeler, sayfa, çizim bilgisi) özeti; `parts`: bütün sayfaların okuduğu proje
  parçalarının özetleri: `settings`, `layers`, `styles`, `blocks`, `book` (paftalar dışında kitap: ana sayfalar,
  şablonlar, profiller).

`file.publish.status` her yayınlanmış paftanın satırına `date`'i (sayfanın baytlarının üretildiği gün) ve `changes`'i
ekler: `changed` (yayında da şimdi de kaynak, içerik özeti değişmiş), `added` (şimdi kaynak, yayında değildi),
`removed` (yayında kaynaktı, şimdi değil: silinmiş, çıkmış, gizlenmiş), `sheet` (paftanın kendi kaydı değişti),
`parts` (değişen proje parçaları; bağlı resimlerin sürümü değiştiyse `resources`). Dökümü olmayan (ADR 1320'den önceki
ya da listelenemeyen) sayfada `changes` yoktur. `changes` sayfanın eskidiğinin kanıtı değil, nedenidir: eskime kararı
ADR 1090'daki sayfa özetidir. Bir nesnenin yalnız çizilmeyen özniteliği değiştiyse nesne `changed`'dadır ama sayfa
`current` kalır.

### 3. Güncelleme yayını (`previous`)

`file.publish.sheets {path: <yeni klasör>, mode: "separate", previous: <önceki teslim>}`:

- Önceki teslim mutlak bir klasör, `yayin.json`'u `kentos.publish/1` olmalı (`no_previous`); yeni klasör onunla aynı
  olamaz (`invalid_input`; güncelleme önceki teslimin üzerine yazılmaz). Yalnız ayrı PDF'lerle (tek PDF'te değişmeyen
  sayfa ayrı korunamaz) ve `resume`/`resumable` olmadan (`invalid_input`).
- Seçilen her pafta için, sayfa özeti önceki teslimdeki sayfanınkine eşitse ve o paftanın tek başına olduğu PDF diskte
  teslim edilen SHA-256'yı taşıyorsa, PDF baytı baytına yeni klasöre kopyalanır (ad şablonu değiştiyse yeni adıyla);
  sayfa kaydı önceki tarihini (`date`) ve bulgularını korur. Öbür paftalar bugünün tarihiyle yeniden üretilir.
  Çıktıdaki `carried` kopyalananları, `previous` önceki teslimi (yol, `yayin.json`'un SHA-256'sı, tarihi, kaynağı) verir.
- Sayfa özeti içerik akışını, boyutu ve yüz/resim/grafik durumu adlarını kapsar; sayfanın coğrafi bağını (`geo`),
  katman (OCG) seçeneğini, resimlerin piksellerini ve çözünürlüğü kapsamaz. Bu yüzden `geo`, `layers`, `dpi` ya da bağlı
  resimlerin sürümü (`resourcesSha256`) önceki teslimdekinden başkaysa hiçbir sayfa kopyalanmaz.
- Sayfası değişip yeniden üretilen paftanın revizyonu önceki teslimdekiyle aynıysa (boş da olsa) hata olmayan
  `revision_unchanged` uyarısı: “Pafta bilgisinde revizyonu ilerletin”. Yalnız dosyası bozulduğu için yeniden üretilen
  (özeti aynı) paftaya uyarı yoktur.
- Klasör ADR 0615'teki gibi geçici klasörde kurulur ve tek adlandırmayla görünür; iptal ya da hata önceki teslimi ve
  yeni klasörü olduğu gibi bırakır. `plan` hiçbir şey yazmadan hangi paftaların kopyalanacağını söyler (önceki teslimin
  PDF'lerini okuyup SHA'larını sınar).
- Yeni teslimin `yayin.json`'u `previous`'u, `carried`'ı ve kendi kaynak dökümünü taşır; güncellenmiş teslimin
  `file.publish.status`'u çizimin şimdiki hâliyle bütün sayfalarda `current`'tır.

### 4. Arayüz

Masaüstü “Paftaları toplu yayınla” penceresi: Durum satırlarında sayfanın tarihi ve eskiyen sayfada nedenin kısa özeti
(“1 nesne değişti, 2 nesne eklendi · pafta düzeni · katmanlar”). Teslim eskiyse “Güncelle” düğmesi: önceki teslimi
`previous` yapar, yolu `<teslim>-<bugün>` önerir, ayrı PDF'i seçer, yayında olan paftaları seçer ve denetimi başlatır;
rapor hangi paftaların kopyalanacağını ve yeniden üretileceğini söyler; “Güncellemeyi kaldır” baştan yayına döner.

### 5. Otomasyon

Komut yüzeyi değişmedi: katalog özeti ve örneği, `no_previous` reddi (yetenekler), SDK (`scripts/python/sdk.py`),
MCP (katalogdan), masaüstü kayıtları aynı komutlar. Ortak durumlar: `fixtures/commands/v1/file.publish.sheets.update.json`
(25 durum; `scripts/fixtures/sheet_update_cases.py --check`): eskiyen paftalar, sayfa kaynakları ve nesne özetleri
kutulardan ve JSON'dan bağımsız hesaplanır. Rust koşucusu `crates/native/headless/tests/headless/sheet_update.rs`.

### 6. Kabul ve ölçüm

- Aynı 60 parsellik veri (TUREF/TM30, 500 km doğu, 4540 km kuzey) 1:500 batı, 1:500 doğu, 1:1000 ve 1:5000 paftalarda:
  20 m'lik kenar kâğıtta 40/20/4 mm (plot testlerinin bağımsız PDF okuyucusu; 120/120/180/240 kenar), her sayfanın
  çerçeve köşelerinin zemin koordinatı GDAL'ın GeoPDF okumasıyla (`gdalinfo -json`, GDAL sayfayı her eksende tam
  piksele yuvarlar: A3 150 dpi'da 2480 × 1754) beklenen zeminle 1 mm içinde (sınama sınırı 5 cm), başlıktaki revizyon ve
  tarih Poppler `pdftotext` ile.
- Bir parselin revizyonu yalnız onu gösteren sayfaları yeniden üretir; öbürleri baytı baytına ve önceki tarihiyle
  kopyalanır; önceki teslim değişmez.
- Kaynak dökümünün maliyeti (ilk sürüm her nesneyi `cad.sheets.set`'in değişen nesne listesine vererek 100 000 parselde
  hata ayıklama derlemesinde 80,7 s sürdü: o liste birkaç nesne için, sıralaması ve tekrar denetimi karesel): görünüm
  penceresi başına nesnelerin üzerinden tek geçiş. 100 000 parsel, dört pafta, 134 104 kaynak: döküm 0,34 s, 100 000
  nesne özeti 2,5 s (hata ayıklama derlemesi, M5; release ölçülmedi). Döküm yalnız `execute`'ta yazılır; `plan` ve
  denetim onu kurmaz. `file.publish.status` her nesnenin özetini bir kez hesaplar.

### 7. Kalan işler

- Atlas sayfaları (zemin bilinmeyen pencere) için sayfa başına kaynak dökümü ve atlas nesnesinin kimliği.
- Tek PDF teslimin güncellemesi (değişmeyen sayfaları önceki PDF'ten nesne düzeyinde taşımak).
- Devam ettirilebilir (`resumable`) güncelleme yayını.
- Anotasyon/tabloların kalıcı kaynak kimliği ve revizyonuna bağlanması (kot, ad, eğri kotu, kenar/alan, istasyon), CAD
  gerçek boylu yazı ile GIS dinamik etiketin ayrılması, datum/epok/kalite notları ve dış kurum eksen adları eşlemesi,
  resmî profil kapısı: MAP-18'in öbür maddeleri.
- Web (ADR 0228): yeni alanlar `PublishSheets.ts`, `SheetsPublished.ts`, `SheetPublishStatus.ts`,
  `SheetSourceChanges.ts`, `PreviousPublication.ts` üretildi; web'in yayın penceresi ve ortak durum koşucusu TODOS §0.2.
