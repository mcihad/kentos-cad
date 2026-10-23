# ADR 0598: Bölmede ve birleştirmede soy bağı, kaynağa yazılmış adımın geri alınması

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-11'in kalanı:
  - parsel bölünürken eski ile yeni nesnelerin bağı;
  - sunucuda (bağlı kaynakta) onaylanmış değişikliğin geri alınması.

  ADR 0590'ın çoktan çoğa bağlantı tablosuyla CBS-11 kapanır.
- **Bağlam belgesi:**
  - ADR 0419: alanın bölme ve birleştirme kuralı;
  - ADR 0503: bağlı katman;
  - ADR 0538 ve ADR 0553: taban ve uzlaştırma;
  - ADR 0582 ve ADR 0584: PostGIS'te satır düzeyinde denetim ve birlikte yazma.
- **`.kcad` şeması:** 101 (ayarların `lineage`'i).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.

## Bağlam

Bölme ve birleştirme şimdiye dek nesneleri doğru yazıyordu:

- ilk parça kaynağın kimliğini korur, öbürleri yeni kimlik alır;
- alanlar ADR 0419'un kuralıyla paylaşılır;
- form ve ilişkiler aynı adımda izler.

Ama yeni parçanın hangi nesneden geldiği hiçbir yerde kalmıyordu. Kadastroda bu bağ işin kendisidir: ifrazın parselleri
kökünü, tevhidin parseli birleşenlerini bilir.

Yerel geri alma çizimindir. Kaynağa yazılmış bir adımın geri alınması da bir yazmadır; kaynakta o arada başka biri
değiştirmiş olabilir. Bu da açıkça söylenmeliydi.

## Karar

### 1. Soy kaydı projenin ayarıdır

Ayarların `lineage`'i (`kentos_contracts::LineageRecord`) adımların yaptığı sırayla kayıtlardır. Her kayıt:

- `child`: nesnenin kalıcı kimliği;
- `parents`: geldiği nesneler, adımın sırasıyla; kendisi değil, her biri bir kez;
- `operation`: `split` ya da `merge`;
- `step`: geri alma adımının adı.

Kurallar:

- çizimde olmayan nesne de anılabilir (birleşen ve silinen ata);
- aynı kayıt bir kez tutulur;
- en çok 100 000 kayıt; eskisi önce düşer;
- kuralları `lineage_problem`'dir.

`.kcad`'e şema 101'le yazılır, yalnız kayıt varken. Bağımsız Python okuyucusu ve yazıcısı da okur ve yazar; örnek dosya
`lineage.kcad`, bozuk sekiz örnek de var.

Soy, katmanın ya da nesnenin değil projenin ayarıdır. Böylece nesnenin özü değişmez; tablosu, sunucusu ve bulut eşitlemesi
aynı kalır. Ayarlar zaten bütün olarak gider ve gelir.

### 2. Kaydı adımın kendisi yazar, geri alma da geri alır

`cad.entities.edit` (Kır, Birleştir ve Böl de bu yoldan gider) kaydı aynı geri alma adımında yazar. İşlem `Op::Lineage`'dir;
değer listelerinin `Op::Domains`'i gibi önce ve sonrayı tutar. Böylece:

- geri alma, kaydı parçalarla birlikte geri alır;
- yineleme, kaydı yeniden verir.

Kural, ADR 0419'un parçalarıdır: verisini taşıyan parça güncelleme, veriyi koruyan değiştirme ya da veriyi koruyan ekleme
olabilir.

- **Bölme** (`areaSplit`, `split`, `break`, `partsSplit`): bir nesnenin iki ya da daha çok parçası olunca, nesnenin kendisi
  dışındaki her parçanın atası o nesnedir.
- **Birleştirme** (`areaUnion`, `join`, `partsJoin`): nesneler silinir ve veriyi tek parça taşırsa, o parçanın ataları
  kendi nesnesi (başkaysa) ile silinenlerdir.

Python'da `Document.lineage`, `ancestors`, `descendants`.

### 3. Kaynağa yazmak soyu da yazar

Bağlı katmanın yazması (`cad.sources.sync`, tek katman ya da `with`) yazdığı her satırın soyunu tabloların aynı işleminde
yazar. Yazılan satır eklenen, değişen ya da silinen satırdır.

- **GeoPackage:** `kentos_lineage` tablosu (`table_name`, `child`, `parent`, `operation`, `step`). Uzantısı
  `gpkg_extensions`'ta, `kentos_uids` gibi.
- **PostGIS:** şemanın `kentos_lineage` tablosu; kimlikler `uuid`. Tablo listesinde görünmez.

Yazılan satırın soy satırları çizimin kayıtlarıyla değiştirilir.

- Geri alınmış bir bölmenin parçası silinirken soy satırı da gider.
- Sonradan silinen ama kaydı duran parçanın satırı kalır; o bir tarihtir.
- Soy kaydı olmayan ve tablosu bulunmayan dosyaya hiçbir şey eklenmez.

### 4. Kaynağa yazılmış adımın geri alınması denetimli bir yazmadır

Geri alma çizimindir; kaynağa ancak bir sonraki yazmayla gider. O yazma, öbür yazmalar gibi denetlenir:

- PostGIS'te değiştirdiği satırlar tabana karşı satır satır (ADR 0582);
- GeoPackage'ta dosyanın sürümüyle, gerekirse Kaynakla uzlaştır'la (ADR 0538, 0553).

O arada kaynakta aynı satırı başkası değiştirdiyse yazma `source_changed` ile reddedilir; hiçbir tablo yazılmaz.

Masaüstü bunu geri alma anında söyler. `Document::undo_report` geri alınacak adımın nesnelerini verir. Uygulama katmanının
`sources::undoing_written`'ı, adımın bir bağlı katmanda değiştirdiği her nesnenin katmanın tabanında tutulan hâlde olduğunu
(adım yazılmış) ve geri almanın birini farklılaştıracağını bulur. Tablo okunmaz. Bu durumda ileti, katmanın adıyla şöyledir:
“Bu adım … kaynağına yazılmıştı; kaynakta hâlâ duruyor. Kaynağa yaz geri almayı kaynağa da yazar …”.

## Doğrulama

- **Ortak durumlar:** `fixtures/commands/v1/cad.entities.{edit,break,join}.lineage.json`.
  - 115 durum: Böl, Kır, Birleştir, Parçalara ayır ve Parçaları birleştir; retler, geri alma ve yineleme.
  - Bağımsız başvuru `scripts/fixtures/lineage_cases.py`: komutların kendi bağımsız başvurularla yazılmış durumları, ADR'nin
    kuralıyla adım adım ayarların `lineage`'i. KentOS kodu yok.
- **Kodek:**
  - `kentos-kcad`'in `lineage` testi ve taşıma matrisi;
  - `kcad_v2_reference.py --check` (1133 dosya) ve `tools/kcad/kcad.py`.
- **GeoPackage:** `kentos-native-application`'ın `a_split_parcel_writes_its_parent_beside_its_rows`'u.
  - Akış: böl, yaz, geri al, yaz, yinele, yaz, birleştir, yaz.
  - Yazılan dosya `fixtures/geopackage/v1/kentos-soy.gpkg`. `lineage_cases.py --written` onu sqlite3'le okur.
  - GDAL'ın `validate_gpkg.py`'si yalnız QGIS'in kaynak dosyasında da olan boş geometri bayrağını söyler.
- **PostGIS:** `a_split_parcel_writes_its_parent_in_the_database`, geçici veritabanında (`postgis_cases.py`).
  - Akış: böl, yaz, geri al, yaz, yinele, yaz.
  - Başkası aynı parselin adını değiştirdikten sonra geri almanın yazması `source_changed` ile reddedilir; adı ve soy satırı
    yerinde kalır.
  - `postgis_cases.py` son hâli psycopg2'yle denetler: tek satır, çocuk ve ata tablonun satırları (kaydı olmayan satırın
    kimliği ADR 0505'in UUID v5'iyle).
- **Masaüstü:** `undoing_a_written_step_says_the_source_still_holds_it`. Yazılmış adımın geri alınması söylenir, sonraki yazma
  satırı geri getirir; yazılmamış adımın geri alınması söylenmez.
- **Python:** `test_lineage.py` (Birleştir, Kır, atalar, torunlar, geri alma).

## Sonraya kalan

- Soyun pafta, rapor ve Öznitelikler'de gösterilmesi.
- Bağlı tablonun soy satırlarının okunup çizimin kaydına alınması: okuma yalnız satırları ve kimlikleri alır.
- Sunucudaki veritabanı projesinde soy: ayarlarla gider; sunucunun kendi tablosu yok.
