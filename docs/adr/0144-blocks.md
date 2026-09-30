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
  - Girdi: ad, taban noktası ve nesneler (kimlikleriyle), isteğe bağlı açıklama; `replace` ile yerleştirmenin katmanı (`layerId`; Blok oluştur etkin katmanı verir, verilmezse `no_layer`).
  - Nesneler olduğu gibi kopyalanır (geometri, katman, renk, kalınlık, öznitelik, etiket, sembol); yerel kimlikleri girdinin sırasıyla 1'den; iki kez verilen kimlik tek nesnedir.
  - Yeni tanım yazılır. `replace` ile seçilen nesneler silinir, yerine tanımın taban noktasına bir yerleştirme konur. Hepsi tek adımda.
  - Kilitli katmandaki nesne tanıma alınır ama `replace` ile silinmez: nesnelerden biri kilitliyse `replace` reddedilir.
- **Yerleştirme:** `cad.entities.create`'in ve `cad.entities.edit`'in geometrisi (`EntityGeometry`) `insert` türünü alır: blok, yer, ölçek, dönüş, aynalama (yalnız true yazılır). Blok çizimin olmalı (`unknown_block`, katman denetiminden sonra), ölçek sıfırdan büyük (`invalid_scale`).
- **Patlat:** `cad.entities.edit`'in `add` değişikliği isteğe bağlı kendi katmanını, rengini, kalınlığını, özniteliklerini ve etiketini alır; verilen alan `from`'unkinin yerine geçer. Patlat bir yerleştirmeyi tanımının nesnelerine böyle açar: nesnenin katmanı çizimde katman olarak varsa o, yoksa yerleştirmeninki; rengi ve kalınlığı yoksa yerleştirmeninki (çekirdek verir); öznitelikleri ve etiketi kendisinin. Verilen katman var, grup değil, kilitli değil olmalı (`changes[i].layerId`); kalınlık 0 ile 100 mm arasındadır.
- **`cad.blocks.edit` v1:**
  - `rename`: yeniden adlandırma;
  - `redefine`: içindekileri seçilen nesnelerden yeniden kurma; bütün yerleştirmeler güncellenir;
  - `rebase`: taban noktasını taşıma;
  - `remove`: kullanılmayan tanımı silme;
  - `purge`: kullanılmayanların hepsini silme; yalnız kullanılmayanların kullandıkları da gider.
  - Her biri tek geri alma adımıdır: rename, redefine, rebase “Blok değiştir”, remove “Blok sil”, purge “Blokları temizle”. Bir şey değiştirmeyen istek hiçbir şey yazmaz.
- **Hata kodları:** `empty_name`, `no_entities`, `no_layer`, `no_block`, `no_base`, `unknown_block`, `duplicate_block` (Türkçe harf katlamasıyla, ileti ilk tanımın adını verir), `block_cycle`, `block_too_deep`, `block_in_use`; iletiler belgelerin sözleridir (`kentos_contracts::blocks`). Plan tanımı boş UUID kimlikle, yerleştirmeyi yuva 0 ile verir.
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
3. **Komutlar.** `cad.blocks.define`, `insert` yaratma, `cad.blocks.edit`; ortak durumlar. *(29 Eylül: tamam, iki platformda. Sözleşme `cad_blocks.rs`, `EntityGeometry::Insert`, `add`'in kendi alanları; masaüstü `blocks_define.rs`, `blocks_edit.rs`, web `blocksDefine.ts`, `blocksEdit.ts`; ortak durumlar bağımsız Python üreticisiyle (`scripts/fixtures/blocks_command_cases.py`: 23 + 22; `cad.entities.create` ve `cad.entities.edit`'e yerleştirme ve Patlat durumları), üç koşucu (web, masaüstü, Python SDK) geçer; koşucularda `captureBlock`, `$blockOf:`, `$block:`, `blocks`, `blockIds`. Patlat iki platformda yerleştirmeyi açar (`block-explode` izi üç varyantta). Başsız sunucu, MCP (14 çizim komutu) ve Python SDK (`kentos.cad.blocks`, `Document.blocks()`) yeni komutları taşır.)*
4. **Araçlar ve arayüz.** Blok oluştur, Blok ekle, Bloklar paneli, Öznitelikler; `usage-blocks` senaryosu ve resimler. *(29–30 Eylül: tamam, iki platformda. Blok ekle ve Blok oluştur iki platformda, izleri (`block-insert`, `block-define`) üç varyantta. Blok ekle bloğu tıklanan ya da yazılan noktalara yerleştirir; Blok (B), Ölçek (Ö), Dönüş (D) ve Aynala (A) oturumca hatırlanır; hayalet deponun `insert_outlines`'ıdır. Blok oluştur seçimden sonra taban noktasını alır ve adı soran pencereyi açar: ilk boş “Blok n” önerilir (`blocks::free_name`, web'de `freeBlockName`), ad ve açıklama Rust'ın `str::trim`'i gibi kırpılır (web'de `trimName`), `cad.blocks.define`'ın her değişiklikteki cevabı alanların altında durur ve Oluştur ondan sonra açılır. “Seçilenleri blokla değiştir” oturumca hatırlanır; tanımlanan blok Blok ekle'nin sıradaki bloğudur; nesnelerin yerine konan yerleştirme seçilir. İzler pencereyi başlığıyla yanıtlar: `dialog` adımının `fill`, `check` ve `press`'i web'de fare ve klavyeyle, masaüstünde denetimin iletisiyle (`traces/answers.rs`). Bloklar paneli dokun üst yuvasında Katmanlar ve İşlemler'in yanında üçüncü sekmedir (yerleşimin `dockTab`'ı `blocks` alır; dar dokta arkadaki sekmeler iki platformda da kısalır): her blok resmiyle (deponun çizgileri, sığdırılmış), adı, açıklaması ve çizimdeki yerleştirme sayısıyla (`blocks::placements`, iki yanda aynı durum); arama; tıklama bloğu Blok ekle'nin sıradakisi yapar, çift tık ya da Enter yerleştirir, F2 yerinde adlandırır, Delete kullanılmayanı siler; satırın menüsü taban noktasını seçili ya da tek yerleştirmesinde gösterilen noktayla değiştirir (`block::insert_local`), seçimden ve gösterilen taban noktasından yeniden tanımlar, yerleştirmelerini seçer. `block.panel` ve `block.purge` (Blokları temizle) Çizim'in Blok grubunda. Her değişiklik `cad.blocks.edit` ile, iki platformda aynı sözlerle (web `app/blocks.ts`, masaüstü `kentos_interaction::blocks`). Öznitelikler yerleştirmenin bloğunu (▾), konumunu, ölçeğini (sıfırdan büyük), dönüşünü (derece yazılır; `blocks::turn_of` onu 0–360°'ye alıp radyana çevirir, çeyrek dönüşler tam kalır; Blok ekle de aynı kuralla) ve aynalanmasını (▾) `cad.entities.edit`'in özellik işlemiyle düzenler, her biri bir geri alma adımı. `usage-blocks` senaryosu (vana simgesi Blok oluştur ile “Vana” olur, Blok ekle ile iki yere daha yerleşir, Bloklar paneli sayar, biri patlatılıp geri alınır, temizleme, kaydet ve aç) iki platformda üç varyantta geçer ve iki boyutta, iki temada resimlenir.)*
5. **Biçimler ve sunucu.** DXF okuma ve yazma, GeoJSON, PostGIS. *(30 Eylül, 5a: DXF okuma tamam, iki platformda. Okuyucu (`kentos_formats::dxf`) dosyanın adlı, dış başvuru olmayan bloklarını dosyanın sırasıyla tanım yapar: kimlikleri okuyucunun sırası (1, 2, …), nesneleri kendi çerçevelerinde, 0 katmanında, 1, 2, … numaralı; BYLAYER nesne kendi katmanının rengini ve kalınlığını açık değer olarak taşır, tanımın en üstündeki BYBLOCK renk ve kalınlık yoktur (yerleştirmeninkini alır). Tanımında 0 dışındaki katmanda nesne olan blok raporlanır: o katmanların gizliliği ve kilidi uygulanmaz. Yerleştirme konumu, eşit X ve Y ölçeği, dönüşü ve aynalanmasıyla tutulur (X ve Y'nin işaretleri ayrıysa aynalı, X eksiyse dönüşe 180° eklenir); yüksekliği (Z) alınmaz ve söylenir. Adsız bloklar (`*`), MINSERT, eşit olmayan ölçekli ve eğik düzlemdeki yerleştirmeler ve açılan bir bloğun içindekiler bugünkü gibi açılır, nedeniyle raporlanır; kendini içeren yerleştirme alınmaz, başka bloklar üzerinden kurulan döngü kırılır; 16 düzeyden derin iç içe bloklu dosya bloklar patlatılarak yeniden okunur ve söylenir. ATTRIB değerleri yerleştirmenin öznitelikleridir; görünen ATTRIB yazısı 6. adıma dek ayrıca yazı olarak alınır. Yerleştirilmemiş “_” blokları (AutoCAD'in ölçü okları) düşer; kaynak bilgisi “Blok: n tanım (m tanesi yerleştirilmemiş)”. Sözleşmede `ImportResult.blocks` ve `DxfReadOptions.explode_blocks`, `FORMATS_VERSION` 13. İçe aktarma (masaüstü `exchange/apply.rs`, web `io/apply.ts`, `io/drawingImport.ts`) tanımları katmanlarla aynı geri alma adımında, her birini yeni kimlikle ve çizimde olmayan adla ekler (`blocks::import_names`, web'de `importNames`: Türkçe harf katlamasıyla “Kapı” çizimde varken gelen “KAPI” “KAPI (2)” olur), yerleştirmeleri (çizimin ve tanımların içindekileri) yeni kimliklere bağlar. Tanımlar dosyanın nesneleri gibi denetlenir (web `readBlockDefinitions`, masaüstü `unusable_block`); dosyanın getirmediği bloğa yerleştirme reddedilir, hiçbir şey eklenmez. Büyük içe aktarmada tanımlar ilk adımda eklenir; Durdur onları da geri alır. Pencerede DXF için “Blokları patlat” (kapalı; değişince dosya yeniden okunur, “Başka dosya…” seçimi korur), özette alınacak tanım sayısı ve çizimde adı olanların yeni adları, günlükte “n blok tanımı eklendi” ve ad değişiklikleri. Resimler iki platformda, iki temada, iki boyutta: masaüstü `exchange::tests::screens`, web `shots.mjs blocks`.)* *(30 Eylül, 5a'ya ek: DXF'in 0 katmanındaki tanım nesnesi kendi katmanı olmayan (`""`, bloğun) nesne olur; başka katmandaki adıyla gelir ve içe aktarma onu çizimin hedef katmanına, hedef yoksa `""`'ye bağlar: Patlat parçaları AutoCAD'in EXPLODE'u gibi kendi katmanlarına koyar, olmayan bir katmana düşmez. Tanımın içindeki delikler de alanlarına birleşir.)* *(30 Eylül, 5b: DXF yazma tamam, iki platformda. Yazıcı nesnelerin yerleştirdiği blokları ve onların içindekileri BLOCK ve blok kaydı olarak yazar; blok, içerdiği bloklardan sonra gelir, yerleştirilmeyen blok yazılmaz (`dxf/writer/blocks.rs`). Tanımın nesneleri bloğun kaydına aittir; kendi katmanı olmayan 0 katmanında, olan kendi katmanının DXF adındadır; kendi rengi ve kalınlığı olmayan BYBLOCK yazılır (62 0, 370 −2), böylece AutoCAD de onları yerleştirmenin renginde çizer. Blok adları harflerini korur: DXF'in kabul etmediği karakter (“*” dahil) “_” olur, DXF'in büyük/küçük harf ayırmayan karşılaştırmasında başka bir adla çakışan ad “ (2)” alır; her değişiklik söylenir. Yerleştirme INSERT'tir: konumu, X ve Z ölçeği, Y ölçeği (aynalıysa eksi), dönüşü derece olarak; derece radyanı tam geri vermiyorsa KentOS verisi (`turn`) radyanı taşır, okuyucu dosyadaki derece yazdığıyla aynı kaldıkça onu kullanır. Tanımda olmayan bloğun yerleştirmesi, kendini içeren bloğun kendi yerleştirmesi yazılmaz ve söylenir; bloktaki ölçünün değer yazısı yazılmaz ve söylenir. Başlığın kapsamı yerleştirmelerin yerleşmiş kapsamını içerir. Sözleşmede `DxfWriteInput.blocks`, `FORMATS_VERSION` 14; yazıcının JSON okuyucusu yerleştirmeyi ve tanımları okur: web'in DXF ve GeoJSON dışa aktarması yerleştirme içeren katmanlarda “unknown variant insert” ile düşüyordu, artık yazar (GeoJSON 5c'ye dek yerleştirmeyi söyleyip atlar). Bloksuz çizimin DXF'i bayt bayt öncekidir. İki dışa aktarma penceresi yerleştirmelerin ve yerleştirdikleri blokların (`blocks::placed_blocks`, web'de `placedBlocks`) nasıl yazılacağını söyler. Doğrulama: yazıcının testleri (yapı, okuyucuyla gidiş-dönüş, adlar, döngü), masaüstünde içe aktar, dışa aktar ve yeniden oku; örnek `fixtures/formats/v1/dxf-write/blocks` iki platformda aynı baytlara yazılır ve `scripts/fixtures/dxf_write_reference.py` onu KentOS kodu olmadan denetler (ezdxf kurulu değil). Web'in içe ve dışa aktarma tablolarında uzun katman adı sütunları görüş dışına itiyordu; ad satır kaydırır.)* *(30 Eylül, 5c: GeoJSON tamam, iki platformda. Yerleştirme, bloğunun nesneleri yerleşmiş olarak tek bir GeometryCollection'dır, yerleştirmenin öznitelikleri, katmanı ve etiketiyle; iç içe bloklar açılır, açılım ortak çekirdeğin (`kentos_geometry_core::block`: çeyrek dönüşler tam). Nesneler sözleşmeyle çekirdek arasında alan alan geçer (`kentos_formats::blocks`; JSON aracılığıyla değil: biçim modülü +226 KB büyüyordu, böyle +53 KB, 1 390 010 → 1 442 876 bayt). Yerleşmiş parça düzdür, noktanın kotu dışında. Bloğun yazısı, ölçüsü ve sonsuz doğrusu yazılmaz ve söylenir; GeoJSON'a yazılacak nesnesi olmayan ya da tanımı girdide olmayan bloğun yerleştirmesi yazılmaz ve söylenir. Okumada blok yoktur: koleksiyonun üyeleri ayrı nesneler olarak, yerleştirmenin öznitelikleriyle gelir. Sözleşmede `GeoJsonWriteInput.blocks`, `FORMATS_VERSION` 15. İki GeoJSON penceresi yerleştirmenin nasıl yazılacağını söyler. Doğrulama: `fixtures/formats/v1/gis/export/bloklu` (düz, aynalı ve dörtte bir dönüşlü, tam olmayan dönüşlü yerleştirme, iç içe blok, delikli alan, yazılı ve yalnız yazılı blok, tanımı olmayan blok) iki platformda aynı baytlara yazılır; `scripts/fixtures/gis_reference.py` yerleşmiş koordinatları kuraldan kendisi hesaplayıp (çeyrek dönüşler tam, 1 µm hoşgörüyle) denetler; masaüstünde DXF al ve GeoJSON ver. Web'in tablolarında ad ve not sözcük sınırından kaydırır: `anywhere` ad sütununu tek harfe indiriyordu.)*
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
