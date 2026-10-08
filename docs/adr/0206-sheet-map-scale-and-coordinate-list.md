# ADR 0206: Paftada haritanın ölçeği ve koordinat listesinin kaynağı

- **Durum:** kabul edildi (2026-10-08). Sahibin isteği (8 Ekim): “layout tasarımında ölçek elle girilebilir kalsın ama
  haritanın ölçeğini otomatik getiren bir düğme olsun”; “paftadaki koordinat listesinde koordinatların nereden geldiği belli
  olsun, sütun adları vb. ayarlanabilsin”. İki parça, iki platformda.
- **Bağlam belgesi:** ADR 0164 (pafta düzeni; §3.1 öğe türleri, §7 veriye bağlı özellikler, §11 arayüz), ADR 0205 (projenin
  çizim ölçeği ve Ölçek yaz…).

## Bağlam

- Haritanın ölçeği standart listeden seçilir ya da yazılır (Ölçek, Elle 1/…); yeni harita projenin çizim ölçeğiyle başlar.
  Çizimin ölçeği sonradan değişince (ADR 0205) ya da harita başka ölçekle kurulunca projenin ölçeğine dönmek için sayıyı
  bilmek ve yazmak gerekir. Çizim alanında görüleni haritaya almak için de merkez “Görünümden al” ile gelir, ölçek elle bulunur.
- Koordinat listesinin kaynağı “Çizimde seçili nesneler” ya da bir katmandır. Seçili nesneler **o anki** seçimdir: çizimde
  seçim değiştikçe liste değişir, boş seçimde liste boşalır; listenin hangi nesnelerden geldiği pafta tasarımcısında görünmez.
  Sütun başlıkları sabittir (Nokta, Y (m), X (m), Z (m); yerel projede X, Y) ve alan satırı “Alan = … m²”dir.

## Karar

### 1. Haritanın ölçeği çizimden ve görünümden

Harita bölümünde (sabit görünümlü harita) Ölçek ile Elle 1/… alanlarının altında üç düğme:

- **Çizim ölçeğini al**: haritanın ölçeği projenin çizim ölçeği olur (paydası tam sayıya yuvarlanır); merkez değişmez.
  Harita zaten bu ölçekteyse düğme kapalıdır. Ölçek elle seçilip yazılabilmeye devam eder.
- **Görünüme sığdır**: harita çizim alanında şu an görüneni gösterir. Merkezi çizim alanının merkezi, ölçeği görünümün
  genişliğini ve yüksekliğini (metre), haritanın kendi dönüşüyle, çerçeveye (kâğıtta µm) sığdıran **en büyük standart
  ölçek**tir (paydası gerekeni karşılayan en küçük standart payda); hiçbiri yetmiyorsa gerekeni karşılayan en küçük tam sayı.
  Dönük haritada görünümün kutusu dönüşle büyür: genişlik `w·|cos θ| + h·|sin θ|`, yükseklik `w·|sin θ| + h·|cos θ|`.
  Hesap çekirdekte: `kentos_sheet::atlas::fit_view_scale` (web'e `fitViewScale`), atlasın sığdırmasıyla aynı payda kuralı.
- **Görünümden al**: bugünkü gibi yalnız merkez.

Birden çok harita seçiliyse her biri kendi çerçevesi ve dönüşüyle, tek adımda. Atlas haritasında düğmeler yoktur (ölçeğini
atlas verir).

### 2. Koordinat listesinin kaynağı

Kaynak üç türdür (`CoordSource`):

- **Seçilen nesneler** (`objects`, yeni): listenin nesneleri kalıcı kimlikleriyle (`uid`) listede saklanır; **Seçimi al** o an
  çizimde seçili olanları alır (en çok 10 000). Çizimden silinen nesne listeden düşer, sayısı söylenir.
- **Katman** (`layer`): katmanın bütün nesneleri.
- **Çizimde şu an seçili olanlar** (`selection`): eski davranış, o anki seçim; eski paftalar ve yeni liste böyle açılır,
  arayüzde “canlı” diye belirtilir ve özeti sabitlemek için Seçimi al'ı söyler.

Bölümde kaynağın ne verdiği bir satırla yazılır: kaç nesne, kaç nokta, tek kapalı şekil mi (“Seçilen 1 nesne: kapalı şeklin
4 köşesi, alanı 812.40 m².”, ““Parsel” katmanı: 12 nesne, 48 nokta.”, “Çizimde seçili nesne yok: liste boş. …”); sözler
çekirdeğindir (`display::coordinate_summary`, web'e `coordinateSummary`), ev sahibi kaynağın nesne sayısını ve çizimde
olmayanları listenin girdisinde verir (`CoordinateInput.objects`, `missing`). **Çizimde göster** kaynağın nesnelerini çizimde
seçer, modele döner ve onlara yakınlaşır. Öğeler listesinde listenin altında kaynağı yazar (web).

### 3. Sütunlar

`CoordinateListItem.columns` (yazılmamışsa bugünkü başlıklar): Nokta, doğu (Y ya da yerel X), kuzey (X ya da yerel Y), Z
sütunlarının başlıkları ve alan satırının sözü (`area`, “Alan”). Boş bırakılan başlık varsayılanıdır. Başlıklar tek satır,
en çok 40 harftir; çekirdek denetler (`invalid_column_name`). Birimler başlığın içindedir, kullanıcı yazar. Varsayılanlar
çekirdeğin `display::coordinate_headings`'idir (web'e `coordinateHeadings`; web alanların soluk yazısında gösterir).
Masaüstü listenin Ondalık, Z sütunu, Kapanış ve Alan satırı alanlarını da artık gösterir (web'de vardı).

## Kapsam dışı

Ortak köşelerin listede bir kez yazılması, sütun sırasının değişmesi, sütun gizleme (Nokta, Y, X her zaman vardır).

## Uygulama

- **Çekirdek** (`kentos-sheet`): `atlas::fit_view_scale`; `kinds`'in `CoordSource::Objects` (`CoordObjects`), `CoordColumns`,
  `CoordinateListItem.columns`; `validate`'in kaynak ve başlık denetimi; `display::coordinate_headings`, `coordinate_summary`;
  `CoordinateInput.objects` ve `missing`. WASM `fitViewScale`, `coordinateSummary`, `coordinateHeadings`.
- **Web**: `ui/sheet/inspector/mapSection.ts` (üç düğme), `ui/sheet/inspector/dataSections.ts` (kaynak, özet, Seçimi al, Çizimde
  göster, başlıklar), `app/sheet/inputs.ts` (`coordinateObjects`, `coordinateInputOf`), sheet host'un `mapPlace().view`,
  `coordinateInput`, `coordinateObjects`; öğe listesinde kaynak (`product/sheet/adapter.ts`).
- **Masaüstü**: `kentos-sheet-ui`'ın `Context.view_size` ve `layers`'ı, `Message::MapScaleFromDrawing`, `MapFitView`,
  `MapCentreFromView`, `Coord*` iletileri, denetçinin `map_row`'u ve `coordinate_part`'ı, başlıklar `Conv::Optional` ile;
  `apps/desktop/src/sheets.rs` Seçimi al ve Çizimde göster'i çizimin seçimiyle karşılar, `sheet_inputs::coordinate_objects`.

## Doğrulama

8 Ekim 2026:

- `cargo test -p kentos-sheet -p kentos-sheet-wasm -p kentos-sheet-ui`: 497 test; yeni `fit.rs` (elle hesaplanmış ölçekler,
  dönük çerçeve, standart dışı), `coordinates.rs` (başlıklar, özetin sözleri, denetimler, çizilen başlıklar ve alan satırı),
  tasarımcının `a_map_takes_its_scale_and_place_from_the_drawing`, `a_coordinate_list_takes_its_objects_and_headings`'i.
- `cargo test -p kentos-desktop`: 827 test (`sheet_inputs`'un kimlikle okuma testi dahil); clippy temiz.
- `node crates/wasm/sheet-wasm/tests/smoke.mjs`, `pnpm test` (3 975), `pnpm typecheck`.
- Resimler: masaüstü `kentos-sheet-ui --test screens adr_0206` (`.run/shots/sheet-desktop/*-0206-*`), web
  `sheet-shots.mjs --only harita-olcek-dugmeleri,koordinat-listesi-kaynak`.
