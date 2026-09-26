# Ürün komutu fixture'ları (`kentos.command-cases` v1)

TODOS.md `CMD-04..07`, [ADR 0013](../../docs/adr/0013-product-command-contract.md), [ADR 0022](../../docs/adr/0022-first-product-command.md). Bir ürün komutunun doğrulama (`validate`), plan (`plan`) ve yürütme (`execute`) davranışını durum durum yazar: sonucun durumu, hata kodu, alan yolu, ileti ve uyarılar; yürütmeden sonra da belgenin hâli.

Aynı dosyayı iki uygulama koşar; ikisi de bütün durumları geçmelidir:

| Dosya | Komut | Web (Vitest) | Masaüstü (`cargo test -p kentos-native-application`) |
|---|---|---|---|
| `v1/cad.polygon.create.json` | `cad.polygon.create` v1 | `apps/web/src/product/fixtures.test.ts` (`CadDocument`) | `crates/native/application/tests/fixtures.rs` (`kentos_domain::Document`) |

## Dosya

```json
{
  "format": "kentos.command-cases",
  "version": 1,
  "command": "cad.polygon.create",
  "commandVersion": 1,
  "title": "…",
  "note": "…",
  "setup": { "format": "kentos.document", "version": 1, "…": "DocumentSnapshotV1" },
  "cases": [{ "name": "…", "note": "…", "setup": { }, "steps": [ ] }]
}
```

- `command` ve `commandVersion`, koşucunun kendi kaydında (web `WEB_COMMANDS`, masaüstü `DESKTOP_COMMANDS`) bulduğu işleyiciyi seçer. Kayıtta olmayan komut koşucuyu durdurur.
- `setup` bir `.kcad` v1 çizimidir. Her durum onu uygulamanın çizim açtığı yoldan açar: web `readSnapshot` + `replaceWith`, masaüstü `DocumentSnapshotV1::from_json` + `Document::from_snapshot`. Durum kendi `setup`'ını verebilir. Açılan çizim temizdir, geçmişi boştur; yeni nesne dosyadaki en büyük kimliğin ardından numaralanır.
- `note` alanları yalnız okuyana yöneliktir.

## Adım

| `op` | Alanlar | Ne yapar |
|---|---|---|
| `validate`, `plan`, `execute` | `input`, `nonFinite`?, `result` | Komutu bu kipte çalıştırır; sonucu `result` ile karşılaştırır |
| `undo`, `redo` | `returns` | Belgenin geri alması ya da yinelemesi; `returns` adımın adıdır (`"Ekle"`) ya da `null` |
| `captureRevision` | `as` | Belgenin şimdiki sürümünü bu adla saklar |
| `captureUid` | `id`, `as` | Nesnenin kalıcı kimliğini bu adla saklar |

Her adımda `expect` (aşağıda) ve `note` de bulunabilir. Komut adımında `result` zorunludur.

- **`input`** komutun girdisidir, sözleşmenin tipiyle (`PolygonCreate`, JSON adlarıyla).
- **`nonFinite`**: JSON NaN ve ±∞ taşıyamaz. `{ "pts[1].y": "NaN" }` gibi bir tablo, girdi okunduktan sonra o alana `NaN`, `Infinity` ya da `-Infinity` koyar. Yol, hata iletisinin `path` alanıyla aynı yazılır.
- **`result`** sonucun tamamıdır (`CommandResult`): `status` ve duruma göre `output` ve `warnings`, ya da `error`. Alan alan tam eşit olmalıdır. Sayılar sayı olarak karşılaştırılır (dosyanın `1`'i belgenin `1.0`'ıdır).

## Yer tutucular

Sürüm ve kalıcı kimlik iki uygulamada aynı sayı değildir; dosya onları adla yazar.

| Yazılan | `input` içinde | `result` içinde |
|---|---|---|
| `"$current"` | adımdan önceki sürüm, ondalık metin | adımdan sonraki sürüm |
| `"$ad"` | `captureRevision` ile `ad` adıyla saklanan sürüm | aynı |
| `"$uid"` | — | yazılan nesnenin kalıcı kimliği: küçük harfli, tireli bir UUID ve `output.id` yuvasındaki nesnenin kimliği |

**Sürüm neden yalnız karşılaştırılır?** Sürüm bir sayaçtır; sözleşmesi eşitliktir. Web açılışı bir kez sayar, masaüstü saymaz (ADR 0020). Kimlik yeni nesnede rastgeledir (UUIDv7, ADR 0014).

## Beklenti (`expect`)

Yalnız yazılan alanlar denetlenir.

| Alan | Anlamı |
|---|---|
| `ids` | nesnelerin kimlikleri, belge sırasıyla |
| `entities` | `{ "kimlik": nesne }`: nesnenin tamamı, kalıcı kimliği hariç, alan alan eşit |
| `canUndo`, `canRedo`, `dirty` | doğru/yanlış |
| `revision` | `"same"`: adımdan önceki sürümle aynı; `"changed"`: farklı |
| `uids` | `{ "kimlik": ad }`: nesnenin kalıcı kimliği `captureUid`'in bu adla sakladığıdır; `"new"`: saklananların hiçbiri değildir |

## Kurallar

- Beklenen değerler sözleşmeden (ADR 0022: denetimler, sıraları, kodlar, yollar, iletiler) elle yazılır ve iki koşucuyla doğrulanır. Bir uygulamanın çıktısından kopyalanmaz. Değiştirmek incelenmiş bir davranış değişikliğidir (CLAUDE.md §23.4).
- İletiler kelimesi kelimesine karşılaştırılır: iki uygulama aynı cümleyi kurar. Kapalı alan aracının iletileri (kilitli ve gizli katman) değişmeden buradan gelir; izler (`fixtures/interaction/v1`) de onları geçer.
- Yalnız bir uygulamada olabilen durum buraya konmaz: masaüstünde yuvaların tükenmesi (`slots_exhausted`) kendi testindedir.
