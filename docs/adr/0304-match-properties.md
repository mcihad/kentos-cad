# ADR 0304: Özellik kopyala'da seçilen özellikler

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-18`: “Match properties; aktarılacak katman,
  renk, linetype, kalınlık, stil ve annotation özelliklerini seçebilsin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0140 (Özellik kopyala), 0260 (nesnenin görünüşü), 0183 (yazı ve ölçü stilleri), 0186 (tarama ekleri),
  0146 (kılavuz), CLAUDE.md §9.2 (araç = komut; dokunulan araç kurala getirilir).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Özellik kopyala (ADR 0140) kaynağın katmanını, rengini, kalınlığını ve sembolünü veriyordu. Hangi özelliklerin gideceği
seçilemiyordu. Çizgi tipi, saydamlık, yazı, ölçü, tarama ve kılavuz özellikleri hiç gitmiyordu. Kuralları araç kendisi kuruyordu,
`cad.entities.set` ile yazıyordu; manifestte düzeyi `semantic` yazsa da anlamsal bir komutu yoktu.

AutoCAD'in MATCHPROP'u, Ayarlar (Settings) penceresinde seçilen özellikleri verir:

- Temel özellikler: renk, katman, çizgi tipi, çizgi tipi ölçeği, kalınlık, saydamlık, kalınlık (thickness), çizim stili.
- Özel özellikler: ölçü, yazı, tarama, çoklu çizgi, görünüm penceresi, tablo, malzeme, gölge, çok kollu kılavuz.

Özel özellik yalnız aynı türden nesneye gider.

## Karar

### 1. Komut

`cad.entities.match` v1. Girdi:

- `source`: kaynak nesnenin kalıcı kimliği;
- `uids`: hedefler;
- `properties`: verilecek özellikler, her biri bir kez; verilmezse hepsi;
- `expectedRevision`.

Çıktı ve plan `cad.entities.edit`'inkidir. Tek geri alma adımının adı “Özellik kopyala”dır. Değişmeyen nesne çıktıda yoktur; hiçbiri
değişmezse adım yazılmaz.

### 2. Özellikler

**Nesnenin kendi özellikleri.** Kaynağın değeri gider. Kaynakta değer yoksa (katmana göre renk, sembol yok) hedefinki kalkar.

| Özellik | Hedefte | Not |
|---|---|---|
| `layer` | katmanı | |
| `color` | rengi | |
| `lineType` | çizgi tipi | |
| `lineTypeScale` | çizgi tipi ölçeği | |
| `lineWeight` | kalınlığı | Yalnız çizgiyle çizilenden çizgiyle çizilene. Nokta, yazı, ölçü, tarama, yerleştirme, resim, raster ve bulut çizgiyle çizilmez. |
| `transparency` | saydamlığı | |
| `symbol` | sembolü | |

**Türün özellikleri.** Yalnız aynı türden nesneler arasında gider; nesnenin öbür alanları kalır.

| Özellik | Giden | Kalan |
|---|---|---|
| `text` | yüzü (stil, yazı tipi, kalın, eğik, yatıklık), yüksekliği, genişlik çarpanı | sözler, yer, dönüş, hiza, zemin, eğri, nesneye bağı |
| `dimension` | görünüşü (stil, oklar, boylar, değerin yazılışı, çizgilerin rengi, kalınlığı ve tipi), değer yüksekliği, zemini | noktaları, türü, ilişkisi |
| `hatch` | deseni (ad, ölçek, açı, çizgiler ya da degrade) | bölgesi, ilişkisi |
| `leader` | oku, ok boyu, notun yüksekliği, zemini | çizgisi, notu, dönüşü |

**Öznitelikler, etiket ve ekler** verilmez.

### 3. Yazma ve kurallar

**Yazma.** Nesnenin kendi özellikleri `cad.entities.set` ile yazılır. Kaynağın katmanına taşıma onun kurallarıyla olur:

- kilitli katman reddedilir; söz kaynakta, yolu `source`;
- gizli katman `layer_hidden` uyarısı verir.

Kalınlık yalnız çizgiyle çizilenlere gittiği için iki yazma olabilir; ikisi aynı adımdadır, uyarılar bir kez söylenir. Türün
özellikleri ardından, nesnelerin o anki hâli üzerine belgeye yazılır (`update_many`). Bütün alanlar kalır; yazının nesneye bağı da.

**Atlananlar ve ret.** Kaynak hedeflerin arasındaysa atlanır. Kilitli katmandaki hedef ADR 0278'in kuralıyla uyarıyla atlanır;
hepsi kilitliyse ret olur. Kilitli katmandaki kaynaktan özellik alınır: okumak düzenlemek değildir.

**Retlerin sırası:**

1. `invalid_uid` (önce kaynak, sonra hedefler);
2. `no_entities` (hedef yok);
3. `no_properties` (boş liste; yeni kod);
4. `invalid_revision`, `revision_conflict`;
5. `entity_not_found` (önce kaynak, sonra hedefler);
6. `no_entities` (yalnız kaynağın kendisi);
7. `layer_locked` (bütün hedefler kilitli, sonra kaynağın katmanı).

**Plan** yazmayı çizimin bir kopyasında yapar ve nesnelerin son hâlini verir. Böylece iki yazmanın sırası ve kuralları da planda
görünür.

### 4. Araç

Özellik kopyala komutla yazar. İstemde anahtarlar vardır: Katman (K), Renk (R), Çizgi tipi (T; ölçeği de), Kalınlık (L),
Saydamlık (S), Sembol (M), Yazı (Y), Ölçü (Ö), Tarama (A), Kılavuz (V).

- Hepsi açık başlar (AutoCAD'in varsayılanı). Tıklamak ya da harfi yazmak açar ya da kapatır.
- Anahtarlar oturumda kalır (`Memory::match_off`, kapalıların bitleri).
- İmlecin yanındaki etiket açık olanların değerlerini söyler; kaynak yazı, ölçü, tarama ya da kılavuzsa türünün satırı da çıkar.
- İleti verilenleri sayar: “Özellikler 2 nesneye kopyalandı: “Parsel” katmanı, renk, …”. Hiçbir nesne değişmediyse “Nesneler zaten
  kaynağın seçilen özelliklerinde.” der.
- Hepsi kapalıyken bir şey yazılmaz, istemden birinin açılması istenir.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/cad_match.rs` (`EntitiesMatch`, `MatchProperty`); `codes.rs`'in `no_properties`'i;
  `EditOperation::MatchProperties`; katalog ve yetenekler.
- **Komut:** `crates/native/application/src/match_props.rs` (`entities_match`).
- **Araç:** `crates/native/interaction/src/match_properties.rs`.
- **Python ve MCP:** `kentos.cad.entities.match`; MCP yıkıcı listede. Komut geçmişinde “Özellik kopyala”.
- **Manifest:** `matchProperties` `semantic`, `via` `cad.entities.match`, `was` `cad.entities.set`.
- **Resimler:** `apps/desktop/src/match_scenes.rs`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/match_command_cases.py` KentOS kodu olmadan yazıldı. Ortak durumlar
  `fixtures/commands/v1/cad.entities.match.json`'dadır (32 durum):
  - bütün özellikler ve seçilenler;
  - kalınlığın çizgiyle çizilenlere gitmesi;
  - kaynakta olmayan değerin hedeften kalkması;
  - yazı, ölçü, tarama ve kılavuz;
  - türün özelliğinin başka türe gitmemesi;
  - karışık hedefler;
  - kaynağın kendisi;
  - kilitli hedef, gizli ve kilitli katmandaki kaynak;
  - değişmeyen nesne;
  - bütün retler ve sıraları, plan ve doğrulama, sürüm çakışması.

  Yetenek testi her kodu görür.
- **Araç:** `crates/native/interaction/tests/all/splitting.rs`:
  - istemde anahtarlar, etiket;
  - tıklama ve pencere, her biri bir adım;
  - Katman ve Renk kapalıyken yalnız öbürleri gider;
  - kilitli hedefte komutun sözü.
- **Süre** (`cargo test --release -p kentos-native-application --test all match_props::timing -- --ignored --nocapture`, bu
  makinede, 100 000 nesnelik çizimde):
  - 10 000 parselin planı 147 ms, yazılması 128 ms;
  - bir nesneye tıklama 0,03 ms.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
