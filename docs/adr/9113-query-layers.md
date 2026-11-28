# ADR 9113: SQL sorgusu katmanı — sorgu kaynağında canlı okunur

- **Durum:** kabul edildi (2026-11-22). Plan: [plan/veri.md](../../plan/veri.md) VY-16 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalan: ADR 9101 §8.2 ve §10.
  SQL sorgusundan yapılan katman bağlı kopya olarak kalıyordu, yani sorgunun bütün satırları çizime kopyalanıyordu.
- **Önceki kararla ilişkisi:** ADR 0582 §3–§4'te PostGIS'e bağlan'da yazılan SQL bağlı kopya olarak bağlanıyordu. Bu
  karar o pencere için kalkar: sorgu artık veri katmanı olarak eklenir (§3). Komutların bağlı sorgusu değişmedi
  (`cad.sources.*`, `table` bir SQL).
- **Sözleşmeler:**
  - `.kcad` ve `cad.layers.data` değişmedi. Sorgu, veri katmanının `table` metnidir (§1).
  - Sürücü sözleşmesine `Source::query_columns` eklendi; varsayılanı `Unsupported`.
  - `cad.layers.rows`'un ortak durumlarına 5, `cad.layers.data`'nınkilere 2 sorgu durumu eklendi.
- **Bağımlılık:** yok.

## §1 Karar: sorgu, tablonun metnidir

**Sorun:** veri katmanı bir tabloyu adıyla okur (ADR 9101). Sorgunun adı yoktur; anahtarını ve geometrisini de bir
katalog söylemez.

**Masaüstü CBS:** QGIS'te sorgu, DB Manager'ın SQL penceresinde yazılır. “Load as new layer” işaretlenince iki şey
seçilir: benzersiz değerli sütun (“Column(s) with unique values”) ve geometri sütunu. Katmana bir ad verilir. Katman
sorguyu veritabanında alt sorgu olarak okur. Satırları kopyalanmaz ve düzenlenemez.

**Karar:** veri katmanının `table` metni bir sorgu olabilir:

```text
-- kentos: key=fid geometry=geom
SELECT fid, ada, geom FROM kadastro.parsel WHERE ada = 105
```

- İlk satır bir SQL yorumudur. Metin yine elle çalıştırılabilen sorgunun kendisidir.
  - `key`: satırları ayıran tam sayı sütun.
  - `geometry`: geometri sütunu. Yazılmazsa katman geometrisiz bir tablodur.
- Bu satırla başlamayan metin bir tablonun adıdır. Sözleşmede ayrı bir alan açılmadı. Böylece:
  - proje dosyası değişmedi;
  - komut değişmedi;
  - veri katmanının bütün yolları sorguyu tablo gibi taşır: tablo, seçim, Bilgi, dışa aktarma, sınıflar.
- Metin en çok 2048 bayt olabilir; veri katmanının bütün metinleri için sınır budur (`MAX_DATA_TEXT`).
  - Komutun bu ret mesajı “tablosu boş” diyordu. Artık “boş ya da 2048 karakterden uzun” der.
  - Pencere daha uzun bir sorguyu eklemez. Sorgunun veritabanında bir view yapılmasını önerir: view tablolarla birlikte
    listelenir ve canlı bağlanır.
- Katman ağacında, tabloda ve kartta kaynağın yeri metin olarak değil “SQL sorgusu” diye yazılır (`table_words`).
- Bir sorgu yalnız okur: tek bir `SELECT` ya da `WITH`. Noktalı virgülle ayrılmış ikinci bir deyim olamaz.
  - Yorumlar denetimden önce atılır.
  - Tırnak içindeki noktalı virgül deyim sonu sayılmaz (`query::check`).

## §2 Karar: sürücüler sorguyu alt sorgu olarak okur

Her iki sürücü sorguyu `(<sorgu>)` olarak okur. Kutu, koşul, sıra, sayfa ve sayım bu alt sorgunun üstünde kurulur.

**PostGIS:**

- **Sütunlar:** sorgudan geçici bir view (`kentos_sorgu`) yapılır ve sütunları `pg_attribute`'tan okunur. Bu, geri
  alınan bir işlemde olur. Sorgu çalışmaz. PostgreSQL yazan bir sorguyu view olarak kabul etmez.
- **Anahtar:** int2, int4 ya da int8 olmalı. Başka bir türse ya da yoksa ret, nedeniyle birlikte.
- **Geometri:** `geometry` ya da `geography`. Sistemi, türü ve boyutları ilk satırınkidir (`LIMIT 1`).
  - Sorgu ilk kez burada çalışır. İşlem bundan önce salt okunur yapılır. Bir işlem ilk deyiminden sonra da salt
    okunura geçebilir.
  - Böylece sorgunun çağırdığı bir işlev de yazamaz. Büyük nesne yaratan (`lo_create`) bir sorgu reddedilir (sınandı).
  - Salt okunur geçiş kaldırılınca aynı sınama kırmızıya döndü.
- **Okuma:** satırlar zaten `READ ONLY` bir işlemde, sunucudaki bir imleçle okunuyordu (`features`).
  - Sayım, farklı değerler ve gruplar düz bağlantıda koşuyordu. Sorgu katmanında artık bunlar da salt okunur bir
    işlemde koşar.
  - Bu önemli, çünkü geometrisiz bir sorgunun şekli okunurken sorgu hiç çalışmaz. İlk kez eklenirken sayılınca
    çalışır; düz bağlantıda çağırdığı işlevin yazması kalıcı olurdu.
  - Tablonun okuması değişmedi.
- Tahmini satır sayısı sorguda kullanılmaz, satırlar sayılır: sorgunun istatistiği yoktur.

**GeoPackage:**

- Okuma bağlantıları dosyayı salt okunur açar (`SQLITE_OPEN_READ_ONLY`). Sorgu dosyayı değiştiremez.
- **Tek deyim (28 Kasım 2026, [ADR 9401](9401-correctness-fixes-from-the-review.md) §3.4):** salt okunur bağlantı
  yine de geçici tablo, eklenen dosya ve `PRAGMA` alır, saklanan bağlantı bunları sonraki okumaya taşır. Sorgunun
  metni SQLite'a verilmeden önce onun kendi tırnak ve yorum kurallarıyla okunur; ilk deyimden sonra bir şey ya da bir
  değişken taşıyan metin reddedilir. Geçici view tek deyim alan çağrıyla kurulur.
- **Sütunlar:** geçici bir view bağlantının kendi geçici şemasına yapılır ve atılır; salt okunur bağlantı orayı
  yazabilir. Bildirilen türler `pragma_table_info`'dan okunur. Bir tablonun sütunu tablonun türünü taşır. Hesaplanan
  sütunun türü yoktur; türünü ilk değeri söyler.
- **Geometri:** sistemi GeoPackage geometrisinin başlığından okunur.
- Sorgunun mekânsal indeksi yoktur. Kutu her geometrinin kendi kutusuyla sorulur.

**Her ikisi:**

- Sorgu katmanı hiçbir zaman yazılabilir değildir (`capabilities`).
- İşlemin ekleme, değiştirme ve silme yolları sorguyu `refuse_write` ile reddeder: “Sorgu katmanı salt okunurdur:
  satırları sorgunun okuduğu tabloda değiştirilir.”
- `unique` ve `groups` anahtarı da alan olarak kabul eder. Anahtarın benzersizliği bunlarla denetlenir (§3).
- **Uygunluk takımı:** iki sürücüde de aynı sınamalar koşar (`conformance::query_checks`):
  - sütunlar ve türleri, önerilen anahtar, şekil;
  - yazılamazlık;
  - satırlar ve sayısı;
  - kutu ile koşul birlikte;
  - gruplar;
  - yazmanın reddi;
  - metin anahtarın ve `DELETE`'in reddi.

## §3 Karar: PostGIS'e bağlan'da sorgu katmanı

“Gelişmiş: sorgu katmanı (SQL)” açılınca SQL yazılır.

- **Sütunları oku:** sorgunun sütunları arayüzün dışında okunur (`Source::query_columns`).
  - Anahtar, tam sayı sütunlar arasından seçilir. Önce `fid`, `id`, `gid`, `ogc_fid` önerilir, yoksa ilk tam sayı
    sütun.
  - Geometri, geometri sütunları arasından seçilir. Son seçenek “Geometrisiz (tablo)”dur.
  - Katmanın adı yazılmazsa “Sorgu: <ilk tablo>” olur.
  - Tam sayı sütun yoksa pencere bunu söyler ve sorguya bir anahtar eklenmesini önerir.
- **Ekle:** anahtarın satırları ayırdığı kaynakta denetlenir.
  - Sorgunun satırları sayılır. 1 000 000 satıra kadar anahtarın farklı değerleri okunur. Bir değeri iki satır
    paylaşıyorsa ret, kaç satırda kaç değer olduğuyla.
  - Daha büyük sorgunun anahtarı olduğu gibi alınır. Okuma o zaman da doğrudur. Yanlış anahtar yalnızca seçimi,
    etiketi ve kartı yanlış satıra götürür.
- Katman salt okunur eklenir. Katman menüsünde “Düzenlemeyi başlat” kapalıdır.
- Tablolar ve sorgu aynı Ekle'yle birlikte eklenebilir (“2 kaynağı ekle”).
- Koordinat sistemi sorusu yalnız tablolar için sorulur. Sorgunun sistemi ilk geometrisininkidir ve ancak eklenirken
  bilinir. Katman, her veri katmanı gibi projenin sisteminde çizilir.

**Bulunup kapatılan hata:** SQL kutusuna yazılan metnin yalnız ilk harfi giriyordu.

- İlk tuşta pencerenin “Bağlandı: …” satırı siliniyordu. Altındaki bütün satırlar bir sıra yukarı kayıyor, kutu
  klavyeyi kaybediyordu. Gerçek bir kullanıcı da her sorguda kutuya ikinci kez tıklamak zorunda kalırdı.
- Satır artık bağlantınındır ve SQL değişince kalır. Sınama bunu kilitler.

## §4 Kayıt ve kişisel veri

Proje dosyasına sorgunun satırları yazılmaz, yalnız metni yazılır. Bu, bekleyen düzenlemelerin kararıyla aynıdır
(ADR 9111, 6698 sayılı KVKK md. 12).

Sorgu metni, kullanıcının yazdığı değerleri olduğu gibi taşır (örneğin `WHERE` içindeki bir ad). Kişisel bir değeri
koşul olarak yazan sorgu, o değeri proje dosyasında da taşır.

## §5 Planda olmayan kararlarım

1. **Sorgunun biçimi bir SQL yorumu.** Sözleşmeye `query` alanı eklemek proje dosyasını, komutu, Python'u ve TypeScript'i
   değiştirirdi. Yorum satırıyla metin hem okunur hem elle çalışır.
2. **Anahtar 1 000 000 satıra kadar denetlenir.** Bir milyon satırlık sorguda denetimin sunucudaki payı sayım 109 ms,
   farklı değerler 126 ms. Satırlar uygulamaya da taşınır; denetim arayüzün dışında koşar.
3. **Sorgu her yerde salt okunur çalışır** (§2): ilk satırın okunduğu işlem ve sorgunun sayımı, değerleri, grupları.
   Yazan bir sorgu eklenirken reddedilir. Yalnız satır okumasında reddedilseydi, eklendikten sonra hiç okunamayan bir
   katman kalırdı. Geometrisiz bir sorgunun yazması da sayılırken kalıcı olurdu.
4. **2048 bayttan uzun sorgu** pencerede söylenir, view önerilir. Komutun ret mesajı da düzeltildi (§1).
5. **Kapatılan eski kırmızı:** VY-05'te eklenen `data_save_failed` kodu yeniden denenebilir işaretliydi. Kod katalogunun
   kuralı “yalnız çakışma yeniden denenir” der. `kentos-contracts` sınaması 21 Kasım'dan beri bu yüzden kırmızıydı.
   - Tablonun reddettiği bir yazma, örneğin boş geçilemeyen bir alan, aynen yeniden denenince geçmez.
   - Kod artık yeniden denenebilir değil. Katalog yeniden üretildi.
6. **Python'un bütün durum koşusu** `python/tests` klasöründen çalıştırılır. Dosya komutlarının bağımsız koşucuları
   oradadır; depo kökünden çalıştırılınca bulunamıyordu.

## §6 Kanıt

**Sınamalar:**

- `kentos-data`:
  - sorgu metninin biçimi gidip gelir;
  - yalnız tek bir okuyan deyim kabul edilir (yorum, tırnaktaki noktalı virgül, `DELETE`, yorumlu `UPDATE`).
- Uygunluk takımı GeoPackage'ta ve yalıtılmış PostgreSQL 18.4 / PostGIS 3.6.3'te geçti (§2).
- PostGIS: `a_query_layer_is_run_read_only`. Büyük nesne yaratan bir sorgu dört yerde reddedilir:
  - geometrisiyle şekli okunurken;
  - geometrisizken sayılırken, değerleri okunurken ve grupları okunurken.

  Sınamadan sonra veritabanında hiçbir büyük nesne yoktu (`pg_largeobject_metadata`: 0).
- `cad.layers.rows`, 37 durum, 5'i yeni (`satirlar.gpkg`, beklentiler bağımsız betikte):
  - sorgunun satırları ve sayısı: 30 değil 7;
  - arama ve Türk alfabesiyle sıra;
  - kutu ve WKT;
  - kaynakta süzgeç;
  - yazan deyimin reddi.

  Rust ve Python koşucusu geçti. Bozuk bir beklentiyi iki koşucu da yakaladı: tablonun 30 satırı ve kutuda 9.
- `cad.layers.data`, 35 durum, 2'si yeni: sorgu, metni olduğu gibi salt okunur katman olur; 2048 bayttan uzun
  sorgu reddedilir.
- Bütün ortak durumlar Rust'ta geçti.
- Python'da tek kırmızı `cad.arch.ifc` (`compartments: 1 != 0`). BIM-13 işinden geliyor, veri katmanıyla ilgisi yok.
- Masaüstü:
  - GeoPackage'ta sorgu katmanı salt okunur eklenir, adı verilir;
  - tablonun 40 satırı yerine sorgunun 20 parkını okur;
  - iki satırın paylaştığı anahtar reddedilir;
  - düzenleme reddedilir.
- PostGIS penceresi:
  - sütunlar okununca `fid` önerilir;
  - anahtar ve geometri değiştirilir;
  - sistem sorusu sorulmaz;
  - “Bağlandı” satırı SQL yazılırken kalır;
  - başka bir sorgu eski sütunları unutturur;
  - 2048 bayttan uzun sorgu eklenmez ve söylenir.

**Gerçek pencere:** yalıtılmış PostGIS'teki bir milyon parsel, gerçek fare ve klavye (Xvfb). Görüntüler
`.run/gui-evidence/veri/vy16/sonra`.

1. PostGIS'e bağlan › Gelişmiş'te `SELECT fid, ada, parsel, kullanim, geom FROM public.perf_parseller WHERE kullanim =
   'Park' AND ada < 120` yazıldı.
2. Sütunları oku sorguyu çalıştırmadan sütunlarını getirdi: anahtar `fid`, geometri `geom`. Ad “Parklar (sorgu)”
   yazıldı (`01-sutunlar-okundu`).
3. Ekle'den sonra katman ağacın en üstüne geldi, görünüm kapsamına yaklaştı. Parseller gizlenince haritada yalnız
   sorgunun parkları kaldı: ada 100–119, her dördüncü parsel (`02-harita`, `03-yakin`).
4. Tablo: “5.000 satır · süzgeç ve sıra kaynakta”. Anahtar sütunu `fid`, her satırın kullanımı Park (`04-tablo`).
5. Katman menüsünde “Düzenlemeyi başlat” kapalıydı (`05-menu`).

Veritabanında hiçbir şey değişmedi.

## §7 Kapsam dışı

- **GeoPackage penceresinde sorgu yazma yeri yok.** Önceden de yoktu: ADR 0582'nin sorgu katmanı yalnız PostGIS
  penceresindeydi. Sürücü (§2), komut (§1) ve masaüstünün ekleme yolu GeoPackage sorgusunu okuyor (sınandı).
  Pencereye yer açmak ayrı bir iş.
- **Komutların bağlı sorgusu** (`cad.sources.*`, ADR 0582 §3) bağlı kopya olarak kaldı. Bir betik canlı sorgu
  katmanını `cad.layers.data` ile, `table`'a sorgu metnini vererek ekler.
- **Yazılabilir sorgu yok:** bir sorgunun satırları, okuduğu tablonun veri katmanında düzenlenir.

## Uygulama

- **Sürücü sözleşmesi:** `crates/native/data/src/query.rs` (yeni): `QueryTable`, `check`, `likely_key`,
  `refuse_write`. `driver.rs` (`query_columns`), `conformance.rs` (`query_checks`).
- **PostGIS:** `crates/native/data-postgis/src/query.rs` (yeni). Ayrıca:
  - `catalog.rs`: `Shape.query`;
  - `read.rs`: tahmin yok; sayım, değerler ve gruplar salt okunur işlemde; `query_columns`; anahtar alan olarak;
  - `write.rs`: ret;
  - `tests/conformance.rs`: salt okunur sınaması.
- **GeoPackage:** `crates/native/data-gpkg/src/query.rs` (yeni). Ayrıca `source.rs` (sorgunun şekli, `FROM`,
  yetenekler, `query_columns`) ve `write.rs` (ret).
- **Sözleşme:** `crates/shared/contracts/src/gis/data_layer.rs`: `table_words`, uzun metnin ret mesajı.
  `core/codes.rs`: `data_save_failed` yeniden denenmez. Katalog yeniden üretildi.
- **Ortak durumlar:** `scripts/fixtures/layer_rows_command_cases.py`, `layer_data_command_cases.py`.
- **Masaüstü:**
  - `apps/desktop/src/shell/exchange/postgis_query.rs` (yeni);
  - `postgis.rs`: alanlar, olaylar, birlikte Ekle;
  - `gis/data_layers.rs`: `describe_query`, `add_query`, `KEY_CHECKED`;
  - sınamalar `gis/query_layer_tests.rs` (yeni) ve `exchange/postgis_tests.rs`.
- **Gerçek pencere betiği:** `scripts/usage/real/session.sh`, oturum klasöründen veritabanı bağlantılarını da kopyalar
  (`KENTOS_REAL_SETTINGS`); sahibin kendi listesi hiç kopyalanmaz.
