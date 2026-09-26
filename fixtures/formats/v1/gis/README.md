# GeoJSON ve Shapefile içe aktarma fixture'ları (`formats/v1/gis`)

GeoJSON (RFC 7946) ve Shapefile (ESRI Shapefile Technical Description, Temmuz 1998; dBASE III+) okumasının ortak kurallarını sınayan dosyalar. Kurallar [ADR 0046](../../../../docs/adr/0046-geojson-and-shapefile.md)'dadır. Aynı dosyaları üç okuyucu okur: Rust okuyucusu (`crates/shared/formats`, `tests/gis.rs`), aynı kodun tarayıcıdaki WASM modülü (`apps/web/src/io/gis.wasm.test.ts`) ve Rust'a bakmadan kurallardan yazılmış bağımsız Python başvurusu [`tools/formats/gis.py`](../../../../tools/formats/gis.py). Her fixture'ın `<ad>.expected.json`'ı başvurunun kanonik çıktısıdır; Rust ve WASM her fixture'ı aynı nesnelere, float'ları bit bit aynı, aynı bildirilen SRID ve kodlamayla okur.

Dosyaları [`scripts/fixtures/gis_reference.py`](../../../../scripts/fixtures/gis_reference.py) yazar, yalnız Python standart kitaplığıyla ve KentOS kodu kullanmadan. GeoJSON metinleri elle yazılmıştır (sayıların yazılışı, kaçışlar, üye sırası, yinelenen anahtar tam istendiği gibi olsun diye; betikte `\u` kaçışları `@u` diye yazılır, betik ters bölüyü kendisi koyar). Shapefile takımları `struct` ile bayt bayt paketlenir. Okuyucunun çıktısı tek kanıt değildir: betikteki `SUMMARY` her fixture'ın vermesi gerekenleri kurallardan elle yazar; her Shapefile takımı (başlık, kapsam, z/m aralıkları, kayıt numaraları, `.shx` yerleri, tablo) ayrıca ayrıştırılır, halkaların yönü ve deliklerin yeri kesirli (tam) aritmetikle denetlenir.

```bash
python3 scripts/fixtures/gis_reference.py            # dosyaları yazar (denetimden geçmezse hiçbirini yazmaz)
python3 scripts/fixtures/gis_reference.py --check    # hiçbir şey yazmaz; karşılaştırır ve okur
python3 tools/formats/gis.py read fixtures/formats/v1/gis/noktalar.shp   # bir fixture'ın kanonik çıktısı
```

`--check` her dosyayı bellekte yeniden üretip diskteki ile bayt bayt karşılaştırır, betiğin yazmadığı dosyayı bildirir, diskteki dosyaları yukarıdaki gibi denetler ve her fixture'ı okuyucunun dosya yoluyla (CLI gibi) okuyup `expected.json` ve `SUMMARY` ile karşılaştırır. Ayrıca `export/` altındaki Rust GeoJSON yazıcısı çıktısını denetler (aşağıda). Farkta sıfırdan farklı kodla çıkar.

## Kanonik çıktı

`{"declaredSrid": N | null, "encoding": "…" (yalnız Shapefile), "objects": [...]}`; nesneler okuma sırasıyla `point` (`p`, `z`?), `line` (`a`, `b`), `polyline` (`pts`) ya da `polygon` (`pts`, `holes`?), hepsinde `kind`, `layer`, `label`? ve `attrs` (değerler hep metin). Anahtarlar bu sırada yazılır; `?` olanlar yalnız varsa bulunur, `attrs` hep vardır (boş olabilir). Dosya bir boşlukla girintili, UTF-8'dir; float'lar en kısa geri dönen yazımladır.

## Fixture'lar

| Dosya | Sınadığı | Sonuç |
|---|---|---|
| `features.geojson` | FeatureCollection, en sonda yazılan `name` (varsayılan katman `ornek`); z'li nokta, 3B+2B MultiPoint, 2 ve 4 konumlu LineString, MultiLineString, delikli Polygon, bir dış halkası kapanmamış MultiPolygon, GeometryCollection ve iç içe olanı, null geometri, tek konumlu LineString, dört sayılı nokta, `kentos` (katman `Sınır`/etiket `P-12`; sayı katman; yalnız etiket; boş metinler), bilinmeyen tür, `features` içinde çıplak geometri ve `geometry`'siz Feature; özniteliklerde `\u` kaçışlı Türkçe, `1.50`, `1e3`, `-0.0`, `-0`, u64'ü aşan tamsayı, uzun ondalık, true/false/null, boş metin/nesne/dizi, iç içe nesne (vekil çiftiyle yazılmış karakter) ve satır sonu, tırnak, `\/`, sekme, denetim karakteri, ters bölü taşıyan dizi (sıkı JSON metni), yinelenen anahtar | 4326; 6 nokta, 4 çizgi, 3 çoklu çizgi, 4 alan |
| `tm.geojson` | eski `crs` adı `urn:ogc:def:crs:EPSG::5256`; float64'ten uzun ve üslü yazılmış koordinatlar; delikli parsel (`Parsel`, `101/5`), bina (`Bina`), `kentos`suz noktalar dosya adı katmanında (`tm`) | 5256; 3 nokta, 2 alan |
| `bare-polygon.geojson` | Feature'sız çıplak Polygon, crs ve name yok; x ve y'si ilkiyle aynı son konum (z'si farklı) düşer; kapanınca iki konuma inen delik dışarıda kalır | 4326; 1 alan (1 delik) |
| `feature-epsg.geojson` | tek Feature, `{"type":"EPSG","properties":{"code":2320}}` | 2320; 1 çoklu çizgi |
| `crs84.geojson` | `urn:ogc:def:crs:OGC:1.3:CRS84`; boş `name` varsayılan katmanı dosya adına bırakır | 4326; 1 nokta |
| `nonfinite.geojson` | `1e999`/`-1e999` en küçük birimi düşürür (MultiPoint'in noktası, LineString, MultiLineString üyesi, bütün Polygon, MultiPolygon üyesi, GeometryCollection üyesi; z'de de); `1e-999` sonlu 0.0'dır; üçüncüden sonraki sonsuz sayı yok sayılır; sayı olmayan ya da eksik konumlar ve geometri olmayan üyeler hiçbir şey vermez; öznitelikteki `1e999` yazıldığı gibi kalır | 4326; 7 nokta, 1 çizgi, 1 çoklu çizgi, 1 alan |
| `noktalar.shp` | PointZ (M'li) ×3; dil sürücüsü 0xCA, `.cpg` yok; C (`ÇŞĞÜÖİı`; baştaki boşluk kalır, boş alan düşer), N (`  105.200`), F (`1.23456780000e+003`), L (`T`, `F`, `?`), D (`20260926`, boşluklar, `00000000`); AUTHORITY'siz TUREF TM36 ESRI WKT | 5256, Windows-1254; 3 nokta |
| `yollar.shp` | PolyLine: 2 ve 4 noktalı parçalı kayıt, tek noktalı parçalı kayıt (hiçbir şey; tablo sırası kaymaz), 3 noktalı parça; `.cpg` `UTF-8` (0x57 dil sürücüsü dikkate alınmaz); UTF-8 alan adı `ŞERİT`; PROJCS'in kendi `AUTHORITY["EPSG","2320"]`'si (içtekiler değil) | 2320, UTF-8; 1 çizgi, 2 çoklu çizgi |
| `parseller.shp` | Polygon: saat yönünde dış halka ve ters yönde deliği; iki dış halka, delik kendi dış halkasından önce yazılmış; yalnız ters yönde halka (kendi başına alan); tabloda silinmiş kayıt; alanı tam sıfır halka; dil sürücüsü 0x6B (CP857), CP857 alan adları `MALİK`, `NİTELİK`; AUTHORITY'siz WGS 84 GEOGCS | 4326, CP857; 5 alan |
| `kuyular.shp` | MultiPointZ (M'li), 3 ve 2 nokta; `.cpg` `ISO-8859-9`; L `Y`, `n`; D `20250314` ve ` 1.3.25 `; `.prj` yok | null, ISO-8859-9; 5 nokta |
| `karisik.shp` | PointM, araya Null kayıt; tabloda bir kayıt eksik (son noktanın özniteliği yok); tanınmayan `.cpg` `KOI8-R` (0x03 dil sürücüsüne dönülmez); UTM 36N'in bütün parametreleriyle Lambert Conformal Conic | null, Windows-1254; 3 nokta |
| `alanlarz.shp` | PolygonZ (M'li): dış halka ve iki delik (z düşer); dil sürücüsü 0x03, `é` içeren metin; AUTHORITY'siz WGS 84 / UTM 36N ESRI WKT | 32636, Windows-1252; 1 alan (2 delik) |

Fixture'larda bilerek bulunmayanlar: tek başına vekil (lone surrogate), iç içe nesnede yinelenen anahtar ve kod sayfasına göre okuyucularda farklı çözülen baytlar (Windows-1252/1254'te 0x81, 0x8D, 0x8E, 0x8F, 0x90, 0x9D, 0x9E; CP857'de 0xD5, 0xE7, 0xF2; ISO-8859-9'da 0x80–0x9F, çünkü WHATWG bu adı Windows-1254 okur). Betik bu baytları tabloda arar.

## `export/`: GeoJSON yazıcısının çıktısı

`export/<ad>.input.json` yazıcıya verilen girdidir (`{"entities": [...], "layers": [{"id", "name"}], "srid": N, "name": S}`), `export/<ad>.geojson` Rust yazıcısının yazdığıdır; bu betik onları yazmaz. `--check` her çifti bağımsız okuyucuyla geri okur ve yazıcının kurallarıyla karşılaştırır: kök, adı girdinin adı olan FeatureCollection; SRID 4326'da `crs` yok, öbürlerinde `urn:ogc:def:crs:EPSG::<srid>`; nesneler girdinin sırasıyla (text, dimension, xline, ray yazılmaz), katman adı, boş olmayan etiket ve öznitelikler aynı; point, line, yaysız polyline ve yaysız polygon/hatch koordinatları bit bit (alan sağ el kuralıyla: dış halka saat yönünün tersine, delik saat yönünde; ters yöndeki halka ilk köşesi yerinde kalarak çevrilir, alanı sıfır halka olduğu gibi); daire 73 noktalı kapalı yol; yay, elips, eğri ve yaylı yol/alan için uç ve çember toleransı `1e-9·max(1, r)`. Tam elips `|t1 − t0| ≥ 2π − 1e-9` sayılır. Denetlenen çiftler `--check` çıktısında listelenir.

## Belirsizlikler ve seçimler

Kuralların açık bıraktığı, başvurunun seçtiği noktalar; Rust okuyucusu da böyle okur (ADR 0046), ayrıldığı yer aşağıda ayrıca yazılıdır:

- `.cpg`: boşluk, sekme ve satır sonları her yerden silinir, ASCII harfler büyütülür; tablo adları da aynı biçimde karşılaştırılır (`ANSI 1254` = `ANSI1254`). `.cpg` Latin-1, `.prj` UTF-8 (hatalı bayt U+FFFD) olarak okunur.
- `crs` adı: `EPSG:`/`EPSG::` sonu büyük/küçük harf duyarsız ve metnin tam sonunda; `CRS84` araması duyarlı. `{"type":"EPSG"}` kodu kesirsiz, üssüz tamsayı yazımı ya da yalnız rakamlı metindir.
- Alanı tam sıfır halka hiçbir zaman dış halka olmaz (saat yönünde halkası olmayan kayıtta da). Hiçbir dış halkanın tutmadığı delik kendi başına alan olur ama başka delik almaz; delikleri yalnız saat yönündeki halkalar alır.
- M türlerinde (PointM, MultiPointM, PolyLineM, PolygonM) M bloğu gereklidir (eksikse kayıt hiçbir şey vermez); yalnız Z türlerinde isteğe bağlıdır.
- `.shp` başlığında yalnız 100 bayt ve 9994 dosya kodu aranır (yoksa dosya reddedilir); başlıktaki uzunluk ve sürüm denetlenmez, `.shx` okunmaz.
- DBF: C alanının sonundaki, N, F, L ve D'nin iki ucundaki boşluk ve NUL baytları silinir (bazı yazıcılar alanı NUL ile doldurur); D'de takvim denetimi yoktur (`20261399` → `2026-13-99`); N, F, L ve D'deki ASCII olmayan bayt U+FFFD olur; `*` dışındaki silinme baytı silinmemiş sayılır.
- Bozuk DBF başlığı (0x0D'siz alan tanımları, kayda sığmayan alanlar): başvuru dosyayı reddeder; Rust okuyucusu kayda sığan alanları okur, sığmayanları ve tablonun hatasını raporlar, şekiller yine alınır. Fixture'larda bu durum yoktur.
- WKT: anahtar sözcükler büyük/küçük harf duyarsız, `[` `]` ve `(` `)` kendi eşiyle kapanır, metinde `""` bir tırnaktır; AUTHORITY adı tam `EPSG`, kodu tırnaklı rakamlardır; datum ailesi önceliği TUREF, ED50, WGS 84; "harf ve rakam" ASCII'dir; çözümlenemeyen WKT ve kökü PROJCS/GEOGCS olmayan WKT null verir.
- GeoJSON: Feature'ın `geometry`'si nesne değilse hiçbir şey vermez; kökü nesne olmayan belge reddedilir.
