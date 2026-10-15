# ADR 0312: Çizim standardı, görünüşün kaynağı ve kâğıdın renkleri

- **Durum:** kabul edildi (2026-10-15). `plan/cad.md` §9 `CAD-STYLE-03`, `06`, `07`, `16`. Sahibin isteği: “cad-style maddelerini
  bitir ama her maddeyi ayrı ayrı yapmak yerine 3 veya 4'er yaptıktan sonra test et”. Bu dört madde ikinci gruptur. Kapsamı ben
  belirledim.
- **Bağlam belgesi:** ADR 0311 (katmanları bulma ve çevirme), 0260 (nesnenin görünüşü, `byLayer`), 0144 (blok), 0183 (yazı ve ölçü
  stilleri), 0164 ve 0252 (pafta ve PDF), 0306 (Denetle), CLAUDE.md §9.2, DESIGN.md §9a.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Standart yok:** bir ofisin katman adları, renkleri, kalınlıkları ve yazı tipleri bir yerde yazılı değildi. Ne denetlenebiliyordu
  ne de bir çizime uygulanabiliyordu (AutoCAD'in CAD Standards'ı ve `.dws` dosyası).
- **Görünüşün kaynağı dağınıktı:** bloğun nesnesi renk ve kalınlığı yerleştirmeden alıyordu, ama çizgi tipini ve saydamlığı
  almıyordu. Kesikli çizilmiş bir kapı bloğu düz çiziliyordu. DXF yazıcısı tanımdaki değersiz nesnenin çizgi tipini ve saydamlığını
  BYBLOCK yazmıyordu; AutoCAD onları BYLAYER okuyordu. Okuyucu da tanımdaki BYLAYER çizgi tipini yerleştirmenin sayıyordu.
- **Kâğıdın renkleri üç yerdeydi:** paftanın paleti (`#111111` mürekkep), pafta girdilerinin lejant sembolleri (`fg-dim` ve `paper`
  siyah) ve DXF'in ACI tablosu ayrı ayrı tutuluyordu. Aynı paftada haritanın `fg-dim`'i gri, lejantınki siyahtı; `paper` (maske) lejantta
  siyah basılıyordu.

## Karar

### 1. Çizim standardı veri olarak

`DrawingStandard` (`kentos_contracts::cad_standards`):

- **Katmanlar:** her biri adı, açıklaması, rengi, çizgi tipi, kalınlığı, Yazdır'ı ve topladığı adlar (`from`: `cad.layers.find`'ın
  örüntüsü, virgülle seçenekler).
- **Ad örüntüleri (`naming`):** öbür katmanların alabileceği adlar (`A-*`); boşsa yalnız standardın adları.
- **Yazı tipleri:** yazıların yazı tipleri; boşsa serbest. İlki düzeltmenin verdiğidir.
- **Yazı ve ölçü stilleri:** projede olması gerekenler; liste doluysa başkası olmaz.
- **Çizgi tipleri:** katmanların ve nesnelerin alabilecekleri; boşsa serbest.
- **Katmana göre (`byLayer`):** nesneler rengini, çizgi tipini ve kalınlığını yalnız katmanından alır.

Yerleşik standartlar `catalog/v1/standards/` altında veridir; sözleşme onları derlemeye alır (`built_in_standard`). Komutlara
standart adıyla (`name: "mimari-ofis"`) ya da kendisiyle (`standard`) verilir. Geçersiz standart `invalid_standard`, yerleşiklerde
olmayan ad `standard_not_found`, hiçbiri `no_standard`.

### 2. Standardı denetle (`CAD-STYLE-16`)

Salt okunur `cad.standards.check`. Bulgular (`StandardIssue`: tür, konu, katman, beklenen, bulunan, nesne sayısı, düzeltilir mi,
ileti) şu sırayla gelir:

1. **Standardın katmanları,** standardın sırasıyla: eksik katman; rengi (harf büyüklüğüne bakmadan), çizgi tipi, kalınlığı ve
   Yazdır'ı farklı katman.
2. **Çizimin öbür katmanları,** ağacın sırasıyla:
   - Bir standart katmanın topladığı ada uyan katman: o katmana birleşir. Kilitliyse (kendisi ya da grubu) düzeltilemez.
   - Ad örüntülerine de uymayan katman: düzeltilemez, adı söylenir.
   - İzinli olmayan çizgi tipli katman: düzeltilemez.
   - Servis katmanları denetlenmez.
3. **Yazı tipleri ve stiller:**
   - Projenin yazı tipi standartta yoksa bulgudur.
   - Standardın adlı stilinin yazı tipi standardınki olmalıdır; öbür stillerin yazı tipi izinli olmalıdır.
   - Eksik yazı ve ölçü stilleri bulgudur; standartta olmayan stiller düzeltilemez.
4. **Nesneler:**
   - İzinli olmayan kendi çizgi tipiyle kaç nesne olduğu.
   - Standart katmana göreyse, kendi rengi, çizgi tipi ya da kalınlığı olan nesnelerin sayısı. Düzeltmesi Özellik kopyala ya da
     Öznitelikler'in “Katmana göre”sidir; bu komut nesnelerin görünüşüne dokunmaz.

“Plot profile”: KentOS'ta yazdırmanın profili katmanın Yazdır'ı ve kalınlığıdır (kâğıttaki kalem). Standardın katmanları ikisini de
taşır; ayrı bir CTB/STB tablosu yoktur.

### 3. Standardı uygula: ayrı, planlı düzeltme (`CAD-STYLE-16`, `03`)

`cad.standards.apply`. Dört seçeneği vardır, hepsi açık başlar: `layers`, `restyle`, `rename`, `styles`.

1. **Birleştirme (`rename`):** standart katmanlara ait eski katmanlar Katmanları çevir'in yazmasıyla birleşir (ADR 0311 §5). Kurallar
   standardın `from`'larıdır; standart katman yoksa standardın görünüşüyle açılır.
2. **Katmanlar:**
   - Eksik standart katmanlar standardın sırasıyla `parent` grubun sonuna eklenir (`layers`). Yazdırılmayanın Yazdır'ı kapalıdır.
   - Var olanlar standardın rengini, çizgi tipini, kalınlığını ve Yazdır'ını alır (`restyle`). Birleşmede açılan katman, `restyle`
     kapalıyken de alır.
   - Birinci ve ikinci adım tek geri alma adımıdır: “Standardı uygula”. Yazdır, kilit gibi, geri alma adımı değildir (ADR 0260
     §4): geri alınınca katmanlar döner, Yazdır uygulandığı gibi kalır.
3. **Stiller ve yazı tipi (`styles`):** proje ayarı olarak yazılır (ADR 0183): projenin yazı tipi, stillerin yazı tipleri ve eksik
   stiller. Yeni stiller UUIDv7 kimlik alır; cevap stilleri adlarıyla söyler.

Planı, kopya üstünde aynı yazmadır; önizleme odur. Kilitli eski katman Katmanları çevir'in retlerini verir, yolu `standard`'dır.

### 4. Görünüşün kaynağı: tek kural (`CAD-STYLE-06`)

`kentos_contracts::resolve_look`. Bir nesnenin her parçası (renk, çizgi tipi, desen ölçeği, kalınlık, saydamlık) şuradan gelir:

- **Kendi değeri** varsa ondan (açık değer).
- **Blokta ve parça `byLayer`'daysa:** nesnenin kendi katmanından (AutoCAD'in bloktaki BYLAYER'ı).
- **Blokta, öbür parçalar:** yerleştirmeden, yerleştirmenin çizildiği gibi (BYBLOCK). İç içe bloklar sırayla.
- **Çizimde:** katmanından. Katmanın saydamlığı yoktur (0), desen ölçeği 1'dir.

Kural her yerde aynıdır:

- **Çizim alanı ve pafta:**
  - Blok parçası (`block::Piece`) artık çizgi tipini, ölçeğini ve saydamlığını da taşır (`PieceStroke`); iç içe yerleştirmenin
    değeri BYBLOCK olarak geçer.
  - Stil programı parçayı kuralın çözdüğü görünüşle çizer; yerleştirmenin kendi çizgi tipi ve saydamlığı parçalarına geçer.
  - **Hata (ADR 0260'ın eki):** masaüstü blok tanımlarını depoya `core_entity` ile veriyordu ve o nesnenin görünüşünü (`byLayer`,
    çizgi tipi, ölçek, saydamlık) geçirmiyordu. Bloktaki “katmana göre” masaüstünde hiç çalışmıyordu. Düzeldi.
- **DXF yazıcısı:** tanımdaki değersiz nesnenin çizgi tipi `6 ByBlock`, saydamlığı `440 0x01000000` (BYBLOCK) yazılır; `byLayer`
  parçası BYLAYER kalır.
- **DXF okuyucusu:** tanımdaki BYLAYER çizgi tipi, 0'dan başka katmanda o katmanın çizgi tipidir; renk ve kalınlıkta olduğu gibi.

Ortak durumlar `fixtures/look/v1/cases.json`'dadır (bağımsız başvuru `look_cases.py`).

### 5. Kâğıdın renkleri: tek tablo (`CAD-STYLE-07`)

Tema renkleri (`fg`, `fg-dim`, `ink`, `paper`, `label`) ekranda arayüzün temasıyla çizilir; kâğıtta ve dosyada asla:

- **Kâğıtta:** `kentos_contracts::print_color`. Pafta, PDF ve lejant aynı tabloyu kullanır: kâğıt beyaz, `ink` siyah, `fg` ve
  `label` #111111, `fg-dim` #555555. Gerçek renk olduğu gibi kalır. Paftanın `PaperPalette`'i ve pafta girdilerinin lejant renkleri
  bu tablodandır. Lejantta `fg-dim`'in ve `paper`'ın siyah basılması düzeldi.
- **DXF'te:** CAD paleti (ACI) değişmedi: `ink` ve `fg` 7, `fg-dim` 8, `paper` 255; gerçek renk en yakın ACI ve 420 truecolor.

Hiçbir çıktı arayüzün temasını okumaz: pafta kâğıdın paletiyle (`paper_style`), DXF yazıcısı ACI tablosuyla çalışır; ikisi de
temayı almaz. Ortak durumlar `fixtures/look/v1/print.json`'dadır (bağımsız başvuru `print_color_cases.py`). Sözleşme kâğıdın
renklerini, DXF yazıcısı ACI'leri bu dosyayla denetler.

### 6. Mimari ofis profili (`CAD-STYLE-03`)

Yerleşik `mimari-ofis` standardının içeriği:

- **On katman:**
  - A-DUVAR, A-KAPI, A-PENCERE, A-MOBİLYA;
  - A-EKSEN (noktalı kesik), A-ÖLÇÜ, A-YAZI, A-TARAMA, A-REVİZYON;
  - A-YARDIMCI (kesikli, yazdırılmaz).
  - Her biri rengi, kalınlığı ve topladığı eski adlarla gelir (Türkçe ve İngilizce, AIA'nın `A-WALL` gibi adları).
- **Ad örüntüsü:** `A-*`.
- **Yazı tipleri:** Barlow, Arimo.
- **Stiller:** A-Yazı, A-Başlık ve A-Ölçü.
- **Çizgi tipleri:** sürekli, kesikli, noktalı kesik.
- **Katmana göre:** açık.

Boş bir CAD çizimine Standardı uygula on katmanı, iki yazı ve bir ölçü stilini ekler ve yazı tipini Barlow yapar.

### 7. Arayüz

- **Yer:** CAD'de Yönet sekmesinde, CBS'de Veri sekmesinde yeni Standart paneli: Denetle ve Uygula
  (`catalog/v1/additions.json`; ikonlar büyüteçli sayfa ve onay işaretli katman yığını).
- **Çizim standardı penceresi** (`drawing_standard.rs`; iki komut da onu açar):
  - standardın adı ve açıklaması, bulguların sayısı;
  - bulgular tablosu: Tür, Konu, “Çizimde → standartta” ve düzelip düzelmediği (✓ ya da uyarı);
  - satıra basınca tam iletisi tablonun altında görünür, katmanı Katmanlar'da seçilir;
  - dört onay kutusu 2 × 2; önizleme planın sayılarıyla; Uygula yazar, pencere bulguları yeniden okur.
  - Pencere 1100 × 650'de bütünüyle sığar.
- **Komut adları:** `standards.check` (CHECKSTANDARDS), `standards.apply` (STANDARDS).

### 8. Python ve MCP

Komutlar katalogdan gelir: `kentos.cad.standards.check`, `.apply`. MCP'de `cad.standards.apply` yıkıcı listededir.

## Doğrulama

- **Ortak durumlar:**
  - `cad.standards.check` 25 ve `cad.standards.apply` 23 durum (`standards_cases.py`);
  - görünüşün kaynağı 22 durum (`look_cases.py`);
  - kâğıdın renkleri 16 durum (`print_color_cases.py`).
- **Testler:**
  - blok parçası yerleştirmenin kesikli çizgisini ve saydamlığını alır, katmana göre parçası kendi katmanınınkini
    (`style::scene_tests`);
  - DXF yazıcısının BYBLOCK grupları: kayıtlı örnekler ve bağımsız denetim (`dxf_write_reference.py`);
  - DXF okuyucusunda tanımdaki BYLAYER çizgi tipi (`a_definitions_by_layer_line_type_…`);
  - lejantın tema renkleri haritanınkiyle aynı (`kentos-plot`).
- **Resimler:** `standart-bulgular`, `standart-uygulandi`, `standart-serit`, `standart-blok` (`standard_scenes.rs`).
- **Süreler** (release, 500 katman ve 100 000 nesne; katmanların dörtte beşi eski adlı, nesnelerin yedide biri kendi renkli;
  `standards_cmd::timing`):

| İş | Ölçü | Süre |
|---|---|---|
| Standardı denetle (Mimari ofis) | 414 bulgu | 8,5 ms |
| Standardı uygula, plan | 400 katman birleşir, 80 000 nesne, 10 katman eklenir | 269 ms |
| Standardı uygula, yazma | aynı iş | 268 ms |
