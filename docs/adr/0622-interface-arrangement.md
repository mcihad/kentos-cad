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

## 3. Şerit (sırada)

Masaüstünün şeridi web'in donmuş envanterinden değil, masaüstüne ait bir tanımdan okunacak:
`catalog/v1/ribbon.json`, CAD ve CBS için ayrı sekmeler. İlkeler:

- Bir komut ana sekmelerde bir kez yer alır. Kopyaları kalkar; bağlamsal sekmelerde tekrar
  edebilir.
- Düğmeler yazılıdır. Büyük düğme ikon ve ad gösterir; küçük düğme ikon ve ad yan yana
  gösterir. Yalnız ikona dar pencerede inilir.
- İşlem araçları (`processing.run.*`) ana sekmelerde yer almaz: araç kutusundadır (sağ dok,
  İşlemler) ve ilgili bağlamsal sekmelerde gelir.
- CAD'in sekmeleri Netcad ve AutoCAD'den: Dosya, Giriş, Plan (imar planı: yol, ada ve parsel,
  alan işlemleri, plan yazıları), Ekle, Açıklama, Değiştir, Mimari, Görünüm, Yönet, Çıktı.
- CBS'nin sekmeleri ArcGIS Pro ve QGIS'ten: Dosya, Harita, Düzenle, Veri, Analiz, Ölçme,
  Görünüm, Çıktı. Bugünkü Raster sekmesi kalkar; araçları Raster bağlamsal sekmesine gider.

## 4. Seçime göre bağlamsal sekme (sırada)

Seçimin türü bir aileyse şeritte o ailenin sekmesi açılır ve öne gelir (AutoCAD'in Tarama
Düzenleyicisi gibi):

- Aileler: Nokta, Çizgi, Alan, Yazı, Ölçü, Kılavuz, Tarama, Blok, Tablo, Resim, Raster, Nokta
  bulutu ve CAD'de mimari nesneler.
- Karışık seçimde genel "Seçim" sekmesi açılır.
- Seçim kalkınca önceki sekmeye dönülür.
- Sekme yalnız o türe yapılabilecek işleri taşır, işlem araçları dahil.

## 5. CBS'de katman türü ve seçim (sırada)

- Katmanın geometri türü olur: Nokta, Çizgi, Alan, Açıklama ya da Karışık.
- Türlü katmana başka türden nesne yazan komut `layer_geometry_mismatch` ile reddedilir.
  Kural komuttadır, Python ve MCP de uyar. Şeritte uymayan çizim araçları kapalıdır.
- Proje ayarı "Otomatik seçim": açıkken bugünkü seçim, kapalıyken yalnız etkin katman seçilir.
  CBS'de kapalı, CAD'de açık başlar.

## 6. Sağ tık (sırada)

Sağ tık AutoCAD'in varsayılanı gibi her zaman o anki işin menüsünü açar ve hiçbir zaman imlecin
yerine nokta koymaz:

- Komut yokken: genel menü.
- Seçim varken: düzenleme menüsü.
- Komut sürerken: Onayla, İptal, komutun seçenekleri, tek seferlik kenet.
- Sıcak tutamakta: tutamaç menüsü (Esnet, Taşı, Döndür, Ölçekle, Aynala, Taban noktası, Kopya,
  Çıkış).
- Shift ile: kenet menüsü.

Hızlı sağ tıkın Enter olması (AutoCAD'in zamana duyarlı sağ tıkı) bir kullanıcı tercihi olarak
kalır ve kapalı başlar.

## 7. Python çıktısı (sırada)

Uygulama ayarı "İşlemlerin Python kodunu konsola yaz" (FreeCAD'in "Show script commands in
Python console"u gibi). Açıkken çizilen, seçilen, taşınan her işin Python kodu Python
konsoluna yazılır; kapalıyken yazılmaz. Her aracın kodu tek tek denetlenir: anlamsal komutla mı
yazıyor, Python'da ve MCP'de karşılığı var mı (ADR 0244'ün manifesti).
