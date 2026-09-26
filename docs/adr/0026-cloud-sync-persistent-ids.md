# ADR 0026: Bulut eşitlemesinde kalıcı nesne kimliği (ADR 0014 dilim 3)

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0014'ten gelir (dilim 3). Silinip yeniden oluşturulan kimliğin sürümü, cihaz taslağının yeni biçimi ve eski taslakların göçü, `exists` çakışmasının anlamı, dışarıdan gelen nesnenin kimliği ve geri alma geçmişi bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.8, §13, §16, §21, §23; TODOS.md `DOM-03`, `DOM-04`, `SYNC-07`, `SYNC-08`, `SYNC-12`; ADR 0006, 0013, 0014, 0015, 0024

## Bağlam

- Dilim 1'den beri çizimdeki her nesnenin kalıcı `uid`'i var. Bulut izleyicisi (`app/cloud/tracker.ts`) ise nesneyi ilk gönderişinde kendi ürettiği bir v4 kimlikle gönderiyordu. Nesnenin iki kimliği vardı; eşleme yalnız oturumun belleğinde ve cihaz taslağında yaşıyordu. Aynı çizim iki kez yüklenince her nesne başka kimlik alıyordu.
- Sunucuda (migration 0001) `kentos.feature`'ın birincil anahtarı `(tenant_id, project_id, id)`'dir; kimliği istemci seçer; `project.changes` beklenen sürüm ve idempotency anahtarıyla çalışır (ADR 0006, 0013). Silme satırı kaldırır. Yeni satırın sürümü 1'den başlıyordu.
- Kimlik kalıcı olunca aynı kimlik silindikten sonra yeniden oluşturulur: geri alınan silme, başkasının silmesine karşı “Benimkini kaydet”. Sürüm 1'den yeniden başlarsa bir sürüm aynı kimliğe iki kez verilir:
  - Ayşe nesneyi 2. sürümde siler, geri alır (1. sürüm), bir kez düzenler (2. sürüm).
  - Bora 2. sürümü görmüş, çevrimdışı düzenlemiş; düzenlemesi cihaz taslağında “taban 2” diye bekliyor.
  - Bora projeyi yeniden açınca taban sunucunun sürümüne eşit görünür; düzenlemesi Ayşe'ninkinin üstüne çakışmasız yazılır.

  Bu CLAUDE.md §21.3'ü (başkasının güncel işi sessizce ezilmez) çiğner. Eski izleyici de geri alınan silmeyi aynı kimlikle yeniden oluşturuyordu; kalıcı kimlikle bu yol her geri almada kullanılır.

## Karar

### Tek kimlik

- Nesnenin `uid`'i sunucudaki `feature.id`'dir. İzleyicinin eşlemesi ve v4 üretimi kalktı.
  - İzleyici nesneyi kalıcı kimliğiyle tanır: sunucudaki sürümü ve o sürümdeki metni. Sunucuda olmayan nesne izlenmez; geri alınıp gelen nesne aynı kimlikle yeniden oluşturulur.
  - Gönderilmemiş değişiklikler (`dirty`), çakışmalar ve cihaz taslağı da kalıcı kimlikle anahtarlıdır. Yuva yalnız belgeye nesne koyarken ya da çıkarırken sorulur (`slotOf`).
  - Belgenin `touched` olayı dokunduğu nesnelerin kalıcı kimliklerini de verir (`uids`, `ids` ile aynı sırada): silinen nesnenin kimliği belgeden artık sorulamaz.
- **Açılış:** `readProject` (`app/cloud/incoming.ts`) her nesneye sunucunun kimliğini `uid` olarak verir. v1 okuyucusu dosyadaki `uid`'i attığı için kimlikler nesnenin yanında taşınır. Kimlik küçük harfli, tireli UUID değilse proje açılmaz.
- **Başka editörün nesnesi, taslaktan gelen nesne:** `CadDocument.applyExternal` yeni nesneye getirdiği kimliği verir (kimliği yoksa yeni v7). Çizimdeki nesne kendi kimliğini korur. Reddedilir (hiçbir şey değişmeden): UUID olmayan kimlik, aynı değişiklikte iki kez gelen kimlik, çizimde başka nesnenin kimliği, bir nesnenin kimliğini değiştiren değer.
- **Gönderim:** değişikliğin kimliği (`FeatureChange.id`) nesnenin `uid`'idir. Nesnenin gövdesi, sözleşmenin `Entity`'si, kimlik taşımaz (sözleşmede `EntityId` dilim 4'tür). `entityJson` yuvayı ve kimliği karşılaştırmaya almaz.

### Silinen kimliğin dönüşü ve sürümler (sunucu)

- Silme satırı kaldırmaya devam eder. Aynı kimliğin yeniden oluşturulması kabul edilir: geri alma ve “Benimkini kaydet” bunu ister.
- **Oluşturulan nesnenin sürümü, commit'in veri revizyonudur** (`project.data_revision`, commit'le birlikte artan). Değişiklik sürüme yine 1 ekler.
- **Neden yeter:** bir kimliğin aldığı her sürüm, onu yazan commit'in veri revizyonundan büyük değildir. Oluşturma revizyona eşittir; her değişiklik ayrı bir commit'tir ve revizyon her commit'te en az 1 artar. Yeniden oluşturulan kimlik, önceki sürümlerinin hepsinin üstünden başlar. Silinmeden önceki bir sürüme dayanan değişiklik ya da silme `changed` çakışmasıdır; sunucu güncel kopyayı döner.
- Proje satırı commit boyunca kilitli olduğundan okunan revizyon commit'in revizyonudur (`debug_assert`).
- **Şema ve migration değişmedi.** Migration 0005 gerekmedi: sütunlar aynı, yalnız oluşturmanın yazdığı sayı değişti. Mevcut satırlar kuralı zaten sağlar (1'den başlayan sürümler revizyonun altında kalır).
- **Görünen etki:** yeni projenin ilk nesneleri yine 1 alır; sonra oluşturulanlar daha büyük sayılar alır. Sürüm opak ondalık metindir (ADR 0006). Web sürümü yanıttan okur; yüklemenin “her nesne 1. sürümde” varsayımı kalktı.
- **Seçilmeyen yol:** silinen kimlikler tablosu (tombstone; kimliğin son sürümü, yeniden oluşturmada +1). Migration 0005, yeni satır güvenliği kuralı ve yetkiler isterdi. Tablo her silinen kimlik için sonsuza dek büyürdü; budanırsa sorun geri gelirdi. Veri revizyonu aynı güvenceyi ek depo olmadan verir.

### Çakışmalar ve geri alma

- **`exists`** artık “sunucuda bu kimlikte nesne zaten var” demektir: aynı nesne, başkası önce geri getirmiş.
  - “Benimkini kaydet” benimkini sunucunun sürümü üzerine değişiklik olarak gönderir. Eskiden nesneye yeni bir kimlik verip ikinci bir nesne yaratıyordu.
  - “Sunucudakini al” nesneyi yuvasında sunucunun hâline getirir.
  - Pencere satırı “kimliği başkasında” yerine “sunucuda zaten var” der ve nesneyi kalıcı kimliğiyle bulur (`SyncConflict.localId` kalktı).
- **Başkasının geri getirdiği nesne ve geri alma:** benim sildiğim nesneyi başkası geri getirirse nesne çizime yeni bir yuvada, aynı kimlikle girer. Benim geri alma ve yineleme adımlarım o nesneyi eski yuvasına koyar ve çizimde bir kimlik iki nesnede olurdu. `applyExternal` artık geçmişi yuvayla birlikte **kalıcı kimliğe** göre de temizler (`forgetHistoryOf(slots, uids)`).
- **İki tarafın da sildiği nesne** (taslakta silme, sunucuda yok) iş bitmiş sayılır; çakışma değildir.
- Sunucudan gelen kopyalar (başka editörün olayları, yoldaki komutun yanıtı, “Sunucudakini al”) açık düzenleme bitince hesaplanır ve hemen uygulanır; aradaki bir düzenleme hesabın dışında kalamaz.

### Cihaz taslağı (IndexedDB `kentos.cloud/drafts`)

- **Biçim 2:** `version: 2`; `changes` nesnenin kalıcı kimliğiyle anahtarlı; nesne gövdesi kimliksiz.
- **Eski taslaklar** (`version` yok) aynı şekildedir. Anahtar, nesnenin sunucudaki kimliği ya da eski izleyicinin yeni nesneye verdiği v4'tür. Yoldaki komut (`inflight`) da aynı v4'le gönderilmişti.
  - `readDraft` (`app/cloud/drafts.ts`) bu anahtarları nesnelerin kalıcı kimliği yapar.
  - Yoldaki komut kendi anahtarıyla yeniden gider: işlenmişse sunucu günlükten yanıtlar, işlenmemişse şimdi işlenir. Sonraki değişiklikler aynı nesneleri adlandırır. Kayıp yok, çoğaltma yok.
  - Eski taslak bir kez okunur; sonraki kayıt biçim 2 yazar. Biçim 2'yi eski kod da okuyabilir (şekil aynı).
  - Anahtar küçük harfe çevrilir. UUID olmayan anahtar yeni kimlikle yeni nesne olur. Hiçbir sürüm böyle anahtar yazmadı; sunucu da kabul etmezdi.
- **Tam okunamayan ya da bu hesabın olmayan taslak** önce olduğu gibi `<anahtar>#unreadable-<zaman>` altında ayrıca saklanır. Sonra okunabilen kısmı geri gelir ve kullanıcıya söylenir. Kullanıcı verisi yeni bir kayıtla ezilmez (CLAUDE.md §2).
- **Çizime konamayan değişiklik** (nesnenin katmanı artık yok) gönderilmez ama taslakta kalır ve kullanıcıya söylenir (`SyncCore.held`). Önceden taslaktan düşüyordu. Aynı nesne burada düzenlenirse yeni düzenleme onun yerini alır.

### Yükleme

- Her nesne kendi `uid`'iyle gider; sürümü yanıttan alınır (`app/cloud/upload.ts`).
- Proje oluşturma, her parti ve kilitlerin geri yüklenmesi, sunucu yanıt vermedikçe **aynı idempotency anahtarıyla** yeniden denenir (0,5 / 1 / 2 / 4 / 8 s, sonra hata). Yanıtı kaybolan adım günlükten yanıtlanır.
- **Aynı dosya iki kez yüklenirse** iki proje olur, nesneleri aynı kimliklerle; anahtar proje başınadır. v1 dosyasının kimlikleri içerikten türer (ADR 0014 dilim 2). Web'de çizilen nesneler de aynı çizim iki kez yüklenince aynı kimlikleri taşır.
- İçe aktarma (DXF, NCN …) yeni nesnedir: açık bulut projesinde de yeni kimlik alır; okuyucunun getirdiği bir `uid` alınmaz.

## Sonuçlar

- **Testler (web, Vitest):**
  - `app/cloud/syncIdentity.test.ts`: bir kimlik iki yönde; kaybolan yanıttan sonra tek yazım; geri alınan silme eski sürümlerin üstünde; silme ve dönüşten önceki sürüme dayanan düzenleme çakışma; başkasının geri getirdiği nesne geri almayla ikinci kez gelmez; `exists` iki seçimle; açık projeye içe aktarma kimlik çoğaltmaz.
  - `app/cloud/drafts.test.ts`: eski koda birebir uyan taslaklar (yoldaki komut işlenmiş ya da hiç ulaşmamış), bekleyen eski taslak, kalan kısmın biçim 2'de yazılması, `readDraft`, ayrıca saklama, çizime konamayan değişiklik.
  - `app/cloud/upload.test.ts`: aynı çizim iki projeye aynı kimliklerle; kayıp yanıtta aynı parti bir kez.
  - `model/external.test.ts`: `applyExternal` kuralları; `touched` kalıcı kimlikleri; geçmişin kimliğe göre temizlenmesi. `model/identity.test.ts`'in 2000 adımlı rastgele testi, silinmiş kimlikleri başka editörden geri getirir.
  - Sahte sunucu (`fakeServer.ts`) sunucunun kuralını izler: oluşturma sürümü veri revizyonudur. Aynı anahtar başka istekle gelirse reddeder.
- **Testler (sunucu, PostGIS, `KENTOS_TEST_DB=required`):** `crates/server/application/tests/persistent_ids.rs`:
  - iki kez silinip geri getirilen kimlik: sürümler 1, 2, 4, 5, 7; eski sürüme dayanan değişiklik ve silme güncel kopyayla reddedilir;
  - aynı v1 dosyasının türetilmiş kimlikleri aynı kurumun iki projesinde; her biri ötekinden bağımsız değişir ve silinir.
- **Testler (e2e, `pnpm e2e:cloud`, gerçek `kentosd` ve `kentos_cad`):**
  - çizginin sunucudaki kimliği `uid`'idir;
  - yanıtı düşürülen komut bir kez yazılır;
  - geri alınan silme daha yüksek sürümle döner, eski sürümle düzenleme 409;
  - iki kişinin geri getirdiği nesne tek kalır: “sunucuda zaten var”, koyu, açık ve Büyük yazı;
  - `fixtures/document/v1/identity/sample.compact.kcad` iki kez açılıp yüklenir; iki projede bağımsız Python referansının kimlikleri vardır.
- **Kasıtlı bozma denemesi:** yeniden gönderilen komuta yeni kimlik ve yeni anahtar verildi. `sync.test.ts` (“never commits twice”) ve `syncIdentity.test.ts` (“writes once, under the objects’ own ids”) düştü: iki yazım, iki kat nesne. Değişiklik geri alındı.
- **Sıcak yol:** değişmedi. Kimlik yalnız düzenleme olayında (işaretçi hareketinde değil) diziye eklenir; çizim, seçme ve kenet kimlik taşımaz.

## Sınırlar ve kalanlar

- Sözleşmenin `Entity`'si ve `.kcad` v1 kimlik taşımaz: dilim 4 (`EntityId`, binary v2).
- Masaüstünde bulut eşitlemesi yok.
- Çizime konamayan değişikliği başka katmana taşıyacak ya da bırakacak bir arayüz yok. Değişiklik cihazda kalır, her açılışta yeniden denenir ve söylenir.
- Ayrıca saklanan (`#unreadable-`) taslakları gösteren ya da geri yükleyen bir arayüz yok; kaybolmazlar, destek için durur.
- Aynı projeyi iki sekmede açmak aynı taslak anahtarını yazar; bu dilimden önce de öyleydi.
- Veritabanı geri yüklenirse (PITR) revizyonlar geri gider ve bir sürüm yeniden verilebilir. Geri yüklemeden sonra istemciler projeyi yeniden açar (ADR 0006); ayrıca ele alınmadı.
