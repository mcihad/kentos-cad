# ADR 0317: Blok özniteliklerinin seçenekleri, eşitleme, değiştirme, sorgu ve kütüphane kartları

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §10 `CAD-BLOCK-08`, `09`, `10`, `12`. Sahibin isteği: “CAD blocok ile
  devam et”; maddeler 3–4'erli gruplarla yapılıp sınanır. Bu dört madde üçüncü gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0144 (bloklar ve öznitelikleri), 0225 (bilgi kartının şablonu), 0278 (kilitli katman);
  - 0315 (blokta kararlı kimlik, birim), 0316 (Blok düzenleyici, `update`);
  - CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Öznitelik yalnız bir yazıydı.** Etiketi, sorusu, varsayılanı ve yeri vardı. AutoCAD'in ATTDEF seçenekleri yoktu:
  görünmez, sabit, doğrula, ön ayarlı, konum kilidi, çok satırlı. Alan da yoktu (değeri başka değerlerden yazılan öznitelik).
- **Tanım değişince yerleştirmeler kendi başınaydı.** Kalkan etiketin değeri yerleştirmede kalıyordu. Adı değişen etiketin
  değeri kayboluyordu. Kimse ne olduğunu söylemiyordu (ATTSYNC yoktu).
- **Blokla toplu iş yapılamıyordu.** Bir bloğun yerleştirmeleri başka bir blokla değiştirilemiyordu. Saymak, değere göre bulmak ve
  çizelge çıkarmak da yoktu (BLOCKREPLACE, BCOUNT, öznitelik çıkarma).
- **Bloklar paneli bir listeydi.** Bloğun kategorisi, etiketleri ve sürümü yoktu; büyük kitaplıkta aranamıyordu.

## Karar

### 1. Özniteliğin seçenekleri (`CAD-BLOCK-08`)

`AttributeDefinition`'a düz alanlar olarak eklendi (`AttributeOptions`). Hepsi kapalı başlar ve kapalıyken yazılmaz:

| Seçenek | Anlamı | DXF 70 |
|---|---|---|
| `invisible` | Çizilmez. Değeri tutulur, listelenir ve çizelgelenir. | 1 |
| `constant` | Değeri her yerleştirmede tanımın `value`'su. Yerleştirme kendi değerini tutmaz. | 2 |
| `verify` | Blok ekle iki kez sorar, ikisi aynı olunca yerleştirir. | 4 |
| `preset` | Blok ekle sormaz, varsayılanı alır. | 8 |
| `movable` | Yerleştirme yazısını taşıyabilir. Kapalıyken yazı tanımın koyduğu yerde kilitlidir. | (16 yazılmaz) |
| `multiline` | Değer satır sonları taşır, her biri bir satır. `boxWidth` verilirse kutuya sarılır. | — |
| `field` | Şablon (`{NO}/{TUR}`): her yerleştirme için doldurulup değer olarak yazılır. | — |

- **Kurallar** (`kentos_contracts::blocks::options_problem`):
  - Sabit öznitelik sorulmaz; doğrula ve ön ayarlı onunla seçilemez.
  - `boxWidth` yalnız çok satırlıda olur ve sonlu, artı bir boydur.
  - Alan şablonu boş olamaz; derlenmeyen şablon `invalid_field` ile reddedilir.
  - Bozuk seçenek `invalid_attribute_options` ile reddedilir.
- **Gösterim** (çekirdeğin `Attribute::shown`, `shape_at`):
  - Tek satırlı özniteliğin satır sonları boşluk olur.
  - Çok satırlı olan satırlarıyla ve kutusuyla çizilir.
  - Görünmez olan çizilmez.
  - Sabit olan tanımın değerini gösterir.
  - Taşınabilir olan yerleştirmenin `attributePlaces`'indeki yerde durur. Yerler tanımın koordinatlarındadır.
- **Patlat** aynı kuralla yazar:
  - görünmez ve boş olanlar yazı olmaz;
  - taşınmış olan yerinde kalır.
- **Yerleştirmenin yerleri** (`InsertEntity.attributePlaces`): etiketten noktaya, yalnız taşınabilir özniteliklerin. Boş eşleme
  yazılmaz.
  - Yeri değiştiren `cad.blocks.edit`'in `place` işlemidir (girdisi yerleştirmenin kimliği, `tag` ve `at`; `at` yoksa yer kalkar).
  - Taşınamayan öznitelik `attribute_not_movable`, bilinmeyen etiket `unknown_tag`, yerleştirme olmayan nesne `not_insert` ile
    reddedilir.
- **Alanlar:**
  - Şablon bilgi kartının dilidir (ADR 0225, `info_templates`): `{ifade}` parçaları yerleştirmenin öznitelikleri ve projenin `@`
    değerleriyle doldurulur.
  - Blok ekle, eşitleme, değiştirme ve `fields` işlemi (UPDATEFIELD, Alanları güncelle) alanları yazar.
  - Alanın değeri elle verilemez. `cad.blocks.insert` ve `cad.entities.set` sabit özniteliğe değer verilmesini
    `constant_attribute` ile reddeder.
- **`.kcad` şema 58** (`FORMATS_VERSION` 67): seçenekler, `attributePlaces` ve bloğun kartı (§4).
  - Kapalı seçenek `false` olarak yazılmaz; okuyucu `false`'u reddeder.
  - Bağımsız okuyucu ve yazıcı `tools/kcad/kcad.py`, `kcad_v2_reference.py`; örnek `fixtures/kcad/v2/block-cards.json`.

### 2. Eşitleme (`CAD-BLOCK-09`)

`cad.blocks.edit`'in `sync` işlemi (ATTSYNC, Öznitelikleri eşitle).

- **Kendiliğinden:** tanımı değiştiren `attributes` ve `update` işlemleri de aynı adımda eşitler. `sync: false` istenirse eşitlemez.
- **Saklama ilkesi:** yerleştirme yalnız kendisine verilen değerleri tutar (Blok ekle varsayılanda bırakılanı yazmaz). Eşitleme
  yalnız artık uymayanı yazar (`blocks_sync::sync`):
  - adı değişen etiketin değeri yeni etikete taşınır (`renames`, eski addan yeni ada);
  - artık hiçbir özniteliğin olmadığı ya da `removeTags`'te adı geçen etiketin değeri kalkar;
  - sabit olan özniteliğin kendi değeri kalkar (tanımınki görünür);
  - taşınamaz olan özniteliğin yeri kalkar;
  - alanın değeri yeniden doldurulur.
- **Rapor** (`BlocksEdited.sync`, `AttributeSync`):
  - değişen yerleştirmeler (`inserts`);
  - yerleştirmelerde değeri olmayan yeni etiketler (`added`, yerleştirme varsayılanı gösterir);
  - kalkanlar (`removed`), taşınanlar (`renamed`, eski ve yeni ad);
  - kalkan sabit değerler (`constant`), yerler (`places`), alanlar (`fields`).
  - Masaüstü bunu iletiyle söyler.
- **Kapsam:**
  - Yalnız çizimin kendi yerleştirmeleri eşitlenir. Başka bir tanımın içindeki yerleştirme değerlerini korur (tanım kendi
    düzenlemesiyle değişir).
  - Kilitli katmandaki yerleştirme olduğu gibi kalır ve `layer_locked` uyarısıyla sayılır (ADR 0278).
- **Tanımın sürümü:** nesneleri, tabanı, öznitelikleri ya da birimi değişen tanımın kartı varsa sürümü bir artar.

### 3. Değiştirme ve sorgu (`CAD-BLOCK-10`)

#### `cad.blocks.replace` (BLOCKREPLACE, Başka blokla değiştir)

- Bir bloğun yerleştirmeleri başka bir bloğu yerleştirir; tek adım “Bloğu başkasıyla değiştir”.
- **Taşınanlar:**
  - yer, ölçek, dönüş, ayna, katman, görünüş ve öbür veriler kalır;
  - değerlerden yalnız yeni bloğun sabit olmayan özniteliklerininkiler, yerlerden yalnız taşınabilirlerinki kalır (`dropped`,
    `places` sayılır);
  - yeni bloğun alanları doldurulur.
- **Seçim:** `uids` verilirse yalnız onlar değişir.
- **`purge`:** artık hiçbir şey yerleştirmiyorsa eski blok silinir.
- **Sınır:** yalnız çizimin kendi yerleştirmeleri değişir; eski bloğu yerleştiren tanım onu yerleştirmeyi sürdürür.
- **Retler, sırasıyla:** `no_block`, `same_block`, beklenen sürüm, `unknown_block`, `entity_not_found`, `not_insert`, bütünü
  kilitliyse `layer_locked` (bir kısmı kilitliyse uyarı), `invalid_field`.

#### `cad.blocks.query` (BCOUNT, Blok çizelgesi)

Salt okunurdur; plan aynı cevabı verir. Üç rapor vardır:

- **`count`:** her bloğun çizimdeki yerleştirmeleri katman katman, ve başka blokların yerleştirmelerinin tanımları içinden kaç kez
  yerleştirildiği (`nested`, iç içe çarpılarak).
- **`instances`:** koşullara uyan yerleştirmelerin kalıcı kimlikleri (seçim için).
- **`schedule`:** aynı yerleştirmelerin satırları.
  - Sütunlar Blok, Katman, X, Y, Dönüş (derece) ve istenen etiketler. Etiket istenmezse blokların etiketleri sırasıyla, birer kez.
  - Hücre sayı ya da metindir.
- **Süzme:** `blocks` ve `layers` ile; `where` koşullarıyla (etiket, işleç, değer).
  - İşleçler `eq`, `ne`, `contains` (Türkçe küçük harfe katlanmış metin), `empty`, `notEmpty`, `lt`, `gt` (sayı; sayı olmayan değer
    uymaz).
  - Yerleştirmenin değeri: sabitse tanımınki, yoksa kendi değeri, yoksa varsayılan.
- **Retler:** `unknown_block`, `layer_not_found`, `invalid_condition`, `unknown_tag`.

#### Masaüstü

- **Blok çizelgesi penceresi** (`block_schedule.rs`; `block.schedule`, Ekle › Blok; ya da Bloklar panelinde Say ve çizelgele…):
  - üstte sayılar;
  - bir koşul (etiket, işleç, değer) ve Bul;
  - çizelge; çok satırlı değer pencerede ` / ` ile tek satır;
  - Seç, Panoya kopyala, Dışa aktar ▾ (XLSX, ODS, CSV; `table_write`).
- **Bloklar panelinde bloğun menüsü:** Öznitelikleri eşitle (`block.sync`), Alanları güncelle, Başka blokla değiştir ▸, Kart…
  (`block.card`), Say ve çizelgele….
- **Blok öznitelikleri penceresi:** her satırın seçenekleri, çok satırlının kutu genişliği ve alan şablonu.
- **Öznitelik değerleri** (Blok ekle'nin sorusu):
  - sabit, ön ayarlı ve alan olanları sormaz; hiçbiri kalmazsa açılmaz;
  - doğrulananı ikinci bir alanla sorar, ikisi aynı olana dek Yerleştir kapalıdır;
  - çok satırlıyı satır kutusuyla sorar.

### 4. Kütüphane kartları (`CAD-BLOCK-12`)

- **Bloğun kartı** (`BlockDefinition.card`, `BlockCard`):
  - kategori: `/` ile yol, ör. `Mimari/Kapılar`;
  - etiketler: her biri bir kez, Türkçe katlanarak;
  - sürüm: kart yazılınca 1, tanımın çizdiği her değişiklikte bir artar.
- **Kartın açıklaması** bloğun var olan `description`'ıdır.
- **Kartın gerisi kaynaktan gelir:** küçük resim, boyu, birimi (ADR 0315 §4), öznitelikleri ve yerleştirme sayısı. Kartta ayrıca
  saklanmaz.
- **Yazma:** `cad.blocks.edit`'in `card` işlemi (`category`, `tags`, `description`); “Blok değiştir” adımı. Kuralları
  `card_problem`: boş bölümlü kategori ve yinelenen etiket `invalid_block_card`.
- **Masaüstünde Blok kartı penceresi** (`block_card.rs`): kategori, etiketler, açıklama; yanında sürüm, boy, birim, öznitelikler.
- **Bloklar paneli:**
  - Liste ve Kartlar düğmeleri;
  - kategori seçicisi (alt kategorileriyle);
  - kart görünümünde iki sütun: büyük resim, ad, kategori, sürüm ve yerleştirme sayısı; ipucunda boy ve birim, etiketler,
    öznitelikler.
  - Arama adın, açıklamanın, kategorinin, etiketlerin ve öznitelik etiketlerinin içinde bulur.
- **Parametreler** dinamik blokla gelir (`CAD-BLOCK-14`…`18`).

## Sonuçlar

- Öznitelikler AutoCAD'in seçeneklerini ve alanları taşır. Tanımı değişen bloğun yerleştirmeleri aynı adımda tanıma uyar ve
  rapor edilir.
- Python ve ajan blokları sayar, değere göre bulur, çizelgeler ve değiştirir; masaüstünün penceresi aynı komutları kullanır.
- Büyük kitaplıkta bloklar kategori ve etiketle bulunur.
- **Sınırlar:**
  - Taşınabilir özniteliğin yeri tutamaçla taşınmaz; yeri `place` işlemi ya da Python ve ajan verir.
  - DXF'e seçeneklerin 1, 2, 4 ve 8 bitleri yazılır. Konum kilidi (16) yazılmaz; başka programda bütün öznitelikler taşınabilir
    açılır. Görünmez özniteliğin değeri ATTRIB olarak yazılmaz. DXF okurken sabit ATTDEF yazı, görünmez ATTDEF alınmaz kalır
    (ADR 0144'teki gibi).
  - Alanın şablonu yalnız yerleştirmenin değerlerini ve projenin `@` değerlerini okur; nesnenin geometrisi (alan, uzunluk) yoktur.
  - Çok satırlı özniteliğin değeri tablonun Varsayılan hücresinde ilk satırıyla görünür.
  - Web ertelendi.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `block_attribute_cases.py`: `cad.blocks.edit`'in seçenek, eşitleme, alan, yer ve kart durumları (21), `cad.blocks.replace` (22),
    `cad.blocks.query` (20).
  - `blocks_command_cases.py`: var olan durumlar eşitleme raporuyla.
  - `block_insert_cases.py`: sabit ve alan öznitelikleri.
  - `set_command_cases.py`: sabit özniteliğe değer reddi.
  - `kcad_v2_reference.py`: şema 58, `block-cards`, bozuk dosyalar.
- **Çekirdek ve sözleşme:**
  - `kentos-geometry-core`'un `block_attrs` testi: yerleştirmenin ve Patlat'ın yazıları seçeneklere göre.
  - `kentos-formats`'ın `an_attribute_s_options_are_its_flags`'i: ATTDEF'in 70'i.
  - `capabilities.rs`: her yeni kod bir durumda.
- **Masaüstü resimleri:**
  `KENTOS_SHOTS_ONLY=blok-oznitelik-secenekleri,blok-oznitelik-degerleri,blok-karti,blok-cizelgesi,blok-kartlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-blok-*`).
