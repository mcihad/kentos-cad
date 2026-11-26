# ADR 9324: Alanın listesi ve aralığı kaynağa kural olarak yazılır ve geri okunur

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  4, P2; ikinci dilim). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 9318](9318-data-layer-columns.md) (`cad.layers.columns`, sürücü sözleşmesine sütun ekleme);
  - [ADR 9323](9323-data-table-typed-cells.md) (maddenin ilk dilimi);
  - [ADR 1075](1075-geopackage-field-rules-round-trip.md) (GeoPackage şema uzantısının dışa aktarmada yazılıp okunması);
  - [ADR 0418](0418-retired-values.md) (kullanımdan kalkan kodlar).

## Bağlam

- **Kusur:** yeni GeoPackage'ta bir alana liste bağlanınca `gpkg_data_columns` ya da `gpkg_data_column_constraints`
  yazılmıyordu. Kaynak tablonun DDL'inde de `durum` için denetim (CHECK) yoktu. Listenin yalnız projede mi yoksa
  kaynakta da mı olduğu hiçbir yerde görünmüyordu.
- **İncelemenin kabulü:**
  - Alan özelliklerinde “Proje kuralı” ile “Kaynak şemasında kısıt” ayrı gösterilsin.
  - GeoPackage'ta şema uzantısıyla gidiş-dönüş olsun; PostgreSQL'de denetim kuralı yazılsın.
  - Katman verisiyle uyumsuz eski kodlar listelensin.
  - Kullanım dışı kalan kod eski kayıtta okunsun, yeni seçimde sunulmasın.
- **Kod:** şema uzantısı GeoPackage dışa aktarmasında yazılıp içe almada okunuyordu (ADR 1075). Veri katmanlarının sürücü
  sözleşmesinde kural yoktu.

## Araştırma

- **OGC GeoPackage şema uzantısı:**
  - `gpkg_data_columns` bir sütunu kurala bağlar.
  - `gpkg_data_column_constraints` kuralı değer listesi (`enum`, değer ve açıklama) ya da aralık (`range`, uçlarıyla)
    olarak tutar.
  - Uzantı bilgi tutar; SQLite yazmayı bu kurala göre denetlemez
    ([GeoPackage 1.4, şema uzantısı](https://www.geopackage.org/spec140/index.html#extension_schema)).
- **PostgreSQL:**
  - `CHECK` kısıtı her eklemede ve güncellemede satırın yeni hâlini denetler.
  - `NOT VALID` ile eklenen kısıt var olan satırları taramaz, sonraki yazmaları denetler.
  - Kısıta `COMMENT ON CONSTRAINT` ile açıklama eklenebilir
    ([ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html),
    [COMMENT](https://www.postgresql.org/docs/current/sql-comment.html)).
- **ArcGIS Pro:** alan kısıtları (domain) veri tabanının içinde tutulur ve alanlara bağlanır
  ([alan domain'leri](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/an-overview-of-attribute-domains.html)).
- **KentOS kararı:** liste projede kalır (ADR 0430). Kaynağa yazmak ayrı ve açık bir adımdır; kaynağın kuralı katmana geri
  okunur.

## Karar

### 1. Sürücü sözleşmesi (`kentos-data`)

- **Yeni türler:** `ColumnRule` (değerler ve etiketleri ya da uçlarıyla aralık) ve `RuledColumn` (sütun ve kuralı; kural
  yoksa sütunun kuralı kaldırılır).
- `Source::rules(table)`: kaynağın tuttuğu kurallar; kural tutmayan kaynak boş döner.
- `Transaction::set_rules(table, columns)`: her sütunun kuralını değiştirir. Var olan satırlar denetlenmez. Yapamayan
  kaynak bunu söyler; işlemin hiçbir şeyi yazılmaz.
- `Capabilities.set_rules`: yazılabilir GeoPackage; PostgreSQL'de tablonun sahibi (`add_columns` gibi). Sorguda yoktur.
- **GeoPackage:** şema uzantısı. Kural adı `<tablo>_<sütun>`'dur. Yeniden yazılan kuralın eski satırları önce silinir;
  liste şimdiki kodlarını tutar.
- **PostgreSQL:** `kentos_<sütun>` adlı `CHECK … NOT VALID` kısıtı yazılır (boş değer her kurala uyar).
  - Kısıtın açıklaması kuralı ve etiketlerini KentOS'un kısa metniyle tutar (`kentos-kural/1`); geri okuma oradan yapılır.
  - Kullanımdan kalkan kodlar da listededir: onları tutan eski satırlar geçerli kalır.
- **Ortak uygunluk paketi (`conformance::rules_checks`):** geri alınan yazma kural bırakmaz; liste ve aralık yazıldığı
  gibi okunur; yeniden yazılan liste şimdiki kodlarını tutar; kaldırılan kural gider. İki sürücü de geçer.

### 2. `cad.layers.columns` `rules` (Kuralları kaynağa yaz)

- **Yazılan:** katmanın alanlarının değer listeleri (kullanımdan kalkan kodlar da, etiketleriyle) ve sayı aralıkları,
  kendi işleminde tabloya kural olarak yazılır.
  - Listesi ya da aralığı olmayan alanın, birden çok değerli alanın, anahtarın ve geometrinin sütunu olduğu gibi kalır.
  - Hiçbir alanda kural yoksa `no_field_rules`.
- **Katman değişmez;** geri alma adımı yoktur. Yazılan kural, eklenen sütun gibi tabloda kalır.
- **Cevap:** `ruled` yazılan sütunlardır. `outside` satırların kurala uymayan değerlerini alan alan, satır sayısıyla
  söyler (ör. `mahalle` M9, 1 satır); o satırlar değişmez.
- **Retler:** `data_layer_read_only`; `columns_not_permitted` (sahip olmayan hesap, salt okunur dosya, sorgu).
- **Plan:** dışarıda kalanları sayar, yazmaz.

### 3. Geri okuma

- `refresh` (Kaynak şemasını yenile) ve tablonun veri katmanı olarak eklenmesi (`data_describe`) kaynağın kuralını,
  kendi kuralı olmayan alana liste ya da aralık olarak verir. Projenin kuralı kaynağınkinin üstündedir.

### 4. Masaüstü: Alanlar

- Veri katmanında Kaynak şemasını yenile'nin yanında “Kuralları kaynağa yaz” düğmesi vardır.
- Seçili alanın satırı kuralın nerede olduğunu söyler:
  - “Kural: yalnız proje. KentOS yazarken denetler; tabloda kural yok, başka programlar denetlenmeden yazar.”
  - Ya da “Kural: proje ve kaynak”: GeoPackage'ta “kuralı şema uzantısında tutar, başka programlar okur”,
    PostgreSQL'de “veritabanı da başka programların yazdığını denetler”.
- Yazınca sonuç tablonun altında söylenir. Kurala uymayan eski değerler satır sayılarıyla söylenir; PostgreSQL'de o
  satırların değer düzeltilmeden değiştirilemeyeceği eklenir.
- Sözler sürücüye göre doğrudur: SQLite şema uzantısını denetlemez, PostgreSQL denetler.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Kuralları kaynağa yaz, kaynağın kuralını okuma, proje/kaynak ayrımı | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.layers.columns(action="rules")`; `test_layer_columns.py` yazmayı, dışarıda kalan değeri ve başka katmanın geri okumasını sınar. |
| MCP | var | Aynı komut (MCP'nin yıkıcı komutlar listesinde). |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Açık

- Başka bir tabloya başvuru (referans tablo ve yabancı anahtar) olarak liste yazılmaz; liste kısıtı yazılır.
- Bağımlı listeler (alt tür, bağımlı değer) kaynağa tek bir liste olarak yazılır; bağımlılık projede kalır.
- Dış bir istemcinin PostgreSQL'de kurala aykırı yazmasının reddi uygunluk paketinde sınanır; GeoPackage'ta dış yazma
  denetlenmez.
- Gerçek pencerede fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Sürücüler:**
  - `cargo test -p kentos-data-gpkg conformance`: geçti.
  - Geçici PostGIS'te (55432) `cargo test -p kentos-data-postgis --test conformance`: 7 sınama geçti. Sahip olmayan
    hesabın kural yazması `Denied`, kaynakta kural kalmıyor.
  - `kentos-data-postgis` `rules::tests`: kural metni gidiş-dönüş, tırnak kaçışı, 63 baytlık ad.
- **Ortak durumlar:** `scripts/fixtures/layer_columns_command_cases.py` 36 durum yazıyor (6'sı yeni):
  - yazma ve dışarıda kalanlar (`durum` B, `no` 3), ikinci katmanın yenilemede listeyi ve aralığı geri okuması;
  - planın yazmaması; yeniden yazılan listenin yalnız şimdiki kodu;
  - retler: `no_field_rules`, salt okunur, sorgu.
  - Rust (`KENTOS_CASES_ONLY=cad.layers.columns`) ve Python (`test_command_cases.py`) koşar.
- **Python:** `test_layer_columns.py` (3 sınama), yerel modül yeniden derlendikten sonra.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- source_columns_tests layer_fields`. Yeni sınama: liste önce yalnız
  projede; Kuralları kaynağa yaz M9'u 1 satırla söylüyor; pencere yeniden açılınca kural GeoPackage'tan okunuyor.
- **Resim** (`.run/gui-evidence/kullanim-9324/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-yalniz-proje-kurali`: “Kural: yalnız proje …”;
  - `02-kurallar-kaynakta`: “Kural: proje ve kaynak …” ve yazmanın sonucu, eski kod M9.
