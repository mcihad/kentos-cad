# ADR 0501: Standart ilişkiler (DE-9IM) ve `cad.entities.relate`

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-22.
- **Bağlam belgesi:** ADR 0200 (mekânsal sorgu, Konuma göre seç), 0258 (kapalı çoklu çizgi çizgidir), 0500 (bindirme
  korpusu); OGC SFA 1.2.1 §6.1.15, PostGIS ST_Relate.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** Konuma göre seç'e standart profil seçeneği (arayüz, CBS-40), PostGIS sağlayıcısına itilen
  sorguyla aynı sonuç (CBS-04), coğrafi sistemde jeodezik uzaklık.

## Bağlam

Konuma göre seç'in altı ilişkisi 1 mm toleransla çalışır (ADR 0200). Kullanıcının yakınlık sorusu için doğrudur, ama
standart değildir:
- sınırdaki nokta İçeren'de sayılır;
- `covers`, `touches`, `crosses`, `overlaps`, `equals` yoktur.

PostGIS ile yerel sorgu farklı sonuç verirse bu ürün hatası olur. Eski komutların anlamı değişmeden, standart bir
profil gerekiyordu.

## Karar

### 1. Okuma

Nesneler türleriyle okunur (`de9im::geometry_of_standard`):
- **Alan:** çokgen, daire, tam elips, tarama.
- **Çizgi:** çizgi, çoklu çizgi (kapalısı da: kapalı çizgi alan değildir, ADR 0258), yay, elips yayı, eğri, NURBS
  (kapalısı da).
- **Öbürleri:** sorgunun okuduğu gibi (noktalar; yazının ve bloğun yeri).

Yardımcı çizgi ve ışın okunmaz (`unread`). Çizginin sınırı uçlarıdır, mod-2 kuralıyla; kapalı yolun sınırı yoktur.
Z okunmaz (ST_Relate gibi 2B).

### 2. Matris

`ops::de9im::relate` düğümlemeyle çalışır. Her nesnenin kenarları öbürünün kenarlarının kestiği yerde, köşelerinde ve
üstündeki noktalarında bölünür. Matrisin girdileri şöyle bulunur:
- **Boyut 1:** parçanın ortası öbür nesneye göre iç, sınır ya da dış olarak yerleştirilir.
- **Boyut 0:** her düğüm.
- **Boyut 2 (iki alan):** her sınır parçasının iki yanındaki noktalar. İki sınırın her yüzü bir parçanın yanındadır.

Yaylar kesindir. "Aynı nokta" payı, en büyük koordinatın son basamağının birkaç katıdır, en az 10⁻⁹ m (`tolerance`).
Kutuları ayrık iki nesnenin matrisi düğümlenmeden yazılır (`relate_fast`). Yüklemler matristen OGC SFA'nın
desenleriyle, PostGIS'in okuduğu gibi okunur (`Predicate::holds`): intersects, disjoint, contains, within, covers,
coveredBy, touches, crosses, overlaps, equals. `ST_Relate` deseni de okunur (T, F, *, 0, 1, 2; büyük küçük harf
fark etmez).

### 3. Komut

`cad.entities.relate` v1 salt okunurdur; Python'da `kentos.cad.entities.relate`, MCP'de araçtır.
- **Girdi:** `a` ve `b` kimlik listeleri; `predicate` ya da `pattern`. İkisi de yoksa bütün çiftler matrisleriyle döner.
- **Çıktı:** çiftler, `a`'nın sonra `b`'nin sırasıyla, her biri matrisiyle; okunmayanlar; revizyon.
- **Kapsam:** kimlikler verildiği için gizli ve kilitli nesneler de okunur.
- **Retler:** `no_objects`, `invalid_input` (ikisi birden), `invalid_pattern`, `entity_not_found` (yol `a[i]`,
  `b[i]`), `too_many_pairs` (1 000 000'dan çok çift).
- **Kod metinleri:** `invalid_pattern`'in genel metni katman adı örüntüsünü de kapsayacak biçimde genişledi;
  `too_many_pairs` yeni koddur.

## Denetimler

- `scripts/fixtures/relate_cases.py --check`: `cad.entities.relate.json`, 33 durum.
  - Düz kenarlı çiftlerin matrisleri GEOS'tan, QGIS 4.2.3'ün Python'u üzerinden.
  - Dairenin çiftleri tanımdan elle yazıldı, çünkü GEOS daireyi kirişlere böler.
- Birim testleri: sınırdaki nokta (`FF20F1FF2`: covers evet, contains hayır), ortak kenarlı kareler (`FF2F11212`).

## Sonuçlar

- Kesin topolojik ilişki ile toleranslı yakınlık ayrı seçeneklerdir; Konuma göre seç'in anlamı değişmedi.
- Standart profil PostGIS ve QGIS ile aynı matrisleri verir: sağlayıcıya itilen sorgunun yerel karşılığı hazırdır.
