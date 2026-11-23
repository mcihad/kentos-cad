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
- [~] **VY-25 — Yazımın işlem kaydı; yanıtı kaybolan COMMIT sorulur (rapor 30, P0).**
  ([ADR 9126](../docs/adr/9126-write-receipts-and-uncertain-commits.md); terim [ADR 9175](../docs/adr/9175-transaction-record-wording.md))
  - `DataError::Uncertain`: PostGIS'te sunucudan gelmeyen COMMIT hatası; uygulamada `WriteFailure::Uncertain`, asla ret
    değil.
  - İşlem kaydı satırlarla aynı işlemde `kentos_writes`'a (PostGIS şemada, GeoPackage dosyada); tabloyu sahibin
    “İşlem kaydı tablosunu oluştur” düğmesi oluşturur.
  - Yanıt kaybında yazım ve gönderilenin anlık görüntüsü tutulur, günlüğe yazılır; sonraki Kaynağa yaz yazımı işlem
    kaydına sorar, bulunursa yazılanı çizimden çıkarır, bulunmazsa yazar. İşlem kaydı tutmayan kaynakta kendiliğinden
    göndermez, Yeniden gönder…
  - Kayıp yanıtlı sınama kaynağı (`kentos_data::lossy`) ile çekirdek ve masaüstü sınamaları.
  - Kalan: gerçek PostgreSQL'de kesintiler (Docker yanıt vermedi), Python/MCP'de işlem kaydı, web.
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
- [~] **VY-37 — Proje tek adla görünür; yeni proje adını dosyasından alır (rapor 3, P1).**
  ([ADR 9138](../docs/adr/9138-one-project-name.md))
  - “Yeni proje” adlı proje kaydında dosyanın adını alır; `shown_name` denetçide ve açılış iletisinde; eski dosyalar dosya
    adıyla.
  - Kalan: sihirbazın ad, amaç, CRS ve saklamayı bir arada sunan ilk sayfası, boş kent haritası, son projeler, web.
- [~] **VY-38 — GeoPackage teslimi ne gittiğini söyler (rapor 6, P1).**
  ([ADR 9139](../docs/adr/9139-geopackage-delivery.md))
  - Özet: katmanlar sayılarıyla, giden tablo/stil/ilişki, geometrisiz kalanlar, gitmeyen form/görünüm/çalışma alanı,
    kopya oluşu; salt okunur hedef `file_unwritable`; eğri türleri `gpkg_geom_<tür>` ile.
  - Kalan: dosya seçildikten sonra onay, yarıda kesme sınaması, web.
- [~] **VY-39 — PostGIS kataloğu aranırken yerinde durur (rapor 7, P1).**
  ([ADR 9140](../docs/adr/9140-postgis-catalog-steady.md))
  - Tablolar sabit yükseklikte kaydırılır, arama üstte kalır; seçilen tabloların kimlik satırı (anahtar, geometri,
    SRID, yazma hakkı).
  - Kalan: şema ağacı, sık kullanılanlar, geometri sütunu adı ve kapsam, canlı rol/kesinti sınamaları, web.
- [~] **VY-40 — Form tasarımcısı alanı olmayan katmanı söyler, alanı yerinde oluşturur (rapor 8, P1).**
  ([ADR 9141](../docs/adr/9141-form-designer-new-field.md))
  - Paletin ve ağacın sözleri; Yeni alan `cad.layers.edit` ile katmana yazar ve forma koyar.
  - Kalan: yayımlama denetiminin ayrı sözleri, yayımlama rolü, yol formunda bütün sınama, web.
- [~] **VY-41 — Kapalı oturumda hücre yolunu söyler; tablo yüklenen satırı ayırır (rapor 9, P1).**
  ([ADR 9142](../docs/adr/9142-data-table-closed-cell-and-loaded-rows.md))
  - Çift tıklamada bildirim ve Düzenlemeyi başlat; satırda “(N yüklendi)”.
  - Kalan: iki sağlayıcıda aynı araç düzeni, uçtan uca zincir ve değer örnekleri, web.
- [~] **VY-42 — Katman stili uyan işleyicileri önce sunar; kategoriye girmeyenleri söyler (rapor 10, P1).**
  ([ADR 9143](../docs/adr/9143-renderers-by-geometry-and-unclassed-values.md))
  - `fits`/`kinds`: geometriye göre sıra ve “uymayan” başlığı; kategorili tablonun altında uyarı ve iki düğme.
  - Kalan: etiket önizlemesi, stil/QML kayıp bildirimi, değer değişiminde uçtan uca sınama, web.
- [~] **VY-43 — İşlemin girdisi çizimi izler; boş girdide Çalıştır kapalı (rapor 11, P1).**
  ([ADR 9144](../docs/adr/9144-processing-inputs-follow-the-drawing.md))
  - Son değerlerden nesne girdileri ve sonuç adı çıkar; seçim ya da aracın varsayılanı; `input_empty` Çalıştır'ı kapatır.
  - Kalan: türetilmiş ad, seçim satırı, iş kaydı, sınamalar, web.
- [~] **VY-44 — Etiket denetimi adayları ve nedenleri söyler; Bilgi örtüşen nesneleri sayar (rapor 13, P1).**
  ([ADR 9145](../docs/adr/9145-label-candidates-and-overlapping-hits.md))
  - `labelling/candidates.rs`: adaylar, örnek metin, ayrı nedenler, katmanda olmayan şablon alanı; Alan ekle.
  - Kartın altında katman ve kimlik; `ViewChange::InfoHits` ile “Bu noktada N nesne · i/N” ve Sonraki.
  - Kalan: görüntüleme alanı, nesne ile veri satırının birlikte sayılması, etiket önizlemesi, veri katmanında etiket, web.
- [~] **VY-45 — Topoloji yalnız okuduğu değişince eskir; tolerans karşılaştırmasıyla, kapsamıyla söylenir (rapor 14, P1).**
  ([ADR 9146](../docs/adr/9146-topology-staleness-by-inputs-and-scope.md))
  - `Checked.mark` ve `inputs_changed`: kural özeti, bağlı kaynak sürümü, kuralların katmanlarındaki şekil/katman.
  - `tolerance_words`: değerlerle karşılaştırma; sekmede “Kapsam: … · çizim sürümü N (şimdi M) · tolerans …”.
  - Kalan: veri katmanı kaynağının dışarıdan değişimi, PostGIS sınaması, kurumun gerçek eşikleri, web.
- [~] **VY-46 — Yeni pafta kapsamı önce seçilir, ölçeği söylenir; baskı denetimine belediye senaryoları (rapor 15, P1).**
  ([ADR 9147](../docs/adr/9147-new-sheet-extent-and-print-checks.md))
  - `sheet-ui/new_extent.rs`: Görünüm, Katman, Çizimin tamamı, Sabit ölçek; `fit::map` ile ölçek ve söz.
  - Masaüstü katman kutuları; iki pafta/Türkçe harf, boş atlas ve lejant senaryoları.
  - Kalan: resimli önizleme, sorusuz şablon, çevrimdışı servis ve dış kaynakla yeniden basım, çok sayfalı atlas kabulü, web.
- [~] **VY-47 — Kaynağa yazmada çakışma alan alan: ayrı alanlar birleşir, aynı alan seçilir (rapor 26, P1).**
  ([ADR 9148](../docs/adr/9148-field-level-write-conflicts.md))
  - `data_clash`: ilk/benim/tablodaki karşılaştırması ve birleştirme; `WriteFailure::Conflict` satır çakışmalarını taşır.
  - `data_conflict.rs` penceresi; `data_rebase`, `data_untake`, `data_let_go`; seçimden sonra yeniden kayıt ve denetim.
  - Kalan: gerçek PostgreSQL'de iki oturum, Python/MCP'de alan alan seçim, uzun değerler, web.
- [~] **VY-48 — Formla düzenle tablodan seçilen kaynak satırına ulaşır (rapor 27, P1).**
  ([ADR 9149](../docs/adr/9149-form-edit-from-a-table-row.md))
  - `forms/row_target.rs`: alınmışsa o nesne, düzenleniyorsa okunup alınır, kapalıysa Düzenlemeyi başlat, salt okunur söylenir.
  - Form başlığında anahtar; işlem aracında seçim yalnız ilk nesne girdisinin (ADR 9144 daraltıldı).
  - Kalan: salt okunur satırın formda gösterilmesi, PostGIS sınaması, web.
- [~] **VY-49 — Rolün okumadığı alan boş gösterilmez, adıyla söylenir (rapor 29, P2).**
  ([ADR 9150](../docs/adr/9150-fields-the-role-does-not-read.md))
  - `features/role_fields.rs`: `withheld_fields` ortak çözümleyiciden; tabloda sütun yok, alt satır adını söylüyor; kartta satır yok.
  - Kalan: NULL ile boş metnin ayrı görünmesi, veri katmanı tablosunda rol süzgeci, bulutta üç hesapla kabul, web.
- [~] **VY-50 — Kaynak okunamazken tablo bekleyen değişiklikleri gösterir (rapor 31, P1).**
  ([ADR 9151](../docs/adr/9151-pending-rows-when-the-source-is-unread.md))
  - `features/pending_rows.rs`: değişen satır yerel ve önceki değerle, silinen anahtarıyla; “doğrulanmadı” sözü.
  - Kalan: son doğrulama zamanı, gerçek PostgreSQL kesintisi, web.
- [~] **VY-51 — Kaynakta silinen satırın taslağı bırakılınca sağlam satırlar yazılır (rapor 32, P1).**
  ([ADR 9152](../docs/adr/9152-a-row-gone-in-the-source.md))
  - Pencerede yazılacak/bırakılacak sayı; tabloda “kaynakta silinmiş” satırlar; değişmeyen alan kaynağın değeri; geri almanın sayısı.
  - Kalan: gerçek PostgreSQL'de deneme, yeni kayıt yetkisinin sorulması, web.
- [~] **VY-52 — Birleşik görünüm çoklu ve yetim eşleşmeyi sayar; ilişki kaldırma etkisi önceden (ek kabul: ilişkiler).**
  ([ADR 9153](../docs/adr/9153-join-counts-and-relation-removal-preview.md))
  - Rapor satırında “birden çok kayıtla” ve “hiçbir kayıt bağlanmadı”; İlişkiler'de kaldırılacak ilişkinin etkisi.
  - Kalan: yeniden adlandırma etkisi, alt kayıtların eski form sürümü, yol sınıfı kodlarıyla sınama, web.
- [~] **VY-53 — Adres aramasında eşit adaylar mahalleleriyle söylenir (ek kabul: adres ve istatistik).**
  ([ADR 9154](../docs/adr/9154-address-ties-named-by-neighbourhood.md))
  - Belirsiz'de “2 aday aynı puanda (100): Cumhuriyet, Kızılay mahallelerinde; mahalleyi ekleyerek ayırın”.
  - Kalan: veri katmanında seçili/görünümdeki satırlarla istatistik, tablo/grafik/dışa aktarım toplamlarının uçtan uca sınaması, web.
- [~] **VY-54 — Taşınan dosyanın veri katmanları bir kerede yeniden bağlanır (ek kabul: kurtarma ve taşınabilirlik).**
  ([ADR 9155](../docs/adr/9155-relink-data-layers-of-a-moved-file.md))
  - `data_relink.rs`: aynı adresteki bütün veri katmanları; anahtar/geometri/alan şeması denetimi; Kaynak durumunda düğme.
  - Kalan: üç durumun tek pencerede bildirimi, kesinti/yedek sonrası sayım, kalıcı kimlikle denetim, web.
- [~] **VY-55 — İstatistik ve Grafik veri katmanının seçili ve görünümdeki satırlarını sayar (rapor 17'nin kalanı).**
  ([ADR 9156](../docs/adr/9156-statistics-of-chosen-and-seen-data-rows.md))
  - Satırlar anahtar ve kutularıyla; Seçililer tablonun seçimi, Görünümdekiler görünümün kutusu.
  - Kalan: tablo/grafik/dışa aktarım toplamının tek sınaması, kaynak satırlarının ölçüleri, web.
- [~] **VY-56 — İşlemin girdisi seçimin katmanını ve katmanın sayısını söyler (rapor 11'in kalanı).**
  ([ADR 9157](../docs/adr/9157-processing-selection-names-its-layer.md))
  - “seçili, “Parsel” katmanından (katmanda 1 nesne)”; birden çok katmanda kaç katmandan.
  - Kalan: türetilmiş ad, iş kaydının alanları, sınamalar, web.
- [~] **VY-57 — Özet istatistik tablosu dosyaya sayı olarak yazılır (rapor 23'ün kalanı).**
  ([ADR 9158](../docs/adr/9158-typed-summary-statistics-table.md))
  - Nesne/Değer INTEGER, toplam ve ölçüler REAL, boş hücre NULL; kesin metin pencerede kalır.
  - Kalan: öbür tablo veren araçlar, Kaydet ▾ biçimleri, PostGIS sınaması, web.
- [~] **VY-58 — Salt okunur bağlantının satırı formda yalnız görüntülenir (rapor 27 ve 28'in kalanı).**
  ([ADR 9159](../docs/adr/9159-read-only-row-in-its-form.md))
  - Satır kaynağından okunur, çizime alınmaz; başlık “fid 22 · salt okunur”, Kaydet yazmaz; on geçişlik yazım sınaması.
  - Kalan: iki PostGIS bağlantısıyla on geçiş, salt okunur formda ilişkili kayıtlar, web.
- [~] **VY-59 — Bilgi kartının başlığı kaydı görüntüleme alanıyla adlandırır (rapor 13'ün kalanı).**
  ([ADR 9160](../docs/adr/9160-display-field-in-the-card-title.md))
  - Adı ad sözü olan alan, yoksa sözcüklerinde ad sözü olan; “ada/adres” değil; rolün okumadığı alan değil.
  - Kalan: aynı tıklamada nesne ile veri satırı, etiket önizlemesi, veri katmanında etiket, yer bulucu, web.
- [~] **VY-60 — Yayın denetimi formu durduran sorunları türleriyle ayrı söyler (rapor 8'in kalanı).**
  ([ADR 9161](../docs/adr/9161-publish-check-kind-by-kind.md))
  - Boş form, zorunlu alan, varsayılan değer, değer listesi, öbürleri; komutun reddi hepsini söyler, boş form yayınlanmaz.
  - Kalan: yayımlayan rol, yol formunda bütün sınama, kodun katalog açıklaması, web.
- [~] **VY-61 — Silinmiş satır yeni satır olarak yalnız ekleme yetkisiyle yazılır (rapor 32'nin kalanı).**
  ([ADR 9162](../docs/adr/9162-new-row-only-with-the-insert-right.md))
  - PostGIS INSERT/UPDATE/DELETE yetkilerini ayrı okur; yetkisiz bağlantıda silinmiş satırın taslağı bırakılır.
  - Kalan: gerçek PostgreSQL'de ikinci hesapla deneme, sütun düzeyinde yetkiler, web.
- [~] **VY-62 — Bekleyen satırlar kaynağın son doğrulama zamanını söyler (rapor 31'in kalanı).**
  ([ADR 9163](../docs/adr/9163-last-verification-of-pending-rows.md))
  - Sayfa okunduğunda ve kaynağa yazıldığında zaman tutulur; günlükle gider, çevrimdışı açılışta “son doğrulama” der.
  - Kalan: gerçek PostgreSQL kesintisi, satır başına alma anı, kaynak hatalarının Türkçesi, web.
- [~] **VY-63 — Etiket penceresi aynı veriden örnekler gösterir (rapor 10'un kalanı).**
  ([ADR 9164](../docs/adr/9164-label-examples-on-the-same-data.md))
  - İlk ve en uzun metin, boş sayısı, görünür ölçekler ve şimdiki görünüm, kâğıtta yazı yüksekliği.
  - Kalan: QML kayıpları, planlı→kontrol sınaması, kendi biçimiyle önizleme, kurallı sınıfların aralığı, web.
- [~] **VY-64 — Formun ilişkili listesi veri katmanının satırlarını kaynaktan okur (rapor 22'nin kalanı).**
  ([ADR 9165](../docs/adr/9165-related-rows-from-the-source.md))
  - Üst anahtarla bağlı veri katmanı öğesi kaynaktan okunur; satırlar `kaynak:<anahtar>` ile dünyaya girer.
  - Kalan: kaynak satırının formunu listeden açmak, 500'den çok satırda sayfalı okuma, silmenin gerçek pencerede denenmesi, web.
- [~] **VY-65 — Veri katmanının tablo çubuğu çizim katmanınınkiyle aynı düzendedir (rapor 9'un kalanı).**
  ([ADR 9166](../docs/adr/9166-data-table-bar-like-the-drawings.md))
  - Aynı araçlar aynı sırada; Sütunlar, Dışa aktar, İncele, Alanlar çalışır; öbürleri kapalı, ipucunda neden ve sonraki adım.
  - Kalan: uçtan uca zincir ve örnek değer sınaması, veri katmanında Form bölmesi ve kopyalama, web.
- [~] **VY-66 — Bilgi aynı tıklamadaki nesneleri ve veri satırlarını birlikte sayar (rapor 13'ün kalanı).**
  ([ADR 9167](../docs/adr/9167-objects-and-data-rows-counted-together.md))
  - Nesnenin yanında veri katmanlarının satırları da sorulur; “1 nesne, 1 veri satırı · 1/2”, Sonraki satırlara geçer.
  - Kalan: etiket önizlemesinin biçimi, veri katmanında etiket, bir katmandan birden çok satır, web.
- [~] **VY-67 — Var olan GeoPackage'ın üstüne yazmadan önce silinen ve yazılan tablolar söylenir (rapor 6'nın kalanı).**
  ([ADR 9168](../docs/adr/9168-writing-over-a-geopackage-asks.md))
  - Seçilen dosya varsa onay: silinen, yeniden yazılan ve yeni tablolar, stil tablosu, yazılan sayılar; Geri ya da Üstüne yaz.
  - Kalan: yalnız hedef tablolara yazmak ya da eklemek, yarıda kesme sınaması, web.
- [~] **VY-68 — Düzenle sekmesi çizilen veri katmanının oturumunu sürekli gösterir (rapor 4'ün kalanı).**
  ([ADR 9169](../docs/adr/9169-session-in-the-edit-tab.md))
  - Katmanlar panelinde Düzenlemeyi başlat/bitir; salt okunur bağlantıda kapalı, ipucunda nedeni.
  - İki açık oturumda alan yalnız hedefe gidiyor (sınama).
  - Kalan: iki oturumun PostGIS'te denenmesi, bulut rolünün gerekçesi, web.
- [~] **VY-69 — Seçimi kaldır veri tablosunun seçimini de temizler; tablo zinciri uçtan uca sınanır (rapor 9'un kalanı).**
  ([ADR 9170](../docs/adr/9170-deselect-data-rows-and-the-table-chain.md))
  - Seç→kart→form→süz→temizle zincirinde katman ve anahtar korunur; 0 değerdir, NULL ve boş metin dilin tek boşudur; Türkçe sıra ve arama.
  - Kalan: kodlu değer, NULL/boş metin işareti (29), PostGIS'te zincir, web.
- [~] **VY-70 — Topoloji sonucu veri katmanının kaynağı dışarıda değişince eskir (rapor 14'ün kalanı).**
  ([ADR 9171](../docs/adr/9171-topology-old-after-an-outside-write.md))
  - Okunan kaynağın adresi, tablosu ve dosyanın parmak izi saklanır; değişince not ve tam Denetle.
  - Kalan: PostGIS'in dışarıdaki yazımı, kendi yazımını ayırmak, kurum eşikleri, web.
- [~] **VY-71 — PostGIS kataloğu şema ağacı ve sık kullanılan tablolarla (rapor 7'nin kalanı).**
  ([ADR 9172](../docs/adr/9172-catalog-schema-tree-and-favourites.md))
  - Şemalar açılıp kapanır, arama eşleşenleri açar; yıldızlı tablolar başta, bağlantının adıyla cihazda saklanır.
  - Kalan: ilişkili öznitelik tabloları, geometri sütunu adı ve kapsam, canlı sınamalar, web.
- [~] **VY-72 — Stil dosyası alışverişinde her kayıp ayrı satırda söylenir (rapor 10'un kalanı).**
  ([ADR 9173](../docs/adr/9173-style-file-losses-one-by-one.md))
  - QML ve SLD'nin yazma ve almasında başlık ve sayı, ardından her kayıp numaralı satırda; birleşik uyarı yinelenmez.
  - Kalan: planlı→kontrol uçtan uca sınaması, kayıpların pencerede tablosu, web.
- [~] **VY-73 — Masaüstü formu yayımlama iznini tasarlama izninden ayırır (rapor 8'in kalanı).**
  ([ADR 9174](../docs/adr/9174-the-publish-right-on-the-desktop.md))
  - `forms.publish` yoksa Yayınla kapalı ve nedeni rolün adıyla; taslak, yayından kaldırma ve kurallar tasarım izniyle.
  - Kalan: yol formunda bütün sınama, tasarlayıp yayımlamayan hazır rol (sahibin kararı), web.
- [x] **VY-74 — Yazımın kaynaktaki izi “işlem kaydı”dır; düğme “İşlem kaydı tablosunu oluştur” (sahibin sözü).**
  ([ADR 9175](../docs/adr/9175-transaction-record-wording.md))
  - “Yazım makbuzu” ve “Yazım makbuzlarını kur” bırakıldı: makbuz ödeme ve teslim belgesidir, tablo oluşturulur.
  - Tablonun adı (`kentos_writes`) ve kodun İngilizce adları aynı; masaüstü sınaması düğmeyi ve tablonun oluşmasını
    denetler. Web: TODOS.md §0.2.
- [~] **VY-75 — Kaynaktaki alt kayıt ilişkili listeden kendi formuyla açılır (rapor 22'nin kalanı).**
  ([ADR 9176](../docs/adr/9176-open-a-source-row-from-the-related-list.md))
  - Listede Aç, tabloda ve kartta aç simgesi; alt katmanın formu üst kaydın formunun üstünde açılır.
  - Alt katman düzenleniyor ve öğe izin veriyorsa satır oturuma alınır; değilse gösterilir, form nedenini söyler.
  - Kalan: 500'den çok alt kayıtta sayfalı okuma, gösterilen kayıttan oturumu başlatıp yazılabilir açmak, web.
- [x] **VY-76 — Yolun planlı → kontrol geçişinde sayaç, harita, kâğıt, tablo ve lejant tek kuralı izler (rapor 10'un
  kalanı).** ([ADR 9177](../docs/adr/9177-one-rule-when-a-value-changes.md))
  - Veri katmanının geometri sınıfı tablosundan: lejant (pencere ve pafta) veri katmanını gösterir, Katman stili yalnız
    uyan sembolü sunar.
  - Uçtan uca sınama: sayaç, ekran, kâğıt, tablo ve lejant Diğer değerler kapalı ve açıkken aynı şeyi söyler.
  - Kalan: gerçek pencerede ve PostGIS'te deneme; web (TODOS.md §0.2).
- [~] **VY-77 — GeoPackage teslimi yarıda kesilince dosya bütün kalır; öbür tabloları korumanın yolu söylenir (rapor
  6'nın kalanı).** ([ADR 9178](../docs/adr/9178-a-killed-geopackage-delivery-stays-whole.md))
  - Gerçek süreç ölümüyle sınama: on SIGKILL'in her birinde adın altında bütün bir GeoPackage kalıyor.
  - Üstüne yaz onayı silinecek tabloların altında katmanın kendi dışa aktarmasını (Katmanı değiştir, Katmana ekle)
    söylüyor.
  - Kalan: teslimde “yalnız bu tablolar” ve “ekle” (komut girdisi), uzun teslimin Durdur'u, web.
- [~] **VY-78 — Tampon'un durumları ayrı sınanır; iş kaydı çalıştırmayı sözle tutar (rapor 11'in kalanı).**
  ([ADR 9179](../docs/adr/9179-buffer-cases-and-the-run-in-words.md))
  - `RunRecord::words`: girdi çözüldüğü gibi, sayı birimiyle, seçenek adıyla, yazılan katman yeni ya da var olan;
    Geçmiş kartında.
  - Tampon geçersiz girdiyi söyler, onarmaz; metre, coğrafi, Durdur, papyon ve var olan ad ayrı sınanıyor.
  - Kalan: girdiden türetilen sonuç adı (ortak durumlarla), geçersiz girdi sözünün öbür araçlara yayılması, web.
- [x] **VY-79 — Yol formu aynı veride baştan sona sınanır; yeni alt kayıt üst kaydın kodunu alır (rapor 8'in
  kalanı).** ([ADR 9180](../docs/adr/9180-the-road-form-end-to-end.md))
  - Değer listesi, koşullu alan, alt bakım kayıtları, rol kısıtı ve sürüm yükseltmesi tek yolda birlikte sınanıyor.
  - Kusur: Yeni alt kayıt kimlik yazıyordu; artık ilişkinin üst anahtarının değerini (yolun kodu) yazıyor.
  - Kalan: kısıtın kaynakta da uygulanması (GeoPackage, PostgreSQL), web (TODOS.md §0.2).
- [x] **VY-80 — İstatistik ve Grafik veri katmanını kaynağın kendi sayımıyla sayar (rapor 17'nin kalanı).**
  ([ADR 9181](../docs/adr/9181-statistics-counted-by-the-source.md))
  - Tümü'de her alan kaynağın `GROUP BY`'ıyla sayılır; satırlar okunmaz. Toplamı ve ortalamayı KentOS kesin ondalıkla
    yapar (kaynağın `SUM`/`AVG`'si kayan noktalıdır).
  - Sonuçlar okunan satırlarınkiyle aynı, oturumdaki satırlar dahil. Tek alan okuyan grafik de kaynağın sayımıyla
    çizilir.
  - Bir milyon parselin yedi alanı: 31 931 ms ve 5 536 MB yerine 5 713 ms ve 736 MB (hata ayıklama derlemesi);
    özetler harfi harfine aynı. PostGIS'te (sınandı) 37 134 ms ve 5 536 MB yerine 6 572 ms ve 791 MB.
  - Değere ya da çubuğa basmak o satırları tabloda seçer (Shift ekler, Ctrl çıkarır). Satırları okunan yoldaki
    olmayan yuva kusuru düzeldi.
  - Kalan: Seçililer, Görünümdekiler ve iki alanlı grafiklerin kaynakta sayımı; sürüm derlemesinde ölçüm; gerçek
    pencere; web.
- [~] **VY-81 — Değerlendirmenin PostgreSQL kalanları gerçek sunucuda sınanır.**
  ([ADR 9182](../docs/adr/9182-postgresql-remainders-on-a-real-server.md))
  - Sınama düzeneği `pg_testing.rs`: sınamanın tablosu ve hesabı, ikinci okuyucu; COMMIT'i sunucuya ulaşmadan ya da
    ulaştıktan sonra kesen ve kapanınca sunucuyu ulaşılmaz yapan bir aktarıcı (gerçek kesinti).
  - Geçti: 4, 16, 23 (iki sınama kusuru düzeldi), 26 (iki oturum), 27, 28, 30 (yeniden açılış dahil), 31, 32 (ekleme
    yetkisiz hesap).
  - 14: `Source::change_mark` (PostGIS'te tablonun istatistik sayaçları); topoloji sonucu dışarıdaki yazımla
    eskiyor, işaret sekme açıkken arka planda soruluyor.
  - Kalan: öykünmesiz bağımsız sunucuda yineleme (7 → VY-82).
- [x] **VY-82 — PostGIS kataloğu seçilen tablonun geometri sütununu, kapsamını ve ilişkili tablolarını söyler (rapor
  7'nin kalanı).** ([ADR 9183](../docs/adr/9183-catalog-column-extent-and-related-tables.md))
  - Katalog geometri sütununun adını ve yabancı anahtarla bağlı tabloları okur; seçilen tablonun kimliği bunları ve
    arayüzü bekletmeden sorulan kapsamı söyler.
  - İlişkili tablolar seçilen tablonun altında düğmeyle seçilir, geometrisiz tablolar listelenmiyorken de.
  - Gerçek PostgreSQL'de sınandı: anahtarsız görünüm, salt okunur hesap, kesinti ve yeniden listeleme.
  - Kalan: gerçek pencerede deneme, web.
- [x] **VY-83 — CBS projesi tek sayfada kurulur: ad, amaç, konum ve saklama (rapor 3'ün kalanı).**
  ([ADR 9184](../docs/adr/9184-cbs-project-on-one-page.md))
  - Sihirbazın CBS adımları ikidir; ad, amaç, il ve koordinat sistemi, saklama aynı sayfada. Kartlar sonuçlarını birer
    cümleyle söyler; veri notu bağlamakla kopyalamayı ayırır.
  - Boş kent haritası yalnız Taslak katmanıyla başlar; Kadastro paftası öntanımlı kalır.
  - Kurum şablonu arayüzü bekletmeden okunur: katmanları, stilleri ve kurum ayarları gelir; nesneleri, paftaları ve
    kendi işi gelmez; CAD projesi reddedilir.
  - GeoPackage ya da PostgreSQL seçilince proje hiçbir yere yazılmaz; ilk Kaydet Veri kaynağı'nı o yerle ve adıyla açar.
  - Kalan: betikte amaç ve şablon, şablonun paftaları, gerçek pencere, web.
- [x] **VY-84 — CBS'nin görünürlüğü katmana ve seçime göre; alt şeritte CAD terimi yok (rapor 12'nin kalanı).**
  ([ADR 9185](../docs/adr/9185-cbs-visibility-by-layer-and-selection.md))
  - Seçim yokken etkin katman bağlamsal sekmeyi açar: raster, nokta bulutu, ağın katmanı (yeni Ağ sekmesi, CBS).
    Seçim önce gelir; sekme öne kendiliğinden geçmez, başka katmanda kapanır.
  - CBS'nin alt şeridinde Kalınlık lambası yok (Çizgi kalınlığı Görünüm'de); çizimin sekmesi “Harita”.
  - Görünürlük matrisi: boş CBS, nokta, çizgi, alan, raster, nokta bulutu, ağ, salt okunur tablo, oturumdaki tablo,
    karışık seçim; CAD'den CBS'ye dönüş; IFC CBS'de erişilmez, binanın CAD projesinde erişilir.
  - Kalan: yetki ve sağlayıcıya göre şeridin kendisi, CBS'ye özgü panel düzeni, gerçek pencere, web.
- [x] **VY-85 — Etiketler örneği haritanın biçimiyle çizer; veri katmanında örnekler ve Denetle (rapor 13'ün kalanı).**
  ([ADR 9186](../docs/adr/9186-label-example-and-data-layer-labelling.md))
  - “Haritada böyle görünür” şeridi ilk metni stilin rengi, halesi, zemini, gölgesi ve yazısıyla çizer; görünüş
    haritanın yerleşen etiketleriyle ortak işlevdendir (`placed_look`).
  - Veri katmanında örnekler tablonun ilk 64 satırından, arayüzü bekletmeden okunur; özet katmanı nesne saymaz.
  - Denetle veri katmanının etiketlerini görünümün okuduğu satırlarda sayar; okuma yokken nedenini söyler.
  - Kalan: şeritte eğri ve çok satırlı etiket, gerçek pencere, web.
- [x] **VY-86 — Yeni paftanın resimli kapsamı, sorusuz şablon, erişilemeyen katmanla yeniden basım, çok sayfalı atlas
  (rapor 15'in kalanı).** ([ADR 9187](../docs/adr/9187-sheet-extent-picture-reprint-and-atlas-acceptance.md))
  - Kapsam'ın resmi kâğıdı, ana haritanın çerçevesini, çizimin ve seçilen kutuyu haritanın göstereceği yerde çizer;
    sabit ölçekte kırpma görünür.
  - Sorusuz ama ana haritalı şablon yalnız Kapsam'ı sorar; haritasız ve sorusuz şablon hemen yapılır.
  - Ön denetim dosyası olmayan GeoPackage'ı, okunamayan kaynağı ve yanıt vermeyen servisi `layer_unreachable` uyarısıyla
    söyler; pafta yine basılır.
  - Yüz sayfalık atlas tek PDF; `pdfinfo` ve `gdalinfo` ile sayfa sayısı ve sayfaların yer bilgisi sınandı.
  - Kalan: servisin önbellekteki karolarla basımının ayrımı, resimde dönüş ve genel bakış, gerçek pencere, web.
