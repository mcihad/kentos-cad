# KentOS — Veri yönetimi planı (veri sürücüleri, canlı katman, düzenleme oturumu)

Başlangıç: **20 Kasım 2026**. Sahibin isteği (özetle): GeoPackage ve PostGIS için birinci sınıf destek, ileride Oracle,
MySQL ve SQL Server'ın gelebileceği ayrı crate'lerde bir sürücü soyutlaması; veritabanından eklenen katmanın bütün
veriyi önbelleğe almaması, görüneni gerçek zamanlı okuması; alttaki tablonun süzme ve aramasının veritabanında yapılması;
kategorili/dereceli/kurallı stillerin sınıflarının katmanın altında görünmesi ve açılıp kapanması; düzenleme modu,
kaydetme yapısı, işlem (transaction) yönetimi, etkin katman ve katman türlerinin bir CBS'ye uygun hâle gelmesi
(“şu anki yapı uygun değil”); katmanın sağ tuşunda ayrıntılı bir dışa aktarma penceresi; GeoPackage projeleri.

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
- [ ] **VY-03 — Canlı çizim.** ([ADR 9101](../docs/adr/9101-data-layers.md) §6; çekirdeği yapıldı, etiket ve bilgi
  aracı sürüyor)
  - Veri katmanı görünen alanı ve her yandan yarısı kadar payı arka planda okur, parça geldikçe çizer. Görünüm başka
    yere gidince eski istek durur.
  - Geometri yarım piksele sadeleştirilir; PostGIS'te sunucuda. Pay içindeki kaydırma yeniden okumaz.
  - Kategorili, dereceli ve kurallı stiller çizim katmanlarıyla aynı motordan geçer.
  - Gerçek pencerede bir milyon PostGIS parseli ve bir GeoPackage'ın yolları ve direkleri, bütün kentten sokağa kadar
    kaynaktan çizildi. Çizimde 0 nesne vardı.
  - Kalan:
    - veri katmanının etiketleri;
    - bilgi aracının kaynaktan sorması;
    - seçim VY-04'te (tablo ile birlikte), kenet VY-05'te (düzenleme oturumu ile birlikte);
    - stil penceresinin sınıflandırmasının sürücüden okuması VY-07'de.
- [ ] **VY-04 — Alttaki tablo kaynakta.**
  - Öznitelik tablosu veri katmanında sayfa sayfa okunur.
  - Sıralama, süzgeç, arama ve sayı SQL'de; “görünümdekiler” ve “seçililer” kaynaktan.
  - Hücre düzenlemesi düzenleme tamponuna yazılır (VY-05).
- [ ] **VY-05 — Etkin katman ve düzenleme oturumu.**
  - Katman düzenlemeye açılır ve kapatılır.
  - Değişiklikler önce katmanın tamponunda bekler: eklenen, değişen ve silinen kayıtlar, kendi geri alma adımlarıyla.
  - Çizim araçları etkin ve düzenlenen katmana yazar.
  - Kaydet, Geri al ve Vazgeç; seçili ya da bütün katmanlar için.
  - Kaydedilmemiş değişiklik katmanda işaretlenir; düzenleme kapatılırken ve proje kapanırken sorulur.
  - Kaydetme hatasında tampon korunur.
- [ ] **VY-06 — İşlem yönetimi.** Proje ayarı, üç kip:
  - yerel düzenleme tamponu;
  - aynı veritabanının katmanları için otomatik işlem grubu;
  - bütün düzenlenebilir katmanların birlikte açılıp kapandığı tamponlu işlem grubu.
  
  Kaynak başına tek işlemde yazılır; başkasının değiştirdiği kayıtla çakışma söylenir.
- [ ] **VY-07 — Sınıflar katman ağacında.**
  - Kategorili, dereceli ve kurallı stilin sınıfları katmanın altında listelenir; her birinin kutusu o sınıfı açar ya da
    kapar.
  - Sağ tuş: Hepsini göster, Hepsini gizle, Tersine çevir, Bu sınıfın nesnelerini seç, Tabloda göster, Sembolü
    düzenle, renk.
  - İsteğe bağlı nesne sayısı.
- [ ] **VY-08 — Dışa aktarma penceresi.** Katmanın sağ tuşunda Dışa aktar › Farklı kaydet… ve Seçilenleri farklı
  kaydet…. Seçenekler:
  - biçim, dosya ya da veritabanı tablosu, katman adı, koordinat sistemi, kodlama;
  - yalnız seçilenler, kapsam, süzgeç;
  - alanlar: seçim, yeniden adlandırma, görünen değer;
  - geometri türü, çok parçalıya zorla, Z/M;
  - biçime özel seçenekler;
  - var olan dosyada: üstüne yaz, katmanı değiştir ya da ekle;
  - üst veri, haritaya ekle.
- [ ] **VY-09 — Projeler GeoPackage'ta ve PostgreSQL'de.**
  - Proje GeoPackage dosyasının içine (`kentos_projects`) ve bir PostgreSQL şemasına kaydedilir ve açılır.
  - Kaynaklar'da dosyanın katmanlarıyla birlikte görünür.
  - Aynı dosyadaki katmanlar dosyayla birlikte taşınır.
