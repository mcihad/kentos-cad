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

## 5. CBS'de katman türü ve seçim (sırada)

- Katmanın geometri türü olur: Nokta, Çizgi, Alan, Açıklama ya da Karışık.
- Türlü katmana başka türden nesne yazan komut `layer_geometry_mismatch` ile reddedilir.
  Kural komuttadır, Python ve MCP de uyar. Şeritte uymayan çizim araçları kapalıdır.
- Proje ayarı "Otomatik seçim": açıkken bugünkü seçim, kapalıyken yalnız etkin katman seçilir.
  CBS'de kapalı, CAD'de açık başlar.

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

## 7. Python çıktısı (sırada)

Uygulama ayarı "İşlemlerin Python kodunu konsola yaz" (FreeCAD'in "Show script commands in
Python console"u gibi). Açıkken çizilen, seçilen, taşınan her işin Python kodu Python
konsoluna yazılır; kapalıyken yazılmaz. Her aracın kodu tek tek denetlenir: anlamsal komutla mı
yazıyor, Python'da ve MCP'de karşılığı var mı (ADR 0244'ün manifesti).
