# ADR 0508: İşlemler'in araçları komut olarak: cad.processing.tools ve cad.processing.run

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-41'in ilk dilimi. İşlemler'in yerleşik araçları sürümlü,
  tipli bir komutla Python'a ve yapay zekâ ajanına açılır:
  - araç keşfi;
  - açık değerler;
  - kapsam;
  - etki sayıları;
  - ret kodları;
  - plan.
- **Bağlam belgesi:** ADR 0084 (İşlemler), 0124 (arka planda çalışma), 0131 (Python SDK), 0133 (MCP), 0253 (komut
  protokolü), 0505 (sağlayıcının kaydı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - başsız ev sahibinin dosyaları: raster, nokta bulutu ve NetCDF okuyan araçlar başsızda `tool_failed` verir;
  - modellerin komutla çalışması;
  - masaüstünün İşlemler penceresinin yazmasının komut kaydına (`kentos.calls/1`) girmesi;
  - uzun işin `start`/`job` yolu;
  - kapsam matrisinin işlem + seçenek + veri türü + ev sahibi düzeyinde tutulması.

## Bağlam

İşlemler'in yüzlerce aracı masaüstünün penceresinden çalışıyordu. Python'un ve MCP'nin onlara yolu yoktu. Plan şunu
ister: UI hesapları Python ve AI kullanıcısına yeniden yazdırılmasın; ekrana ait seçim ve görünüm açık girdi olsun;
başsız yol fare istemesin.

İşlemler crate'i uygulama katmanının üstündedir: araçları belgeye uygulamanın yoluyla yazar. Bu yüzden komutların
işleyicisi araçları doğrudan çağıramaz.

## Karar

### 1. Kayıt

- Uygulama katmanı `processing_host::ToolHost`'u tanımlar: `list`, `check`, `run`.
- İşlemler crate'i `command::register()` ile yerleşik araçlarını kaydeder. Bu, PostGIS sağlayıcısının kaydının
  yoludur (ADR 0505).
- Kaydı yapanlar:
  - masaüstünün `main`'i;
  - başsız sunucunun her oturumu (`Session::of`), dolayısıyla Python, MCP ve masaüstünün ajan bağlantısı;
  - uygulamanın ortak durum koşucusu.
- Kaydedilmeyen ortamda iki komut da `tools_unavailable` ile reddedilir.

### 2. `cad.processing.tools`

- Araçları listeler: kimlik, ad, kategori yolu, açıklama, takma adlar.
- Her parametre için: ad, etiket, tür, açıklama, isteğe bağlılık, Gelişmiş ayarlarda olup olmadığı, bu çizimdeki
  varsayılan, seçimin değerleri.
- `query` İşlemler penceresinin aramasıdır (sözcükler, Türkçe harfler katlanır); tam yazılan kimlik ya da takma ad da
  bulur. `category` bir kategoriyi alt kategorileriyle verir; bilinmeyen kategori `invalid_input`.

### 3. `cad.processing.run`

- Pencerenin çalıştırıcısıdır (`Runner::run`): aynı doğrulama, aynı hesap, aynı tek geri alma adımı (adı aracın adı).
- Verilmeyen parametre çizimin varsayılanını alır.
- **Ekran okunmaz:**
  - nesne girdisi `all`, `layer`, `uids` (kalıcı kimlikler) ya da `selection` ve `visible`;
  - son ikisi girdinin kendi `selection`'ı (kimlikler) ve `view`'u (kutu);
  - yuvayla `ids` reddedilir; `view`'suz görünen alan reddedilir.
- Dosya parametresi `{ path }` ise tablo dosyası pencerenin okuduğu gibi okunur (`Limits::PROCESSING`). `{ name, rows }`
  olduğu gibi geçer.
- **Retler sırayla:**
  - `tools_unavailable`, `unknown_tool`;
  - `invalid_uid`, `entity_not_found` (seçimde ya da nesne girdisinde, yeriyle);
  - `invalid_parameters`: olmayan parametre, aracın kuralı, okunmayan dosya; ilk sorun parametresiyle;
  - `tool_failed`: araç çalıştı ve durdu (kendi reddi, alanın kuralı); çizim değişmez.
- **Cevap:**
  - özet;
  - eklenen, değişen ve seçilen nesneler kalıcı kimlikleriyle. Çalıştırıcının `Outcome::Ok`'u artık değişenleri
    seçilenlerden ayrı verir (`updated`);
  - açılan katmanlar;
  - çıktılar (nesne çıktıları kalıcı kimliklerle);
  - notlar (düzey ve metin);
  - adımın adı, sürüm.
- `plan` aracı çizimin okuma kopyasında çalıştırır. Aynı algoritmayla ne yazılacağını söyler, yeni nesnelerin
  kimlikleri kopyanındır; çizim değişmez.
- Python `kentos.cad.processing.run` ve `.tools` katalogdan üretilir. MCP'de araçlar katalogdan gelir;
  `cad.processing.run` yıkıcı listededir.

### 4. Sonucu dosyaya yazma (`sink`, CBS-23'ün ilk dilimi)

- `sink: { path, overwrite }` verilince araç çizimin okuma kopyasında çalışır. Kopyada aracın yaptığı nesneler dışındakiler
  bırakılır; yaptıklarının katmanları yeni bir GeoPackage'a yazılır (`geopackage::write`, `file.export.gpkg`'nin
  dönüşümü):
  - her katman bir tablo, katmanın adıyla;
  - katmanın alanları sütun, kalıcı kimlikler `kentos_uids`'te;
  - yaylar dairesel eğri olarak kesin;
  - yazı, ölçü, blok ve tarama türlerine göre sayılır (`export_losses`).
- Çizim değişmez, geri alma adımı yoktur. Cevabın `sink`'i `file.export.gpkg`'ninkidir: yol, boy, SHA-256, tablolar ve
  satırları, kalanlar.
- **Dosya bütün yazılır ya da hiç:** hedefin yanında geçici adla yazılır, tek adımla yerine konur. Var olan dosya yalnız
  `overwrite` ile değişir ve yenisi tamamlanana dek eski dosya kalır. `plan` dosyayı yazar, sayar ve siler.
- **Retler:**
  - `invalid_file_path`: mutlak değil, `.gpkg` değil ya da klasörü yok;
  - `file_exists`;
  - `file_unwritable`;
  - `nothing_to_export`: araç yeni nesne yapmadı ya da yaptıklarının GeoPackage'da geometrisi yok.
- **Soy:** cevabın `inputs`'u aracın okuduğu katmanları sırasıyla verir. Bağlı katmanda kaynağın kimliği ve okunan
  sürümü de yazılır (ADR 0503).
- **Sonraya kalan:**
  - var olan GeoPackage'a tablo eklemek;
  - kaynağı sayfa sayfa okuyan, sonucu parça parça yazan akış;
  - disk üstünde ara depo;
  - işin iptalinin dosyaya etkisi (bugün iş tek parçadır, iptal yalnız İşlemler penceresindedir);
  - PostGIS'e yazmak.

## Doğrulama

- `fixtures/commands/v1/cad.processing.run.json`: 49 durum, `scripts/fixtures/processing_command_cases.py`'den. Aracın
  hesabı ikinci kez yazılmaz:
  - 30 durum İşlemler'in kendi bağımsız ortak durumlarıdır (`fixtures/processing/v1/cases.json`, `queries.json`,
    `generate.json`, `geometry.json`), komutun sözleriyle: seçim kimliklerle, nesne çıktıları kimliklerle, değişen ve
    eklenen nesneler yuvalarıyla, adım ve geri alma;
  - yalnız beklentisi bütün sonucu söyleyen durumlar alınır (çıktıları olan aracın çıktıları yazılmış olmalı);
  - kalan 10 durum komutun kendi kurallarıdır: retlerin sırası, `uids` kapsamı, `view`'suz görünen alan, yuvayla
    `ids`, plan, validate.
- `fixtures/commands/v1/cad.processing.tools.json`: 23 durum. Araçların kimlikleri, adları, takma adları ve
  parametreleri (ad, etiket, tür, isteğe bağlılık, sıra) web'in donmuş envanterinden (`docs/inventory/web.json`,
  web'in TypeScript tanımlarından). Masaüstünün sonradan eklediği parametre listede gerekçesiyle durur (Anahtarla
  birleştir'in Yinelenen anahtar'ı, ADR 0439).
- Yetenek testi: her kod bir durumda. `tools_unavailable` uygulamanın kendi testindedir (`processing_run.rs`: orada
  araçlar kaydedilmez).
- Python ortak durumları SDK'dan da koşar (`test_command_cases.py`).
- **Dosyaya yazma:**
  - planlı dört durum aynı üreticiden: G1'in parsel × imar kesişimi, tampon, Ada'ya göre birleştirme ve yazısı kalan
    numaralama; beş ret;
  - uygulamanın `processing_sink` testi G1'i gerçekten yazar: dosya cevabın boyu ve SHA-256'sıyla aynı, çizim
    değişmez, geçici dosya kalmaz, var olan dosya `overwrite`'sız değişmez;
  - `KENTOS_WRITE_GPKG=1` ile yazılan `fixtures/geopackage/v1/kentos-run-*.gpkg` dosyalarını
    `processing_sink_cases.py` GDAL'ın OGR'ıyla okur. Parçaların alanları kaynakların OGR kesişimi, tamponlar OGR'ın
    1024 parçalı tamponu (%0,05), birleşimler grubun OGR birleşimi; öznitelikler İşlemler'in durumlarının dediği gibi
    (iki yanın soyu, paylaştırılan Değer); sistem EPSG:5254, her satır geçerli, `kentos_uids` her satırı adlandırır.
