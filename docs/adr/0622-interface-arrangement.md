# 0622 — Arayüz düzeni: CAD ile CBS'nin ayrılması, şerit ve bağlamsal sekmeler

Tarih: 23 Ekim 2026. Dal: `ux/arayuz-duzeni`. Kapsam: masaüstü (web ADR 0228 gereği bekler;
web'de gerekenler TODOS.md §0.2'de).

## Sahibin isteği

Sahip (23 Ekim) arayüzü "dağınık ve mantıksız" buldu ve şunları istedi:

- CAD ile CBS'nin ayrımı net olsun. Ürünün asıl amacı Netcad tarzında imar planı ve benzeri
  planlar çizen bir CAD; AutoCAD'in işlerini de yapar. CBS ayrı bir türdür. Uygulamanın
  içinden ikisi arasında geçiş olmaz; geçiş ileride yalnız dönüştürmeyle olur.
- Şeritteki çok sayıda araç ve menü değerli, toplu gruplarda olsun. Yüzlerce yazısız küçük
  ikon yeni kullanıcıya bir şey anlatmaz.
- CBS'de seçim ve katman türü CBS mantığıyla çalışsın:
  - Projede "otomatik seçim" açıksa bugünkü seçim (tıkla, küçük menüyle sıradakini seç) kalır.
    Kapalıysa yalnız etkin katmanın nesneleri seçilir.
  - Her katmanın türü olur ve katmana yalnız o türden nesne çizilir. Çizgi katmanı etkinken
    alan araçları kapalıdır.
- Sahnede bir nesne seçilince şeritte türüne göre bir sekme açılır. Örnekler: çizgi için
  "Çizgi", alan için "Alan". Nokta bulutu ve rasterin işlem araçları kendi sekmelerinde gelir;
  aynı araçlar sağdaki İşlemler'de de kalır. İşlem araçları ana sekmelere konmaz.
- Sağ tık AutoCAD'deki gibi o anki işe göre davranır. Tutamakla taşırken sağ tık noktayı
  bırakmaz.
- Ayarlarda açılan bir seçenekle her işlemin Python kodu konsola yazılır; kapalıysa yazılmaz.
  Her aracın Python ve MCP karşılığı tek tek gözden geçirilir.
- Yeni projede ölçü ve yazı ölçekleri projenin birimine ve ölçeğine göre ayarlanır. Bugün
  "1:1000 metre" sanılan projede ölçüler 2,5 mm yazılıyor.

Netcad'in arayüzü wiki.netcad.com.tr'den araştırıldı (Netcad GIS 8.6):

- Şerit sekmeleri: Giriş (Çizim, Sorgu, Düzenleme, Görüntü), Düzenle (Düzenleme, Dönüşüm),
  Analiz, Araçlar, Detaylar (Dizi, Tarama, Ölçme, Röleve, Etiket, Çizim), Görünüm.
- İmar planı araçları ayrı modüllerdedir: Planet (Yol çiz, Kavşak, Refüj kapat, Semboller,
  Hesaplar) ve Netmap (Ada, Parsel, İfraz, Tevhit).
- Nokta alınırken bir "Nokta Seçim Araçları" sekmesi açılır.
- Proje ölçeği değişince sembol ve yazı boyları ölçeğe göre güncellenir.

## 1. Proje türü projeyle başlar ve değişmez

Türü olan proje türünü korur:

- `workspace.cad` ve `workspace.gis` kapalıdır; nedenini söylerler.
- Durum çubuğundaki tür hücresi yalnız türü gösterir, menüsü yoktur.
- Şeritteki "Proje türü" menüsü kalktı.
- Proje ayarları'nda tür salt okunurdur.

Türü sorulmamış eski proje (Hibrit ya da türsüz) türünü bir kez seçer: açılıştaki soruyla,
durum çubuğundaki hücreye basarak ya da Proje ayarları'nda. Komut satırı ve kısayollar
şeritte olmayan komutu yine çalıştırır (ADR 0165).

CAD ile CBS arasında dönüştürme sonraki bir iştir. Bir türün verisini öbür türün kurallarına
(katman türleri, eksen düzeni) çeviren açık bir komut olacak.

## 2. Yeni projede ölçek ve yazı yükseklikleri

Hesap doğruydu: `annotation_height = mm / 1000 × plotScale`. 1:1000 projede ölçü yazısı 2,5 m
çıkar. Sorun projenin 1:1000 olmamasıydı:

- Yerel CAD projesinin varsayılan ölçeği birimine bakmadan 1:1'di.
- Durum çubuğundaki "Ekran 1:N" kutusu projenin ölçeğini değil, ekranın yakınlığını
  değiştiriyordu.

Karar:

- Metrelik CAD çizimi bir plandır (`kentos_project::wizard::cad_scales`). Ölçekleri 1:100 ile
  1:25.000 arasıdır (`PLAN_SCALES`), varsayılanı 1:1000'dir. Koordinat sistemli CAD projesi de
  metrede olduğu için aynı listeyi alır.
- Milimetre, inç ve ayak projeleri 1:1, santimetre projesi 1:50 ile başlar; bunlar teknik
  çizim ölçeklerini alır.
- Seçilmiş ölçek yeni birimin listesinde yoksa birimin varsayılanına döner.
- Sihirbazın Ölçek ve ayrıntılar sayfasında "Yazı yükseklikleri" var: yazı ve ölçünün kâğıttaki
  boyu (mm) ve seçilen ölçekte çizimdeki karşılığı (`drawn_height_text`).
- Özet "Yazılar" satırında aynı şeyi söyler. Yazılan yükseklikler `NewProject.annotation` ile
  projeye yazılır (ADR 0205 §1).
- CAD projesinin durum çubuğunda ayrı bir "Ölçek 1:N" hücresi vardır (AutoCAD'in açıklama
  ölçeği). Projenin çizim ölçeğini gösterir ve oradan değiştirilir; değişince genel
  yükseklikteki açıklamalar izler (ADR 0205 §3).
- "Ekran 1:N" hücresi görünümün yakınlığı olarak kalır.

Ortak durumlar `fixtures/project/v1/wizard.json`'dadır: "Yazılar" satırı ve "CAD, yerel, m"
durumu. Şeridin ölçek listesi birimiyle birlikte `fixtures/shell/v1/ribbon.json`'dan okunur.

## 3. Şerit

Masaüstünün şeridi web'in donmuş envanterinden değil, kendi dosyasından okunur: `catalog/v1/ribbon.json`
(`kentos.ribbon` 1). Dosyayı `scripts/ui/ribbon.py` yazar; `--check` dosyanın güncel olduğunu denetler.

Dosyanın yapısı:

- `tabs`: her proje türünün sekmeleri.
- Sekmede paneller, panelde öğeler. Öğe bir komut, bölünmüş düğme, menü ya da `builtin` (Katmanlar, Özellikler, Şablonlar)
  olabilir; panelde ayrıca `overflow` (panel adının ▾ listesi) ve `launcher` bulunabilir.
- Bölünmüş düğme `{"split": "circle"}` biçiminde yazılır ve envanterdeki aynı anahtarlı düğmeyi yöntemleriyle alır. Yöntem
  ekleri (ADR 0287) yine eklenir.
- Komutların adı, ikonu ve açıklaması envanterden ve `additions.json`'dan gelir. `additions.json`'un `ribbon` yerleri web'in
  dönüşü için durur; masaüstü onları okumaz.

İlkeler:

- **Bir komut ana sekmelerde bir kez geçer.** İstisna: Giriş en sık işleri taşır, uzman sekmeler (Değiştir, Açıklama) tam
  takımı (AutoCAD'in Giriş ile Açıklama ilişkisi).
- **Düğmeler yazılıdır.** Büyük düğmede ikon ve ad, küçük düğmede ikon ve ad yan yanadır.
  - Her sekme 1440 px'te panellerini en çok bir adım indirerek gösterir. İndirilen panelde de düğmeler yazılıdır: büyük
    düğmeler küçülür, yalnız ikona inilmez. Test: `ribbon_tests::every_tab_fits_1100_pixels_without_scrolling`.
  - Seyrek araçlar panelin ▾ listesindedir.
- **İşlem araçları** (`processing.run.*`) ana sekmelerde yer almaz. Araç kutusunda (sağ dok, İşlemler) ve seçimin bağlamsal
  sekmelerinde bulunurlar.
  - İstisnalar: CBS'nin Seçim panelindeki İfadeyle seç ve Konuma göre seç. Bunlar seçimin kendisidir (ArcGIS Pro'nun Seçim
    grubu).
- Tema, vurgu, yazı tipi, yazı boyutu ve çizim zemini kartları şeritten kalktı; Uygulama ayarları › Görünüm'dedir. Görünüş
  panelinde Tema ▾ kalır.
- Özellikler panelindeki çizim ölçeği durum çubuğuna geçti (§2).
- Hiçbir şey kaybolmaz. `catalog::tests::the_ribbon_keeps_every_command_the_old_one_placed` testi web'in şeridinin ve eklerin
  yerleştirdiği, masaüstünde çalışan her komutun yeni şeritte bir yerde olduğunu denetler: sekme, bağlamsal sekme, bölünmüş
  düğme, menü, ▾ listesi, durum çubuğunun lambaları ya da ? menüsü. Aynı test işlem araçlarının araç kutusunda olduğunu da
  denetler.

**CAD** (Netcad'in Giriş, Düzenle, Detaylar ve Planet ile Netmap düzeni ve AutoCAD'in Home'u):

| Sekme | Paneller |
|---|---|
| Dosya | Proje, Kaydet, Çizimler, Bulut, Alışveriş (İçe aktar ▾, Dışa aktar ▾), Ayarlar |
| Giriş | Çizim, Değiştir, Açıklama, Katmanlar, Özellikler |
| Plan | Yol, Ada ve parsel, Alan işlemleri, Plan yazıları, Topoloji |
| Ölçme | Sorgu, Saha, Poligon ve alım, Kestirme ve dengeleme, Noktalar, Oturtma |
| Ekle | Blok, Şablonlar, Dış başvuru, Resim ve raster, Harita, Ekler, Adres |
| Açıklama | Yazı, Ölçü, Kılavuz, Tablo, Tarama, İşaretleme, Ölçek |
| Değiştir | Pano, Seçim, Dönüştür, Dizi, Kenar, Köşe, Nesne, Kot ve sıra |
| Mimari | Duvar, Açıklık, Mahal, Taşıyıcı, Merdiven, Görünüm, Etiket ve çizelge, Proje |
| Görünüm | Gezinme, Görünümler, Koordinatlar, Görünüş, Paneller |
| Yönet | Stiller, Standart, Temizlik, Veri, Otomasyon, Kısıtlar, GIS |
| Çıktı | Pafta, Yayınla |

**CBS** (ArcGIS Pro ve QGIS düzeni):

| Sekme | Paneller |
|---|---|
| Dosya | Proje, Kaydet, Çizimler, Bulut, Alışveriş, Ayarlar |
| Harita | Gezinme, Katman ekle (Veri ekle ▾, Altlık ▾), Seçim, Sorgu, Etiket, Koordinat sistemi |
| Düzenle | Oluştur, Şablonlar, Katmanlar, Değiştir, Alan işlemleri |
| Veri | Katman, Tablo, Form, Kaynak, Topoloji, Ekler, Standart, Stil |
| Analiz | İşlemler, Ağ analizi, Arazi, Zaman ve senaryo, Adres, Otomasyon |
| Ölçme | Saha, Poligon ve alım, Kestirme ve dengeleme, Noktalar, Oturtma |
| Görünüm | Görünümler, Koordinatlar, Görünüş, Paneller |
| Çıktı | Pafta, Yayınla, Lejant |

CBS'nin eski Raster sekmesi kalktı; araçları Raster bağlamsal sekmesindedir.

`sheet.new` (Yeni pafta) ve `sheet.fromTemplate` (Şablondan pafta) masaüstünde çalışır oldu. İkisi pafta kipinin kendi yeni
pafta ve şablon galerisi iletilerini gönderir.

## 4. Seçime göre bağlamsal sekme

Seçimin nesnelerinin türleri tek bir sekmenin `kinds`'ına sığıyorsa o sekme açılır (`Catalog::contextual_for`). Sığmıyorsa
`*` sekmesi, yani karışık seçimin "Seçim" sekmesi açılır. Sekmenin başlığı türün adı ve sayıdır: "Çizgi 1", "Alan 3".

| Sekme | Türler |
|---|---|
| Nokta | point |
| Çizgi | line, polyline, arc, spline, nurbs, xline, ray |
| Alan | polygon, circle, ellipse |
| Yazı | text |
| Ölçü | dimension |
| Kılavuz | leader |
| Tarama | hatch |
| Blok | insert |
| Tablo | table |
| Resim | image |
| Raster | raster |
| Nokta bulutu | pointcloud |
| Duvar (CAD) | wall |
| Açıklık (CAD) | opening |
| Mahal (CAD) | space |
| Mimari nesne (CAD) | grid, column, stair, ramp, tag, viewmark |

Sekmeler yalnız o türe yapılabilecek işleri ve işlem araçlarını taşır. Örnekler:

- Alan: Geometri ▾, Üret ▾, Genelleştir ▾, Sorgu ▾.
- Raster: Yüzey analizi, Hidroloji, Uzaklık ve maliyet, Hesap, İstatistik, Vektöre, Uygunluk, Uzaktan algılama, Çok
  boyutlu.
- Nokta bulutu: Zemin süzgeci, Seyrelt, Sınıfla, Kırp, Birleştir, Karola, Rasterleştir, Sınır çıkar.

Seçim yapılınca ya da değişince sekmesi öne gelir; seçim kalkınca önceki sekmeye dönülür (`follow_selection_tab`).
AutoCAD'in bağlamsal sekmeleri de böyledir. Kullanıcı tercihi `drafting.contextualTabs` (Uygulama ayarları › Görünüm)
kapatılırsa sekme yalnız gösterilir, öne gelmez. Sekme seçimin ve çizimin değişiklik sayılarıyla önbellekte tutulur; büyük
seçimde her karede yeniden taranmaz.

## 5. CBS'de katman türü ve otomatik seçim

### Katman türü

Katmanın türü (`LayerNode.geometry`, `.kcad` şema 102) neyi tuttuğunu söyler; QGIS'in ve ArcGIS'in katmanı gibi tek
türden nesne alır. Tür yoksa katman Karışık'tır, her türü alır; CAD katmanları böyledir. Grubun türü olmaz.

| Tür | Aldığı nesneler |
|---|---|
| Nokta | nokta |
| Çizgi | çizgi, çoklu çizgi, yay, eğri (NURBS dahil), daire, elips |
| Alan | alan, daire, elips |
| Açıklama | yazı, ölçü, kılavuz, tablo, etiket, tarama, blok, resim, görünüm işareti |

Kapalı eğri (daire, elips) hem çizgi hem alan katmanına uyar. Yardımcı çizgi ve ışın hiçbir türlü katmana girmez.

Kural çizimin kendi denetimindedir (`kentos_domain::integrity`). Nesneyi yeni yazan, katmanını değiştiren ya da türünü
değiştiren her yazma denetlenir: komut, araç, Python, MCP, yapıştırma ya da içe aktarma. Uymayan yazma
`layer_geometry_mismatch` ile reddedilir ve hiçbir şey yazılmaz. İleti katmanı ve türü söyler: "“Yol” bir çizgi katmanı;
alan bu katmana yazılmaz".

`cad.layers.edit`'e eklenenler:

- `geometry` işlemi katmanın türünü verir; `mixed` türü kaldırır. Adımı "Katman türü"dür ve geri alınır. Katmanda türe
  uymayan nesne varsa reddedilir; ileti türleri sayar ("“Yol” katmanında 1 çizgi, 1 blok var; alan katmanı olamaz").
  Gruba ve servis katmanına tür verilmez.
- `add`'in `geometry`'si yeni katmanın türüdür; grup tür almaz.
- Kopyasını oluştur türü korur.

Masaüstü (`layer_types.rs`):

- Katman ağacında türlü katmanın satırında türünün işareti katmanın renginde durur, QGIS'in sembolü gibi. Karışık
  katmanın satırında renk karesi kalır.
- Katmanın sağ tıkında Katman türü ▸ beş seçenek sunar. Uymayan seçim reddedilir ve nedeni söylenir.
- CBS projesinde Katmanlar panelinin Yeni katman düğmesi ve Veri › Yeni katman ▾ türü sorar. Komutlar
  `layer.newPoint`, `layer.newLine`, `layer.newArea`, `layer.newAnnotation`; Karışık katman `layer.new`'dur.
- Etkin katman türlüyken başka türden nesne yazan çizim araçları kapalıdır. İpuçları ve başlatılmaları nedenini söyler ve
  uyan katmanı adıyla önerir. Aracın ne yazdığı bir tablodadır (`tool_makes`). Kendi katmanına yazan araçlar tabloda
  yoktur (Parsel oluştur, Etiketleri yazıya çevir); onların yazmasını çizimin denetimi tutar.

### Otomatik seçim

Projenin ayarıdır (`ProjectSettings.autoSelect`, `.kcad` şema 102).

- Açıkken tıklama ve kutu bütün görünen katmanların nesnelerini seçer.
- Kapalıyken yalnız etkin katmanın nesnelerini seçer; CBS'de bir katman düzenlenirken olduğu gibi.
- Verilmemişse CBS projesinde kapalı, öbürlerinde açıktır. Türü henüz sorulmamış proje eskisi gibi seçer.

Kapalıyken seçim süzgecinin tuttuğu bütün adımlar yalnız etkin katmanın nesnelerini alır (`selectable::only_layer`):

- tıklama ve üzerine gelme;
- pencere ve kesişen kutu, Çokgenle, Çitle ve Daireyle seç;
- Tümünü seç ve Ters çevir;
- komutların nesne seçen adımları.

Seçim sorgusunun (`cad.entities.select`) `layers`'ı etkin katmandır. Dışarıda kalanlar söylenir ("başka katmandaki 12
nesne dışarıda kaldı"). Başka katmanın nesnesine tıklamak hiçbir şey seçmez; ileti nedenini ve ne yapılacağını söyler.

Nesneden katman okuyan adımlar seçim değildir; bütün görünen katmanlardan alır:

- Katmanı etkin yap, gizle, yalıt ve kilitle;
- Katmanı eşle'nin hedefi;
- araçların kenar seçen adımları (Buda'nın sınırları gibi).

Ayar `edit.autoSelect` ("Otomatik seçim") ile açılıp kapanır ve `cad.settings.set`'in `autoSelect`'iyle yazılır. Çizimi
kaydedilmemiş yapar ama geri alma adımı değildir. Yeri:

- CBS'de Harita › Seçim;
- CAD'de Değiştir › Seçim ▾;
- durum çubuğunda "Oto. seçim" hücresi;
- Proje ayarları › Genel › Seçim.

Ortak izlerde konusu seçim olmayan dört CBS sahnesi (`layer-by-object.kcad`, `feature-table.kcad`, `temporal.kcad`,
`line-parts.kcad`) ayarı açık yazar. Kuralın kendi izi `auto-select.json`'dur.

### Doğrulama

- `cad.layers.edit`'in 75 ve `cad.settings.set`'in 50 ortak durumu, katman türünün 12 ve otomatik seçimin 1 yeni durumuyla:
  `layer_tree_command_cases.py`, `settings_set_command_cases.py`. Hepsi masaüstünün komutlarıyla geçer.
- `.kcad` şema 102: `fixtures/kcad/v2/layer-geometry.kcad`. Bozuk örnekler: şema 101'de tür, grupta tür, bilinmeyen tür,
  `autoSelect` boolean değil. Bağımsız Python yazıcısı ve okuyucusu da bunları denetler.
- Rust testleri:
  - `kentos-native-application`'ın `layer_types`'ı: türlü katmana komutla yazma ve katmanı değiştirme reddi; uyan türler;
    türün verilmesi ve geri alınması.
  - Masaüstünün `layer_types` testi: aracın kapanması ve nedeni.
- Python: `python/tests/test_layer_types.py`.
- İz: `auto-select.json` üç çeşitte geçer.
- Resimler: `layer_type_scenes.rs` (`.run/shots/arac-katman-turu-*`, `arac-oto-secim-*`).

## 6. Sağ tık

Sahibin bildirdiği hata şuydu: tutamakla nokta taşırken sağ tık noktayı imlecin yerine bırakıyor ve taşımayı yapıyordu;
başka işlerde ise sağ tık işi iptal ediyordu. Kökü şu: hızlı sağ tık, onay alan her durumda Enter sayılıyordu. Seçim aracının
sıcak tutamağı da Enter'ı onay sayar ve tutamağı imlecin son yerine koyar.

Karar AutoCAD'in varsayılanıdır: sağ tık her zaman o anki işin menüsünü açar ve hiçbir zaman nokta koymaz, onaylamaz ya da iptal
etmez (`drawing_menus.rs`).

| Durum | Menü |
|---|---|
| Tutamaçla taşıma | Tutamaç menüsü: tutamaç kipleri istemin seçenekleriyle (Taşı, Döndür, Ölçekle, Aynala, Taban noktası, Kopya), Bırak (Esc), tek seferlik kenet. Onayla yoktur; tutamaç bir noktaya kadar yerinde kalır |
| Komut sürerken | Onayla / bitir, İptal, komutun seçenekleri, Nokta hesapla, tek seferlik kenet, Kilit, çizim yardımcıları |
| Seçim varken | Yinele, Taşı, Kopyala, Döndür, Ölçekle, Aynala, Sil, pano, Öne getir, Arkaya gönder, Benzerini seç, Seçimi kaldır, Seçime yakınlaştır, Koordinat listesi, Eylemler |
| Seçim yokken | Yinele, Geri al, Yinele (redo), Yapıştır, Tümünü göster, Pencere yakınlaştır, Kaydır, Önceki görünüm, Tümünü seç, Önceki seçim, Koordinat listesi |
| Bir tutamağın üstünde | Tutamağın kendi işleri (ADR 0279), ardından seçim menüsü |
| Shift ile | Tek seferlik kenet menüsü |

Tercih `drafting.rightClickEnter` ("Hızlı sağ tık Enter", Uygulama ayarları › Görünüm) kapalı başlar. Açıkken AutoCAD'in zamana
duyarlı sağ tıkı gibi çalışır: kısa basış onay alan komutu onaylar (Enter), 300 ms basılı tutmak menüyü açar. Tutamaçla
taşırken bu tercih de noktayı bırakmaz; tutamaç menüsü açılır.

Ortak etkileşim izlerinde (`fixtures/interaction/v1`) `rightClick` adımı web'in kısa sağ tıkıdır, yani Enter. Masaüstünün iz
oynatıcısı bu yüzden izleri tercih açıkmış gibi oynatır (`traces/player.rs`). Varsayılan davranışı birim testleri sınar:
`a_right_click_opens_the_menu_of_what_is_going_on` ve `a_right_click_never_drops_a_moved_grip_at_the_pointer`.

## 7. Python çıktısı

Tercih `drafting.pythonEcho` ("İşlemlerin Python kodunu konsola yaz", Uygulama ayarları › Görünüm › Python konsolu) kapalı
başlar; FreeCAD'in "Show script commands in Python console"u gibidir. Açıkken iki şey Python konsoluna "Komut" etiketli
satırlarla, kod olarak yazılır (`python/echo.rs`):

- **Komutlar:** açık çizimin çalıştırdığı her ürün komutu. Kaynak çizimin çağrı kaydıdır (`kentos.calls/1`, Komut
  geçmişinin okuduğu satırlar). Kod tek çağrıdır (`python_call::python_echo`):
  `cad.entities.transform.execute_input(doc, cad.entities.transform.input_type.from_json({...}))`. Başında komutun
  Türkçe başlığı yorum olarak durur. Kısıtları gevşeterek yapılan çağrı `with doc.relaxed():` altına yazılır. Formun
  gizli alanları gizli yazılır (ADR 0443 §2).
- **Seçim:** elle yapılan her seçim `doc.select([...])` olarak, kalıcı kimliklerle yazılır; seçim kalkınca
  `doc.select([])`. büyük seçimlerde de kimlikler eksiksiz yazılır (§9).

Konsolun kendi çalıştırmalarının yaptıkları yazılmaz; kodları zaten oradadır. Çizim açılınca ya da tercih açılınca o ana
kadar yapılanlar yazılmaz.

Seçimin bugüne kadar Python karşılığı yoktu. SDK'ya `Document.select(uids)` ve `Document.selection()` eklendi:

- Masaüstünün konsolunda bunlar pencerenin seçimidir; araçlar sonra onunla çalışır. Konsol bağlantısında `select` ve
  `selection` istekleriyle karşılanır. Bilinmeyen kimlik `unknown_object` ile reddedilir ve seçim değişmez.
- Penceresiz çizimde çizimin kendi seçimidir; silinen nesne seçimden düşer.

Komut geçmişinin Python kodu da (`python_call_in`, `python_script`) gevşetilmiş çağrıyı artık `with doc.relaxed():`
altında yazar. Daha önce gevşetme kayboluyordu.

Her aracın Python ve MCP karşılığının denetimi ayrı bir iştir (§8).

## 8. Araçların Python ve MCP karşılığı

Şeridin bütün yerleri (CAD ve CBS sekmeleri, bağlamsal sekmeler, menüler, taşmalar, başlatıcılar) tek tek denetlendi:
1953 yer, 742 tekil öğe (komut, araç ya da aracın yöntemi).

Sonuç:

- Ölü düğme yok. Şeridin kullandığı her komutun katalogda ve masaüstünde işleyicisi var.
- Girdisi Python'a çevrilemeyen komut yok: `python_call` her girdiyi `execute_input(doc, input_type.from_json(...))`
  olarak yazar. Gevşetilmiş çağrı §7'den beri `with doc.relaxed():` altındadır.
- İşlemler'in araçları (`processing.run.*`) Python ve MCP'den `cad.processing.run` ve `.batch` ile çalışır (ADR 0508,
  0529). Manifestteki "çalıştırılamaz" notu bayattı; düzeltildi.
- Asıl boşluk, sonucu arayüzün hesaplayıp hazır geometri olarak yazdırdığı araçlardır (`geometry` düzeyi, 79 öğe). Ürün
  komutu hiç olmayan 20 öğe daha vardır (`none` düzeyi). Python ya da ajan bunların sonucunu aynı komutla üretemez,
  geometriyi kendisi hesaplamak zorundadır. Sıralı liste TODOS.md `UX-14`'tedir; canlı liste `docs/cad/manifest.md`'dir.

Bu dilimde düzeltilenler:

- **Manifestin kapsamı:** `scripts/cad/manifest.py` kapsamı artık masaüstünün şeridinden
  (`catalog/v1/ribbon.json`) okur; donmuş web envanterinin şeridini kullanmaz. Bölünmüş düğmeler envanterdeki
  anahtarlarından çözülür. Envanterde bulunmayan anahtar sorun sayılır.
- **Sorguların ve İşlemler araçlarının karşılığı:** Manifest bunların Python ve MCP karşılığını da yazar. Komutu olan
  sorgu ya da İşlemler aracı artık boşluk sayılmaz.
- **Dosya komutlarının adı:** Python'da `kentos.cad.file.export.dxf` gibi yazılır; içe aktarma `file.import_`
  altındadır.
- **Bayat kayıtlar:**
  - `file.export.dxf` anlamsaldır. Pencerenin komuttan geçmesi ayrı bir iştir.
  - `processing.newModel` bir penceredir (`host`).
  - `levels.processing`'in tanımı güncellendi.
- **MCP'nin salt okunur işareti:** `readOnlyHint` artık katalogdan gelir. Katalog `effect: query` diyorsa, komut yıkıcı
  değilse ve kendi dosyasını yazmıyorsa (dosya komutları baskı denetimi dışında yazar) işaret verilir. Eskiden 14 komutluk
  elle bir liste vardı; işaretsiz kalan 41 sorgu artık işaretli. Test: `the_read_only_hint_follows_the_catalogs_effect`.
- **MCP'nin yıkıcı işareti:** Var olan nesnelerin sırasını değiştiren `cad.entities.order` eklendi. Yıkıcı sayılmayanlar:
  - `cad.layers.filter`, katmanın görünümüdür ve hiçbir şey silmez (ADR 0211);
  - `cad.templates.apply` ve `cad.profiles.apply` yalnız eksiği ekler.

İçe aktarmaların yıkıcı işareti olduğu gibi kaldı. DXF ve DWG'nin eklemeli sayılması testlerle bilerek seçilmiştir.

## 9. Kesilen çalışmanın denetimi ve devamı

23 Ekim, aynı dalda devam edilirken bulunan ve düzeltilen açıklar:

- **Proje türü kilidi:** yalnız pencereyi kapatmak yeterli değildi; `cad.settings.set` mevcut CAD projesini CBS'ye ve
  tersine çevirebiliyordu. Komut artık `project_type_locked` ile girdinin tamamını reddeder. `validate`, `plan` ve
  `execute` aynı kuralı uygular; aynı tür tekrar verilebilir, türü sorulmamış ve eski Hibrit proje bir kez seçer.
  Ortak durum sayısı 62'ye çıktı; ad ve ölçeğin ret sırasında yerinde kalması da sınanır.
- **Bağlamsal şeridin klavyesi:** F6/Alt, seçimin açmadığı bütün bağlamsal sekmelere de harf veriyordu. Yalnız görünen
  sekme hedef olur. Seçimin sekmesi klavye hedefleri yenilenmeden önce izlenir. Komut ara, görünen bağlamsal
  sekmedeki İşlemler aracının yerini de gösterebilir; kapalı çizim aracının nedenini söyler.
- **Çizimler arasında önbellek:** seçimin ve belgenin sayaçları farklı çizimlerde aynı olabilir. Sekme önbelleğinin
  anahtarına belge oturumu eklendi; bir çizimin türü ötekine taşınmaz.
- **Katman araçları:** Döndürülmüş dikdörtgen (`rectangle3`) da alan yazar; çizgi katmanında kapalıdır. Kot noktası
  kendi `kot` katmanına yazdığından etkin katmanın türü yüzünden kapatılmaz.
- **Python çıktısı:** büyük seçimler artık yalnız yorum değildir; 200'den fazla nesnenin de bütün kimlikleriyle
  çalıştırılabilir `doc.select([...])` kodu vardır. Konsolun 128 KiB çıktı sınırı yürütülebilir komut kodunu kesmez;
  yorumlayıcının normal çıktısı sınırlı kalır. Başarılı geri al/yinele `doc.undo()` / `doc.redo()` yazar; aracın
  içinden gerçekten yazılmış son nesne geri alındığında da. Yalnız taslağın son köşesini silmek belgeyi geri almaz.
- **Seçimin ortak yüzeyi:** `drawing.select` ve `drawing.selection` MCP'ye eklendi. Penceresiz seçim Python'a özel
  bir sözlükte tutulmaz; native `Session`'ın durumudur. Masaüstünde Python ve MCP aynı seçim nesnesini ve aynı
  doğrulayıcıyı kullanır. Ajanın çizimi arka sekmedeyse onun saklanan seçimi değişir; öndeki seçim yerinde kalır.
  Seçim belgeyi, revizyonu, dosyayı veya geri alma geçmişini değiştirmez. Açık kimlik listesi programın istediği
  seçimdir; sahneden tıklama/kutu seçiminin etkin katman kuralından ayrı, `Document.select`'in mevcut anlamıdır.
  Liste olmayan girdi veya metin olmayan öğe `invalid_input`, bulunmayan kimlik `unknown_object` olur; listenin
  bütünü doğrulanmadan seçim değişmez. Tekrarlı kimlikler ilk sırayı koruyarak teke iner; silinmiş nesne okunmaz.
- **Halka:** ilk sıradaki çizim temellerinden `cad.donut.create` eklendi. Girdi merkez, iç ve dış çap (metre), katman ve
  görünüştür. Dış çap iç çaptan büyük, iç çap sıfır veya pozitiftir; sıfır iç çap disk yapar. Tek dolu tarama, iki
  yarım daire yayıyla kesin dış ve iç sınır, tek geri alma adımı. Araç bu komutla yazar; Python/MCP geometri
  hesaplamaz. `donut_command_cases.py` KentOS yüklemeden 60 basamaklı trigonometrik başvuruyla 25 durum üretir.
- **Daire dilimi:** `cad.sector.create` eklendi. Girdi merkez `c`, yarıçap `r` (metre), başlangıç ve bitiş açısı
  `a0`, `a1` (radyan, saat yönü tersine; süpürme `a1 − a0`'ın tam tura göre kalanı), katman ve görünüştür. Halka merkez,
  başlangıçtaki ve bitişteki yay noktasıdır; yay kenarı `tan(süpürme/4)` bükümüyle kesindir. Sıfır ya da tam tur
  süpürme `degenerate_shape`, sıfır yarıçap `invalid_distance` ile reddedilir. Araç önizlemeyi komutun halkasından
  (`shapes::sector_ring`) alır, Çakışma ve Topoloji alanı değiştirmedikçe komutla yazar (`points::write_area_by`);
  komutu kullanmayan eski `write_area` kalktı. `shape_command_cases.py` KentOS yüklemeden 27 durum üretir; masaüstünde
  `semantic_tool_tests.rs` aracı gerçek fare olaylarıyla sürer: kayıttaki son çağrı `cad.sector.create`'tir, girdi
  hazır halka değil kullanıcının verdiğidir, Python yankısı aynı çağrıyı taşır, sağ tık nesne koymaz.
- **Revizyon bulutu:** `cad.revcloud.create` eklendi. Çerçeve doğu ve kuzey boyunca bir dikdörtgenin iki köşesi
  (`rectangle`), bir çokgenin köşeleri (`polygon`; bir nanometre içinde yinelenen köşe ve ilk köşeye dönüş düşer) ya da
  kapalı bir nesnenin kalıcı kimliğidir (`object`: alan, kapalı çoklu çizgi, daire, kapalı eğri, tam elips; dış
  çevresi turda 64 kirişle, nesne kalır). Yay boyu `arc` metredir: araç kâğıt milimetresini çizimin ölçeğiyle çevirip
  verir; komut ne etkin katmanı ne ölçeği okur. Halka saat yönünün tersine çevrilir, her kenar `max(1, round(boy /
  arc))` eşit kirişe bölünür, her kiriş 0,5 bükümlü dışa yaydır (çekirdeğin `cloud_of`'u). 10 000'den çok yay
  `too_many_points`, açık nesne `not_closed`, olmayan nesne `entity_not_found` ile reddedilir; sayı, yaylar
  yapılmadan çekirdeğin `cloud_arcs`'ıyla bulunur. Araç önizlemeyi ve yazılacak halkayı `shapes::revcloud_ring`'den
  alır, Nesne kipinde tıklanan nesnenin kimliğini verir. `shape_command_cases.py` 34 durum üretir; masaüstünde iki
  kip gerçek fareyle ve komut satırıyla sürülür. Daire dilimi ve bulutun katman alanı retleri
  (`invalid_attribute`, `attribute_required`) `cad.polygon.create`'in yolundan gelir ve yetenek testinde gerekçelidir.
- **Çakışma ve Topoloji komutun girdisi:** oturumun iki anahtarı artık aracın kendi kırpması ve köşe bağlaması değil,
  komutun `rules`'udur (`AreaRules`: `avoid` kaçınılan katmanlar, `topology`, `points`). `cad.polygon.create` ve
  `cad.rectangle.create`, `cad.polygon.regular`, `cad.sector.create`, `cad.revcloud.create` aynı yoldan yazar
  (`polygon::ruled_areas`, `area_rules.rs`): kaçınılan katmanların görünen alanlarıyla örtüşen kısım çıkarılır, kalan
  tek nesnedir, parçaları ve delikleri olduğu gibi; bütünüyle örtülen `area_covered` ile reddedilir; Topoloji yeni
  alanı görünen çizgi, yol ve alanlarla 1 µm içinde köşe köşe bağlar, komşular `cad.entities.edit`'in `vertexAdd`
  denetimiyle aynı adımda yazılır, kilitli komşu köşe almaz. Söylenenler uyarıdır: `area_clipped`, `corners_joined`,
  `neighbours_locked` (iletileri aracın eskileri, ortak izler değişmeden geçer). Komşular ev sahibinin geometri
  deposunun görünenleridir; araçlar kendi depolarını verir, başsız sunucu çizimden kurar. Kesilen sınır komşuların
  üzerindeki köşelerini taşır (ortak köşe korunur). `cad.entities.adjoin` (Bitişik alan) yalnız yeni sınırı alır:
  yol, kenar başına büküm, komşu katmanlar (yoksa kendi katmanı) ve isteğe bağlı kutu (aracın görünümü); bölgeyi
  çekirdeğin `Neighbours::fill`'i bulur, kapanmayan yol `no_region`, tek adım “Bitişik alan”. Kapalı alan, Alan
  olarak çiz, şekil araçları ve Bitişik alan artık komutla yazar; aracın kırpma ve bağlama kodu kalmadı. Topoloji komşulara biçimi değiştirmeyen köşe eklediği için MCP'de bu komutlar
  yıkıcı sayılmaz. `area_rules_cases.py` dik köşeli alanların birim hücreleriyle, KentOS kodu olmadan 24 + 24 durum
  üretir; çalıştırıcının `shapes` beklentisi halkanın başladığı köşeden bağımsız köşeleri, net alanı, parçaları ve
  delikleri karşılaştırır.
- **Parsel oluştur:** `cad.parcels.create` köşeleri (ve kenar başına bükümü), isteğe bağlı numarayı ve öznitelikleri,
  görünüşü ve `rules`'u alır; etkin katman ne olursa olsun `parsel` katmanına yazar, katman yoksa yeni projedeki gibi
  (“Parsel sınırı”) aynı adımda açar ve cevapta `opened` der. Numara verilmezse parsellerin Parsel değerlerinin
  başındaki en büyük tam sayının bir fazlasıdır (JavaScript'in parseInt'i: “12/A” 12); verilen numara kırpılır, boşsa
  `invalid_parcel_number`, başka parselde de varsa `parcel_number_taken` uyarısıyla yine verilir. Öznitelikler Ada,
  Parsel, Mahalle, Nitelik “Arsa”, Tapu alanı (m²), Pafta; verilenler üstüne, Parsel numaradır, boşlar kanonik biçimde
  yazılmaz; etiket numaradır, tapu alanı tapunundur ve boş kalır. Kilitli parsel katmanı aracın sözüyle reddedilir.
  Araç köşeleri ve oturumun kurallarını verir, parseli seçer, açılan katmanı söyler; aracın kendi numaralaması, kırpması
  ve köşe bağlaması (`junctions.rs`, `overlap.rs`'in kırpması) kalktı. `area_rules_cases.py` 21 durum üretir.
- **Python denetiminin dosyaları:** kayıtlı başarılı çağrıların yanında `FILE_COMMANDS`'ın tamamının katalog örnekleri
  de Python koduna çevrilir ve SDK'nın girdi tipiyle geri okunur. Ortak durumun bir adımı başka komut
  çalıştırıyorsa denetim dosyanın başlığını değil o adımın gerçek komutunu kullanır. SDK'nın eklediği dolu
  varsayılanlar yalnız girdi şemasında aynen tanımlıysa kabul edilir; verilen değerlerin değişmesi gizlenmez. Bu son denetim dosya yazma testi değildir;
  mevcut başsız dosya testlerinin yanında kodun/kataloğun bütünlüğünü sınar.

Bu bölüm `UX-14`'ün bütün tarihsel araç borcunu kapattığı anlamına gelmez. Halka, Daire dilimi, Revizyon bulutu,
Çakışma/Topoloji, Bitişik alan ve Parsel oluştur dilimleri kapandı; diğer açık araçlar (yapım yardımcıları, P1 ve P2)
listedeki sıralarıyla kalır.

### 9.1 Bu devamın doğrulaması

23 Ekim, bu çalışma ağacında çalıştırıldı:

- Native uygulama: 142 başarılı; ortam gerektiren 22 denetim varsayılan koşuda atlandı. Ortak Halka durumlarının
  25'i ve proje ayarlarının 62'si bu koşunun içindedir.
- Başsız uygulama: 47 entegrasyon ve 10 kitaplık denetimi; MCP: 26 entegrasyon ve 22 kabul denetimi başarılı.
- Python: 256 denetim başarılı. Gerçek masaüstü Python konsolunun açık çizime nesne yazması ayrıca çalıştırıldı ve
  geçti. Geçmişin 7.819 çağrısı SDK'dan kayıpsız geri okundu; bunun dosya yazma denetimi olmadığı §9'da açıklanır.
- Masaüstü: 70 hedefli denetim başarılı, 10 ortam denetimi bu koşuda atlandı. Ortak etkileşim izlerinin üç varyantı
  geçti. Python çıktısında 6.000 UUID'nin 128 KiB üstünde de kesilmediği gerçek konsol bileşeniyle sınandı.
- Görsel kabul: bağlamsal sekmeler, ana şeritler, sağ tık menüleri, Python çıktısı ve Halka için ayrı seçilerek
  çalıştırılan beş görsel denetim 55 görüntü üretti. Katman türü/otomatik seçim denetimi ayrıca 12 görüntü üretti.
  Açık/koyu ve dar pencere örnekleri, Halka'nın 18 px yazısı, CAD/CBS ana şeritleri, Çizgi/Alan/Raster/Nokta bulutu
  sekmeleri, taşınan tutamacın menüsü ve katman kısıtları görüntü dosyaları açılarak incelendi. Test girdileri gerçek
  widget klavye/fare olaylarından geçti; sağ tık yeni nesne yazmadı. Bağlı raster/bulut dosyası olmayan örneklerde
  sahne yalnız kayıtlı sınırlarını gösterir; bu resimler raster/bulut çözümleme testi değildir.
- Rust → TS dışa aktarma: 1.594 denetim, komut kataloğu: üç denetim başarılı. SDK, CAD manifesti (884 satır), şerit
  ve bağımsız fixture üreticileri güncel; mimari bağımlılık denetimi 47 crate / 61 hedefte temiz.
- İlgili beş native paketin bütün hedeflerinde Clippy `-D warnings` geçti; masaüstü üretim ikilisi derlendi.
  Bağımlılık `block 0.1.6` için Cargo'nun mevcut gelecek Rust uyumluluğu bildirimi sürer.

Web arayüzü testleri ve derlemesi ADR 0228 nedeniyle çalıştırılmadı; yalnız ortak üretilen sözleşmeler güncellendi.
Bu kanıtlar bütün eski araçların `UX-14` dönüşümünü tamamladığı iddiası taşımaz.
