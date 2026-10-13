# ADR 0253: Komutların ortak protokolü — keşif, özet, durum damgaları, idempotency, kayıt, değişiklik kümesi ve uzun işler

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §5 `CAD-CMD-07`–`14`, `16`–`20` (`08`'in belge, dosya ve
  uzun iş kısmı). Kapsamı ben belirledim (sahibin sözü: “CAD-CMD adımlarını tamamla”, “bunu iyi yap”).
- **Bağlam belgesi:** ADR 0013 (ürün komutları kataloğu), 0022 (masaüstü işleyicileri), 0003 (işlem ve geri alma), 0026 (bulutta
  kimlik ve idempotency), 0124 (işlemlerin ayrı iş parçacığında hesabı), 0130 (başsız sunucu), 0137 (ajan bağlantısı), 0244
  (manifest), 0248 (ayar ve stil komutları, geri alma politikası), 0251 (pafta kitabı), 0252 (ilk kabul çizimi).
- **Semantik komutlar** (`CAD-CMD-01`, `05`, `06`: Esnet, Birleştir, Kır, Böl, Sınır, Tarama) ayrı kararda, ADR 0254'te.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Ürün komutları bugün şöyle:

- Tipli girdi alıyorlar, `CommandResult` dönüyorlar, `validate`, `plan`, `execute` biçimleri var.
- Her çağıran (araç, Python, MCP, ajan) komutun işleyicisini doğrudan çağırıyor.
- Bir komutu sarmalayan ortak bir katman yok. Bu yüzden şunlar eksik:

| Eksik | Madde |
|---|---|
| Plan yalnız komutun kendi çıktısı: neye dokunduğunu, kimin onunla değiştiğini, kutusunu, kaynakların sürümünü söylemiyor | `09` |
| Planla yürütmenin aynı sonucu verdiği güvence değil, beklenti | `10` |
| Belgenin tek sürümü (`expectedRevision`) var; nesnenin, tanımın, ayarın sürümü yok | `11` |
| Ağda ya da süreçte yinelenen çağrı ikinci kez uygulanabilir | `12` |
| Ne yapıldığı, hangi girdiyle, hangi algoritmayla, kaydedilmiyor; tekrar oynatılamıyor | `18` |
| Ret kodları katalogda değil; hata nesneyi söylemiyor; yeniden denenip denenmeyeceği yok | `07`, `19` |
| Nesne, katman, ayar ve paftayı birlikte değiştiren iş tek adım değil | `13`, `14` |
| Uzun hesap belgeyi tutuyor; iptalin aşamaları tanımsız | `16`, `17` |
| Dışa aktarma komut değil; dosyanın geri alınıp alınmadığı söylenmiyor | `08`, `20` |

Bir de kural sızıntısı var: komutun “ret: hiçbir şey yazılmadı” sözü güvence değil, işleyicinin dikkatine kalmış. Geri alma dışı
yazmalar (ayarlar, stil kitaplığı, pafta kitabı, katmanın görünürlüğü ve kilidi) yarıda kalan bir işte geri dönmüyor.

## Karar

### 1. Tek yol: her yürütme bir adımdır (belge)

`kentos_domain::Document::run_step` bir komutun bütün yürütmesini kapsar:

- **Bütün ya da hiç.** Gövde hata dönerse ya da bekçi reddederse her şey geri döner:
  - nesneler, katman ağacının adımlı değişiklikleri ve bloklar, işlemlerin tersiyle;
  - geri alma dışı yazmalar, ilk yazıldıkları anda alınan yedekten: çizimin adı, proje ayarları, stil kitaplığı, pafta kitabı,
    katmanların görünürlüğü, kilidi, keneti, adı, açıklığı ve etkin katman;
  - belgenin sürümü ve kirli işareti eski hâline döner; nesil ilerler (gösterenler yeniden okur).
- **İzleyenler adımın içinde.** Bağlı yazılar, ilişkili taramalar ve ilişkili ölçüler (ADR 0175, 0186, 0249) bekçiden önce izler.
  Bekçi adımın son hâlini görür.
- **Adın kaynağı.** Dıştaki adım adsız açılır, ilk içteki işlemin adını alır (“Ötele”, “Köşe yuvarla”). Geri almanın adı değişmez.
- **İç içe olunca.** Açık bir işlemin ya da grubun içinde `run_step` bir kayıt noktasıdır. Hata yalnız kendi yazdıklarını geri
  alır (geri alma dışı yazmalar dahil); izleyenler ve adımın kesinleşmesi dıştakinindir.
- **Rapor.** Adım bir rapor verir: eklenen, değişen, silinen nesneler (önceki ve sonraki hâlleriyle), katman ağacının, blokların
  ve geri alma dışı parçaların değişenleri, adımın adı ve geri alma sırasındaki numarası.
- Bugünkü `transact` ve gruplar aynen kalır.

Bunun sonucu olarak “ret: hiçbir şey yazılmadı” artık güvencedir: komutun sarmalayıcısı (§3) her ret, çakışma, girdi isteği ve
iptalde adımı geri alır.

### 2. Durum damgaları (`CAD-CMD-11`)

Her nesnenin, blok tanımının ve projenin parçalarının bir **durum damgası** vardır (`etag`):

- Biçim: kanonik JSON'unun SHA-256'sının ilk 16 onaltılık hanesi.
  - Kanonik JSON: `serde_json::Value` üzerinden, anahtarlar sıralı, sayılar en kısa gidiş-dönüş yazımıyla.
  - Sayının yazımı: kendine geri okunan en kısa ondalık basamaklar; iki aday eşit yakınsa çift olanı (ECMAScript, CPython ve
    serde_json'un ryu'su böyle yazar; Rust'ın `{:e}`'si eşitlikte yukarı yuvarlıyordu, `canonical.rs`'in `shortest`'i
    düzeltir). 10⁻⁵ ≤ |x| < 10¹⁶ ondalık, öbürleri işaretli üsle yazılır (`5.960464477539063e-8`).
  - Nesnede kalıcı kimlik (`uid`) girer, yuva (`id`) girmez.
- Damga içerikten gelir. Bu yüzden:
  - kayıt ve açılış damgayı değiştirmez;
  - Python'da, MCP'de ve masaüstünde aynı nesne aynı damgayı taşır;
  - geri alınan nesne planın gördüğü hâle döner, damgası da döner (aynı durum, aynı damga).
- Damgalanan parçalar: nesneler; bloklar; katman ağacı (`layers`); çizimin adı ve ayarları (`settings`); stil kitaplığı (`styles`);
  pafta kitabı (`sheets`, resimler SHA-256'larıyla).
- Okuyan her yanıt damgayı verir: nesnenin kaydı, sayfalı liste, belgenin özeti.
- Çağrının `expected`'ı (§4) beklenen damgaları taşır. Biri tutmazsa ya da nesne artık yoksa hiçbir şey yazılmaz. Sonuç
  `conflict`, kodu `stale_state`; hata tutmayan nesnelerin kimliklerini (`uids`) ve ilkinin yolunu (`expected.objects.<uid>`)
  söyler.
- Planın özeti (§5) okuduğu ve değiştirdiği her şeyin damgasını verir. Bir blok tanımı değişince ona dayanan plan geçersizdir.
  Dış başvurular (xref) gelince aynı kuralla girer.

### 3. Komutun sarmalayıcısı: kayıt (`CAD-CMD-18`)

Her ürün komutunun `execute`'u tek bir sarmalayıcıdan geçer (`kentos_native_application::call`). Çağıran kim olursa olsun
(araç, İşlemler, Python, MCP, ajan) aynı yoldur. Sarmalayıcı:

- girdiyi kanonik JSON'a çevirir (normalleştirilmiş girdi: tipli girdinin yeniden yazımı, varsayılanlar dahil);
- yürütmeyi `run_step` içinde yapar;
- başarıda belgenin **kaydına** bir satır yazar.

Kaydın satırı (`kentos.calls/1`):

| Alan | Ne |
|---|---|
| `seq` | belgedeki sıra numarası |
| `command`, `version` | komut ve sürümü |
| `input` | normalleştirilmiş girdi; 1 MiB'ı aşarsa yalnız özeti (`inputDigest`) kalır |
| `requestId`, `fingerprint` | idempotency (§6) |
| `base`, `revision` | yürütmeden önceki ve sonraki belge sürümü |
| `context` | birim, sistem (SRID), proje türü, koordinat düzlemi (`world`; kullanıcı koordinat sistemi gelince onun tanımı) |
| `algorithm` | çekirdeğin sürümü, sayısal politikanın kimliği (`kentos.numeric/1`), yapının kimliği |
| `seed` | rastgelelik kullanan komutun tohumu (bugün yok, alan hazır) |
| `created`, `changed`, `removed` | çıktı kimlikleri; tekrar oynatmada eşleme bunlardan |
| `warnings` | komutun uyarıları |
| `step`, `status` | geri alma adımı ve durumu: `done`, `undone`; geri alma ve yineleme günceller |

- Komutla yazılmayan düzenleme de satır olur (`kind: "edit"`: adımın adı ve değişen kimlikler). Kayıt böylece belgenin bütün
  değişikliklerini sırasıyla söyler; komut olmayanı tekrar oynatılamaz diye işaretler.
- Grup ve işlem içinde yazılan satırlar dıştaki adım kesinleşince kesinleşir; geri alınan iş satır bırakmaz.
- Kayıt oturumundadır (son 2 000 satır) ve dışa verilir: `kentos.replay/1` belgesi (projenin birimi ve sistemi, satırlar).
- **Tekrar oynatma:** `done` durumundaki komut satırları sırayla yeniden çalışır.
  - Önceki satırların ürettiği kimlikler yenileriyle eşlenir: `created` listeleri sırayla.
  - Her satır için sonuç: `same`, `differs` (sayılar, uyarı kodları ya da üretilenler farklı), `failed` ya da `skipped`
    (komut olmayan düzenleme).
- Python ve MCP'de `journal` ve `replay` var.

### 4. Çağrı zarfı

Kataloğun her komutu aynı zarfla çağrılır; girdi değişmez, zarf yanına gelir:

```json
{ "command": "cad.entities.offset", "version": 1, "op": "execute",
  "input": { … },
  "requestId": "4f1c…",
  "expected": { "revision": "12", "objects": { "<uid>": "<etag>" }, "blocks": { "<id>": "<etag>" },
                "layers": "<etag>", "settings": "<etag>", "styles": "<etag>", "sheets": "<etag>",
                "plan": "<planId>" } }
```

- Yüzler:
  - başsız sunucu `Session::call`, `rpc`'nin `run`'ı (aynı alanlar);
  - Python `doc.run(…, request_id=, expected=)`, tipli sarmalayıcıların `execute(…, request_id=, expected=)`'i;
  - MCP araçlarının `requestId` ve `expected` alanları;
  - masaüstünün ajan bağlantısı.
- Girdideki `expectedRevision` kalır: zarfın `expected.revision`'ıyla aynı anlamdadır, ikisi birlikte verilirse ikisi de denetlenir.

### 5. Planın ve yürütmenin özeti; planla yürütmenin aynılığı (`CAD-CMD-09`, `10`)

Her `plan` ve `execute` cevabı komutun kendi çıktısının yanında ortak bir **özet** (`summary`, `CommandSummary`) taşır:

- `added`, `changed`, `removed`: kalıcı kimlikler.
- `dependents`: girdide adı geçmediği hâlde değişenler (izleyen ölçüler, bağlı yazılar, ilişkili taramalar).
- `counts`.
- `bounds`: dokunulanların önceki ve sonraki kutularının birleşimi, metrede.
- `layers`, `blocks`: eklenen, değişen, silinenler.
- `project`: değişen geri alma dışı parçalar (`name`, `settings`, `styles`, `sheets`, `layerFlags`).
- `undo`: adımın adı (adım yazıldıysa) ve adıma girmeyen parçalar.
- `warnings`, kayıplar dahil (ör. `measure_lost`).
- `sources`: dokunulanların ve onların dayandığı blokların önceki damgaları, belgenin sürümü; değişen proje parçalarının önceki
  damgaları.
- `stamps`: dokunulanların sonraki damgaları. Zincirleme çağrı yeniden okumadan `expected` kurar.
- `algorithm`: komut, sürüm, sayısal politika, çekirdek.

Özet hep aynı yoldan çıkar: yürütmenin raporundan. Plan ise belgenin okuma kopyasında **yürütmenin kendisidir**. Böylece planın
söylediğiyle yazılan aynı algoritmadan, aynı sayısal politikayla gelir.

Plan bir **`planId`** verir: SHA-256 (32 hane) üzerinden şunların kanonik JSON'u:

- komut, sürüm, normalleştirilmiş girdi;
- kaynakların damgaları;
- sonucun özü: eklenenler kimliksiz içerikleriyle, değişenlerin sonraki içerikleri, silinenlerin kimlikleri, proje parçalarının
  sonraki damgaları.

`execute`'a `expected.plan` verilirse:

- yürütme gerçek belgede yapılır;
- aynı öz hesaplanır;
- tutmazsa adım bütünüyle geri alınır. Sonuç `conflict`, kodu `plan_changed`.

Önizlenen ile yazılan arasında fark sessizce geçemez: ya aynısıdır ya hiçbir şey yazılmaz.

### 6. Idempotency (`CAD-CMD-12`)

- `requestId` (1–128 karakter, `[A-Za-z0-9._:-]`) bir yazma isteğini adlar.
- Belge, yürüttüğü isteklerin defterini tutar. Satır: `requestId`, parmak izi (komut, sürüm, biçim, normalleştirilmiş girdi ve
  `expected`'ın kanonik JSON'unun SHA-256'sı), kaydın sırası, durum, ilk cevap.
- Aynı `requestId`:
  - **aynı parmak izi:** hiçbir şey uygulanmaz, ilk cevap döner (`replayed: true`; adım geri alınmışsa `undone: true`);
  - **başka parmak izi:** ret, kod `request_reused`, yol `requestId`. Kimlik başka bir iş için yeniden kullanılmış, bu bir hatadır.
- Yalnız yazan (`completed`) yürütmeler deftere girer. Ret ve çakışma girmez: durum değişmiş olabilir, yeniden denemek doğrudur.
- **Defter `.kcad`'e yazılır** (belge şeması 45, `requests`; son 256 istek, cevaplar 64 KiB'a kadar).
  - Gerekçe: “uygulandı, kaydedildi, cevap gitmeden süreç çöktü” durumunda yeniden deneme açılan dosyada defteri bulmalıdır.
  - Etki ile defter aynı dosyada, aynı kayıtla gider.
- Yalnız istemcinin kimlik taşıması yetmez; sunucu (belge) parmak izini denetler, sonucu saklar, geri alınmayı bilir.
- Bulutta veritabanı projesi kendi idempotency'sini sürdürür (ADR 0026); defter onun yerine geçmez.

### 7. Keşif ve hatalar (`CAD-CMD-07`, `19`)

Kataloğun her komut tanımına `capabilities` eklendi:

- `targets`: işlemin ya da yöntemin hangi nesne türlerine uygulandığı (`operation`, `kinds`, not).
- `refusals`: verebileceği ret kodları, sonuç durumuyla (`failed`, `conflict`, `needsInput`) ve ne zaman verildiğiyle.
- `warnings`: uyarı kodları ve ne zaman verildikleri.

Kodların anlamları tek yerde (`kentos_contracts::codes`); işleyiciler aynı sabitleri kullanır. Her kodun türü vardır: ret
(`failed`), çakışma, `needs_input`, uyarı, bazılarında uyarı ve hiçbirinde ret (Çoklu tara'nın `not_closed`'u) ve ev sahibinin
(`host`): komutun cevabı değil, Python kütüphanesinin, konsolun ya da araç sunucusunun kendi hatası (`unknown_method`,
`no_document`, `busy` …). Doğruluk testle tutulur:

- ortak durumlarda (`fixtures/commands/v1`) görülen her ret ve uyarı kodu komutun listesindedir;
- listedeki her kod ya bir durumda görülür ya da gerekçesiyle ulaşılamaz sayılır;
- semantik komutlar her nesne türüyle denenir: listede olan tür kabul edilir, olmayan listedeki kodla reddedilir.

`CommandError`'a eklenenler:

- `uids`: hatanın ilgili olduğu nesnelerin kalıcı kimlikleri. Hata yolu girdide bir kimliğe çıkıyorsa sarmalayıcı onu da ekler.
- `retryable`: yeniden denemenin anlamı var mı. Çakışmalar evet; girdinin kendisinden gelen retler hayır.

İleti yalnız insanlar içindir. İstemciler koda göre dallanır; Python'un hata sınıfları koddan gelir, ileti hiçbir yerde
karşılaştırılmaz.

### 8. Ortak değişiklik kümesi (`CAD-CMD-13`, `14`)

`cad.changeset.apply` v1 birden çok komutu tek iş olarak yazar.

Girdi:
- `steps`: `command`, `version?`, `input`, `as?`;
- `label?` (geri alma adımının adı; yoksa “Değişiklik kümesi”);
- `expectedRevision?`.

Yürütme:
- **Başvurular.** Sonraki adımın girdisinde `"$ad"`, adı verilen adımın `output.uid`'idir; `"$ad[i]"` `output.created[i]`'dir.
- **Atomiklik.** Adımlar sırayla, tek `run_step` içinde çalışır. Biri reddederse hepsi geri döner; nesneler, bloklar, katmanlar,
  ayarlar, stiller ve pafta kitabı dahil. Hata adımın yoluyla döner (`steps[2].input.uids[0]`), kodu adımın kodudur.
- **Kesinleşmede denetim.** İzleyenler adımların sonunda bir kez çalışır, ardından belgenin bütünlüğü yeniden denetlenir
  (`Document::integrity`, yalnız dokunulanlar):
  - nesnenin katmanı vardır;
  - yerleştirmenin bloğu vardır;
  - ilişkili ölçünün kopan bağı nedeniyle işaretlidir;
  - bağlı yazının nesnesi vardır;
  - ilişkili taramanın sınırı vardır.
  
  Yeni bir sorun varsa iş bütünüyle geri alınır (`integrity_failed`, `uids` sorunlu nesneler). Aynı denetim her komutun
  kesinleşmesinde de çalışır.
- **Geri alma.** Nesneler, katmanlar ve bloklar tek geri alma adımıdır. Ayarlar, stil kitaplığı ve pafta kitabı ADR 0248 ve
  0251'in politikasıyla adım değildir. Cevabın özeti hangi parçanın adıma girdiğini söyler (`undo.outside`).
- **Kayıt ve özet.** Kayıtta tek satır olur (adımların satırları onun içinde); özet tektir; `requestId` bütün kümeyi adlar.
- **Sınırlar.** Değişiklik kümesi iç içe verilemez (`nested_changeset`). Dosyaya yazan komut içine giremez
  (`side_effect_in_changeset`): yan etki geri alınamaz.

### 9. Uzun işler, ilerleme ve iptal (`CAD-CMD-16`, `17`, `08`)

Her komut iş olarak başlatılabilir: `start(call)` bir iş kimliği döner.

İşin aşamaları:

| Aşama | Ne olur | İptal |
|---|---|---|
| `queued` | sırada | hiçbir şey yazılmaz |
| `computing` | belgenin okuma kopyasında, ayrı iş parçacığında hesap; ilerlemeyi söyleyen komut `progress` verir | komut bayrağı sorar; sormasa da sonucu atılır |
| `staged` | sonuç hazır, belgeye yazılmadı; iş parçacığı hesabı bitirince bu aşamayı kendisi yazar, `job` onu görür | sonuç atılır |
| `committing` | belgeye yazılıyor | kısa ve bölünmez; yazma bitince `cancel` sonucu söyler (`already_committed`) |
| `done` / `failed` / `conflict` / `cancelled` | bitti | sonuç değişmez |

- Hesap sürerken belge tutulmaz: kullanıcı ve başka istekler çalışmaya devam eder.
- Kesinleşme kuralı:
  - belge hesaptan beri değişmediyse kopyanın adımı olduğu gibi yazılır;
  - değiştiyse önce dokunulanların önceki hâlleri belgedekiyle karşılaştırılır. Aynıysa yeni yuvalarla yazılır (başvurular
    kalıcı kimlikle olduğu için güvenlidir); değilse sonuç `conflict` (`document_changed`, `retryable`).
- Yüzler: başsız sunucu `start`, `job`, `cancel`, `wait`; Python `doc.start(…)` (`Job`: `phase`, `progress`, `cancel`,
  `result`); MCP `drawing.job.start`, `drawing.job.status`, `drawing.job.cancel`; masaüstünün ajan bağlantısı aynı yöntemler
  (kesinleşme arayüzün iş parçacığında).

### 10. Dosyaya yazan komutlar (`CAD-CMD-20`, `08`)

DXF ve paftaların PDF'i katalog komutu oldu:

- `file.export.dxf` v1: `path`, `layers?`, `overwrite`;
- `file.export.pdf` v1: `path`, `sheets?`, `overwrite`.

Akışları:
- **`validate`:** yolu ve girdiyi denetler.
- **`plan`:** dosyayı yazmadan yazılacak olanı söyler: nesne ve katman sayıları, yazıcının raporu, paftalar, notlar, tahmini bayt.
- **`execute`:** geçici adla yazar, diske indirir, geri okuyup karşılaştırır, sonra atomik yer değiştirir. Yarım dosya yoktur;
  eski dosya ancak yenisi tamsa gider. `overwrite: false` iken var olan dosya reddedilir (`file_exists`).

Etkileri:
- Belgeyi değiştirmezler, geri alma adımı değildirler. **Belgenin geri alması dosyayı geri almaz;** katalog ve cevap bunu
  söyler.
- Kayıtta yan etkiyle görünürler: yol, bayt, SHA-256. Tekrar oynatma dosyayı yeniden yazmaz, karşılaştırır: komutun planını
  `overwrite` açık kurar, yazılacak baytların SHA-256'sını kayıttakiyle karşılaştırır. Aynıysa `same`, değilse `differs`;
  plan kurulamıyorsa `skipped` ve retin kodu (`replay_with`, başsız sunucunun `file_commands::replay_row`'u).
- Değişiklik kümesine giremezler (§8).
- Başsız sunucunun `export_dxf` ve `export_pdf`'i bu komutların ince sarmalayıcısıdır. Masaüstünün pencereleri aynı yazıcıyı ve
  aynı atomik yazmayı kullanır.

### 11. Kapsamın sınırı

- Sunucunun proje komutları (`project.*`) kendi protokolünü sürdürür: `ApiError`'ın kodu, yolu, revizyonu ve yeniden deneme
  bilgisi, sunucu tarafı idempotency (ADR 0013, 0026).
- İşlemler'in araçları kendi çalıştırıcısında kalır (ADR 0084, 0124): girdiler, hesap kopyada, tek adım. Katalog komutu olmaları
  ayrı iştir.
- İçe aktarmanın (DXF, GeoJSON, Shapefile, NCZ) katalog komutu olması ayrı iştir: koordinat sistemi sorusu ve katman eşlemesi
  girdiye dönüşmeli.

## Uygulama

- **Belge:**
  - `run_step` ve raporu, yedekler, adım numarası;
  - damgalar `etag.rs`;
  - kayıt ve defter `calls.rs`;
  - bütünlük `integrity.rs`;
  - geri almanın ve yinelemenin kayıttaki durumu.
- **Sözleşme:**
  - `command_call.rs` (`CallEnvelope`, `CallExpectations`, `CommandSummary`, `CallRecord`, `ReplayReport`, `JobStatus`);
  - `codes.rs`;
  - `CommandError`'ın `uids` ve `retryable`'ı;
  - kataloğun `capabilities`'i;
  - `cad_changeset.rs`, `file_export.rs`;
  - belge şeması 45'in `requests`'i.
- **Uygulama katmanı:**
  - `registry.rs` (komutların adla tipli dağıtımı, başsız sunucudan taşındı);
  - `call.rs` (sarmalayıcı, özet, `planId`, parmak izi, beklentiler, defter);
  - `changeset.rs`, `jobs.rs`, `replay.rs`.
- **Başsız sunucu:**
  - `Session::call`, `start`, `job`, `cancel`, `journal`, `replay`;
  - dosya komutları;
  - `rpc`'nin yeni yöntemleri.
- **Yüzler:** Python ve MCP.
- **Masaüstü:**
  - ajan bağlantısının `call`, `start`, `job`, `cancel`, `journal` yöntemleri;
  - DXF ve PDF pencereleri atomik yazmayla.
- **Dosya biçimi:**
  - spesifikasyon §6.13;
  - kodek;
  - bağımsız Python okuyucu ve yazıcısı;
  - örnek ve bozuk dosyalar;
  - `FORMATS_VERSION` 57.

## Doğrulama

- **Belgenin birim testleri:**
  - `run_step`'in geri alması (geri alma dışı parçalar dahil);
  - izleyenlerin bekçiden önce olması;
  - iç içe kayıt noktası;
  - damgaların kayıt-açılış ve geri alma kararlılığı;
  - kaydın geri alma ve yinelemeyle durumu.
- **Protokolün ortak durumları** (`fixtures/protocol/v1/cases.json`, bağımsız Python başvurusu
  `scripts/fixtures/protocol_cases.py`):
  - zarfın, beklentilerin, idempotency'nin, `planId`'nin, parmak izinin ve özetin kuralları;
  - sayıların kanonik yazımı: 3186 sayı (eşit yakın adaylar, alt normaller, üs sınırları);
  - Rust (`protocol_cases.rs`) ve Python aynı durumları geçer.
- **Değişiklik kümesi** (`fixtures/commands/v1/cad.changeset.apply.json`): atomiklik, başvurular, bütünlük reddi, geri alma
  adımı.
- **Kayıt ve tekrar oynatma:** kabul çizimi (ADR 0252) dört yoldan kaydedilir: masaüstünün arayüzü (iz), ajan bağlantısı,
  Python ve MCP.
  - Her yolun kaydı yeni bir belgede oynatılır: her satır `same`; dosya komutlarının satırları da (baytları aynı).
  - Arayüzün izi komutsuz düzenleme bırakmaz: izin `journal` beklentisi (`edits` 0, komut sayısı, tekrar oynanır).
  - Arayüzün kaydı Python'unkiyle aynı komutları aynı sırayla taşır.
- **Uzun işler:** iptalin her aşaması (hesapta, `staged`'de, bittikten sonra `already_committed`); kesinleşmede belge
  değişmişse eşleme ya da çakışma.
- **Dosya komutları:** yarım dosya bırakmama (yazma sırasında hata), `overwrite`, geri okuma.
- **Keşif:** kataloğun kodlarının ortak durumlarla çapraz denetimi; semantik komutların tür denemesi.

## Sonuçlar

- Bir ajan, betik ya da arayüz:
  - komutun neye dokunacağını yazmadan görür;
  - planı onaylanmış hâliyle yazar;
  - yinelenen isteği ikinci kez uygulamaz;
  - ne yaptığını sırasıyla geri okuyup başka bir belgede tekrar oynatabilir.
- Retlerin “hiçbir şey yazılmadı” sözü güvencedir.
- Açık kalanlar:
  - ayarların ve stil tablolarının geri alma adımı olması (ADR 0248'in göç yolu);
  - içe aktarmanın ve İşlemler araçlarının katalog komutu olması;
  - dış başvurular (xref) gelince damgalarının eklenmesi;
  - web'in karşılıkları (TODOS.md §0.2).
