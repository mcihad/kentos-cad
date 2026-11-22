# KentOS — Veri yönetimi planı (veri sürücüleri, canlı katman, düzenleme oturumu)

Başlangıç: **20 Kasım 2026**. Sahibin isteği (özetle): GeoPackage ve PostGIS için birinci sınıf destek, ileride Oracle,
MySQL ve SQL Server'ın gelebileceği ayrı crate'lerde bir sürücü soyutlaması; veritabanından eklenen katmanın bütün
veriyi önbelleğe almaması, görüneni gerçek zamanlı okuması; alttaki tablonun süzme ve aramasının veritabanında yapılması;
kategorili/dereceli/kurallı stillerin sınıflarının katmanın altında görünmesi ve açılıp kapanması; düzenleme modu,
kaydetme yapısı, işlem (transaction) yönetimi, etkin katman ve katman türlerinin bir CBS'ye uygun hâle gelmesi
(“şu anki yapı uygun değil”); katmanın sağ tuşunda ayrıntılı bir dışa aktarma penceresi; GeoPackage projeleri.
21 Kasım'da: bütün maddeler bitene kadar durmadan sürülür; proje dosyaya ya da veritabanına kaydedilebilir, CBS verisi
ise her zaman bir veri kaynağındadır (VY-09).

Sıra mantık sırasıdır: her madde bir öncekinin üstüne kurulur. Her madde: araştırma (resmî belgeler, kurulu masaüstü
CBS'nin ekranları, Türkiye mevzuatı), ADR, kod, testler, gerçek pencerede ekran görüntüleri, commit ve push. Başvuru
ekranları `.run/gui-evidence/veri/` altındadır (Git'e girmez).

ADR'ler **9100**'den başlar.

## Maddeler

- [x] **VY-01 — Sürücü soyutlaması, GeoPackage ve PostGIS sürücüleri.** ([ADR 9100](../docs/adr/9100-data-drivers.md))
  - Ortak sözleşme `kentos-data` saftır: değer, şema, koşul ağacı, istek, akış, işlem, kayıt, parola bulucu, değer
    denetimi.
  - Sürücüler ayrı crate'lerdir: `kentos-data-gpkg`, `kentos-data-postgis`.
  - İstek kaynağa gider: kutu dizinden, koşul ve sıralama SQL'de, sadeleştirme PostGIS'te sunucuda. PostGIS satırları
    sunucu imleciyle okunur; veritabanı başına tek bağlantı havuzu vardır.
  - Uygunluk takımını iki sürücü de geçer. Bir milyon parselde PostGIS'te sokak görünümü 11 ms, bütün kent 1,26 s;
    GeoPackage'da 0,2 ms ve 0,8 s.
  - Kalan: masaüstüne bağlanması VY-02'de.
- [x] **VY-02 — Katman türleri ve veri katmanı.** ([ADR 9101](../docs/adr/9101-data-layers.md) §2–§5, §7)
  - Projede “veri katmanı” ayrı bir türdür (`LayerNode.data`). Taşıdıkları:
    - kaynak: sürücü, adres ve tablo;
    - anahtar, geometri sütunu, türü ve sistemi;
    - tablonun kutusu;
    - salt okunur olduğu.
  - Tablonun sütunları katmanın alanlarıdır. Komut: `cad.layers.data` (ekle, değiştir, kaldır; 33 ortak durum).
    `.kcad`'da CBS verisi sürüm 2.
  - Satırları çizime kopyalanmaz. Katmanlar'da türünün simgesi, kaynağının imi ve ipucuyla görünür: veri katmanı
    (nokta, çizgi, alan, tablo).
  - Kaynaklar'dan (“Veri katmanı olarak ekle”) ve PostGIS'e bağlan'dan eklemek yalnız kataloğu okur.
  - Proje kapatılıp açılınca kaynak yeniden bağlanır; parola projeye yazılmaz, cihazın kayıtlı bağlantısından bulunur.
  - Kalan:
    - katmanın süzgeci veri katmanında henüz uygulanmıyor (VY-04'te SQL'e);
    - göreli yolun yazılması ve çözülmesi VY-09'da;
    - SQL sorgusu katmanı bağlı kopya olarak kaldı.
- [x] **VY-03 — Canlı çizim.** ([ADR 9101](../docs/adr/9101-data-layers.md) §6)
  - Veri katmanı görünen alanı ve her yandan yarısı kadar payı arka planda okur, parça geldikçe çizer. Görünüm başka
    yere gidince eski istek durur.
  - Geometri yarım piksele sadeleştirilir; PostGIS'te sunucuda. Pay içindeki kaydırma yeniden okumaz.
  - Kategorili, dereceli ve kurallı stiller çizim katmanlarıyla aynı motordan geçer.
  - Gerçek pencerede bir milyon PostGIS parseli ve bir GeoPackage'ın yolları ve direkleri, bütün kentten sokağa kadar
    kaynaktan çizildi. Çizimde 0 nesne vardı.
  - Etiketler çizimin etiketleriyle aynı motordan geçer (okunan satır 20 000'i geçmezse).
  - Bilgi, nesne olmayan yere tıklanınca görünen veri katmanlarına üstten alta sorar. Satır bütün sütunlarıyla
    kaynaktan okunur; kartı katmanın şablonundan geçer.
  - Kalan:
    - seçim VY-04'te (tabloyla birlikte), kenet VY-05'te (düzenleme oturumuyla birlikte);
    - stil penceresinin sınıflandırmasının sürücüden okuması VY-07'de.
- [x] **VY-04 — Alttaki tablo kaynakta.** ([ADR 9102](../docs/adr/9102-data-table.md))
  - Metin Türk alfabesiyle sıralanır, iki sürücüde de. `cad.layers.rows` 28 ortak durumla.
  - Masaüstünde sayfalı tablo; bir milyon satırda denendi. Ara ve süzgeç Enter'la sorulur.
  - Tabloda seçilen satırlar haritada çevrelenir; Seçime yakınlaş.
  - Kalan: Seç aracıyla haritada satır seçmek VY-05'te.
  - Öznitelik tablosu veri katmanında sayfa sayfa okunur.
  - Sıralama, süzgeç, arama ve sayı SQL'de; “görünümdekiler” ve “seçililer” kaynaktan.
  - Hücre düzenlemesi düzenleme tamponuna yazılır (VY-05).
- [x] **VY-05 — Etkin katman ve düzenleme oturumu.** ([ADR 9103](../docs/adr/9103-edit-session.md))
  - Katman Katmanlar'ın sağ tuşundan düzenlemeye açılır ve kapatılır; açılınca etkin katman olur.
  - Değişiklikler kaydedilene kadar çizimde katmanın nesneleri olarak bekler: çizilen yeni satır, Seç'le alınıp
    değiştirilen satır, silinen satır. Çizimin bütün araçları onlarda çalışır; geri alma çizimle tek sıradadır.
  - Kaydet bir katmanın değişikliklerini kaynağının tek işlemiyle yazar; Geri al bırakır; bitirirken ve çizimden
    çıkarken Kaydet / Kaydetme / Vazgeç diye sorulur. Bütün katmanlar için de.
  - Kaydedilmemiş değişiklik Katmanlar'da turuncu kalemle ve çizimde kesik çerçeveyle görünür.
  - Kaydetme reddedilirse hiçbir şey yazılmaz, değişiklik bekler, suçlu nesne seçilir.
  - `cad.layers.session` 29 ortak durumla; Python ve MCP veri sürücülerini kaydeder.
  - Gerçek pencerede bir milyon PostGIS parselinde çizildi, taşındı, silindi, kaydedildi. Veritabanı her adımı doğru
    tuttu.
  - Düzenlenen katmanın satırlarına kenetlenilir; satır sadeleştirilmeden okunur, kenet kendi köşesine düşer.
  - Öznitelik tablosunda hücre çift tıklanıp düzenlenir; satır alınır, değer tampona yazılır.
  - Kalan: kaydedilmemiş yeni satırlar tabloda listelenmez (durum satırı sayar); alınan satırların dosyada saklanması
    (ADR 9104 §5).
- [x] **VY-06 — İşlem yönetimi.** ([ADR 9104](../docs/adr/9104-transactions.md))
  - Proje ayarları › Genel › Birlikte düzenlenen katmanlar: Her katman ayrı, Aynı kaynaktakiler birlikte, Bütün
    katmanlar birlikte (`settings.editGrouping`; `.kcad`'da CBS verisi sürüm 3).
  - Grubun katmanları birlikte açılır, kaydedilir, geri alınır, kapanır. Aynı kaynağın katmanları tek işlemle yazılır;
    biri yazılamazsa hiçbiri yazılmaz.
  - Kaydetmeden önce alınan satırlar aynı işlemde yeniden okunur (`Transaction::rows`, PostGIS'te `FOR UPDATE`).
    Başkası değiştirdiyse ya da sildiyse hiçbir şey yazılmaz ve `rows_changed` söylenir. Üstüne yaz (`force`) yalnız
    değiştirilen alanları ve şekli yazar; başkasının öbür alanlardaki değişikliği kalır.
  - Sunucuda açık işlem tutulmaz: kilit uzun tutulmaz, bağlantı koparsa iş gitmez (ADR 9104 §2).
  - Kaydetme ya da geri almadan sonra Öznitelik tablosu bulunduğu yeri yeniden okur.
  - Gerçek pencerede, bir milyon satırlık PostGIS tablosunda:
    - başka bir bağlantının değiştirdiği parsel kaydedilirken çakışma soruldu, hiçbir şey yazılmadı;
    - Üstüne yaz taşınan şekli yazdı, öbür kullanıcının kullanım değeri kaldı;
    - aynı GeoPackage'ın iki katmanı birlikte açılıp kapandı, PostGIS katmanı açılmadı.
  - Kalan: alınan satırların proje dosyasında saklanması.
- [x] **VY-07 — Sınıflar katman ağacında.** ([ADR 9105](../docs/adr/9105-layer-classes.md))
  - Katman stilinin sınıflandırması (kategorili, aralıklı, kurallı) katmanın altında satır olur: kutu, sembolün rengi,
    ad; kurallar derinlikleriyle. Kutu sınıfı açar kapar; işleyiciye `enabled: false` (Diğer değerler `otherEnabled`)
    yazılır, katmanın gözü gibi geri alma adımı değildir. Kapalı sınıfın nesneleri çizilmez, etiketlenmez, kenet almaz.
  - Sağ tuş: Tümünü göster, Tümünü gizle, Tersine çevir; Bu sınıfın nesnelerini seç; Öznitelik tablosunda göster;
    Renk ▸; Sembolü düzenle…; Nesne sayısını göster.
  - Nesne sayısı (`classCounts`, `.kcad`'da CBS verisi sürüm 4): çizimin kuralıyla; veri katmanında kaynaktan bir
    tarama, arka planda.
  - `cad.layers.classes` (24 ortak durum); `cad.layers.set`'e `classVisible`, `classInvert`, `classCounts`.
  - Gerçek pencerede bir milyon parselde: dört sınıfın her biri 250.000; Konut kapatılınca parselleri etiketleriyle
    kalktı; tabloda gösterme, seçim, renk.
  - Kalan: çizim katmanının etiketleri kapalı sınıfa bakmaz; sınıf satırında çift tık yok; kaynakta gruplanmış sayım.
- [x] **VY-08 — Dışa aktarma penceresi.** ([ADR 9106](../docs/adr/9106-layer-export.md))
  - Katmanın sağ tuşunda Dışa aktar ▸ Farklı kaydet… ve Seçilenleri farklı kaydet… (seçim yokken soluk).
  - Hedefler: GeoPackage tablosu, PostGIS tablosu, GeoJSON, Shapefile, CSV, KML.
  - Seçenekler:
    - koordinat sistemi, kodlama;
    - yalnız seçilenler, yalnız görünümdekiler, İfadeyle seç dilinde süzgeç;
    - alanlar: seçim, yeniden adlandırma, görünen değer; liste kendi kaydırma çubuğuyla;
    - geometri türü (geometrisiz dahil), çok parçalıya zorla, Z/M;
    - tablo ve biçim seçenekleri;
    - hedef varsa: yazma, dosyanın üstüne yaz, katmanı değiştir, ekle;
    - üst veri, haritaya ekle.
  - Komut `cad.layers.export` (31 ortak durum). Veri katmanının satırları parça parça okunur ve yazılır.
  - Pencere arka planda yazar: ilerleme, Durdur. Durdurulan ya da başarısız yazma hiçbir şey bırakmaz; üstüne
    yazılacak dosya olduğu gibi kalır (`.part`).
  - GeoPackage sistemi adı ve WKT'siyle yazılır, tanıtıcı benzersizdir; başka bir CBS kitaplığı dosyayı doğru okur.
  - Kalan: 32 bitlik tam sayının GeoPackage türü, süzgecin kaynağa gitmesi, akışlı GeoJSON/Shapefile/KML (ADR 9106 §7).
- [x] **VY-09 — Proje dosyada ya da veritabanında; CBS verisi her zaman bir veri kaynağında.** Sahibin sözü (21 Kasım
  2026): projeler veritabanında “saklanacak değil saklanabilecek”; “veriler her halükarda veritabanında saklanacak CBS
  projelerinde”; GeoPackage, PostgreSQL ya da ileride başka bir veritabanı.
  - Proje her zamanki gibi dosyaya (`.kcad`) kaydedilir. İsteyen onu bir GeoPackage dosyasının içine (`kentos_projects`)
    ya da bir PostgreSQL şemasına da kaydeder ve oradan açar. Bu bir seçenektir, zorunlu değildir.
  - Projenin veritabanında saklanması da sürücü soyutlamasından geçer (`kentos-data`'da bir proje deposu sözleşmesi,
    GeoPackage ve PostGIS sürücülerinde gerçekleşmesi). Yeni bir veritabanı sürücüsü projeyi de saklayabilir.
  - CBS projesinde nesne verisi proje dosyasına gömülmez, her zaman bir veri kaynağındadır. Yeni katman bir kaynağın
    yeni tablosudur: projenin GeoPackage'ı ya da seçilen GeoPackage veya veritabanı şeması. Nesne tutan eski CBS
    katmanları için tek adımda bir kaynağa taşıma yolu sunulur.
  - Veritabanındaki proje Kaynaklar'da kendi dosyasının ya da şemasının katmanlarıyla birlikte görünür. Aynı
    dosyadaki katmanlar dosyayla birlikte taşınır.
  - Adımlar ([ADR 9107](../docs/adr/9107-project-in-a-source.md)):
    - [x] **09a** Proje deposu sürücü sözleşmesinde (`ProjectStore`; GeoPackage `kentos_projects` uzantısı,
      PostgreSQL şeması); Dosya › Veri kaynağı'ndan kaydet ve aç; veri katmanının göreli yolu yazılır ve okunur, dosya
      taşınınca katman bulunur; Python `save_to`, `open_from`, `projects`.
    - [x] **09b** Kaynaklar'da projeler: GeoPackage'ın tablolarının ve şemanın tablolarının yanında; tıklayınca
      açılır; Veri kaynağı penceresinden onaylı silinir.
    - [x] **09c** Yeni katman bir kaynağın yeni tablosu: `cad.layers.create` (24 ortak durum); CBS projesinde Nokta,
      Çizgi, Alan katmanı… Yeni katman penceresini açar (GeoPackage ya da PostgreSQL, geometri, alanlar).
    - [x] **09d** Nesne tutan eski CBS katmanlarını bir kaynağa taşıma: `cad.layers.migrate` (21 ortak durum); katmanın
      sağ tuşunda Veri kaynağına taşı…; süzgecin gizledikleri de taşınır, tabloya yazılamayan nesne varsa hiçbir şey
      yazılmaz; tek geri alma adımı.

## İkinci tur: kalanlar ve kırık testler (21 Kasım 2026)

Sahibin sözü (21 Kasım 2026, VY-09'un raporundan sonra): “Hepsini düzelt eksikleri tamamla.” Kapsam: ADR 9100–9107'nin
“Kalan” bölümlerinde yazılı her eksik ve testlerdeki kırıklar. Bilerek yapılmayanlar (sunucuda açık işlem, ADR 9104 §2)
ve donmuş web işi (ADR 0228) kapsam dışıdır. Akış aynı: araştırma, ADR, kod, testler, gerçek pencere, commit ve push.

- [x] **VY-10 — Kırık testler.** Masaüstü: 2000 satır sınırını aşan `shell/exchange/gpkg_tests.rs` konularına göre
  bölündü (ADR 8054 §7); eklentinin form demosu ADR 1080'in kuralına göre beklenir. Headless: shapefile testi "nokta"
  katmanını önce Karışık yapar (ADR 1074); BIM paket testleri ADR 8133'e göre (7. soru dosyanın; dairenin ocağı
  yüksekliğiyle). Tam ortak durum koşusu temiz:
  - 41 mimari durumu bağımsız betiklerinden yeniden üretildi: görünüşte `memberHashes`, IFC'de kayıt sayıları
    (IfcOpenShell), programda adı geçen komşu, bölüm çizelgesinde boş kapsam notu, altlıkta dosyanın özeti. Python
    yolunda bağımsız KCAD yazıcısı altlıklı belgenin mimari modül sürümünü (18) yazar.
  - `cad.arch.walk`'ın hiç ortak durumu yoktu: 27 durum (`scripts/fixtures/arch_walk_cases.py`, shapely), koşucu
    kaydı ve yetenekleri.
  - Yetenek denetimi: veri komutlarımın andığı ama hiçbir durumun vermediği kodlara durum (servis katmanı, yapılamayan
    hedef, bilinmeyen sistem, uzun tablo adı); `cad.layers.set` ile `cad.arch.performance` verdikleri kodları anar;
    taşıma tablo adı hatasını `table` alanında söyler.
- [x] **VY-11 — Sürücü sözleşmesi: 32 ve 64 bit tam sayı, ayrık değerler, gruplanmış sayım.**
  ([ADR 9108](../docs/adr/9108-integer-widths-and-groups.md))
  - `FieldKind::Int { bits }`. GeoPackage'a genişliği tutan en dar tür yazılır: 32 bit `MEDIUMINT`, 64 bit `INTEGER`
    (GDAL'ın eşlemesi). PostGIS'te `smallint`, `integer` ya da `bigint`. Sütunun tutmadığı değer reddedilir.
  - Kaynakta gruplanmış sayım `Source::groups` (`GROUP BY`; Türk alfabesinin sırası, boş değer sonda). Kendi sorgusu
    olmayan sürücü için varsayılan sayar. Ayrık değerler de Türkçe sıralı döner.
  - Uygunluk takımı iki sürücüde geçti. Kurulu masaüstü CBS kitaplığı KentOS'un yazdığı `no` alanını `Integer`,
    `sayac` alanını `Integer64` okudu; önce ikisi de `Integer64` okunuyordu.
  - Gerçek pencere: Kaynaklar'dan eklenen tablonun Alanlar'ında “Tam sayı” ve “Büyük tam sayı”.
- [x] **VY-12 — İfadeden SQL'e.** ([ADR 9109](../docs/adr/9109-expression-to-the-source.md))
  - ADR 0581 §3'ün kuralı tek yerde: `pushdown::sent` bir ağaç verir. SQL ve sürücülerin koşul ağacı onun iki
    yazılışıdır (`data_condition`). SQLite'ta sayı koşulu metinleri de geçirir (`Condition::Text`).
  - Alttaki tablonun süzgeci tam gidiyorsa kaynak cevaplar, gitmiyorsa daraltılmış satırlar burada sorulur. Dışa
    aktarmanın ve canlı çizimin süzgeci de kaynağa gider.
  - Bir milyon PostGIS parselinde `kullanim = 'Konut'` sayımı 7 550 ms'den 28 ms'ye indi;
    `alan > 600 ve kullanim = 'Park'` 9 008 ms'den 26 ms'ye.
  - Kurallı sınıfların sayımı VY-13'te.
- [x] **VY-13 — Sınıflar ve stil.** ([ADR 9110](../docs/adr/9110-classes-from-the-source.md))
  - Stil penceresinin Sınıflandır ve Sınıfla düğmeleri veri katmanında değerleri kaynaktan alır; Nesne sütunu kaynağın
    satır sayılarını gösterir.
  - Katman ağacının sayımı kaynakta: kategorili stil tek `GROUP BY` (bir milyonda 8,1 sn'den 0,56 sn'ye), aralıklı
    stil sınıf başına koşulla (8,0 sn'den 0,12 sn'ye).
  - Sınıf satırına çift tık sınıfın sembolünü tasarımcıda açar.
  - Çizim katmanında kapalı sınıfın nesnesi etiketlenmez.
  - Değeri boş alan etikette boş görünür.
- [x] **VY-14 — Düzenleme.** ([ADR 9111](../docs/adr/9111-editing-pending-with-the-project.md))
  - Kaydedilmemiş yeni satırlar alttaki tablonun başında, “yeni” imiyle durur. Basmak nesneyi çizimde seçer.
  - Formu olmayan veri katmanında çizilen satır seçilir ve Öznitelikler açılır. “Çizimden sonra formu aç” ikisini de
    kapatır.
  - Bekleyen değişiklikler proje kaydedilince saklanır: yeni satırlar dosyada, alınan satırlar ve silinen anahtarlar
    **bu cihazda**, projenin yanında, yalnız kullanıcının okuyabildiği bir günlükte. Proje dosyasına kişisel veri
    yazılmaz (6698 md. 12).
  - Açılınca hepsi geri gelir. Kaydederken başkasının değiştirdiği satır sorulur.
  - Çıkarken “Projede tut” cevabı değişiklikleri tabloya yazmadan projede bırakır.
  - Bulunup kapatılan açık: proje kaydından sonra tabloya yazılan yeni satırlar dosyada kalıyordu. Proje yeniden
    açılıp kaydedilince tabloya ikinci kez yazılıyorlardı. Artık açılışta çizime alınmazlar.
- [x] **VY-15 — Dışa aktarmanın kalanları.** ([ADR 9112](../docs/adr/9112-export-streams-extent-systems.md))
  - GeoJSON, Shapefile ve KML akışlı yazılır. Bir milyon parselde en yüksek bellek GeoJSON 2 905 → 290 MB, KML 3 455 →
    284 MB, Shapefile 3 739 → 287 MB; çıktı bütün yazıcınınkiyle bayt bayt aynı.
  - Kapsam: Bütün katman, Görünümdekiler, Kutuya değenler. Kutu projenin eksenleriyle (Y doğu, X kuzey) yazılır,
    görünümden ya da katmanın kapsamından alınır, ya da haritada çizilir.
  - Koordinat sistemi aranabilir listeden seçilir; Yeni katman da aynı listeyi kullanır.
  - KML'ye semboller: katmanın stili (her sınıf kendi stili), katmanın rengi ya da sembolsüz. Başka bir CBS'nin
    kitaplığı dosyayı aynı renklerde çizdi.
- [x] **VY-16 — SQL sorgusu katmanı canlı.** ([ADR 9113](../docs/adr/9113-query-layers.md))
  - PostGIS'e bağlan'da yazılan sorgu bağlı kopya değil, sorguyu kaynağında okuyan veri katmanıdır.
  - Sütunları oku sorguyu çalıştırmadan sütunlarını getirir. Anahtar tam sayı sütunlardan, geometri geometri
    sütunlarından seçilir; ad verilir.
  - Ekle anahtarın satırları ayırdığını kaynakta denetler (1 000 000 satıra kadar).
  - Katman salt okunurdur: sorgu her yerde salt okunur bir işlemde ya da salt okunur açılmış dosyada çalışır.
  - Sorgu, katmanın tablo metnidir (`-- kentos: key=… geometry=…` satırıyla). Proje dosyası ve komut değişmedi.
  - Gerçek pencerede bir milyon parselden 5 000 park: tablo “5.000 satır · süzgeç ve sıra kaynakta”.
  - Bulunup kapatılan hata: SQL kutusu ilk harften sonra klavyeyi kaybediyordu.
- [x] **VY-17 — Taşımada yazılar.** ([ADR 9114](../docs/adr/9114-text-tables.md))
  - Taşıma yazı tutan katmanı artık reddetmiyor. Yazılar aynı kaynakta `<tablo>_yazilar` tablosuna yazılır: nokta,
    `metin`, `yukseklik` (m), `aci` (derece), `hiza` (Yazı'nın adları), `genislik`. Yalnız yazı tutan katman kendi
    tablosuna yazılır.
  - Yazı katmanı ana katmanın üstünde, “<ad> (yazılar)” adıyla gelir. Satırları CAD yazısıyla aynı yoldan yazı olarak
    çizilir.
  - Düzenlemede yazı olarak alınır, kaydedilince satırına döner. Yazı aracıyla yeni yazı eklenir.
  - Ölçü hâlâ reddedilir. Yazının stili, zemini, eğrisi gibi tablonun tutmadıkları uyarıyla söylenir.
  - Gerçek pencerede 20 direk ve 22 yazı: harita taşımadan sonra aynı; dosyayı başka bir CBS'nin kitaplığı da okudu.
- [x] **VY-18 — Küçük ölçekte ince çizgiler.** ([ADR 9115](../docs/adr/9115-data-layer-line-widths.md))
  - Model ekranında veri katmanının çizgileri kâğıt kalınlıklarını her ölçekte korur. Stil çekirdeği kâğıt
    milimetrelerini piksel olarak çizer; uzaklaşınca dünyayla incelip soluk bir piksele inmezler. Yakınlaşınca model
    ekranı onları zaten tutuyordu.
  - Çizim katmanları, işaretler, taramalar, pafta ve PDF değişmedi.
  - Gerçek pencerede bir milyon parsel: kent ve ilçe ölçeğinde yollar önce kayboluyordu, şimdi belirgin.

Sahibin sözü (22 Kasım 2026): “gis içerisinde geopackage 1. sınıf destek sunulmasını istedim ama geopackage açarken
içinde proje olup olmadığını arıyor, içinde kentos projesi yoksa bile açsın ama projeyi de oluştursun içine, geopackage
qgis kadar iyi kullanmalı.”

- [x] **VY-19 — GeoPackage proje olarak açılır.** ([ADR 9116](../docs/adr/9116-geopackage-opened-as-a-project.md))
  - Dosya › Aç, son açılanlar ve boş pencereye sürüklenen dosya GeoPackage'ı açar. Python'da `Document.open`, MCP'de
    `drawing.open` da açar.
  - Dosyanın tek projesi açılır. Birden çok projesi varsa adları söylenir, biri seçilir.
  - Projesi yoksa tablolarından yeni bir CBS projesi kurulur ve dosyanın içine yazılır; Kaydet oraya yazar:
    - tablolar veri katmanıdır, satırları dosyada kalır;
    - sistem, tabloların çoğunun projeye alınabilen sistemidir;
    - katmanlar noktalar, çizgiler, alanlar, geometrisiz tablolar sırasıyla gelir;
    - tablonun varsayılan stili (`layer_styles`) katmanın görünüşüdür.
  - Veri kaynağından aç'ta projesi olmayan dosya, yazılan adla proje olur.
  - 10 bağımsız durum (`scripts/fixtures/gpkg_open_cases.py`, sqlite3). Native, masaüstü, Python ve MCP sınamaları.
  - Kalan, QGIS'le arada: ilişkiler, üst veri, açılışta tablo seçimi, bakım (ADR 9116 §5). Raster döşemeleri ve
    stilin geri yazılması VY-20'de yapıldı. Sırayı sahip seçecek.

Sahibin seçimi (22 Kasım 2026): ADR 9116 §5'ten “Raster ve stil yazımı”. Aynı gün bildirdiği hata: “gpkg dosyalarını
açarken çok ciddi ölçek problemleri var, … zoom yapılamıyor ve ekranda nokta gibi çizimler”.

- [x] **VY-20 — GeoPackage'ın döşemeleri raster, görünüşler tabloya, derecede yakınlaşma.**
  ([ADR 9117](../docs/adr/9117-geopackage-rasters-and-looks.md))
  - Açılan dosyanın döşeme tabloları en altta bağlı raster olur. Sistem oyuna katılırlar, ilk görünüme girerler.
    Alınamayan söylenir.
  - GeoPackage'ta saklanan projede rasterin dosyası GeoPackage'a göre yazılır; taşınan dosyada raster bulunur.
  - Kaydet, değişen görünüşü tablonun varsayılan stili olarak `layer_styles`'a yazar (QML ve SLD). Değişmeyen
    yazılmaz, eski stiller durur. Python ve MCP'de kayıt cevabının `looks`'u.
  - `cad.layers.renderer` metin olmayan kategori değerini reddeder (önce sessizce boş oluyordu).
  - Derece projesinde (WGS 84) yakınlaşma artık metre projesindeki kadar derin: kameranın sınırı birimin metresiyle
    çarpılır. Önce 11 m/pikselde duruyordu, küçük alanlar nokta gibi kalıyordu.
  - 13 bağımsız durum. Native, kamera, masaüstü, Python, MCP sınamaları. QGIS 4.2.2 ile karşılaştırma. Sahibin indirdiği
    Boston ve NGA dosyalarının kopyalarıyla resimler. Gerçek pencerede raster ve görünüş: Katman stili'nde kapatılan
    sınıf Kaydet'le tabloya yazıldı, QGIS onu çizmedi.
  - Kalan (ADR 9117 §7): noktadan ibaret kutulu döşeme tablosu, başka sistemdeki döşemenin çevrilmesi, web.

## Üçüncü tur: belediye kullanım değerlendirmesi (22 Kasım 2026)

Sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Kaynak, release ikilisiyle gerçek pencerede
yapılan belediye kent bilgi sistemi değerlendirmesidir (32 madde ve 9 işletme kabulü; P0 veri bütünlüğü, P1 günlük iş,
P2 kullanılabilirlik). Raporun sırası izlenir: önce P0'lar (1, 2, 16, 25, 30), sonra 4, 5, 12 ve 17–24. Madde
numaraları raporunkidir.

- [x] **VY-21 — Bakmak dosyayı değiştirmez (rapor 1, P0).** ([ADR 9122](../docs/adr/9122-geopackage-looked-at-unchanged.md))
  - Projesi olmayan GeoPackage bellekte açılır; dosyanın baytları, tabloları ve indeksleri aynı kalır. Salt okunur dosya
    da açılır.
  - İlk Kaydet, Veri kaynağına kaydet penceresinde dosyayı ve adı gösterir; Vazgeç hiçbir şey yazmaz. Python ve MCP'de
    yolsuz kayıt saklar (`kept`).
  - “Kaydet ve kapat” veri kaynağında saklanan projede de kayıttan sonra kapatır.
  - Kalan: gerçek pencerede deneme, web.
- [x] **VY-22 — Proje kaydı ile kaynağa yazım ayrı (rapor 2, P0).**
  ([ADR 9123](../docs/adr/9123-project-save-and-source-writes-told-apart.md))
  - Kaynağa yazılmayan satırlar adın yanında amber rozet ve durum çubuğunda “Kaynağa yazılmadı: n katmanda m satır”
    hücresidir; projenin noktası yalnız projeyi söyler. Yazım reddedilince hücre tehlike renginde, neden yanında.
  - Katman değişiklikleri penceresi: katmanlar kaynaklarıyla, satırlar alan alan (kaynakta / bu cihazda); Kaynağa yaz,
    ikinci basışta Değişiklikleri geri al, Hepsini kaynağa yaz; bekleyenlerin nerede durduğu.
  - Kapatma sorusu: Kaynağa yaz, Taslak olarak tut, Değişiklikleri geri al, Vazgeç; her birinin etkisi yazılı. Formun
    Kaydet'i veri katmanında “bekliyor” der.
  - Günlüğün yeri ilk kayıtta da çözülmüş klasörle hesaplanır (bağlantılı klasörde bekleyenler açılışta geliyor).
  - Kalan: CBS durum çubuğunun dar penceredeki taşması (rapor 12), çevrimdışı tabloda bekleyenler (rapor 31), web.
- [~] **VY-23 — Topoloji veri katmanının bütün tablosunu denetler (rapor 16, P0).**
  ([ADR 9124](../docs/adr/9124-topology-over-a-data-layers-table.md))
  - Kuralların andığı veri katmanının satırları kaynaktan bütün geometrisiyle okunur; çizimin tuttuğu satırlar
    çizimdeki hâlleriyle. Sonuç yakınlaştırmadan bağımsız.
  - Satırlar katman ve anahtarla adlanır, bulgu seçilince tabloda seçilir; kaynaktaki satırı değiştiren düzeltme
    nedeniyle reddedilir. Okunamayan kaynak “denetlenemedi”, hiç nesne yoksa “temiz sayılmaz”.
  - Raporun verisiyle masaüstü ve çekirdek sınamaları (2.000 m² tek bulgu, kenardan temas temiz, çizim katmanında
    aynı sonuç).
  - Kalan: PostGIS durumu gerçek veritabanında koşturulmadı (Docker yanıt vermedi); kaynaktaki satırı kendiliğinden
    alarak düzeltme; web.
- [~] **VY-24 — Formla kayıt yazmak rolün kayıt iznine bağlı (rapor 25, P0).**
  ([ADR 9125](../docs/adr/9125-records-written-by-role-not-by-design-right.md))
  - `records_locked`: bulut projesinde `forms.use` ya da `feature.write`; arşiv ve izinsiz rol nedenini rolün adıyla
    söyler. Tasarım ve yayın `forms_locked`'ta (project.edit) kaldı.
  - Salt okunur form nedenini söyler (rol, arşiv, katman kilidi). Kaydet, iş akışı adımı, çoklu kayıt, alt kayıtlar
    aynı karardan geçer.
  - Kalan: iki gerçek hesapla bulut sunucusunda kabul (sunucu sınamaları veritabanı ister, Docker yanıt vermedi); web.
- [~] **VY-25 — Yazım makbuzları; yanıtı kaybolan COMMIT sorulur (rapor 30, P0).**
  ([ADR 9126](../docs/adr/9126-write-receipts-and-uncertain-commits.md))
  - `DataError::Uncertain`: PostGIS'te sunucudan gelmeyen COMMIT hatası; uygulamada `WriteFailure::Uncertain`, asla ret
    değil.
  - Makbuz satırlarla aynı işlemde `kentos_writes`'a (PostGIS şemada, GeoPackage dosyada); kurulum sahibin düğmesiyle.
  - Yanıt kaybında yazım ve gönderilenin anlık görüntüsü tutulur, günlüğe yazılır; sonraki Kaynağa yaz makbuzu sorar,
    bulunursa yazılanı çizimden çıkarır, bulunmazsa yazar. Makbuzsuzda kendiliğinden göndermez, Yeniden gönder…
  - Kayıp yanıtlı sınama kaynağı (`kentos_data::lossy`) ile çekirdek ve masaüstü sınamaları.
  - Kalan: gerçek PostgreSQL'de kesintiler (Docker yanıt vermedi), Python/MCP'de makbuz, web.
- [~] **VY-26 — CBS projesi binanın komutlarını sunmaz (rapor 12'nin sızıntısı, P1).**
  ([ADR 9127](../docs/adr/9127-cbs-project-offers-no-building-commands.md))
  - kentos-cad'de CBS projesi açıkken yalnız Mimari'nin şeridindeki komut aramada çıkmaz, çalıştırılınca nedeniyle
    reddedilir; Harita, CBS ve yolun komutları kalır.
  - CBS şeridinden bina IFC'si, UCS paneli ve Analiz'deki nokta bulutu araçları çıktı (bağlamsal sekmede ve aramada).
  - Kalan: görünürlük matrisi, CBS alt şeridindeki CAD terimleri, raster ve ağ için bağlamsal araçlar, web.
- [~] **VY-27 — Katmana bir basış hedefi seçer; veri katmanına oturumunda çizilir (rapor 4, P1).**
  ([ADR 9128](../docs/adr/9128-layer-target-and-data-layer-drawing.md))
  - CBS projesinde Katmanlar'da bir basış katmanı etkin yapar (CAD'de çift tıklama sürer).
  - Nesne yazan araç veri katmanında oturum kapalıyken, salt okunurken ya da geometri uymazken kapalı ve nedenli;
    çağrılınca Düzenlemeyi başlat düğmeli bildirim.
  - Kalan: Düzenle sekmesinde sürekli hedef ve oturum denetimi, yetki gerekçesi, iki oturumda yanlış yazım sınaması, web.
- [~] **VY-28 — Kaynak durumu veri katmanlarını sayar; tablo seçicisi aynı adlıları ayırır (rapor 5 ve 28, P1).**
  ([ADR 9129](../docs/adr/9129-data-layers-in-source-status-and-table-picker.md))
  - Kaynak durumu'nda veri katmanları: kaynak, erişim, oturum ve kaynağın gerçek satır sayısı (sorulana kadar
    “hesaplanmadı”); bağlı katman yokken boş kaynak denmez.
  - Denetçide “Veri katmanları” satırı; tablo başka katmandayken “Çizim hedefi”; Katman ▾'da aynı adlılar kaynakla,
    salt okunur işaretli.
  - Kalan: alınmış kopya ve bağlı katmanla tek katalog, form başlığında salt okunur, on geçişlik yazım sınaması, web.
- [~] **VY-29 — İstatistik ve Grafik veri katmanını kaynağından sayar (rapor 17, P1).**
  ([ADR 9130](../docs/adr/9130-statistics-and-charts-over-data-layers.md))
  - Veri katmanının satırları kaynaktan arka planda okunup saklanır (çizimin tuttukları o anki hâlleriyle); ortak
    okuyucu onları kullanır. Okunurken, okunamazken ve boşken ayrı sözler; bekleyen değişiklik söylenir.
  - Raporun bakım verisiyle sınama (5 dolu, 1 boş, toplam 500, ortalama 100, en az 0; durum 3 değer).
  - Kalan: veri katmanında Seçililer ve Görünümdekiler, grafikten seçim, sağlayıcının toplamıyla sayım, web.
- [x] **VY-30 — Katman süzgeci veri katmanında kaynağın satırlarını sayar (rapor 19, P1).**
  ([ADR 9131](../docs/adr/9131-layer-filter-counts-a-data-layers-rows.md))
  - Önizleme ve bildirim kaynağın satırlarını sayar (okunurken söyler, okununca yeniden sayar); “3 / 6 satır görünür”.
  - Kalan: sıfır eşleşmenin ayrı sınaması, web.
- [x] **VY-31 — Tablonun arama, ifade ve sırası katmanındır; katmanda olmayan alan söylenir (rapor 20, P1).**
  ([ADR 9132](../docs/adr/9132-table-conditions-by-layer.md))
  - Koşullar katmanın kimliği ve kaynağıyla saklanır, tablo katmana dönünce geri gelir; seçimle ya da etkin katmanla
    geçişte de izler.
  - `cad.layers.rows` katmanda olmayan alanı anan süzgeci `field_not_found` ile reddeder; çizim katmanının tablosu aynı
    sözü söyler. Reddedilen soru her iletide yeniden sorulmaz.
  - Kalan: koşulları proje dosyasında saklamak (sahibin kararı), web.
- [x] **VY-32 — İfadeyle seç veri katmanında koşulu kaynağa sorar (rapor 18, P1).**
  ([ADR 9133](../docs/adr/9133-select-by-expression-on-data-layers.md))
  - Koşul `layers_rows` ile kaynakta sorulur; seçim biçimi tablonun anahtar seçimine uygulanır; tablo katmana gelir,
    harita çerçeveler, Seçime yakınlaştır veri satırlarını da çerçeveler.
  - Girdi satırı ve önizleme kaynağın satırlarını söyler; Durdur seçimi korur; CAD özellik süzgeci reddedilir.
  - Kalan: Seçili kapsamı veri satırlarıyla, ölçülerde metre birimi, web.
- [x] **VY-33 — Birleşik görünüm veri katmanlarının satırlarını kaynaklarından okur (rapor 21, P1).**
  ([ADR 9134](../docs/adr/9134-join-views-over-data-layers.md))
  - `joins_query::read_with`: veri katmanı tarafı kaynağın satırları (`data_row_uid`) ve çizimin tuttukları; masaüstü
    `data_rows`'tan arka planda okur, görünüm beklerken söyler; hücreler yalnız okunur.
  - Okunamayan kaynak, boş katman ve eşleşme yok ayrı sözler; `all_records` geometrisiz satırı tutar.
  - Kalan: Kaynak tablosu seçeneğini veri sürücüsüne taşımak, kaynakta birleştirme ölçümü, web.
- [~] **VY-34 — İlişki üst katmanın kendi kodunu anahtar alabilir (rapor 22, P1).**
  ([ADR 9135](../docs/adr/9135-relation-parent-keys.md))
  - `ProjectRelation.parent_key` (CBS verisi 6); `ParentKey` tek kural: silme izleme, ilişkili kayıtlar, formun listesi ve
    sayımı, `ilişkili_toplam`, `cad.records.page`, formdan bağlama; benzersiz olmayan kodda çocuklar korunur.
  - İlişkiler penceresinde Üst anahtar ve Kaydet öncesi denetimler (benzersizlik, tür, bağlı, eşleşmeyen, NULL);
    İlişkiden kur anahtarla.
  - Kalan: veri katmanı üzerindeki formun ilişkili listesini kaynaktan okumak, gerçek pencerede silme denemesi, web.
- [~] **VY-35 — İşlemin tablosu dosyaya sütun türleriyle yazılır (rapor 23, P1).**
  ([ADR 9136](../docs/adr/9136-typed-table-outputs.md))
  - Tablo çıktısında isteğe bağlı `kinds` ve `values`; GeoPackage yazıcısı INTEGER/REAL sütun ve düz değer yazar,
    türsüz rapor metin kalır; PostGIS aynı geçici dosyadan geçer.
  - Histogram türlerini söyler; SQLite ve GDAL ile doğrulandı.
  - Kalan: öbür tablo veren araçların türleri, Kaydet ▾'ın sayıları, PostGIS sınamasının koşturulması, web.
- [x] **VY-36 — Bilgi yerel rasterin tıklanan hücresini söyler (rapor 24, P2).**
  ([ADR 9137](../docs/adr/9137-identify-on-local-rasters.md))
  - Bilgi'nin tıklamasında gösterilen rasterler arka planda sorulur; kart katman, bant değerleri (NoData ayrı), hücre ve
    Y, X söyler; kapsam dışı söylenir; nesne ile raster değeri arasında geçiş.
  - Kalan: üst üste rasterlerin listesi, uzak rasterler, web.
