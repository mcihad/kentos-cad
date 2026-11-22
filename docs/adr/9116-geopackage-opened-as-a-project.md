# ADR 9116: GeoPackage proje olarak açılır

- **Durum:** kabul edildi (2026-11-22); adım VY-19. Plan: [plan/veri.md](../../plan/veri.md) VY-19.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis içerisinde geopackage 1. sınıf destek sunulmasını istedim ama
  geopackage açarken içinde proje olup olmadığını arıyor, içinde kentos projesi yoksa bile açsın ama projeyi de
  oluştursun içine, geopackage qgis kadar iyi kullanmalı.”
- **Web:** ertelendi (ADR 0228). **`.kcad`:** biçim değişmedi. **Komut:** yok. Açmak, ADR 9107 §4'teki Aç gibi bir
  kabuk işidir; dosyaya yazılan proje Kaydet'in yazdığıdır.
- **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Önce:** ADR 9107'ye göre proje bir GeoPackage'ın `kentos_projects` tablosunda saklanabiliyordu. Ama dosya yalnız
Dosya › Veri kaynağından aç penceresinden açılıyordu. Orada da yalnız listedeki bir proje açılabiliyordu. Dosya ›
Aç `.gpkg`'yi tanımıyordu (“Diğer veri türleri için İçe aktar kullanın”). Headless, Python ve MCP de aynı retle
karşılıyordu. Projesi olmayan bir GeoPackage proje olarak açılmıyordu.

**Masaüstü CBS** (QGIS 3.44 kullanım kılavuzu, 22 Kasım 2026'da okundu):

- *Opening Data*, §11.1.3: “When adding vector datasets containing multiple layers, the Select Items to Add dialog
  will appear.” Seçenekleri: katmanları bir gruba ekle, sistem ve iç tabloları göster, boş katmanları göster. Dosya
  tarayıcıdan, dosya yöneticisinden ya da haritaya sürüklenerek de eklenir.
- *General Tools*, §8.5.2.2–8.5.2.3: GeoPackage'ta stiller `layer_styles` tablosunda durur. Bir tablonun birden çok
  stili olabilir ama yalnız bir varsayılanı olabilir. Kılavuza göre “When loading a layer in QGIS, if a default style
  already exists for this layer, QGIS loads the layer with this style.”
- *Working with Projections*, §6: yeni projenin sistemi ya ayarlardaki varsayılan sistemdir ya da “the CRS of the
  first layer loaded into a new project”.
- *Joins and Relations*, §12.4: Project Properties › Relations'taki Discover Relations, GeoPackage'ın ilişkilerini
  (related tables uzantısı) bulur. Bu özellik 3.28'de geldi.
- ADR 9107 §1: proje GeoPackage'ın `qgis_projects` tablosunda saklanır, tarayıcıda katmanların yanında görünür.

**Ölçüt:** QGIS'te GeoPackage açmak için önce bir projede olmak, sonra katmanları eklemek gerekir. KentOS'ta dosyanın
kendisi açılır ve proje dosyanın içinde kurulur. Sahibin istediği budur. Katmanların, stillerin ve sistemin nasıl
alınacağı QGIS'in davranışından alındı.

## §2 Karar: açılışın üç sonucu

`kentos_native_application::gpkg_open::open(path, project)`:

- **Dosyanın tek projesi** (ya da `project` ile adı verilen) açılır. Ondan sonrası ADR 9107 §4'teki Aç gibidir:
  katmanlar dosyadan bulunur, Kaydet oraya yazar.
- **Birden çok proje** varsa ve hiçbirinin adı verilmediyse adları söylenir, proje açılmaz. Masaüstünde Veri
  kaynağından aç penceresi bu dosyayla açılır ve liste okunmuş gelir. Headless'ta ret `several_projects`'tir.
- **Proje yoksa** dosyanın tablolarından yeni bir proje kurulur, dosyanın adıyla ya da `project` adıyla dosyanın
  içine yazılır (ADR 9107 §2'deki gibi geri okunup karşılaştırılır). Çizim o proje olur, Kaydet oraya yazar.
  **22 Kasım 2026'da değişti ([ADR 9122](9122-geopackage-looked-at-unchanged.md)):** açılış artık hiçbir şey yazmaz;
  proje bellekte kurulur ve ilk Kaydet, dosyayı ve adı gösterdikten sonra onu dosyanın içinde saklar.
  - **Dosya yazılamazsa** (salt okunur, kilitli) proje yine açılır. Kaydedilmemiş sayılır, neden yazılamadığı söylenir
    ve Kaydet bir yer sorar. (ADR 9122'den beri açılış yazmadığı için salt okunur dosya öbürleri gibi açılır; yazılamadığı
    ilk Kaydet'te söylenir.)
- **GeoPackage değilse** (SQLite imzası yoksa) reddedilir: “… bir GeoPackage değil: dosyanın başında SQLite veritabanı
  imzası yok.”
- Açılış satır okumaz ve satır yazmaz. Tablonun kataloğu okunur; satırlar ADR 9101'deki gibi görünüm gerektikçe okunur.

**Yeni projenin içeriği** (hepsi `scripts/fixtures/gpkg_open_cases.py`'de bağımsız yazıldı):

- **Tür:** CBS projesi (`Workspace::Gis`), ölçek 1:1000, yazı tipi Barlow. Yeni proje sihirbazının CBS
  varsayılanlarıdır.
- **Sistem:** geometrili tabloların çoğunun sistemi alınır, yalnız bir projenin alabileceği sistemler arasından
  (`fixtures/crs/v1/registry.json`). Eşitlikte dosyada önce gelen tablonunki alınır.
  - Böyle bir sistem yoksa proje sistemsiz (Yerel, 0) açılır ve söylenir. Sistem tahmin edilmez (CLAUDE.md §5).
  - Notlar: tablolar birden çok sistemdeyse `mixed_systems` (öbürleri projenin sisteminde çizilir, ADR 9101 §6);
    sistem projeye alınamıyorsa `system_not_listed`; hiçbir tabloda sistem yoksa `no_system`; dosya boşsa `no_tables`.
  - QGIS ilk katmanın sistemini alır. KentOS'ta çoğunluk alınır, çünkü dosyanın ilk tablosu küçük bir yardımcı tablo
    olabilir.
- **Katmanlar:** her tablo bir veri katmanıdır (ADR 9101): adı tablonun adı, alanları sütunları
  (`data_describe::describe`, Katman olarak ekle'nin aynısı). Sıra şöyledir:
  - noktalar, çizgiler, alanlar, öbür geometriler, en altta geometrisiz tablolar;
  - her grubun içinde dosyanın sırası.

  Üstte yeni projenin Taslak çizim katmanı durur ve etkin katman odur, yeni projelerde olduğu gibi. Standart kadastro
  katmanları kurulmaz; projenin verisi dosyanın tablolarıdır.
- **İç tablolar katman olmaz:** başka programın stil tablosu (`layer_styles`) ve KentOS'un tabloları (`kentos_…`),
  QGIS'in “sistem ve iç tabloları göster” seçeneği kapalıyken olduğu gibi. Boş tablo katmandır: düzenlenip doldurulabilir.
- **Görünüş:** tablonun varsayılan stili (`layer_styles`, `useAsDefault` olan, yoksa en son yazılan, ADR 0627 §7)
  katmanın çizimidir. Stilin okunamayan kısmı söylenir. Tablonun KentOS veri kümesi bilgisi (ADR 0633 §5) katmana
  geçer.
- **İlk görünüm:** projenin sistemindeki tabloların kutularını birlikte çerçeveler. Başka sistemdeki tablonun kutusu
  projenin sistemine çevrilir.
- **İlişkiler** bu adımda alınmaz, varsa sayısı söylenir (§5).
- Proje komutlarla kurulur (`cad.layers.data`, stil). Sonra anlık görüntüsünden yeniden okunur, böylece geri alma
  geçmişi boş açılır.

## §3 Karar: girişler

- **Masaüstü:**
  - Dosya › Aç'ın süzgeçlerinde GeoPackage var (veri katmanı olan programlarda). `.gpkg` açılınca ADR 9107 §4'teki
    gibi arka planda okunur.
  - Proje kurulduysa: ““bina.gpkg” içinde KentOS projesi yoktu: tablolarından “bina” projesi kuruldu ve dosyanın
    içine yazıldı (6 veri katmanı). Kaydet bu dosyaya yazar.” Notlar bunun ardından söylenir.
  - Dosya son açılanlara “GeoPackage — proje dosyanın içinde” diye girer. Oradan açılınca artık içindeki proje açılır.
  - Çizim açık değilken sürüklenen GeoPackage aynı yoldan açılır. Çizim açıkken Kaynaklar'a eklenir, önceki gibi.
  - Veri kaynağından aç'ta projesi olmayan dosya için “Bu dosyada proje yok. Aç, tablolarından yeni bir proje kurar ve
    onu bu dosyaya yazar.” yazar. Proje adı dosyanın adıyla gelir.
  - Mimari ve Yol gibi veri katmanı olmayan programlar GeoPackage'ı proje olarak açmaz, bunu söyler.
- **Headless, Python, MCP:**
  - `Session::open` `.gpkg`'yi `Session::open_geopackage`'a verir. Python'da `Document.open`, MCP'de `drawing.open`
    da buradan geçer; MCP'de `project` ile adı verilir.
  - Özetteki `openedGeoPackage` neyin açıldığını (`made`), yazılıp yazılmadığını ve notları söyler.
  - Bir kaynaktan açılan oturum (bu adım ya da `open_from_source`) nereden açıldığını tutar. Yolsuz `save()` oraya
    yazar. Dosyaya kaydedince oturum o dosyanın olur.
- **Modül:** `gpkg_open` ve `data_describe` CBS modülündedir (`mod-gis`). Bu modül olmadan derlenen headless
  GeoPackage'ı açmaz, nedenini söyler.

## §4 Kanıt

- **Bağımsız durumlar:** `scripts/fixtures/gpkg_open_cases.py` 10 GeoPackage'ı sqlite3 ile yazar. Beklenenleri bu
  ADR'nin kurallarından KentOS kodu kullanmadan çıkarır (`fixtures/data/v1/gpkg-open/expected.json`). Durumlar:
  - her türden tablo, boş tablo, geometrisiz tablo ve stil tablosu (varsayılan stil daha eski yazılmış ve kazanır);
  - sistemlerin çoğunluğu, eşitliği, projeye alınamayan sistem, tanımsız sistem (−1), sistemsiz dosya, boş dosya;
  - bir projeli, iki projeli ve GeoPackage olmayan dosya.
- **Native:** `tests/all/gpkg_open.rs` her durumu bir kopyada açar: sistem, notlar, katman sırası, tablo, anahtar,
  geometri, alanlar, görünüş, ilk görünüm. Proje dosyaya yazılmıştır ve yeniden açılınca o gelir. Adı verilen proje
  açılır ya da projesiz dosyada o adla kurulur; olmayan adda proje listesi söylenir. Salt okunur dosya kaydedilmemiş
  açılır ve dosyada proje yoktur.
- **Masaüstü, Python, MCP:** `gpkg_open_tests`, `test_gpkg_open`, MCP `gpkg_open` sınamaları.
- **Gerçek pencere** (`.run/gui-evidence/gpkg-ac-9116`):
  - `katmanlar.gpkg`'nin bir kopyası program açılırken verildi. Harita tabloları çizdi. Binalar dosyanın varsayılan
    stiliyle geldi: Ada A kırmızı, Ada B mavi, C ve D yeşil. Katmanlar'da altı katman vardı, Komut geçmişinde “kuruldu”
    iletisi. sqlite3 dosyada `kentos_projects` satırını (3717 bayt) ve uzantı kaydını gösterdi.
  - Program kapatılıp aynı dosya yeniden açıldı: yeni proje kurulmadan dosyadaki proje açıldı, dosyada yine tek proje
    vardı.
  - Dosya › Aç'ın sistem penceresinde süzgeç GeoPackage dosyalarını seçilebilir gösterdi. Karma sistemli dosya seçildi:
    proje EPSG:5256 oldu, karışık sistem notu söylendi, WGS 84 tablosu projenin sisteminde çizildi.
  - Başlangıç ekranının Son dosyalar listesinde dosya “GeoPackage — proje dosyanın içinde” diye göründü.

## §5 Kalan (QGIS'le aradaki fark)

Açılış dışında GeoPackage'ın tam kullanımı için sırada olanlar (sahip seçecek):

- ~~**Raster döşeme tabloları**~~ — yapıldı ([ADR 9117](9117-geopackage-rasters-and-looks.md) §2–§3): açılışta bağlı
  raster olarak gelir, yolu GeoPackage'a göre yazılır.
- **İlişkiler:** related tables uzantısındaki ilişkiler veri katmanları arasında projenin ilişkileri olmalı. Bugün
  yalnız İçe aktar (`file.import.gpkg`) çizim katmanlarına alıyor.
- ~~**Stilin geri yazılması**~~ — yapıldı ([ADR 9117](9117-geopackage-rasters-and-looks.md) §4): Kaydet, değişen
  görünüşü tablonun varsayılan stili olarak `layer_styles`'a yazar.
- **Üst veri:** `gpkg_metadata` katmanın bilgisine okunup yazılmalı.
- **Birden çok geometri sütunu**, eğri ve 3B türler, görünümlerin (view) katman olması.
- **Açılırken tablo seçimi:** QGIS'in Select Items to Add'i gibi açılışta hangi tabloların katman olacağı seçilmeli;
  bugün hepsi gelir.
- **Bakım:** dosyanın küçültülmesi (VACUUM), mekânsal dizinin yeniden kurulması, bütünlük denetimi.
