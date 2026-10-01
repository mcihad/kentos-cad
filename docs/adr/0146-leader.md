# ADR 0146: Kılavuz

- **Durum:** kabul edildi (2026-10-01). Yön sahibin kararıdır (biçim değişiklikleri özellikleriyle; sıra köşe kotu, çok parçalı alan, blok, yazı ekleri, kılavuz, yeni ölçü türleri). Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0145 (şema 7: yazı ekleri), ADR 0144 (blok, Patlat), ADR 0055 (çizimin yazıları), ADR 0060 (Yazı aracı ve yerinde düzenleme), ADR 0061 (ölçülendirme), ADR 0066 (Öznitelikler), ADR 0068 ve 0074 (tutamaçlar), ADR 0140 (“Sözleşmede yeni nesne türü isteyenler”), ADR 0009 (DXF); `docs/specs/kcad-v2.md` §6.6.

## Bağlam

Kroki, pafta ve vaziyet planında bir nesne okla gösterilir, okun ucuna not yazılır: “Mevcut bina”, “Ø150 PVC”, “Ada 101 Parsel 5”. AutoCAD'de bu LEADER'dır, Türkçesiyle kılavuz; birden çok oklusu MULTILEADER'dır (çoklu kılavuz). Netcad'de kullanıcı çizgiyle ve yazıyla elle çizer.

KentOS'ta bugün kılavuz yoktur:

- **Elle çizilen kılavuz dağılır.** Kullanıcı oku çoklu çizgiyle, notu Yazı ile ayrı çizer. Ok başı yoktur; taşıyınca ikisi ayrı gider, not kayar.
- **DXF'in kılavuzu bozulur.** LEADER çoklu çizgi olarak okunur, ok başı ve bağlı notu kaybolur (“ok başı ve bağlı yazı ayrı” notu). MULTILEADER hiç okunmaz (“AutoCAD'de patlatıp yeniden kaydedin”).
- **ADR 0140 bunu bekletti:** kılavuz, “sözleşmede yeni nesne türü isteyen” işler arasındadır. `.kcad` şeması, sunucunun `cad.rs`'i ve iki okuyucuyla ayrı bir kararla yapılacaktı.

Sahibin sırası (29 Eylül, ADR 0143 Bağlam): yazı eklerinden sonra kılavuz.

## Karar

### 1. Veri

Kılavuz yeni bir nesne türüdür, `leader` (arayüzde “Kılavuz”, AutoCAD'in Türkçesindeki ad; sahibin seçimi, 1 Ekim). Ok, kırık çizgi, kol ve not tek nesnedir: birlikte seçilir, taşınır, silinir. Alanları:

- **`pts` — köşeler:** en az iki. İlki okun ucudur (gösterilen yer), sonuncusu kolun başladığı yerdir.
- **`text` — not:** tek satır, yazı gibi. İsteğe bağlıdır: yokken kılavuz yalnız oktur, kol ve not çizilmez. Boş not yazılmaz; notun yokluğudur.
- **`height` — notun yüksekliği:** metre, sonlu ve 0'dan büyük. Yazı gibi kâğıt mm çarpı çizim ölçeğidir; araç böyle hesaplar.
- **`rotation` — notun doğrultusu:** derece, saat yönünün tersine (yazının `rotation`'ı gibi). Kol da bu doğrultudadır.
- **`arrow` — ok başı:** yokken dolu oktur. Numaralı metin, üç değer:
  - `open`: açık ok;
  - `dot`: dolu nokta;
  - `none`: ok başı yok.
  - Dolu ok bir değer değildir, alanın yokluğudur (yazının sol taban hizası gibi). Böylece bir kılavuzun tek bir yazılışı olur.
- **`mask` — notun zemini:** yazının zemini gibidir (ADR 0145). Yalnız `true` yazılır.
- **Nesnenin olağan alanları:** katman, renk, kalınlık, öznitelikler, etiket.
- **Adlar:** sözleşmede `LeaderEntity` ve `LeaderArrow`; TypeScript'te aynı adlar.

### 2. Ölçüler ve yerleşim

Her ölçü notun yüksekliği `h` cinsindendir. Bunlar AutoCAD'in varsayılan oranlarıdır: yazı 2,5 mm iken ok 2,5 mm, kol 5 mm. Kılavuz çizim ölçeğiyle büyür, yazı gibi.

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
- **Uzunluk:** kılavuzun uzunluğu köşelerinin uzunluğudur; kol sayılmaz. Alanı yoktur.

### 3. `.kcad`: belge şeması 8

- **Yeni tür:** `leader` türü şema 8'dedir. Yazıcı 8'i yalnız belgede ya da bir blok tanımında kılavuz varken yazar. Öbür çizimler şema 2–7 olarak kalır, eskisiyle bayt bayt aynıdır. Şema 8, şema 7'yi kapsar.
- **Eski şema:** şema 2–7 yükünde `leader` bilinmeyen türdür (`unknown_kind`). Eski okuyucu kılavuzu sessizce düşürmez, dosyayı açmaz.
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
  - yeni örnek dosyalar (`fixtures/kcad/v2`): her ok türü, notsuz, zeminli ve dönük kılavuz, blok tanımında kılavuz; şema 7'de `leader` reddi ve bozuk değerler.
- **Sunucu:** veritabanı projesi kılavuzu `cad_definition` olarak saklar (CLAUDE.md §15). Okuyucunun kurallarıyla denetler. İzdüşümü köşelerin LineString'idir; ok başı, kol ve not izdüşümde yoktur.

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
- **Patlat:** kılavuz parçalarına ayrılır:
  - köşeler ve kol bir çoklu çizgi olur;
  - not bir yazı olur (hizası, yüksekliği, dönüşü ve zeminiyle);
  - dolu ok ve nokta dolu tarama olur, açık ok çoklu çizgi olur.
- **Kenar düzenlemeleri:** Kır, Buda, Uzat, Ötele, Birleştir kılavuzu açık bir iletiyle reddeder: “önce Patlat”.
- **Blok:** tanım kılavuz içerebilir. Açılımda kılavuz kalır; Patlat'la yerleştirmeden kılavuz olarak çıkar.

### 5. Çizim

- **Çizgi, kol ve ok başı** sahnenin parçasıdır; nesnenin rengi ve kalınlığıyla çizilir. Dolu ok ve nokta dolgu olarak çizilir, iki çizicide aynı üçgenlerle.
- **Not** yazılar gibi sahnenin üstünde çizilir (ADR 0055). Depo, etiket kaydında notun taban çizgisinin başladığı noktayı, dönüşünü ve zemininin genişliğini verir (`LABEL_LEADER`). İki çizici notu yazının yolundan çizer.
- **Ekranda boy:** notun yüksekliği 5 pikselden küçükken not çizilmez, yazılar gibi; çizgi ve ok başı yine çizilir.

### 6. Komutlar

- **`cad.entities.create`:** geometrisine `leader` eklenir. Kılavuz aracı bununla, “Kılavuz” adımında yazar.
- **`cad.entities.edit`:**
  - `properties` işlemi notu, yüksekliği, dönüşü, oku ve zemini değiştirir;
  - `grip`, `vertexAdd` ve `vertexRemove` köşeleri değiştirir;
  - Patlat, yerleştirmedeki gibi kendi `add`'leriyle yazar.
- **`cad.entities.transform`:** dönüşümler çekirdekten gelir.
- **Ortak durumlar:** bağımsız Python üreticileriyle, üç koşucuda (web, masaüstü, Python SDK).

### 7. Araç ve arayüz

- **Kılavuz aracı:** Çizim › Açıklama'da, Yazı'nın yanında. Takma adlar: `KILAVUZ`, `LEADER`, `LE`, `MLEADER`, `MLD`.
  - İlk tık okun ucudur, sonraki tıklar köşelerdir.
  - Enter ya da sağ tık köşeleri bitirir. Yazı kutusu kolun ucunda açılır, notun yanına göre hizalanır.
  - Enter notu yazar. Boş kutuda Enter notsuz kılavuz yazar (yalnız ok). Esc bir adım geri gider.
  - Seçenekler:
    - Ok (O): çipinden açılan ikonlu menü (dolu, açık, nokta, yok);
    - Yükseklik (Y): kâğıt mm, Yazı'nınkiyle ortak;
    - Zemin (Z);
    - Geri (G): son köşeyi siler, yol araçlarındaki gibi.
  - Önizlemede ok başı, çizgiler, kol ve notun yeri görünür.
- **Öznitelikler:** “Kılavuz” bölümünde Not, Yükseklik, Dönüş, Ok (ikonlu açılır liste), Zemin, Köşe sayısı ve Uzunluk satırları. Çoklu seçimde farklı değer “Çeşitli” görünür; her değişiklik tek “Değiştir” adımıdır.
- **Yerinde düzenleme:** nota çift tıklamak yazı kutusunu notun yerinde açar (ADR 0060). Notsuz kılavuzda çizgiye çift tıklamak kutuyu notun duracağı yerde açar; boşaltılan not kaldırılır, kılavuz yalnız ok olur.

### 8. Değişim biçimleri

- **DXF okuma:**
  - **LEADER:**
    - köşeleri (10) kılavuzun köşeleridir.
    - Kolu varsa (75 = 1) son köşe kolun ucudur. Notu olan kılavuzda bu köşe atılır, yerini KentOS'un kolu alır. KentOS'un yazdığı kılavuz böylece aynen geri gelir.
    - Ok başı yoksa (71 = 0) ok yoktur. Varsa türü ok bloğunun adından gelir: önce nesnenin boyut stili değişikliği (ACAD DSTYLE, 341), sonra boyut stilinin DIMLDRBLK'i (341). `_Open…` açık, `_Dot…` ve `_Small` nokta, `_None` yok olur; öbür adlar dolu oktur ve söylenir. KentOS'un verisi açık ve nokta oku geri verir.
    - Bağlı notu 340'ın gösterdiği MTEXT'tir. Dosyada LEADER'dan önce ya da sonra gelebilir, iki durumda da kılavuza katılır, ayrı yazı olmaz.
      - MTEXT'in ilk satırı not olur. Yüksekliği, dönüşü ve zemini (90) kılavuzunkidir.
      - MTEXT'in öbür satırları ayrı yazı olarak notun altına gelir: ilk satırdan uzaklıkları korunur, notun hizasını ve dönüşünü alırlar. Rapor bunu söyler.
      - KentOS'un verisi, MTEXT hâlâ aynısını söylüyorsa notu ve dönüşü tam verir.
    - Bağlı notu olmayan kılavuz notsuzdur: 73 = 3; ya da 340 bir bloğu ya da toleransı gösterir; ya da gösterdiği nesne dosyada yoktur (son durumu rapor söyler).
      - Yüksekliği 40'tır. 40 yoksa boyut stilinin ok boyudur (DIMASZ × DIMSCALE, nesnenin değişikliğiyle). O da yoksa 2,5 alınır ve söylenir.
    - Eğri yollu kılavuz (72 = 1) kırık çizgi olur; rapor söyler.
  - **MULTILEADER:**
    - İlk ok çizgisinin köşeleri ve son noktası (kolun başı) kılavuzun köşeleridir.
    - MTEXT içeriği, yeri (12), yüksekliği (41), doğrultusu (13) ve zemini (292) ile LEADER'ın notu gibi kılavuza katılır. Ok başı bloğu 342'dir, adı LEADER'ınki gibi okunur.
    - Öbür ok çizgileri ve blok içerik alınmaz; rapor söyler. Blok içerikli ya da içeriksiz kılavuz notsuzdur, yüksekliği ok boyudur (42).
    - Eğri ok çizgisi (170 = 2) kırık çizgi olur; rapor söyler.
- **DXF yazma:**
  - Kılavuz bir LEADER'dır. Köşeleri yazılır; notu varsa son köşe olarak kolun ucu da yazılır (75 = 1; 74 kolun yanı; 211 notun doğrultusu). Ok başı 71'dir (yok için 0), yükseklik 40'tır.
  - Notu, bağlı bir MTEXT'tir. Notun yerinde, hizasıyla (71 = 4 sol orta, 6 sağ orta), yüksekliği ve doğrultusuyla yazılır. LEADER onu 340 ile, o LEADER'ı reaktörüyle gösterir.
  - Zemin MTEXT'in kendi zeminidir (90 = 3, çizimin zemin rengi); başka programlar da gösterir.
  - Açık ve nokta ok KentOS verisidir. Başka programlar dolu oku gösterir, KentOS geri okur; dışa aktarma raporu bunu söyler.
  - MTEXT'in yazımı notu ya da doğrultusu tam söyleyemediğinde KentOS verisi tam değeri taşır.
- **GeoJSON yazma:** köşelerin LineString'i yazılır. Not, yazılarda olduğu gibi yazılmaz; rapor söyler.
- **NCZ ve Shapefile:** kılavuzları yoktur, değişmez.

### 9. Kapsam dışı

- **Çok satırlı not:** yazının çok satırlısıyla birlikte ayrı karardır (ADR 0145 §8).
- **Çok oklu kılavuz** (birden çok ok tek nota) ve **blok içerikli kılavuz** (MULTILEADER'ın blok içeriği).
- **Eğri yollu kılavuz.**
- **Kılavuz stili tablosu:** ok boyu, kol boyu ve boşluk ayrı ayrı ayarlanamaz; bugün yükseklikten ölçülür.
- **Notun hizasının elle seçilmesi:** hiza kolun yanından gelir.
- **Bul ve değiştir ile Okunur yap'ta kılavuzun notu:** sonraki karar.

### 10. İş sırası

1. **Sözleşme ve `.kcad` şema 8.** Tür ve alanlar, kodek ve tipli sütunlar, belirtim, Python okuyucusu ve yazıcısı, örnekler; iki belge kılavuzu taşır; sunucu denetler ve saklar. *(1 Ekim: tamam. Sözleşmede `LeaderEntity` (köşeler, isteğe bağlı not, yükseklik, dönüş, isteğe bağlı ok, zemin) ve `LeaderArrow` (`open`, `dot`, `none`; dolu ok alanın yokluğu), `Entity::Leader`. KCAD şema 8 yalnız belgede ya da bir blok tanımında kılavuz varken yazılır; öbür çizimler bayt bayt aynı. Okuyucu şema 7 ve öncesinde `leader`'ı `unknown_kind`, ikiden az köşeyi, 0 ya da eksi yüksekliği, boş notu, bilinmeyen oku (`filled` dahil) ve `mask: false`'u `bad_value` olarak, yeriyle reddeder; yazıcı aynılarını yazmaz. Tipli sütunlarda yeni tür: köşeler, yükseklik ve dönüş; not metin, ok tam sayı, zemin bayrak (`FORMATS_VERSION` 17); web'in sütunları aynı düzende. Bağımsız Python yazıcısı ve okuyucusu: `leaders.kcad` (dört ok türü, notsuz, zeminli, dönük, kendi renkli ve kalınlıklı kılavuz; blok tanımında kılavuz) ve altı bozuk örnek; reddedilen şema artık 9. Web açılışta aynı kuralları denetler (`snapshot.ts`). İki belge kılavuzu taşır (`document-ops` senaryosu: köşe yaması, notun silinmesi, geri alma). Sunucu denetler, `cad_definition` olarak saklar, izdüşümü köşelerin LineString'idir. Çekirdeğin `Shape::Leader`'ı: dönüşümler (köşeler taşınır, dönüş yazınınki gibi, ölçek yüksekliği çarpar), tutamaçlar (köşeler ve parça ortaları; orta tutamaç köşe ekler), esnetme, kenet (köşeler uç, parça ortaları orta nokta), kenarlar ve depo paketi (tür 15; web ve masaüstü `store-records`'ta aynı sayıları verir). GeoJSON köşelerin LineString'ini yazar; not ve ok başı yazılmaz, rapor söyler. DXF yazıcısı 5. adıma dek kırık çizgiyi çoklu çizgi olarak yazar ve söyler. Öznitelikler'de Köşe sayısı ve Uzunluk, ifade dilinde `$köşe`. Bilinen ara durum: kılavuz şimdilik köşelerinin çizgisi olarak çizilir; ok başı, kol ve not 2. adımdadır. Komutların geometrisi (`EntityGeometry::Leader`) 3. adımdadır: o zamana dek tutamaç ve Öznitelikler kılavuzun geometrisini yazamaz, dönüşümler (`cad.entities.transform`) yazar.)*
2. **Çekirdek ve çizim.** Yerleşim işlevi ortak durumlarıyla; depo (seçme, kenet, kapsam, not etiketi), dönüşümler, tutamaçlar, Patlat'ın parçaları; iki çizici. *(1 Ekim: tamam. Yerleşim `geom::leader::layout` (web'e `leaderLayout`); bağımsız Python başvurusu `scripts/fixtures/leader_cases.py` on durumu `fixtures/leader/v1/layout.json`'a yazar, çekirdek ve web WASM'ı 1e-9 m içinde aynısını verir. Çizim kaydında yeni tür `MIXED`: çizgi kolun ucuna dek ve açık okun kolları, ardından dolu okun ya da noktanın alanı; stil motoru çizgileri katmanın çizgi simgesiyle (nesnenin rengi ve kalınlığı), alanı düz dolguyla çizer; iki platformun seçim ve üzerine gelme vurgusu aynı parçaları çizer. Not, yazının yolundan: `LABEL_LEADER` ve bloktaki kılavuz için `LABEL_PIECE_LEADER`; 5 pikselden küçük not çizilmez, kılavuzun etiketi çizilmez. Depo: tıklama çizgide, kolda ve notun kutusunda; pencere bütün kapsamla, kesişim ve çit notun kutusuyla da; kenet köşeler ve kolun ucu; kapsam köşeler, ok başı, kol ve notun kutusu; `entity_edges` kolu da sayar. Patlat: çoklu çizgi (köşeler ve kolun ucu), dolu ok ya da nokta için dolu tarama, açık ok için çoklu çizgi, not için hizası, yüksekliği, dönüşü ve zeminiyle yazı. Masaüstünün “Tümünü göster”i, orta tuşa çift tıklama ve açılıştaki görünüm artık web'inki gibi deponun kapsamını kullanır (blok yerleştirmesi parçalarıyla). Bilinen ara durum: bloktaki kılavuz masaüstünün seçim vurgusuna 3. adımda (`EntityGeometry::Leader`) girer; web'in blok parçası türü de kılavuzu o adımda öğrenir.)*
3. **Komutlar.** `create`, `edit` ve `transform`; ortak durumlar üç koşucuda. *(1 Ekim: tamam. Sözleşmede `EntityGeometry::Leader` (köşeler, isteğe bağlı not, yükseklik, dönüş, isteğe bağlı ok, zemin) ve `cad.entities.create`'in `leader` işlemi (adımı “Kılavuz”); TypeScript türleri, katalog ve Python SDK'sı yeniden üretildi. İki işleyici aynı kurallarla denetler: en az 2 köşe (`too_few_points`) ve boş olmayan not (`empty_text`) sayılardan önce, sıfırdan büyük yükseklik (`invalid_height`) sonlu sayılardan sonra; null not ve ok alanın yokluğudur. Ortak durumlar: `cad.entities.create` (yazma, redler, sıra), `cad.entities.edit` (Öznitelikler'den not, yükseklik, dönüş, ok ve zemin; tutamaç, köşe ekleme ve silme; redler; kilitli katman; Patlat'ın parçaları bağımsız yerleşim kuralından), `cad.entities.transform` (döndür, ölçekle, aynala; bağımsız dönüşüm başvurusu `affine_reference.py` kılavuzu da dönüştürür). Çekirdeğin Buda, Uzat, Kır ve Ötele'si kılavuzu “önce Patlat” iletisiyle reddeder. Komutlar kılavuzu yazabildiği için tutamaç, pano ve bloktaki kılavuzun masaüstü vurgusu da çalışır.)*
4. **Araç ve arayüz.** Kılavuz aracı, Öznitelikler, yerinde düzenleme; iki platformda izler ve resimler. *(1 Ekim: tamam. Kılavuz aracı web'de `tools/leaderTool.ts`, masaüstünde `kentos_interaction::leader`; istemler, iletiler, seçenekler (Ok, Yükseklik, Zemin, Geri) ve ok menüsünün ikonlu satırları iki platformda aynı. Yükseklik Yazı'nınkiyle ortaktır; Ok ve Zemin oturum boyunca hatırlanır. Kolun ucunda açılan yazı kutusu notun yanına göre hizalanır; boş kutuda Enter notsuz kılavuz yazar, Esc köşelere döner. Yazı kutusunun isteği bunun için boş girdi, yer tutucu ve ipucu taşır (web `TextInputRequest`, masaüstü `TextField`). Ortak iz `fixtures/interaction/v1/leader.json` (üç kılavuz: dolu ok ve not, açık ok ve zeminli not, notsuz nokta; Geri, Esc ile geri dönüş ve en az iki köşe uyarısı) iki platformda geçer. Öznitelikler web'de `leaderRows`, masaüstünde `properties::leader_rows`: Not, Yükseklik, Dönüş, Ok ve Zemin, çoklu seçimde “Çeşitli”, değeri zaten olan kılavuz yazılmadan tek “Değiştir” adımı. Yerinde düzenleme iki platformda; notsuz kılavuzun kutusu notun duracağı yerde açılır. Resimler: masaüstü `kilavuz-*`, web `shots.mjs leaders`.)*
5. **Biçimler.** DXF okuma ve yazma, GeoJSON; örnek dosyalar ve bağımsız denetim. *(1 Ekim: tamam. Okuyucu LEADER'ı ve 340'ın gösterdiği MTEXT'i dosyadaki sıraları ne olursa olsun birleştirir: önce gelen öbürünü bekler, kılavuz notun ilk satırının yerini alır ya da sonradan gelen notun ilk satırı kılavuza geçer; başka hiçbir nesne yer değiştirmez (`dxf/emit/notes.rs`). Boyut stilleri ve blok kayıtları tablolardan okunur (`DIMSTYLE`'ın 40, 41, 341'i; `BLOCK_RECORD`'un adı); nesnenin ACAD DSTYLE değişikliği önce gelir (`dxf/leaders.rs`). MULTILEADER iç içe bölümleriyle okunur. Blok tanımındaki ve Blokları patlat ile açılan kılavuz da notunu alır. Yazıcı LEADER ve MTEXT'i birbirini gösterecek biçimde yazar (`writer/entities/leader.rs`); web'in JSON okuyucusu kılavuzu okur. Okuma örneği `fixtures/formats/v1/leaders.dxf` (dokuz kılavuz, iki MULTILEADER, blok) elle hesaplı beklentilerle Rust'ta ve WASM'da; yazma örneği `dxf-write/leaders` iki platformda aynı baytlarla, bağımsız Python denetimiyle, geri okununca aynı kılavuzlar. MTEXT'in “biçimlendirme kaldırıldı” notu artık yalnız gerçek biçimlendirmede söylenir. Resimler: masaüstü `aktar-*-19…21`, web `shots.mjs leaders` (import-dxf-leaders…, export-dxf-leaders).)*

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Kılavuz tek nesne olduğundan ok, çizgi ve not birlikte taşınır ve düzenlenir. DXF'in kılavuzu ok başı ve notuyla gelir ve gider.
- Yeni bir nesne türü; şema, kodek, iki belge, iki çizici, depo, sunucu, komutlar ve biçimler birlikte değişir. Bu, bloktan (ADR 0144) sonraki en geniş şema adımıdır.
- Ok başı ve kol yükseklikten ölçüldüğünden kılavuz stili tablosu gerekmez. Ayrı ok boyu isteyen dosya (DXF'in DIMASZ'ı) yükseklik oranına yaklaşır; okuma bunu söyler.

## Doğrulama

- **Biçim:** bağımsız Python okuyucusu ve yazıcısıyla `fixtures/kcad/v2` örnekleri ve bozuk dosyalar; iki belge kılavuzu taşır (`fixtures/document-ops`); sunucunun reddi ve izdüşümü veritabanı testinde.
- **Hesap:** yerleşimin ortak durumları bağımsız hesapla (`fixtures/leader/v1`), iki platformda; seçme, kenet, dönüşüm ve Patlat çekirdek testlerinde.
- **Komutlar:** ortak durumlar web, masaüstü ve Python SDK'sında.
- **Araç:** iki platformda iz (`fixtures/interaction/v1`) ve resimler (açık ve koyu tema, 1440×900 ve 1100×650).
- **Biçimler:** DXF'in LEADER, MTEXT'li LEADER ve MULTILEADER örnekleri elle hesaplı beklentilerle; yazıcının örneği bağımsız denetimle (`dxf_write_reference.py`).
