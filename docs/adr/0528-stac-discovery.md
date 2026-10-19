# ADR 0528: STAC keşfi ve veri hazırlama kaydı

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-31'in ilk dilimi (“mevcut görüntü analizine STAC keşfi ve veri
  hazırlama kaydı ekle”). Kapsamı ben belirledim; ilke “Performance First”.
- **Bağlam belgesi:** `plan/gis.md` CBS-31; ADR 0242 (uzaktan algılama: Spektral indis'in Ölçek ve Öteleme'si), 0204 ve
  0207 §1 (raster ve adresten okunan COG), 0208 (harita servisleri, ağ ev sahibinindir), 0253 (komut protokolü), 0517
  (adresli rasterin komutla eklenmesi), 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** (1) imza isteyen katalogların (Planetary Computer SAS, belirteçli sunucular) imzalı adresi
  oturumda tutup raster okumasına vermesi ve Bağlantılar'a bağlanması; (2) `s3://` adresli varlıkların (Landsat, istek
  ödemeli kovalar) okunması; (3) kaynağı başka sistemdeki öğenin projenin sistemine yeniden izdüşümü (bugün
  `crs_mismatch`); (4) kalite maskesinin Raster hesaplayıcı'ya ve Spektral indis'e uygulanması (bugün kaydedilir, bulutlu
  piksel hesaba hâlâ girer); (5) bantların ortak ızgaraya örneklenmesi (bugün `stac_grid_differs` uyarısı);
  (6) eğitim ve test örneklerinin, model, sınıf ve tohumun, mekânsal doğrulamanın kaydı (CBS-31'in ikinci yarısı);
  (7) kaydın `.kcad` şemasına projenin kaydı olarak yazılması (bugün her rasterin özniteliklerinde); (8) Python'da ağ
  (Python ana bilgisayarı ağ kütüphanesi taşımaz); (9) `/collections` ve kök belgesinin komut olarak sunulması
  (bugün pencerenin okuyucuları, `kentos_services::stac::catalog`).

## Bağlam

Uzaktan algılama araçları (ADR 0242) bir rasteri alır; görüntüyü bulmak, doğru bantları seçmek, bantların ölçeğini ve
ötelemesini, nodata'sını ve kalite bandını doğru eşlemek kullanıcıya kalıyordu. STAC (SpatioTemporal Asset Catalog) bunu
açık bir sözleşme yapar: öğe (item) bir çekimdir, varlık (asset) onun bir dosyasıdır; `eo:bands` bandı, `raster:bands` veri
türünü, nodata'yı, ölçeği ve ötelemeyi, `proj:*` ızgarayı söyler. QGIS'in STAC eklentisi ve ArcGIS'in STAC bağlantısı aynı
işi yapar: ara, öğeyi seç, varlığı ekle. Bizde olmayan: arama, seçilen varlığın ızgarasının ve ölçeğinin açıkça kayda
geçmesi ve hesaplara verilebilmesi.

## Karar

### 1. Kapsam

Var: `cad.stac.search` (salt okunur arama), `cad.stac.prepare` (seçilen varlıkları adresleriyle bağlı raster olarak, kendi
grubunda ve tek adımda ekler, hazırlama kaydını verir), çekirdekte okuyucular (`kentos_services::stac`), masaüstünde
“STAC'ta ara” penceresi, MCP araçları, Python tipli katman. Yok: §Sonraya kalan.

### 2. Ağ ev sahibinindir

Çekirdek ve komutlar saftır; istek göndermez. `kentos_services::stac::Fetch` arayüzünü ev sahibi kaydeder
(`stac_host::register`): masaüstü ve MCP bulut istemcisinin HTTP'siyle (`kentos_cloud::stac::HttpFetch`: 30 saniye, 64 MB,
bir sunucuya en çok dört istek). Kaydı olmayan ev sahibi `stac_unavailable` der (Python). Testler için kayıtlı cevaplar:
`stac_host::Recorded` bir klasörün `index.json`'undan (yöntem, adres, kanonik gövde → dosya ya da hata) cevap verir;
`KENTOS_STAC_RECORDED` klasörü adlandırırsa kayıtlı ev sahibi yerine bu kullanılır (her aramada okunur).

### 3. Arama (`cad.stac.search`)

- **Uç nokta:** http(s) adresi, sunucu adı olan; kullanıcı adı, parola, sorgu ve parça yok (`invalid_stac_endpoint`); şema ve
  sunucu küçük harfe, sondaki “/” atılır.
- **İstek:** `POST <uç nokta>/search`, gövde kompakt JSON, anahtarlar alfabetik, tam sayı değerli sayılar noktasız:
  `bbox` (batı, güney, doğu, kuzey; derece; `invalid_stac_bbox`: sonlu, boylam ±180, enlem ±90, batı < doğu, güney ≤ kuzey),
  `collections`, `datetime`, `ids`, `limit` (1–100, varsayılan 10; `invalid_stac_limit`) ve bulut için
  `query: {"eo:cloud_cover": {"lte": n}}` (0–100; `invalid_stac_cloud_cover`). Koleksiyon ve öğe kimlikleri boş olmaz
  (`invalid_stac_collection`).
- **Tarih (`invalid_stac_datetime`):** RFC 3339 an ya da `başlangıç/bitiş`; uç `..` ya da boş açık; çıplak gün
  `YYYY-MM-DD` başlangıçta `T00:00:00Z`, bitişte `T23:59:59Z`, tek başına günün kendisi; `T` ve `Z` büyük harfe, kesir ve
  kayıklık yazıldığı gibi; iki ucu açık olmaz; başlangıç bitişten sonra olamaz (an olarak karşılaştırılır, kayıklık
  uygulanır); gerçek takvim (artık yıl).
- **Sayfalama:** cevabın `next` bağlantısı (yöntem, adres, gövde, `merge`) cevapta `next` olarak döner; girdiye `next`
  verilirse o istek gönderilir (POST'ta `merge` ise gövde aramanın gövdesinin üstüne katlanır). Adresi uç noktanın
  kaynağında (şema, sunucu, kapı) değilse `stac_next_foreign`: bir sunucunun sözü isteği başka yere göndermez. Göreli adres
  uç noktanın kaynağına çözülür.
- **Cevap:** `request` (gönderilen), `items`, `returned` (sunucunun gönderdiği), `dropped` (bulut oranı için süzülen),
  `matched` (`numberMatched` ya da `context.matched`), `next`. Bulut oranı gelende yeniden denetlenir; oranı olmayan öğe
  kalır. Kimliksiz öğe atlanır (`returned`'a sayılır). Cevap JSON değilse ya da `features` yoksa
  `stac_response_unreadable`, sunucu hata verirse `stac_request_failed`.
- **Retlerin sırası:** `invalid_stac_endpoint`, `invalid_stac_bbox`, `invalid_stac_datetime`, `invalid_stac_cloud_cover`,
  `invalid_stac_limit`, `invalid_stac_collection`, `stac_next_foreign`, `stac_unavailable`, `stac_request_failed`,
  `stac_response_unreadable`. `validate` yalnız isteği kurar.

### 4. Öğe ve varlık okuma

STAC 1.0 ve 1.1: `eo:bands` ve `raster:bands`, yoksa birleşik `bands` (`eo:common_name`, `eo:center_wavelength`, `data_type`,
`nodata`, `scale`, `offset`); `proj:epsg` ya da `proj:code` (`EPSG:n`), `proj:shape` (satır, sütun), `proj:transform` (ilk
altı sayı); varlığınki öğenin özelliklerininkini yener. Çekim zamanı `datetime`, yoksa `start_datetime` (bitiş
`end_datetime`). Lisans: öğenin `license`'ı ve `license` bağlantısı. Varlıklar anahtara göre sıralıdır; adresi olmayan
varlık okunmaz. Beklenen türden başka tür taşıyan değer yok sayılır.

### 5. Kalıcı kimlik ve imza

Varlığın kalıcı kimliği kataloğun adresi + koleksiyon + öğe + varlık anahtarıdır (+ imzasız adres). İmzalı adres geçici bir
kanıttır, yer değildir: sorgusu bir imza ya da belirteç taşıyorsa (`sig`, `signature`, `X-Amz-Signature`, `X-Amz-Credential`,
`X-Amz-Security-Token`, `AWSAccessKeyId`, `X-Goog-Signature`, `X-Goog-Credential`, `token`, `sas_token`, `access_token`,
`api_key`, `apikey`, `Key-Pair-Id`, `Policy`; ad büyük küçük harfe bakmaz) sorgunun tamamı atılır, yetki bilgisi (`kullanıcı:parola@`)
atılır ve `signed: true` işaretlenir. İmzasız sorgular (`?versionId=3`) kalır. İmza hiçbir cevapta, kayıtta, özniteliklerde
ve komut günlüğünde görünmez.

### 6. Hazırlama (`cad.stac.prepare`)

- **Girdi:** aramanın verdiği `item` (yalnız kullanılan varlıklar bulunsa yeter), `assets` (veri bantları; aynı anahtar bir
  kez), isteğe bağlı `qa` (`asset` ve `mask`), `search` (aramanın kaydı), `group`, `expectedRevision`.
- **Raster:** her varlık tek bantlı bir raster, adresi `url` (ADR 0207 §1); dosya açılmaz. `affine` = `[c, a, b, f, d, e]`
  (`proj:transform`'un `[a, b, c, d, e, f]`'sinden), genişlik = `proj:shape[1]`, yükseklik = `proj:shape[0]`, veri türü
  `data_type`'tan (`uint8, int8, uint16, int16, uint32, int32, float32, float64`), `srid` = EPSG. Görünüş gri; 8 bit değilse
  veri bandında yüzde 2–98, kalite bandında en küçük–en büyük gerdirme; `nodata` kataloğun nodata'sı.
- **Yerleşim:** tek yeni grup (`group`; yoksa `STAC: <öğe kimliği>`, en çok 120 harf, `invalid_group_name`) ve her varlık için
  anahtarını ad alan bir katman; veri bantları verilen sırayla, kalite bandı sonda. Tek geri alma adımı “STAC verisini
  hazırla”. Sistem projeninkinden başkaysa `crs_mismatch` (dönüşüm yok).
- **Kalite bandı:** maske kuralı verilmezse varlığın anahtarına göre (büyük küçük harf fark etmez): `scl` → Sentinel-2
  sınıfları 0, 1, 3, 8, 9, 10 (veri yok, doygun, bulut gölgesi, orta ve yüksek olasılıklı bulut, ince sirüs); `qa_pixel` →
  Landsat bitleri 0–4 (dolgu, genişletilmiş bulut, sirüs, bulut, bulut gölgesi). Başka varlığa kural verilir
  (`stac_qa_unknown`). Verilen kural sıralanır ve tekrarsız yazılır; boş ya da sınıf > 65 535 ya da bit ≥ 32 reddedilir. Maske
  bu komutta uygulanmaz, kaydedilir.
- **Retlerin sırası:** `invalid_stac_item`, `stac_assets_empty`, `stac_too_many_assets` (> 32), `stac_asset_not_found`,
  `stac_qa_is_data`, `stac_href_not_http`, `stac_no_grid`, `stac_multiband`, `stac_unsupported_sample`, `stac_qa_unknown`,
  `crs_mismatch`, `invalid_group_name`, `invalid_revision`, `revision_conflict`; her neden önce bütün varlıklarda aranır.
- **Uyarılar:** `stac_signed_href_stripped`, `stac_item_incomplete` (çekim zamanı ya da bulut oranı yok),
  `stac_grid_differs`, `stac_scale_differs`, `stac_qa_grid_differs`.

### 7. Hazırlama kaydı

`kentos.stac-preparation` 1: arama (`endpoint`, `bbox`, `datetime`, `collections`, `maxCloudCover`), `catalog`, `collection`,
`item`, `datetime`, `cloudCover`, `platform`, `license`, her bant (`asset`, `commonName`, imzasız `href`, `sample`, `nodata`,
`scale` ve `offset` (kataloğunki, yoksa 1 ve 0), `width`, `height`, `affine`, `epsg`, `layer`, `uid`), kalite bandı ve maske kuralı
(`defaultMask`: platformun kuralı mı) ve `reflectance` (bütün veri bantları `reflectance` rolünde ve aynı ölçek ve
öteleme taşıyorsa: Spektral indis'in Ölçek ve Öteleme'si). Kayıt her rasterin özniteliklerinde de durur: `stac.catalog`,
`stac.collection`, `stac.item`, `stac.asset`, `stac.href`, `stac.role` (`data` ya da `qa`), `stac.band`, `stac.datetime`,
`stac.cloud_cover`, `stac.platform`, `stac.scale`, `stac.offset`, `stac.nodata`, `stac.license`, `stac.search`,
`stac.qa_mask` (`values:0,1,3,8,9,10` ya da `bits:0,1,2,3,4`). `.kcad` şeması değişmez; projenin kaydı olarak şemaya yazılması
sonraya kalan.

### 8. Arayüz

`raster.stac` “STAC'ta ara” (`catalog/v1/additions.json`; CBS › Raster, CBS › Veri ve CAD › Ekle'nin Raster panellerinde
Sanal mozaik'in yanında; ikon `rasterStac`: kare tarlalı bir görüntü ve büyüteç). Pencere: STAC adresi, koleksiyon
(Sentinel-2 L2A ve Landsat C2 L2 düğmeleri), başlangıç, bitiş, en çok bulut, sayfa boyu, alan (“Görünümden al”: görünümün
kutusu projenin sisteminden WGS 84'e), Ara ve Sonraki sayfa; bulunan öğeler; seçilen öğenin varlıkları (Veri ve Kalite işaretleri,
ölçek · öteleme, piksel, notlar); grubun adı; komutun planı (`cad.stac.prepare`'in `plan`'ı) ve uyarılar; Hazırla. Öğe seçilince
kırmızı, yakın kızılötesi ve kalite bandı kendiliğinden işaretlenir.

### 9. Komutlar ve otomasyon

İki ürün komutu, katalog, yetenekler, ortak durumlar (§Doğrulama), Python tipli katman (`kentos.cad.stac`), MCP araçları
(`cad.stac.search` salt okunur), manifest (`raster.stac`: `semantic`, `via` iki komut).

### 10. Performans

Arama ağ gecikmesidir ve kendi iş parçacığında koşar (arayüz iş parçacığında değil). Okuma, öğe başına varlık sayısıyla doğrusaldır
(100 öğe × 40 varlık); hazırlama dosya açmaz ve ağa gitmez.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/cad_stac.rs` (türler, kodlar `codes.rs`, yetenekler `capabilities.rs`, katalog
  `catalog.rs`); TS türleri üretilmiş dosyalardır.
- **Çekirdek:** `crates/shared/services/src/stac/` (`endpoint`, `datetime`, `href`, `request`, `read`, `catalog`, `prepare`).
- **Komutlar:** `crates/native/application/src/stac_search.rs`, `stac_prepare.rs`, `stac_host.rs`; `registry.rs`.
- **Ağ:** `crates/native/cloud/src/stac.rs` (`HttpFetch`); masaüstü ve MCP kaydeder.
- **Masaüstü:** `apps/desktop/src/rasters/stac.rs`.
- **Otomasyon:** `python/kentos/cad/stac.py` (üretilir), `crates/native/mcp/src/tools.rs` (salt okunur işareti).
- **Seçilen ikon:** `rasterStac`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/stac_cases.py --check` (STAC belirtiminden, KentOS kodu olmadan; Python'un `datetime`
  ve `decimal`'iyle): uç noktanın normal biçimi (14), tarihler (22: artık yıl, kayıklıkla sıralama, kesir, açık uçlar),
  adreslerin imzasızı (12), arama istekleri (14: gövdenin anahtar sırası, `merge`, yabancı `next`), kayıtlı cevapların
  okunuşu (10 dosya). Çekirdeğin testi aynı dosyayı okur (`crates/shared/services/tests/all/stac.rs`, 5 test) ve
  başvuruyla uyuşur; ayrıca çekirdeğin 19 STAC birim testi.
- **Kayıtlı cevaplar** (`fixtures/services/v1/stac`): 3 gerçek Earth Search cevabı (19 Ekim 2026, kesilmiş: Sentinel-2 L2A iki
  sayfa, Landsat C2 L2), kök belge ve koleksiyonlar; `synthetic-*` dosyalarını betik yazar (imzalı SAS adresleri, STAC 1.1
  birleşik bands, ızgarasız ve desteklenmeyen varlıklar, GET `next`, bulut süzgecini yok sayan sunucu, boş sonuç, hata
  belgesi). Testler çevrimdışıdır; gerçek cevaplardan öğrenilenler: Sentinel-2'nin ölçeği 0,0001 ve ötelemesi −0,1
  (`earthsearch:boa_offset_applied`), Landsat'ın adresleri `s3://` (istek ödemeli) ve ızgarası öğenin özelliklerinde,
  Planetary Computer'da kalite anahtarı büyük harfli `SCL` (varsayılan kuralın anahtarı büyük küçük harfe bakmaz).
- **Ortak durumlar:** `cad.stac.search` 35, `cad.stac.prepare` 38 durum (`fixtures/commands/v1`), masaüstü koşucusunda
  (`every_command_case_matches_the_desktop_handlers`) ve Python'un tipli katmanında (`KENTOS_STAC_RECORDED`) geçer;
  yetenek testi (her ret ve uyarı kodu bir durumda ya da gerekçesiyle: `stac_unavailable` birim testinde).
- **Testler:** `kentos-native-application` 85 (bütün durumlar) ve 34 birim (2'si STAC), `kentos-services` 72 birim (19'u STAC) ve 15 (5'i STAC), `kentos-mcp` 21 (STAC testi dahil), Python 148,
  masaüstü `rasters::stac` 3; clippy `-D warnings` (services, contracts, application, cloud, mcp, desktop) temiz; `pnpm arch:deps`
  temiz; `python3 scripts/cad/manifest.py --check` ve `scripts/python/sdk.py --check` temiz.
- **Resimler** (`KENTOS_SHOTS_ONLY=stac-pencere,stac-hazirla,stac-sistem,stac-hazir cargo test -p kentos-desktop tools_screens -- --ignored`):
  `.run/shots/arac-stac-*` (1440×900 ve 1100×650, koyu ve açık). İncelemede bulunup düzelenler: çekim zamanı mikrosaniyeyle
  taşıyordu (saniyeye kısaltıldı); Sentinel-2'de hem `nir` hem `nir08` olduğundan öntanım iki yakın kızılötesi bandı işaretliyordu
  (`nir`); “Hazırla” resminin raster piramitleri gerçek ağa gidip sonraki resimlerin köşesinde kalıyordu (sahne adresleri
  kapalı bir kapıya çevirir).
- **Ölçülmeyenler:** gerçek sunucuyla canlı arama testi (testler çevrimdışı; gerçek cevaplar bir kez alınıp kaydedildi), süre
  ölçümü (ağ gecikmesi baskın; okuma 100 öğe × 40 varlıkta ölçülmedi), imzalı adresli dosyanın açılması (sonraya kalan 1),
  gerçek Planetary Computer cevabı (imzalı adresler belirtimden yazılmış sentetik cevaptır).
- **Yolda bulunan eski hatalar:** `catalog::tests::no_typed_name_names_two_things` (`karsilastir`: data.compare ve view.swipe),
  `catalog::tests::the_additions_hold_against_the_inventory` (`exportCsv` ikonu envanterde zaten var) ve
  `history::text::tests::the_titles_are_the_catalog_s` (cad.rasters.georef, file.export.csv, file.import.csv `TITLES`'ta yok)
  bu dilimden önce de düşüyor; bu dilimin eklediği iki komut `TITLES`'a yazıldı ve ek dosyası, `exportCsv` çıkarılınca testi geçiyor.
