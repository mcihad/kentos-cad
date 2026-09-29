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
  - uzunluğu `pts`'ten farklı bir `zs` `bad_value`'dur, yeriyle (`…/holes/0/zs`);
  - sonlu olmayan bir kot, CBOR profilinin her sayı için kuralıyla `non_finite`'tir;
  - kotu olmayan çizgi ucunun anahtarı yazılmaz; `null` yalnız `zs`'in içinde geçerlidir (`za: null` `wrong_type`).
- **Birlikte değişenler (ADR 0025'in kuralı):**
  - spesifikasyon §6.1, §6.6;
  - Rust kodeği ve tipli sütunlar (`io/columns.ts` ↔ `kcad/src/columns.rs`, `FORMATS_VERSION`);
  - bağımsız Python okuyucusu (`tools/kcad/kcad.py`);
  - bağımsız yazıcıyla yeni örnek dosyalar (`fixtures/kcad/v2`: kotlu çizgi, kotlu ve kotsuz köşeli çoklu çizgi, kotlu delikli alan, şema 3'te `zs` reddi).
- **Sunucu:** veritabanı projesi nesneyi `cad_definition` olarak saklar (§15). Kot orada da taşınır; PostGIS türevi 2B kalır, Z'li türev ayrı bir karardır.

### 3. Değişim biçimleri

- **Sıfır kuralı** (DXF, NCZ): her yerde Z'si 0 olan nesne kotsuzdur. 2B çizimler sahte sıfır kot almaz. Bir köşesi sıfırdan farklıysa bütün köşeler, sıfırlar da, kotunu korur. Ayrıca şu nesneler kotludur:
  - yüksekteki bir bloğun içindeki nesne;
  - düzlem yüksekliği olan nesne;
  - KentOS'un `xdata`'sı "bu sıfırlar veridir" diyen nesne.
- **DXF okuma:**
  - LINE'ın 10/11 Z'leri → `za`, `zb`.
  - 3B POLYLINE'ın (70 bayrağı 8) köşe Z'leri → `zs`.
  - LWPOLYLINE'ın yüksekliği (38) ve 2B POLYLINE'ın başlık yüksekliği bütün köşelerin kotudur. Ekstrüzyon hesaba katılır: aynalı düzlemde kot eksidir, eğik düzlemde köşeden köşeye değişir.
  - Blokların içindekiler aynı kuralla okunur, eklemenin Z'si (Z ölçeğiyle) eklenir.
  - Kapanan çoklu çizgi ilk köşesini tekrarlıyorsa ilk köşenin kotu kalır; farklıysa not düşülür.
  - 3DFACE, SPLINE, LEADER ve HATCH'in Z'si bu adımda düşer.
- **DXF yazma:**
  - Kotlu çoklu çizgi ve alan 3B POLYLINE, kotlu çizgi Z'li LINE (30, 31) olur.
  - Kotsuz köşe 0 yazılır. KentOS'un verisi (`xdata` `noz`, köşe başına bir bit) onun kotsuz olduğunu söyler; KentOS geri okurken kotsuz sayar. Başka bir program o köşenin Z'sini değiştirmişse işaret geçersiz sayılır.
  - Yaylı çoklu çizgi 3B POLYLINE olamaz:
    - köşe kotları eşitse LWPOLYLINE yüksekliği (38) olarak kayıpsız yazılır;
    - farklıysa yaylar korunur, kotlar yazılmaz ve rapor bunu söyler.
  - Kotsuz çizim eskisiyle bayt bayt aynıdır.
- **Shapefile:**
  - PolyLineZ ve PolygonZ'nin Z'leri okunur, deliklerinki de.
  - Z türünde 0 bir kottur.
  - Sonlu olmayan Z köşeyi kotsuz bırakır ve not düşülür.
- **GeoJSON:**
  - Okurken koordinatın üçüncü sayısı kottur; iki sayılı konum kotsuzdur. Dördüncü ve sonraki sayılar düşer, raporlanır.
  - Yazarken kotlu köşe üç sayıyla, kotsuz köşe iki sayıyla yazılır; aynı dizide ikisi karışabilir (RFC 7946). Yayın örneklenen noktaları iki ucun kotunu yay boyunca paylaşır.
  - Kotsuz çıktı eskisiyle bayt bayt aynıdır.
- **Netcad NCZ:**
  - Okuyucu her köşenin Z'sini zaten okur (`ncz::format::Coord.z`). Çizginin iki Z'si, çoklu çizginin köşe Z'leri ve noktadan noktaya çizilen kapalı alan kot olur; sıfır kuralı geçerlidir.
  - Kutu, pafta, üçgen, akıllı nesne ve sıkıştırılmış eğri köşeleri kot almaz: Z'leri ya uydurma 0'dır ya da okunmaz.
  - Z'nin yerinde kalem kalınlığı duran noktalar kot sayılmaz.
- **Rapor:** her içe aktarma, raporun kaynak bilgisinde kotlu nesneleri sayar: "Kotlu nesne: n". Eski "Z (yükseklik) düştü" notu kalktı.

### 4. Hesap ve düzenleme

- **Dönüşümler:** taşı, kopyala, döndür, ölçekle, aynala, dizi ve hizala yalnız düzlemde çalışır; kot köşeleriyle, deliklerinki de, olduğu gibi gider (`with_shape`, web'de `withGeometry`). Ölçekle de kotu ölçeklemez: belge 2.5B'dir (TODOS.md `NUM-09`).
- **Düzenlemenin yazdığı köşelerin kotu** (`cad.entities.edit`). Çekirdeğin geometrisi kotsuz gelir. İşleyici (iki platformda aynı) her köşeye kotu, komutun adını verdiği nesnelerden, ortak çekirdeğin kuralıyla verir (`ops::elevation::carry_elevations`). İlk uyan kural geçerlidir:
  1. Bir kaynak köşesiyle çakışan köşe (1 µm) onun kotunu alır: Yönü çevir, Patlat, Birleştir, Sadeleştir, köşe silme.
  2. Köşeyi kendisi taşıyan işlemde (tutamaç, Esnet, Öznitelikler'e yazılan koordinat) ve Ötele'nin kopyasında, köşe sayısı aynıysa, aynı sıradaki köşenin kotu geçer.
  3. Bir kaynak kenarının üstündeki köşe (1 µm), kenarın iki ucunun kotundan kenar boyunca doğrusal kot alır: düz kenarda uzunlukla, yayda açıyla. Buda, Kır, Parçala, Köşe ekle, Köşe yuvarla ve Pah'ın uçları, alan işlemlerinin kesişimleri böyledir. Kenarın bir ucu kotsuzsa yeni köşe de kotsuzdur.
  4. Açık yolun ucu, bir kaynağın düz uç kenarının uzantısındaysa, o kenarın eğimi sürer (Uzat, Uzat-kısalt).
  5. Ötele'de öbür köşeler kaynağın en yakın noktasının kotunu alır.

  Hiçbir kural uymazsa köşe kotsuzdur.
- **Kotu taşıyamayan sonuç** kotu sessizce silmez. Sonuç yazılır ve komut uyarır: `elevation_lost`, "n nesnenin kotu bu işlemde korunmadı.", yol `changes`. Örneğin kotlu çizgi yaya dönerse yay kot taşımaz.
- **İçine tıklayarak alan ve Tarama'nın bölgesi** bu adımda kotsuzdur.
- **Uzunluk:**
  - Düzlem uzunluğu değişmez; alan, çevre, kenar ölçüsü ve ölçülendirme hep düzlemde kalır.
  - Bütün köşeleri kotlu nesnenin 3B uzunluğu ayrı bir değerdir (`length_3d`). Öznitelikler panelinde ve üzerine gelme kartında "3B uzunluk" olarak görünür.

### 5. Arayüz

- **Öznitelikler paneli:**
  - Çizgide "Kot (başlangıç)" ve "Kot (bitiş)" satırları.
  - Çoklu çizgi ve alanda "Kot" satırı; köşeler aynıysa değeri, değilse "en düşük–en yüksek" aralığını gösterir (uzun tire boşluksuz: dört basamaklı kotlar da hücreye sığar), kotsuzsa "kot yok" der. Satır düzenlenince bütün köşelerin kotu o değer olur.
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
