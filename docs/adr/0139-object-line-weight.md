# ADR 0139: Nesnenin kendi çizgi kalınlığı

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0138 (Netcad NCZ ve kare kare içe aktarma), ADR 0025 (KCAD v2), ADR 0090 (stilli çizim); `docs/specs/kcad-v2.md` §6.1, §6.6; TODOS.md `FMT-01`, `FMT-10`.

## Bağlam

- Sahibin gözlemi: Rust tarafı DXF'in çizgi kalınlıklarını aktarmıyor. Doğruydu: modelde yalnız katmanın kalınlığı vardı (`LayerStyle.line_weight`). DXF'in her nesnesine yazdığı 370 grubu ve Netcad'in her kayda verdiği kalem kalınlığı okunuyor, sonra atılıyordu. NCZ raporu bunu söylüyordu: “KentOS nesne başına kalınlığı henüz taşımıyor”.
- C++ tarafı (KentOSCad) taşıyor. Her nesnenin kısıtlı bir görünüşü (`Appearance`) vardır: kalınlık kağıtta mikrometre olarak tutulur, kaynağı da ayrıca (`Explicit`, `ByLayer`, `ByBlock`).
  - DXF okuyucusu 370 sıfır ya da pozitifse değeri nesnenin kendi kalınlığı yapar.
  - NCZ okuyucusu kalemi (milimetrenin onda biri) aynı şekilde alır.
- Web araç çubuğunda yeni nesneler için bir “Kalınlık” seçicisi vardı, ama hiçbir yere bağlı değildi.

## Karar

### 1. Sözleşme

`EntityBase.line_weight`:

- nesnenin kendi kalınlığıdır, kağıtta milimetre, katmanınkiyle aynı birimde;
- `0` en ince çizgidir;
- yoksa “katmana göre”dir;
- `0`…`100` mm arasıdır (`MAX_LINE_WEIGHT`): DXF'in en kalını 2,11 mm, bir Netcad kalemi 100 mm'ye çıkar.

JSON'da ve TypeScript'te adı `lineWeight`'tır. Katalog şeması aralığı taşır. Biçim sınırının sürümü 9 olur (`FORMATS_VERSION`).

### 2. DXF

**Okurken 370** (milimetrenin yüzde biri) C++ okuyucusunun kuralıyla okunur:

- `≥ 0` nesnenin kendi kalınlığıdır;
- `−1` (BYLAYER), `−3` (çizimin varsayılanı) ve okunamayan bir değer katmanınkidir;
- `−2` (BYBLOCK), onu yerleştiren bloğun kalınlığıdır.

Bloklar patlatıldığı için BYBLOCK okurken çözülür: ekleme (INSERT) kendi kalınlığını, BYLAYER diyorsa katmanınkini üyelerine verir. Renkte de böyledir.

**Yazarken** nesnenin kalınlığı AutoCAD kalınlıklarının en yakınına yuvarlanır ve 370'e yazılır.

- Yuvarlanan değer KentOS'un kendi verisinde tam olarak durur (`weight`, `dxf/xdata.rs`) ve rapor bunu söyler.
- Okuyucu o değeri yalnız 370 hâlâ onun yuvarlandığı kalınlıkken kullanır: başka bir programdaki düzenleme bayat veriyi yener.

### 3. Netcad NCZ

Kaydın kalemi C++ okuyucusundaki gibi alınır:

- `0 < w ≤ 1000` (milimetrenin onda biri) ise `w / 10` mm'dir ve kaydın oluşturduğu **her** nesneye verilir. Akıllı nesnenin sembolü de kendi kalemiyle çizilir.
- Sıfır, negatif (Netcad'in kendi DXF dışa aktarımı böyle yazar) ya da 100 mm'yi aşan kalem katmanınkidir.

Rapor artık “nesnenin kendi kalınlığıyla alındı (en ince–en kalın)” der.

### 4. `.kcad`: belge şeması 3

Şema 3, şema 2'nin kendisi ve nesnenin `lineWeight` alanıdır (spesifikasyon §6.1, §6.6).

- Yazıcı `3`'ü yalnız bir nesnenin kendi kalınlığı varken yazar. Kalınlığı olmayan çizim şema 2 olarak kalır, baytları öncekiyle aynıdır: var olan bütün örnek dosyalar değişmeden kaldı.
- Şema 2 yükünde `lineWeight` bilinmeyen alandır, aralık dışı `bad_value`'dur.
- Sütunlarda (`kentos_kcad::columns`, `io/columns.ts`) bayrak `8`'dir ve kalınlık, nesnenin kayan sayılarının ilkidir.

Yeni örnek dosyalar:

- `line-weights.kcad`: bağımsız Python yazıcısıyla (`scripts/fixtures/kcad_v2_reference.py`); Rust kodeği aynı baytları yazar;
- `broken/line-weight-in-schema-2.kcad`;
- `broken/line-weight-too-heavy.kcad`;
- `broken/schema-version-4.kcad`: eskiden `schema-version-3`'tü; artık 3 geçerlidir.

`tools/kcad/kcad.py` şema 3'ü okur.

### 5. Çizim

Web ve masaüstü, katmanın düz görünüşünü nesnenin rengine ve kalınlığına göre kurar:

- `symbolsOfLayerStyle` / `symbols_of_layer_style` kalınlığı da alır;
- nesnenin kalınlığı yoksa katmanınki kullanılır;
- Kalınlık kapalıyken her çizgi yine bir pikseldir.

İki tarafı birbirine bağlayan `fixtures/style/v1/batches.json`'a iki yeni durum girdi: kendi kalınlığı olan nesneler, bir de kalınlıklar kapalıyken.

### 6. Sunucu

`kentos.feature.line_weight`:

- migration 0011; `null` katmanınki demektir;
- `0`…`100` denetimi veritabanında da vardır.

Geometrisi kaynak olan nesnenin (nokta, çizgi, düz çoklu çizgi, alan) tanımı yoktur. Bu yüzden kalınlık, renk ve etiket gibi kendi sütunundadır. Aynı şekilde yazılır, okunur ve kopyalanır.

## Sonuçlar

**Testler:**

- DXF'in 370'i: kendi kalınlığı, BYLAYER, BYBLOCK üyeleri, varsayılan, okunamayan değer (`crates/shared/formats/tests/dxf.rs`).
- 370'in yazılması ve tam değerin geri gelmesi (`tests/dxf_write.rs`).
- NCZ kaleminin nesneye geçmesi (`crates/shared/ncz/tests/fixtures.rs`).
- KCAD örnek dosyaları üç okuyucuda: Rust, tarayıcı, Python.
- Sunucunun sütunu (`crates/server/application/src/cad.rs`).
- İki çizicinin ortak batch dosyası.

**Açık kalanlar (sıradaki iş):**

- Öznitelikler'de kalınlık satırı (web ve masaüstü, `cad.entities.set` ile);
- araç çubuğu ve şeridin “Kalınlık”ının yeni nesnelere verilmesi.

**Taşınmayanlar:**

- Nesnenin kendi çizgi tipi (DXF 6) hâlâ taşınmaz. C++ tarafı da onu yalnız sayar.
- GeoJSON görünüş taşımaz.
