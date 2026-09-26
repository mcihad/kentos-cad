# Ürün komutu fixture'ları (`kentos.command-cases` v1)

TODOS.md `CMD-04..07`, [ADR 0013](../../docs/adr/0013-product-command-contract.md), [ADR 0022](../../docs/adr/0022-first-product-command.md), [ADR 0027](../../docs/adr/0027-line-and-polyline-commands.md), [ADR 0029](../../docs/adr/0029-desktop-selection-and-snap.md), [ADR 0032](../../docs/adr/0032-desktop-drawing-tools.md), [ADR 0037](../../docs/adr/0037-desktop-modify-tools.md), [ADR 0047](../../docs/adr/0047-desktop-edit-tools.md), [ADR 0057](../../docs/adr/0057-desktop-drawing-tools-3.md). Bir ürün komutunun doğrulama (`validate`), plan (`plan`) ve yürütme (`execute`) davranışını durum durum yazar: sonucun durumu, hata kodu, alan yolu, ileti ve uyarılar; yürütmeden sonra da belgenin hâli.

Aynı dosyaları iki uygulama koşar: web `apps/web/src/product/fixtures.test.ts` (`CadDocument`), masaüstü `crates/native/application/tests/fixtures.rs` (`kentos_domain::Document`, `cargo test -p kentos-native-application`). İkisi de bütün durumları geçmelidir. Kayıttaki her komutun bir dosyası vardır; her dosyada en az 20 durum bulunur.

| Dosya | Komut | Durum |
|---|---|---|
| `v1/cad.polygon.create.json` | `cad.polygon.create` v1 (ADR 0022) | 23 |
| `v1/cad.line.create.json` | `cad.line.create` v1 (ADR 0027) | 20 |
| `v1/cad.polyline.create.json` | `cad.polyline.create` v1 (ADR 0027) | 23 |
| `v1/cad.entities.delete.json` | `cad.entities.delete` v1 (ADR 0029) | 23 |
| `v1/cad.point.create.json` | `cad.point.create` v1 (ADR 0032) | 23 |
| `v1/cad.circle.create.json` | `cad.circle.create` v1 (ADR 0032) | 23 |
| `v1/cad.arc.create.json` | `cad.arc.create` v1 (ADR 0032) | 23 |
| `v1/cad.entities.transform.json` | `cad.entities.transform` v1 (ADR 0037; hizalama ADR 0047) | 38 |
| `v1/cad.entities.edit.json` | `cad.entities.edit` v1 (ADR 0047) | 28 |
| `v1/cad.entities.array.json` | `cad.entities.array` v1 (ADR 0047) | 25 |
| `v1/cad.entities.create.json` | `cad.entities.create` v1 (ADR 0057; `hatch` işlemi ADR 0062) | 27 |

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

- **`input`** komutun girdisidir, sözleşmenin tipiyle (`PolygonCreate`, `LineCreate`, `PolylineCreate`, `EntitiesDelete`, `PointCreate`, `CircleCreate`, `ArcCreate`, `EntitiesTransform`, `EntitiesEdit`, `EntitiesArray`, `EntitiesCreate`; JSON adlarıyla).
- **`nonFinite`**: JSON NaN ve ±∞ taşıyamaz. `{ "pts[1].y": "NaN" }` ya da `{ "a.x": "Infinity" }` gibi bir tablo, girdi okunduktan sonra o alana `NaN`, `Infinity` ya da `-Infinity` koyar. Yol, hata iletisinin `path` alanıyla aynı yazılır (`c.x`, `r`, `a0`, `transform.center.y`). İsteğe bağlı bir sayının (`z`) yerini girdi bir sayıyla verir; tablo o sayıyı değiştirir.
- **`result`** sonucun tamamıdır (`CommandResult`): `status` ve duruma göre `output` ve `warnings`, ya da `error`. Alan alan tam eşit olmalıdır. Sayılar sayı olarak karşılaştırılır (dosyanın `1`'i belgenin `1.0`'ıdır).

## Yer tutucular

Sürüm ve kalıcı kimlik iki uygulamada aynı sayı değildir; dosya onları adla yazar. Var olan nesneye kalıcı kimliğiyle başvuran komut (`cad.entities.delete`) kimliği önce `captureUid` ile alır, sonra `$uid:ad` ile yazar.

| Yazılan | `input` içinde | `result` içinde |
|---|---|---|
| `"$current"` | adımdan önceki sürüm, ondalık metin | adımdan sonraki sürüm |
| `"$ad"` | `captureRevision` ile `ad` adıyla saklanan sürüm | aynı |
| `"$uid"` | — | yazılan nesnenin kalıcı kimliği: küçük harfli, tireli bir UUID ve `output.id` yuvasındaki nesnenin kimliği |
| `$uid:ad` (bir metnin içinde de) | `captureUid` ile `ad` adıyla alınan kalıcı kimlik | aynı; bir iletinin içinde de (`“$uid:a” kimlikli nesne çizimde yok…`) |
| `"$uidOf:22"` | 22 yuvasındaki nesnenin şimdiki kalıcı kimliği | aynı, adımdan sonra: bir komutun az önce yazdığı kopyanın kimliği |

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
- Bir dönüşümün beklenen geometrisi (`cad.entities.transform`) dönüşümün tanımından, aynı işlem sırasıyla çift duyarlıkla bağımsız hesaplanır (`scripts/fixtures/affine_reference.py`); uygulamanın çıktısından alınmaz. Dosyayı `scripts/fixtures/transform_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir dizinin (`cad.entities.array`) kopyaları sözleşmenin tanımından aynı yolla hesaplanır: ızgarada yer yer öteleme, kutupsal dizide adım adım dönüş ya da kopyalanan nesnelerin kutusunun ortasının dönüşü. Kutupsal durumlar çeyrek ve sekizde bir turlarla kurulur: orada çekirdeğin sinüs ve kosinüsü (fdlibm'inki, V8'inki gibi) Python'unkiyle aynıdır; başka açılarda son bitte ayrılabilir. Dosyayı `scripts/fixtures/array_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir düzenlemenin (`cad.entities.edit`) geometrisi girdide verilir; beklenen nesneyi sözleşmenin kuralı kurar (update yalnız geometriyi değiştirir; replace ve add katmanı ve rengi, keepData ile öznitelikleri ve etiketi alır, simgeyi almaz). Dosyayı `scripts/fixtures/edit_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir eklemenin (`cad.entities.create`) geometrisi de girdide verilir; beklenen nesneyi sözleşmenin kuralı kurar: geometrisi, girdinin katmanı, verildiyse rengi, öznitelikleri (verilmediyse boş) ve etiketi; simgesi yoktur. Dosyayı `scripts/fixtures/create_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- −0 dosyada yazılmaz: JavaScript'in yazdığı JSON onu 0 yapar. −0'ın korunduğu iki koşucunun kendi testlerindedir (`apps/web/src/wasm/transform.wasm.test.ts`, `crates/native/application/tests/transform.rs`).
