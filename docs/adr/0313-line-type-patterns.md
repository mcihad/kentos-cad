# ADR 0313: Çizgi tipleri: LIN desenleri, ölçek ve faz

- **Durum:** kabul edildi (2026-10-15). `plan/cad.md` §9 `CAD-STYLE-08`, `09`, `10`, `11`. Sahibin isteği: “cad-style maddelerini
  bitir ama her maddeyi ayrı ayrı yapmak yerine 3 veya 4'er yaptıktan sonra test et”. Bu dört madde üçüncü gruptur. Kapsamı ben
  belirledim.
- **Bağlam belgesi:** ADR 0260 (nesnenin görünüşü: `lineType`, `lineTypeScale`), 0312 (görünüşün kaynağı), 0183 (yazı ve ölçü
  stilleri, proje ayarı olan tablolar), 0157 (stilli çizimin karoları), 0164 ve 0252 (pafta ve PDF), 0025 (KCAD v2), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Yalnız dört çizgi tipi vardı:** Sürekli, Kesikli, Noktalı kesik ve Noktalı (`LineType`). Kesiklerin boyları sabitti (kâğıtta
  3 + 1,5 mm gibi). Bir ofisin kendi tipleri (yol ekseni, parsel sınırı, doğalgaz, çit) taşınamıyordu.
- **LIN dosyası okunmuyordu:** AutoCAD'in ve onu izleyen yazılımların tipleri `.lin` dosyasındadır. DXF'in LTYPE tablosu da aynı
  deseni taşır; okuyucu yalnız adı alıp en yakın yerleşik tipe çeviriyordu.
- **Ölçeklerin ilişkisi yazılı değildi:** nesnenin kendi ölçeği (`lineTypeScale`, CELTSCALE) vardı; projenin genel ölçeği (LTSCALE)
  yoktu. Çizim birimindeki bir desenin kâğıttaki boyu tanımsızdı.
- **Faz:** kesikli bir çoklu çizginin deseni her köşede baştan mı başlıyor, yoksa sürüyor mu? Kodda sürüyordu, ama bunu söyleyen
  test ve belge yoktu.

## Karar

### 1. Çizgi tipi veri olarak (`CAD-STYLE-08`)

`LineTypeDef` (`kentos_contracts::line_types`):

- **Ad ve açıklama:** ad büyük küçük harf ve baştaki, sondaki boşluk gözetmeden eşsizdir (`same_line_type_name`).
- **Öğeler (`elements`):** en çok 12 (LIN'in sınırı). Her öğe:
  - çizgi (artı uzunluk), boşluk (eksi), nokta (0);
  - ya da karmaşık öğe: yazı (`text`: metin, yazı stili) ya da şekil (`shape`: ad, SHX dosyası); ikisinin de yeri (`S=` ölçek,
    `R=`/`A=`/`U=` dönüş ve mutlaklığı, `X=`, `Y=` kayma). Karmaşık öğenin kendi uzunluğu 0'dır; deseni ondan sonraki boşluk sürdürür.
- **Birim (`unit`):** `paper` (kâğıtta mm; KentOS'un kendi tipleri, her ölçekte kâğıtta aynı) ya da `drawing` (çizimin kendi
  uzunlukları, projenin biriminde; LIN'den ve DXF'ten gelenler).
- **Kaynak (`source`):** geldiği dosyanın adı (`ornek.lin`, `DXF`).

Proje tipleri ayarlarında tutar (`lineTypes`); tablo bir proje ayarıdır, yazı stilleri gibi: değişince çizim kaydedilmemiş olur,
geri alma adımı değildir. Katmanın görünüşü ve nesnenin görünüşü bir tipi adıyla anar (`linePattern`). Yanlarında en yakın yerleşik
tip `lineType`'ta kalır (`nearest`: boşluksuzsa Sürekli, yalnız noktalıysa Noktalı, noktası varsa Noktalı kesik, yoksa Kesikli).
Deseni okumayan bir okuyucu onu çizer.

**Ad neden `linePattern`:** taramanın kendi `pattern`'ı vardır (ADR 0186). Aynı JSON anahtarı iki anlam taşımasın diye.

**KCAD şema 56** (`FORMATS_VERSION` 65):

- ayarlarda `lineTypes` (opak liste; boşsa yazılmaz, boş liste ve aynı adlı iki tip reddedilir) ve `lineTypeScale` (0'dan büyük, en
  çok 1000; 1 yazılmaz, 1 yazan dosya reddedilir);
- katman stilinde, katman durumlarının düğümlerinde, nesnede ve blok nesnesinde `linePattern`;
- sütunlarda görünüşün `LOOK_PATTERN` biti ve metin sütunu.

Şema 55 yazan bir dosyada bunlardan biri varsa dosya reddedilir. Bağımsız Python okuyucusu ve yazıcısı aynı kuralları uygular.

**Bulut:** veritabanı projesinde nesnenin deseni `feature.look` sütunundadır (ADR 0260'ın sütunu; göç gerekmez). Katmanın deseni
katman ağacında, çizgi tipleri ve ölçek proje ayarlarında `ProjectPatch` ile gider. Taşıma matrisi (`carriage.rs`) bunu denetler.

### 2. LIN dosyası ve komut

**Okuyucu ve yazıcı** (`kentos_formats::lin`):

- **Başlık:** `*AD[,açıklama]`. Ardından `A,` ve en çok on iki öğe. `;;` yorumdur.
- **Okunmayan satır söylenir, gerisi okunur:**
  - başlıksız desen;
  - A ile başlamayan desen (B, C gibi hizalamalar AutoCAD'de de yalnız A'dır);
  - kapanmayan köşeli parantez ya da tırnak;
  - sayı olmayan uzunluk;
  - on ikiden çok öğe;
  - boşlukla başlayan desen;
  - desensiz başlık;
  - iki kez geçen ad (ilki alınır).
- **Yazıcı:** en kısa, geri okunan biçimde yazar (`.5`, `-.25`).

**`cad.linetypes.edit`** v1 (`line_types_edit.rs`; catalog, Python `kentos.cad.linetypes.edit`, MCP'de yıkıcı listede):

| İşlem | Ne yapar |
|---|---|
| `load` | LIN metninin tiplerini alır. `names` verilirse yalnız onları alır; dosyada olmayanlar `problems`'ta. Projede aynı adlı tip kalır (`skipped`); `replace` ile değişir. |
| `add` | Bir tipi ekler ya da değiştirir. |
| `remove` | Bir tipi kaldırır. Bir katman, katman durumu ya da nesne (bloklardakiler dahil) onunla çiziliyorsa `line_type_in_use`, sayılarıyla. |
| `scale` | Projenin çizgi tipi ölçeğini (LTSCALE) yazar; 1 ölçeği kaldırır. |
| `export` | Tipleri (ya da `names`'i) LIN metni olarak verir; bir şey yazmaz. |

- **Ret kodları:** `no_lin`, `no_definition`, `invalid_line_type`, `empty_name`, `line_type_not_found`, `line_type_in_use`,
  `invalid_line_type_scale`, revizyon.
- **Kullanan komutlar:** `cad.layers.edit` ve `cad.entities.set` `linePattern` alır, olmayan adı `line_type_not_found` ile reddeder.
  - Yalnız `linePattern` verilirse `lineType` en yakın yerleşik tip olur.
  - Yalnız yerleşik `lineType` verilirse desen kalkar.
  - `null` deseni kaldırır, `lineType`'a dokunmaz.
- **Özellik kopyala** deseni çizgi tipiyle birlikte taşır.

### 3. Karmaşık tipler ve SHX: karar (`CAD-STYLE-09`)

- **Saklanır:** yazı ve şekil öğeleri olduğu gibi saklanır. KCAD'e, LIN'e ve DXF'e (yazı öğesi) geri yazılır; kayıp yoktur.
- **Çizilmez:**
  - Yazı öğesinin yazısı ve şeklin SHX çizimi çizilmez. Yerleri desende boşluk olarak kalır.
  - SHX dosyalarını okumuyoruz: AutoCAD'in derlenmiş şekil biçimidir, dosyası çoğu zaman yanında gelmez, yazı tipi değildir.
    Çizdirmek ayrı bir iştir (TODOS.md).
- **Söylenir:**
  - Çizgi tipleri penceresi tipin yanında “yazı ve şekil: boşluk” der.
  - DXF yazıcısı şekil öğesini yazmaz ve raporunda “Çizgi tipi” satırıyla söyler. Yazı öğesi önceki öğeye 74/75/340/46/50/44/45/9
    gruplarıyla bağlanır; stilin tutamacı KentOS'un yazı stilidir.
  - DXF okuyucusu yazıyı (74'ün 2. biti) ve şekli (4. bit; adı numarası, dosyası stilin tutamacı) okur.

### 4. Ölçeklerin ilişkisi (`CAD-STYLE-10`)

Bir öğenin çizimdeki boyu (`pattern_length_m`):

```text
kâğıt deseni:  uzunluk (mm) × çizim ölçeği / 1000
çizim deseni:  uzunluk × projenin biriminin metreye oranı
         × projenin çizgi tipi ölçeği (LTSCALE) × nesnenin çizgi tipi ölçeği
```

- **Çizim hattında:** desen kâğıtta mm'ye çevrilir (`kentos_native_style::line_types::dashes_of`; çizim deseni çizim ölçeğiyle) ve
  LTSCALE'le çarpılır. Nesnenin kendi ölçeğini ADR 0260'ın kuralı ekler.
- **Yakınlaştırma:** kesiklerin kâğıttaki boyunu değiştirmez.
- **Pafta:** haritalar sembolleri kâğıtta mm çizer. Paftadaki her harita görünümünde kesik kâğıtta aynı boydadır; AutoCAD'in
  PSLTSCALE = 1'i gibi.
- **Çoklu çizginin deseni:** bütün yol boyunca sürer (§5). AutoCAD'in “polyline linetype generation”ı açıktır. DXF'e
  LWPOLYLINE 70'in 128 bitiyle (PLINEGEN) yazılır.
- **DXF:** `$LTSCALE` iki yönde. İçe alınan dosyanınki projede ölçek yoksa alınır.

### 5. Faz ve köşeler (`CAD-STYLE-11`)

- **Uzaklık:** stil çekirdeği bir yolun her parçasına yolun başından o parçaya kadarki uzaklığı yazar (parçanın altı sayısının
  beşincisi; `BatchSink::stroke`). Kapalı yolun kapatan kenarı son kenardan sürer.
- **Çizim hatları:** masaüstünün wgpu'su (`stroke.wgsl`'in `dashCover(s0 + x, …)`'u), web'in WebGPU ve WebGL2'si ve paftanın
  PDF'i aynı toplulukları aynı uzaklıkla okur. Bir köşede kesilen kesik köşeden sonra sürer; desen köşede baştan başlamaz.
- **Uçlar:** başlık yalnız yolun kendi uçlarına konur. Ara köşeler yuvarlak birleşir.
- **Yaylar:** kirişlerine bölünmüş yaylar aynı kuralla sürer.
- **Sınır:**
  - Bütün çizim hatlarında bir desen en çok 8 sayı (dört çizgi ve boşluk çifti) çizer: gölgelendiricinin iki `vec4`'ü.
  - Daha uzun desen kısaltılır ve pencerede “ilk dört çizgi ve boşluk çizilir” denir.
  - Desenin kendisi saklanır.

### 6. Kaynak: katman, nesne, blok

ADR 0312'nin kuralı deseni de kapsar:

- nesnenin kendi deseni;
- blokta katmana göre parçanın katmanınınki, yoksa yerleştirmeninki (BYBLOCK, iç içe yerleştirmelerden);
- çizimde katmanınki.

Kendi yerleşik tipi olan nesne katmanın desenini almaz.

### 7. Arayüz

- **Çizgi tipleri penceresi** (`line_types_window.rs`, `lineTypes.manage`; LINETYPE, ÇİZGİTİPİ):
  - projenin tipleri: ad, açıklama, birim, kaynak, not ve desenin çizimi;
  - LIN'den yükle…, Dışa aktar…, Sil;
  - Çizgi tipi ölçeği (LTSCALE) ve Uygula;
  - yüklemenin sonucu tek cümle: eklenen, atlanan, okunmayan satır ve ilki.
  - Satırlar tek satırlık, uzun ad ve dosya adı üç noktayla kısalır. Pencere 1100 × 650'de sığar.
- **Şerit:** CAD'de Yönet'in, CBS'de Veri'nin Standart panelinde (`catalog/v1/additions.json`).
- **Öznitelikler:** Çizgi tipi satırı projenin desenlerini de listeler.
- **Katmanlar:** sağ tıkın Çizgi tipi ▸ menüsü projenin desenlerini ve “Çizgi tipleri…”yi listeler.

## Doğrulama

- **Ortak durumlar:**
  - LIN okuma ve yazma 21, ölçek 13 durum: `fixtures/linetypes/v1/cases.json`. `line_type_cases.py` KentOS kodu olmadan yazılmıştır;
    Rust okuyucusu ve yazıcısı bit bit aynı.
  - `cad.linetypes.edit` 34 durum.
  - `cad.entities.set`'e 2, `cad.layers.*`'e 4 durum.
- **KCAD:** `patterns.json` ve `.kcad` örneği, beş bozuk dosya; `schema-version-56` artık geçerli olduğu için `57` oldu. Bağımsız
  Python okuyucusu ve yazıcısıyla denetlenir.
- **DXF:**
  - desenli tiplerin gidip gelmesi (`line_types_with_patterns_go_out_and_come_back`);
  - kayıtlı örnekler PLINEGEN'le yeniden yazıldı, bağımsız denetim (`dxf_write_reference.py`) PLINEGEN'i de arar.
- **Testler:**
  - fazın yol boyunca sürmesi, kapalı yolda da (`a_pattern_runs_on_along_a_path_past_its_vertices`);
  - desenin kâğıt mm'si (`a_pattern_becomes_dashes_on_paper`).
- **Resimler:** `cizgitipi-pencere`, `cizgitipi-cizim`, `cizgitipi-oznitelik` (`line_type_scenes.rs`).
- **Süreler** (release, 100 000 çizgi, 1:500, LTSCALE 0,5; `line_type_timing`):

| İş | Süre |
|---|---|
| Projenin çizgi tiplerinin okunması | 0,011 ms |
| Yerleşik kesikli katman | 32,6 ms |
| LIN desenli katman (SINIR) | 33,5 ms |
| LIN desenli katman, onda biri kendi deseniyle | 35,3 ms |

LIN dosyasının okunma süresi ölçülmedi (on iki öğelik birkaç yüz tip, okuma tek geçiş).
