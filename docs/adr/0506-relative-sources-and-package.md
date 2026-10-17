# ADR 0506: Bağlı kaynağın göreli yolu, yeniden bulunması ve Paketle

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-36'nın ikinci dilimi:
  - bağlı katmanın kaynağı projenin klasörüne göre de yazılır;
  - taşınan proje kaynağını açılışta bulur;
  - projeyi kullandığı dosyalarla bir klasöre toplayan Paketle.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0505 (PostGIS), 0192 (resim), 0204 (raster), 0207 (nokta bulutu),
  0225 (ekler), 0327 (bağlı tablo), 0253 §10 (dosya yazan komutlar).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - PostGIS tablosunun çevrimdışı kopyası (GeoPackage olarak pakete);
  - lisans ve erişim sınırına göre süzme;
  - stillerin, formların ve modellerin dış bağımlılıklarının pakete toplanması (bugün projenin içindeler);
  - pakette katmanın bağını gömülü kopyaya çevirme seçeneği.

## Bağlam

Bağlı katmanın kaynağı mutlak yolla saklanıyordu (ADR 0503). Proje başka bir klasöre ya da bilgisayara taşınınca
kaynak bulunamıyordu; kullanıcı Yeniden bağla ile tek tek göstermek zorundaydı ve masaüstünde bunun bir düğmesi yoktu.

Bağlı resim, raster, bulut, tablo ve ek dosyaları zaten çizimin klasörüne göre göreli olabiliyordu, ama projeyi ve
dosyalarını birlikte teslim etmenin yolu yoktu. Plan şunları ister: göreli yol, yeniden konumlandırma, içerik özeti
ve “Paketle” (QGIS'in Package Layers'ı, AutoCAD'in eTransmit'i). Parola projeye girmemeli.

## Karar

### 1. Kaynağın göreli yolu

- `SourceLink.relative`: kaynağın, proje dosyasının klasörüne göre yolu; parçalar `/` ile ayrılır.
- Kural (`relative_problem`, iki okuyucuda da):
  - boş değil, mutlak değil;
  - boş ya da `.` parçası yok;
  - `..` yalnız başta;
  - bir dosyayı gösterir.
- Veritabanı bağında göreli yol olmaz.
- Alan şema 73'ün içinde, isteğe bağlıdır. Şema 73 bu dalda doğdu ve henüz `main`'e girmedi, bu yüzden yeni şema
  gerekmedi.
- **Yazma:**
  - Kaydetme, her GeoPackage bağına göreli yolunu yazılan anlık görüntüde ekler (`source_places::with_relative`):
    masaüstünün kaydı ve başsız sunucunun `files::write`'ı.
  - Çizim değişmez; başka sürücüdeki dosyanın göreli yolu yazılmaz.
  - Yeniden bağlama eski göreli yolu düşürür, sonraki kayıt yenisini yazar.
- **Açılış** (`source_places::relocate`): yolu yerinde olmayan GeoPackage bağı, göreli yolunun gösterdiği dosyaya
  taşınır. Koşul, o dosyanın SHA-256'sının bağın sürümü olmasıdır: katmanın son eşleştiği dosya.
  - Aynı yerde başka içerikte dosya varsa bağlanmaz (`differs`), yeniden bağlanmayı bekler.
  - İkisi de yoksa bağ kalır (`missing`), nesneler çizimdedir.
  - Taşıma projenin ayarıdır, geri alma adımı değildir. Kaydetmeye değer bir değişikliktir: proje kirlenir.
- Masaüstü açılışta bulunanı, bulunmayanı ve farklı olanı bildirir. Başsız sunucu sonucu `Session::located`'da tutar.

### 2. Yeniden bağla

- Katmanlar'ın sağ tıkında Kaynak ▸ **Yeniden bağla…** ve `source.relink`: GeoPackage seçilir, `cad.sources.edit`'in
  `relink`'i çalışır.
- Aynı içerikteki dosyaya bağlanır, başkasını reddeder (`source_changed`).
- Veritabanı bağının adresi PostGIS'e bağlan ile değişir.

### 3. Paketle (`file.package`)

- Dosya yazan komut (ADR 0253 §10): sorgu etkili, çizimi değiştirmez, geri alma adımı değildir. Başsız sunucu, Python
  ve MCP'de katalogdan gelir.
- **Girdi:**
  - `path`: paketin klasörü, mutlak; yok ya da boş olmalı (`file_exists`);
  - `name`: proje dosyasının adı, yoksa çizimin adı, `.kcad` eklenir;
  - `from`: göreli bağların okunacağı, proje dosyasının klasörü.
- **Plan:** çizimin anlık görüntüsünü dolaşır.
  - Çizimin ve blokların nesnelerinde:
    - resmin ve rasterin `file`'ı;
    - rasterin dünya dosyaları (`.wld`, `.pgw`, `.jgw`, `.tfw` …) ile `.aux.xml` ve `.ovr`'si, kopyanın adıyla;
    - bulutun `sources[].file`'ı;
    - bağlı tablonun `source.path`'i;
    - eklerin `file`'ı.
  - Ayarlarda bağlı katmanların GeoPackage'ları.
  - Her dosya `veri/` altına bir kez kopyalanır. Adı başka bir dosyanınsa `ad-2.uzantı` alır; karşılaştırma büyük
    küçük harfe duyarsızdır, böyle bir diskte de ayrı kalsınlar diye.
  - Bağlar kopyalara yazılır:
    - bağlı dosyalar klasöre göre göreli (`veri/…`);
    - bağlı katmanın kaynağı kopyanın mutlak yolu, göreli yolu `veri/…`. Klasör nereye giderse açılışta bulunur (§1).
- **Kopyalanmayanlar** bağlarıyla kalır, her biri söylenir (`left`, uyarı `package_left`):
  - veritabanı tablosu (`database`): bağ kalır, paketi açan bilgisayar veritabanına kendisi ulaşır;
  - adres (`address`): resmin ve bulutun `url`'si;
  - yerinde olmayan dosya (`missing`).
  - Parola ve belirteç projede olmadığından pakete girmez.
- **Yazma:** klasör bütün yapılır ya da hiç.
  1. Hedefin yanında geçici bir klasör açılır.
  2. Kopyalar yazılır; her biri geri okunur, boyu ve SHA-256'sı planınkiyle karşılaştırılır. Kopyalanırken değişen
     dosya paketi durdurur.
  3. Proje dosyası doğrulanarak yazılır (`files::write_bytes`).
  4. Klasör tek adımla yerine konur. Hata olursa geçici klasör silinir.
- Büyük dosyalar 1 MiB'lık parçalarla özetlenir (`encoding::sha256_reader_hex`; sözleşme zaten `sha2`'ye bağlıydı,
  yeni bağımlılık yok).
- Kaydın dosyası (`FileEffect`) proje dosyasıdır; kopyalar cevapta tek tek yazılır.
- **Masaüstü:** Dosya alışverişi'nde **Paketle…** (ikon `projectPack`). Klasör sorulur, iş çizimin kopyasında,
  arayüzün iş parçacığının dışında yapılır. Sonuç ve kalanlar söylenir.

## Doğrulama

- `crates/native/headless/tests/headless.rs`'in `a_project_is_packed_with_its_files_and_opens_where_the_folder_goes`'i:
  - proje: bağlı GeoPackage, iki kez bağlı bir resim (göreli), dünya dosyalı raster, yerinde olmayan dosya ve
    veritabanı bağı;
  - plan bir şey yazmaz; paket, kopyalar, kalanlar ve uyarı denetlenir; ikinci kez `file_exists`;
  - klasör taşınıp eski veri silinince açılış bağı kopyaya taşır ve `cad.sources.describe` aynı der;
  - göreli `path` reddedilir.
- `scripts/fixtures/package_cases.py`, KentOS kodu olmadan aynı paketi okur (bağımsız KCAD okuyucusu ve hashlib):
  - dosyalar tam bu dördü, kaynaklarıyla bayt bayt aynı;
  - bağlar paketin içinde;
  - bağlı katmanın göreli yolu ve sürümü doğru;
  - kalanlar olduğu gibi.
- `source_places`'in birim testi: göreli yolların yazılması ve çözülmesi.
- Masaüstü (`gpkg_tests`): Yeniden bağla aynı dosyayı alır, başkasını reddeder; paketin sonucu bildirilir.
- KCAD örnekleri: `sources.kcad`'in ilk bağı göreli yollu. Bozuk örnekler: mutlak, ortasında `..` ve veritabanının
  göreli yolu (`broken/sources-relative-absolute.kcad`, `sources-relative-climbs.kcad`,
  `sources-database-relative.kcad`).
