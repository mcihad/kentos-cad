# ADR 0641: Ağın artımlı kurulması ve olayların yalnız dokunulanlarla yerleşmesi

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-27'nin kalanı: etkilenen bölgenin artımlı kurulması, yalnız
  dokunulan rotaların olaylarının yerleştirilmesi. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0209 §3 ve §12 (graf ve kurma), 0521 (seviyeler, dönüşler), 0562 §6 (artımlı grafın koşulu: tam
  kurmaya eşitliği rastgele düzenlemelerle kanıtlanmış), 0567 (araç profili ve tarih), 0569 (olaylar rotalarını izler), 0640
  (bölge ölçeğinde ölçüm).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Sonraya kalan:**
  - Grafın R-ağacı ve düğümlerin parçaları her yeniden kurmada baştan kurulur. Bölge ölçeğinde yeniden kurmanın en büyük
    payı onlardır (§4).
  - İşlemler, Python ve MCP her soruda grafı baştan kurar; yeniden kurma yalnız masaüstünün ağ iş parçacığındadır.
  - Aynı katmanı bir türde iki kez okuyan ağ (ilk tutan süzgeç alır) ve cihaz yapılandırmasını öznitelikten seçen ağ
    çizimden baştan okunur (§3).

## Bağlam

Masaüstü bir ağı her düzenlemeden sonraki ilk soruda baştan kuruyordu. İki iş yapılıyordu: arayüzün iş parçacığında ağın
katmanlarının bütün nesneleri okunuyordu, ağın iş parçacığında graf baştan kuruluyordu. 1,2 milyon parçalık bölgede (ADR 0640)
okuma yaklaşık 0,7 s, kurma 1,78 s sürüyordu. Bir sokağın ucunu taşıyan kullanıcı bu kadar bekliyordu.

Olay bağları (ADR 0569) da bağın katmanına dokunan her yazmada bağın bütün olaylarını yeniden yerleştiriyordu. 100 000 olaylı
bir bağda bir rotanın taşınması bütün olayları rotalarına yeniden oturtuyordu.

## Karar

### 1. Grafın artımlı kurulması

`Graph::build_kept` grafı `build_with` gibi kurar ve yeniden kurmanın kullandıklarını saklar:

- her kenarın kesimleri, her noktanın düğümü;
- her kenarın okunamayan maliyet alanları;
- kenarların ve düğüm noktalarının özetleri (bütün değerlerinin iki 64 bitlik karması);
- kurulduğu kenarların ilkellerinin ve düğüm noktalarının hücre dizinleri (taban).

`Graph::rebuild` yeni kenarları ve düğüm noktalarını bütünüyle alır ve `build_with`'in vereceği grafı bit bit verir:

1. **Eşleme:** yeni kenarlar ve düğüm noktaları saklananlarla sırayla eşlenir. Eşleme ya özetlerle yapılır (`rebuild`; bir
   farktan sonra iki yanın sonraki 64 öğesine bakılır) ya da ev sahibinin söylediğiyle (`rebuild_known`: masaüstü hangi
   nesneleri yeniden okuduğunu bilir, §3). Eşlenenler saklıdır, ötekiler yenidir, eşlenmeyen saklılar gitmiştir. Sırası
   bozulan eşleme yeni sayılır.
2. **Yeniden kesim:** değişen noktalar, giden ve yeni kenarların uçları ile giden ve yeni düğüm noktalarıdır. Bir kenarı
   yalnız onlar başka türlü kesebilir. Değişen bir noktanın toleransı içindeki saklı kenar, yeni kenarlarla birlikte
   kurmanın kendi kuralıyla yeniden kesilir. Kendisine yakın her uç ve düğüm noktası kurmanın sırasıyla denenir. Öteki saklı
   kenarlar kesimlerini korur; kenarın kendi iç seviyesi (`Level::Own`) yeni numarasını alır.
3. **Düğümler:** giden ya da yeniden kesilen bir noktayı tutan düğüm dağılır. Onun noktaları ve her yeni nokta, toleransı
   içindeki noktalarla yeniden birleştirilir. Öteki düğümler bütün kalır: nokta çiftleri değişmemiştir. Düğümler kurmanın
   sırasıyla yeniden numaralanır (her biri ilk noktasının yerinde). İki bütün düğüm arasındaki seviye aralıkları saklananlardır.
4. **Graf:** kurmanın birleştirdiği gibi birleştirilir. Kesimlerini koruyan kenar parçalarını, ilkellerini ve maliyetlerini
   önceki graftan alır. Maliyetler tanımın verdiği gibidir; bir sorunun araç profili ve tarihi onları değiştirdiyse
   (`Graph::vary`) değiştirilmemiş hâlleri alınır.

Taban dizinleri, geometrisini koruyan kenarlar için geçerli kalır. Tabandan beri değişen kenarlar ve düğüm noktaları her yeniden
kurmada kendi küçük dizinlerine girer. Kurallar değiştiyse, ya da tabandan beri kenarların veya düğüm noktalarının dörtte
birinden çoğu değiştiyse grafı baştan kurar ve yeni bir taban saklar. Cevabın `Rebuilt`'i baştan kurulduğunu ya da kaç kenarın
yeniden kesildiğini ve kaç noktanın yeniden birleştirildiğini söyler.

Yolda tam kurma da hızlandı:

- Noktaların birleştirilmesi hücrelerin ucuz bir karmasıyla ve her hücre çiftine bir kez bakarak yapılıyor (bir hücrenin kendi
  çiftleri ve sonraki dört hücreyle olanlar).
- Kesimler tek bir dizide tutuluyor (kenar başına bir dizi yerine).
- Hücre dizini kutuları toleransın bir kıl ötesine büyütüyor. Bu sayede en yakın nokta denetiminin bulduğu her nokta, dizinin
  hücreleri ne olursa olsun adaylar arasındadır; tam kurma ile yeniden kurma aynı adayları bulur.

### 2. Olaylar yalnız dokunulanlarla

Bir yazma bağın olay katmanına ya da rota katmanlarından birine dokunduğunda yeniden yerleştirilenler:

- yazmanın yazdığı olay nesneleri;
- rota katmanlarındaki bir nesnenin yazmadan önce ya da sonra taşıdığı her rota kimliğinin olayları.

Belgenin `step_objects`'i adımın her yazmasını önceki ve sonraki hâliyle verir. Yazmanın erişmediği bir olayın rotası öncekiyle
aynı nesnedir: yerindeyse yerinde kalır.

Eşdeğerlik şöyledir: her olay yerindeyken ya da yerleştirilemezken, yalnız dokunulanları yerleştirmek bütün olayları
yerleştirmekle aynı taşımaları verir. Yerleştirilemeyen olay ancak kendisine ya da rotasının kimliğine dokunulunca
yerleştirilebilir hâle gelir; ikisi de dokunulan sayılır. Uyarıda (`events_dirty`) yalnız bu yazmanın eriştiği olaylar sayılır.

Bağı tanımlamak olayları taşımaz (ADR 0569); yerinde olmayan eski olaylar Olay kaymasını denetle ile ya da rotalarına dokunan
ilk yazmayla yerine gelir.

### 3. Masaüstünde yalnız değişen nesneler okunur

Ağın iş parçacığı, her ağın son girdisini (kenarlar, düğüm noktaları, dönüş çizgileri) grafıyla birlikte saklar. Bir
düzenlemeden sonraki soruda arayüzün iş parçacığı yalnız belgenin değişiklik günlüğünün adlandırdığı nesneleri okur
(`networks::read::read_changed`):

- ifadeleri yalnız onlar için değerlendirir, yollarını yalnız onlar için depodan alır;
- her türün üyelerinin sırasını (tanımın katman sırası, katmanın belge sırası) ve okunmayanların listesini çıkarır.

Ağın iş parçacığı yeni girdiyi son girdiden ve yeniden okunanlardan kurar (`assemble`). Hangi kenarın ve düğüm noktasının
hangisi olduğunu bildiği için grafı `rebuild_known` ile yeniden kurar.

Bunun dayanağı şudur: ağın ifadeleri yalnız kendi nesnelerine bakar (ADR 0214 §1). Bu yüzden her nesne tek başına okunabilir
ve parça okuma bütün okumayla aynı girdiyi verir. İki tanımda bu varsayım tutmaz; onlar her zaman baştan okunur
(`readable_in_part`):

- aynı katmanı bir türde iki kez listeleyen tanım: ilk tutan süzgeç nesneyi alır;
- cihaz yapılandırmasını öznitelikten seçen tanım: sözü bütün noktaları sayar.

Tanım, katman ağacı ya da bağlı katmanın kaynağı değişince de baştan okunur.

### 4. Süreler

Bu makinede, release, `network::timing` (ızgara şehir; `KENTOS_PERF_GRID=775` bölge):

| İş | Şehir (100 800 parça) | Bölge (1 199 700 parça) |
|---|---|---|
| Tam kurma, önce | 118 ms | 1 778 ms |
| Tam kurma, şimdi | 85 ms | 1 145 ms |
| Kurma, yeniden kurma için saklayarak | 72 ms (15 MB saklı) | 1 172 ms (175 MB saklı) |
| Yeniden kurma, özetlerle: bir sokağın ucu taşındı | 38 ms | 500 ms |
| … bir sokak silindi, biri çizildi | 33 ms | 467 ms |
| … yüz sokağın hızı değişti | 31 ms | 459 ms |
| Yeniden kurma, bildirilen eşlemeyle: bir sokağın ucu taşındı | 34 ms | 440 ms |
| … bir sokağın hızı değişti | 26 ms | 402 ms |
| … bir sokak silindi, biri çizildi | 26 ms | 421 ms |
| … yüz sokağın hızı değişti | 27 ms | 420 ms |

Bölgede yeniden kurmanın 600 ms'si (ilk sürüm) aşamalarıyla ölçüldü: özetler ve eşleme 125, hazırlık 59, yeniden kesim 42, taşıma
ve düğümler ~75, grafın birleştirilmesi 300 (parçalar 165, R-ağacı 90). Kalan payın çoğu baştan kurulan R-ağacı ve düğümlerin
parçalarıdır (Sonraya kalan).

Masaüstünde aynı şehrin çizim olarak okunması (`networks::read_tests::timing`):

| İş | Süre |
|---|---|
| Baştan okuma (arayüzün iş parçacığı) | 78 ms |
| Yalnız değişenin okunması (bir sokak) | 21 ms |
| Ağın iş parçacığında girdinin birleştirilmesi | 20 ms |
| Yeniden kurma, bildirilen eşlemeyle | 30 ms |

Bir düzenlemeden sonraki ilk sorunun hazırlığı 163 ms'den (baştan okuma ve kurma) 71 ms'ye iner. Bunun 21 ms'si arayüzün iş
parçacığındadır (önce 78).

Olay bağlarında yeniden yerleştirilen olay sayısı yazmanın eriştikleriyle sınırlıdır. Bir rotanın taşınması yalnız o rotanın
olaylarını okur ve yerleştirir; rotaların nesnelerinden yalnız istenen kimliklerinkiler rota olarak kurulur.

## Doğrulama

- **Graf:** `rebuild.rs`'in testi üç tanımla yapılıyor: uçlarda kıl kadar tolerans; köşelerde seviyelerle; uçlarda iki metre,
  uzun zincirlerle. Her tanımda sekiz tohum ve tohum başına kırk adım var. Her adımda bir ile otuz beş arasında rastgele
  düzenleme yapılıyor:
  - kenarın ucu bir kavşağa, bir sokağa ya da başka bir ucun tam üstüne taşınır;
  - değerleri değişir; silinir, eklenir; sırası değişir;
  - düğüm noktası eklenir, taşınır, silinir.

  Önceki graf ara sıra bir günün kapanmalarına göre değiştirilir. Tohumların yarısı özetlerle, yarısı bildirilen eşlemeyle
  yeniden kurar. Her adımda yeniden kurulan graf, tam kurmanın grafıyla her alanında aynıdır; R-ağacı aramasıyla
  karşılaştırılır. Rapor da aynıdır, saklanan kesimler ve düğümler de. Test kasıtlı hatalarla denendi; üçünü de yakaladı:
  dağılan düğümler unutulunca, iç seviye yeniden numaralanmayınca, değiştirilmiş maliyetler kopyalanınca.
- **Olaylar:** `event_follow.rs`'in testi 40 tohumda 25'er rastgele yazma yapıyor: rota ya da olay taşınır, rota kimliği,
  ölçü, başlangıç ya da öteleme değişir, olay ya da rota çizilir, biri silinir. Her yazmada yalnız dokunulanları yerleştirmek,
  bütün olayları yerleştirmekle aynı taşımaları verir. Rotanın yazmadan önceki kimliği sayılmayınca test düşer.
- **Ortak durumlar:** `fixtures/commands/v1/cad.linearref.follow.json`'da 27 durum var; `linearref_follow_cases.py` yeni
  kuralla, KentOS kodu olmadan üretir. Yeni ve değişenler:
  - olaya dokunan yazma aynı rotanın kaymış öbür olayını yerinde bırakır;
  - ilgisiz bir özniteliğiyle rotaya dokunan yazma o rotanın bütün olaylarını yerine koyar;
  - rotaya dokunan yazma öbür rotanın kaymış olayına bakmaz;
  - rota bekleyen bir kimliğe dönünce o kimliğin olayı rotaya oturur, eski kimliğinkiler söylenir;
  - bağı tanımladıktan sonra rotasına dokunan yazma kaymış olayı taşır.
- **Masaüstü:** `networks::read_tests` ağ izlerinin çiziminde iki ağla, tohum başına 20 adımla çalışır. Her adımda nesneler
  taşınır (bütünü ya da bir ucu), öznitelikleri değişir, kopyalanır, silinir ya da başka katmana taşınır. Her adımda:
  - parça okuma ve birleştirme, baştan okumayla aynı girdiyi, üyeleri, alınmayanları ve sözleri verir;
  - bildirilen eşlemeyle yeniden kurulan graf, tam kurmayla aynıdır.

  Ağ araçlarının testleri değişmeden geçer.
