# ADR 0319: Dinamik bloğun tutamaçları, değerleri, vekil görünüşü ve DXF'i

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §10 `CAD-BLOCK-17`, `18`, `19`, `20`. Sahibin isteği: “CAD blocok ile
  devam et”; maddeler 3–4'erli gruplarla yapılıp sınanır. Bu dört madde beşinci ve son gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0144 (bloklar), 0280 (tutamaç kipleri);
  - 0318 (dinamik bloklar: model, eylemler, varyant, `cad.blocks.parameters`);
  - CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Değer yalnız komutla değişiyordu.** ADR 0318 dinamik bloğun modelini, varyantını ve `cad.blocks.parameters`'ı getirdi. Ama
  çizimde kapının genişliği tutamakla çekilemiyor, Öznitelikler parametreleri göstermiyordu.
- **Görünürlüğün kapsamı kanıtlanmamıştı.** Varyant açılım, kenet, seçim ve çizimde kullanılıyordu. Gizli parçanın tıklanmadığı
  ve kenetlenmediği bir testle gösterilmemişti.
- **Dinamik bloğu bilmeyen okuyucu yanlış çiziyordu.** DXF yazıcısı değiştirilmiş kapıyı tanımın varsayılanıyla yazıyordu. 1,2 m
  sol el kapı DXF'te 0,9 m sağ el çıkıyordu.
- **Bağımsız geometriye çevirmenin adı yoktu.** Patlat varyantı nesnelere açıyordu. Ama yerleştirmeyi düz bir blokta tutmanın
  yolu yoktu.

## Karar

### 1. Parametre tutamaçları (`CAD-BLOCK-17`)

- **Tutamaçlar** (çekirdeğin `block_dynamic::Dynamic::grips`'i):
  - seçili dinamik yerleştirmenin elle verilen her parametresi bir tutamaç alır;
  - doğrusal: çizgisi üstünde, uzunluğu kadar ötede;
  - kutupsal ve nokta: kendi noktasında;
  - açı: merkezinden yerleştirmenin büyük kenarının yarısı kadar ötede;
  - çevir: ekseninin ortasında.
- **Tutamaçsız olanlar:** görünürlük ve tablo listeden seçilir (§3). İfadeyle hesaplanan parametre verilmez.
- **Açık çevirmeler:** öbür tutamaçlar açık çevirmelerin aynasında durur. Sol el kapının genişlik tutamacı kanadın ucundadır.
  Çekilen nokta değere çevrilmeden önce aynadan geri alınır.
- **Yerleştirmenin dönüşümü:** tutamaçlar onunla çizime yerleşir (`Store::param_grips`). Çekilen nokta tanımın koordinatlarına
  döner (`Store::param_value`).
- **Değer** (`Dynamic::value_at`): kuralına uyar. Kuralın en yakın değeri `Rule::nearest`'tir:
  - listede en yakın (eşitlikte ilki);
  - yoksa `min` ve `max` içinde en yakın adım, dışarı taşan adım bir geri.
  - doğrusal: çizgiye iz düşümü (başlangıcında ya da gerisinde değer yok);
  - kutupsal: kuralın uzaklığında, o yönde;
  - açı: varsayılana en yakın tur;
  - nokta: noktanın kendisi.
- **Seçim aracı** (`kentos_interaction::param_grip`):
  - çekilen ya da tıklanıp sıcaklaşan tutamaç imleci izler;
  - önizleme yerleştirmenin o değerle çizimidir (`Store::varied_outlines`), kesikli. Yanında kesikli ölçü çizgisi ve değer;
  - yazılan uzunluk tutamacın yönünde, yazılan nokta noktadır;
  - bırakma, tıklama, Enter ya da hesaplanan nokta değeri `cad.blocks.parameters`'ın `set`'iyle yazar, tek adım;
  - çevir tutamacı tıklanınca çevirme döner. Esc tutamacı bırakır.
- **Görünüş:** AutoCAD'deki gibi biçimleriyle ayrılırlar:
  - doğrusal: çizgisi boyunca iki ok başı;
  - kutupsal: eşkenar dörtgen;
  - açı: halkalı daire;
  - nokta: daire;
  - çevir: yüz yüze iki ok başı.
  - renk seçim penceresinin mavisi, zeminin rengiyle haleli.

### 2. Görünürlüğün kapsamı (`CAD-BLOCK-17`)

- **Tek kaynak:** gizli parça açılımda, tıklamada, kenette ve çizimde yoktur. Hepsi deponun aynı varyantını okur (ADR 0318 §2).
- **Pafta:** deponun parçalarını çizer; aynı kapsamdır.
- **Kanıt:** çekirdek testi `a_hidden_piece_is_hidden_everywhere`.

### 3. Parametrik düzenleme arayüzü (`CAD-BLOCK-18`)

Öznitelikler'de tek dinamik yerleştirmenin “Blok parametreleri” bölümü:

- **Satırlar:** her parametre ve yerleştirmenin gösterdiği değer (`block_params::current`: tuttuğu, yoksa varsayılan; tablonun
  uyan satırı, yoksa —).
- **Düzenleme:**
  - doğrusal: sayı;
  - açı: derece;
  - kutupsal ve nokta: “doğu, kuzey”;
  - çevir: Evet/Hayır;
  - görünürlük ve tablo: listeden.
  - ifadeyle hesaplanan parametre gösterilir, düzenlenmez (“ifadeyle hesaplanır”).
- **Değerler ▾:** “n değer değişti” ve Hepsini varsayılana döndür (`reset`).
- **Aynı yol:** her yazma `cad.blocks.parameters`'tır: kural, ret sözleri ve geri alma adımı tutamakla, Python'la ve MCP'yle
  aynı. Kuralın reddi uyarı olarak söylenir.
- **Önizleme:** tutamağın önizlemesi aynı varyanttır.
- **Kilitli katman:** düzenleyici yoktur.

### 4. Ayrı yetenekler (`CAD-BLOCK-19`)

Dinamik blokla yapılan üç iş ayrı komutlardır, katalogda ayrı yetenekleriyle:

| İş | Komut | Yazar mı |
|---|---|---|
| Kaynak parametreli düzenleme | `cad.blocks.parameters` (yerleştirmenin değerleri), `cad.blocks.edit`'in `dynamic`'i (tanım) | Evet |
| Vekil görünüş | `cad.blocks.proxy` (yeni) | Hayır |
| Bağımsız geometriye çevirme | `cad.blocks.freeze` (yeni, Sabit bloğa çevir); `cad.entities.explode` (Patlat) | Evet |

#### `cad.blocks.proxy` v1

- **Ne verir:** yerleştirmelerin çizdiği nesneleri düz nesneler olarak.
  - dinamik yerleştirmede tanımın nesneleri, değerlerle sırayla eylemlerden geçmiş;
  - dizinin kopyaları sonda, yerel kimlikleri yeni;
  - görünürlük durumunun gizledikleri yok;
  - düz bloğun yerleştirmesinde tanımın kendisi.
- **Nesnelerin alanları:** her nesne geldiği nesnenin bütün alanlarını taşır (`with_shape`); yalnız geometrisi varyantınkidir.
  Çekirdek her nesnenin kaynağını verir (`vary_traced`).
- **Cevap:** `default`, `values`, `entities` (tanımın koordinatlarında). Hiçbir şey yazılmaz.
- **Retler:** `no_entities`, `invalid_uid`, `entity_not_found`, `not_insert`.

#### `cad.blocks.freeze` v1, tek adım “Sabit bloğa çevir”

- **Ne yapar:** her ayrı blok ve değerler için tek yeni düz blok kurar.
  - nesneleri vekil görünüşünkiler;
  - taban noktası, birimi, öznitelik tanımları, açıklaması ve kartı kaynağınki;
  - dinamik parçası yok.
- **Adı:** `name` (sonrakilere “ 2”, “ 3” …) ya da kaynağın adı ve “(sabit)”, “(sabit 2)” …, alınmış adlar atlanarak.
- **Yerleştirme:** yerini, ölçeğini, dönüşünü, katmanını, görünüşünü ve özniteliklerinin değerlerini korur, parametreleri kalkar.
- **Plan:** yeni blokların kimliği boştur (`00000000-…`); kimlik yazarken verilir.
- **Retler, sırasıyla:**
  - `no_entities`, `invalid_uid`;
  - boş ad `empty_name`;
  - `invalid_revision`, `revision_conflict`;
  - `entity_not_found`, `not_insert`, dinamik olmayan blok `not_dynamic`;
  - alınmış verilen ad `duplicate_block`;
  - kilitli katmandakiler atlanır, hepsi kilitliyse `layer_locked`.
- **Arayüz:** Ekle › Blok'ta Sabit bloğa çevir (`block.freeze`, ikonu kilitli blok), seçili yerleştirmelerle.

### 5. DXF'te vekil görünüş (`CAD-BLOCK-20`)

- **Değerli yerleştirme** (`block_variants::plain_for_export`, başsız sunucunun `dxf_input`'u):
  - dinamik bloğun değer tutan yerleştirmesi, çizdiğinin düz bloğuna yazılır;
  - ad: bloğun adı ve “V1”, “V2” …, alınmış adlar atlanır;
  - aynı blok ve aynı değerler tek bloktur;
  - açıklama değerleri söyler;
  - kimlik kaynağınınkinden türetilir: aynı çizim aynı dosyayı yazar.
- **Varsayılan değerdeki yerleştirme** bloğun kendisini yerleştirir.
- **Dinamik parça** (parametreler ve eylemler) DXF'e hiç yazılmaz. Rapor bunu blok başına söyler: “Dinamik blok: “Kapı”:
  parametreleri ve eylemleri DXF'e yazılmadı; …”. Söz `DxfWriteInput.notes` ile yazıcının raporuna gelir.
- **Okuma:** AutoCAD'in dinamik bloğu DXF'te adsız bir bloğun (`*U…`) yerleştirmesidir. Okuyucu onu eskisi gibi patlatarak,
  çizildiği hâliyle alır ve söyler.
- **Sınanmamış:** AutoCAD'in kendi dinamik blok verisi (`AcDbDynamicBlockGUID`, `ACAD_ENHANCEDBLOCK`, `AcDbBlockRepresentation`)
  gerçek bir dosya korpusuyla sınanmadı. KentOS bu yüzden “tam uyumlu” demez: ne yazarken ne okurken parametreler taşınır.
  Korpus sahibinden gelince sınanır (TODOS.md).

## Sonuçlar

- **Kapı tutamakla değişir.** Genişliği, açılışı, eli ve görünüşü çizimde, Öznitelikler'de, Python'da ve MCP'de aynı komutla
  değişir. Öznitelikleri korunur. Planın kabulü budur.
- **Çıktılar aynı görünüşte.** Çizim, pafta (PDF) ve DXF aynı varyantı çizer.
- **Sınırlar:**
  - nokta ve kutupsal tutamaçlar yalnız çevirmelerin aynasını izler. Taşıma ve döndürmeyle kayan bir parametrenin tutamacı
    tanımdaki yerindedir;
  - DXF'te dinamik blok taşınmaz; KentOS'ta yeniden açılan dosyanın blokları düzdür (§5);
  - tanımın içindeki iç içe dinamik yerleştirme DXF'e değerleriyle değil, varsayılanıyla gider;
  - görünürlük ve tablonun tutamacı yoktur (Öznitelikler'den seçilir).

## Doğrulama

- **Bağımsız başvuru** (`scripts/fixtures/dynamic_block_cases.py`, KentOS kodu olmadan):
  - varyantı ADR'den kendisi kurar: taşıma, ölçek, dizi, döndürme, ayna, görünürlük;
  - `fixtures/commands/v1/cad.blocks.proxy.json` (22 durum) ve `cad.blocks.freeze.json` (23 durum);
  - geometri `within: 1e-9` ile; yeni bloğun çizdiği `cad.blocks.proxy` adımıyla denetlenir.
- **Çekirdek:**
  - `store::param_grips`'in testleri: tutamaçların yeri, kuralın değeri, döndürülmüş ve aynalı yerleştirme;
  - önizleme ve gizli parçanın her yerde gizli oluşu.
- **Etkileşim:** `kentos-interaction`'ın `param_grips` testleri:
  - genişlik tutamacı çekilince en yakın adım yazılır, tek adım ve geri alınır;
  - tıklanan tutamaç sıcaklaşır, önizler ve yazılan uzunluğu alır;
  - çevir tutamacı döner, Esc bırakır.
- **DXF:** `kentos-headless`'in `a_varied_dynamic_insert_goes_to_dxf_as_the_block_it_draws`'u:
  - kitaplığın kapısı 1,2 m ve sol el, “Kapı V1” olarak yazılır, bir kez;
  - varsayılan kapı “Kapı”yı yerleştirir;
  - rapor söyler;
  - geri okunan blok 1,2 m sola açılır.
- **Yetenekler:** `capabilities.rs`, her kod bir durumda.
- **Masaüstü resimleri:**
  - `KENTOS_SHOTS_ONLY=blok-tutamac,blok-parametreler,blok-sabit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`;
  - `.run/shots/arac-blok-tutamac-*`, `arac-blok-parametreler-*`, `arac-blok-sabit-*`.
