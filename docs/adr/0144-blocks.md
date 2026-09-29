# ADR 0144: Blok

- **Durum:** kabul edildi (2026-09-29). Yön sahibin kararıdır (biçim değişiklikleri özellikleriyle; sıra köşe kotu, çok parçalı alan, blok). Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-09-29
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0143 (şema 5), ADR 0009 (DXF: bloklar bugün patlatılır), ADR 0037 ve 0047 (dönüşüm ve düzenleme komutları), ADR 0006 (CAD ve PostGIS kaynağı), ADR 0138 (NCZ); TODOS.md `CAD-04`, `FILE-08`, `PG-19`, `PG-22`, `FMT-02`.

## Bağlam

Harita CAD'inde aynı çizim parçası yüzlerce kez kullanılır: rögar, elektrik direği, ağaç, poligon noktası işareti, kapı, pafta anteti. AutoCAD'de ve Netcad'de bu, bir kez tanımlanan ve çok kez yerleştirilen **blok**tur. Tanım düzeltilince bütün yerleştirmeler değişir.

KentOS'ta blok yok:

- DXF okuyucusu INSERT'leri açıp içlerindeki nesneleri tek tek çizime ekler (ADR 0009). Bağ kopar: yüz rögarın biri düzeltilirse öbürleri değişmez.
- Aynı işaretin yüz kopyası yüz kez saklanır ve düzenlenir.
- DXF'e geri yazılan çizimde blok yoktur.

Stil sembolleri (ADR 0090–0094) başka bir kavramdır. Onlar katmanın görünüşüdür; nesnenin kendi geometrisi değildir. Blok ise çizimin kendi geometrisidir.

## Karar

### 1. Veri

- **Blok tanımı** (`BlockDefinition`) belgenin `blocks` listesindedir. Alanları:
  - `id`: kalıcı kimlik (UUID);
  - `name`: benzersiz ad; Türkçe büyük/küçük harf farkı gözetilmeden karşılaştırılır;
  - `base`: taban noktası, tanımın kendi koordinatlarında;
  - `entities`: içindeki nesneler, sırasıyla; kimlikleri tanımın içinde yereldir;
  - `attributes`: öznitelik tanımları (§7);
  - `description`: isteğe bağlı açıklama.
- **İçindekiler** her türden nesne olabilir; başka bir bloğun yerleştirmesi de (iç içe blok). Döngü yasaktır: tanım kendini doğrudan ya da dolaylı olarak içeremez. İç içelik en çok 16 düzeydir.
- **Yerleştirme** yeni bir nesne türüdür, `insert` (arayüzde “Blok”). Alanları:
  - `block`: tanımın `id`'si;
  - `p`: ekleme noktası;
  - `scale`: pozitif ölçek, tek sayı;
  - `rotation`: saat yönünün tersine radyan;
  - `mirror`: tanımın x ekseninde aynalanıp aynalanmadığı;
  - nesnenin olağan alanları: katman, renk, öznitelikler, etiket, kalınlık.
- **Dönüşüm sırası:** taban noktasından ekleme noktasına öteleme; aynalama, ölçek ve dönüş taban noktası çevresinde. Dönüşüm benzerliktir, şekil türü korunur: daire daire kalır, yay yay. Eşit olmayan eksen ölçeği yoktur; DXF'ten böyle gelen yerleştirme açılır (§5).
- **Görünüş:** içindekiler yerleştirmenin katmanıyla çizilir: görünürlük, kilit ve stil onundur (DXF'in 0 katmanı ve BYBLOCK anlamı). Kendi rengi ya da kalınlığı olan içerik nesnesi onları korur. İçerik nesnesinin kendi katmanı saklanır ama çizimi etkilemez. Bu v1 sadeleştirmesidir; DXF'ten başka katmanda içerikle gelen blok raporlanır.
- **Başvuru bütünlüğü:**
  - Yerleştirilmiş bir tanım silinemez.
  - Bilinmeyen tanıma başvuran yerleştirme, dosyada okunmaz (`unknown_block`); belgede de yazılmaz.

### 2. `.kcad`: belge şeması 6

- `blocks` alanı ve `insert` türü şema 6'dadır.
- Yazıcı 6'yı yalnız belgede bir tanım ya da bir yerleştirme varken yazar. Başka her çizim 2–5'tir ve eskisiyle bayt bayt aynıdır. Şema 6, şema 5'i kapsar.
- Şema 2–5 yükünde `blocks` ve `insert` bilinmeyen alan ya da türdür. Eski okuyucu reddeder, sessizce düşürmez.
- **Kodlama:**
  - Tanımlar kendi bölümünde yazılır.
  - İçindekiler, belgenin nesneleriyle aynı nesne kodlamasını kullanır.
  - Yerleştirme tipli sütunlarda yeni bir tür kodudur; `FORMATS_VERSION` artar.
- **Okuyucunun denetimleri:**
  - benzersiz ad ve kimlik;
  - döngü ve derinlik;
  - bilinmeyen başvuru;
  - ölçeğin pozitif ve sonlu olması.
- Belirtim, bağımsız Python okuyucu ve yazıcı, örnek ve bozuk dosyalar kodekle birlikte değişir.

### 3. Hesap (ortak çekirdek)

- **Açılım:** yerleştirme tanımın şekillerine dönüşümle açılır; iç içe olanlar da. Tanım başına bir kez hesaplanır, yerleştirme başına dönüştürülür.
- **Depo:** yerleştirme tek bir öğedir. Çizimi, seçimi (içindekilerden birine tıklama ya da pencere), kutusu ve keneti (içindekilerin kenet noktaları ile ekleme noktası) açılımından gelir. Tanım değişince onu kullanan her yerleştirme yeniden açılır: bir kez düzenle, her yerde güncellensin.
- **Tutamaç:** ekleme noktası.
- **Dönüşümler:** Taşı, kopyala, döndür, ölçekle, aynala ve hizala yerleştirmenin alanlarını değiştirir; tanıma dokunmaz. Aynala `mirror`'ı çevirir, dönüşü düzeltir.
- **Patlat:** yerleştirme tanımın içindekilerine döner. Dönüşüm uygulanır, bir düzey açılır, iç içe yerleştirmeler yerleştirme kalır. Katmanı boş (bloğun) içerik yerleştirmenin katmanını alır. İçerik nesnesi kendi öznitelik ve rengini, yerleştirmenin öznitelikleri tanım nesnesi olmayan öznitelikleri korur.
- **Ölçüler:** yerleştirmenin uzunluğu ve alanı yoktur. Öznitelikler'de blok adı, ekleme noktası, ölçek, dönüş ve aynalama gösterilir.
- Kır, Buda, Uzat, Ötele gibi kenar düzenlemeleri yerleştirmeyi açık bir iletiyle reddeder: “önce Patlat”.

### 4. Komutlar

- **`cad.blocks.define` v1:**
  - Girdi: ad, taban noktası ve nesneler (kimlikleriyle).
  - Yeni tanım yazılır. `replace` ile seçilen nesneler silinir, yerine tanımın taban noktasına bir yerleştirme konur. Hepsi tek adımda.
  - Kilitli katmandaki nesne tanıma alınır ama `replace` ile silinmez: nesnelerden biri kilitliyse `replace` reddedilir.
- **Yerleştirme:** `cad.entities.create`'in geometrisi `insert` türünü alır.
- **`cad.blocks.edit` v1:**
  - `rename`: yeniden adlandırma;
  - `redefine`: içindekileri seçilen nesnelerden yeniden kurma; bütün yerleştirmeler güncellenir;
  - `rebase`: taban noktasını taşıma;
  - `remove`: kullanılmayan tanımı silme;
  - `purge`: kullanılmayanların hepsini silme.
  - Her biri tek geri alma adımıdır.
- Ortak durumlar `fixtures/commands/v1`'de, bağımsız Python denetimiyle; iki platform geçer.

### 5. Değişim biçimleri ve sunucu

- **DXF okuma:** BLOCK tanım, INSERT yerleştirme olur; ATTRIB değerleri yerleştirmenin öznitelikleridir.
  - İçe aktarma penceresinde “Blokları patlat” seçeneği vardır; varsayılanı kapalıdır.
  - Eşit olmayan eksen ölçekli INSERT, MINSERT ve kâğıt uzayı blokları bugünkü gibi açılır ve raporlanır.
  - İçinde başka katmanda nesne olan blok raporlanır (§1 görünüş).
- **DXF yazma:** tanım BLOCK ve blok kaydı, yerleştirme INSERT olur. İç içe tanımlar önce yazılır.
- **GeoJSON:** yerleştirme, açılımının geometrisiyle (GeometryCollection) ve kendi öznitelikleriyle yazılır; raporlanır. Okumada blok yoktur.
- **NCZ:** Netcad blokları ayrı adımdır; bu ADR'nin kapsamı dışındadır.
- **Sunucu (PostGIS):**
  - Yerleştirmenin kaynağı `cad_definition`'dır; izdüşümü açılımının GeometryCollection'ıdır.
  - Tanımlar projenin kendi tablosunda, kimlikleriyle saklanır. Tanım değişince onu kullanan yerleştirmelerin izdüşümü yenilenir.
  - Bu adımın göçü ayrıdır; o adımda planlanır.

### 6. Araçlar ve arayüz

- **Blok oluştur:** seçimden önce seçme; ad ve taban noktası sorulur; “Seçilenleri blokla değiştir” seçeneği vardır.
- **Blok ekle:**
  - Tanım listeden, önizlemesiyle seçilir.
  - Ekleme noktası tıklanır ya da yazılır.
  - Ölçek ve dönüş, seçenekle ya da yazılarak verilir.
  - Öznitelik tanımı olan blokta değerler sorulur.
- **Bloklar paneli:**
  - her tanım için önizleme, ad ve yerleştirme sayısı;
  - yeniden adlandırma, taban noktası, yeniden tanımlama (seçimden) ve kullanılmayanları temizleme;
  - çift tıkla Blok ekle.
- **Öznitelikler:** yerleştirmenin satırları (blok, Y, X, ölçek, dönüş, aynalı) düzenlenir. Patlat yerleştirmeyi de açar.
- Adlar ve iletiler iki platformda aynıdır. Bir kullanım senaryosu (`usage-blocks`) iki platformda oynatılır ve resimlenir.

### 7. Öznitelik tanımları

- **Tanım:** her öznitelik tanımı bir etiket (`tag`), bir soru metni, bir varsayılan değer ve bir yazı yerleşimi taşır. Yazı yerleşimi nokta, yükseklik, açı ve hizadır.
- **Değer:** yerleştirmenin o etiketteki özniteliğidir. Değer boşsa varsayılan gösterilir.
- **Çizim:** değerler tanımdaki yerde, yerleştirmenin dönüşümüyle yazı olarak çizilir.
- **DXF:** ATTDEF ve ATTRIB karşılığıdır.

### 8. Kapsam dışı

- **Dış başvuru (xref):** ayrı karardır.
- **Dinamik bloklar ve parametreli bloklar:** ayrı karardır.
- **Blok düzenleme penceresi:** tanımı yerinde, ayrı bir düzenleme kipinde değiştirmek sonraki adımdır. v1'de yeniden tanımlama seçilen nesnelerden yapılır.

### 9. İş sırası

1. **Sözleşme ve `.kcad` şema 6.** Tanımlar, `insert`, tipli sütunlar, belirtim, Python okuyucu ve yazıcı, örnekler. İki belge tanımları geri alınabilir durum olarak taşır. *(29 Eylül: tamam, iki platformda. Çekirdeğin `Shape::Insert`'i de bu adımda geldi: dönüşümler benzerliği birleştirir, depo yerleştirmeyi ekleme noktasıyla çizer ve seçer; açılım 2. adımdır. Veritabanı projesi blokları 5. adıma dek açık bir iletiyle reddeder.)*
2. **Çekirdek.** Açılım, depo (çizim, seçme, kenet, kutu), dönüşümler, Patlat, iki çizici. *(29 Eylül: tamam, iki platformda; Patlat 3. adımda. `kentos_geometry_core::block` tanımı bir kez düzleştirir: iç içe yerleştirmeler benzerlikleriyle açılır, kendi rengi ve kalınlığı olmayan iç nesne iç yerleştirmeninkini alır, parçalar taban noktasına göre tutulur, çeyrek dönüşler tamdır. Depo yerleştirmeyi parçalarıyla çizer (`GROUP` kaydı), seçer (parçaya tıklama, kapalı parçanın içi, pencere, kesişim, çit, daire), kenetler (parçaların noktaları, ekleme noktası düğüm), kutusunu parçalardan alır; tanım değişince her yerleştirme yeniden açılır. Bloğun yazıları ve ölçü değerleri yerleştikleri yerde çizilir (`LABEL_PIECE_TEXT`, `LABEL_PIECE_DIMENSION`). Stil çekirdeği parçaları kendi renk ve kalınlıklarıyla, yoksa yerleştirmeninkiyle çizer; tarama parçası yerleşmiş deseniyle döner. Vurgu ve dönüşüm hayaleti parçalarladır. `usage-blocks-view` izi iki platformda, üç varyantta ve resimleriyle. Web'in kaydı tanımları düşürüyordu (`projectHead`); düzeltildi, sayfanın yolundan `blocks.kcad` bayt bayt yazılır. Patlat, yerleştirme yaratmayla birlikte 3. adımdadır: sözleşmenin geometrisi `insert`'i o adımda alır.)*
3. **Komutlar.** `cad.blocks.define`, `insert` yaratma, `cad.blocks.edit`; ortak durumlar.
4. **Araçlar ve arayüz.** Blok oluştur, Blok ekle, Bloklar paneli, Öznitelikler; `usage-blocks` senaryosu ve resimler.
5. **Biçimler ve sunucu.** DXF okuma ve yazma, GeoJSON, PostGIS.
6. **Öznitelik tanımları.** Gösterim, soru, düzenleme, DXF ATTDEF ve ATTRIB.

## Sonuçlar

- DXF'ten gelen bloklar bağlarını korur ve DXF'e blok olarak döner.
- Bir işaret bir kez düzeltilir, her yerde değişir; dosya küçülür.
- Bloksuz çizimin baytları öncekiyle aynıdır; eski okuyucu onu açmayı sürdürür.

## Doğrulama

- **Dosya:** şema 6 örnekleri bağımsız Python yazıcısıyla üretilir; döngülü, bilinmeyen başvurulu ve yinelenen adlı bozuk örnekler reddedilir.
- **Hesap:** açılımın koordinatları el hesabıyla: 90° ve aynalı dönüşte tam sayılar kalır. İç içe iki düzey. Patlat–yeniden tanımla gidiş-dönüşü.
- **Komutlar:** ortak durumlar iki platformda; tek geri alma adımı; kullanılan tanımın silinmesinin reddi.
- **Biçimler:** DXF gidiş-dönüşü; ezdxf ile denetim.
- **Kullanım:** `usage-blocks` senaryosu iki platformda.
