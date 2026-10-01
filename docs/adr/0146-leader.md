# ADR 0146: Lider

- **Durum:** kabul edildi (2026-10-01). Yön sahibin kararıdır (biçim değişiklikleri özellikleriyle; sıra köşe kotu, çok parçalı alan, blok, yazı ekleri, lider, yeni ölçü türleri). Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0145 (şema 7: yazı ekleri), ADR 0144 (blok, Patlat), ADR 0055 (çizimin yazıları), ADR 0060 (Yazı aracı ve yerinde düzenleme), ADR 0061 (ölçülendirme), ADR 0066 (Öznitelikler), ADR 0068 ve 0074 (tutamaçlar), ADR 0140 (“Sözleşmede yeni nesne türü isteyenler”), ADR 0009 (DXF); `docs/specs/kcad-v2.md` §6.6.

## Bağlam

Kroki, pafta ve vaziyet planında bir nesne okla gösterilir, okun ucuna not yazılır: “Mevcut bina”, “Ø150 PVC”, “Ada 101 Parsel 5”. AutoCAD'de bu liderdir (LEADER; çok kılavuzlu MULTILEADER). Netcad'de kullanıcı çizgiyle ve yazıyla elle çizer.

KentOS'ta bugün lider yoktur:

- **Elle çizilen lider dağılır.** Kullanıcı oku çoklu çizgiyle, notu Yazı ile ayrı çizer. Ok başı yoktur; taşıyınca ikisi ayrı gider, not kayar.
- **DXF'in lideri bozulur.** LEADER çoklu çizgi olarak okunur, ok başı ve bağlı notu kaybolur (“ok başı ve bağlı yazı ayrı” notu). MULTILEADER hiç okunmaz (“AutoCAD'de patlatıp yeniden kaydedin”).
- **ADR 0140 bunu bekletti:** lider, “sözleşmede yeni nesne türü isteyen” işler arasındadır. `.kcad` şeması, sunucunun `cad.rs`'i ve iki okuyucuyla ayrı bir kararla yapılacaktı.

Sahibin sırası (29 Eylül, ADR 0143 Bağlam): yazı eklerinden sonra lider.

## Karar

### 1. Veri

Lider yeni bir nesne türüdür, `leader` (arayüzde “Lider”). Ok, kırık çizgi, kol ve not tek nesnedir: birlikte seçilir, taşınır, silinir. Alanları:

- **`pts` — köşeler:** en az iki. İlki okun ucudur (gösterilen yer), sonuncusu kolun başladığı yerdir.
- **`text` — not:** tek satır, yazı gibi. İsteğe bağlıdır: yokken lider yalnız oktur, kol ve not çizilmez. Boş not yazılmaz; notun yokluğudur.
- **`height` — notun yüksekliği:** metre, sonlu ve 0'dan büyük. Yazı gibi kâğıt mm çarpı çizim ölçeğidir; araç böyle hesaplar.
- **`rotation` — notun doğrultusu:** derece, saat yönünün tersine (yazının `rotation`'ı gibi). Kol da bu doğrultudadır.
- **`arrow` — ok başı:** yokken dolu oktur. Numaralı metin, üç değer:
  - `open`: açık ok;
  - `dot`: dolu nokta;
  - `none`: ok başı yok.
  - Dolu ok bir değer değildir, alanın yokluğudur (yazının sol taban hizası gibi). Böylece bir liderin tek bir yazılışı olur.
- **`mask` — notun zemini:** yazının zemini gibidir (ADR 0145). Yalnız `true` yazılır.
- **Nesnenin olağan alanları:** katman, renk, kalınlık, öznitelikler, etiket.
- **Adlar:** sözleşmede `LeaderEntity` ve `LeaderArrow`; TypeScript'te aynı adlar.

### 2. Ölçüler ve yerleşim

Her ölçü notun yüksekliği `h` cinsindendir. Bunlar AutoCAD'in varsayılan oranlarıdır: yazı 2,5 mm iken ok 2,5 mm, kol 5 mm. Lider çizim ölçeğiyle büyür, yazı gibi.

- **Ok başı:** okun ucunda, ilk parçanın doğrultusunda.
  - Dolu ok: boyu `h`, eni `h/3` olan dolu üçgen.
  - Açık ok: aynı üçgenin iki yan çizgisi.
  - Nokta: çapı `h/2` olan dolu daire.
  - İlk parça ok başından kısaysa ok başı yine çizilir; seçme ve kenet çizginin kendisini kullanır.
- **Kol:** son köşeden notun doğrultusunda `2h` uzunluğunda yatay çizgi.
- **Kolun yanı:** son parça notun doğrultusunda ileri gidiyorsa (izdüşümü sıfır ya da artı) kol sağa, değilse sola uzanır. Köşe sürüklenince kol ve not kendiliğinden yan değiştirir, AutoCAD'deki gibi.
- **Not:** kolun ucundan `h/2` ötededir.
  - Kol sağdaysa notun ortası solundan (`middleLeft`), soldaysa sağından (`middleRight`) tutturulur (ADR 0145'in hizaları).
  - Notun kutusu, zemini ve genişliği yazınınkiyle aynı ölçüdür (`TextPlace`).
- **Uzunluk:** liderin uzunluğu köşelerinin uzunluğudur; kol sayılmaz. Alanı yoktur.

### 3. `.kcad`: belge şeması 8

- **Yeni tür:** `leader` türü şema 8'dedir. Yazıcı 8'i yalnız belgede ya da bir blok tanımında lider varken yazar. Öbür çizimler şema 2–7 olarak kalır, eskisiyle bayt bayt aynıdır. Şema 8, şema 7'yi kapsar.
- **Eski şema:** şema 2–7 yükünde `leader` bilinmeyen türdür (`unknown_kind`). Eski okuyucu lideri sessizce düşürmez, dosyayı açmaz.
- **Okuyucu reddi:**
  - ikiden az köşe `bad_value`;
  - sonlu olmayan köşe, yükseklik ya da dönüş `non_finite`;
  - 0 ya da eksi yükseklik `bad_value`;
  - boş not `bad_value`;
  - bilinmeyen ok `bad_value`;
  - `mask: false` `bad_value`.
- **Birlikte değişenler (ADR 0025'in kuralı):**
  - spesifikasyon §6.6 ve şema 8'in paragrafı;
  - Rust kodeği ve tipli sütunlar (`io/columns.ts` ↔ `kcad/src/columns.rs`; yeni tür kodu, `FORMATS_VERSION` artar);
  - bağımsız Python okuyucusu ve yazıcısı;
  - yeni örnek dosyalar (`fixtures/kcad/v2`): her ok türü, notsuz, zeminli ve dönük lider, blok tanımında lider; şema 7'de `leader` reddi ve bozuk değerler.
- **Sunucu:** veritabanı projesi lideri `cad_definition` olarak saklar (CLAUDE.md §15). Okuyucunun kurallarıyla denetler. İzdüşümü köşelerin LineString'idir; ok başı, kol ve not izdüşümde yoktur.

### 4. Hesap (ortak çekirdek)

- **Yerleşim** (`geom::leader::layout`): ok başının üçgeni ya da dairesi, kolun iki ucu, notun yeri ve hizası. Tek işlevdir. İki platform onu kullanır; bağımsız hesapla sınanır (`fixtures/leader/v1/layout.json`).
- **Depo:**
  - Seçme: çizgiye, kola ve notun kutusuna tıklama; pencere ve çit seçimi kutudan.
  - Kenet: köşeler (okun ucu dahil) ve kolun ucu.
  - Kapsam: köşeler, ok başı, kol ve notun kutusu.
- **Tutamaç:** her köşe, çoklu çizgideki gibi; kenar ortası yeni köşe ekler. Son köşeyi taşımak kolu ve notu birlikte taşır. Köşe, iki köşe kalana dek silinebilir.
- **Dönüşümler:** taşı, kopyala, döndür, ölçekle, aynala ve hizala köşeleri taşır.
  - Dönüş `rotation`'a eklenir; ölçek yüksekliği çarpar.
  - Aynalamada yazının kuralı sürer: not okunur kalır, dönüşe 180° eklenir. Kolun yanı yeniden hesaplanır.
- **Patlat:** lider parçalarına ayrılır:
  - köşeler ve kol bir çoklu çizgi olur;
  - not bir yazı olur (hizası, yüksekliği, dönüşü ve zeminiyle);
  - dolu ok ve nokta dolu tarama olur, açık ok çoklu çizgi olur.
- **Kenar düzenlemeleri:** Kır, Buda, Uzat, Ötele, Birleştir lideri açık bir iletiyle reddeder: “önce Patlat”.
- **Blok:** tanım lider içerebilir. Açılımda lider kalır; Patlat'la yerleştirmeden lider olarak çıkar.

### 5. Çizim

- **Çizgi, kol ve ok başı** sahnenin parçasıdır; nesnenin rengi ve kalınlığıyla çizilir. Dolu ok ve nokta dolgu olarak çizilir, iki çizicide aynı üçgenlerle.
- **Not** yazılar gibi sahnenin üstünde çizilir (ADR 0055). Depo, etiket kaydında notun taban çizgisinin başladığı noktayı, dönüşünü ve zemininin genişliğini verir (`LABEL_LEADER`). İki çizici notu yazının yolundan çizer.
- **Ekranda boy:** notun yüksekliği 5 pikselden küçükken not çizilmez, yazılar gibi; çizgi ve ok başı yine çizilir.

### 6. Komutlar

- **`cad.entities.create`:** geometrisine `leader` eklenir. Lider aracı bununla, “Lider” adımında yazar.
- **`cad.entities.edit`:**
  - `properties` işlemi notu, yüksekliği, dönüşü, oku ve zemini değiştirir;
  - `grip`, `vertexAdd` ve `vertexRemove` köşeleri değiştirir;
  - Patlat, yerleştirmedeki gibi kendi `add`'leriyle yazar.
- **`cad.entities.transform`:** dönüşümler çekirdekten gelir.
- **Ortak durumlar:** bağımsız Python üreticileriyle, üç koşucuda (web, masaüstü, Python SDK).

### 7. Araç ve arayüz

- **Lider aracı:** Çizim › Açıklama'da, Yazı'nın yanında. Takma adlar: `LIDER`, `LEADER`, `LE`.
  - İlk tık okun ucudur, sonraki tıklar köşelerdir.
  - Enter ya da sağ tık köşeleri bitirir. Yazı kutusu kolun ucunda açılır, notun yanına göre hizalanır.
  - Enter notu yazar. Boş kutuda Enter notsuz lider yazar (yalnız ok). Esc bir adım geri gider.
  - Seçenekler:
    - Ok (O): çipinden açılan ikonlu menü (dolu, açık, nokta, yok);
    - Yükseklik (Y): kâğıt mm, Yazı'nınkiyle ortak;
    - Zemin (Z);
    - Geri al (U): son köşeyi siler.
  - Önizlemede ok başı, çizgiler, kol ve notun yeri görünür.
- **Öznitelikler:** “Lider” bölümünde Not, Yükseklik, Dönüş, Ok (ikonlu açılır liste), Zemin, Köşe sayısı ve Uzunluk satırları. Çoklu seçimde farklı değer “Çeşitli” görünür; her değişiklik tek “Değiştir” adımıdır.
- **Yerinde düzenleme:** nota çift tıklamak yazı kutusunu notun yerinde açar (ADR 0060).

### 8. Değişim biçimleri

- **DXF okuma:**
  - **LEADER:**
    - köşeleri (10) liderin köşeleridir; ok başı (71) varsa dolu oktur, yoksa ok başı yoktur;
    - bağlı notu (340'ın gösterdiği MTEXT) lidere katılır, ayrı yazı olmaz: ilk satırı not, yüksekliği ve dönüşü liderinkidir;
    - bağlı notu olmayan lider notsuzdur;
    - eğri yollu (72 = 1) lider kırık çizgi olur ve söylenir.
  - **MULTILEADER:**
    - ilk kılavuz çizgisinin köşeleri, kolu ve MTEXT içeriğinin ilk satırı lider olur;
    - öbür kılavuz çizgileri ve blok içerik alınmaz, söylenir.
- **DXF yazma:**
  - lider, bağlı bir MTEXT'li LEADER olarak yazılır: köşeler, son köşe olarak kolun ucu ve ok başı (71);
  - MTEXT notun hizasıyla yazılır (71 = 4 ya da 6) ve LEADER'ı 340 ile gösterir;
  - ok türü ve zemin KentOS verisidir; başka programlar dolu oku gösterir, KentOS geri okur. Dışa aktarma penceresi ve rapor bunu söyler.
- **GeoJSON yazma:** köşelerin LineString'i yazılır. Not, yazılarda olduğu gibi yazılmaz; rapor söyler.
- **NCZ ve Shapefile:** liderleri yoktur, değişmez.

### 9. Kapsam dışı

- **Çok satırlı not:** yazının çok satırlısıyla birlikte ayrı karardır (ADR 0145 §8).
- **Çok kılavuzlu lider** (birden çok ok tek nota) ve **blok içerikli lider** (MULTILEADER'ın blok içeriği).
- **Eğri yollu lider.**
- **Lider stili tablosu:** ok boyu, kol boyu ve boşluk ayrı ayrı ayarlanamaz; bugün yükseklikten ölçülür.
- **Notun hizasının elle seçilmesi:** hiza kolun yanından gelir.
- **Bul ve değiştir ile Okunur yap'ta liderin notu:** sonraki karar.

### 10. İş sırası

1. **Sözleşme ve `.kcad` şema 8.** Tür ve alanlar, kodek ve tipli sütunlar, belirtim, Python okuyucusu ve yazıcısı, örnekler; iki belge lideri taşır; sunucu denetler ve saklar.
2. **Çekirdek ve çizim.** Yerleşim işlevi ortak durumlarıyla; depo (seçme, kenet, kapsam, not etiketi), dönüşümler, tutamaçlar, Patlat'ın parçaları; iki çizici.
3. **Komutlar.** `create`, `edit` ve `transform`; ortak durumlar üç koşucuda.
4. **Araç ve arayüz.** Lider aracı, Öznitelikler, yerinde düzenleme; iki platformda izler ve resimler.
5. **Biçimler.** DXF okuma ve yazma, GeoJSON; örnek dosyalar ve bağımsız denetim.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Lider tek nesne olduğundan ok, çizgi ve not birlikte taşınır ve düzenlenir. DXF'in lideri ok başı ve notuyla gelir ve gider.
- Yeni bir nesne türü; şema, kodek, iki belge, iki çizici, depo, sunucu, komutlar ve biçimler birlikte değişir. Bu, bloktan (ADR 0144) sonraki en geniş şema adımıdır.
- Ok başı ve kol yükseklikten ölçüldüğünden lider stili tablosu gerekmez. Ayrı ok boyu isteyen dosya (DXF'in DIMASZ'ı) yükseklik oranına yaklaşır; okuma bunu söyler.

## Doğrulama

- **Biçim:** bağımsız Python okuyucusu ve yazıcısıyla `fixtures/kcad/v2` örnekleri ve bozuk dosyalar; iki belge lideri taşır (`fixtures/document-ops`); sunucunun reddi ve izdüşümü veritabanı testinde.
- **Hesap:** yerleşimin ortak durumları bağımsız hesapla (`fixtures/leader/v1`), iki platformda; seçme, kenet, dönüşüm ve Patlat çekirdek testlerinde.
- **Komutlar:** ortak durumlar web, masaüstü ve Python SDK'sında.
- **Araç:** iki platformda iz (`fixtures/interaction/v1`) ve resimler (açık ve koyu tema, 1440×900 ve 1100×650).
- **Biçimler:** DXF'in LEADER, MTEXT'li LEADER ve MULTILEADER örnekleri elle hesaplı beklentilerle; yazıcının örneği bağımsız denetimle (`dxf_write_reference.py`).
