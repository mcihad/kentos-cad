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
