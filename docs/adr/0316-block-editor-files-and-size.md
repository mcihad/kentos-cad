# ADR 0316: Blok düzenleyici, blokların dosya API'si ve blok boyutu sınırı

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §10 `CAD-BLOCK-05`, `06`, `07`, `11`. Sahibin isteği: “CAD blocok ile devam
  et”; maddeler 3–4'erli gruplarla yapılıp sınanır. Bu dört madde ikinci gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0144 (bloklar), 0193 (çizimler arası alışveriş: Dosyadan blok ekle, Başka çizimden al), 0263 (blok içi yol);
  - 0315 (parçanın kararlı yerel kimliği), 0253 (komut protokolü, dosya komutları), 0309 (pano komutları);
  - CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Blok düzenlenemiyordu.** Tanımı değiştirmenin tek yolu Yeniden tanımla'ydı: nesneleri çizime patlatıp düzenlemek ve seçerek
  yeniden tanımlamak. Bu yol yerleştirmenin parçalarının yerel kimliklerini yeniden numaralar. Ölçünün blok içi bağı (ADR 0315 §1)
  bu yüzden kopardı.
- **WBLOCK yoktu.** Bloğu ayrı bir çizim olarak yazmak için önce yerleştirip Seçilenleri dosyaya kaydet'i kullanmak gerekiyordu.
  Taban noktası da başlangıca gelmiyordu.
- **Dosyadan blok almanın komutu yoktu.** Dosyadan blok ekle ile Başka çizimden al'ın blokları ADR 0193'ün kurallarıyla masaüstünün
  penceresinde yazılıyordu. Python ve ajan aynı işi yapamıyordu.
- **İç içe bloğun bütçesi yoktu.** Döngü ve 16 düzeylik derinlik reddediliyordu. Ama 16 düzeyde her düzey 10 yerleştirmeyle tek
  tanım 10¹⁶ parçaya açılabilir; açılım, kenet ve çizim bunu bitiremez.

## Karar

### 1. Blok düzenleyici (`CAD-BLOCK-05`)

- **Giriş:**
  - Bloklar panelinde bloğun menüsünden Bloğu düzenle;
  - seçili yerleştirmeyle `block.edit` (BEDIT, BLOKDUZENLE), şeritte Ekle › Blok.
- **Ayrı çizim:** çizim kenara alınır, yerine tanımın nesnelerinden kurulmuş bir çizim açılır.
  - Çizimin katmanları, blokları, stilleri ve ayarları gelir; çizimin başka nesnesi gelmez.
  - Bloğun kendi katmanı (`""`, DXF'in 0'ı) burada ayrı bir katmandır: “0 (blok)”. Etkin katman odur.
  - Her nesnenin numarası tanımdaki yerel kimliğidir; 0 ya da iki kez geçen kimlik sıradaki boş numarayı alır.
  - Kendi geri alması vardır; Kaydet'e kadar çizime hiçbir şey yazılmaz.
- **Kapsam:** bütün araçlar düzenleyicinin çiziminde çalışır. Açıkken şunlar bekler:
  - çizimin dosya, bulut, pafta ve proje komutları (“Blok düzenleyici açık: önce Kaydet ya da Vazgeç ile çizime dönün.”);
  - bulut eşitlemesi ve kurtarma kopyası;
  - pencere kapanmaz.
- **Kaydet:**
  - Çizim geri gelir ve tanım `cad.blocks.edit`'in yeni `update` işlemiyle tek adımda yazılır (Blok değiştir).
  - Önce çizimin kopyasında denetlenir: ret (blok kendini içerirse) düzenleyiciyi açık bırakır.
- **Vazgeç:** çizim olduğu gibi geri gelir.
- **Dönüş:** ikisi de çizimi bırakıldığı görünüme ve seçime döndürür.
- **`update` işlemi:** girdisi bloğun kimliği, tanımın yeni nesneleri (`entities`), isteğe bağlı taban noktası ve öznitelikleri.
  - Nesnelerin yerel kimlikleri korunur; 0 ya da iki kez geçen kimlik sıradaki boş numarayı alır (`numbered`).
  - Bağlar ve çizim sırası nesnelerden düşer (`geometry::unlinked`).
  - Tanımın kuralları (`rules_with`) çizimin bütün bloklarıyla denetlenir: döngü, derinlik, boyut, bilinmeyen blok.
  - Retler `cad.blocks.edit`'in var olan kodlarıdır, yenisi `duplicate_piece`. Ortak durumlarda 4 `update` durumu vardır.

### 2. Kaydetmenin geçersiz kıldıkları (`CAD-BLOCK-06`)

- `update` bloğun kimliğini korur. Belge, değişen tanımın bütün yerleştirmelerini bir adımda değişmiş sayar (ADR 0144 §3).
- **Ölçünün bağı** parçasını yerel kimliğiyle izler (ADR 0315 §1). Düzenleyicide kalan nesne kimliğini koruduğu için bağ kopmaz.
- **Masaüstü:** düzenleyici kapanınca deponun izlediği belge değişir (`spatial.reload`, `followed`), Kaydet'in adımı günlükten
  gelir. Kenet, kutu ve stilli çizimin parçaları değişen yerleştirmelerle yeniden kurulur. Geri alma tek adımda eski tanımı getirir.

### 3. Blokların dosya API'si (`CAD-BLOCK-07`)

#### `file.export.block` (WBLOCK)

- **Girdi:** `path`, ve ya bloğun kimliği (`block`) ya da nesnelerin kimlikleri (`uids`); isteğe bağlı `base` ve `overwrite`.
- **Çıktı:** ayrı bir `.kcad` v2 çizimi.
  - Blok kipinde tanımın nesneleri kendi katmanlarında; bloğun katmanındakiler “0” katmanında (yoksa açılır).
  - `byLayer` düşer: dosyada çizim nesnesidirler.
  - Kimlikleri bloğun kimliğinden ve sırasından türetilir (`UUIDv5(blok, "piece/i")`), böylece aynı blok aynı dosyayı yazar.
  - Kullandıkları katmanlar, iç bloklar, stiller, kitaplık öğeleri ve ayarlar Seçilenleri dosyaya kaydet'in kuralıyla gelir
    (`exchange::selection`).
  - Taban noktası (verilen ya da bloğunki) başlangıca taşınır.
- **Komutun türü:** dosya komutudur (ADR 0253 §10): plan baytları ve özeti verir, yazma atomiktir, kayıtta dosyanın özeti durur.
- **Takma adlar:** `wblock`, `blokyaz`.
- **Masaüstü:** Bloklar panelinde Dosyaya yaz… ve şeritte Bloğu dosyaya yaz (`block.writeFile`) aynı işlevle yazar
  (`kentos_headless::file_commands::export_block`).

#### `file.import.block`

Bir `.kcad` çizimini (v2 ya da v1 JSON) okur, getirdiğini tek adımda yazar: “Dosyadan blok: ad.kcad”.

- **`drawing` kipi (varsayılan):** dosya tek blok olur (Dosyadan blok ekle).
  - Adı `name`, boşsa dosyanın adıdır. Taban noktası nesnelerin sol alt köşesidir.
  - İç blokları ve katmanları yollarıyla gelir.
  - Resimler ve tablolar bloğa konamaz; `objects_left_out` uyarısıyla söylenir.
- **`blocks` kipi:** dosyanın blokları adlarıyla, yerleştirdikleriyle gelir (Başka çizimden al).
  - `names` verilmezse dosyanın bütün blokları gelir.
  - Ad büyük küçük harfe bakmadan bulunur, dosyadaki yazılışıyla alınır.
  - Çizimde olan ad korunur ve `skipped`'te söylenir.
  - `replace` ile çizimdeki aynı adlı blok yerinde yeniden tanımlanır: kimliği kalır, bütün yerleştirmeleri dosyanınkini gösterir
    (Başka çizimden yeniden tanımla).
- **Kimlik çakışması:** dosyanın bir bloğunun kimliği çizimde zaten varsa (aynı dosya iki kez alındığında) o blok yeni kimlik alır,
  yerleştirmeleri de onunla gider. Böylece yeni tanım çizimdeki bir tanımın üstüne kimliğiyle yazılmaz.
  - Bunu ortak durumlar buldu: önceki masaüstü yolunda ikinci alma ilk alınanın iç bloklarını eziyordu.
- **Ayarlar:** parçaların kullandığı yazı ve ölçü stilleri ile kitaplık öğeleri ayardır (ADR 0193 §2); adımın yanında yazılır.
- **Kural ve yazma:** ADR 0193'ünkiler (`exchange::file_block`, `exchange::take`, `exchange_write`).
- **Retler, sırayla:**
  - `invalid_file_path`, `file_not_found`, `import_unreadable`;
  - beklenen sürüm;
  - `nothing_to_import` (boş `names` ya da bloksuz dosya), `unknown_block` (dosyada olmayan ad), `block_refused`.
- **Kilitli katman:** reddedilmez. Kilit, nesneyi düzenlemeyi engeller, bloğun tanımına girmesini engellemez.
- **Plan:** çizimin kopyasında çalışır, çizimin sürümünü söyler.
- **Masaüstü:**
  - Dosyadan blok ekle bu komutla yazar ve ardından Blok ekle'yi başlatır.
  - Bloklar panelinde Başka çizimden yeniden tanımla… ile şeritte `block.redefineFromFile`, `blocks` kipini `replace` ile
    çalıştırır.
- **Python ve MCP:** `kentos.cad.file.import_.block`, `kentos.cad.file.export.block`. İkisi de MCP'nin yıkıcı listesinde: biri
  dosyanın üstüne yazabilir, öbürü blokları yeniden tanımlayabilir.

### 4. Bloğun boyutu (`CAD-BLOCK-11`)

- **Sınır:** döngü (`block_cycle`) ve 16 düzey (`block_too_deep`) yanında, bir tanım iç içe yerleştirmeleriyle bir milyondan çok
  parçaya açılamaz (`block_too_large`, `MAX_BLOCK_PIECES`).
- **Sayım:**
  - Öznitelik tanımları da sayılır.
  - Sığdan derine, doymalı toplamla hesaplanır (`blocks::sizes`). 10¹⁶'lık bir tanım taşmadan reddedilir.
- **Eşit uygulama:** kural `kentos_contracts::blocks`'un `nesting`'indedir ve bütün yollar onu çağırır:
  - `.kcad` okuyucusu, bağımsız Python okuyucusu (`tools/kcad/kcad.py`);
  - iki belge, `cad.blocks.define`, `cad.blocks.edit`;
  - sunucunun proje değişiklikleri ve içe aktarması (`blocks::check`).
- **Ortak durum yok:** milyon parçalı tanım ortak durum dosyasını megabaytlarca büyütür. Bu yüzden durum `capabilities`'te
  gerekçesiyle ayrılmıştır. Kural `kentos-contracts`'ın testinde ve `fixtures/kcad/v2/broken/block-too-large.kcad`'de sınanır
  (bin parçalı tanımın 1001 yerleştirmesi).

## Sonuçlar

- Blok yerinde düzenlenir, yerel kimlikler korunur; ölçünün blok içi bağı düzenlemeden geçer.
- Python ve ajan blokları dosyaya yazar ve dosyadan alır; masaüstünün üç penceresi aynı komutla yazar.
- Kötü niyetli ya da bozuk bir `.kcad` iç içe bloklarla belleği tüketemez.
- **Sınırlar:**
  - Düzenleyici çizimin yerine açılır; yerinde düzenleme (REFEDIT) yoktur, çizimin öbür nesneleri düzenlerken görünmez.
  - Düzenleyicinin geri alması Kaydet'te tek adıma iner.
  - Bir milyonun altındaki ama çok büyük bloklar hâlâ yavaştır; bütçe yalnız sınırsız büyümeyi keser.
  - Web ertelendi.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `block_file_cases.py`: `file.import.block`, 25 durum. Kaynak çizim `fixtures/blocks/v1/ofis-bloklari.kcad` (v1 JSON). Çizimin ne
    olduğu `exchange_cases.py`'nin `file_block` ve `take`'inden gelir; kimlik çakışması ve ad kuralları bu betiktedir.
  - `blocks_command_cases.py`: `update` işleminin 4 durumu.
  - `kcad_v2_reference.py`: `broken/block-too-large.kcad`, `tools/kcad/kcad.py` ile.
- **Çekirdek ve sözleşme:**
  - `kentos-contracts`'ın `a_definition_opening_into_more_than_a_million_pieces_is_refused`'ı;
  - `capabilities.rs`: `block_too_large` gerekçesiyle, `objects_left_out` durumla.
- **Başsız sunucu:** `a_block_is_written_to_a_file_and_read_into_another_drawing`. Kapsadıkları:
  - plan ve yazma aynı baytı verir;
  - taban noktası başlangıçta;
  - başka çizime tek blok olarak gelir;
  - `file_exists`, `no_block`, `unknown_block`, `entity_not_found`, `no_entities`, `invalid_file_path`.
- **Masaüstü:** `block_editor::tests`. Kapsadıkları:
  - düzenleyicide tanımın dört nesnesi;
  - dosya komutu bekler;
  - Kaydet tek adım ve beş parça;
  - Geri al dört parça;
  - Vazgeç dokunmaz.
- **Masaüstü resimleri:**
  `KENTOS_SHOTS_ONLY=blok-duzenleyici,blok-duzenleyici-cizim,blok-duzenleyici-kaydedildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-blok-duzenleyici-*`).
