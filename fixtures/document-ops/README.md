# Belge işlemi fixture'ları (`kentos.document-ops` v1)

TODOS.md `DOM-02`, [ADR 0020](../../docs/adr/0020-native-document-model.md). Web'in `CadDocument`'inin (`apps/web/src/model/document.ts`) ekleme, değiştirme, silme, işlem, kayıt noktası, grup, geri alma, kirli bayrağı, sürüm ve katman davranışını adım adım anlatır. Başvuru web'dir: dosyalar web'in bugünkü davranışını yazar.

Aynı dosyaları iki uygulama koşar:

| Koşucu | Belge |
|---|---|
| `apps/web/src/model/documentOps.test.ts` (Vitest) | web: `CadDocument` |
| `crates/native/domain/tests/fixtures.rs` (`cargo test -p kentos-domain`) | masaüstü: `kentos_domain::Document` |

İkisi de bütün senaryoları geçmelidir.

## Dosya

```json
{
  "format": "kentos.document-ops",
  "version": 1,
  "title": "…",
  "note": "…",
  "setup": { "format": "kentos.document", "version": 1, "…": "DocumentSnapshotV1" },
  "scenarios": [{ "name": "…", "note": "…", "setup": { }, "steps": [ ] }]
}
```

- `setup` bir `.kcad` v1 çizimidir (`DocumentSnapshotV1`). Her senaryo onu uygulamanın çizim açtığı yoldan açar: web `readSnapshot` + `replaceWith`, masaüstü `DocumentSnapshotV1::from_json` + `Document::from_snapshot`. Senaryo kendi `setup`'ını verebilir.
- Açılan belge temizdir, geçmişi boştur. Yeni nesne dosyadaki en büyük kimliğin ardından numaralanır.
- `note` alanları yalnız okuyana yöneliktir. “(web bugün böyle)” notu, garip bulunup ADR 0020'de bildirilen bir web davranışını işaretler.

## Adım

Her adımda `op` ve işleme göre alanlar bulunur. Bütün adımlarda şu alanlar da olabilir:

- `expect`: adımdan sonraki durum (aşağıda);
- `returns`: işlemin döndürdüğü değer;
- `catch`: adım bu iletiyle hata vermeli. Hata orada yakalanır, sonra `expect` denetlenir;
- `note`.

| `op` | Alanlar | Web | Masaüstü (`kentos_domain::Document`) | Döndürdüğü |
|---|---|---|---|---|
| `check` | | yalnız `expect` | | |
| `add` | `entity`: kimliksiz nesne | `add` | `add` | yeni kimlik |
| `addMany` | `entities`, `label`? | `addMany` | `add_many` | kimlik listesi |
| `update` | `id`, `patch` | `update` | `update` (yamalı tam nesne) | |
| `updateMany` | `patches`: `{id, …yama}` listesi, `label`? | `updateMany` | `update_many` | uygulanan yama sayısı |
| `remove` | `ids` | `remove` | `remove` | |
| `transact` | `label`, `steps`, `throw`? | `transact` | `transact` | |
| `beginGroup` | `label` | `beginGroup` | `begin_group` | |
| `endGroup`, `cancelGroup` | | en son açılan grubun `end`/`cancel`'ı | `end_group`, `cancel_group` | |
| `undo`, `redo` | | `undo`, `redo` | `undo`, `redo` | adımın adı ya da `null` |
| `captureRevision` | `as`: ad | `revision`'ı saklar | `revision` | |
| `markSaved` | `revision`: saklanan ad | `markSaved` | `mark_saved` | |
| `markUnsaved` | | `markUnsaved` | `mark_unsaved` | |
| `repeat` | `times`, `steps` | adımları `times` kez koşar | | |
| `setVisible` | `id`, `visible` | `layers.setVisible` | `set_layer_visible` | |
| `toggleVisible` | `id` | `layers.toggleVisible` | `toggle_layer_visible` | |
| `toggleLocked` | `id` | `layers.toggleLocked` | `toggle_layer_locked` | |
| `isolate` | `id` | `layers.isolate` | `isolate_layer` | |
| `showAll` | | `layers.showAll` | `show_all_layers` | |
| `setExpanded` | `id`, `expanded` | `layers.setExpanded` | `set_layer_expanded` | |
| `setActive` | `id` | `layers.setActive` | `set_active_layer` | |
| `rename` | `id`, `name` | `layers.rename` | `rename_layer` | |
| `setLayerStyle` | `id`, `patch`, `label`? | `setLayerStyle` | `set_layer_style` (yamalı tam stil) | |

- **Yama** web'deki gibi sığdır: yamanın her alanı nesnenin (ya da stilin) aynı adlı alanının yerine geçer; `null` alanı siler (web'de `undefined`). Nesnenin kimliği yamayla değişmez. Masaüstü tam nesne alır: koşucu yamayı JSON üstünde nesneye uygular ve sonucu verir.
- **`transact`:** `steps` işlemin gövdesidir, içinde başka `transact` olabilir. `throw` verilmişse gövde adımlarından sonra bu iletiyle hata verir. Yakalanmayan hata dıştaki işleme geçer, en dışta senaryoyu düşürür.
- **Grup:** `endGroup` ve `cancelGroup` en son açılan grubu kapatır. Açık grubun içinde açılan grup dıştakine katılır; onun kapanışı bir şey yapmaz.

## Beklenti (`expect`)

Yalnız yazılan alanlar denetlenir.

| Alan | Anlamı |
|---|---|
| `ids` | nesnelerin kimlikleri, belge sırasıyla (çizim sırası, dosyanın yazdığı sıra) |
| `count` | nesne sayısı |
| `entities` | `{ "kimlik": nesne }`: nesnenin tamamı, alan alan eşit (sayılar sayı olarak) |
| `byLayer` | `{ "katman": [kimlik…] }`: katmanın nesneleri, belge sırasıyla |
| `canUndo`, `canRedo`, `dirty` | doğru/yanlış |
| `revision` | `"same"`: adımdan önceki sürümle aynı; `"changed"`: farklı |
| `layers` | `{ "katman": { visible, locked, expanded, name, style, isVisible, isLocked } }`. İlk beşi düğümün kendi değeridir; `isVisible` ve `isLocked` üst grupları da hesaba katan yanıttır |
| `activeLayer` | etkin katmanın kimliği |

**Sürüm neden yalnız karşılaştırılır?** Sürüm bir sayaçtır. Sözleşmesi şudur: yazılan içerik değişince değişir, ve `markSaved(r)` belgeyi yalnız `r` hâlâ güncel sürümse temizler. Artış miktarı sözleşme değildir. Web bazı değişiklikleri iki kez sayar (katman stili), masaüstü bir kez. Kayıt kuralı `captureRevision` → değişiklik → `markSaved` adımlarıyla denetlenir.

## Kurallar

- Beklenen değerler web'in kodundan elle yazılır, iki koşucuyla doğrulanır. Bir uygulamanın çıktısından kopyalanmaz. Değiştirmek incelenmiş bir davranış değişikliğidir (CLAUDE.md §23.4).
- Web'in yazılı bir kararla çeliştiği davranışlar fixture'a konmaz; ADR 0020'de bildirilir, masaüstü karara uyar ve kendi testinde (`crates/native/domain/tests/document.rs`) sınanır. Örnek: başarısız işlemdeki katman stili değişikliği web'de belgeyi kirletir (ADR 0003'e aykırı).
- Kapsam dışında: dışarıdan gelen değişiklik (`applyExternal`, `forgetHistoryOf`), `load`, açılıştan sonra `replaceWith`, katman ekleme (`layers.add`), ad/ayar/stil kitaplığı değişikliği ve belge olayları (`changed`, `attrs`, `touched`). Kalıcı kimlik (`uid`, ADR 0014) web belgesinde henüz yoktur; masaüstü onu kendi testinde sınar.
