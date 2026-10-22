# ADR 0584: Aynı kaynağa birlikte yazma ve yazmada Z ile M

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-11: aynı işlem kaynağındaki katmanların değişiklikleri birlikte
  yazılır; bağımsız veritabanlarına atomiklik vaat edilmez; Z, M ve eğri aktarımında desteklenmeyen durum yazmadan önce
  reddedilir.
- **Bağlam belgesi:**
  - ADR 0503: bağlı katman ve kaynağa yazma;
  - ADR 0532: GeoPackage'ta birlikte yazma (`with`) ve ilişki eşleme tabloları;
  - ADR 0582: PostGIS'te satır düzeyinde çakışma;
  - ADR 0224: köşede ölçü (M).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Sonraya kalan:**
  - PostGIS'te çoktan çoğa başvurunun eşleme tablosu: bugün yalnız GeoPackage'ta (ADR 0532 §2);
  - parsel bölmede eski ile yeni kimliğin soy bağı (kaynakta ayrı bir tablo);
  - sunucuda onaylanmış değişikliğin geri alınması: yeni, sürüm denetimli bir yazma olarak.

## Bağlam

ADR 0532'den beri aynı GeoPackage dosyasının katmanları `cad.sources.sync`'in `with`'iyle tek SQLite işleminde yazılıyordu.
PostGIS'te ise bir yazma bir tablo demekti. `with` verilince `transaction_split` ile reddediliyordu. Bir parselin kaydı ile
malik satırları ya da bir yolun eksen ve kaplama katmanları aynı veritabanında olduğu hâlde ayrı ayrı yazılıyordu. Biri
yazılıp öbürü reddedilirse veritabanı yarım kalıyordu.

İkinci eksik Z ve M'ydi. GeoPackage'ın geometri sütunu Z ve M'yi yasaklar (0), ister (1) ya da isteğe bırakır (2). PostGIS'in
tipli sütunu (`geometry(PolygonZ, 5256)`) da aynısını söyler. KentOS bunlara bakmadan yazıyordu:

- 2B sütuna kotlu nesne Z'siyle gidiyordu. GeoPackage'ta dosya standarda aykırı kalıyordu, PostGIS'te veritabanının hatasıyla
  yarıda dönüyordu.
- Z'li sütuna kotsuz nesnede de durum aynıydı.
- Kısmen kotlu nesnede (bazı köşelerin kotu boş) WKB'nin tek boyutu ilk köşeden seçiliyordu. Kotsuz köşelere NaN yazılıyor ya
  da öbür köşelerin kotları sessizce düşüyordu.

## Karar

### 1. Aynı veritabanına tek işlem

`cad.sources.sync`'in `with`'i PostGIS katmanlarını da alır. Bütün katmanlar aynı adresteki veritabanına bağlıysa hepsi tek
işlemde yazılır (`sources/database.rs`):

1. Her katman okunur ve yazmadan önce denetlenir:
   - salt okunur tablo;
   - tabanı olmayan katmanda alanın değişmesi;
   - sütunun tutamadığı geometri ve §2'nin Z ile M'si;
   - alanın dışındaki komşular (ADR 0531).
2. Tek işlem açılır. Tablolar adlarının sırasıyla kilitlenir (`SHARE ROW EXCLUSIVE`), böylece aynı tabloları yazan iki
   istemci birbirini beklemez.
3. Her tablonun dokunulan satırları katmanın tabanına karşı yeniden denetlenir (ADR 0582 §2).
4. Değişiklikler yazılır. Bir tablo reddederse hiçbiri yazılmaz.

Ret, reddeden katmanı gösterir: ilki `layer`, öbürleri `with`. Satır çakışmasında başkasının değiştirdiği nesnelerin kalıcı
kimlikleri `uids`'tedir. Cevabın `layers`'ı katman katman eklenen, değişen ve silinen sayısını verir.

Sağlayıcı arayüzüne `TableProvider::edit_many` eklendi. Varsayılanı tek tabloyu `edit` ile yazar, birden çoğunu reddeder.
PostGIS sağlayıcısı onu tek işlemle uygular; tek tablonun `edit`'i de aynı işlevden (`edit_in`) geçer.

Farklı kaynaklar birlikte yazılmaz: başka adresteki veritabanı, bir GeoPackage ile bir PostGIS katmanı ya da iki ayrı dosya
`transaction_split`'tir. İki kaynağa birden tek işlem kurulamaz ve öyleymiş gibi gösterilmez.

Masaüstünde Kaynak ▸ Kaynağa birlikte yaz, katmanla aynı kaynakta olan iki tür katmanı toplar:
- ilişkiyle bağlı olanlar;
- yazılmamış değişikliği olan bağlı katmanlar.

Bildirimi veritabanını adresiyle, dosyayı adıyla söyler.

### 2. Yazmada Z ve M

Bir nesne tablonun geometri sütununa ancak Z ve M'si sütunun bayraklarına uyuyorsa yazılır (`geopackage::dims_problem`).
Sıra şudur:

1. Kotu olan nesne, Z'yi yasaklayan sütuna (z = 0) yazılmaz.
2. Bazı köşeleri kotlu, bazıları kotsuz nesne hiçbir sütuna yazılmaz. Kotsuz köşeye değer uydurulmaz, kotlu köşenin değeri de
   düşürülmez.
3. Kotu olmayan nesne, Z'yi isteyen sütuna (z = 1) yazılmaz.
4. Aynı üç kural M için de geçer.

Uymayan nesne yazmayı reddeder (`geometry_mismatch`, nesnenin kalıcı kimliği ve nedeniyle), hiçbir şey yazılmaz. Nesne
katmanın bekleyen değişikliği olarak kalır: `cad.sources.describe` onu sayar ve sıradaki işi "yaz" der.

PostGIS'in tipsiz `geometry` sütunu her boyutu alır: Z ve M isteğe bağlı (2) sayılır. Tipli sütunun Z ve M'si tipinden okunur
(`PolygonZ`, `LineStringM`, `…ZM`).

## Doğrulama

- `scripts/fixtures/source_dims_cases.py`: 22 ortak durum (`fixtures/commands/v1/cad.sources.sync.dims.json`), KentOS kodu
  olmadan.
  - Dosya `fixtures/geopackage/v1/boyut.gpkg`'dir. GDAL'ın OGR'ı yazar: 2B alan, Z'li alan, M'li çizgi, 2B çizgi; isteğe
    bağlı iki sütunun bayrakları sqlite3'le 2'ye çekilir.
  - Sütunların bayrakları sqlite3'le okunur, karar ADR'nin kuralıyla verilir.
  - Durumlar: kotlu, kısmen kotlu, kotu silinmiş ve ölçülü nesneler; düzeltilince geçen kısmi kot; yalnız özniteliği değişen
    satırlar boyutlarıyla.
- `crates/native/postgis/tests/all.rs`'in `layers_of_one_database_go_in_together`'ı, `scripts/fixtures/postgis_cases.py`'nin
  geçici PostGIS'inde (`boyut` şeması):
  - iki tablo tek işlemde yazılır;
  - başka bir çizim tablolardan birinin satırını değiştirdikten sonra ikisinin yazılması `source_changed` ile reddedilir,
    öbür tablo da yazılmaz;
  - GeoPackage ile PostGIS birlikte `transaction_split`'tir;
  - Z'li sütuna kotları silinmiş alan `geometry_mismatch`'tir.
  - Betik veritabanının son hâlini psycopg2'yle ayrıca denetler.
- Var olan GeoPackage birlikte yazma ve satır düzeyi PostGIS durumları değişmeden geçer.
