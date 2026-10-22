# ADR 0582: PostGIS'te alan, view ve sorgu katmanı, satır düzeyinde çakışma

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-04'ün kalanı: milyon satırlı tabloda bütün satırları indirmeden ilk
  harita, view ve SQL sorgu katmanı, QGIS'le aynı tabloyu aynı anda düzenleme, milyon satır ölçümü.
- **Bağlam belgesi:**
  - ADR 0505: PostGIS bağlı katmanı;
  - ADR 0510: bağlı katmanın parçası;
  - ADR 0538: kaynakla uzlaştırma ve nesne başına taban;
  - ADR 0581: kaynak sorgusu.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Sonraya kalan:**
  - tetikleyici ya da outbox ile değişiklik bildirimi: bugün değişiklik, kaynağın durumu sorulunca alanın sürümüyle bulunur
    (yoklama); `LISTEN/NOTIFY` dayanıklı geçmişin yerine geçmez;
  - yazılabilir view (bugün view ve sorgu katmanı salt okunurdur);
  - birden çok geometri sütunu olan tabloda sütun seçimi;
  - GeoPackage'ta satır düzeyinde çakışma: GeoPackage tek kullanıcılı bir dosyadır, dosyanın sürümü kalır.

## Bağlam

ADR 0505 PostGIS tablosunu bütün olarak okur ve yazar. Bunun iki sonucu var:

- **Bütün tablo okunur.** Milyon satırlı kurum tablosu bağlanınca bütün satırlar çizime iner. ADR 0581'in ölçümünde bu 11,8 s ve
  4,3 GB'tır. ADR 0510'un alan sınırı yalnız GeoPackage'ta vardı.
- **Sürüm bütün tablonundur.** Sürüm, satırların SHA-256'sıdır. QGIS'te biri tablonun herhangi bir satırını değiştirince KentOS'un
  her yazması `source_changed` ile reddedilir; bu, sessiz ezmeyi önler ama iki düzenleyicinin ayrı satırlarda çalışmasını da
  önler. Her yazma da bütün tabloyu okuyup özetler.

View ve sorgu katmanı hiç yoktu. Sağlayıcı yalnız tek tam sayı birincil anahtarlı tabloları listeler.

## Karar

### 1. PostGIS'te alan

- `cad.sources.edit`'in bağlaması ve `extent` işlemi veritabanı tablosunda da alan sınırı alır. Kural ADR 0510'unkidir.
- Sağlayıcının okuması alanı alır:
  - `read(adres, tablo, alan)`;
  - alan, tablonun sisteminde `geom && ST_MakeEnvelope(…)` ile süzülür, mekânsal dizin varsa ondan;
  - geometrisi olmayan satır alanla gelmez.
- **Alanın sürümü:** okunan satırların SHA-256'sı, alanın kendisiyle birlikte. Alanın dışında değişen satır alanın sürümünü
  değiştirmez.

### 2. Satır düzeyinde çakışma (yalnız veritabanı)

Yazma, `LOCK TABLE … IN SHARE ROW EXCLUSIVE MODE` altında tek işlemdir (ADR 0505). İşlemin içinde:

1. Güncellenecek ve silinecek satırlar anahtarlarıyla yeniden okunur.
2. Uygulama her birini tabanla karşılaştırır (ADR 0538: nesnenin tablodaki özeti, son eşleşmede):
   - satır tabanın özetinden farklıysa ya da artık yoksa başka biri onu değiştirmiştir;
   - böyle bir satır varsa işlem geri alınır ve hiçbir şey yazılmaz. Ret `source_changed`'dir; `uids` değişen satırların
     nesneleridir.
3. Değişen satır yoksa değişiklikler yazılır. Yazılmayan satırları başkası değiştirmiş olabilir; bu yazmayı engellemez.
4. Alanın sürümü yazmadan önce bağın sürümünden farklıysa, başka biri alanda başka satırları değiştirmiştir:
   - yazma tamamlanır ve `source_others` uyarısı verilir;
   - bağın sürümü değişmez, böylece kaynağın durumu “değişti” der;
   - okuma (`direction: read`) o satırları katmana getirir.

Taban yoksa (katman `MAX_BASE_ROWS`'tan büyük ya da taban eski) ADR 0505'in kuralı sürer: alanın sürümü bağınkiyle aynı
olmalıdır.

GeoPackage değişmez: dosya tek kullanıcılıdır, sürüm dosyanın SHA-256'sıdır.

### 3. View ve sorgu katmanı

`table` iki biçim daha alır:

- **View:** `şema.view` (görünüm ya da somut görünüm).
- **Sorgu:** `SELECT` ya da `WITH` ile başlayan SQL. `SELECT * FROM (<sql>) AS q` olarak okunur.
  - Okuma `READ ONLY` işlemde olur. Sorgu veritabanında hiçbir şey değiştiremez; deneyen sorguyu PostgreSQL reddeder.
  - Noktalı virgül ve birden çok deyim reddedilir.

Her ikisinde:

- **Anahtar:** tam sayı bir sütun, sırayla `fid`, `id`, `gid`, `ogc_fid`. Yoksa ret `no_key`. Aynı anahtar iki satırda
  görülürse ret `key_not_unique`.
- **Geometri:** türü `geometry` olan ilk sütun.
  - View'da sistem `geometry_columns`'tan gelir.
  - Sorguda sistem ilk geometrinin `ST_SRID`'sidir. Bağın `srid`'i onu söyler; sorgu katmanının satırları farklı sistemlerde
    olmamalıdır.
- **Salt okunur.** Yazma `source_readonly` ile reddedilir.
- **Kalıcı kimlik:** `database_row_uid(veritabanı, ad, anahtar)`. Sorgunun adı SQL'in SHA-256'sının ilk 16 hanesidir: aynı sorgu
  aynı kimlikleri verir.
- **Listeleme:** `tables` view'ları da listeler (anahtarı ve geometrisiyle). Anahtarı olmayan view listelenmez. Sorgu
  listelenmez; yazılarak bağlanır.
- Sorgunun sütunları geçici bir view'la bulunur: view geri alınan bir işlemde yapılır, sorgu çalışmaz. Yazan bir sorguyu
  PostgreSQL view olarak kabul etmez.
- **Kaynak sorgusu:** `cad.sources.query` (ADR 0581) view ve sorguyu da sorgular: sorgu `(… ) AS q` alt sorgusu olur, süzgeç
  ve kutu onun üstünde.

### 4. Masaüstü

PostGIS'e bağlan penceresi:

- View'ları tablolarla birlikte listeler. View'ın satırında “salt okunur” rozeti vardır.
- **Sorgu katmanı:** SQL yazılır, Dene ilk satırları ve sütunları gösterir, Bağla bağlar.
- **Yalnız görünen alanın satırları:** GeoPackage penceresindeki gibi. Geometrili tablo ve view'da kullanılır; sorguda da
  kullanılır.

Kaynak durumu penceresi ve bağlı katmanın kaynak satırı `source_others`'tan sonra “değişti” der ve Tablodan oku'yu önerir
(ADR 0543).

### 5. Ölçüm

`kentos-postgis`'in `postgis_timing` testi, geçici veritabanında `generate_series` ile 1 000 000 parsellik tablo kurar. Ölçülenler:

- görünen alanla bağlama (ilk harita);
- alanın sürümü;
- tek satırın yazılması, satır denetimiyle;
- kaynak sorgusunun ilk sayfası ve bütün tablonun istatistiği.

## Doğrulama

- `postgis_cases.py` (geçici docker PostGIS):
  - tablo, view ve sorgu katmanının bağlanması, alanla bağlama;
  - view ve sorgunun salt okunurluğu;
  - `no_key` ve `key_not_unique`.
- **QGIS'le eşzamanlı düzenleme:** QGIS 4.2.3'ün PostgreSQL sağlayıcısı (PyQGIS) aynı tabloyu düzenler. Senaryo:
  1. KentOS parseli alanıyla bağlar, iki satırı düzenler.
  2. QGIS aynı satırlardan birini ve başka bir satırı değiştirip kaydeder.
  3. KentOS'un yazması reddedilir; `uids` QGIS'in değiştirdiği ortak satırdır. Tablo QGIS'in hâlindedir.
  4. KentOS ortak satırdaki değişikliğini bırakıp yazar. Yazma geçer, `source_others` uyarısı gelir; QGIS'in öbür satırı
     ezilmemiştir.
  5. Okuma QGIS'in satırlarını katmana getirir. Tablo psycopg2 ile denetlenir.
- Sağlayıcının birim testleri: sorgu metninin kuralları (tek deyim, `SELECT`/`WITH`), anahtarın seçimi.
- Süre (`postgis_timing`, 22 Ekim, release, geçici PostGIS 18 ve 3.6, 1 000 000 parsel, GiST dizinli):

  | İş | Süre |
  |---|---|
  | Görünen alanla bağlama, ilk harita (374 nesne) | 51 ms |
  | Alanın durumu | 24 ms |
  | Bir satırın yazılması, satır denetimiyle | 75 ms |
  | Kaynak sorgusu, kutu ve süzgeç, ilk sayfa (357 satır okundu) | 42 ms |
  | Kaynak sorgusu, bütün tablonun süzgeçli istatistiği (500 000 satır okundu) | 1 404 ms |

  Bütün tabloyu bağlamak GeoPackage'ta 11,8 s ve 4,3 GB'tı (ADR 0581); alanla bağlama tablonun boyundan bağımsızdır.
