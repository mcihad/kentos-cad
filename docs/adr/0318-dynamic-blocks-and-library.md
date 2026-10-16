# ADR 0318: Dinamik bloklar ve KentOS blok kitaplığı

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §10 `CAD-BLOCK-13`, `14`, `15`, `16`. Sahibin isteği: “CAD blocok ile
  devam et”; maddeler 3–4'erli gruplarla yapılıp sınanır. Bu dört madde dördüncü gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0144 (bloklar), 0314 (`kentos.blocks` dosyası), 0315 (yerel kimlik, birim);
  - 0316 (Blok düzenleyici), 0317 (öznitelikler, kartlar);
  - CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Bir blok tek boyda ve tek görünüşte çiziliyordu.** 70, 80, 90 ve 100'lük kapı dört ayrı bloktu. Sol ve sağ el, içe ve dışa
  açılış da öyle. AutoCAD'in dinamik bloğu (parametre ve eylem) yoktu.
- **KentOS'un kendi blokları yoktu.** Kullanıcı her projeye boş başlıyordu. Başka yazılımın kitaplığını dağıtmak da lisans
  sorunuydu.

## Karar

### 1. Model (`CAD-BLOCK-14`)

Tanımın `dynamic`'i (`kentos_contracts::block_dynamic`): parametreler ve sırasıyla uygulanan eylemler. Noktalar tanımın
koordinatlarındadır.

| Parametre | Değeri | Varsayılanı |
|---|---|---|
| `linear` (`from`, `to`) | `from`→`to` boyunca uzunluk | iki noktanın uzaklığı |
| `polar` (`from`, `to`) | `to`'nun yeni yeri (kural uzaklığa) | `to` |
| `angle` (`center`, `base`) | açı, radyan | `base` |
| `point` (`at`) | nokta | `at` |
| `flip` (`a`, `b`) | açık ya da kapalı: `a`→`b` doğrusunda ayna | kapalı |
| `visibility` (`states`) | durumun adı; her durum yerel kimlikleriyle nesne gizler | ilk durum |
| `lookup` (`rows`) | satırın adı; satır başka parametrelere değer verir | yok |

| Eylem | Parametre | Etkisi |
|---|---|---|
| `move` | doğrusal, kutupsal, nokta | nesneler parametrenin değişimi kadar taşınır |
| `stretch` (`min`, `max`) | doğrusal, kutupsal, nokta | kutudaki noktalar taşınır, öbürleri kalır |
| `scale` | doğrusal, kutupsal | `from` çevresinde değer / varsayılan oranında |
| `array` (`spacing`) | doğrusal | varsayılanın tuttuğunun ötesindeki her aralığa bir kopya |
| `rotate` | açı | merkez çevresinde değişim kadar |
| `flip` | çevir | açıkken doğruya göre ayna |

- **Sayının kuralı** (`NumberRule`): `min`, `max`, `step` (`min`'den, yoksa 0'dan adımlar) ve `list`. Karşılaştırma boyunun milyarda
  biri payla yapılır.
- **`expression`:** değeri öbür sayı parametrelerinden hesaplayan ifade (`@Genişlik / 2`, ifade dilinin `@` değerleriyle).
  - Böyle bir parametre verilmez.
  - İfadeler, sayıları kadar geçişle hesaplanır.
- **Kurallar** (`dynamic_problem`), sırasıyla:
  - en az bir parametre, en çok 256 parametre ve 256 eylem;
  - adlar boş değil, boşlukla başlayıp bitmez, Türkçe büyük küçük harfe bakılmadan bir kez;
  - noktalar sonlu, doğrunun iki noktası ayrı;
  - kural sonlu; en küçük en büyükten büyük değil; adım artı; ifadenin listesi ve adımı yok; varsayılan kurala uyar;
  - tek görünürlük; durum adları bir kez; gizlenen nesne blokta;
  - tablo satırları bir kez; satır yalnız tablo olmayan ve hesaplanmayan parametrelere, kurala uyan değer verir;
  - eylemin parametresi var ve türüne uyar; nesneleri blokta ve bir kez; esnetmenin kutusu düzgün; dizinin aralığı artı.
- **Eşit uygulama:** kurallar `.kcad` okuyucusunda, bağımsız Python okuyucusunda, iki belgede ve komutlarda aynıdır
  (`blocks::check`, `BadDynamic`). İfadeler komutlarda denetlenir (`expressions_problem`).
- **`.kcad` şema 59** (`FORMATS_VERSION` 68):
  - Tanımın `dynamic`'i sözleşmenin JSON'u olarak yazılır. Okuyucu onu geri okur ve bilinmeyen alanı (`unknown_field`), varsayılan
    değeri yazılmışı (`bad_value`) reddeder.
  - Yerleştirmenin `parameters`'ı: değerler `true`/`false`, float, metin ya da `[x, y]`.

### 2. Eylemler ve varyant (`CAD-BLOCK-15`)

- **Saklama:** yerleştirme yalnız varsayılandan farklı değerleri tutar (`InsertEntity.parameters`). Tablonun değeri tutulmaz;
  satırın verdiği değerler tutulur.
- **Varyant** (çekirdeğin `block_dynamic::Dynamic::vary`):
  - Tanımın nesneleri eylemlerin sırasıyla, her biri öncekinin bıraktığı üstünde değişir.
  - Ardından görünürlük durumunun gizledikleri çıkar.
  - Dizinin kopyaları sıradaki yerel kimlikleri alır.
  - Sonuç belirlidir: aynı değerler aynı varyantı verir.
- **Önbellek:** `Blocks` dinamik tanımları tutar. Varyant bir kez kurulur ve blok ve değerlerle (`Params::key`) önbelleğe
  alınır (en çok 4096). Varsayılan değerli yerleştirme tanımın kendisini çizer.
- **İç içe:** başka bir tanımın içindeki dinamik yerleştirme kendi değerlerinin varyantını çizer.
- **Tek kaynak:** açılım (`expand`), blok içi yol (`piece`, `path_in`), kutu, kenet, seçim, Patlat ve çizim aynı varyantı kullanır.
  Ev sahipleri değerleri çekirdeğe verir: `kentos_formats::blocks::core_params`, `core_dynamic`, çizim hattının sahnesi.
- **Dönüşümler değerleri taşır:** taşıma, döndürme, esnetme, tutamaç, iç içe yerleştirme.

### 3. Değerler ve doğrulama (`CAD-BLOCK-16`)

#### `cad.blocks.parameters`

Get, set ve reset; yazma tek adım “Blok parametreleri”.

- **Değerin çözümü** (`block_params::resolve`), sırasıyla:
  - ad bloğun (`unknown_parameter`) ve hesaplanmayan bir parametrenin (`derived_parameter`);
  - değer türüne ve kuralına uyar (`invalid_parameter_value`); tablonun satırı değerlerini yazar;
  - ifadeler hesaplanır ve sonuçları kurallarına uyar;
  - varsayılana eşit değerler düşer.
- **Cevap:** her yerleştirmenin bütün değerleri, varsayılanlarıyla. Tablonun değeri, değerleri tutan ilk satırın adıdır.
- **Retler, sırasıyla:**
  - girdi: `no_entities`, `invalid_uid`, boş değer (`invalid_parameter_value`);
  - beklenen sürüm;
  - yerleştirmeler: `entity_not_found`, `not_insert`, `not_dynamic`;
  - adlar ve değerler, her yerleştirmenin bloğuna.
  - Kilitli katmandakiler atlanır; hepsi kilitliyse `layer_locked`.

#### Öbür komutlar

- **`cad.blocks.insert`'in `parameters`'ı:** değerler aynı kuralla çözülür. Dinamik olmayan bloğa değer `not_dynamic` ile reddedilir.
- **`cad.blocks.edit`'in `dynamic` işlemi:** tanımın parametreleri ve eylemleri yazılır, ya da kaldırılır.
  - Kurallara uymayan `invalid_dynamic_block` ile reddedilir.
  - Yerleştirmelerin artık uymayan değerleri düşer, ifadeler yeniden hesaplanır. Düşen değerler `AttributeSync.parameters`'ta
    sayılır; kilitli katmandakiler atlanır ve söylenir.
  - Kartın sürümü artar.

### 4. KentOS blok kitaplığı (`CAD-BLOCK-13`)

- **İçerik:** KentOS'un kendi ölçülerinden çizilmiş 16 blok (`catalog/v1/blocks/kentos-temel.kblok`, üreticisi
  `scripts/blocks/library.py`). Başka bir yazılımın ya da kişinin hakkı yoktur; KentOS'la birlikte dağıtılır.
  - **Kapılar:** dinamik kapı (genişlik 60–120, dışa açılır, sol el, açılımı gizli, Banyo 70 / Oda 90 / Giriş 100 tablosu;
    taban noktası menteşe) ve çift kanatlı kapı.
  - **Pencere:** genişliği 60–300 cm esner.
  - **Mobilya:** masa (uzunluk esner), sandalye, kanepe, çift kişilik yatak, dolap (genişlikle kapılar dizilir).
  - **Islak hacim:** lavabo, klozet, duş teknesi, küvet.
  - **İşaretler:** kot işareti (KOT), kuzey oku, kesit işareti (KESİT, yönü çevrilir).
  - **Antet:** kâğıt boyunda, PROJE, PAFTA, ÖLÇEK, TARİH ve ÇİZEN öznitelikleriyle.
- Her bloğun kartı (kategori, etiketler, sürüm 1), açıklaması ve birimi (m) vardır.
- **Alma:**
  - `cad.blocks.edit`'in `import` işlemi `library` yerine `name: kentos-temel` alır. Bilinmeyen ad `invalid_library` ile reddedilir.
  - Masaüstünde `block.library` (KentOS blok kitaplığı, Ekle › Blok) bunu çalıştırıp Bloklar panelini açar.
  - Çizimde adı olan blok olduğu gibi kalır.

## Sonuçlar

- Bir kapı bloğu bütün genişliklerde, iki elde, iki açılışla ve açılımsız çizilir. Yerleştirmelerin öznitelikleri ve değerleri
  korunur.
- Python ve ajan değerleri adla okur ve yazar; geometri hesaplamazlar.
- Yeni proje KentOS'un temel bloklarıyla başlayabilir.
- **Sınırlar:**
  - Parametrelerin tutamaçları ve Öznitelikler'deki satırları yok (CAD-BLOCK-17, 18).
  - Parametrenin noktaları eylemlerle birlikte taşınmaz (AutoCAD'in zincirlemesi yok). Kapının elini çeviren ayna menteşenin
    üstündedir; bu yüzden kapının taban noktası menteşedir.
  - Eylemler öznitelikleri taşımaz.
  - Kutupsal parametrenin açısına kural yok, yalnız uzaklığına var.
  - DXF'e dinamik yerleştirme varsayılan tanımıyla yazılır; değerli yerleştirmenin varyantı adsız blok olarak yazılmaz
    (CAD-BLOCK-20).
  - Kitaplığın adları ve ölçüleri Türkiye'deki yaygın ölçülerdir; kurum standardı değildir.
  - Web ertelendi.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `dynamic_block_cases.py`: `cad.blocks.parameters` (33) ve `cad.blocks.edit`'in `dynamic`'i (28).
  - `block_insert_cases.py`: altı dinamik durum.
  - `blocks_command_cases.py`: yerleşik kitaplık.
  - `kcad_v2_reference.py`: şema 59, `dynamic-blocks` ve 11 bozuk dosya, `tools/kcad/kcad.py` ile.
  - `scripts/blocks/library.py --check`: kitaplığın metni.
- **Çekirdek:** `kentos-geometry-core`'un `block_dynamic` testleri.
  - Kapı varsayılan, geniş, çevrilmiş ve açılımsız.
  - Önbellekten aynı varyant.
  - İç içe kapı kendi değerleriyle.
  - Dolabın dizisi.
- **Sözleşme:** `kentos-contracts`'ın `a_door_is_a_dynamic_block_and_its_values_are_checked`'i.
- **Yetenekler:** `capabilities.rs`, her yeni kod bir durumda.
- **Masaüstü resimleri:**
  `KENTOS_SHOTS_ONLY=blok-dinamik,blok-kitaplik,blok-kitaplik-kategori cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-blok-*`).
