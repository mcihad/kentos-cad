# 1201 — Harita işi: taşınan proje, tek yerden yeniden bağlama ve seçime göre araçlar

Tarih: 2026-11-06. plan/map-cad.md MAP-01 (ikinci dilim). Durum: taslak. Önceki dilim [ADR 1200](1200-map-work-structure-and-provenance.md).

## Dayanak

- **Göreli yol varsayılandır.** AutoCAD'in `REFPATHTYPE` değişkeni dış başvurunun ilk eklendiği anda tam, göreli ya da
  yolsuz saklanacağını seçer; 2018'den beri varsayılan göreli yoldur, başka sürücüdeki dosya göreli saklanamaz
  ([REFPATHTYPE](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-LT/files/GUID-CCC90130-7575-4901-8905-FBB4553DAA3F.htm),
  [AutoCAD 2018 Xref Enhancements](https://www.autodesk.com/blogs/autocad/autocad-2018-xref-enhancements/), 6 Kasım 2026'da
  okundu). QGIS'in proje özelliklerinde “Save paths” göreli/mutlak seçimi vardır; proje ile veri ortak bir üst klasörde
  değilse QGIS mutlak yola döner ([QGIS issue 23580](https://github.com/qgis/QGIS/issues/23580),
  [bug 8589](https://issues.qgis.org/issues/8589)). ArcGIS Pro kaynakları göreli saklar; veri ile `.aprx` aynı sürücüde
  değilse mutlak yol kalır (Esri Community tartışması,
  [how can i store relative pathnames in arcgis pro](https://community.esri.com/discussion/453649/how-can-i-store-relative-pathnames-in-arcgis-pro)).
  Bunlar topluluk ve hata kayıtlarıdır, resmî belge değildir; ortak davranışı (göreli saklama, başka kökte mutlak yol)
  KentOS zaten bağlı GeoPackage ve dış başvuru için uygular ([ADR 0349](0349-linked-files-status-and-search-paths.md),
  [ADR 0506](0506-relative-sources-and-package.md)). Harita işinin dosyası ([ADR 1200](1200-map-work-structure-and-provenance.md))
  yalnız mutlak yol taşıyordu: klasörüyle taşınan ya da Paketle'yle gönderilen projede bütün kampanya ve ortofoto öğeleri
  “Dosyası yok” oluyordu. Bu bir hataydı.
- **Kırık bağı tek yerden onarmak.** ArcGIS Pro'da kaynağı taşınmış katmanın kaynağı değiştirilince aynı yerdeki bütün
  kırık katmanlar da onarılır ([Repair broken data sources](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/repair-broken-data-links.html)).
  ADR 1200'ün `relink`'i öğenin katmanlarındaki raster ve resimleri götürüyordu; aynı dosyayı gösteren dış başvuru ise
  eski yolda kalıyordu (örnek: “Kadastro CAD altlığı” öğesinin dosyası hem öğe hem dış başvuru).
- **Seçime göre araç ve bağımsız yapmak.** Civil 3D'de desteklenen bir nesne seçilince o nesnenin bağlamsal sekmesi açılır;
  sekmenin “Launch Pad” paneli iş akışında sıradaki komutları verir; farklı türler birlikte seçilince “Multiple” sekmesi
  açılır ([About the Autodesk Civil 3D Ribbon](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-CEC80555-4DB7-4841-B63A-1F1E0743F270.htm)).
  Veri başvurusu bağımsız nesneye yükseltilebilir (promote), o zaman kaynağını izlemez (ADR 1200 §Dayanak'taki Data
  Shortcuts kaynağı). KentOS'ta bağlamsal sekmeler nesne türüne göre zaten açılır ([ADR 0622](0622-interface-arrangement.md)
  §4); harita işindeki rolü (kaynak, türetilmiş, bağımsız) şeritte görünmüyordu ve türetilmiş bir nesneyi kaydından
  çıkarmanın tek yolu bütün kaydı (`forget`) silmekti.
- **Netcad:** Netcad'in tabaka ve bağlamsal menü davranışını anlatan resmî bir belge bu oturumda da bulunamadı (arama
  sonuçları yalnız ikincil kaynak ve kısayol listeleri verdi). Netcad'e dayanan bir karar yazılmadı.

## Karar

1. **Harita işi dosyasının göreli yolu (`.kcad` şema 193).** `WorkFile` `relative`'i taşır: kaydedilirken proje dosyasının
   klasöründen yol (`/` ayırıcı, `..` yalnız başta; başka kökte yazılmaz), bağlı GeoPackage'ın `relative`'iyle aynı kural
   (`relative_problem`). Kaydet `with_relative` ile yazar (çizim değişmez). Açılışta yolunda olmayan dosya göreli yolunda
   aranır; içeriği öğenin revizyonuysa öğenin yolu oraya geçer (geri alma adımı değil, proje ayarı), başka içerik varsa
   alınmaz ve söylenir (“kabul edilen içerik değil; bağlanmadı”). `map.work.query`'nin yeni `drawing`'i (açık projenin
   dosyası, mutlak) verilince dosya yolunda yoksa göreli yoluyla, sonra projenin arama yollarında (`xrefPaths`; önce
   göreli yolu, sonra adı) aranır. Dosya satırı `via` (`path`, `relative`, `searchPath`, `none`), başka yerde bulunduysa
   `at` ve kayıtlı `relative`'i söyler; derin okuma bulunan dosyanın SHA-256'sını karşılaştırır. Mutlak olmayan `drawing`
   `invalid_file_path`'tir. Yazıcı 193'ü bir öğenin dosyası `relative` taşıyınca seçer; 192 ve öncesi alanı
   `unknown_field` sayar.
2. **Paketle ve bağlı dosyalar.** Paketle (`file.package`, ADR 0506) harita işi dosyalarını `veri/`'ye kopyalar
   (`PackagedKind::workFile`), bağı kopyaya çevirir ve göreli yolunu yazar; bir rasterin de gösterdiği dosya bir kez
   kopyalanır; bulunamayan dosya `left` (missing) olur. `cad.assets.check` onları `workFile` türüyle listeler (`source`
   öğenin kimliği, `name` öğenin adı); Dış başvurular penceresinin diğer bağlar listesi “Harita işi dosyası” der.
3. **Tek yerden yeniden bağlama dış başvuruları da götürür.** `relink` aynı adımda, kaynağı eski dosya olan (yolu aynı
   ya da diskte aynı dosya) üst düzey dış başvurunun yolunu da yeni dosyaya çevirir, son yüklediği içerik (`digest`) yeni
   dosyanınkiyse; içerik yeniden okunmaz, tanım değişmez (çıktıda `links`). Başka içerik yüklemiş dış başvuruya ve eski
   dosyaya bağlı GeoPackage katmanına dokunmaz: dış başvurunun içeriğini değiştirmek kendi komutunun
   (Dış başvurular › Yolu değiştir) işidir; bağlı katmanın bağı proje ayarıdır, geri alma dışında kalır (ADR 0506) ve
   geri alınabilir bir adımın içine karıştırılmaz. Her biri için uyarı `link_not_followed` söylenir. Geri alma öğeyi,
   rasterleri ve dış başvuruları birlikte geri getirir.
4. **Bağımsız yap (`release`).** `map.work.edit`'in yeni işlemi verilen nesneleri kendi türetme kayıtlarından çıkarır;
   nesnesi kalmayan kayıt kalkar (`released`), çıkan nesne sayısı `freed`. Kayıtta olmayan nesne `not_derived`
   (çizimden silinmiş ama kaydı adlandırdığı nesne bırakılabilir). Tek geri alma adımı “Bağımsız yap”.
5. **Seçime göre bağlamsal araçlar (yalnız KentOS Harita).** Seçim, Nokta, Çizgi, Alan ve Raster bağlamsal sekmelerinde,
   yalnız harita işinde (`works: ["map"]`) **Harita işi** paneli: **Kaynağını göster** (`map.workSource`, ikon
   `workSource`: kaynak sayfasına kesik çizgiyle bağlı nokta) seçili nesnenin rolünü, öğesini, türetmesini, işlemini ve
   güncelliğini söyler, öğenin katmanını Katmanlar'da seçer, Harita işi panelini öne getirir; **Bağımsız yap**
   (`map.workRelease`, ikon `workRelease`: ayrılmış zincir halkası) seçimdeki türetilmiş nesneleri bırakır. Seçimde
   harita işine ait nesne yokken düğmeler kapalıdır (`App::available`); seçim yokken sekme zaten kapalıdır. Harita işi
   panelinde seçim varken **Seçimi türet** satırı: dosyası olan öğelerden biri seçilir, seçili nesneler “Elle türetme
   kaydı” işlemiyle o öğeden türetilmiş diye kaydedilir (içe aktarmanın kendi kaydını yazması ayrı karardır, ADR 1200
   §Sonra).
6. **CAD / CBS işaretleri.** Bu dilimin bütün yüzeyi **yalnız Harita (CAD)**'dır: harita işi yalnız `cadWork: map`
   projesinin ayarıdır. Komutlar `ribbon.json`'da yalnız harita işinin bağlamsal panellerinde durduğu için KentOS GIS,
   Mimari ve Yol onları sunmaz (`App::offers`); Komut ara ve komut satırı orada “KentOS Harita'da var” der. **Ortak
   (Harita + CBS):** göreli yol kuralı (`relative_problem`), arama (`links::find`), Paketle ve `cad.assets.check`
   CBS projesinin bağlı dosyalarında da aynı koddur; CBS'ye harita işi taşınmadı.

7. **Adlar ve panelin düzeni (sahibin 6 Kasım düzeltmesi).** “Kampanya” Türkçe harita yazılımlarında ve arazi
   yönergelerinde kullanılmıyor; Netcad'in Hesap modülü ve üniversite arazi yönergeleri “arazi ölçümü/ölçü verisi” ve
   “detay alımı” der (Netcad Hesap ürün sayfası, KTÜN topoğrafik harita yapımı arazi föyü,
   [AKÜ arazi yönergesi](https://harita.aku.edu.tr/wp-content/uploads/sites/6/2022/07/2021-2022_Arazi_Yonergesi.pdf)).
   Arayüzdeki adlar: `campaign` **Arazi ölçümü** (başlık “Arazi ölçümleri”; örnek öğe “GNSS ölçümü 2026-10”),
   `reference` **Altlık** (“Altlıklar”), `working` Çalışma verisi, `design` Tasarım, `check` **Denetim**, `delivery`
   **Teslim**. Sözleşmenin İngilizce tanımlayıcıları değişmez. Panelin üstü dört satırlık düğme yığını yerine tek araç
   satırıdır: tür, ad, sonra yazısı ipucunda olan simge düğmeleri — Katmanla ekle (`Layers`), Dosyayla ekle (`Folder`),
   Dosyaları oku (`Hash`, açık/kapalı); seçimde türetilmemiş nesne varsa altında tek satır: “N nesnenin kaynağı: [öğe]”
   ve Seçimi türet (`Link`).

## Yol üstünde düzeltilenler

- Python ortak durum koşucusu kurulumdaki `$tmp:` yollarını çevirmiyordu: `map.work.*`'ın 62 durumundan 21'i Python'da
  düşüyordu (ADR 1200 teslim edildiğinde de). `python/tests/test_command_cases.py` kurulumu masaüstü koşucusu gibi
  çevirir; harita işi, standart istisnaları ve bağlı dosyalar 130 durumda geçer.
- `cad.standards.exceptions` `invalid_standard`'ı anıyordu ama hiçbir durum vermiyordu; `road.profiles.draw`'ın durumu
  `unknown_superelevation` veriyordu ama katalog anmıyordu (yetenek testi kırıktı). Birine durum eklendi, öbürü kataloğa.
- KCAD kurtarma okuyucusunda iki `expect` (kullanıcı dosyasında panik yolu) hata dönüşüne çevrildi; geometri, yol ve stil
  çekirdeğinde clippy'nin reddettiği `f64::min/max` (NaN kuralı), gereksiz `drop`, karmaşık tipler ve `loop` düzeltildi.
  Uygulama crate'inin eski clippy borcu (≈30 bulgu, başka işlerden) ayrı iştir.

## Bütçe

`map.work.query` hızlı okumada her dosyaya en çok bir yol + bir göreli yol + arama yolları kadar `is_file` bakar (bin öğe
için ölçülmedi; panel ağacı yalnız revizyon ya da derinlik değişince okur). Açılışta yalnız yolunda olmayan dosyanın
göreli yolundaki dosyası akışla (bütün dosyayı belleğe almadan) SHA-256'lanır. `map.workSource`/`Release`'in açık/kapalı
kararı seçimin en çok 10 000 nesnesinde hesaplanır; daha büyük seçimde düğmeler kapalıdır.

## Kanıt

- Ortak durumlar `scripts/fixtures/map_work_cases.py` (bağımsız; dosya yolu çözümü ve SHA-256 betikte): `map.work.query`
  22 → 27 (göreli yol, `../ortak`, arama yolunda ad, göreli yolda başka içerik, mutlak olmayan `drawing`; her dosya
  satırında `via`), `map.work.edit` 40 → 48 (bağımsız yap: kısmi, bütün kayıt, iki kayıt, kendi çizimi, kaynak nokta,
  boş, geçersiz kimlik; dış başvuruyu götüren yeniden bağlama ve uyarısı, geri alma).
- `.kcad` şema 193: `map-work-relative.kcad` (biri klasör içinde, biri `../ortak`), `broken/map-work-relative-in-schema-192`,
  `-absolute`, `-climbs`, `-empty`; bağımsız okuyucu `tools/kcad/kcad.py` ve yazıcı `scripts/fixtures/kcad_v2_reference.py`.
- Masaüstü `map_work_tools_tests.rs`: bağlamsal sekmenin Harita işi paneli rolle açılıp kapanır (türetilmiş nokta,
  kaynak çizgi, kullanıcının çizgisi, raster, karışık seçim), Kaynağını göster paneli ve katmanı getirir, Bağımsız yap tek
  adım ve geri alınır, Seçimi türet `tr-3` yazar; GIS, Mimari ve Yol komutları sunmaz; klasörüyle taşınan proje
  kampanyasını göreli yolundan bulur (kaydet → taşı → aç). Resimler `.run/gui-evidence/map-work-1201/` (açık geniş,
  koyu dar: Nokta sekmesi ve Kaynağı, Bağımsız yap sonrası, Seçimi türet satırı).

## Otomasyon

Python (`kentos.cad`, katalogdan üretilir) ve MCP (`map.work.edit` yıkıcı listede) aynı işleyiciyi çağırır:

```python
from kentos import cad
doc = cad.Document.open("/is/proje/ana.kcad")
tree = cad.map.work.query(doc, drawing="/is/proje/ana.kcad", deep=True)
for item in tree.items:
    if item.file:
        print(item.name, item.file.via, item.file.at)   # "relative", "/is/proje/veri/kampanya.nmea"
cad.map.work.edit(doc, operation={"op": "release", "uids": ["0192f5c0-1200-7000-8000-000000000002"]})
```

MCP: `{"tool": "map.work.edit", "arguments": {"operation": {"op": "relink", "item": "is-3", "file": "/arsiv/kadastro.dxf"}}}`
cevabında `links` (yeni dosyaya geçen dış başvuru sayısı) ve gerekiyorsa `link_not_followed` uyarısı.

## Sonra

- İçe aktarma (GNSS, nokta dosyası, raster, dış başvuru) komutlarının türetme kaydını kendisinin yazması (sahibin
  kararı bekliyor: kendiliğinden mi, pencerede seçimle mi).
- Bağlı GeoPackage katmanının bağını geri alınabilir yapıp yeniden bağlamaya katmak (ADR 0506'nın sınırı).
- Dosyasız öğelerin revizyonu ve teslim damgası (MAP-18), tipli katman kuralları (MAP-01 dördüncü alt madde: katmanın
  türü ve alan kuralları; katman türü ADR 0622 §5'te var, öğe türüne göre profil yok).
