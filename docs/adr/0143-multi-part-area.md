# ADR 0143: Çok parçalı alan

- **Durum:** kabul edildi (2026-09-29). Yön sahibin kararıdır; ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-09-29
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0142 (şema 4: köşe kotu), ADR 0065 ve 0069 (alan işlemleri ve komutları), ADR 0046 (GeoJSON, Shapefile), ADR 0009 (DXF), ADR 0006 (CAD ve PostGIS kaynağı); `docs/specs/kcad-v2.md` §6.1, §6.6; PiriCAD `netcad_plan.md` N-08.

## Bağlam

Netcad'de bir alan birden çok parçalı olabilir. Alan Birleştir kesişmeyen alanları tek, çok parçalı bir nesne yapar ve delikleri korur. Çoklu Doğru Birleştir ve Parçası Çıkar parçaları bir nesneye katar ya da ondan ayırır. Ayrıştır parçaları ayrı alanlara böler.

KentOS'ta bugün bir kapalı alan tek bir dış halka ve onun delikleridir. Bu yüzden:

- **Alan birleştir:** birbirine değmeyen alanlar ayrı nesne kalır.
- **Alan kesiştir ve çıkar:** birden çok parça çıkınca her parça ayrı nesnedir.
- **GeoJSON:** MultiPolygon'un parçaları ayrı nesnelere bölünür ve her parça özelliğin özniteliklerini kopya olarak taşır. Tek kayıt birden çok nesne olur; geri yazılınca MultiPolygon'a dönmez.
- **Shapefile:** birden çok dış halkalı Polygon kaydı da ayrı nesnelere bölünür.

Sahibin kararı (29 Eylül): `.kcad` şemasını değiştiren işler kendi özellikleriyle ve kendi şema adımlarıyla gelir. Sıra köşe kotu (ADR 0142), çok parçalı alan, blok, yazı ekleri, kılavuz ve yeni ölçü türleridir.

## Karar

### 1. Veri

- **Kapalı alan** (`polygon`) `parts` alanı alır. Bu alan, ilk parçanın ötesindeki parçaların listesidir.
- **Parça** (`AreaPart`): `pts`, `bulges`, `holes`, `zs`. Anlamları alanın kendi alanlarıyla aynıdır: dış halka, yaylar, delikler, köşe kotları (ADR 0142).
- **İlk parça** alanın kendi `pts`, `bulges`, `holes` ve `zs`'idir. Tek parçalı alanda `parts` yoktur; bugünkü her alan böyle kalır. Bu biçim, parçayı bilmesi gerekmeyen her okuyucuyu tek parçalı alanda değiştirmez; köşe ve tutamaç sıraları ilk parçada bugünkü gibidir.
- **Parçaların ilişkisi:** komutlar birbirini örtmeyen parçalar üretir. Bir parça başka bir parçanın deliğinde durabilir (adanın içinde ada). Okuyucu parçaların örtüşmesini denetlemez, deliklerin dış halkanın içinde olmasını denetlemediği gibi. İçe aktarmanın getirdiği örtüşen parçalar olduğu gibi kalır.
- **Parçaların sırası** anlamlıdır ve korunur. Komutlar parçaları büyükten küçüğe dizer.
- **Çoklu çizgi** bu adımda çok parçalı olmaz. Netcad'in çok parçalı doğrusu gerekirse sonra, kendi adımıyla gelir.
- **Adlar:** sözleşmede (`contracts::entity`) `parts` ve `AreaPart`; TypeScript'te aynı adlar.

### 2. `.kcad`: belge şeması 5

- `polygon`'un `parts`'ı, parça haritalarının dizisidir. Parça haritası: `pts` nokta listesi (zorunlu), `bulges` float dizisi, `holes` halka dizisi, `zs` kot listesi (isteğe bağlı).
- Parçanın noktası ve yayı alanın kurallarıyla denetlenir: en az 2 nokta (yaylı kenarla), `bulges` nokta sayısından uzun olamaz, kot listesi nokta sayısı kadardır.
- Yazıcı `5`'i **yalnız bir alanın `parts`'ı varken** yazar. Başka her çizim şema 2, 3 ya da 4'tür ve eskisiyle bayt bayt aynıdır. Şema 5 şema 4'ü kapsar.
- Şema 2–4 yükünde `parts` bilinmeyen alandır (`unknown_field`). Eski okuyucu çok parçalı çizimi açmayı reddeder, parçaları sessizce düşürmez.
- Tipli sütunlar parçaları taşır; `FORMATS_VERSION` artar. Bağımsız Python okuyucusu ve yazıcısı, örnek dosyalar ve bozuk örnekler kodekle birlikte değişir.

### 3. Hesap

- **Ölçüler:** alan, parçaların alanlarının toplamıdır; her parçada delikler düşülür. Çevre bütün halkaların toplamıdır. Ağırlık merkezi alanla ağırlıklıdır. 3B çevre (ADR 0142) yalnız bütün köşelerin kotu varken vardır.
- **Seçme:** nokta bir parçanın içindeyse (deliğinde değilse) alan seçilir. Pencere bütün parçaları içine alınca alan seçilir; kesişim penceresi bir parçaya değince seçer.
- **Kenet:** bütün parçaların köşeleri, kenar ortaları ve kenarları.
- **Tutamaçlar:** ilk parça bugünkü sırasıyla (köşeler, kenar ortaları, deliklerin köşeleri); ardından her ek parça aynı sırayla.
- **Dönüşümler** (taşı, kopyala, döndür, ölçekle, aynala, hizala) bütün parçalara uygulanır.
- **Düzenlemeler:** köşe ekle ve sil, tutamaç, esnet, köşe yuvarla ve pah dokunulan parçada çalışır, öbür parçalar olduğu gibi kalır. Ötele her parçayı öteler; örtüşen sonuçlar birleşir. Patlat bütün parçaların kenarlarını verir.
- **Kotlar** (ADR 0142) parça parça taşınır.
- **Çizim:** her parça delikleriyle doldurulur ve çizilir; seçim vurgusu bütün parçalardır. Etiket ve alan yazısı en büyük parçanın içine konur.

### 4. Komutlar ve araçlar

- **`cad.entities.edit`:**
  - `areaUnion`, `areaIntersect` ve `areaSubtract` `oneObject` seçeneği alır; varsayılanı `false`, bugünkü davranıştır. `true` ile birbirine değmeyen sonuçlar tek, çok parçalı alan olur (Netcad'in Alan Birleştir'i). Kadastroda tevhit bitişik parselleri birleştirir; bu yüzden varsayılan değişmez.
  - Yeni `partsJoin` (Parçaları birleştir): seçili alanlar tek, çok parçalı alan olur. Örtüşen alanlar birleşerek tek parça olur. Sonuç ilk seçilen alanın yuvasını, kimliğini, katmanını ve özniteliklerini alır.
  - Yeni `partsSplit` (Parçalara ayır): çok parçalı alan her parçası için bir alan olur. İlk parça alanın yuvasını ve kimliğini korur; öbürleri yeni alandır ve öznitelikleri taşır.
- **Şerit:** Değiştir › Alan paneline Parçaları birleştir ve Parçalara ayır; birleştir, kesiştir ve çıkarda “Tek nesne (T)” çipi.
- **Uygulama notu (29 Eylül):** `cad.entities.edit` yalnız değişiklikleri taşır; `oneObject` komutun alanı olmadı, üç aracın “Tek nesne (T)” seçeneğidir (masaüstünde `Memory.area_one_object`). Açıkken iki ya da daha çok parçalı sonuç tek `add` ile, büyükten küçüğe dizili tek alan olarak yazılır (çekirdeğin `one_area`'sı). Birleştir, kesiştir ve çıkar çok parçalı alanı bütün olarak alır (kesiştirmede `intersect_area_sets`). Parçaları birleştir, hiçbir parça öbürüyle örtüşmüyorsa her parçayı olduğu gibi, kotlarıyla dizer; örtüşme varsa parçaların birleşimini yazar. Parçalara ayır parçaları olduğu gibi (kotlarıyla) ayırır.
- Kilitli katman, tek geri alma adımı ve ortak durumlar (`fixtures/commands/v1`, bağımsız Python denetimiyle) öbür alan işlemlerindeki gibidir.

### 5. Değişim biçimleri ve sunucu

- **GeoJSON:** MultiPolygon çok parçalı alandır, okumada ve yazmada. Raporun “parçaları ayrı nesneler olarak alındı” notu MultiPolygon için kalkar.
- **Shapefile:** birden çok dış halkalı Polygon kaydı çok parçalı alandır. Delikler, halkanın yönüne ve içinde durduğu dış halkaya göre parçalarına gider.
- **DXF:** yazarken her halka ayrı kapalı LWPOLYLINE'dır, bugünkü deliklerin yazılışı gibi. DXF'in çok parçalı alan nesnesi yoktur; okuma değişmez.
- **Sunucu (PostGIS):** çok parçalı alanın izdüşümü MultiPolygon'dur. Yay yoksa geometri kaynaktır, varsa kaynak `cad_definition`'dır (ADR 0006).

### 6. Arayüz

- **Öznitelikler:** parça sayısı 1'den çoksa “Parça sayısı” satırı. Alan, Çevre ve Köşe sayısı bütün parçaların toplamıdır.
- **Üzerine gelme kartı:** parça sayısı 1'den çoksa “Parça: n”.
- **İpuçları ve iletiler** iki platformda aynı sözcüklerle.

### 7. İş sırası

1. **Sözleşme, `.kcad` şema 5 ve bellekteki belge:** spesifikasyon, kodek, sütunlar, Python okuyucusu ve yazıcısı, örnekler. Web modeli ve masaüstünün `kentos-domain`'i parçaları taşır; kaydet ve aç onları bayt bayt korur.
2. **Hesap:** ölçüler, depo (çizim, seçme, kenet, etiket), tutamaçlar, dönüşümler, düzenlemeler; iki çizici.
3. **Değişim biçimleri ve sunucu.**
4. **Komutlar:** `oneObject` seçeneği, `partsJoin`, `partsSplit`; ortak durumlar.
5. **Arayüz:** iki platformda, resimleriyle.

## Sonuçlar

- MultiPolygon ve çok dış halkalı Shapefile kaydı tek nesne olarak gelir ve öyle geri yazılır: bir kaydın kimliği ve öznitelikleri tek kalır.
- Parçasız çizim bugünkü dosyayla bayt bayt aynıdır; eski okuyucu onu açmayı sürdürür.
- Alan birleştirmenin kadastrodaki varsayılanı değişmez; çok parçalı sonuç açık bir seçimdir.

## Doğrulama

- **Dosya:** şema 5 örnekleri bağımsız Python yazıcısıyla üretilir; Rust ve Python okuyucusu onları okur. Parçasız çizimin baytları öncekiyle aynıdır.
- **Hesap:** iki kare parçalı alanın alanı, çevresi ve ağırlık merkezi el hesabıyla; |A ∪ B| + |A ∩ B| = |A| + |B| mm² içinde; yaylı parselle kesişimde yay aynı merkez ve yarıçapla kalır.
- **Değişim biçimleri:** MultiPolygon ve çok parçalı Shapefile gidiş-dönüşü; bağımsız okuyucu (`tools/formats/gis.py`) aynı değerleri okur.
- **Komutlar:** ortak durumlar iki platformda; tek geri alma adımı ve kilitli katman reddi.
