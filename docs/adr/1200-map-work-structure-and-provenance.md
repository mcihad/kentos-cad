# 1200 — KentOS Harita: iş yapısı, kaynak ve türetme kaydı

Tarih: 2026-11-06. plan/map-cad.md MAP-01. Durum: taslak; ilk dilim aşağıda, kalanlar “Sonra”da.

## Dayanak

- **Kaynaktan türeyen nesnenin kaydı:** Civil 3D'de veri kısayolu (data shortcut) paylaşılabilir bir kaynak nesneye
  giden yoldur; tüketen çizimdeki veri başvurusu (data reference) kaynağın salt okunur kopyasıdır, geometrisi orada
  değişmez; kaynak değişince başvuru çizim açılırken ya da elle eşlenir (Synchronize), ona dayanan nesneler yalnız
  gerçek bir veri değişikliğinde “Out Of Date” olur; başvuru bağımsız nesneye yükseltilebilir (promote), o zaman
  kaynağını izlemez ([About Data Shortcuts](https://help.autodesk.com/cloudhelp/2025/ENG/Civil3D-UserGuide/files/GUID-E91D2116-0F01-4A6B-B9C6-A47ED56F0F5F.htm),
  [To Synchronize a Reference Object](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-B079A088-6594-4095-A50B-7A8FC5770C5C.htm),
  6 Kasım 2026'da okundu). Proje öğesinin durum simgeleri yerel kopyanın kaynağa göre güncel, eski ya da yeni olduğunu
  ve “paylaşılan nesnenin kaynağı güncel değil” durumunu ayrı gösterir
  ([About Project Item State Icons](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-FC6D5F65-B796-42D4-BE54-DB9B3D63EEAB.htm)).
- **Taşınan kaynağı tek yerden bağlamak:** ArcGIS Pro'da kaynağı taşınmış katman çizilmez, İçindekiler'de kırmızı ünlem
  taşır; ünleme basıp kaynağı değiştirmek aynı yerde kaynağı bulunan bütün kırık katmanları da onarır; projenin
  bütün kaynakları Update data sources ile birlikte güncellenir; yerine konan kaynağın geometri türü aynı olmalıdır
  ([Repair broken data sources for layers and tables](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/repair-broken-data-links.html)).
  QGIS açılışta bulamadığı katmanları “Handle Unavailable Layers” penceresinde toplar; katman ağacındaki göstergesi
  “Click to set a new data source” der ([QgsProjectBadLayerHandler](https://qgis.org/pyqgis/latest/core/QgsProjectBadLayerHandler.html)).
  AutoCAD'in Reference Manager'ı birçok dış başvurunun kayıtlı yolunu bul-değiştir ile birlikte değiştirir, ama geri
  alınamaz ([To Find and Replace Paths of References](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT/files/GUID-1E07BA3C-8C5F-465F-B540-D5B4BB8CAB8B.htm)).
- **Kaynak, etkinlik, türetme:** W3C PROV-DM (W3C Recommendation, 30 Nisan 2013) varlığı (entity), etkinliği (activity)
  ve türetmeyi (wasDerivedFrom: bir varlığın başkasına dönüşmesi ya da ondan kurulması) ayırır; türetme isteğe bağlı
  olarak etkinliği ve kullanımı/üretimi adlandırır; revizyon (wasRevisionOf) türetmenin özel hâlidir
  ([PROV-DM](https://www.w3.org/TR/prov-dm/)).
- **Netcad ve OpenCities Map:** plan/map-cad.md'nin S01 ve S10 incelemesi: uzmanlık akışları ve şemalı CAD+CBS verisi
  tek bir kalabalık şeridin anlatamayacağı iş yapısını gösterir. Netcad'in teknik wiki'si bu oturumda da okunmadı;
  bir Netcad komutunun parametresi buraya yazılmadı.
- **KentOS'ta bugün:** dış başvurunun (`xref.rs`) ve bağlı GeoPackage katmanının (`sources.rs`) kendi yolu ve SHA-256
  özeti vardır; raster ve resim nesnesi gösterdiği dosyanın yolunu taşır (`RasterFields.file`, `ImageFields.file`);
  bölme ve birleştirme soy kaydı tutar ([ADR 0598](0598-lineage-and-undoing-a-written-step.md)); Yol projesinin ağacı çizimlerin kaynaklarını ve
  eskimesini gösterir ([ADR 0911](0911-road-project-tree-views-and-rebuild.md)). Harita projesinde ise bir ölçüm
  kampanyası, ortofoto ya da CAD altlığı proje öğesi değildir; bir noktanın hangi kayıttan, hangi revizyondan ve
  hangi işlemle geldiği kaydedilmez, kaynak taşınınca her kaydın yolu ayrı ayrı düzeltilir.

## Karar

1. **İş yapısı proje ayarıdır, yeni nesne türü değildir.** Harita projesinin `settings.mapWork`'ü (`MapWork`) iki
   liste taşır:
   - *Öğeler* (`is-N`): türü (`campaign` ölçüm kampanyası, `reference` referans kaynak, `working` çalışma verisi,
     `design` tasarım, `check` denetim sonucu, `delivery` teslim paketi), adı (1–200 karakter), durumu, katmanları ve,
     projenin dayandığı bir dosyaysa dosyası: mutlak yol ve projenin son kabul ettiği içeriğin SHA-256'sı (öğenin
     **revizyonu**). Mevcut Katmanlar, Kaynaklar ve İşlemler yerinde kalır; öğe onları adlandırır, yerlerine geçmez.
   - *Türetme kayıtları* (`tr-N`): nesnelerin kalıcı kimlikleri, geldikleri öğe (dosyası olan), o öğenin o anki
     revizyonu ve onları üreten işlem (komutun kimliği ya da adımın adı). PROV'un wasDerivedFrom'u: varlık nesne,
     kaynak öğe, etkinlik işlem.
   - **Ne nerede tutulur:** *anlam katman profilindedir* — nesne, katmanını ya da üstündeki en yakın grubu adlandıran
     öğenindir; bir katman tek öğenin olur. Kampanya ve referans öğesinin katmanındaki nesne **kaynaktır**; türetme
     kaydının adlandırdığı nesne **türetilmiştir** (kaydı katmanından önce gelir); geri kalanı kullanıcının
     **bağımsız çizimidir** (bir tasarım ya da çalışma öğesinin katmanında olsa da). *İlişki türetme kaydındadır*;
     *sözleşme* iki listenin kurallarıdır (kimlikler, bir katman tek öğede, bir nesne tek kayıtta, kayıt dosyası olan
     öğeden). Böylece nokta, çizgi, raster gibi var olan nesneler değişmeden kalır; Mimari ve CBS'nin nesneleri ve
     yüzeyleri dokunulmadan kalır.
   - *Durum* `draft` (taslak), `review` (denetlenecek), `approved` (onaylı teslim revizyonu). Onay ofisin kendi teslim
     kaydıdır; kurum (TKGM, belediye) onayı değildir, KentOS'ta kurum onayı yoktur; hiçbir denetim durumu
     kendiliğinden değiştirmez. Katmanlarında güncel olmayan türetilmiş nesne olan öğe onaylanmaz (`approval_stale`);
     onaydan sonra kaynağı değişen öğe `stale` görünür.
2. **Kalıcılık: `.kcad` şema 188** (geçici numara; birleşmede yeniden numaralanır). `mapWork` ayarların 7 harfli
   anahtarları arasında `mapSrid`'den sonra, JSON olarak yazılır; yalnız `cadWork: map` projesinin olur; boş yapı,
   `draft` durumu ve boş listeler yazılmaz; okuyucu kuralları ve yazıcının biçimini denetler. Bağımsız okuyucu
   `tools/kcad/kcad.py` ve yazıcı `scripts/fixtures/kcad_v2_reference.py` aynı kuralla: `map-work.kcad`, şema 186'da
   (187 başka dalın) reddedilen, mimari projede, iki öğede katman, dosyasız öğeden türetme, iki kayıtta nesne ve yazılmış
   `draft` örnekleri.
3. **Komutlar.** `map.work.query` (salt okunur; salt okunur kullanıcı da okur): öğeler (nesne sayısı, dosyası ve
   durumu: `found`/`missing`; `deep` ile SHA-256 okunarak `current`/`changed`/`missing`), türetmeler (`current`,
   `stale`: kaynağın revizyonu kaydınkinden başka — derin okumada dosyanın bugünkü içeriği —, `missing`: nesnesi
   silindi) ve `uid` ile bir nesnenin rolü (`source`, `derived`, `free`), öğesi, kaynağı, revizyonu, işlemi ve
   güncelliği. `map.work.edit` her işlemi tek geri alma adımıyla: `add`, `set` (ad, durum, katmanlar), `remove` (öğe ve
   ondan türetme kayıtları; nesneler kalır), `relink`, `refresh`, `derive`, `forget`.
   - **Tek yerden yeniden bağlama** (`relink`): öğenin yolu ve öğenin katmanlarında eski dosyayı gösteren raster ve
     resimlerin yolu aynı adımda yeni dosyaya geçer (ArcGIS Pro'nun aynı yerdeki kırık katmanları birlikte onarması
     gibi, ama geri alınabilir — Reference Manager'ın tersine). İçerik aynıysa türevler güncel kalır; farklıysa yeni
     içerik öğenin revizyonu olur ve ondan türetilenler güncel değildir (uyarı `source_content_changed`). Kilitli katmandaki
     raster ya da resim varsa hiçbiri değişmez (`layer_locked`). Dış başvurunun (`xref`) ve bağlı GeoPackage
     katmanının kendi yolu kendi komutundadır (Dış başvurular, Kaynağa bağla › Yeniden bağla); bu dilim onları
     değiştirmez, aynı veriye iki yazılabilir yol otoritesi kurulmaz (CLAUDE.md §15).
   - `refresh`: dosyanın bugünkü içeriği kabul edilir. Kaynağın dosyası diskte değişince proje bunu kendiliğinden
     kabul etmez: derin okuma `changed` der, türevler güncel değildir; kullanıcı Yenile ile kabul eder.
   - `derive`: nesneleri üreten işlem kaydı yazar. Bu dilimde içe aktarma ve çizim komutları kaydı kendileri yazmaz;
     kullanıcı (ya da Python/MCP) açık komutla yazar. Komutların kendi kaydı “Sonra”dadır.
4. **Masaüstü.** Yalnız KentOS Harita'da (ve hepsini gösteren kentos-cad'de) sağ dokta **Harita işi** paneli (DESIGN.md
   durum ağacı, Yol projesi panelinin kalıbı): altı başlık (Ölçüm kampanyaları, Referans kaynaklar, Çalışma verisi,
   Tasarımlar, Denetim sonuçları, Teslim paketleri), öğenin durum hapı (Taslak, Denetlenecek, Onaylı teslim revizyonu;
   sorunlar: Dosyası yok, Dosyası değişti, Katmanı yok, Onaydan sonra kaynağı değişti), altında türetmeleri (Güncel,
   Güncel değil, Nesnesi eksik). Öğenin satırı katmanlarını Katmanlar'da seçer; türetmenin satırı nesnelerini seçip
   gösterir; Dosyaları oku derin okur. Öznitelikler'de tek nesne seçilince **Kaynağı** bölümü: rolü, öğesi, kaynağı,
   revizyonun ilk 12 hanesi, üretim işlemi ve güncelliği. Mimari'nin ve GIS'in panelleri değişmez.
5. **Otomasyon.** Python `kentos.cad` ve MCP aynı katalogdan; `map.work.edit` MCP'nin yıkıcı listesindedir (remove,
   forget ve relink var olanı değiştirir). Web dururken (ADR 0228) web'in paneli TODOS.md §0.2'dedir.

## Aşamalar

MAP-01.1 (bu dilim): §1–§5. Ortak durumlar `scripts/fixtures/map_work_cases.py` (bağımsız; SHA-256 hashlib ile, kurallar
ve cümleler bu ADR'den): `map.work.query` 22, `map.work.edit` 40. `.kcad` şema 188 örnekleri bağımsız okuyucu ve yazıcıyla. Masaüstü testi (`map_work_panel_tests.rs`): aynı projede GNSS kampanyası, iki ortofoto, CAD altlığı ve tasarım yüzeyi; seçilen her nesnenin Kaynağı satırları; kampanya panelden yeni dosyaya bağlanınca iki türetme “Güncel değil”, geri alma güncel getirir; panelden öğe ekleme, denetime gönderme ve onay; `.kcad`'e yazılıp yeniden açılınca aynı öğeler, türetmeler ve nesne kaynağı, okuma revizyonu ve kirli durumu değiştirmez; Mimari, GIS ve Yol'da panel ve satırlar yok. Resimler açık geniş ve koyu dar (`.run/gui-evidence/harita-isi-{guncel,kaynak-degisti}-*`).

## Sonra

- İçe aktarma (GNSS, nokta dosyası, raster, dış başvuru) ve çizim komutlarının türetme kaydını kendilerinin yazması;
  türetilmiş nesnenin elle değişmesinin (Yol'daki `edited` gibi nesne damgasıyla) görülmesi; yeniden üretme.
- Dış başvuru ve bağlı kaynağın öğeyle aynı yerden yeniden bağlanması (onların komutları üzerinden, tek adımda).
- Dosyasız öğelerin (tasarım, teslim) revizyonu ve onaylı teslim revizyonunun içerik damgası (MAP-18'in teslimiyle).
- Seçime göre bağlamsal araçlar, tipli ölçüm/plan/varlık katman kuralları (MAP-01'in üçüncü ve dördüncü alt maddeleri).
- Göreli yol (proje klasörüyle taşınan veri).
