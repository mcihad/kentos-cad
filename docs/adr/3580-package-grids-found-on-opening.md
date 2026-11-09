# 3580 — Paketten açılan projede ızgaraların kendiliğinden bulunması

Tarih: 2026-11-09. plan/map-cad.md MAP-17, altıncı tur. Durum: taslak. ADR 3180'in “Sonra”sındaki “paketten açılan
projede ızgaraların kendiliğinden bulunması: komutlar `izgaralar/`'a da baksın ya da açılışta kütüphaneye eklensin”.

İlgili:
- [ADR 0506](0506-relative-sources-and-package.md): bağlı kaynağın göreli yolu, açılışta yeniden bulunması, Paketle;
- [ADR 3180](3180-delivery-package-grids.md): teslim paketinde projenin kullandığı ızgaralar (`izgaralar/<SHA-256>`).

## Dayanak

- **PROJ, Resource files** ([belge](https://proj.org/en/stable/resource_files.html), 9 Kasım 2026'da yeniden okundu):
  ızgaralar arama yollarında aranır: kullanıcının klasörü, `PROJ_DATA`, kurulum klasörü ve **çalışma klasörü**. Proje
  klasörünün kendi ızgarası PROJ'un kendi kuralında da bir arama yeridir.
- **KentOS bugün:**
  - Paketle projenin adladığı ızgaraları `izgaralar/<SHA-256>.<gsb|gtx|tif>` olarak taşıyor (ADR 3180);
  - komutlar ızgarayı yalnız cihazın kitaplığında arıyordu. Paketi alan makine dosyaları Izgaralar penceresiyle elle
    eklemedikçe geoit kotları `geoid_missing` ile reddediliyordu, datum seçimi değer vermiyordu;
  - açılışta bağlı GeoPackage'lar zaten göreli yollarından bulunuyor (`source_places::relocate`, ADR 0506).
- **Seçenekler:** açılışta kitaplığa kopyalamak kullanıcının makinesini sorusuz değiştirir; dosyayı yerinde okumak
  değiştirmez. KentOS ikincisini seçti: kalıcı kitaplık kullanıcının elindedir.

## Karar

1. **Açılışta** (`project_grids::find`, dosyadan açılan her projede: masaüstü, başsız sunucu, Python, MCP) projenin
   adladığı her ızgara için sırayla bakılır:
   - çekirdekte okunmuşsa (bu çalışmada) **core**;
   - cihazın kitaplığında dosyası varsa **library** (komut isteyince okunur, bugünkü gibi);
   - yoksa proje dosyasının yanındaki `izgaralar/<SHA-256>.<uzantı>`: SHA-256'sı adıyla tutarsa çekirdeğe okunur
     (**package**), tutmazsa kullanılmaz (**changed**), okunamazsa nedeniyle (**unread**);
   - hiçbiri yoksa **missing**.

   Adlanan ızgaralar Paketle'ninkilerdir: datum seçimlerinin NTv2'si, kotların geoit modeli, kampanyaların hız ızgarası.
   SHA-256'sız ayar dosya adlamaz, sayılmaz.
2. **Cihazın kitaplığı yazılmaz.** Izgara bu çalışma için çekirdektedir. Kalıcı olsun isteyen Izgaralar'dan ekler; ileti
   bunu söyler.
3. **Söylenen:**
   - masaüstünde açılış iletisi: “Projenin yanındaki izgaralar/ klasöründen 1 ızgara kullanılıyor: “TG-20 (paket)””.
     Değişmiş ya da hiçbir yerde olmayan ızgara uyarı olarak söylenir;
   - başsız sunucunun özeti (`summary.grids`, MCP'nin `drawing.open`'ı): her ızgaranın adı, SHA-256'sı, türü (`ntv2`,
     `geoid`, `velocity`), ayarı ve bulunduğu yer.
4. Komutlar değişmez: çekirdekteki ızgarayı kitaplığınki gibi kullanırlar (geoit ve hız modelinde `crs_models::load`,
   NTv2'de `ntv2::get` önce çekirdeğe bakar).
5. **Kapsam işareti:** Paketle gibi **CAD + CBS** (her modül).

## Kanıt

- **Çekirdek** (`project_grids::tests`; TG-20'nin son düğümü bir milimetre oynatılmış, başka testin okumadığı model):
  - dosya yanında yokken missing;
  - adıyla tutmayan dosya changed, çekirdeğe girmez;
  - doğru dosya package ve çekirdekte; sonraki bakış core; ileti adını ve sayısını söyler.
- **MCP** (`crates/native/mcp/tests/package_grids_found.rs`):
  - ölçek katsayılarının TM36 projesi kendi geoidi ve Türkiye NTv2 ızgarasıyla, kitaplıkta ikisiyle açılır (library) ve
    paketlenir;
  - kitaplık boşken proje dosyası tek başına iki ızgarayı missing der, `cad.crs.factors` `geoid_missing` ile reddedilir;
  - paketin dosyası ikisini package bulur, komut cevap verir; boş kitaplık boş kalır.
- **Bağımsız** (`python/tests/test_package_grids_found.py`, hashlib): Python'un `Document.open`'ı ile aynı akış
  (paketteki dosyanın adı SHA-256'sı, tek başına ret, paketten cevap, kitaplık yazılmaz).
- **Masaüstü** (`exchange::package_tests::a_package_s_grid_is_found_when_it_is_opened`): paketten açılınca ileti
  ızgaranın adını söyler, model çekirdektedir. Paketle testleri ve ADR 3180'in testleri değişmeden geçer. Resimler
  `.run/gui-evidence/paket-izgara-acilis-3580/`.

## Sonra

- DXF'te TABLE (ACAD_TABLE) yazımı: varlık çok karmaşık (Autodesk DXF başvurusu; ezdxf de yalnız okur). AutoCAD ya
  da bağımsız bir okuyucuyla doğrulanamadan yazılırsa dosya başka programda hiç açılmayabilir; doğrulama yolu bulunana
  dek tablo çizgi ve yazı olarak kalır.
- Tanımı olmayan ek özniteliklerin ATTRIB'i (MAP-17'nin öbür maddesi).
- Web (TODOS §0.2).
