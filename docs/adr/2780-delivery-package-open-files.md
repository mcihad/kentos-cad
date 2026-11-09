# 2780 — Teslim paketinde devir dosyaları ve kayıp raporu

Tarih: 2026-11-08. plan/map-cad.md MAP-17, dördüncü tur. Durum: taslak. ADR 1880'in ve 2380'in “Sonra”sındaki “teslim
paketinin matrisi ve kayıp raporunu taşıması”; planın satırı: “Teslim paketi çizim + bağlı kaynaklar + kullanılan
profil/ızgara bilgisi + kayıp/kalite raporu taşısın.”

İlgili:
- [ADR 0506](0506-relative-sources-and-package.md), [0596](0596-package-sources-offline.md), [1111](1111-package-resource-report.md):
  Paketle (`file.package`), kaynakların yolu, yazı tipi ve stil raporu;
- [ADR 1660](1660-exchange-round-trip-check.md), [1880](1880-exchange-support-matrix.md),
  [2380](2380-exchange-blocks-tables-relations.md): gidiş-dönüş denetimi, destek matrisi, bloklar, tablolar, ilişkiler.

## Dayanak

- **AutoCAD eTransmit** (ADR 1111'in okuması): çizim ve destek dosyaları pakete girer; rapor dahil edilenleri ve
  dışarıda bırakılanları ayrı listeler, `.txt` olarak paketin içindedir.
- **MicroStation, Create Package** ([belge](https://docs.bentley.com/LiveContent/web/MicroStation%20Help-v28/en/CreatePackage.html),
  8 Kasım 2026'da okundu): ana tasarım dosyasına bağlı dosyalar (WorkSet, yapılandırma değişkenleri, çizgi stilleri,
  tohum dosyaları, iç içe başvurular) göreli yollarla bir `.pzip`'e toplanır. Belge ayrı bir kayıp raporundan söz etmez.
- **QGIS, Package layers** ([belge](https://docs.qgis.org/3.40/en/docs/user_manual/processing_algs/qgis/database.html),
  8 Kasım 2026'da okundu): katmanlar tek bir GeoPackage'a toplanır, stiller ve üst veri isteğe bağlı; ilişkili katmanlar
  projenin ilişkileriyle eklenebilir. Biçimin taşımadığı şey raporlanmaz.
- **KentOS bugün:**
  - Paketle projeyi, kullandığı dosyaları, dış başvuru kaynaklarını, profilleri ve kaynak raporunu (`paket.json`)
    taşıyordu. Açık biçimde bir kopya yoktu;
  - Dışa aktarma kaybı denetimi ve destek matrisi (ADR 1660, 1880, 2380) bir biçimin neyi koruyup neyi kaybettiğini
    söylüyordu, ama yalnız pencerede ve kendi geçici dosyasında. Teslim edilen dosyayla bağı yoktu.

Üreticilerin paketleri dosyaları toplar, açık biçime dönüşün kaybını söylemez. KentOS paketin açık dosyalarını teslim
edildikleri hâliyle denetler ve kaybı pakete koyar.

## Karar

1. **`file.package`'a `exchange`** (ek alan; komut sürümü değişmez):
   - `file.exchange.matrix`'in sorusu: biçimler (boş: altısı), katmanlar, tolerans;
   - her biçim KentOS'un yazıcısıyla `devir/<ad>.<uzantı>`'ya yazılır. Ad proje dosyasının adıdır;
   - yazıcının yanına yazdığı her dosya da (GML'in `.xsd` şeması) pakete girer.
2. **Denetim teslim edilen dosyanın kendisidir:**
   - dosya planın geçici klasöründe yazılır ve oradan `file.exchange.check`'in kurallarıyla geri okunur (`trip`);
   - paket o dosyayı bayt bayt kopyalar, SHA-256'sıyla geri okuyup karşılaştırır (ADR 0506'nın kopya kuralı);
   - yazılan nesneler paketin nesneleridir: `leaveOut` ile dışarıda bırakılan katmanın nesneleri açık dosyalara da girmez.
3. **Taşınanlar:**
   - cevabın ve `paket.json`'un `exchange`'i: destek matrisi ve her biçimin denetimi (`ExchangeMatrixed`). İstenmezse
     yoktur; eski paketlerin `paket.json`'u aynı kalır;
   - `files`'ta her açık dosya `exchange` türüyle; `from`'u onu yazan komut (`file.export.gpkg` …), raporunki
     `file.exchange.matrix`;
   - `devir/devir-raporu.csv`: matris, Dışa aktarma kaybı denetimi penceresinin sözleriyle. Baş satır biçimler, her
     satır bir özellik; hücreler `;` ile, UTF-8. Sözler masaüstüyle ortaktır (`kentos_commands::exchange_words`).
4. **Uyarı `export_losses`:** açık dosyalar bir şeyi korumuyorsa bir kez söylenir: biçim biçim korunmayan özellikler;
   ayrıntının yeri; proje dosyasının kaynak olduğu.
5. **Retler:**
   - sorunun kendi retleri kaynaklar okunmadan önce: `invalid_tolerance` (`exchange.tolerance`), `layer_not_found`
     (`exchange.layers[i]`);
   - yazıcının kendi reddi (örneğin `nothing_to_export`) ve `exchange_unreadable` `exchange.formats[i]` altında;
   - doğrulama dosya yazmaz.
6. **Masaüstü** (Paketle):
   - pencere bağlı katman olmasa da açılır. **Devir dosyaları** alanında altı biçim, üçer üçer;
   - özette işaretli biçimler; ileti devir dosyalarını ve raporun yerini söyler, uyarı ayrıca.
7. **Kapsam işareti:** Paketle ve destek matrisi gibi **CAD + CBS** (her modül).

“Kullanılan profil/ızgara bilgisi”nin profil kısmı ADR 0558'den beri pakettedir (`profiller/`). Izgara (CRS dönüşüm
ızgarası) bilgisi bu ADR'de yoktur.

## Kanıt

- **Rust** (`kentos-headless`):
  - `package_exchange::tests`: raporun baş satırı ve hücre sözleri, uyarının sözleri, `;` içeren hücre, iki kez istenen
    biçim;
  - `tests/headless/package_exchange.rs`, örnek çizim (fixtures/exchange/v1/devir.json) altı biçimle:
    - yedi açık dosya ve rapor, `paket.json`'daki boy ve SHA-256'yla;
    - paketteki matris, paketin projesi klasörden açılınca `file.exchange.matrix`'in verdiğidir;
    - ADR 1660'ın tablosu: yazı DXF dışında kaybolur;
    - uyarı bir kez; plan paket yazmaz; tek biçim; retler `exchange.` altında;
  - eski paket, kaynak raporu ve devir testleri aynen geçer.
- **Bağımsız** (`python/tests/test_package_exchange.py`, KentOS kodu yok):
  - hashlib: her dosyanın boyu ve SHA-256'sı `paket.json`'dakidir;
  - GDAL `ogrinfo`: her CBS dosyasında denetimin yazıldı dediği kadar nesne (Shapefile zip'in içinde);
  - ezdxf: DXF'te çizimin yazısı vardır, matris onu korunur der;
  - csv: baş satır altı biçim, her hücrenin ilk sözü matristeki durumdur;
  - her biçimin denetimi, paketin proje dosyası açılınca `file.exchange.check`'in verdiğidir.

  5 test geçer. Eski `test_exchange_check.py` aynen geçer (10 test).
- **MCP** (`crates/native/mcp/tests/package_exchange.rs`): bloklu örnek GeoPackage ve DXF'le paketlenir. Blok ve tablo
  yalnız DXF'te, ilişki yalnız GeoPackage'da korunur (ADR 2380'in tablosu); `paket.json` aynı matrisi taşır.
- **Masaüstü** (`exchange/package_tests.rs`):
  - bağlı katmansız çizimde pencere açılır;
  - DXF ve GeoPackage matrisin sırasıyla gider;
  - paket iki dosyayı ve raporu taşır; ileti ve uyarı söylenir.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/teslim-devir-2780/` (raporun örneği yanında).

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("ada.kcad")
r = cad.file.package(doc, path="/teslim/ada", exchange={"formats": ["gpkg", "dxf"]})
for row in r.exchange.rows:
    print(row.aspect, [(c.format, c.state) for c in row.cells])
```

MCP: `{"tool": "file.package", "arguments": {..., "exchange": {"formats": ["gpkg", "dxf"]}}}`.

## Sonra

- Paketin CRS ızgara bilgisi yapıldı: [ADR 3180](3180-delivery-package-grids.md) (projenin adladığı NTv2, geoit ve hız
  ızgaraları `izgaralar/`'da).
- Kurum profiline göre biçim önerisi; LandXML ve IFC alt modelleri.
- Web (TODOS §0.2).
