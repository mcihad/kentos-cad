# Ürün komutu fixture'ları (`kentos.command-cases` v1)

TODOS.md `CMD-04..07`, [ADR 0013](../../docs/adr/0013-product-command-contract.md), [ADR 0022](../../docs/adr/0022-first-product-command.md), [ADR 0027](../../docs/adr/0027-line-and-polyline-commands.md), [ADR 0029](../../docs/adr/0029-desktop-selection-and-snap.md), [ADR 0032](../../docs/adr/0032-desktop-drawing-tools.md), [ADR 0037](../../docs/adr/0037-desktop-modify-tools.md), [ADR 0047](../../docs/adr/0047-desktop-edit-tools.md), [ADR 0057](../../docs/adr/0057-desktop-drawing-tools-3.md). Bir ürün komutunun doğrulama (`validate`), plan (`plan`) ve yürütme (`execute`) davranışını durum durum yazar: sonucun durumu, hata kodu, alan yolu, ileti ve uyarılar; yürütmeden sonra da belgenin hâli.

Aynı dosyaları iki uygulama koşar: web `apps/web/src/product/fixtures.test.ts` (`CadDocument`), masaüstü `crates/native/application/tests/all/fixtures.rs` (`kentos_domain::Document`, `cargo test -p kentos-native-application`). İkisi de bütün durumları geçmelidir. Kayıttaki her komutun bir dosyası vardır; her dosyada en az 20 durum bulunur.

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
| `v1/cad.entities.edit.json` | `cad.entities.edit` v1 (ADR 0047; `properties` işlemi, Öznitelikler; alan işlemleri, ADR 0065; tutamaçların `grip`, `straightEdge`, `arcEdge` işlemleri; ADR 0140'ın `split`, `reverse`, `simplify`, `cleanup` işlemleri; köşe kotunun taşınması, `elevation_lost` uyarısı ve Kot ver'in `elevation` işlemi, ADR 0142; çok parçalı alan, ADR 0143; yerleştirmenin geometrisi ve Patlat'ın kendi alanlı `add`'leri, ADR 0144) | 73 |
| `v1/cad.entities.array.json` | `cad.entities.array` v1 (ADR 0047; ADR 0140'ın `path` yerleşimi) | 30 |
| `v1/cad.entities.create.json` | `cad.entities.create` v1 (ADR 0057; `hatch` işlemi ADR 0062; `boundary`, ADR 0065; Hesap pencerelerinin `traverse`, `polarSurvey`, `forwardIntersection`, `resection` işlemleri; ADR 0140'ın `pointsBetween`, `intersectPoint`, `dimensionChain`, `dimensionBaseline` işlemleri; blok yerleştirmesi, ADR 0144) | 36 |
| `v1/cad.entities.set.json` | `cad.entities.set` v1 (Öznitelikler: katman, renk, kalınlık, sembol, öznitelik, etiket) | 32 |
| `v1/cad.blocks.define.json` | `cad.blocks.define` v1 (ADR 0144: Blok oluştur) | 23 |
| `v1/cad.blocks.edit.json` | `cad.blocks.edit` v1 (ADR 0144: Bloklar paneli) | 22 |

**Yalnız masaüstünün durumları** `v1/desktop/`'tadır ([ADR 0207](../../docs/adr/0207-point-clouds-and-large-data.md) §10): nokta bulutunu şimdilik yalnız masaüstü ekler, çizer ve işler (sahibin kararı, 8 Ekim 2026), web'in koşucusu bu klasörü okumaz (`v1/*.json`). Biçim aynıdır; masaüstünün koşucusu (`the_desktops_own_cases_match_its_handlers`) dosya başına 20 durum aramaz. Durumları KentOS kodu olmadan sözleşmenin kurallarından `scripts/fixtures/pointcloud_command_cases.py` yazar (`--check` farkı arar). Web nokta bulutunu alınca durumlar yukarıdaki dosyalara taşınır.

| Dosya | Komut | Durum |
|---|---|---|
| `v1/desktop/cad.entities.create.json` | `cad.entities.create` v1: Nokta bulutu ekle (`pointCloud`; bağlı, adres, gömülü, sanal bulut), `invalid_pointcloud`, `not_finite`, `unknown_asset`, katman | 5 |
| `v1/desktop/cad.entities.edit.json` | `cad.entities.edit` v1: Nokta bulutu stili (`pointCloudStyle`) ve retleri | 2 |
| `v1/desktop/cad.entities.transform.json` | `cad.entities.transform` v1: `pointcloud_fixed`, kilitli katmandaki bulut | 2 |
| `v1/desktop/cad.entities.array.json` | `cad.entities.array` v1: `pointcloud_fixed` | 1 |
| `v1/desktop/cad.blocks.define.json` | `cad.blocks.define` v1: `pointcloud_in_block` | 1 |

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
| `captureBlock` | `name`, `as` | Bu adlı blok tanımının kimliğini (ADR 0144) bu adla saklar |

Her adımda `expect` (aşağıda) ve `note` de bulunabilir. Komut adımında `result` zorunludur.

- **`input`** komutun girdisidir, sözleşmenin tipiyle (`PolygonCreate`, `LineCreate`, `PolylineCreate`, `EntitiesDelete`, `PointCreate`, `CircleCreate`, `ArcCreate`, `EntitiesTransform`, `EntitiesEdit`, `EntitiesArray`, `EntitiesCreate`, `EntitiesSetProperties`; JSON adlarıyla).
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
| `"$blockOf:Ad"` | çizimdeki o adlı bloğun kimliği (Türkçe harf katlamasıyla) | aynı, adımdan sonra: az önce tanımlanan bloğun kimliği; `expect.entities`'te de (yerleştirmenin `block`'u) |
| `"$block:ad"` | `captureBlock` ile `ad` adıyla alınan blok kimliği | aynı; `expect.entities`'te de |

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
| `blocks` | çizimin blok tanımları sırasıyla, her biri kimliği hariç, alan alan eşit (ADR 0144) |
| `blockIds` | `{ "ad": kimlik }`: o adlı bloğun kimliği; `"new"`: kurulumdakilerin ve `captureBlock`'un sakladıklarının hiçbiri değildir; `"$block:ad"` ya da kimliğin kendisi |

## Kurallar

- Beklenen değerler sözleşmeden (ADR 0022: denetimler, sıraları, kodlar, yollar, iletiler) elle yazılır ve iki koşucuyla doğrulanır. Bir uygulamanın çıktısından kopyalanmaz. Değiştirmek incelenmiş bir davranış değişikliğidir (CLAUDE.md §23.4).
- İletiler kelimesi kelimesine karşılaştırılır: iki uygulama aynı cümleyi kurar. Kapalı alan aracının iletileri (kilitli ve gizli katman) değişmeden buradan gelir; izler (`fixtures/interaction/v1`) de onları geçer.
- Yalnız bir uygulamada olabilen durum buraya konmaz: masaüstünde yuvaların tükenmesi (`slots_exhausted`) kendi testindedir.
- Bir dönüşümün beklenen geometrisi (`cad.entities.transform`) dönüşümün tanımından, aynı işlem sırasıyla çift duyarlıkla bağımsız hesaplanır (`scripts/fixtures/affine_reference.py`); uygulamanın çıktısından alınmaz. Dosyayı `scripts/fixtures/transform_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir dizinin (`cad.entities.array`) kopyaları sözleşmenin tanımından aynı yolla hesaplanır: ızgarada yer yer öteleme, kutupsal dizide adım adım dönüş ya da kopyalanan nesnelerin kutusunun ortasının dönüşü. Kutupsal durumlar çeyrek ve sekizde bir turlarla kurulur: orada çekirdeğin sinüs ve kosinüsü (fdlibm'inki, V8'inki gibi) Python'unkiyle aynıdır; başka açılarda son bitte ayrılabilir. Dosyayı `scripts/fixtures/array_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir düzenlemenin (`cad.entities.edit`) geometrisi girdide verilir; beklenen nesneyi sözleşmenin kuralı kurar (update yalnız geometriyi değiştirir; replace ve add katmanı ve rengi, keepData ile öznitelikleri ve etiketi alır, simgeyi almaz). Dosyayı `scripts/fixtures/edit_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Bir eklemenin (`cad.entities.create`) geometrisi de girdide verilir; beklenen nesneyi sözleşmenin kuralı kurar: geometrisi, girdinin katmanı, verildiyse rengi, öznitelikleri (verilmediyse boş) ve etiketi; simgesi yoktur. Dosyayı `scripts/fixtures/create_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- Yazının metni boş olamaz (`empty_text`): boş ya da yalnız boşluktan oluşan metin, boşluk Unicode'un `White_Space`'idir (Rust'ın `char::is_whitespace`'i; JavaScript'in `trim`'i değil: U+FEFF'i alır, U+0085'i bırakır). Düzenlemenin ve eklemenin durumları bunu sekme, satır sonu, U+0085, bölünmez boşluk ve ideografik boşlukla sınar; dosyada bu boşluklar `\u0085` gibi kaçışla yazılır.
- Öznitelikler'in geometri satırları `cad.entities.edit`'in `properties` işlemiyle yazar (adımı “Değiştir”); bu durumların kendi kurulumu (`setup`) vardır: nokta, yazı, ölçü, tarama.
- Alan araçları (ADR 0065) `cad.entities.edit`'le kendi adlarıyla yazar: Alan birleştir, Alan kesiştir, Alan çıkar, Alan böl, Alana çevir, Çizgiye çevir; İçine tıklayarak alan `cad.entities.create`'le, adımı “Alan oluştur”.
- Tutamaçlar (ADR 0068) `cad.entities.edit`'le yazar: sürüklenen tutamaç `grip` (adımı “Tutamaçla düzenle”), tutamaç menüsünün Ortasına köşe ekle ve Köşeyi sil'i `vertexAdd` ve `vertexRemove` (adımları “Köşe ekle”, “Köşe sil”), Düz kenar yap ve Yaya dönüştür `straightEdge` ve `arcEdge`; hepsi `update`'le, nesnenin bütün geometrisiyle (deliği de).
- Hesap pencerelerinin “Çizime ekle”si (Poligon hesabı, Kutupsal alım, Önden ve Geriden kestirme) noktaları `cad.entities.create`'le yazar: adı etiket ve öznitelik (Ad, Tür, kot bulunduysa Z (m)), adım pencerenin adı.
- Kapalı alanın halkası (dış halka ya da delik) en az 3 köşelidir; iki kenarından biri yaysa (yay değeri 0 değil; verilmeyen 0 sayılır) 2 köşeli olabilir: alana çevrilen daire, mercek ve dairesel kesit böyledir. Taramanın halkası en az 3 köşelidir. `cad.polygon.create` en az 3 köşe ister.
- Bir özellik değişikliğinin (`cad.entities.set`) beklenen nesnesini sözleşmenin kuralı kurar: verilen katman, renk, sembol ve etiket nesnenin kendisinin yerine geçer, null onu kaldırır; öznitelik adıyla yazılır, null ile silinir, adı geçmeyenler kalır; başka hiçbir alanı değişmez. Zaten istendiği gibi olan nesne çıktıda yoktur; hiçbiri değişmezse adım yazılmaz. Dosyayı `scripts/fixtures/set_command_cases.py` yazar; `--check` onu yeniden kurup karşılaştırır.
- −0 dosyada yazılmaz: JavaScript'in yazdığı JSON onu 0 yapar. −0'ın korunduğu iki koşucunun kendi testlerindedir (`apps/web/src/wasm/transform.wasm.test.ts`, `crates/native/application/tests/all/transform.rs`).
