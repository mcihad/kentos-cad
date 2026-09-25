# ADR 0014: Kalıcı nesne kimliği, çalışma yuvası ve eski dosya göçü

- **Durum:** kabul edildi (yön, 2026-09-25). Uygulama dilimleri aşağıdadır; ilki web belgesine kalıcı kimliği ekler.
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
