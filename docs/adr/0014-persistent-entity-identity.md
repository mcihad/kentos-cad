# ADR 0014: Kalıcı nesne kimliği, çalışma yuvası ve eski dosya göçü

- **Durum:** kabul edildi (yön, 2026-09-25). Uygulama dilimleri aşağıdadır. Dilim 1 (web belgesi) ve dilim 2 (v1 göçü) 25 Eylül'de uygulandı; sonda uygulama notu vardır. Dilim 3 ve 4 açıktır.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md `DOM-03`, `DOM-04`, `DOM-05`, `FILE-05`, `SYNC-06/07`, `PY-11`, `AI-06`; ADR 0002, 0003, 0011, 0012

## Bağlam

Bugün bir nesnenin iki ayrı ve birbirinden habersiz kimliği var (koddan doğrulandı, 25 Eylül):

- **Belgede yerel `u32`:**
  - `CadDocument` yeni nesneye `nextId++` verir (`apps/web/src/model/document.ts`).
  - `.kcad` v1 bu sayıyı yazar (`EntityBase.id`: “the document's local id in v1”).
  - Sayı yalnız o belgenin içinde anlamlıdır. Dosya açılınca yeniden kurulur; iki dosyanın `id: 7`'si ayrı nesnelerdir.
- **Bulutta UUID:**
  - Tarayıcının eşitleme izleyicisi (`app/cloud/tracker.ts`) bir nesneyi ilk gönderişinde ona rastgele bir v4 UUID verir (`crypto.randomUUID()`).
  - Eşleme (yerel → UUID) yalnız o oturumun belleğinde ve cihaz taslağında yaşar.
  - Sunucu kimliği istemcinin seçmesini bilerek ister, çünkü aynı istek tekrarlanınca nesne iki kez oluşmamalıdır (`FeatureChange::Create`).

Sonuçları:

- Aynı `.kcad` dosyası buluta iki kez yüklenirse her nesne iki kez oluşur.
- Python, AI ya da başka bir istemci bir nesneyi yerel dosyada ve bulutta aynı adla anamaz.
- Kopyala/yapıştır ya da geri al sonrasında hangi nesnenin “aynı” olduğu yalnız izleyicinin o anki belleğine bağlıdır.

Sunucu kullanıcı, kurum ve proje kimliklerini zaten UUIDv7 ile üretir (`Uuid::now_v7`).

## Karar

### Üç ayrı kimlik

| Kimlik | Tür | Ömrü ve yeri | Kim kullanır |
|---|---|---|---|
| **Kalıcı nesne kimliği** | UUID; yeni nesnede v7 | Nesne oluşunca verilir, hiç değişmez ve yeniden kullanılmaz. Dosyada (v2), bulutta (`feature.id`), PostGIS'te, Python'da ve AI sonuçlarında aynıdır | Dışarıya açılan her yüz: dosya, API, komut girdisi ve çıktısı, `FeatureRef` |
| **Çalışma yuvası** | `u32` | Açık belgenin ömrü kadar; kalıcı değil | Sıcak yollar: seçme ve kenet deposu, GPU tamponları, seçim kümeleri, geri alma kayıtları |
| **Eski yerel kimlik** | `u32` | Yalnız bir v1 dosyasının içinde anlamlı | Yalnız v1 okuyucusu, göç sırasında |

- Web belgesi bugünkü `Entity.id: number`'ı çalışma yuvası olarak korur. Yanına kalıcı kimliği (`uid`) ekler.
  - Araçlar, depo ve çizim hattı yuvayla çalışmayı sürdürür. Sıcak yolda nesne başına UUID metni taşınmaz (CLAUDE.md §6.2).
  - Dosya, API ve komut sınırında yuva ↔ kalıcı kimlik açık bir dönüştürücüyle eşlenir (`DOM-05`). Sıcak yol tipi alan modelinin tamamıyla zorla birleştirilmez.
- **v7 seçildi:**
  - Oluşma zamanıyla sıralıdır; PostgreSQL B-ağacında rastgele v4'ten daha az dağınıklık yaratır.
  - İstemci üretir, 74 rastgele biti çakışmayı pratikte dışlar.
  - Bugüne kadar üretilmiş v4 kimlikler geçerli kalır; hiçbir kural UUID sürümüne dayanmaz.
- **Gösterim:**
  - API ve JSON'da küçük harfli, tireli metin; PostgreSQL'de `uuid`.
  - Binary `.kcad`'de 16 bayt (FILE-05). CBOR seçilirse UUID etiketi 37 önerilir; ADR 0011'in bayt spesifikasyonu kesinleştirir.

### Kimlik kuralları

- Nesneyi **oluşturan istemci** kimliği verir: web, masaüstü, sunucu işi, Python. Tekrarlanan istek aynı kimlikle gelir, sunucu ikinci kez oluşturmaz (bugünkü idempotency kuralı).
- **Değiştirmek** kimliği korur: taşı, döndür, özellik ya da köşe düzenleme, budamada kalan parça.
- **Yeni nesne yeni kimlik alır:** kopyala/yapıştır, çoğalt, dizi, patlat, böl ya da kır işlemlerinin ek parçaları, içe aktarma.
  - Hangi sonucun “değişen”, hangisinin “yeni” olduğunu işlemin `ChangeSet`'i söyler (`DOM-07`).
  - Kesip yapıştırmak da yeni nesnedir: kesme silmedir, yapıştırma oluşturmadır.
- **Geri alma** silinen nesneyi aynı kimlikle geri getirir; yineleme aynı kimliği yeniden siler ya da oluşturur (ADR 0003).
- **Katman kimlikleri** bugünkü gibi proje içinde benzersiz metinlerdir (`LayerNode.id`, PostGIS `layer_id`). Ad değişince kimlik değişmez.
- **Dış sağlayıcı satırları** (PostGIS tablosu) kendi birincil anahtarlarıyla tanınır: `(kaynak bağlantısı, anahtar)`. KentOS onlara UUID uydurmaz (`PG-10`).
- **Python ve AI başvurusu:** `FeatureRef = { projectId, entityId }`, gerektiğinde revizyonla (`PY-11`, `AI-06`).

### Eski v1 dosyalarının göçü (`DOM-04`)

v1 dosyasında kalıcı kimlik yoktur. Göç **belirlenimlidir**: aynı dosya kaç kez açılırsa açılsın, kaç kez yüklenirse yüklensin aynı kimlikleri verir.

1. Dosya `DocumentSnapshotV1` olarak okunur ve kanonik biçimde yeniden serileştirilir (serde_json, sabit alan sırası). Boşluk, satır sonu ya da alan sırası farkı sonucu değiştirmez.
2. `ad_alanı = UUIDv5(KENTOS_V1_IMPORT, sha256(kanonik metin))`.
3. Her nesne için `kimlik = UUIDv5(ad_alanı, "entity/" + yerel id)`.
4. Dosyanın proje kimliği gerekirse `UUIDv5(ad_alanı, "project")`.

- `KENTOS_V1_IMPORT` sözleşmede sabitlenen bir UUID'dir. Değişmesi bütün eski dosyaların kimliğini değiştirir, bu yüzden hiç değişmez.
- v5 (SHA-1) burada güvenlik değil yalnız belirlenimlilik içindir.
- **Sonuç:** aynı eski dosya iki kez buluta yüklenirse sunucu aynı kimlikleri görür ve nesneleri çoğaltmaz. İçeriği değişmiş bir v1 dosyası (eski bir istemcinin yeniden kaydettiği) başka bir anlık görüntüdür ve yeni kimlikler alır.
- Göç sonrasında dosya v2 olarak kaydedilir. Kimliklerle birlikte göçün kaynağı da yazılır: biçim ve sürüm, kaynağın sha256'sı (`FILE-05`, `FILE-21`). Özgün v1 dosyası ezilmez.

## Sonuçlar

- **Uygulama dilimleri:**
  1. **Web belgesi:**
     - Her nesneye `uid` (v7) verilir. Geri alma ve yinelemede korunur, kopyada yenilenir.
     - Belge içi `uid` → yuva dizini tutulur.
     - Testler: geri alma aynı kimliği döndürür; kopya yeni kimlik alır; çoklu düzenleme tek adımdır.
  2. **v1 okuyucusu:** belirlenimli göç. Aynı dosyaya aynı kimlikleri doğrulayan bir fixture ve bağımsız bir referans (Python'da UUIDv5 hesabı).
  3. **Bulut eşitlemesi:**
     - İzleyici nesnenin `uid`'ini sunucu kimliği olarak kullanır; ayrı eşleme ve v4 üretimi kalkar.
     - Aynı dosyanın iki kez yüklenmesi ve ACK kaybı testleri çoğaltma üretmemelidir.
  4. **Sözleşme ve binary `.kcad` v2:**
     - `EntityId` tipi.
     - Dosya ve API'de kalıcı kimlik; yuva dışarı çıkmaz (`FILE-04/05`).
- **Sıcak yol maliyeti:**
  - Kimlik yalnız oluşturmada üretilir.
  - Depo ve GPU yuvayla çalışır.
  - Dönüştürücü sınırda tek geçiştir. Ölçüm `pnpm perf:interaction` ile aynı kalite düzeyinde alınır.
- **Değişmeyenler:**
  - Sunucunun `feature.id uuid` şeması;
  - istemcinin kimlik seçmesi kuralı;
  - bugünkü v1 dosyalarının okunması.

## Uygulama notu: dilim 1 ve 2 (25 Eylül 2026)

### Dilim 1: web belgesi

- `Entity.id` çalışma yuvası olarak kaldı. Yanına `uid` geldi: küçük harfli, tireli UUID (`apps/web/src/model/entities.ts`).
  - Çizimdeki her nesnenin kimliği vardır (`DrawingEntity`). Çizim dışındaki nesnenin yoktur: önizleme, pano kopyası, dosyadan okunmuş ama çizime girmemiş nesne. Bu yüzden genel `Entity` tipinde alan isteğe bağlıdır.
- Yeni nesne UUIDv7 alır (`apps/web/src/core/uuid.ts`):
  - RFC 9562 düzeni; 62 bit `crypto.getRandomValues`'tan gelir.
  - Aynı milisaniyede 12 bitlik sayaç sayar (RFC 9562 §6.2, yöntem 1). Sayfanın ürettiği kimlikler oluşma sırasıyla sıralanır; saat geri gitse de sıra bozulmaz.
- `CadDocument` `uid → yuva` dizinini tutar: `byUid`, `slotOf`, `uidOf`.
  - Ekleme, silme, değiştirme, geri alma, yineleme, işlemin geri alınması, dışarıdan gelen değişiklik ve `replaceWith` dizini günceller.
  - 2000 adımlı rastgele test dizinin tutarlılığını denetler (`model/identity.test.ts`).
- **Kurallar:**
  - `add`, `addMany`, `load` her zaman yeni kimlik verir. Başka bir nesneden yayılan kopya da (`{ ...e }`) yeni kimlik alır. Pano kimlik tutmaz; kesip yapıştırmak da yeni nesnedir.
  - `update` ve `updateMany` kimliği değiştiremez.
  - Yeni `replace(id, nesne)`: nesnenin içeriğini, türünü de değiştirir; yuva ve kimlik kalır.
  - Buda, Kır ve türü değiştiren köşe ekleme: ilk parça nesnenin kendisidir (`replace`), öbür parçalar yeni nesnedir. Alan böl'de ilk parça alanın kendisidir. Önceden bütün parçalar yeni nesneydi; e2e denetimleri buna göre değişti.
  - Birleştir, Patlat, Alan birleştir, Alan kesiştir, Alan çıkar, Alana çevir ve Çizgiye çevir bugünkü gibi kaynağı silip yeni nesne ekler. Bu karar bu işlemleri adlandırmıyor (açık soru, aşağıda).
  - Geri alma silinen nesneyi aynı yuva ve kimlikle getirir.
- v1 dosyası kimlik yazmaz (`toSnapshot`). Okuyucu dosyada bulduğu `uid`'i almaz.
- Bulut izleyicisi nesneyi kimliksiz karşılaştırır ve gönderir. Dilim 3'e kadar sunucunun `feature.id`'si ayrı eşlemede kalır.
- **Sıcak yol:** geometri deposu nesneyi kendi alanlarından paketler (`wasm/pack.ts`). Çizim, seçme ve kenet kimlik taşımaz. Maliyet yalnız oluşturmadadır: 200 000 nesnelik `replaceWith`'te kimlik üretimi ve dizin yaklaşık 0,1–0,2 s sürer (Node, geliştirme makinesi).
- Ortak işlem fixture'ları (ADR 0020) kimliği göreli denetler: `captureUid` ve `uids` beklentisi, `fixtures/document-ops/v1/identity.json`. Web ve masaüstü koşucusu geçer.

### Dilim 2: v1 dosyalarının belirlenimli göçü

- Algoritma `crates/shared/contracts/src/identity.rs`'tedir (`v1_uids`, `v1_identities`):
  1. Dosya `DocumentSnapshotV1` olarak okunur ve kanonik yazılır: sözleşmenin alan sırası, boşluksuz, float64'lerin en kısa yazımı, opak kısımların (stil öğeleri, katman çizicisi) anahtarları sıralı. Sözleşmenin bilmediği alanlar yazılmaz.
  2. `ad_alanı = UUIDv5(KENTOS_V1_IMPORT, sha256'nın 64 küçük onaltılık hanesi)`.
  3. Nesne `UUIDv5(ad_alanı, "entity/" + yerel id)`, proje `UUIDv5(ad_alanı, "project")`.
- `KENTOS_V1_IMPORT = 4a5259a1-97f7-4742-88ce-b747287025ed`: bir kez çekilmiş rastgele bir v4 UUID. **Hiç değişmez.**
- UUIDv5 kilitli `sha1` 0.10.7 ile, sha256 `sha2` 0.10.9 ile hesaplanır. İkisi de `Cargo.lock`'taydı (axum, sqlx); yeni paket gelmedi. `uuid` crate'inin `v5` özelliği ise `sha1_smol`'u getirirdi.
- **Alan sırası metnin parçasıdır.** Sözleşmede bir alanın yerini değiştirmek bütün v1 kimliklerini değiştirir; fixture testi düşer.
- `DocumentSnapshotV1::from_json` artık önce yalnız biçimi ve sürümü okur, çizimi bir kez doğrudan tipli okur (`serde_json::Value` ağacı kurmaz). Baştaki BOM'u atlar, tarayıcının dosyayı çözerken yaptığı gibi.
- **Bağımsız referans:** `scripts/fixtures/v1_identity_reference.py`, yalnız Python standart kütüphanesi (`json`, `hashlib`, `uuid`, `decimal`). Kanonik metni kendi alan tablosuyla yazar, sha256'yı ve UUIDv5'i hesaplar. Sonuçlar `fixtures/document/v1/identity/`'dedir:
  - Web'in kaydettiği örnek (`../sample.json`), aynı çizimin web'in yazdığı sıkışık hâli (`sample.compact.kcad`) ve başka biçimde yazılmışı (`sample.variant.kcad`: CRLF, sekme, ters alan sırası, 17 haneli üslü sayılar) aynı kimlikleri verir.
  - Bir milimetre oynatılmış çizim (`edited.kcad`) bambaşka kimlikler alır.
  - Elle yazılmış küçük çizim (`minimal.kcad`): eski ayarlar, bilinmeyen alanlar, sayı sınırları (1e-5 / 1e-6, 1e15 / 1e16, -0.0), kaçışlı metin, sıralı olmayan yerel kimlikler.
- **Doğrulama:** Rust'ın kanonik metni referansla bayt bayt aynıdır ve kimlikler aynıdır (`crates/shared/contracts/tests/identity.rs`). Tarayıcı modülü aynı kimlikleri verir (`apps/web/src/io/identity.wasm.test.ts`), masaüstü belgesi de (`crates/native/domain/tests/identity.rs`, `v1_entity_uids`, ADR 0020).
- **Web açılışı** (`app/fileIO.ts`):
  - Kimlikleri dosya biçimi worker'ı hesaplar (`v1Identities`, `FORMATS_VERSION` 4), sayfa dosyayı okurken. `attachV1Identities` onları nesnelere yerel kimlikle eşler, sonra `replaceWith`.
  - Kanonik metin TypeScript'te yeniden yazılmadı: `JSON.stringify` sayıları serde_json'dan farklı yazar (`486512` / `486512.0`).
  - **Neden biçim modülü:** sözleşmenin tipli okuyucusu ve sha1/sha2 biçim modülünü 676 099'dan 920 681 bayta büyüttü (+245 KB; brotli 203 548 → 251 393, +48 KB). Aynı kod geometri çekirdeğine konsa, başlangıçta yüklenen modül 1 169 816'dan 1 491 992 bayta çıkardı (+322 KB; brotli 312 504 → 388 605, +76 KB), her açılışta. Biçim modülü yalnız dosya açılınca, içe ya da dışa aktarılınca yüklenir, worker'da çalışır ve sayfayı bekletmez. İstemcisi de içe ve dışa aktarma pencereleri gibi ilk kullanımda yüklenir.
    - Masaüstü belgesi için işlev paylaşılan (değiştirilemez) çizimi aldı. Opak anahtarlar dosya sırasında kalırsa (serde_json `preserve_order`, hiçbir derlemede açık değil) çizimin kopyasını sıralar; bu kopyalama kodu modülü 934 257 bayta getirdi (brotli 254 977). Son durum: +258 KB, brotli +51 KB.
  - **Süre:** 100 000 nesnelik 26,9 MB çizimde sayfanın okuması ~190 ms, kimlik hesabı ~440 ms sürdü (Node'da aynı modülle). Tarayıcıda ikisi paralel yürür.
  - Kimlik türetilemezse (modül yüklenmedi ya da sözleşme sayfanın kabul ettiği bir dosyayı reddetti) çizim yine açılır. Nesneler bu açılış için yeni kimlik alır, günlük bunu söyler.
  - `DocumentFiles.load` artık `Promise` döndürür; kimlikleri bekler.

### Sınırlar ve kalanlar

- **v1 kimlik yazmaz.** Yeniden açılan dosya yalnız içeriği değişmediyse aynı kimlikleri alır. Düzenlenip v1 olarak kaydedilen dosya başka bir anlık görüntüdür: bütün nesneler, düzenlenmeyenler de, yeni kimlik alır. Proje adı da içeriktir. Kalıcılık binary v2 ile gelir (`FILE-05`, `FILE-21`); v2 göçün kaynağını da yazar (biçim, sürüm, `sourceSha256`).
- Projenin türetilen kimliği (`project`) hesaplanır, ama web belgesi henüz tutmuyor.
- **Açık soru:** Birleştir, Alan birleştir (tevhit), Alan çıkar, Alana çevir ve Çizgiye çevir'de sonuçlardan biri kaynağın kendisi sayılsın mı? Bu işlemler verisini ilk kaynaktan taşır (tevhit “kalan parsel” der), ama kimlik bugün yenidir. Karar verilince `replace` ile değişir.
- **Dilim 3 (bulut) için gerekenler:**
  - İzleyici nesnenin `uid`'ini `featureId` yapar; ayrı eşleme ve v4 üretimi kalkar (`app/cloud/tracker.ts`).
  - Buluttan açılışta `readIncoming` sunucunun kimliğini `uid` olarak verir. v1 okuyucusu dosyadaki `uid`'i attığı için bulut yolu kendi kimliğini ayrı taşımalı.
  - `applyExternal` yeni nesneye sunucunun kimliğini verir; bugün yeni v7 verir.
  - Aygıt taslağı ve gönderilen değişiklik kimliği taşır; `entityJson` onu karşılaştırmaya almaz.
  - Testler: aynı dosyanın iki kez yüklenmesi, ACK kaybı, geri alınan silme.
- **Dilim 4:** sözleşmede `EntityId` tipi, v2 dosyada 16 baytlık kimlik.
- Masaüstünde `replace`'in karşılığı ve araçlar henüz yok (ADR 0020).
