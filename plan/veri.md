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
- [ ] **VY-02 — Katman türleri ve veri katmanı.**
  - Projede “veri katmanı” ayrı bir tür olur. Taşıdıkları:
    - kaynak: sürücü, adres ve tablo;
    - şema, geometri türü ve sistem;
    - anahtar ve alt küme süzgeci;
    - salt okunur ya da düzenlenebilir olduğu.
  - Satırları çizime kopyalanmaz. Katmanlar'da türüyle görünür: çizim katmanı, veri katmanı (nokta, çizgi, alan,
    tablo), raster, servis, nokta bulutu, grup.
  - Kaynaklar'dan ve PostGIS'e bağlan'dan eklemek yalnız kataloğu okur.
  - Proje kapatılıp açılınca kaynak yeniden bağlanır; parola projeye yazılmaz.
- [ ] **VY-03 — Canlı çizim.**
  - Veri katmanı görünen alanı arka planda okur ve parça geldikçe çizer; görünüm başka yere gidince eski istek durur.
  - Sunucuda sadeleştirilmiş geometri; aynı görünüm için önbellek.
  - Kategorili, dereceli ve kurallı stiller ile etiketler, çizim katmanlarıyla aynı motordan geçer.
  - Bilgi aracı, seçim ve kenet kaynaktan sorar.
  - Bütçe ADR 9100 §9'da.
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
