# ADR 0257: DXF'te dosyanın birimi, blok birimi ve içe almanın komutu

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-07`; §16 `CAD-IO-25` ve `CAD-IO-26`'nın DXF içe
  alma kısmı. Kapsamı ben belirledim (sahibin sözleri: “CAD data bölümüne geç ve orayı tamamla”; “Araç = komut” kuralı, 13 Ekim:
  dokunulan araç komuta getirilir).
- **Bağlam belgesi:** ADR 0165 §2 (çizim birimi; DXF ve birim), 0256 §1 (birim kuralı `kentos_contracts::units`), 0144 §5 (DXF
  blokları), 0138 (büyük içe aktarma kare kare), 0253 (komut protokolü), 0183 §7 (DXF'in stilleri).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

DXF okuyucusu dosyanın birimine `$INSUNITS`'ten bakıyordu (ADR 0165 §2):

- **Yerel projede:**
  - bildirilen birim metreye çevrilir;
  - birim bildirmeyen dosyanın değerleri çizimin biriminde sayılır.
- **Koordinat sistemli projede:**
  - değerler metre sayılır;
  - başka birim bildiren dosya yalnız raporda söylenir.

Eksikler:

- **Kullanıcı birimi seçemiyordu.** Birim bildirmeyen dosyanın (eski programların çoğu) ya da yanlış birim bildiren dosyanın
  (AutoCAD'in `acadiso` şablonu ölçme çizimlerine de mm yazar) birimini değiştirmenin yolu yoktu. Karar yalnız raporun bir satırıydı;
  içe almadan önce görünmüyordu.
- **Blok birimi okunmuyordu.** `BLOCK_RECORD`'un 70. grubu (bloğun ekleme birimi) okunmuyordu. Bloğu başka birimde olan dosya
  sessizce geçiyordu.
- **İçe alma komut değildi.** Python ve ajan DXF alamıyordu. Masaüstünün içe alma kuralları (`exchange/apply.rs`) kabuktaydı;
  ADR 0228'e göre kabuk incedir. İşlem manifestinde `file.import.dxf` `none` düzeyindeydi.

## Karar

### 1. Dosyanın birimi: karar ve kaynak

Okuyucunun seçeneklerine `fileUnit` eklenir (`DxfReadOptions.file_unit`). Bu, dosyanın değerlerinin birimidir; kullanıcı ya da
çağıran seçer. Seçilebilen birimler `$INSUNITS`'in en çok kullanılanlarıdır (`FileUnit`):

| `FileUnit` | Birim | `$INSUNITS` |
|---|---|---|
| `mm`, `cm`, `dm`, `m`, `km` | metrik | 4, 5, 14, 6, 7 |
| `in`, `ft`, `yd`, `mi` | uluslararası inç, fit, yarda, mil | 1, 2, 10, 3 |
| `usFt` | ABD ölçme fiti (1200/3937 m) | 21 |

Kural (`kentos_formats::dxf`'in `decide_units`'i):

1. `fileUnit` verilmişse değerler o birimdedir (`source: chosen`).
2. Verilmemişse:
   - koordinat sistemli projede değerler metre sayılır (`metres`). Başka bir birim bildiren dosya `unit_ignored` ile söylenir;
   - yerel projede dosyanın bildirdiği birim kullanılır (`declared`). Tanınan birim, AutoCAD'in `$INSUNITS` için tanımladığı
     24 birimden biridir (mikroinçten parseke).
3. Dosya birim bildirmiyorsa (0 ya da başlıksız) ya da kodu tanınmıyorsa sayılar ölçeklenmez (`assumed`):
   - yerel projede değerler çizimin biriminde sayılır (`project`);
   - sistemli projede metre sayılır (`metres`);
   - `unit_assumed` söyler.

Belirsiz dosya sessizce ölçeklenmez:

- sayılar ya kullanıcının gördüğü ve seçebildiği birimdedir ya da projenin kendi birimindedir;
- karar içe almadan önce gösterilir: masaüstünün penceresinde ve komutun planında.

Çeviri ADR 0256'nın birim kuralıyla yapılır (`Scale`, `$INSUNITS`'in tam oranı: bir çarpma, sonra bir bölme). `$INSUNITS` tablosu
`kentos_contracts::units::insunits`'e taşındı.

Sonuç (`ImportResult.unit`, `UnitDecision`):

| Alan | Ne |
|---|---|
| `declared` | dosyanın bildirdiği kod; 0'da ve başlıksız dosyada yok |
| `used`, `name` | değerlerin okunduğu birimin kodu ve adı |
| `source` | `chosen`, `declared`, `metres`, `project` |
| `factor` | `used`'ın bir biriminde kaç metre var: değerler bununla çarpıldı |
| `assumed` | ne bildirildi ne seçildi |
| `blocks` | birimi dosyanınkinden başka bloklar (§2) |

Uyarıların sözleri (`UnitDecision::warnings`; `path` `fileUnit`):

- `unit_assumed`: “{neden}; değerler ölçeklenmeden {metre | çizimin biriminde (ad)} sayıldı. Dosyanın birimi başkaysa onu seçin
  (fileUnit).” Neden “Dosya birim bildirmiyor” ya da “Dosyanın bildirdiği birim kodu (99) tanınmıyor”dur.
- `unit_ignored`: “Dosya birimini {ad} olarak bildiriyor; koordinat sistemi olan proje dosyayı metre okur, değerler ölçeklenmedi.
  Ölçeklemek için dosyanın birimini seçin (fileUnit).”
- `block_units_differ`: “Bazı blokların birimi dosyanınkinden başka: {ad (birim), …}. Yerleştirmelerin ölçeği bu oranı taşır;
  bloklar ayrıca ölçeklenmedi.”

### 2. Blok birimi

> 22 Ekim 2026: Aşağıdaki ilk uygulamanın bütün tanımları dosyanın birimiyle çevirme kuralı
> [ADR 0589](0589-cad-paper-frames-and-block-units.md) ile değişti. Tanımlar kendi açık birimleriyle
> normalize edilir; INSERT X/Y ölçekleri sahibi/hedef oranıyla telafi edilerek mevcut dünya boyu korunur.
> `block_units_differ` artık bu davranışı bildirir. Eski kural aşağıda tarihsel gerekçe olarak durur.

`BLOCK_RECORD`'un 70. grubu okunur. AutoCAD bir bloğu başka birimdeki çizime eklerken birimlerin oranını yerleştirmenin ölçeğine
yazar; dosyadaki yerleştirmenin ölçeği bu oranı içerir. Bu yüzden:

- bloğun nesneleri dosyanın birim çarpanıyla çevrilir;
- yerleştirme ayrıca ölçeklenmez.

Birimi dosyanınkinden başka (sıfır olmayan ve bildirilen koddan başka) bloklar dosyanın sırasıyla `blocks`'a yazılır ve
`block_units_differ` ile söylenir. AutoCAD'in kendi blokları (`*Model_Space`, `*Paper_Space`, ölçünün `*D…`'si) söylenmez: onlar
çizimin birimindedir.

### 3. `file.import.dxf` v1

Girdi (`ImportDxf`):

| Alan | Ne |
|---|---|
| `path` | okunacak DXF, mutlak yol |
| `fileUnit?` | §1 |
| `explodeBlocks?` | Blokları patlat |
| `skipLayers?` | dışarıda kalacak kaynak katmanlar, dosyadaki adlarıyla |
| `layers?` | kaynak katman adından hedef katman kimliğine |
| `group?` | yeni katmanların gireceği üst grup |
| `expectedRevision?` | protokolün beklenen sürümü |

Kaynak katmanların adları büyük küçük harf ve Türkçe işaretler bir sayılarak eşlenir. Eşlenmeyen kaynak katman şuraya gider:

- çizimde aynı adlı katman varsa ona;
- yoksa dosyanın görünüşüyle yeni bir katmana. Kimliği `import-<adın katlanmışı>`'dır, alındıysa `-2`, `-3` ….

Yeni katmanlar bir gruba girer:

- `group` yoksa dosyanın adındaki gruba;
- boşsa ağacın üstüne;
- adı verilmişse o gruba (ağacın üstünde adıyla bulunur ya da açılır; kimliği katmanlarınkinden sonra aynı kuralla).

Çizimde adı olan blok “Ad (2)”, “Ad (3)” … adıyla girer. Dosyanın yazı ve ölçü stilleri projeye eklenir (ADR 0183 §7).

Bütün alınanlar tek geri alma adımıdır: “DXF: ad.dxf”. Plan dosyayı okur, okuma kopyasında yazar, hiçbir şey yazmaz ve
execute'un aynısını söyler.

Çıktı (`DxfImported`):

| Alan | Ne |
|---|---|
| `file`, `bytes`, `sha256` | okunan dosya: tekrar oynatma aynı dosyayı tanır |
| `unit` | §1'in kararı |
| `objects` | alınan nesneler |
| `targets` | kaynak katman → hedef katman |
| `layers`, `group` | açılan katmanlar (kimlik, ad) ve girdikleri grup |
| `blocks`, `renamed` | alınan tanımlar ve adı değişenler |
| `textStyles`, `dimensionStyles` | projeye eklenen stiller |
| `report` | okuyucunun raporu |
| `revision` | çizimin sürümü |

Retler, bu sırayla:

1. `invalid_file_path` (boş, göreli ya da klasör);
2. `invalid_revision`, `revision_conflict`;
3. `file_not_found`, `import_unreadable` (DXF değil, bozuk ya da sonlu olmayan sayı taşıyan);
4. `layer_not_found`, `not_a_layer`, `layer_locked` (`layers`'ın hedefi, `layers.<ad>`'da; aynı adlı hedef kilitliyse `layers`'ta);
5. `nothing_to_import` (alınacak nesne kalmadı).

Uyarılar: `unit_assumed`, `unit_ignored`, `block_units_differ` ve `import_losses`. Sonuncunun sözü: “Okuyucu dosyanın {n}
öğesini alamadı ({türler}); nedenleri report.skipped'te.”

İçe almanın kuralları masaüstünden uygulama katmanına taşınır (`kentos_native_application::import_apply`; kabukta yeniden dışa
aktarımı kalır). Bu kurallar yeni katmanları, blok kimliklerini ve adlarını, stilleri ve tek adımı kapsar. Masaüstünün bütün içe
aktarma pencereleri (DXF, NCZ, GeoJSON, Shapefile, KML, koordinat listesi, GNSS) ve komut aynı yoldan yazar.

### 4. Masaüstü

DXF içe aktarma penceresinde Bloklar'ın yanında “Dosyanın birimi” satırı vardır:

- Liste “Dosyaya göre (milimetre)” ve on birimdir. Seçim dosyayı yeniden okur (Blokları patlat gibi); başka dosya seçilince
  “Dosyaya göre”ye döner.
- Altında kararın sözü vardır: dosyanın ne bildirdiği ve sayıların çizimin birimine nasıl geçtiği. Örnekler: “Dosya milimetre
  ($INSUNITS 4) bildiriyor. Sayılar ×0,001 ile çizimin birimine (m) çevrilir.”; “Dosya birim bildirmiyor; değerler ölçeklenmeden
  çizimin biriminde (mm) sayıldı. Birimi biliyorsanız seçin. Sayılar aynen alınır.”
- Birimi başka bloklar özette okuyucunun notuyla söylenir (“Blok birimi: PANO (milimetre): …”).

Pencere dosyayı kendi iş parçacığında komutun okuma seçenekleriyle okur (`file_import::read_options`). İçe Aktar şöyle yazar:

- **20 000 nesneye kadar:** komutla, okuduğu dosyayla yazar (`file_import::execute_read`). Girdi pencerenin seçimleridir: yol,
  birim, Blokları patlat; seçilmeyen ve hedefi kilitli katmanlar `skipLayers`'tadır. Kayıtta satır komutundur.
- **Daha büyük dosya:** ADR 0138'deki gibi kare kare girer, ama komutun planıyla (`file_import::import_plan`). Bitince kayda
  komutun satırı yazılır (`file_import::record_progressive`, `call::record_written`): komut, girdi, açılan nesneler ve uyarılar.

`Picked` dosyanın yolunu da tutar (kayıttaki girdi onu yazar).

NCZ penceresi aynı kuralları kullanır ama komutu yoktur: Netcad NCZ'nin içe alma komutu `CAD-IO-26` ile ve NCZ okuyucusunun
dağıtım koşulu (TODOS.md `NCZ-01`) çözülünce gelir.

## Uygulama

- **Sözleşme:**
  - `formats.rs`: `FileUnit`, `UnitSource`, `BlockUnit`, `UnitDecision` (`warnings`), `DxfReadOptions.file_unit`,
    `ImportResult.unit`; `FORMATS_VERSION` 58.
  - `units.rs`: `insunits` ve birim sözlüğüne `insunits`.
  - `file_import.rs`: `ImportDxf`, `DxfImported`, `ImportedLayer`, `RenamedBlock`.
  - `codes.rs`: `file_not_found`, `import_unreadable`, `nothing_to_import`, `unit_assumed`, `unit_ignored`,
    `block_units_differ`, `import_losses`.
  - Kataloğun kaydı ve örnekleri, yetenekleri.
- **Okuyucu:** `dxf/mod.rs`'in `decide_units`'i (eski `units_in`'in yerine), `BLOCK_RECORD`'un 70'i
  (`Library.block_units`).
- **Uygulama katmanı:**
  - `import_apply.rs`: masaüstünün `exchange/apply.rs`'inden taşındı; `Applied` ve `Progressive` açılan katmanları, grubu ve
    hedefleri de verir.
  - `file_import.rs`: `read_options`, `validate`, `plan`, `execute`, `execute_read`, `import_plan`, `record_progressive`,
    `warnings_of_read`.
  - `call.rs`: `record_written`.
  - `DESKTOP_COMMANDS`, `registry.rs`; bağımlılık `kentos-formats`.
- **Masaüstü:** `exchange/drawing_import.rs`: `unit_field`, `unit_said`, `Event::FileUnit`, `run_dxf_import`, `write_import`;
  `exchange/mod.rs`'in `Picked.path`'i.
- **Python:**
  - `kentos.cad.file.import_.dxf`: üreteç anahtar sözcük olan ad alanını `import_` yapar;
  - `Document.import_dxf`.
- **MCP:** araç katalogdan; yıkıcı değil (yalnız ekler).
- **İşlem manifesti:** `file.import.dxf` `semantic`.

## Doğrulama

- **DXF birimleri**, `fixtures/formats/v1/dxf-units` (8 dosya, 24 durum): bağımsız başvuru `scripts/fixtures/dxf_units_cases.py`,
  dosyaları DXF başvurusundan elle yazar, beklentileri bu ADR'nin kuralından hesaplar.
  - Dosyalar: mm, birimsiz, başlıksız, tanınmayan kod, inç, ABD ölçme fiti, metrelik çizimde milimetrelik blok (`*Model_Space`
    başka birimde ama söylenmez) ve mikrometre.
  - Karar, uyarıların kodları ve nesneler metrede bit bit eşit: biçim çekirdeğinin `dxf_units::the_shared_unit_cases_hold`'u.
- **Komut**, `fixtures/commands/v1/file.import.dxf.json` (36 durum): bağımsız başvuru `scripts/fixtures/dxf_import_command_cases.py`,
  KentOS kodu olmadan bu ADR'den.
  - Kapsam: birimin bütün yolları, plan, validate, geri alma, yeni katman ve grup, `group`'un üç biçimi, `layers`, patlatma, adı
    olan blok, okuyucunun alamadığı nesne, her ret.
  - Masaüstü ve Python'un tipli katmanı geçer.
  - İki beklenti bilerek bozulunca ikisi de düşer.
- **Çalıştırıcıların ekleri** (`fixtures/commands/README.md`):
  - `$fixture:yol`: ortak dosyanın mutlak yolu, metnin içinde de;
  - `"$any"`: değeri önemsiz alan (okuyucunun raporu, okuyucunun okuyamama nedeni);
  - `fields`: nesnenin yalnız adı verilen alanları;
  - `nodes`: katman ağacının düğümlerinin yalnız adı verilen alanları ve `parent`.
- **Python:** `test_units.py`'nin `DxfUnit` sınıfı. Milimetrelik oda metrelik projede 4 m'lik duvarla gelir ve geri alınır; birim
  bildirmeyen dosyanın planı `unit_assumed` söyler, santimetre seçilince ×0,01.
- **Masaüstü:**
  - `exchange::tests`'in DXF içe aktarma testleri komutun yolundan geçer;
    `a_dxf_goes_in_by_its_command_in_the_unit_chosen`: birim bildirmeyen dosya projenin biriminde okunur, mm seçilince yeniden
    okunur, 4000 mm'lik duvar 4 m gelir, kayıttaki satır `file.import.dxf`'tir (girdisinde `fileUnit` ve yol);
  - resimler `cargo test -p kentos-desktop exchange::tests::unit_screens -- --ignored --nocapture`,
    `.run/shots/aktar-*-2[3-7]-dxf-*` (iki tema, 1440 × 900 ve 1100 × 650).

## Sonuçlar

- Belirsiz dosya sessizce ölçeklenmez. Karar içe almadan önce görünür, birim seçilebilir.
- Python ve ajan DXF alır. Pencereyle aynı kuralları ve aynı birim kararını kullanırlar.
- **Açık kalanlar:**
  - NCZ'nin, GeoJSON'un, Shapefile'ın ve KML'nin içe alma komutları (`CAD-IO-26`);
  - içe almanın planında kapsam ve stil eşlemesi (`CAD-IO-25`'in kalanı);
  - eşit olmayan ölçekli yerleştirmenin bloğu (`CAD-IO-15`).
