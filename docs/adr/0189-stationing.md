# ADR 0189: Hat boyunca kilometre (Km yaz)

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-26`'nın ardından `CAD-27`; sahibin 5 Ekim
  kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler Netcad'in Obje Üzerinde Dizi'si (??KM
  etiketleri), Civil 3D'nin istasyon etiketleri, ArcGIS'in Generate Transects Along Lines'ı ve QGIS'in Transect ve Points along
  geometry'sidir.
- **Bağlam belgesi:** TODOS.md `CAD-27` (ilgili `CIVIL-09`, `GIS-28`); ADR 0188 (yolun noktası ve km'nin yazılışı), ADR 0185 (Koordinat
  yaz: kâğıtta yükseklik, stil, önizleme), ADR 0145 (okunur yazı), ADR 0149 (gösterim kuralı).

## Bağlam

Bir güzergâhın (yol ekseni, dere, hat) üzerine aralıkla kilometre yazmak, istasyonlara dik işaret ve enkesit hatları çizmek, eksenden
sapmalı nokta dizisi koymak elle yapılıyor. Nokta hesaplayıcı tek noktayı km ve sapmayla verir (ADR 0188 §2); Yol boyunca dizi nesne
kopyalar, km bilmez.

## Karar

### 1. Araç

**Km yaz** (Açıklama sekmesi, Koordinat yaz'ın yanında; CAD projesinin arayüzü). Güzergâha tıklanır; seçim tek bir güzergâhsa onu alır.
Güzergâh Obje üzerinde nokta'nın yoludur (`route_of`: çizgi, çoklu çizgi, yay, daire, elips, eğri, alanın dış sınırı). Bütün nesneler
çizimde soluk görünür; Enter ya da Uygula tek adımda (“Km yaz”) etkin katmana yazar, araç sonraki güzergâhı bekler; Esc bitirir.

### 2. İstasyonlar

- Km güzergâhın ilk köşesinde **Başlangıç**'tır (B; `0+000`), yönü çizildiği yöndür; **Ters** (T) sondan yürür.
- İstasyonlar **Aralık**'ın (A; 20 m) katı olan km'lerdedir (`0+020`, `0+040` …; başlangıç kat değilse ilk kata kadar boşluk kalır).
  **Uçlar** (U; açık) güzergâhın başına ve sonuna da yazar. Bir uç bir kata 10⁻⁹ m'den yakınsa tek istasyondur (katın yazısıyla);
  kapalı yolda sonu başıdır, orada tek istasyon (başlangıcınki) yazılır.
- Km yazısı km'nin metresidir (ADR 0188 §2): kat istasyonları aralığın ondalıklarıyla (20 m → `0+020`, 12,5 m → `0+012.5`), uçlar projenin
  uzunluk ondalıklarıyla, gösterim kuralıyla.
- En çok 20 000 istasyon; fazlası aralığı büyütmesi söylenerek reddedilir.

### 3. Her istasyonda

- **İşaret** (İ; açık): istasyonda yola dik, iki yana kâğıtta 2 mm'lik çizgi.
- **Yazı** (Y; sol): km yazısı yola dik, yolun solunda (sağında, ya da yok), işaretin ucundan yazı yüksekliğinin yarısı ötede başlar;
  dik açıyı geçen doğrultuda yarım tur dönüp okunur kalır (etiketlerin kuralı) ve tabanının sağından asılır. Yazı işaretin (ve
  enkesitin) çizgisinin üstünde değil yanındadır: tabanı çizgiye paralel, harflerin yanında, çizgiden yüksekliğin dörtte biri uzakta.
  **Yükseklik** (H; kâğıtta 2 mm), CAD projesinde **Stil** (S) Koordinat yaz'daki gibi.
- **Enkesit** (E; yok): yazılan yarı genişlikle iki yana dik çizgi; çizginin `Km` özniteliği istasyonun km yazısıdır.
- **Nokta** (N; yok): yazılan sapmayla (sağa artı) istasyonda nokta; katmanın sembolüyle çizilir, `Km` özniteliğiyle.

Seçenekler oturum boyunca hatırlanır (Ters her güzergâhta kapalı başlar). Eğride nokta ve dik doğrultu eğrinin kendisindendir (ADR 0188 §1).

### 4. Çekirdek

`ops::stationing`: istasyonlar (km, yerleri, yolun doğrultusu, yazıları) ve nesneleri (yazılar, işaretler, enkesitler, noktalar); yolun
noktası ve doğrultusu `tools::point_calc`'tan. Web'e işlem tablosundan. Ortak durumlar bağımsız Python başvurusundan
(`scripts/fixtures/stationing_cases.py`).

### 5. Komut

`cad.entities.create`'in `stations` işlemi, adım “Km yaz”; nesneler hesaplandığı gibi verilir.

## Kapsam dışı

M değerli hatlar ve ölçülü tarama (KentOS geometrisinde M yok; `GIS-28`), güzergâhın kendi km tablosu ve eşitlik noktaları
(`CIVIL-04`), enkesitlerden profil, blok dizisi (Yol boyunca dizi var).

## Uygulama

- Çekirdek `crates/shared/geometry-core/src/ops/stationing.rs`: `stations` (katlar ve uçlar, 10⁻⁹ m'de tek istasyon, kapalı yolda tek
  başlangıç, 20 000 sınırı), `objects` (işaret, okunur km yazısı çizginin yanında, enkesit, nokta), `stationing`; işlem tablosunda
  `stationing`. Yolun noktası ve doğrultusu `tools::point_calc::Walk`'tan (ADR 0188'in `station`'ı da onu kullanır).
- Sözleşme: `CreateOperation::Stations`, adım “Km yaz” (`crates/native/application/src/create.rs`, `product/entitiesCreate.ts`); katalog,
  TypeScript tipleri ve Python SDK'sının tipleri yeniden üretildi; komut durumu `create_command_cases.py`'de.
- Masaüstü `kentos_interaction::station_labels` (oturum belleği `Memory::station_*`; nesne başına öznitelik `points::write_objects_each`),
  resimleri `apps/desktop/src/stationing_scenes.rs`; web `tools/stationLabelTool.ts` (`stationOptions`), `writeObjectsEach`; şeritte
  Açıklama › Km (CAD'in arayüzü), katalogda `tool.stationLabels` (KMYAZ, KILOMETREYAZ, ISTASYON).
- İkon `stationLabels`: seçenek sayfasının B'si (A hesaplayıcının KM ikonuna fazla benziyordu; sahip uyurken).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/stationing_cases.py` (50 basamak; yolun kenarları ve km'nin yazılışı ADR 0188'in başvurusundan):
  `fixtures/stationing/v1/cases.json`'un 13 durumu: çizgi (her nesne), katlar arasında başlangıç, ters yön, sağdaki yazı, kuzeye giden
  hatta yarım tur dönen yazı, yaylı güzergâh, 12,5 m'lik aralığın ondalığı, kapalı kare ve daire, harita koordinatları, çok istasyon, sıfır
  aralık, yolu olmayan nesne. Çekirdek doğal (`tests/all/stationing.rs`) ve WASM'da (`stationing.wasm.test.ts`) geçer.
- Ortak iz `stationing.json` (`stationing.kcad`) iki platformda üç varyantta, web'de tek tarayıcıda art arda da geçer.
- Resimler: masaüstü `arac-km-yaz-*`, web `shots.mjs stationing`, iki temada, 1440×900 ve 1100×650.

