# ADR 0590: PostGIS'te çoktan çoğa başvurunun bağlantı tablosu

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-15: formun ilişki standardını dış veri kümesinin işlem sınırına
  bağla (PostGIS dilimi).
- **Bağlam belgesi:**
  - ADR 0532: GeoPackage'ta ilişki profili, `cad.sources.relate`, `with` ve `commit`;
  - ADR 0409: çok değerli başvuru ve çoktan çoğa ilişki;
  - ADR 0582: PostGIS'te satır düzeyinde çakışma;
  - ADR 0584: aynı veritabanına tek işlem.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez. Bağlantı tablosunun kaydı veritabanının kendisindedir (yorumu ve yabancı anahtarları).
- **Sonraya kalan:**
  - formun arayüzde başka katmandan listeleri ve ilişkili kayıtları `cad.records.page` üzerinden okuması (ADR 0532'den);
  - bulutta sayfa sayfa okumanın ve toplamların kurum izinleriyle süzülmesi (CBS-38);
  - `cad.sources.describe`'ın veritabanı tablolarında ilişkileri listelemesi (bugün `cad.sources.relate check` söyler);
  - alanı sınırlı üst katman: alt kaydın başvurduğu üst kayıt çizimde yoksa yazma `relation_dangling` der (GeoPackage'taki
    kuralın aynısı); üst katmanı bütün bağlamak gerekir.

## Bağlam

ADR 0584'ten beri aynı PostGIS veritabanının katmanları tek işlemde yazılıyor: üst kayıt ve alt kayıtları (`cad.sources.sync`'in
`with`'i, `cad.forms.submit`'in `commit`'i) ya hep birlikte ya hiç. Ama çoktan çoğa başvurunun (parselin malikleri, ADR 0409)
veritabanında KentOS'un tanıdığı bir temsili yoktu: `cad.sources.relate` iki tablo aynı veritabanında olsa da `split` diyordu,
başvuru her zaman noktalı virgüllü metin sütunu olarak yazılıyordu. Veritabanında bunun doğal karşılığı bir bağlantı tablosudur
(iki yabancı anahtar). Plan bunu ister: “N:M çok değerli yerel başvuru temsili ile PG junction tablosu arasında açık eşleme kur;
bilinmeyen temsili noktalı virgüllü metne gizlice çevirme.”

## Karar

### 1. Temsil

KentOS'un bağlantı tablosu alt tablonun şemasındadır (varsayılan adı `alt_üst`, örneğin `iliski.parsel_malik`):

- `base_id bigint NOT NULL`: alt satırın anahtarı, alt tabloya yabancı anahtar;
- `related_id bigint NOT NULL`: üst satırın anahtarı, üst tabloya yabancı anahtar;
- `position integer NOT NULL`: başvurunun içindeki sıra (0, 1, …); okuma alanı bu sırayla kurar;
- birincil anahtar `(base_id, related_id)`, `related_id` üzerinde dizin;
- iki yabancı anahtar da `ON DELETE CASCADE`: başka bir istemci bir satırı silerse bağlantı satırları onunla gider, tablo
  kopuk satır tutmaz;
- tablonun yorumu `kentos_references base=şema.alt related=şema.üst`: tabloyu KentOS'un yapar ve yönünü söyler.

Durum, iki tablonun bütün tek sütunlu yabancı anahtarlarından ve KentOS yorumlu tablolardan bulunur:

- `mapped`: yorumu bu yönde, `base_id` alta, `related_id` üste giden anahtarlar ve `position` var;
- `broken`: yorumu bu yönde ama anahtarlardan biri ya da `position` yok (nedeniyle);
- `foreign`: yorumu ters yönde, ya da yorumsuz bir tablo iki tabloya da yabancı anahtarla bağlanıyor (başka bir aracın bağlantı
  tablosu); nedeni tablonun adıyla söylenir;
- `text`: hiçbiri.

KentOS'unki varsa o seçilir; yoksa bulunan ilki söylenir. Başka temsil üzerine yazılmaz, metne de çevrilmez (ADR 0532 §2 ile
aynı kural): `map` `relation_unsupported` der, yazma başvuruyu metin sütunu olarak yazar ve `relation_unmapped` uyarısında nedeni
söyler; yabancı tabloya dokunulmaz.

### 2. `cad.sources.relate` veritabanında

Alt ve üst katman aynı veritabanının tablolarına bağlıysa `split` değildir:

- `check` durumu, eşli ise tabloyu ve satır sayısını söyler;
- `map` tabloyu kurar, yorumunu ve dizinini yazar, satırlarını katmanların başvurularından (alt kaydın anahtar sırasıyla,
  alanın sırasıyla) tek işlemde yazar. Ad verilmezse `alt_üst`; şemasız ad alt tablonun şemasına gider; adın kuralı (harfle ya
  da `_` ile başlar, harf, rakam ve `_`, 63 bayt, `pg_` ile başlamaz, `kentos_uids` değil) `invalid_mapping`; o adda tablo
  varsa `invalid_mapping` (sağlayıcının `Exists`'i). Başvurulan kayıt tabloda yoksa `relation_dangling`. Planı tablo kurmaz;
- `unmap` yalnız KentOS yorumlu tabloyu kaldırır; başvurular katmanda kalır, sonraki yazmada metin sütunu olur.

Tabloların satırları değişmediği için bağların sürümleri olduğu gibi kalır.

### 3. Yazma ve okuma

- Eşli ilişkinin katmanlarından biri yazılırken (tek başına ya da `with` ile) bağlantı satırları alanın başvurularından kurulur
  ve **tabloların işleminde**, aynı kilit sırasında (tablonun adıyla) yazılır: çizimin tuttuğu alt kayıtların satırları silinip
  yenileri konur. Aynı yazmanın eklediği alt ya da üst kayıt, işlemin verdiği anahtarla yer alır (`Key::Inserted`). Parçanın
  alanının dışındaki alt kayıtların satırlarına dokunulmaz.
- Başvurulan üst kayıt çizimde yoksa (silinmiş ya da alanın dışında) `relation_dangling`: hiçbir tablo yazılmaz.
- Alan sütun olarak yazılmaz ve sütundan okunmaz: başvuru tek temsildedir (tabloda sütun varsa ne tutuyorsa onu tutar).
- Okuma (`direction read`, uzlaştırma, taban) alanı bağlantı tablosunun satırlarından `position` sırasıyla kurar; üst
  tablonun anahtarları değerleri okunmadan taranır (`scan`), kalıcı kimlikleri `kentos_uids`'ten.
- Sağlayıcının sözleşmesi: `relation`, `mapping_rows`, `map`, `unmap` ve tabloların düzenlemeleriyle bağlantı tablosunun
  satırlarını tek işlemde yazan `edit_many_mapped` (`MapWrite`: tablo, satırları silinecek alt kayıtlar, yeni satırlar).
  Bağlantı tablosu olmayan sağlayıcı bunları reddeder.

### 4. Masaüstü

İlişkiler penceresinin bölümü veritabanında **Veritabanında** başlığıyla, düğmeleri **Bağlantı tablosu kur** ve **Bağlantı
tablosunu kaldır**'dır; durum metni “Bağlantı tablosu olarak tutuluyor: “iliski.parsel_malik” tablosu, n satır.”

## Doğrulama

- `crates/native/postgis/tests/all.rs`'in `many_to_many_references_keep_a_junction_table`'ı,
  `scripts/fixtures/postgis_cases.py`'nin geçici PostGIS'inde (`iliski` ve `iliski2` şemaları; `iliskili.gpkg`'nin malik,
  parsel ve binaları kalıcı kimlikleriyle, `iliski2`'de başka birinin `sahiplik` bağlantı tablosu):
  - durum `text`; planın satır sayısı ve tablo kurulmadığı; `map`'ten sonra dört satır; ikinci `map` `relation_mapped`;
  - başvuru eklenince ve çıkarılınca yazmanın `relations`'ı ve satırlar; bir parselin maliklerinin sırası;
  - yeni parselin satırı işlemin verdiği anahtarla;
  - taze bir çizimin okuması alanları sırasıyla kurar;
  - başvurulan malik silinince malikin yazması `relation_dangling`, satırlar değişmez;
  - başkasının bağlantı tablosu: `foreign` ve nedeninde tablonun adı, `map` `relation_unsupported`, yazma `relation_unmapped`
    uyarısıyla metin sütununa;
  - `unmap` ve yeniden `map`.
- Betik veritabanının son hâlini psycopg2 ile ayrıca denetler: satırlar ve sıraları, yorum, iki yabancı anahtar ve
  `ON DELETE CASCADE` (bir malik bir işlemin içinde silinince satırları gider, işlem geri alınır), başkasının tablosu iki satırla
  yerinde, başvurusu metin sütununda.
- `crates/native/postgis/src/junctions.rs`'in birim testi: yorumun yazılışı ve okunuşu, başvurunun sıraları.
- ADR 0532'nin GeoPackage durumları (`fixtures/commands/v1/cad.sources.relate.json` ve ötekiler) değişmeden geçer.
