# ADR 0634: GIS türevinde geri alma, bağ koparma, başka kaplar ve mimari türev

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-16'nın kalanı: GIS'te türevi düzenlemenin kaynağa geçerli ters işlemi
  ya da açık ilişki koparma; PostGIS, Shapefile ve GeoJSON türevi; mimari nesne ve ölçü türevi. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0530 (CAD kaynağının GIS türevi), 0502 (GeoPackage), 0514 (Shapefile dışa aktarma), 0046 (GeoJSON),
  0592 (sonucu PostGIS'e yazma), 0505 (PostGIS sağlayıcısı), 0339–0343 (mimari nesneler), 0147 ve 0321 (ölçü), 0253 (komut
  protokolü), CLAUDE.md §15 (kaynak otoritesi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** yaylı duvarın türevi (yüzlerinin dış çizgisi kirişlerle kesin çıkmıyor: bugün sayılıp nedeniyle söylenir),
  etiket ve görünüm işaretinin türevi (başka nesneyi gösteren açıklama), mimari nesnenin ve ölçünün GIS geometrisinden
  değerlerine geri çevrilmesi (bugün reddedilir), Shapefile'da M'si olmayan Z kümesinde kotsuz köşe (biçim 0 yazar; aşağıda §3).

## Bağlam

ADR 0530 türevi GeoPackage'a yazdı, denetledi ve kaynağın yerine geçmesini reddetti: GIS'te değişmiş satır “değişikliği
çizime geri yazılmaz” denip bırakılıyordu. Plan bundan fazlasını ister: “GIS'te türevi düzenleme; kaynağa geçerli ters işlem,
açık ilişki koparma veya ret politikası taşısın”; türev PostGIS'e, Shapefile'a ve GeoJSON'a da yazılsın; mimari nesne ve ölçü
kendiliğinden aynı anlamdaki GIS alanı sayılmadan kendi tablolarında türesin.

## Karar

### 1. `cad.gis.takeback`: GIS'teki değişiklik kaynağa

Yalnız `cad.gis.derivation`'ın **edited** dediği satır alınır: kaynağı dışa aktarmadaki gibidir (satırın `kentos_digest`'i
kaynağın bugünkü özetine eşit) ve satır türevin verdiğinden farklıdır. Satır başına sıra:

1. dosyada yok → `derivation_row_not_found` (tablo da, kimlik de);
2. kaynağı çizimde yok → `row_source_gone`;
3. kaynağı da değişmiş → `row_stale`: iki yan da değişti, GIS'teki değişiklik çizimdekinin üstüne yazılmaz;
4. GIS'te değişmemiş → `row_not_edited`;
5. kaynağı kilitli katmanda → `layer_locked`;
6. geometrisi istenip kaynağın türüne uymuyor → `geometry_kind_mismatch`; mimari nesne ya da ölçü → `geometry_parametric`;
7. değer katmanın alanına uymuyor → `invalid_attribute`, `attribute_required`.

Bir satır reddedilirse hiçbiri yazılmaz. Her şey tek geri alma adımıdır: **“GIS'ten geri al”**.

- **`take: attributes`** (varsayılan): farklı sütunlar kaynağın özniteliklerine, sütunun türüyle okunup katmanın alanının
  kuralıyla; çakışmadan yeniden adlandırılmış sütun kaynağın adıyla (`renamed`). Yazının metni, yüksekliği ve dönüşü; blok
  yerleştirmesinin ölçeği ve dönüşü (sütun derece, kaynak radyan). GIS'te boşaltılan değer özniteliği siler. Geometrisi de
  değişmişse geometri kalır ve söylenir (`geometry_left`).
- **`take: geometry`**: sütunlar ve geometri. Düz geometrili kaynak (nokta, çizgi, bükümsüz çoklu çizgi, alan) GIS'tekini olduğu
  gibi alır, Z ve M'siyle. Kaynağı bir eğri olan nesne (daire, yay, bükümlü kenar, elips, eğri, NURBS) GIS'teki kirişleri alır:
  analitik tanımı bırakılır, **bağ kopar** (`link_broken`, satırın `broken`'ı eski tür: `circle` bir `polygon` olur). Yazı ve
  blok yerleştirmesi noktayı alır.
- Yalnız geometrisi değişmiş satırın öznitelikleri istenirse hiçbir şey yazılmaz (`changed` 0) ve söylenir.
- Mimari nesne ve ölçü (§4) değerlerinden çizilir: geometrisi alınmaz (`geometry_parametric`), hesaplanmış sütunları (duvarın
  genişliği, mahalin adı, ölçülen değer) yazılmaz ve söylenir (`fields_left`); kendi öznitelikleri alınır.

Ardından satırlar eskir (kaynakları değişti): türev yeniden dışa aktarılır. Doğrulama ve plan yazmaz.

### 2. `cad.gis.detach`: açık bağ koparma

İkinci çıkış: değişiklik çizime yazılmaz, tablo türev olmaktan çıkar. Seçilen tabloların (verilmezse bütün türev tabloların)
türetim kaydı, ne olduğunu, ne zaman ve neden koparıldığını söyleyen bir **notla** değişir (`https://kentos.app/derivation-detached/1`,
`kentos.derivation-detached` 1: zaman, isteğe bağlı neden, eski kayıt olduğu gibi) ve satırlar çizimin kalıcı kimliklerini
bırakır; hepsi tek işlemde (GeoPackage'ta tek SQLite işlemi, veritabanında tek işlem, dosyada bütün yazılır ya da hiç).
Ardından tablo her tablo gibi bağlanır, okunur ve içe aktarılır, satırları yeni nesnelerdir; `cad.gis.derivation` onu saymaz.
Çizim değişmez ve geri alma adımı değildir: kopan bağ geri gelmez, türev yeniden dışa aktarılır. Retler: `invalid_file_path`,
`file_not_found`, `import_unreadable`, `not_a_derivation`, `invalid_address`, `source_unreachable`, `derivation_row_not_found`
(türev olmayan tablo, aynı tablo yeniden), `invalid_count` (tablo iki kez), `file_unwritable`.

### 3. Kaplar: GeoPackage, Shapefile, GeoJSON, PostGIS

`file.export.gis`'in `format`'ı. Türetme bir kez, bellekte yapılır (`gis_derive::build`: tablolar, satırlar, kimlikler,
kayıtlar); her kap aynı tabloları yazar. Denetim, geri alma ve bağ koparma hepsini aynı biçime okur (`gis_set::open_any`), bir
kural hepsini sınar.

| Kap | Yol | Kayıt ve kimlik |
|---|---|---|
| GeoPackage (varsayılan) | `.gpkg` | ADR 0530: metadata uzantısı, `kentos_uids` |
| Shapefile | `.zip` | her tablo geometri türü başına bir set (`Parsel`; `Çizim_nokta`, `Çizim_cizgi`, `Çizim_alan`; kayıt ilk setin boş şekilli satırı), `file.export.shp`'nin yazıcısıyla; arşivin `kentos-turev.json`'u (`kentos.derivation-set` 1): setin kaydı, sütunların tablodaki ve .dbf'teki adları (on bayta kısaltılır) ve türleri, geometri türü; kimlik `kentos_uid` sütunu |
| GeoJSON | tek tabloda `.geojson`, çok tabloda `.zip` | FeatureCollection'ın `kentos:table` yabancı üyesi (kayıt, sütunlar, geometri türü); koordinatlar projenin sisteminde (`crs` üyesi adlandırır); zip'te aynı dosyalar ve dizin |
| PostGIS | veritabanının adresi, `schema` (yoksa açılır; varsayılan `public`) | ADR 0592'nin yeni tablo yolu, tek işlem: kayıt tablonun yorumu (`table` alanı `şema.tablo`), kimlikler şemanın `kentos_uids`'i; aynı adlı tablo `table_exists`, `overwrite` yeniden açar |

- Yolun uzantısı biçimin olmalıdır (`invalid_file_path`); GeoJSON'da çok tablo `.zip` ister.
- **Karşılaştırmada kabın kuralı:** halkalar yön ve başlangıç noktasından bağımsız karşılaştırılır (Shapefile dış halkayı saat
  yönünde yazar; bunu yapan GIS değişiklik yapmış sayılmaz). Kabın taşımadığı boyut beklenenden düşülür: GeoJSON'da M yoktur.
  Shapefile'ın Z türünde kotsuz köşe 0'dır (biçimin kuralı, ADR 0514): beklenen geometri de öyle kurulur (`Z_AS_ZERO`); bu yüzden
  Shapefile'dan geometrisi alınan, kotsuz köşeli bir satırın köşeleri 0 kotla gelir.
- Shapefile okuyucusu boş şekilli satırı (geometrisiz kayıt) atlar; satırlar .dbf'ten sırasıyla, şekiller kalıcı kimlikle eşlenir.
- Veritabanının türevinde dosyanın boyu 0, SHA-256'sı okunan tabloların revizyonlarınındır.

### 4. Mimari nesne ve ölçü

Kendiliğinden aynı anlamdaki GIS alanı sayılmazlar: kendi tablolarındadır, `kentos_layer` sütunuyla.

- **`arch`** rolü, tür başına tablo: `mimari_duvar` (düz duvarın yüzlerinin bandı, `geom::wall::body`), `mimari_mahal` (alanı),
  `mimari_aks` (yolu), `mimari_kolon` (dış çizgisi), `mimari_merdiven` ve `mimari_rampa` (bantları), `mimari_aciklik` (pervazlarının
  izi). Geometri türetmenin her şekle uyguladığı kuralla (daire kolon kirişlerle, sapmasıyla). Nesnenin tekil değerleri (sayı,
  metin) sütundur: genişlik, hiza, tür, numara, ad, rıht sayısı, eğim, duvarın kimliği… Yaylı duvar, pervazı olmayan açıklık ve
  çizilemeyen nesne nedeniyle dışarıda kalır. Kaydın `kind`'i nesnelerin türüdür.
- **`dimensions`** rolü, `kentos_dimensions`: ölçülen noktalar çizgi (açıda köşeyle), sütunlar türü (`dim_type`), ölçülen değer
  (`value`), birimi (`unit`) ve elle yazılmış metni (`text`); yazılarla aynı seçime (`annotations`) bağlıdır.
- Etiket ve görünüm işareti başka nesneyi gösteren açıklamadır: türevi yok, nedeniyle sayılır.
- Kaynak özeti nesnenin kendi JSON'udur: duvarın bandı birleşimlerinden bağımsız kendi değerlerinden çizilir, komşusu değişince
  türevi “GIS'te değişmiş” görünmez.

### 5. Masaüstü

- **GIS türevi dışa aktar** penceresinde **Biçim** (GeoPackage, Shapefile, GeoJSON, PostGIS); PostGIS'te adres ve şema alanları,
  Veritabanına yaz.
- **GIS türevini denetle** GeoPackage'ın yanında zip ve GeoJSON'u da açar; GIS'te değişen satır varsa **Geometriyi de al** anahtarı
  ve **GIS'teki değişiklikleri al** (`cad.gis.takeback`, tek geri alma adımı), türev varken **Bağı kopar** (`cad.gis.detach`).

## Doğrulama

- Ortak durumlar KentOS kodu olmadan: `/usr/bin/python3 scripts/fixtures/gis_takeback_cases.py --check` (geri alma 53, bağ koparma
  26; `cad.gis.derivation`'ın 33 durumu `cad_gis_derivation_cases.py`'den). GIS'te düzenlenmiş kopyalar OGR ve json ile
  (`--files`: `ada-geri.gpkg`, `ada-shp-geri.zip`, `ada-yol-geri.geojson`, `mimari-geri.gpkg`); beklenenler çizimin tanımından ve
  GIS'in geometrisinden, kimlikler dosyanın `kentos_uids`'inden. Rust: `KENTOS_CASES_ONLY=cad.gis cargo test -p kentos-native-application --test all every_command_case`.
- KentOS'un yazdığı dosyalar: `KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test gis derived_` Shapefile zip'ini, iki GeoJSON'u,
  mimari GeoPackage'ı ve bağı koparılmış GeoPackage'ı yazar; `gis_takeback_cases.py --written` onları OGR ile okur: Shapefile ve
  GeoJSON satırları GeoPackage türevinin satırlarıyla köşe köşe ve değer değer aynı, mimari satırlar `mimari.json`'dan burada
  hesaplananla aynı (duvarın bandı, kolonun ve bantların dikdörtgenleri, açıklığın izi, ölçünün değeri), not ve nedeni yerinde,
  kimlikler gitmiş, GDAL'ın `validate_gpkg`'si temiz.
- PostGIS: `/usr/bin/python3 scripts/fixtures/postgis_cases.py --only a_drawing_goes_to_postgis` geçici docker veritabanında
  (5432'ye dokunmaz): dışa aktarma, ikinci kez `table_exists`, denetim, GIS'in (psql) değiştirdiği satırın geri alınması, bağ
  koparma; şema psycopg2 ile denetlenir (yorumlar not, kimlikler boş, GIS'in değişikliği yerinde).
- Masaüstü: `cargo test -p kentos-desktop --bin kentos-cad gis_` (pencereler), resimler `.run/shots/gis-turev-*`.
- Süreler (`gis_timing`, release, bu makinede; `KENTOS_GIS_TIMING_PARCELS=20000`: 20 000 yaylı parsel, 4 000 daire, 4 000 elips,
  0,1 mm kirişle milyonlarca köşe; derleme kuyruğunun 9,5 GB bellek sınırında 100 000 parsellik ADR 0530 ölçümü bitmedi):

  | İş | Süre |
  |---|---:|
  | GeoPackage türevi (195 MB) | 2,0 s |
  | denetim, GeoPackage | 1,1 s |
  | 2 000 satırın geri alınması | 0,52 s |
  | Shapefile zip türevi (34 MB) | 2,6 s |
  | denetim, Shapefile | 1,6 s |
  | GeoJSON zip türevi (82 MB) | 5,4 s |
  | denetim, GeoJSON | 4,2 s |

  Shapefile ve GeoJSON yazıcıları önce JSON ağacı kurup 9,8 ve 12,4 s sürüyordu: Shapefile'ın nesneleri sözleşmenin türleriyle
  doğrudan, GeoJSON metni akışla ve sayılar ryu ile yazılır, arşiv hafif sıkıştırmayla (ADR 0223 §8). GeoJSON'un okunması metnin
  bütününün ayrıştırılmasıdır; türlü ayrıştırma sonraya.

