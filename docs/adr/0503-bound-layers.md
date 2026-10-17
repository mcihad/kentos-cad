# ADR 0503: Bağlı katman — GeoPackage tablosunun çalışma kopyası

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-01, CBS-02, CBS-11 ve CBS-36'nın GeoPackage dilimi; CBS-03'ün
  “dosyada yerinde düzenleme” kalanı.
- **Bağlam belgesi:** ADR 0502 (GeoPackage), 0253 (komut protokolü), 0265 (yeni şemanın altı adımı), 0410 (geometrisiz
  kayıt), 0199 (katman alanları).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - PostGIS sağlayıcısı (CBS-04, sqlx ve TLS; sahibin kararı) ve servis kaynakları;
  - kaynak gezgininde dosyanın katmanlarını eklemeden önizleme (CBS-01'in kalanı);
  - kaynağa itilen bbox ve sütun sorgusu, sayfa sayfa okuma, nesne nesne üç yollu birleştirme (CBS-02, CBS-37);
  - taşınabilir paket ve göreli yol (CBS-36'nın kalanı).

## Bağlam

ADR 0502 GeoPackage'ı kopyalayarak alır ve yeni dosya olarak yazar. Plan bundan fazlasını ister:

- dosyanın tablosu açılıp düzenlenebilmeli ve aynı dosyaya geri yazılabilmeli (CBS-01, 03, 11);
- proje kapatılıp açılınca bağ korunmalı, taşınan dosya yeniden konumlandırılabilmeli (CBS-01, 36);
- kaynağın neyi yapabildiği ayrı ayrı söylenmeli, desteklenmeyen yazma gerekçesiyle reddedilmeli (CBS-02);
- yerel geri alma ile kaynağa commit ayrı durumlar olmalı; iki düzenleyici birbirini sessizce ezmemeli (CBS-11).

## Karar

### 1. Bağ projenin ayarıdır; nesneler çizimde kalır

Projenin `sources`'ı (`kentos_contracts::SourceLink`) her bağlı katman için şunları tutar:

| Alan | Anlamı |
|---|---|
| `id` | `kaynak-N` |
| `kind` | `geopackage` |
| `path` | dosyanın mutlak yolu |
| `table` | dosyadaki tablo |
| `layer` | çizimdeki çalışma kopyası |
| `revision` | dosyanın son eşleşmedeki SHA-256'sı |
| `digest` | katmanın son eşleşmedeki özeti |
| `srid` | tablonun bildirdiği EPSG |
| `writable` | yazılabilir mi |

- Katmanın nesneleri belgede kalır: dosya yerinde değilse proje yine nesneleriyle açılır; erişilemeyen kaynak boş katman
  diye kaydedilmez.
- Parola yoktur: GeoPackage parola istemez.
- `.kcad` şema 73 bu listeyi taşır; yalnız bağ varken yazılır. Daha eski okuyucu bağı sessizce düşürmez, bilinmeyen alan
  diye reddeder. Bağımsız Python okuyucusu ve yazıcısıyla birlikte değişti; örnek dosya `sources.kcad`, dört kırık örnek
  (ADR 0265).
- Bağ geri alma adımı değildir (proje ayarı). Bağlamanın satırları alması ve okuma birer geri alma adımıdır.

### 2. İki tarafın sürümü: dosyanın SHA-256'sı ve katmanın özeti

Kopya tutmadan iki tarafın değişikliği ayrılır:

- **Dosya:** son eşleşmeden beri başka bir program dosyayı değiştirdiyse SHA-256 farklıdır. Yazma `source_changed` ile
  çakışma olarak reddedilir.
- **Katman:** özet, nesnelerin türü, geometrisi ve özniteliklerinin kalıcı kimlik sırasıyla kanonik JSON'unun
  SHA-256'sıdır (görünüş dahil değil; `sources::layer_digest`). Katman son eşleşmeden beri değiştiyse okuma
  `pending_changes` ile çakışmadır; `discard` ile sürer.

Bağlama, yazma ve okumadan sonra ikisi de yenilenir.

### 3. Katman ile tablo satır olarak karşılaştırılır

- Nesne ile satır kalıcı kimlikle eşlenir: `kentos_uids` ya da ADR 0502'nin UUIDv5'i.
- Nesne, yazılacağı satıra çevrilip (`geopackage::row_for`) dosyanın satırıyla karşılaştırılır. Böylece metin ve sayı
  dönüşümleri yalancı fark üretmez.
- Koordinatlar çekirdeğin toleransında (1e-6 m) aynı sayılır: yayın orta noktası bulge'dan yeniden hesaplanınca son
  bitleri değişebilir.
- Tam sayı ile ondalık sayı değerce karşılaştırılır.
- Boş geometri geometrisiz kayıtla aynıdır: içe aktarma onu kayıt yapar.
- Okunamayan satırlar karşılaştırmaya girmez: yazma onları silmez.

### 4. Komutlar

- **`cad.sources.describe`** (salt okunur):
  - `path` verilirse dosyanın tablolarını söyler: satır sayısı, geometri türü, EPSG, sütunlar (bildirimi ve alan türü) ve
    yetenekler (okuma, ekleme, güncelleme, silme, tek işlem, Z, M, eğri);
  - verilmezse her bağın durumunu söyler: `found`, `changed`, `edited` ve tabloda olmayan, farklı ve katmanda olmayan
    nesne sayıları.
- **`cad.sources.edit`:**
  - `link`: tabloyu yeni bir katmana alır (`file.import.gpkg`'nin yolu; boş tablo boş katman; geometrisiz tabloda
    sistem uyarısı yok) ya da tablonun nesnelerini olduğu gibi taşıyan katmanı bağlar (`source_differs`);
  - tablo projenin sisteminde olmalı (`crs_mismatch`): bağlı katman çevrilmez;
  - `unlink` bağı çözer, nesneler kalır;
  - `relink` taşınan dosyanın yeni yeridir; içerik aynı olmalı (`source_changed`);
  - bir tablo ve bir katman tek bağa (`source_bound`), en çok 1000 bağ (`too_many_sources`).
- **`cad.sources.sync`:**
  - `write`:
    - farkı tek SQLite işleminde yazar (`GeoPackage::edit`): katmanın tabloda olmayan alanları `ALTER TABLE` ile
      sütun olur; değişenler fid'leriyle güncellenir; yeniler kalıcı kimlikleriyle eklenir (`kentos_uids`, yoksa
      açılır); silinenler silinir;
    - mekânsal dizini GeoPackage'ın kendi tetikleyicileri izler; tanınmayan tablolara ve uzantılara dokunulmaz;
    - geometrisi sütuna uymayan nesne `geometry_mismatch`;
    - tablonun tutmadığı nesneler ve alanı olmayan öznitelikler çizimde kalır (`export_losses`);
    - salt okunur kaynak `source_readonly`.
  - `read`: tablonun satırlarını katmana alır. Görünüş kalır, yeniler eklenir, tabloda olmayanlar silinir; tek geri alma
    adımıdır (“Kaynaktan oku: tablo”).
  - Dosya yerinde değilse `source_gone`.

Planlar dosyayı okur, hiçbir şey yazmaz.

### 5. Masaüstü

- GeoPackage penceresinde **Bağlı katman olarak aç**; **Kaynağa bağla…** (`source.link`) pencereyi seçenek işaretli
  açar. Başka sistemdeki dosya bağlanmaz.
- Katmanlar'da bağlı katmanın rozeti (dosya, tablo, yol) ve sağ tık **Kaynak ▸**: Kaynağa yaz, Kaynaktan oku, Kaynak
  durumu, Bağı çöz.
- Şeritte CBS › Veri › Dosya alışverişi: Kaynağa bağla, Kaynağa yaz, Kaynaktan oku (ayrı panel Veri sekmesini 1100 px'ten taşırıyordu).
- Katmanda yazılmamış değişiklik varken okuma sorar (“Değişiklikleri at ve oku”).
- Sonuçlar bildirimle söylenir.

### 6. Doğrulama

- Ortak durumlar KentOS kodu olmadan yazıldı (`scripts/fixtures/bound_sources_cases.py`). Sayılar şuradan:
  - `cad.sources.describe` 24, `.edit` 26, `.sync` 21;
  - tablolar Python'un sqlite3'üyle dosyanın kendi tablolarından;
  - nesneler ADR 0502'nin OGR başvurusundan;
  - sürümler hashlib'den.
- Dosyaya yazan `execute` ortak durumlarda yok (fixture dosyası değişirdi). Onu geçici kopyada
  `crates/native/application/tests/all/sources.rs` sınar: yaz, dosyayı yeniden bağla, başka programın değişikliği,
  çakışma, okuma, geri alma.
- Yazılan `kentos-synced.gpkg` ve crate'in yerinde düzenlediği `kentos-edited.gpkg` iki yoldan denetlenir:
  - OGR doğru okur ve mekânsal süzgeç yeni yerleri bulur;
  - GDAL'ın doğrulayıcısı QGIS'in dosyasındakinden fazla bulgu vermez (`geopackage_cases.py`).
- Masaüstünde bağlama, yazma, sorulu okuma ve bağı çözme testi; resimler `gpkg-*-4-bagla`, `-5-bagli`, `-6-soru`.

## Sonuçlar

- GeoPackage tablosu içe aktarmadan düzenlenip aynı dosyaya yazılır.
- QGIS'in dosyayı değiştirmesi algılanır; iki yazar birbirini sessizce ezmez.
- Karşılaştırma bütün tabloyu okur. Milyon satırlık tabloda kaynağa itilen sorgu ve sayfalama, PostGIS sağlayıcısıyla
  birlikte gelecek (CBS-02, CBS-42).
