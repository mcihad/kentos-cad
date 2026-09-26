# ADR 0066: Öznitelikler ürün komutlarıyla yazar: `cad.entities.set` ve düzenlemenin `properties` işlemi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.8, §18; TODOS.md `CMD-04`, `CMD-07`, `UI-09`, `UI-11`; ADR 0013 (ürün komutları), 0047 (`cad.entities.edit`), 0057 (`cad.entities.create`), 0063 (masaüstünde Öznitelikler)
- **Web ajanının işi (27 Eylül, `2b4ef35`):** sözleşme, web işleyicisi, ortak durumlar ve web'in paneli. Masaüstü işleyicisi ve masaüstü paneli bu ADR'yle geldi.

## Bağlam

- İki platformun Öznitelikler paneli belgeye doğrudan yazıyordu (ADR 0063):
  - katman, renk ve öznitelikler `updateMany` / `update_many` ile;
  - noktanın, yazının, ölçünün ve taramanın değerleri `update` ile.
- Yerinde yazı düzenleyicisi (çift tık) ve web'in sembol komutları da öyle yazıyordu.
- Python ve yapay zekâ yüzeyi yalnız ürün komutlarını kullanacak (CLAUDE.md §18). Belgeye doğrudan yazan panel, onlarla aynı kuralları paylaşmıyordu: kilitli katman, gizli katman uyarısı, boş metin.

## Karar

### `cad.entities.set` v1 (`crates/shared/contracts/src/cad_properties.rs`)

- **Ne yapar:** kalıcı kimlikleriyle verilen nesnelerin katmanını, rengini, sembolünü, özniteliklerini ya da etiketini tek geri alma adımında değiştirir.
- **Girdi:** `uids`, `layerId`, `color`, `symbol`, `attrs`, `label`, `operation`, `expectedRevision`.
  - Verilmeyen özellik değişmez.
  - `color`, `symbol` ve `label` null ile kaldırılır: nesne katmanının rengiyle ve stiliyle çizilir, etiket görünmez. `layerId: null` ve `attrs: null` verilmemiş sayılır.
  - Öznitelik adıyla yazılır, null ile silinir; adı geçmeyenler kalır.
- **Adımın adı işlemdendir; işlem neyin değişeceğini sınırlamaz:**
  - `layer` "Katman değiştir";
  - `color` "Renk değiştir";
  - `symbol` "Sembol ata" (sembol null ise "Sembolü kaldır");
  - `attributes` "Değiştir";
  - `label` "Etiket değiştir".
- **Değişmeyen nesne yazılmaz:** zaten istendiği gibi olan nesne çıktıda yoktur. Hiçbiri değişmiyorsa adım yazılmaz, sürüm kalır.
- **Çıktı:** değişen nesnelerin kimlikleri ve yuvaları (`changed`, `ids`), sürüm. Plan, değişecek nesneleri yazılacakları gibi gösterir.
- **Retler, bu sırayla:**
  1. `no_entities`: "Özellikleri değişecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.";
  2. `invalid_uid`;
  3. `nothing_to_set`: "Değişecek özellik verilmedi. Katman, renk, sembol, öznitelik ya da etiket verin.";
  4. `invalid_attribute`: öznitelik adı boş ya da yalnız boşluk (Unicode `White_Space`);
  5. `invalid_revision`, `revision_conflict`;
  6. `entity_not_found`;
  7. `layer_not_found`, `not_a_layer` ("… nesneler yalnız bir katmana taşınır …");
  8. `layer_locked`: önce nesnelerin kendi katmanları girdinin sırasıyla ("… üzerindeki nesne düzenlenemez …"), sonra hedef katman ("… nesneler ona taşınamaz …").
- **Uyarı:** `layer_hidden`, yalnız en az bir nesne gizli katmana gerçekten taşınıyorsa: "“X” katmanı gizli; taşınan nesneler görünmeyecek."
- Ortak durumlar: `fixtures/commands/v1/cad.entities.set.json`, 30 durum (`scripts/fixtures/set_command_cases.py`).

### `cad.entities.edit`'in `properties` işlemi ve boş metin

- Geometri bir özellik değildir: Öznitelikler'in geometri satırları ve yerinde yazı düzenleyicisi `cad.entities.edit`'e `properties` işlemiyle yazar. Adım "Değiştir"dir.
- Yeni ret `empty_text`: yazının metni boş ya da yalnız boşluksa. Bu, düzenlemenin ve eklemenin (`cad.entities.create`) ortak geometri denetimine girdi; köşe sayısından sonra, sonluluktan önce gelir.
  - Boşluk Unicode'un `White_Space`'idir (Rust'ın `char::is_whitespace`'i). JavaScript'in `trim`'i değildir: U+FEFF'i alır, U+0085'i bırakır; web kendi `isBlank`'ini kullanır.
  - Ölçünün yazısı denetlenmez: boş yazı ölçülen değeri gösterir.
- Ortak durumlar: düzenleme 35, ekleme 28.

### Masaüstü

- **İşleyici:** `crates/native/application/src/set.rs`. `empty_text` ortak geometri denetimine eklendi (`edit::check_geometry`); `EditOperation::Properties` "Değiştir".
  - `DESKTOP_COMMANDS` katalogla eşittir; ortak durumların koşucusu yeni komutu da koşar.
- **Panel** (`apps/desktop/src/properties/`), `kentos_interaction::properties` üzerinden (web'in `ui/properties/write.ts`'i):
  - Katman ▾ ve Renk ▾ `cad.entities.set`'le yazar.
  - Öznitelik satırı `cad.entities.set`'le yazar. "Parsel" ya da "Ada" değişince, etiket eski değere eşitse aynı adımda etiket de değişir.
  - Geometri satırları ve taramanın deseni `cad.entities.edit`'in `properties` işlemiyle yazar.
  - Komutun reddi ve uyarısı günlükte uyarı olarak söylenir. Gizli katman uyarısını artık panel değil komut verir.
- **Yerinde yazı düzenleyicisi** (`apps/desktop/src/text_field.rs`) de `properties` işlemiyle yazar.

### Aynı birleştirmede web'in iki işi

- **Yazı ve Ölçülendirme** web'de `cad.entities.create` ile yazar (`11685cc`). Adım "Ekle"dir. Masaüstü zaten öyle yazıyordu (ADR 0060, 0061).
- **Parsel** Tapu alanı'nı boş yazar (`65f750a`): tapu alanı çizimden hesaplanmaz, tapudan girilir (CLAUDE.md §7, §23). Parsel de `cad.entities.create` ile yazar.
  - İleti: "Parsel 15 oluşturuldu; geometrik alanı 131.48 m². Ada, mahalle ve tapu alanı bilgisini Öznitelikler panelinden girin."
  - Masaüstünde Parsel henüz yok; taşınınca bu davranışla gelecek.

## Web'den ayrılanlar

- Yok. Sembol komutları (`style.assign`, `style.clearSymbol`) masaüstünde henüz yok; gelince `cad.entities.set`'le yazacaklar.

## Doğrulama

- `cargo test -p kentos-native-application`: ortak durumların hepsi masaüstü işleyicilerinde geçiyor (set 30, düzenleme 35, ekleme 28); katalog eşitliği.
- `pnpm test`: aynı durumlar web işleyicilerinde.
- `apps/desktop/src/properties/tests.rs`: panelin yazdıkları, adımların adları, kilitli katmanın reddi, gizli katman uyarısı.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`.
