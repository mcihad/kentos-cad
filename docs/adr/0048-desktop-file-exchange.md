# ADR 0048: Masaüstünde dosya alışverişi (DXF, koordinat listesi) ve yeni katman

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §5, §7, §9.7, §23; TODOS.md `FMT-01`, `FMT-06`; ADR 0009 (dosya biçimleri), 0020 (masaüstü belgesi), 0023 (tipli ayarlar)
- **Sahibin yönü (26 Eylül):** masaüstü web ile aynı düzeye getirilir; o zamana kadar yalnız masaüstü işi yapılır, sonra yeni plan birlikte yapılır.

## Bağlam

- Web DXF'i ve koordinat listelerini (Netcad NCN, TXT, CSV) ADR 0009'dan beri alıp veriyor (`app/fileExchange.ts`, `ui/io/`). Okuyucu ve yazıcılar ortak Rust'tadır (`crates/shared/formats`); web onları biçim işçisinde WASM olarak çalıştırır.
- Masaüstünde bu komutlar yoktu: şeritte soluktu, "masaüstüne henüz taşınmadı" diyordu.
- Masaüstü belgesi (`kentos-domain`) katman ekleyemiyordu. İçe aktarım yeni katman kurar; web'in "Yeni katman" ve "Yeni grup" komutları da vardı.

## Karar

### Yeni katman ve grup

- `kentos-domain`'e web'in `layers.add` ve `uniqueName` anlamıyla katman ekleme geldi (`Document::add_layer(NewLayer, üst)`, `LayerTree::unique_name`):
  - üst bir grupsa en sona onun içine; bir katmansa o katmanın grubuna, en sona; yoksa ağacın en üstüne;
  - girdiği grup açılır;
  - düzenlemedir (belge kaydedilmemiş olur) ama geri alma adımı değildir (web'de de değil).
- Kimliksiz katman `layer-N` alır. Dosyadan okunan `layer-N` kimlikleri sayacı ileri iter.
  - Web'in sayacı modül geneldir; masaüstününki ağaç başınadır. İkisi de aynı kimliği iki kez vermez.
  - Var olan bir kimlik verilirse masaüstü yeni bir `layer-N` verir. Web ikinci bir düğüm kurar; hiçbir çağıran bunu yapmaz.
- Ortak belge fixture'larına (`fixtures/document-ops/v1`) `addLayer`, `uniqueLayerName` işlemleri ve `tree` beklentisi eklendi. Web ve masaüstü koşucuları aynı üç senaryoyu geçer. Kimlik değeri platforma göre değiştiği için senaryolar kimliği verir.
- Masaüstü komutları `layer.new` ve `layer.newGroup`:
  - `layer.new` etkin katmanın grubuna ekler ve yeni katmanı etkin yapar;
  - `layer.newGroup` ağacın en üstüne ekler;
  - ikisi de Katmanlar panelinin başlığında düğme olarak durur (`apps/desktop/src/layering.rs`).

### Dosya alışverişi (`apps/desktop/src/exchange/`)

- **Komutlar:** `file.import.dxf`, `file.import.ncn`, `crs.points`, `file.export.dxf`, `file.export.ncn`.
- **Aynı okuyucu ve yazıcılar:** web'in işçide çalıştırdığı `kentos-formats`, masaüstünde doğrudan çağrılır. Aynı fixture'lar (`fixtures/formats/v1`) iki tarafta sınanır. Masaüstü bu ortak crate'i kullanabilir (`scripts/arch/deps.mjs`, `shared`); yeni üçüncü taraf paket yoktur (`libm`, `serde`, `serde_json` masaüstünde zaten vardı).
- **Pencereler web'inkilerdir:** metinleri, düzeni ve kuralları aynıdır.
  - DXF içe aktar: kaynak katmanlar, her biri nereye gider (aynı adlı proje katmanına ya da dosyanın adını taşıyan gruptaki yeni katmana), okuyucunun raporu, kapsam ve koordinat sistemi sorusu.
  - Koordinat listesi içe aktar: ayırıcı, ondalık, başlık, sütun sırası; her sütun için rol seçimi, ilk 12 satırın önizlemesi, hatalı satırlar, hedef katman ve yeni katmanın adı.
  - DXF dışa aktar: kapsam (seçili, görünen, tümü; boş kapsam basılamaz), katmanlar, yazılacakların ve değişeceklerin özeti.
  - Koordinat listesi dışa aktar: kapsam, biçim, sütun sırası, başlık, karakter kodlaması.
- **Okuma ve yazma arayüz iş parçacığında değil**, kendi iş parçacıklarında yapılır (`iced_runtime::task::blocking`). Her okumanın bir sırası vardır; eskisine ya da kapanmış pencereye gelen geç yanıt atılır (CLAUDE.md §21.2).
- **Koordinat sistemi (CLAUDE.md §5):**
  - hep sorulur, varsayılan projeninkidir;
  - başka bir sistem içe aktarmayı kapatır ve nedenini söyler: datum dönüşümü, coğrafi ile projeksiyonlu arası ya da dilim dönüşümü henüz yok, sessiz dönüşüm yapılmaz;
  - liste web'in kaydıdır (`fixtures/crs/v1/registry.json`), datuma göre gruplu.
- **İçe aktarım** web'in `applyImport`'unun karşılığıdır (`exchange/apply.rs`):
  - hedefler önce denetlenir: kilitli ya da olmayan hedefte hiçbir şey değişmez;
  - yeni katmanlar kurulur (geri alınmaz, web gibi);
  - bütün nesneler tek geri alma adımıyla girer (“DXF: plan.dxf”, “Koordinat listesi: noktalar.ncn”);
  - katman adları büyük/küçük harf ve Türkçe harf farkı gözetmeden eşlenir (`fold_turkish`);
  - ardından görünüm eklenenleri gösterir (en az 20 m).
- **Dışa aktarım** kaydetme değildir: çizimin kaydedilmemiş durumu değişmez.
  - Kendi metni olmayan ölçünün değeri web'in `dimensionText`'iyle aynı yazılır. Ölçünün yerleşimini ortak kod verir (`kentos_formats::dxf::dimension_layout`, bu dilimde dışarı açıldı).
  - Dosya sistemin kayıt penceresiyle seçilir.
- **KentOS UI:**
  - onay kutusu (`tree_view::check_box`) başka yerlerde de kullanılır ve devre dışı olabilir;
  - parçalı seçimin parçaları devre dışı olabilir (`Segmented::new_with`).

## Bilinçli farklar

- **Nesne denetimi:** web içe aktarılan nesneleri `.kcad` okuyucusunun denetiminden bir kez daha geçirir, çünkü nesneler işçiden düz veri olarak gelir. Masaüstü okuyucunun tipli nesnelerini alır.
  - Kalan tek denetim bütün sayıların sonlu olmasıdır. NaN ya da sonsuz bir sayı çizimde gösterilemez, `.kcad`'e yazılamaz.
  - Bu denetim dosyanın okunduğu iş parçacığında yapılır (`apply::unusable`). Böyle bir nesne varsa hiçbir şey eklenmez; pencere web'in sözleriyle nedenini söyler.
- **Durdurma:** web işçiyi durdurarak okumayı keser. Masaüstünde okuma iş parçacığı sonuna kadar çalışır, yanıtı atılır. Okuyucuların boyut sınırı vardır; DXF en çok bir milyon nesne okur.
- **Kilitli hedef katman:** web'in listesi kilitli katmanı seçtirmez. Masaüstünün seçim kutusu seçeneği devre dışı bırakamaz; kilitli katman “(kilitli)” diye listelenir, seçilirse içe aktarma kapanır ve neden yazılır (web'in aynı sözleri).

## Bu dilimde olmayanlar

- **Nokta adları ve yazılar çizimde görünmez.** Masaüstünde yazı çizimi yoktur (`REN-12`). Adlar nesnede ve etiket olarak saklıdır, dışa aktarımda yazılır.
- **GeoJSON ve Shapefile:** web bunları ortak Rust'ta ekliyor (ADR 0046). Ortak koda girince masaüstüne aynı yolla gelir.
- **Yeni çizim, proje ayarları ve başlangıç ekranı** (`file.new`, `file.settings`, `file.start`) sonraki masaüstü dilimidir.

## Doğrulama (26 Eylül 2026, Linux)

- **`kentos-domain`:**
  - ortak fixture'ların 43 senaryosu geçer, üçü yeni;
  - kimlik sayacı ve yinelenen kimlik için iki masaüstü testi.
- **Web:** `documentOps.test.ts` aynı üç yeni senaryoyu geçer.
- **Masaüstü, 20 test:**
  - `layering` (2): yeni katman etkin katmanın grubuna girer ve etkin olur, “Yeni katman 2”; yeni grup en üste.
  - `exchange::apply` (5), `crs` (2), `words` (2), `coord_import` (1), `coord_export` (1).
  - Uçtan uca akışlar (7), `Picker::File` ile, uygulamanın kendi görevleri sonuna kadar yürütülerek:
    - DXF tek adımla, dosyanın adını taşıyan grupta girer; geri al hepsini alır, katmanlar kalır;
    - başka bir koordinat sistemi içe aktarmayı kapatır, projeninki açar;
    - hiç katman seçilmezse bir şey eklenmez;
    - koordinat listesi, nokta biçimli yeni katmana değerleri olduğu gibi girer;
    - aynı adlı katman (“çizim” → “Çizim”) bulunur;
    - DXF dışa aktarımı okuyucunun geri okuduğu kadar nesne yazar ve çizimi kirletmez;
    - koordinat listesi dışa aktarımı her noktayı tam yazar.
- **Kasıtlı bozmalar, ikisi de yakalandı ve geri alındı:**
  - koordinat sistemi denetimini hep “uyar” yapmak iki testi düşürdü;
  - nesneleri tek tek eklemek (her biri ayrı geri alma adımı) iki testi düşürdü.
- **Görüntüler** (`.run/shots/aktar-*.png`, koyu ve açık): altı pencere durumu (`cargo test -p kentos-desktop exchange::tests::screens -- --ignored`).
