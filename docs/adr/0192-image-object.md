# ADR 0192: Resim nesnesi

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-29`'un ardından `CAD-30`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler AutoCAD'in IMAGE'ı
  (IMAGEATTACH, IMAGECLIP, IMAGEFRAME) ve Netcad'in Resim'idir.
- **Bağlam belgesi:** TODOS.md `CAD-30`; coğrafi raster katmanı `GIS-08`'dedir. ADR 0092 (Stil yöneticisi: projenin kitaplığının PNG ve
  JPEG varlıkları), ADR 0157 (stilli çizimin karoları), ADR 0186 (degradenin nesne başına çerçevesi), ADR 0144 (blok yerleştirmesinin
  aynası), ADR 0177 (Kullanılmayanları temizle), ADR 0025 (`.kcad`).

## Bağlam

Paftaya logo, saha fotoğrafı ya da taranmış bir belge koymak için bugün çizimde bir yol yok; stil kitaplığının görüntüleri yalnız
sembollerin ve döşeme dolgularının parçasıdır ve atlasın küçük adımlarıyla (en çok 512 piksel) çizilir: fotoğraf ve taranmış belge için
yetmez.

## Karar

### 1. Nesne

Yeni nesne türü `image`: sol alt köşesi `p`, genişliği `width` ve yüksekliği `height` (metre, 0'dan büyük), `p` çevresinde dönüşü
`rotation` (radyan, saat yönünün tersine; blok yerleştirmesi gibi), `mirror` (resim kendi x ekseninde ters: Aynala'nın sonucu, blok
yerleştirmesinin aynası gibi). Kaynağı ikisinden biridir, yalnız biri:

- `asset`: projenin kitaplığındaki PNG ya da JPEG varlığının kimliği (**gömülü**);
- `file`: dosyanın yolu (**bağlı**).

İsteğe bağlı: `clip` (kırpma sınırı: resmin kendi kesirleriyle, sol alt 0,0 ve sağ üst 1,1; en az üç köşe, birim karenin içinde; yoksa
bütün resim), `opacity` (0,1–1; yoksa 1). `.kcad` şema 24.

### 2. Gömülü ve bağlı

- **Gömülü:** baytlar projenin kitaplığında varlıktır (`kind: 'asset'`, `format` `png` ya da `jpeg`, `data` data: adresi, `width` ve
  `height` pikselde, `path` `Resimler`); kimliği içeriğinin SHA-256'sının ilk 16 onaltılık hanesiyle `resim-…`: aynı resim bir kez
  saklanır. Kitaplığın değişmesi geri alma adımı değildir (ADR 0092); nesnesi kalmayan varlığı Kullanılmayanları temizle siler (bir
  resmin varlığı kullanılıyor sayılır).
- **Bağlı:** dosyanın yolu yazılır; masaüstü dosyayı okur (yol göreliyse çizim dosyasının klasörüne göre). Bulunamazsa ya da web'de
  (tarayıcı yolu okuyamaz) resmin yerinde açık gri dolgu ve çerçeve çizilir; Öznitelikler masaüstünde “bulunamadı”, web'de “tarayıcıda
  okunamaz” der. Öznitelikler'in **Göm**'ü
  masaüstünde dosyayı okur, web'de dosyayı seçtirir: resim gömülü olur (tek adım “Değiştir”).
- Resim çizimde en çok 4096 piksel kenarla gösterilir (büyüğü küçültülür; varlık ve dosya özgün kalır). 32 MB'tan büyük dosya alınmaz.

### 3. Çizim

Resim, boyası resmin kendisi olan bir alan dolgusudur: çerçevesi (ya da kırpma sınırı) üçgenlenir ve resmin kendi dokusuyla (mip
katmanlı, doğrusal süzgeç, kenarda kenara yaslı) ve saydamlığıyla, belge sırasında öbür nesnelerin arasında çizilir; üstüne çerçevesi
nesnenin renginde ince çizgiyle. Katmanın stili resme uygulanmaz (taramanın kendi deseni gibi); katmanın görünürlüğü ve kilidi uygulanır.

- Stil motorunda yeni kip `MODE_IMAGE`: nesne çerçevesini (çizgi) ve alanını (resim boyası) verir. Boyanın çerçevesi (köşe, boy,
  dönüş, ayna) degradeninki gibi nesne başınadır ve karosuna göre taşınır (ADR 0157, 0186 §3).
- Stilli çizim hattının sözleşmesi 5. sürüm: dolgunun `image` boyası (WGSL `imageFs`, WebGL2'de GLSL ikizi); grup 0'da atlasın yerine
  resmin dokusu ve mip süzgeçli örnekleyici. Doku resmin anahtarıyla (`asset:<kimlik>`, `file:<yol>`) bir kez çözülür ve yüklenir;
  hazır değilken ya da bulunamazsa yerine açık gri.

### 4. Seçme, kenet ve düzenleme

- Resim yalnız kenarından seçilir: çerçevesinin ya da kırpılmışsa kırpma sınırının kenarından (AutoCAD'in IMAGEFRAME'i gibi); içine
  tıklamak altındakini seçer, resim üstüne çizilen planı örtmez. Kenet gösterilen kısmın köşelerinde (uç nokta) ve kenar ortalarında.
- Tutamaçlar dört köşede: sol alt taşır, öbürleri oranı koruyarak sol alta göre ölçekler.
- Taşı, Kopyala, Döndür, Ölçekle, Hizala, Dizi ve Kutupsal dizi resmi alır; Aynala yerini aynalar ve `mirror`'ı çevirir (blok
  yerleştirmesi gibi). Vektör oturtma blok kuralıyladır (yerleştirme noktası ve yerel benzerlik, ADR 0156). Patlat, Ötele, Buda, Uzat ve
  benzerleri resmi almaz (söylenir).
- Öznitelikler: Konum (Y, X), Genişlik, Yükseklik, Dönüş, Saydamlık (%, 0–90), Kaynak (gömülü: adı ve piksel boyu; bağlı: yolu ve
  bulunup bulunmadığı), Kırpma (var ya da yok); Göm (bağlıda).

### 5. Araçlar

- **Resim ekle** (CAD'de Ekle › Resim, CBS'de Veri › Resim): PNG ya da JPEG seçilir; sonra sol alt köşeye tıklanır, ikinci nokta
  genişliği ve dönüşü verir (yazılan sayı genişliktir, dönüş 0); yükseklik resmin oranındandır. Önizlemede resmin çerçevesi ve köşegenleri
  imleçle büyür, yanında boyu. Masaüstünde **Bağlı** (B) seçeneği: dosyanın yolu yazılır, baytlar gömülmez (web'de tarayıcı yolu bilmez,
  seçenek yoktur). Tek adım “Resim ekle”; araç bir resim yerleştirir ve biter (bir sonraki resim için araç yeniden başlatılır, Enter son
  komutu yineler).
- **Resmi kırp**: resmin kenarına tıklanır; **Dikdörtgen** (D; varsayılan, iki köşe) ya da **Çokgen** (Ç; köşeler, Enter); **Kaldır** (K)
  kırpmayı kaldırır. Sınır resmin kendi kesirlerine çevrilir ve resmin karesiyle kesiştirilir, saat yönünün tersine, en alttaki (eşitse
  en soldaki) köşesinden yazılır; yeni sınır eskisinin yerini alır; kesişimin alanı yoksa söylenir. Tek adım “Resmi kırp”; araç sonraki
  resmi bekler. Esc ve Ctrl+Z önce son köşeyi, sonra resmi bırakır.

### 6. Komutlar

`cad.entities.create`'in `image` geometrisi, işlem `image` (adım “Resim ekle”): `asset` projenin kitaplığında bir görüntü varlığı olmalı
(`unknown_asset`); `file` boş olamaz; ikisi birden ya da hiçbiri, sıfır ya da eksi boy, birim karenin dışında ya da üçten az köşeli
kırpma, 0,1–1 dışında saydamlık `invalid_image`. `cad.entities.edit`'in `imageClip` işlemi (adım “Resmi kırp”). Öznitelikler mevcut
`properties` işlemiyle.

## Kapsam dışı

Coğrafi raster ve dünya dosyası (`GIS-08`), resmin piksellerinin düzenlenmesi, perspektif ya da kauçuk levhayla oturtma, parlaklık ve
karşıtlık, DXF'in IMAGE ve IMAGEDEF'i (içe aktarmada atlanır ve sayılır, dışa aktarmada yazılmaz ve söylenir), resim dosyasının ağdan
(URL) bağlanması.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/image.rs` (`ImageEntity`, `ImageFields` ve kuralları `problem`; sınırlar `MIN_IMAGE_OPACITY`,
  `MAX_CLIP_CORNERS`, `MAX_IMAGE_SIZE`, `MAX_IMAGE_PATH`), `Entity::Image`, `EntityGeometry::Image`, `CreateOperation::Image`,
  `EditOperation::ImageClip`; ret kodları `invalid_image`, `unknown_asset`, `image_in_block`. Web'in ikizi `model/imageRules.ts`; katalog,
  TypeScript ve Python SDK'sı yeniden üretildi; `FORMATS_VERSION` 34.
- `.kcad` şema 24 (`SCHEMA_WITH_IMAGES`): kodek (`decode/objects.rs`, `encode/objects.rs`, sütunlarda tür 16), web'in sütunları
  (`io/columns.ts`, `io/kcad.ts`) ve v1 okuyucusu (`model/snapshot.ts`); belge `docs/specs/kcad-v2.md` §6.1 ve §6.6; bağımsız Python okuyucusu
  ve yazıcısı (`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`), örnek `images.kcad` ve on bozuk dosya.
- Çekirdek: `geom/image.rs` (çerçeve, kesirler, gösterilen kısım, aynalanan çerçeve, birim kareye kesme), `ops/image.rs` (`placed`,
  `clip_of`; işlemler `imagePlaced`, `imageClip`); dönüştürme, Vektör oturtma (blok kuralı), tutamaçlar, kenet, kenarlar, seçme (yalnız
  kenar), çizim kaydı, genel bakış, paketleme (tür 19) ve karşılaştırma resmi tanır; Patlat, Ötele ve benzerleri almaz.
- Çizim: stil motorunun `MODE_IMAGE`'i ve `FillPaint::Image`'i (`style/build.rs`, `prim.rs`, `batch.rs`); WGSL `imageFs` ve sözleşmenin
  5. sürümü (`shaders/wgsl/styled.layout.json`); masaüstünde resmin dokusu `crates/render/wgpu/src/styled/pictures.rs` (mip katmanları
  CPU'da, GPU'nun boyutlarıyla: kenar ⌊kenar / 2ⁱ⌋; en çok 4096 piksel; bulunamayan açık gri) ve tutulan resimde `cpu.rs`; web'de
  `render/pictures.ts`, WebGL2 `IMAGE_FS` (`generateMipmap`) ve WebGPU'nun `image` boru hattı (katmanlar tuvalde çizilir), çözme atlasın
  `picture`'ından.
- Komutlar: masaüstü `crates/native/application` (`check_geometry`, `check_blocks`'un `has_picture`'ı, `no_tables`), web `product/`
  (`entitiesEdit.ts`'in `hasPicture`'ı ve `checkBlocks`'u, `blocksDefine.ts`); dönüşmede ikinci aynanın kaldırdığı `mirror` web'de alan
  olarak silinir (`entityOp.ts`, `withGeometry`; blok yerleştirmesininki de).
- Araçlar: masaüstü `kentos_interaction::image_insert`, `::image_clip` (`ImageFile`, `ViewChange::OpenImageFile`; dosya
  `apps/desktop/src/text_file.rs`'in `image_file_*`'ı, kitaplık öğesi `pictures.rs`), web `tools/imageTools.ts` ve `tools/pictureFile.ts`
  (PNG ve JPEG boyu başlıktan; kimlik SHA-256'dan). Katalogda Blok grubunun Resim bölümü: CAD'de Ekle › Resim, CBS'de Veri › Resim.
  İkonlar sahibin önerilen seçenekleri (A ve A).
- Öznitelikler: masaüstü `properties/rows.rs` ve `mod.rs` (Göm dosyayı okur), web `ui/properties/imageRows.ts` (Göm dosyayı seçtirir).
  Kullanılmayanları temizle resmin varlığını kullanılıyor sayar (iki platformda, `layer_purge`).
- Biçimler: DXF'e ve GeoJSON'a resim yazılmaz, raporda söylenir; DXF'in IMAGE'ı içe aktarmada atlanır ve sayılır (eskisi gibi);
  PostGIS izdüşümü gösterilen kısmın çokgenidir, sunucu kuralları denetler.
- Resimler: masaüstü `apps/desktop/src/image_scenes.rs` (`resim-*`), web `shots.mjs images` (WebGL2 ve WebGPU).

## Doğrulama

- Bağımsız başvuru: `python3 scripts/fixtures/image_cases.py --check` (KentOS kodu olmadan, kesirlerle): yerleştirmenin genişliği,
  yüksekliği ve dönüşü (beş doğrultu, aynı nokta reddi); kırpmanın kesirleri (içte, saat yönünde verilen, taşan, saran, köşeyi kesen
  üçgen, en alttaki köşe, aynalı resim, dışarıda ve yalnız kenara değen ret, dönük resim). Çekirdek doğal olarak (`tests/all/image.rs`) ve
  WASM'dan (`tools/image.wasm.test.ts`) aynı dosyayı geçer.
- Komutlar: `fixtures/commands/v1`'de oluşturma (gömülü, bağlı, kırpılmış, saydam; aynasızlık alan değil), sırasıyla `invalid_image` ve
  `unknown_asset` retleri, Resmi kırp ve retleri, blok tanımında ve yeniden tanımında `image_in_block`, Döndür, iki Aynala ve Ölçekle'nin
  kopyası (`affine_reference.py`'nin resim kuralı; JavaScript'in hypot'u). Dönüşme durumu dönüksüz resimle: orada çekirdeğin atan2'si
  (libm, fdlibm) ve Python'unki aynıdır; 0,5 radyanda son bitte ayrılırlar (`array_command_cases.py`'nin notu gibi).
- `.kcad`: Python okuyucusu ve Rust kodeği 278 dosyada aynı sonucu verir (`kcad_v2_reference.py --check`,
  `crates/shared/kcad/tests/all/images.rs`, web `io/kcad.wasm.test.ts`); paketleme testlerinde iki resim (çekirdek `store/pack.rs`, web
  `wasm/transform.wasm.test.ts`; okuyucunun tür sınırındaki hata bu testlerle bulundu).
- Ortak izler iki platformda üç türevde: `image-insert.json` (dosya iz klasöründen, yazılan genişlik, Ctrl+Z, ikinci noktayla dönük
  yerleşme, içine tıklama seçmez, kenarı seçer) ve `image-clip.json` (içine tıklama söylenir, Dikdörtgen, kırpma sınırından seçme, Çokgen,
  Kaldır, Dikdörtgen'e dönüş, Esc, Ctrl+Z).
- Mip katmanlarının boyları GPU'nunkiyle (`styled::pictures` testi, 480 × 360'ta 9 katman); WebGPU ve WebGL2 aynı sahneleri çizer.
