# ADR 0142: Köşe kotu (Z)

- **Durum:** kabul edildi (2026-09-29). Yön sahibin kararıdır; ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-09-29
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0139 (şema 3: nesne çizgi kalınlığı), ADR 0138 (NCZ), ADR 0046 (GeoJSON, Shapefile); `docs/specs/kcad-v2.md` §6.1, §6.6; PiriCAD `netcad_plan.md` N-05; TODOS.md `NUM-09`.

## Bağlam

Netcad'de kot her yerdedir: noktanın Z'si, 3B çoklu çizgiler, kotlu parsel köşeleri, eğim ve eğik mesafe. KentOS'ta bugün yalnız nokta kot taşır (`point.z`).

Bu yüzden içe aktarmada kot kaybolur:

- **DXF:** 3B çoklu çizginin, çizginin ve LWPOLYLINE yüksekliğinin Z'si atılır (`formats/dxf/mod.rs`: "Z is dropped, except a POINT's elevation").
- **Shapefile:** PolyLineZ ve PolygonZ'nin Z'si atılır.

Sahibin kararı (29 Eylül): `.kcad` şemasını değiştiren işler kendi özellikleriyle ve kendi şema adımlarıyla gelir; ilk olarak köşe kotu.

## Karar

### 1. Veri

- **Çizgi:** `za`, `zb`, iki ucun kotu.
- **Çoklu çizgi ve kapalı alan:** `zs`, köşe başına bir kot, `pts` ile aynı uzunlukta. Alanın her deliğinin (`RingGeometry`) kendi `zs`'i olur.
- **Kotsuz köşe:** köşenin kotu yoksa değeri `null`'dır. "Kot yok" ayrı bir durumdur, 0 değildir. Kotlu ve kotsuz iki çizgi birleşince sonuç iki durumu da dürüstçe taşır.
- **Kotsuz nesne:** alan hiç yoktur, `zs` yazılmaz. Bugünkü her nesne böyle kalır.
- **Daire, yay, elips, yazı ve öbür türler:** bu adımda kot almaz. Gerekirse sonra, tek bir `z` ile gelir.
- **Değerin anlamı:** metredir, projenin düşey datumunda. Hangi yükseklik olduğu (ortometrik ya da elipsoidal) proje ayarının işidir (`NUM-04`); bu ADR onu değiştirmez.
- **Adlar:** sözleşmede (`contracts::entity`) `za`, `zb`, `zs`; TypeScript'te aynı adlar (`number | null` dizisi). Katalog şeması aralığı söylemez: kot eksi olabilir (deniz altı, kazı).

### 2. `.kcad`: belge şeması 4

- **Yeni alanlar:**
  - `line`: `za`, `zb` (float ya da `null`);
  - `polyline` ve `polygon`: `zs` (float ya da `null` dizisi, `pts` uzunluğunda);
  - halka: `zs`.
- **Yazıcı ne zaman 4 yazar:** şema 4'ü yalnız bir nesnede köşe kotu varken yazar. Kotsuz çizim şema 2 ya da 3 olarak kalır, eskisiyle bayt bayt aynıdır.
- **Okuyucu reddi:**
  - şema 2 ya da 3 yükünde bu alanlar bilinmeyen alandır (`unknown_field`);
  - uzunluğu `pts`'ten farklı bir `zs` `bad_value`'dur;
  - sonlu olmayan bir kot `bad_value`'dur.
- **Birlikte değişenler (ADR 0025'in kuralı):**
  - spesifikasyon §6.1, §6.6;
  - Rust kodeği ve tipli sütunlar (`io/columns.ts` ↔ `kcad/src/columns.rs`, `FORMATS_VERSION`);
  - bağımsız Python okuyucusu (`tools/kcad/kcad.py`);
  - bağımsız yazıcıyla yeni örnek dosyalar (`fixtures/kcad/v2`: kotlu çizgi, kotlu ve kotsuz köşeli çoklu çizgi, kotlu delikli alan, şema 3'te `zs` reddi).
- **Sunucu:** veritabanı projesi nesneyi `cad_definition` olarak saklar (§15). Kot orada da taşınır; PostGIS türevi 2B kalır, Z'li türev ayrı bir karardır.

### 3. Değişim biçimleri

- **DXF okuma:**
  - LINE'ın 10/11 Z'leri → `za`, `zb`.
  - 3B POLYLINE'ın (70 bayrağı 8) köşe Z'leri → `zs`.
  - LWPOLYLINE'ın yüksekliği (38) sıfırdan farklıysa bütün köşelerin kotudur. Sıfır yükseklik, 2B çizimlerin çoğunda anlamsız olduğundan kot sayılmaz.
  - Blokların içindekiler aynı kuralla okunur, eklemenin Z'si eklenir.
- **DXF yazma:** kotlu çoklu çizgi 3B POLYLINE, kotlu çizgi Z'li LINE olur. Kotsuz köşe 0 yazılır ve KentOS'un verisi (`xdata`) onun kotsuz olduğunu söyler. Rapor bunu söyler.
- **Shapefile:** PolyLineZ ve PolygonZ'nin Z'leri okunur.
- **GeoJSON:**
  - Okurken koordinatın üçüncü sayısı kottur.
  - Yazarken kotlu köşe üç sayıyla yazılır. Kotsuz köşe iki sayıyla yazılır; aynı dizide ikisi karışabilir, RFC 7946 buna izin verir.
- **Netcad NCZ:** çizgi ve alan kayıtlarında köşe kotu varsa okunur. Okuyucunun hangi kayıtlarda Z bulduğu bu ADR'nin ilk işinde ölçülüp buraya yazılır.
- **Rapor:** her okuyucu kaç nesnenin kotlu geldiğini raporuna yazar.

### 4. Hesap ve düzenleme

- **Dönüşümler:** taşı, kopyala, döndür, ölçekle, aynala, dizi ve hizala yalnız düzlemde çalışır; kot değişmez. Ölçekle de kotu ölçeklemez: belge 2.5B'dir (TODOS.md `NUM-09`).
- **Yeni köşenin kotu,** üzerinde durduğu kenarın iki ucunun kotundan, kenar boyunca doğrusal hesaplanır. Bu şu işlemlerde geçerlidir:
  - tutamaçla ya da Köşe ekle ile eklenen köşe;
  - Buda'nın ve Kır'ın kesme noktası;
  - Parçala'nın parçaları;
  - Köşe yuvarla'nın ve Pah'ın yeni uçları;
  - alan işlemlerinde kesişimden doğan köşe (kaynağın kenarından).
  Kenarın bir ucu kotsuzsa yeni köşe de kotsuzdur.
- **Uzat:** yeni ucun kotu, son kenarın eğimiyle doğrusal sürer. Son kenar kotsuzsa uç kotsuzdur.
- **Ötele:** her yeni köşe kaynaktaki karşılığının kotunu alır.
- **Birleştir:** kotlar köşeleriyle gider.
- **Patlat:** çoklu çizginin parçaları kotlarını `za`, `zb` olarak taşır.
- **Sadeleştir:** kalan köşeler kotlarını korur.
- **İçine tıklayarak alan ve Tarama'nın bölgesi:** bu adımda kotsuzdur.
- **Kotu henüz taşıyamayan işlem** kotu sessizce silmez: işlem yapılır, kotun düştüğü uyarıyla söylenir ("2 nesnenin kotu bu işlemde korunmadı."). Hangi işlemlerin taşıdığı bu ADR'nin sınamasıdır.
- **Uzunluk:**
  - Düzlem uzunluğu değişmez; alan, çevre, kenar ölçüsü ve ölçülendirme hep düzlemde kalır.
  - Bütün köşeleri kotlu nesnenin 3B uzunluğu ayrı bir değerdir (`length_3d`). Öznitelikler panelinde ve üzerine gelme kartında "3B uzunluk" olarak görünür.

### 5. Arayüz

- **Öznitelikler paneli:**
  - Çizgide "Kot (başlangıç)" ve "Kot (bitiş)" satırları.
  - Çoklu çizgi ve alanda "Kot" satırı; köşeler aynıysa değeri, değilse "en düşük – en yüksek" aralığı gösterir, kotsuzsa "kot yok" der. Satır düzenlenince bütün köşelerin kotu o değer olur.
- **Tutamaç:** üzerine gelinen köşenin kotunu imleç yanında gösterir.
- **Değiştir › Kot paneli, Kot ver aracı (`tool.setElevation`).** Seçili nesnelerin köşelerine yazar, üç biçimi vardır:
  - sabit: her köşeye aynı kot;
  - artır: her köşeye verilen fark eklenir;
  - sıfırla: kot yok.
  Kot ver `cad.entities.edit`'in yeni `elevation` işlemiyle yazar: tek geri alma adımı, kilitli katmanda ret, iki platformda ortak durumlar (`fixtures/commands/v1`, bağımsız Python denetimiyle).
- **Nokta:** noktanın `z`'si olduğu gibi kalır. Kot ver noktaya da yazar.
- **Koordinat oku ve kenet:** kotlu köşede kotu da söyler: "Y … X … Z …".

### 6. İş sırası

1. **Sözleşme, `.kcad` şema 4 ve bellekteki belge.** Spesifikasyon, kodek, sütunlar, Python okuyucusu ve fixture'lar birlikte. Web modeli ve masaüstünün `kentos-domain`'i kotu taşır; kaydet ve aç kotu bayt bayt korur.
2. **Değişim biçimleri:** DXF okuma ve yazma, Shapefile, GeoJSON, NCZ; bağımsız okuyucularla fixture'lar.
3. **Hesap:**
   - dönüşümler;
   - kenar boyunca doğrusal kot;
   - taşıyamayan işlemin uyarısı;
   - `length_3d`.
4. **Arayüz:** Öznitelikler, tutamaç, Koordinat oku ve Kot ver; iki platformda, resimleriyle.

## Sonuçlar

- DXF'ten, Shapefile'dan, GeoJSON'dan ve NCZ'den gelen kotlar kaybolmaz. Kaydet ve aç onları korur.
- Kotsuz çizim bugünkü dosyayla bayt bayt aynıdır; eski okuyucu kotsuz dosyayı açmayı sürdürür.
- Belge 2.5B kalır: düzlem geometrisi ve kot ayrıdır, katı modelleme yoktur. Yüzey (üçgen model), eşyükselti ve hacim bu kotu okuyacak sonraki işlerdir (TODOS.md `CIVIL-02`, `CIVIL-08`).

## Doğrulama

- **Dosya:** `.kcad` şema 4 örnekleri bağımsız Python yazıcısıyla üretilir ve Rust ile Python okuyucusu onları okur. Kotsuz çizimin baytları öncekiyle aynıdır.
- **Değişim biçimleri:** kotlu DXF, Shapefile ve GeoJSON gidiş-dönüşünde kot korunur; bağımsız okuyucular (`tools/formats/gis.py`) aynı değerleri okur.
- **Hesap:** kenar boyunca kot el hesabıyla sınanır. Örneğin (0,0,10)–(10,0,20) kenarının ortası 15'tir. Taşı ve Döndür kotu değiştirmez.
- **Kot ver:** ortak durumlar iki platformda, tek geri alma adımı ve kilitli katman reddiyle.
