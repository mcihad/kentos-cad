# ADR 0174: Çok parçalı çizgi ve çok noktalı nesne

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın on yedinci işi `HYB-17`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır.
- **Bağlam belgesi:** TODOS.md `HYB-17`, [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0143 (çok parçalı alan:
  aynı biçim, `AreaPart`, şema 5), ADR 0142 (köşe kotu), ADR 0025 (KCAD v2), ADR 0046 (GeoJSON, Shapefile), ADR 0009 (DXF), ADR 0006 (CAD
  ve PostGIS kaynağı), ADR 0152 (ölçü noktası: ad, kod, kot); Netcad Çoklu Doğru Birleştir ve Çoklu Doğru Parçası Çıkar, ArcGIS
  multipart ve multipoint düzenleme, QGIS Add Part, Delete Part, Promote to multipart.

## Bağlam

ADR 0143 alanı çok parçalı yaptı; çoklu çizgi ve nokta tek parçalı kaldı. Bu yüzden bugün:

- **GeoJSON:** MultiLineString ve MultiPoint parçaları ayrı nesnelere bölünür, her parça özelliğin özniteliklerini kopya olarak taşır
  (“parçaları ayrı nesneler olarak alındı”). Tek kayıt birden çok nesne olur; geri yazılınca MultiLineString'e ya da MultiPoint'e dönmez.
- **Shapefile:** çok parçalı PolyLine kaydı ve MultiPoint kaydı da ayrı nesnelere bölünür; iki noktalı parça çizgi olur.
- **Sunucu:** PostGIS'in MultiLineString'i ve MultiPoint'i bir nesneyle karşılanmaz.
- **Netcad'in** Çoklu Doğru Birleştir'i birbirine değmeyen doğruları tek nesne yapar (bir yolun kesik şeritleri, bir hattın kopuk
  bölümleri); KentOS'ta Parçaları birleştir yalnız alanları alır.

## Karar

### 1. Veri

- **Çoklu çizgi** (`polyline`) `parts` alır: ilk parçanın ötesindeki parçaların listesi. Parça alanın parçasıyla aynı biçimdir
  (`AreaPart`: `pts`, `bulges`, `zs`), deliği yoktur. İlk parça çoklu çizginin kendi `pts`, `bulges` ve `zs`'idir. Tek parçalı çoklu
  çizgide `parts` yoktur; bugünkü her çoklu çizgi böyle kalır. Parça en az iki noktalıdır ve açıktır.
- **Nokta** (`point`) `parts` alır: ilk noktanın ötesindeki noktalar (`PointPart`: `p`, `z`). İlk nokta noktanın kendi `p` ve `z`'sidir.
  Tek noktada `parts` yoktur. Çok noktalı nesnenin adı, kodu ve öbür öznitelikleri bütün noktalarındır (ADR 0152); etiketi ilk noktanın
  yanındadır.
- **Çizgi** (`line`) tek parçalıdır: çok parçalı çizgi çoklu çizgi olarak tutulur (Parçaları birleştir çizgiyi çoklu çizgi parçası
  yapar).
- **Parçaların sırası** anlamlıdır ve korunur. Parçalar birbirine değebilir, kesişebilir; okuyucu denetlemez (alanın parçaları gibi).
- **Adlar:** sözleşmede (`contracts::entity`) çoklu çizginin `parts`'ı (`AreaPart`, `holes`'suz) ve noktanın `parts`'ı (`PointPart`);
  TypeScript'te aynı adlar. Çekirdekte `Shape::Polyline`'ın ve `Shape::Point`'in `parts`'ı; parça parça gezmek alanınkiyle aynı
  `area_parts` ile (üç tür için; `is_multi_part`, `join_parts`, `replace_part` de öyle).

### 2. `.kcad`: belge şeması 17

- `polyline`'ın `parts`'ı parça haritalarının dizisidir: `pts` (zorunlu, en az 2 nokta), `bulges`, `zs` (isteğe bağlı); `holes`
  yasaktır. `point`'in `parts`'ı nokta haritalarının dizisidir: `p` (zorunlu), `z` (isteğe bağlı).
- Yazıcı `17`'yi **yalnız bir çoklu çizginin ya da noktanın `parts`'ı varken** yazar; başka her çizim eskisi gibi ve bayt bayt aynıdır.
  Şema 17 şema 16'yı kapsar.
- Şema 2–16 yükünde çoklu çizginin ya da noktanın `parts`'ı bilinmeyen alandır (`unknown_field`): eski okuyucu çizimi açmayı reddeder,
  parçaları sessizce düşürmez.
- Tipli sütunlar parçaları taşır; `FORMATS_VERSION` artar. Bağımsız Python okuyucusu ve yazıcısı, örnek dosyalar ve bozuk örnekler
  kodekle birlikte değişir.

### 3. Hesap

- **Ölçüler:** uzunluk parçaların uzunluklarının toplamıdır; 3B uzunluk bütün köşelerin kotu varken. Köşe sayısı bütün parçalarınkidir.
  Çok noktalının ağırlık merkezi noktalarının ortalamasıdır.
- **Seçme:** bir parçası imlecin yakınındaysa nesne seçilir; pencere bütün parçaları içine alınca seçer, kesişim penceresi bir parçaya
  değince.
- **Kenet:** bütün parçaların uçları, köşeleri, ortaları ve kenarları; çok noktalının her noktası.
- **Tutamaçlar:** ilk parça bugünkü sırasıyla, ardından her ek parça aynı sırayla; çok noktalının her noktası bir tutamaç.
- **Dönüşümler** (taşı, kopyala, döndür, ölçekle, aynala, hizala, oturt) bütün parçalara uygulanır.
- **Düzenlemeler** ADR 0143'teki gibi: tutamaç, köşe ekle ve sil, esnet, köşe yuvarla ve pah dokunulan parçada çalışır, öbür parçalar
  olduğu gibi kalır. Ötele her parçayı öteler. Patlat her parçanın kenarlarını verir; çok noktalı nesne noktalarına ayrılır. Bir yol
  boyunca çalışan düzenlemeler (Buda, Uzat, Kır, Parçala, Sürdür, Biçim değiştir, Birleştir) hangi parçada çalışacağını bilemez ve
  öbürlerini kaybetmemelidir: alanınki gibi söyler, “Bu işlem çok parçalı nesnede çalışmaz; önce Parçalara ayır ile parçalarına
  ayırın.”.
- **Kotlar** (ADR 0142) parça parça taşınır.
- **Çizim:** her parça çizilir; çok noktalının her noktası noktanın sembolüyle. Seçim ve üzerine gelme vurgusu bütün parçalardır.

### 4. Komutlar ve araçlar

- **`cad.entities.edit`'in `partsJoin`'i** (Parçaları birleştir) aynı türden nesneleri alır: alanlar (ADR 0143), çizgiler ve çoklu
  çizgiler (tek, çok parçalı çoklu çizgi), noktalar (tek, çok noktalı nesne). Türler karışıksa: “Parçaları birleştir aynı türden
  nesneleri birleştirir: alanları, çizgileri ya da noktaları.”. Sonuç ilk seçilen nesnenin yuvasını, kimliğini, katmanını ve özniteliklerini
  alır; parçalar seçim sırasıyla, her nesnenin kendi parçalarıyla; kotlar taşınır. Birbirine değen çizgiler birleşmez, parça kalır
  (birleştirmek Birleştir'in işidir).
- **`partsSplit`** (Parçalara ayır) çok parçalı çoklu çizgiyi ve çok noktalı nesneyi de ayırır: ilk parça nesnenin yuvasını ve kimliğini
  korur, öbürleri öznitelikleriyle yeni nesnedir; iki noktalı parça çoklu çizgi kalır.
- Şeritte yerleri değişmez (Değiştir › Birleştir ve böl); ipuçları ve adımlar türleri sayar. Kilitli katman ve tek geri alma adımı
  bugünkü gibidir; ortak durumlar `fixtures/commands/v1`'de, bağımsız Python denetimiyle.

### 5. Değişim biçimleri ve sunucu

- **GeoJSON:** MultiLineString çok parçalı çoklu çizgi, MultiPoint çok noktalı nesnedir, okumada ve yazmada. “Ayrı nesneler” notu bu iki
  tür için kalkar; GeometryCollection eskisi gibi üyelerine bölünür. Tek üyeli MultiLineString ve MultiPoint tek parçalı nesne olur,
  yazılırken LineString ve Point'tir.
- **Shapefile:** çok parçalı PolyLine kaydı çok parçalı çoklu çizgi, MultiPoint kaydı çok noktalı nesnedir; tek parçalı, iki noktalı
  PolyLine kaydı bugünkü gibi çizgidir.
- **DXF:** yazarken her parça ayrı LWPOLYLINE, her nokta ayrı POINT'tir (DXF'in çok parçalı çizgisi ve noktası yoktur); okuma değişmez.
- **Sunucu (PostGIS):** çok parçalı çoklu çizginin izdüşümü MultiLineString, çok noktalınınki MultiPoint'tir. Yay varsa kaynak
  `cad_definition`'dır (ADR 0006).

### 6. Arayüz

- **Öznitelikler:** parça sayısı 1'den çoksa “Parça sayısı” satırı (çok noktalıda “Nokta sayısı”). Uzunluk ve Köşe sayısı bütün
  parçaların toplamıdır.
- **Üzerine gelme kartı:** “Parça: n” (çok noktalıda “Nokta: n”).
- **Köşe tablosu** (ADR 0172) parçaları halkalar gibi sıralar (“Parça 1”, “Parça 2”).
- **Nokta editörü** (ADR 0153) tek noktaları listeler: çok noktalı nesnenin noktaları bir adı paylaşır, satır olmaz (ad, sıra ve çift
  nokta kuralları noktanın kendi adına dayanır); Parçalara ayır ile ayrılınca listede görünür.

### 7. Kapsam dışı

- Çok parçalı yazı, daire, yay, elips ve eğri; GeometryCollection'ın tek nesne olması.
- Parçaların otomatik birleştirilmesi (değen uçların zincirlenmesi Birleştir'in Zincir'idir, ADR 0161).

## Adımlar

1. Sözleşme, `.kcad` şema 17 ve bellekteki belge: spesifikasyon, kodek, sütunlar, Python okuyucusu ve yazıcısı, örnekler. Web modeli ve
   masaüstünün `kentos-domain`'i parçaları taşır; kaydet ve aç onları bayt bayt korur. Bitti (5 Ekim): `PathEntity.parts` çoklu çizgide
   de (deliksiz, en az iki köşe), `PointEntity.parts` (`PointPart`); şema 17 (`SCHEMA_WITH_LINE_PARTS`), tipli sütunlarda noktanın
   parçaları (`FORMATS_VERSION` 28); bağımsız Python okuyucusu ve yazıcısı, örnek `multi-part-lines.kcad` ve beş bozuk örnek (iki eski
   şema, tek köşeli parça, delikli parça, yersiz nokta); iki belgede tür değişince yalnız miras kalan parçalar düşer (ortak
   `document-ops` durumu); web sayfası örneği bayt bayt yazar.
2. Hesap: çekirdeğin `Shape`'i, ölçüler, depo (çizim, seçme, kenet, etiket), tutamaçlar, dönüşümler, düzenlemeler; iki çizici.
   Bitti (5 Ekim): çekirdekte `Shape::Polyline` ve `Shape::Point` `parts` alır, `area_parts` üç türü parçalar; uzunluk ve köşeler
   bütün parçaların, kutu bütün parçaları kapsar, çoklu çizginin etiketi en uzun parçasında, çok noktalının ağırlık merkezi
   noktalarının ortalaması. Depoda çizim kaydı parça başına bir yol (noktaları hep verilir) ve çok noktalı için yeni `MARKERS` kaydı
   (7; stil motoru her noktayı noktanın sembolüyle çizer), tıklama en yakın noktaya, çit ve pencereler parça parça, kenet bütün
   parçaların uçları, ortaları ve kenarları ile her nokta, paketin türleri 16 ve 17. Tutamaçlar, taşı, döndür, ölçekle, aynala,
   oturt, Kauçuk levha, Esnet ve topolojik düzenleme parça parça; Ötele her parçayı aynı yana öteler (QGIS ve ArcGIS gibi, yönüne
   göre sol ya da sağ; yanı imlece en yakın kenar söyler); Patlat her parçanın kenarlarını, çok noktalıyı noktalarını verir. Buda,
   Uzat, Kır, Uzat-kısalt, Sürdür ve Biçim değiştir reddeder (`MULTI_PART_REFUSED`); Parçala ve Birleştir onu dışarıda bırakır,
   Kenar eşleme aday saymaz; Toplu alan ve İzle parçaları ayrı yollar olarak alır; Topolojik temizlik her parçanın uçlarını işler.
   `cad.entities.edit`'in geometrisinde noktanın ve çoklu çizginin `parts`'ı (parçası en az iki köşe, deliği `part_holes` ile
   reddedilir), kotları parça parça taşınır; Kot ver her noktaya ve parçaya. Masaüstünün wgpu çizicisi ve web'in sahnesi her parçayı
   ve noktayı çizer. Çekirdek testleri `tests/all/line_parts.rs`, web'in paket testleri (16, 17).
3. Değişim biçimleri ve sunucu: GeoJSON, Shapefile, DXF yazımı, PostGIS izdüşümü; bağımsız okuyucunun (`tools/formats/gis.py`)
   kuralları. Bitti (5 Ekim): GeoJSON'ın MultiLineString'i ve MultiPoint'i, Shapefile'ın PolyLine ve MultiPoint kaydı okunabilen
   üyelerinin tek nesnesidir (bir üye kalırsa çizgi, yol ya da nokta; boş olan söylenir), bağımsız okuyucu da öyle okur, fixture'ların
   beklenen çıktıları ve elle yazılmış özeti buna göre (`features`, `tm`, `nonfinite`, `kuyular`, `kotlu`, `yollar`, `yollarz`).
   GeoJSON yazıcısı çok parçalı çoklu çizgiyi MultiLineString'e (yaylı parça örneklenir, kotlar yay boyunca karışır), çok noktalıyı
   MultiPoint'e yazar (`export/cizgiler`); DXF yazıcısı her parçayı ayrı çoklu çizgi (kotlu ve yaysız parça 3B POLYLINE), her noktayı
   ayrı POINT olarak, nesnenin verisiyle yazar ve bir kez söyler (`dxf-write/parts`, `dxf_write_reference.py`). Sunucuda EWKB
   MultiPoint ve MultiLineString'i taşır; düz ve kotsuz çok parçalı çoklu çizginin kaynağı MultiLineString'i, kotsuz çok noktalının
   MultiPoint'idir, öbürlerinin kaynağı `cad_definition` (izdüşüm yine her parça); çoklu çizginin parçası en az iki köşelidir ve
   adası olamaz.
4. Komutlar: `partsJoin` ve `partsSplit`'in yeni türleri; ortak durumlar.
5. Arayüz: Öznitelikler, üzerine gelme kartı, köşe tablosu; iki platformda, resimleriyle.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Kodek: bağımsız Python yazıcısının örnekleri (`fixtures/kcad/v2`) iki yönde; eski şemalı yükte `parts` reddi.
- Çekirdek ve araçlar: ortak durumlar ve izler iki platformda; biçimler bağımsız okuyucuyla (`gis_reference.py`).
