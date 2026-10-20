# ADR 0553: Alan alan birleştirme, birlikte değişenlerin grubu ve çok katmanlı uzlaştırma

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-37'nin kalanı ([ADR 0538](0538-source-reconcile.md)'in "Sonraya kalan"ı).
  Kapsamı koordinatörün sırasıyla ben belirledim.
- **Bağlam belgesi:** ADR 0538 (taban ve açık çatışma), 0503 (bağlı katman), 0532 (birlikte yazma, ilişkiler), 0405 ve 0440'lar
  (projenin ilişkileri), 0160 (komşu köşeler), 0043 (masaüstünün bulut istemcisi, taban ve çakışma), 0265 (yeni şema kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - sunucuda, istemcinin gönderdiği tabanla `project.changes`'in alan alan birleştirmesi (§4: bugün birleştirme istemcidedir,
    sunucunun sürüm denetimi aynıdır);
  - çok parçalı ya da çok köşeli (128'den fazla) nesnenin köşe bağı; bugün onlarla grup kurulmaz;
  - katmanın dışındaki komşuların (alan sınırlı çalışma kopyasının dışı, ADR 0531) gruba katılması;
  - PostGIS tablosunun alan alan birleştirmesi (bugün bağlı GeoPackage'ınki; PostGIS'in tabanı da aynı kayıttan alınır, ama
    tek tablolu yazma yoludur, `with` ister reddeder);
  - adlı çalışma sürümü ve review → publish akışı.

## Bağlam

ADR 0538 iki yanın değiştirdiği nesneyi, ayrı alanlarda olsa da, çatışma sayıyordu. Pratikte QGIS'te birinin `malik`'i,
KentOS'ta öbürünün `ada`'sı değişmiş parsel için kullanıcıdan "benimki mi tablodaki mi" demesi istenir, yanlış seçim öbür
değişikliği götürürdü. Üç eksik daha vardı:
- ortak köşesini iki yanın da taşıdığı komşu parseller, ya da bir yanın sildiği üst kayıtla öbür yanın eklediği alt kayıt, nesne
  nesne ayrı ayrı çözülürdü: sonuç topolojisi ya da ilişkisi bozuk bir birleşim çıkabilirdi;
- bir uzlaştırma bir katmanı alırdı; bina–parsel gibi birlikte tutarlı katmanlar ayrı çağrı ve ayrı işlemdi;
- bulut projesinde yalnız sürüm farkı vardı: iki editörün aynı nesnenin farklı alanlarına dokunması bütün nesneyi çatışma yapardı.

## Karar

### 1. Tabanın alan kaydı ve alan alan birleştirme

- **Taban genişler** ([ADR 0538 §1](0538-source-reconcile.md)): `SourceBase.fields` (`.kcad` şema **92**) `rows`'taki her nesne
  için aynı sırada bir kayıt tutar, base64:
  - katmandaki ve tablodaki türün ve geometrinin 8'er baytlık özeti (öznitelik dışı öz, kanonik JSON'un SHA-256'sının ilk
    8 baytı); tabloda satırı olmayan nesnenin tablo özeti 0;
  - bayrak (1: öznitelik özetleri geçerli, 2: köşe özetleri geçerli);
  - her öznitelik için 4 baytlık ad özeti (adın SHA-256'sının ilk 4 baytı, ad özetine göre artan sıra) ve 8 baytlık değer özeti
    (değerin SHA-256'sının ilk 8 baytı);
  - en çok 128 köşenin 4 baytlık özeti (iki koordinatın bitleri üstünde FNV-1a, artan sıra, bir kez).
- **Değerler değil özetler:** taban proje dosyasında durur; öznitelik değerlerini ve geometriyi ikinci kez saklamak dosyayı
  katlardı. Özet çakışması (ad için 2⁻³², değer için 2⁻⁶⁴) bir değişikliğin görülmemesi demektir; ad çakışırsa (aynı nesnenin iki adı
  aynı özette) o nesnenin öznitelik kaydı yazılmaz.
- **Öznitelik kaydı yalnız katmanın yanındandır** ve tablonun öznitelikleri bire bir geri okunduğunda geçerlidir. Okunmuyorsa
  (sütun tipinin biçimlemesi, sistem değişimi) nesne için bayrak 1 kalkar ve nesne eskisi gibi bütün olarak çatışır. Köşe özetleri
  de tablonun geometrisi katmanınkine bit bit eşitken geçerlidir.
- **Boyut:** kayıtlar çözülmüş en çok 8 MiB (`MAX_FIELD_BASE_BYTES`); sığmayan katmanın alan kaydı yoktur, tabanın öteki
  parçası (200 000 nesne sınırı) geçerlidir. Alan kaydı olmayan nesneyi iki yan değiştirdiyse nesne bütün olarak çatışır ve
  `field_base_missing` uyarısı bunu söyler (taban eski sürümde alınmışsa da).
- **Kural** (üç yönlü, taban B, katman L, tablo T):
  - bir öznitelik için: L ya da T B'den farklıysa o yan değiştirdi demektir (değeri olmaması da bir değerdir: kaldırma); yalnız
    biri değiştirdiyse onun değeri, ikisi aynı değere getirdiyse o değer; **ikisi farklı değerlere getirdiyse** o öznitelik
    çatışır, iki yanın değerleriyle;
  - tür ve geometri tek alan sayılır, aynı kuralla: yalnız bir yan değiştirdiyse onunki, ikisi aynı olduysa o, farklıysa
    `geometry` çatışması;
  - çatışma kalmadıysa nesne birleşmiştir: ne L ne T bire bir; `merged` cevabında hangi özniteliğin hangi yandan geldiği
    (`fromLayer`, `fromTable`) ve geometrinin hangi yandan geldiği yazar. Birleşim L ile aynıysa nesne yalnız katmanın değiştirdiği,
    T ile aynıysa yalnız tablonun değiştirdiği sayılır;
  - nesneyi birinin sildiği, öbürünün değiştirdiği çatışma eskisi gibidir (nesne bütün).
- **Çözüm** `take`/`rest` ile **çatışan kısmı** seçer: seçilen yanın öznitelik değerleri ve geometrisi çatışan yerlere gider, çatışmayan
  yerler yine birleşir. (ADR 0538'de yan bütün nesneyi alırdı.) Silinme çatışmasında ve alan kaydı olmayan nesnede yan bütün nesnedir.
- **Cevap:** `conflicts[].fields` yalnız çatışan öznitelikleri söyler (kaydı olmayan nesnede eskisi gibi farklı olan hepsini).
  `SourcesReconciled.merged` birleştirilenleri listeler.

### 2. Birlikte değişenler: grup

Bir nesne ancak şunlardan biriyle bir gruba girer; ikisi de **iki yanın** ürünüdür, yani tek bir yanın yaptığı değişiklik grup
kurmaz:
- **Paylaşılan köşe:** tabanda bir köşeyi paylaşan iki nesnenin (köşe özetleri tabanda) ikisi de değişmişse ve köşe için hem
  katman hem tablo "paylaşmıyor" diyorsa: katman iki nesnede de aynı köşeyi tutmuyor ve tablo da tutmuyor. Yalnız katmanın
  taşıdığı köşe grup kurmaz (tablo hâlâ paylaşıyor); uzak köşelerin taşınması da kurmaz.
- **İlişki:** projenin bir ilişkisinin (`relations`) alt alanı üst nesnenin kalıcı kimliğini tutar. Bir yan üst nesneyi sildiyse, öbür yan
  ona başvuran alt nesneyi ekledi ya da değiştirdiyse, ikisi bir gruptur. Ilişkinin iki katmanı da uzlaştırmada olmalıdır
  (`with`).
- Gruplar geçişlidir (kümeler); iki yan da kümeye değiştirdiyle girmiş olmalıdır.

Grup **bütün olarak, tek yanla** çözülür: üyelere `take` (herhangi birine) ya da `rest` aynı yanı verir; üyelerin ayrı yanlara
seçilmesi `group_split` ile reddedilir. Yan "katman" ise üyeler katmandaki hâlleriyle kalır ve tabloya yazılır (katmanda olmayan,
tablonun eklediği alt kayıt tablodan silinir); "tablo" ise üyeler tablonun hâliyle katmana alınır (silinen üst kayıt geri gelir). Grubun içinde
birleşme yapılmaz: birleşme nesnesi kendi başına bir çatışmadır, grupta olmayan nesne kendi başına birleşir.

`conflicts[]` grubun üyelerini de listeler: kendisi çatışmayan üye `related` türündedir, hepsi `group` kimliğini (üyelerin en düşük
kalıcı kimliği) taşır; `groups[]` kimliği, üyeleri, nedenleri (`sharedVertex`, `relation`) ve seçilmişse yanı söyler.

### 3. Çok katmanlı uzlaştırma

`SourcesReconcile.with`: aynı GeoPackage dosyasının başka bağlı katmanları, aynı çağrıda. Her katman kendi taban ve denetimlerinden geçer
(`no_source_base`, `layer_locked` …; retlerin yolu ilk katman için `layer`, ötekiler için `with`); başka dosyadaki ya da PostGIS katmanı
`transaction_split`, aynı katmanın iki kez verilmesi `invalid_input`. Dosyanın sürümü bir kez okunur (tek `sha256`); tablonun yanları
katmanlara alınır, sonra katmanlar `cad.sources.sync`'in `with`'li yazma yoluyla ([ADR 0532 §1](0532-forms-and-sources-transaction.md))
**tek SQLite işleminde** yazılır: geri alma adımı tek (“Uzlaştır: tablo”), dosyanın reddi hepsini geri alır. Cevapta toplamlar ve
`layers[]` (katman başına iki yön) vardır; çatışma ve birleşimde `layer` adı, tek katmanlıda yoktur. Kalıcı kimlikler proje
genelinde tektir; `take` nesneyi kimliğiyle söyler, katmanı söylemez.

### 4. Bulut: istemcide alan alan birleştirme

Veritabanı projesinin `project.changes`'i bugün nesne başına sürüm denetler; farklı alanlara dokunan iki editör çatışırdı. Bu ADR'de **sunucu
sözleşmesi değişmedi**; birleştirme istemcide, her iki çatışma yolunda, taban istemcide bulunduğu için yapılır:
- İstemci her nesnenin sunucudaki son bilinen hâlini tutar (`Tracked`: sürüm ve nesne). Üç yönlü birleştirme saf ve ortaktır
  (`kentos_contracts::merge_entities`): nesnenin JSON'unun her üst düzey alanı birleştirilir, öznitelikler tek tek; aynı alanı iki yan
  farklı değiştirdiyse hata (alanları söyler); birleşim bir nesne etmezse (tür ile geometri ayrı yan) çatışma.
- **Dışarıdan gelen değişiklik** (`take_remote`): gönderilmemiş yerel değişikliği olan nesne için sunucunun değişikliği gelirse ve
  alanlar çatışmıyorsa birleşim çizime alınır, bilinen sürüm sunucunun yeni sürümü olur, birleşim onun üstüne gönderilmek üzere bekler
  (`Taken.merged`; “… nesne alan alan birleşti” iletisi). Yoldaki komuta dahil nesne, silme ve alan çatışması eskisi gibi çatışmadır.
- **Sunucunun 409'u** (`failed_in`): yoldaki komutun cevabı çatışma ise ve sunucunun verdiği güncel kopya (`current`) bilinen hâlden
  farklı ve alanlar çatışmıyorsa birleşim çizime alınır, bilinen sürüm sunucunun `actual`'ı olur ve komut hemen yeniden gönderilir
  (`After::Retry(0)`); çatışan ya da çizim açıkken olanlar çatışmada kalır. Sunucunun kopyası bilinen hâlden farklı değilse (yalnız sürüm
  oynamış) kullanıcıya sorulur, eskisi gibi.
- **Sunucu tarafı tasarımı** (sonraya): `project.changes`'in `Update`'ine isteğe bağlı `base` (istemcinin başladığı kopya) eklenir; sürüm
  tutmayan bir güncellemede sunucu saklı kopya, `base` ve gelen nesneyle aynı birleştirmeyi kilit altında yapar, temiz birleşimi yeni sürüm olarak
  yazar ve cevapta `merged` kimliklerini söyler; `base`'in yalan olması yazanın kendi yazma hakkını aşmaz (alanların yetkisi `merged`/`projected` ile
  zaten uygulanır), yalnız hangi alanları değiştirmiş saydığını etkiler. Bunun için sözleşme, sunucu ve ikinci istemci yolu gerekir; bu dilimin dışındadır.
- İş çalışırken sunucuda yetkinin yeniden sorulması ([ADR 0542](0542-durable-server-jobs.md), [0550](0550-one-reach-resolver.md)) değişmedi: sunucu işinin
  kesinleşmesinde sürüm denetimi `expectedRevision` iledir (ADR 0542), bu ADR ona dokunmaz.

### 5. Masaüstü

Kaynakla uzlaştır penceresi ([ADR 0538 §3](0538-source-reconcile.md)): aynı dosyanın öteki bağlı katmanları kendiliğinden birlikte uzlaştırılır (hepsi tabanlıysa,
değilse yalnız seçili katman); özet “iki yanın farklı alanlarını değiştirdiği N nesne alan alan birleşir”, “aynı dosyanın öteki
katmanları da birlikte”, “K grup birlikte çözülür” satırlarını söyler; “Birleştirilenler” listesi her nesnenin hangi alanı hangi yandan aldığını
(ve geometrinin hangi yandan geldiğini) yazar; çatışma satırları yalnız çatışan alanları gösterir, grubun üyelerinde grubun nedeni ve boyu yazılır ve
bir üyeye seçilen yan gruptakilerin hepsine geçer. Bulut tarafında dışarıdan gelen birleşme ve 409 birleşmesi komut satırında söylenir.

### 6. Bilinen sınırlar

- Köşe bağı yalnız tabanda 128 köşeye kadar nesnelerdedir ve yalnız katmanın tuttuğu iki nesne arasındadır; iki katmanlı komşuluk (ilişki dışında) grup kurmaz.
- Özet taban yaklaşık nesne başına (16 + 2 + 12·öznitelik + köşe·4) bayttır; 8 MiB'ı aşan katmanda alan birleşimi olmaz.
- Aynı nesnenin alanını iki yan farklı değiştirip biri başka alanı da değiştirdiyse, çözümde seçilen yan yalnız çatışan yeri alır; kullanıcı öbür
  alanı da o yandan istiyorsa Öznitelikler'den sonra düzeltir.
- Bulut istemcisinin birleştirmesi nesnenin JSON üst düzey alanları kadar inceliktedir: bir çoklu çizginin köşeleri tek alan, `pts`'tir.

## Doğrulama

- `python3 scripts/fixtures/reconcile_cases.py --check`: `cad.sources.reconcile`'ın 55 durumu KentOS kodu olmadan (GDAL ile yazılmış dört dosya, kuralın
  bağımsız Python yazımı: üç yönlü birleşim, köşe grubu, ilişki grubu, çok katman); `--files` dosyaları yazar. Kapsadıkları: farklı alanlar,
  öznitelik ve geometri ayrı yanlardan, aynı alan çatışması ve iki yönde çözümü, aynı değer, alan kaldırma; paylaşılan köşenin iki yan taşıması, tek yan taşıması
  ve uzak taşıma (grup yok); silinen üst kayıt ve eklenen alt kayıt; grup çözümleri (`rest`, bir üyeye `take`, `group_split`); iki katman bir çağrıda,
  başka dosyadaki katman, aynı katman iki kez; eski 0538 durumları yeni kuralla.
- `KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all reconcile` yazılan iki dosyayı kaydeder (`kentos-uzlasti.gpkg`,
  `kentos-uzlasti-komsu.gpkg`); `reconcile_cases.py --written` onları OGR ile satır satır denetler. `a_base_without_field_records_conflicts_as_a_whole_and_says_so`
  `field_base_missing`'i sınar.
- `.kcad` şema 92: `fixtures/kcad/v2/source-field-bases.kcad` bağımsız Python yazıcısıyla, şema 91'de reddedilen ve iki bozuk örnek; şema 93 örneği
  `broken/schema-version-93.kcad`; `kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Bulut: `cargo test -p kentos-contracts --lib entity_merge`, `cargo test -p kentos-cloud --lib` (iki yeni yol, çakışma ve yoldaki komut testleri), gerçek
  sunucuyla `kentos-api` `native_tests` (geçici PostGIS).
- Masaüstü: `a_reconcile_takes_the_files_other_layers_and_resolves_a_group_whole`; resimler `cargo test -p kentos-desktop
  exchange::gpkg_tests::reconcile_group_screens -- --ignored --nocapture` (`.run/shots/gpkg-uzlastir-grup-*`).
- Python SDK ve MCP katalogdan: `python3 scripts/python/sdk.py`, `pnpm py:test`; manifest `python3 scripts/cad/manifest.py --check`.
