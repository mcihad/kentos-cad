# ADR 0180: Kayıtlı ölçüler

- **Durum:** kabul edildi (2026-10-05); tamam (2026-10-06, iki platformda, tek parçada). Sıra sahibin kararıdır: TODOS.md §16.0'ın yirmi üçüncü işi `HYB-23`; sahibin 5 Ekim kararıyla
  madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. ArcGIS'in COGO öznitelikleri (Direction, Distance, Radius,
  ArcLength) ve Update COGO'su örnektir; Netcad'de karşılığı yoktur.
- **Bağlam belgesi:** TODOS.md `HYB-23`, `GIS-06`; CLAUDE.md §7 (geometrik alan, kayıtlı alan ve gösterim ayrıdır), §23.1 (kesin
  ondalık değerler sayıya çevrilmeden metin olarak taşınır); ADR 0165 §4 (CBS'de semt grad, kutupsal giriş), ADR 0166 (sayısallaştırma
  kilitleri), ADR 0172 (köşe tablosu: kenarın semti ve uzunluğu), ADR 0177 §6 (raporun CSV biçimi).

## Bağlam

Kadastroda bir sınırın kayıttaki (tapu, plan, ölçü krokisi) semti ve uzunluğu çizimden ölçülenden ayrı bir bilgidir: çizim ölçekten,
sayısallaştırmadan ya da koordinat dönüşümünden küçük farklar taşır; kayıttaki değer hukuki ve kesin yazılmış olandır. Bugün KentOS
kenarın uzunluğunu ve semtini yalnız geometriden hesaplar; yazılarak çizilen bir çizginin yazılan değerleri çizgiye dönüşünce kaybolur.

## Karar

### 1. Kayıtlı ölçü öznitelikleri

- Kayıtlı ölçüler çizgi ve yay nesnelerinin dört özniteliğidir (ArcGIS'in COGO alanları gibi; geometriden ayrı, metin olarak):
  **Kayıtlı semt** (grad, kuzeyden saat yönünde, 0 ≤ semt < 400), **Kayıtlı uzunluk** (m), **Kayıtlı yarıçap** (m, yaylarda) ve
  **Kayıtlı yay uzunluğu** (m, yaylarda). Değer yazıldığı gibi, noktalı ondalık metin olarak saklanır; sayıya çevrilip geri yazılmaz.
- Bir çizginin semti başından sonuna, uzunluğu iki ucu arasındaki düzlem uzunluğudur; yayın semti ve uzunluğu kirişinindir (başlangıçtan
  bitişe), yarıçapı ve yay uzunluğu yayın kendisininkidir. Çoklu çizgi ve alanların kenarları kayıtlı ölçü taşımaz (ArcGIS'te de
  COGO iki noktalı çizgi ve yaydadır): kenarları ölçülü bir parsel çizgilerle çizilir ve Toplu alan (ADR 0151) ile alana çevrilir.
- Öznitelik oldukları için Öznitelikler panelinde görünür ve elle yazılır, ifadelerde kullanılır, DXF, GeoJSON ve Shapefile ile gider ve
  gelir; geometri düzenlemeleri onları değiştirmez (ArcGIS'in COGO alanları da öyledir; Update COGO açıkça çalıştırılır).

### 2. Çizerken kaydetmek

- **Çizgi** aracında sonraki nokta kutupsal yazılınca (`@25.40<123.4567` ya da `25.40<123.4567`, ADR 0165 §4) yazılan değerler çizilen
  çizginin kayıtlı ölçüleri olur: uzunluk her projede (yerel projenin milimetre ya da santimetresi metreye ondalık noktası kaydırılarak,
  kesin), semt yalnız CBS projesinde ve açı birimi grad iken (kutupsal açı orada semttir; CAD'in açısı doğudan saat tersine derecedir ve
  semt olarak kaydedilmez). Tıklanan, kilitlerle bulunan ya da koordinatla yazılan noktaya çizilen çizginin kayıtlı ölçüsü olmaz.
- Kayıt kuralı ortak çekirdektedir (yazılanın okunması ve metnin metreye çevrilmesi); iki platform aynı durumları geçer.

### 3. Denetlemek

- **Kayıtlı ölçüleri denetle** (`cogo.check`): pencere; kayıtlı ölçüsü olan çizgi ve yaylar (bütün çizim ya da seçim; seçim varken
  Kapsam Seçim'le açılır), her kayıtlı ölçü bir satır: Durum (Uyuyor, Farklı, Okunamadı), Katman, Tür, Ölçü (Semt, Uzunluk, Yarıçap,
  Yay uzunluğu), Kayıtlı (yazıldığı gibi), Çizimden (semt 4, uzunluklar 3 ondalıkla) ve Fark (semtte cc, uzunluklarda mm; kayıtlı
  eksi çizimden). Pencere açılınca denetler; özet nesneleri sayar (“6 nesne denetlendi: 4 uyuyor, 1 farklı, 1 okunamadı.”): bir
  ölçüsü okunamayan nesne Okunamadı, okunanlardan biri toleransı aşan Farklı'dır.
  Toleranslar pencerede yazılır: uzunluk (varsayılan 0,01 m, yarıçap ve yay uzunluğu da) ve semt (varsayılan 50 cc). Bir ölçünün
  farkı toleransı aşarsa satır Farklı'dır; kayıtlı değer sayı değilse (ondalık virgül noktadır) Okunamadı. Yalnız farklar
  (varsayılan açık) uyanları gizler; satıra tıklamak nesneyi seçer ve gösterir; Panoya kopyala ve CSV olarak kaydet… (ADR 0177 §6'nın biçimi).
- Semt farkı −200 ile 200 grad arasına indirilir; uzunluklar düzlemdedir (zemin ve elipsoit düzeltmesi kapsam dışı, ADR 0171).

### 4. Çizimden yazmak

- **Kayıtlı ölçüleri çizimden yaz** (`cogo.update`; Update COGO): seçili çizgi ve yayların kayıtlı ölçüleri çizimden ölçülenle yazılır
  (semt 4, uzunluklar 3 ondalıkla); tek geri alma adımı “Kayıtlı ölçüleri yaz”. Kilitli katmandakiler atlanır ve sayılır. Denetle
  penceresinde de düğmesi vardır (Seçilenlere çizimden yaz: satıra tıklamak nesnesini seçer); yazınca denetim yinelenir.

### 5. Yer

- CBS'de Ölçme sekmesinde (Kayıtlı ölçüler), CAD'de Yönet sekmesinde; Ölçme menüsünde. Takma adlar COGO, KAYITLIOLCU,
  KAYITLIOLCUDENETLE (denetle) ve UPDATECOGO, KAYITLIOLCUYAZ (çizimden yaz).

## Kapsam dışı

- Çoklu çizgi ve alan kenarlarının kayıtlı ölçüsü (kenarlar çizgilerle tutulur); kayıtlı alanın (tapu alanı) parsel aracındaki yeri
  ayrıdır (ADR 0067).
- Zemin, elipsoit ve yükseklik düzeltmesi; kotlar.
- Kayıtlı ölçülerden geometri kurmak (ArcGIS'in Traverse'ü kapanma dengelemesiyle): Poligon hesabı penceresi bunu yapar (ADR 0070).

## Uygulama

Tek parçada: çekirdekte ölçme, denetleme ve kayıt kuralı (`ops::cogo`: `measure`, `check`, `record`; işlemler `cogoMeasure`,
`cogoCheck`, `cogoRecord`) ve bağımsız başvurusu (`scripts/fixtures/cogo_cases.py`, `fixtures/cogo/v1/cases.json`); iki platformda
Çizgi aracının kaydı (web `tools/drawTools.ts`'in `typedText`'i, masaüstü `kentos_interaction::line`'ın `typed`'ı; şablon damgasıyla
aynı öznitelik yolundan), denetleme penceresi ve çizimden yazma (web `app/cogo.ts`, `ui/cogo/CogoCheckDialog.ts`; masaüstü
`apps/desktop/src/cogo.rs`); komutlar, ikonlar, şerit ve menü; ortak iz (`fixtures/interaction/v1/cogo.json`, `cogo.kcad`); envanter.

Tamam (6 Ekim): iki platform ortak durumları ve ortak izi geçer; resimler `.run/shots/kullanim/*-cogo-*`.

## Doğrulama

- Çekirdeğin kuralları ortak durumlarla, bağımsız başvurudan (mpmath); iki platform aynı dosyayı geçer.
- Çizgi aracının kaydı, pencere ve çizimden yazma ortak izle; resimler iki temada, 1440 × 900 ve 1100 × 650.
