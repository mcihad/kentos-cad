# ADR 0306: Denetle: bozuk ilişkiler, kayıp stiller ve kullanılmayan kaynaklar

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-20`: “Audit/recover/purge; bozuk ilişki, kayıp
  tanım ve kullanılmayan named resource bulsun; kullanılan içerik temizlenmesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0175 (bağlı yazı), 0177 (Kullanılmayanları temizle), 0183 (yazı ve ölçü stilleri), 0186 (ilişkili tarama),
  0249 (ilişkili ölçü), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

AutoCAD'de üç komut var:

- AUDIT çizimin bütünlüğünü denetler ve isteğe bağlı onarır;
- RECOVER açılmayan dosyayı kurtarır;
- PURGE kullanılmayan adlı kaynakları (katman, blok, yazı ve ölçü stili, çizgi tipi) kaldırır.

KentOS'ta bunların bir kısmı zaten var:

- **Okuyucu reddeder** (`snapshot.rs`, `blocks::check`): olmayan katmandaki ya da gruptaki nesneyi, tanımlanmamış bloğun
  yerleştirmesini, bozuk diziyi. Böyle dosya açılmaz, yarım açılmaz.
- **Kurtarma:** KCAD v2 açılışı (ADR 0025, 0030) her bölümü doğrular. Kaydedilmemiş iş kurtarma kopyasındadır. Ayrı bir RECOVER
  gerekmez.
- **Kullanılmayanları temizle** (ADR 0177): kullanılmayan katmanları, grupları ve blokları penceresinde kaldırır.

Okuyucunun geçirdiği ama çizimde anlamı kalmamış bağlar ise vardır:

- nesnesi çizimde olmayan bağlı yazı (`labelOf`);
- sınır nesnelerinden biri çizimde olmayan ilişkili tarama (`assoc`);
- çizimde olmayan nesneye canlı bağı olan ilişkili ölçü (`refs`);
- projede olmayan yazı stiline bağlı yazı ya da tablo (`textStyle`), olmayan ölçü stiline bağlı ölçü (`dimStyle`).

Bunlar başka çizimden alınan parçalarda, elle düzenlenmiş dosyalarda ve eski sürümlerin kayıtlarında görülür. Belgenin izleme kuralları
(ADR 0175, 0186, 0249) yeni silmeyi doğru izler; yalnız dosyanın getirdiğini izleyemez. Kullanılmayan yazı ve ölçü stillerinin de
temizlenecek bir yolu yoktu.

## Karar

### 1. Komut

`cad.drawing.audit` v1. Girdi:

- `fix`: bozuk nesneleri onar; varsayılan kapalı (yalnız rapor);
- `purge`: temizlenecek kullanılmayan stil türleri, `textStyles` ve `dimensionStyles`;
- `expectedRevision`.

Çıktı `DrawingAudited`'dır:

- `findings`: her bulgunun kodu, nesnenin kimliği, Türkçe iletisi ve onarılıp onarılmadığı;
- `unused`: kullanılmayan kaynakların türü, kimliği, adı ve temizlenip temizlenmediği;
- `changed`: onarılan nesneler;
- `revision`.

Plan aynı çıktıyı yazmadan verir. Onarım tek geri alma adımıdır, adı “Denetle”dir. Stillerin temizliği proje ayarıdır, geri alma
adımı değildir (stil tablolarının öbür düzenlemeleri gibi, ADR 0183). Yazılacak bir şey yoksa bir şey yazılmaz.

### 2. Bulgular

Nesneler çizimin sırasıyla denetlenir. Her nesnenin bulguları şu sıradadır:

| Kod | Ne | Onarım |
|---|---|---|
| `brokenLink` | bağlı yazının nesnesi çizimde yok | bağ kopar (`labelOf`, `labelScale`), yazı kalır |
| `brokenHatch` | ilişkili taramanın dış sınır, ada ya da boş bırakılan nesnesi çizimde yok | ilişki kopar, tarama kalır |
| `brokenDimension` | ilişkili ölçünün canlı bağının nesnesi çizimde yok | bağ silmedeki gibi `object_gone` nedeniyle kopar, nokta yerinde kalır |
| `missingTextStyle` | yazının ya da tablonun yazı stili projede yok | stil Standart olur, görünüşü kalır |
| `missingDimensionStyle` | ölçünün ölçü stili projede yok | stil Standart olur, görünüşü kalır |

Onarım nesnenin geometrisine, verisine ve görünüşüne dokunmaz. Zaten kopmuş ölçü bağı bulgu değildir.

**Kilitli katman:** üstündeki bozuk nesne söylenir ama onarılmaz (`fixed: false`). Sayısı `layer_locked` uyarısıyla söylenir, katmanın
kilidini açma yoluyla. Kilitli olanlar yüzünden komut reddedilmez (ADR 0278).

### 3. Kullanılmayanlar

Sıra: katmanlar, gruplar, bloklar, yazı stilleri, ölçü stilleri.

- **Katman, grup ve blok:** kural Kullanılmayanları temizle'ninkidir (ADR 0177, `layer_purge`): kalanların kullanmadığı en büyük küme.
  Etkin katman ve kilitliler dışarıdadır. Adı katmanın yoludur (“Eski / Eski yol”). Bunlar raporda söylenir, temizlenmez: katman
  ağacını değiştirmek Kullanılmayanları temizle'nin ve `cad.layers.edit`'in, blok silmek `cad.blocks.edit`'in işidir.
- **Yazı ve ölçü stili:** çizimin ya da bir blok tanımının hiçbir nesnesinin adını vermediği stil, tablosundaki sırasıyla. `purge`'te
  adı geçen türün kullanılmayanları kaldırılır (`purged: true`).

Kullanılan hiçbir şey temizlenmez. Onarımla Standart'a dönen nesnenin kayıp stili zaten tabloda yoktur.

### 4. Retler

1. `invalid_revision`, `revision_conflict`.

Başka ret yoktur: her çizim denetlenebilir.

### 5. Araç

Denetle (`tool.audit`; CAD'de Yönet › Temizlik, CBS'de Düzenle › Nesne) komutla önizler ve yazar. Kapsamı bütün çizimdir.

**Seçenekler** oturumda kalır:

- Onar (O): açık başlar;
- Yazı stilleri (Y): kapalı başlar;
- Ölçü stilleri (Ö): kapalı başlar.

**Önizleme:** bozuk nesneler tehlike renginde kesikli çizilir. İstem kısadır, komut satırına sığar: bozuk nesnelerin sayısı
(kilitli katmandakiler ayrıca) ve kullanılmayan kaynakların sayısı. Günlük kullanılmayanları türleri ve adlarıyla söyler ve katman,
grup ve blokları Kullanılmayanları temizle'nin kaldırdığını hatırlatır.

**Davranış:**

- Seçenek değişince plan yeniden yapılır.
- Enter yazar; ileti onarılan nesneleri ve temizlenen stilleri sayar.
- Yazılacak bir şey yoksa Enter “onarılacak ya da temizlenecek bir şey yok” deyip çıkar.

İkon: belge ve büyüteç.

## Uygulama

- **Kurallar:** `crates/native/domain/src/audit.rs` (`audit_object`, `audit_repaired`, `audit`, `unused_styles`).
- **Sözleşme:** `crates/shared/contracts/src/cad_audit.rs` (`DrawingAudit`, `AuditPurge`, `AuditCode`, `AuditFinding`,
  `ResourceKind`, `UnusedResource`, `DrawingAudited`); katalog ve yetenekler.
- **Komut:** `crates/native/application/src/audit_cmd.rs` (`drawing_audit`).
- **Araç:** `crates/native/interaction/src/audit.rs`; `Memory::audit_fix`, `audit_purge_text`, `audit_purge_dims`.
- **Arayüz kaydı:** `catalog/v1/additions.json`'da `tool.audit`, ikonu ve şeritteki yerleri.
- **Python ve MCP:** `kentos.cad.drawing.audit` (yeni `drawing` modülü); MCP yıkıcı listede. Komut geçmişinde “Denetle”.
- **Manifest:** `tool.audit` `semantic`, `via` `cad.drawing.audit`.
- **Resimler:** `apps/desktop/src/audit_scenes.rs`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/audit_command_cases.py` KentOS kodu olmadan yazıldı. Ortak durumlar
  `fixtures/commands/v1/cad.drawing.audit.json`'dadır (20 durum):
  - yalnız rapor; onarım ve geri alma, yineleme; iki kez onarım;
  - stillerin temizliği türlere göre, birlikte, ikinci kez;
  - kilitli katman ve kilidi açılınca onarım;
  - yerleştirmesi silinen blok, nesneleri silinen stiller, onarılacak nesnesi kalmayan çizim;
  - temiz çizim;
  - plan ve doğrulama, iki ret.

  Kurulum, okuyucunun geçirdiği bozuklukları taşır: çizimde olmayan nesneye bağlı yazı, tarama ve ölçü bağı; projede olmayan stiller.
  Yetenek testi her kodu görür.
- **Araç:** `crates/native/interaction/tests/all/audit.rs`:
  - iletiler, istem, seçenekler ve önizleme;
  - tek adımda onarım ve geri alma; kilitli katmandakinin kalması;
  - Onar kapalıyken stillerin temizliği;
  - sağlam çizimde çıkış.
- **Süre** (`cargo test --release -p kentos-native-application --test all audit_cmd::timing -- --ignored --nocapture`, bu makinede):
  100 000 nesnede (10 000 bozuk bağlı yazı, 10 000 kayıp stilli yazı, 20 yazı stilinin onu kullanılmıyor) plan 149 ms (çizimin
  kopyasıyla), onarım ve temizlik 86 ms.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
