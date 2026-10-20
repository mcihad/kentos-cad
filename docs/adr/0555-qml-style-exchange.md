# ADR 0555: Görünüşün QML alışverişi (QGIS'in stil dosyası)

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-17'nin ikinci dilimi (ADR 0516'nın “sonraya kalan”ındaki QML);
  CBS-44'ün QGIS 4.2 ile görünüş alışverişi. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0516 (SLD alışverişi: profil, kayıpların söylenmesi, komutların biçimi), 0213
  (`cad.layers.renderer`), 0100 (ifade dili), 0090–0094 (stil sistemi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - QGIS'in öbür simge katmanları: çizgi deseni dolgusu (KentOS'un taramasıyla), degrade, nokta deseni, işaretli çizgi,
    SVG işareti, yazı işareti;
  - veriye bağlı değerler (QGIS'in ifadesi KentOS'unkine çevrilerek);
  - etiketler (`labeling`), katmanın saydamlığı ve karışım kipi (Katman çizimi, ADR 0226);
  - tematik işleyiciler (ısı haritası, nokta yoğunluğu, kümeleme …) ve MapLibre stili;
  - QGIS'in kendi arayüzüyle elle denetim.

## Bağlam

SLD (ADR 0516) kurumlar arası ortak dildir, ama QGIS'in söylediğinin bir kısmını taşır: boylar 0,28 mm'lik piksele
çevrilir, simgenin saydamlığı, kapalı katman, iç içe kural, merkez işareti yoktur; QGIS kendi SLD'sini her zaman kurallı
görünüş olarak okur. QGIS kullanıcısının görünüşü QGIS'in kendi stil dosyasıyla, `.qml` ile taşınır. Plan, dış stilin
desteklenen alt kümesinin açık olmasını ve çevrilemeyenin söylenmesini ister.

## Karar

### 1. Profil

`kentos_formats::qml` QGIS 4.2'nin `renderer-v2`'sini yazar ve okur. QGIS 3'ün ve 2'nin dosyaları da okunur (`<Option
type="Map">` ve eski `<prop k v>`).

**Görünüşler:** tür türe.

| KentOS | QML |
|---|---|
| tek simge | `singleSymbol` |
| kategorili (alanın değerleri; Diğerleri) | `categorizedSymbol` (`attr` alanın adı; Diğerleri `type="NULL"` sınıfı) |
| aralıklı | `graduatedSymbol` (`lower`, `upper`) |
| kurallı (iç içe, ölçek aralığı, Diğerleri, kapalı) | `RuleRenderer` (`rule` iç içe, `scalemindenom`/`scalemaxdenom`, `ELSE`, `checkstate="0"`) |

- QGIS'in çok değerli sınıfı (`<val>`'ler) KentOS'ta her değer için bir sınıf olur, simgeleri aynı.
- Aralıkların sınırı farklıdır: QGIS'te alt < değer ≤ üst (ilki hariç), KentOS'ta alt ≤ değer < üst (sonuncu hariç).
  İki yönde de birden çok sınıf varsa kayıplarda söylenir.

**Simge katmanları:**

| KentOS | QML | Taşınanlar |
|---|---|---|
| `simpleFill` | `SimpleFill` | renk ve saydamlığı; kenarı ayrı `SimpleLine` |
| `simpleLine` | `SimpleLine` | renk, kalınlık, birim, özel kesikler, uç, köşe, halkalar (`ring_filter`) |
| `shape` işaret | `SimpleMarker` | on beş şekil, dolgu, kenar ve kalınlığı, boy, birim, döndürme |
| `centroidMarker` | `CentroidFill` | iç işaret simgesi; iç nokta ya da ağırlık merkezi (`point_on_surface`) |

- Simgenin saydamlığı `alpha`'dır, katmanın kapalılığı `enabled="0"`.
- Okurken QGIS'in dolgusunun kenarı KentOS'ta ayrı çizgi katmanı olur.
- Qt'nin adlı kesik desenleri (`dash`, `dot`, `dash dot`, `dash dot dot`) kalemin kalınlığının katları olarak alınır
  (4-2, 1-2 …; kıl çizgide kalınlık 1).
- Şekiller: `circle`, `square`, `diamond`, `triangle`, `pentagon`, `hexagon`, `octagon`, `star`, `cross`, `x`↔`cross2`,
  `line`, `arrow`, `arrowhead`↔`filled_arrowhead`, `chevron`↔`arrowhead`, `semicircle`↔`semi_circle`.
- Okurken `equilateral_triangle`, `quarter_circle` ve `cross_fill` en yakın KentOS şekli olur; bilinmeyen şekil kare
  alınır ve söylenir.
- Yazarken `ring` daire, `rectangle` kare yazılır ve söylenir. `gear` ve `arc` QGIS'te yoktur, yazılmaz.

**Birimler:**

| KentOS | QML |
|---|---|
| mm | `MM` |
| piksel | `Pixel` |
| m | `RenderMetersInMapUnits` |

- Okurken `Point` ve `Inch` mm'ye çevrilir.
- `MapUnit` metre okunur: KentOS projelerinin izdüşümlü sistemlerinde harita birimi metredir.
- Yüzde boyu alınmaz.
- Kesiklerin ve işaret kenarının ayrı birimi, katmanın birimine çevrilemiyorsa (kâğıt ile piksel ya da metre) o değer
  alınmaz ve söylenir.

**Döndürme:** KentOS saat yönünün tersine, QGIS saat yönünde döndürür; işaret değişir.

### 2. Süzgeçler

Kuralların süzgeci QGIS'in ifadesidir: `"Ad" = 'x'`. KentOS'unki `[Ad] = 'x'`. İkisi SLD'nin yazıcısı ile okuyucusunun
paylaştığı süzgeç dilbilgisinden (`sld::Filter`) geçer.

- Tanınanlar: alanın sayı ya da metinle karşılaştırması, `BETWEEN … AND …`, `IS NULL`, `IS NOT NULL`, `LIKE`, `AND`,
  `OR`, `NOT` ve parantez.
- QGIS'in `IN (…)`'u eşitliklerin `veya`'sı olur (`parse_qgis`; ayrıştırıcı QGIS kipinde çift tırnaklıyı alan sayar).
- Dilbilgisinin söyleyemediği süzgeç (bir işlev, bir hesap) çevrilmez:
  - yazarken kural alt kurallarıyla yazılmaz;
  - okurken kural alt kurallarıyla alınmaz;
  - ikisi de söylenir.
- Kategorili ve aralıklı görünüşün `attr`'ı bir alan olmalıdır (`"ada"` ya da `ada`). Değilse okuma reddedilir, yazmada
  sınıflar yazılmaz.

### 3. Tek geometri türü

QML bir geometri türü içindir (`layerGeometryType`). Yazıcı katmanın nesnelerinden seçer: alanlar, sonra çizgiler, sonra
noktalar. Katman boşsa alan seçilir. Öbür türlerin simgeleri yazılmaz ve söylenir.

### 4. Kayıplar

Yazarken söylenenler:
- tarama, degrade, desen ve resim dolgusu; işaretli çizgi; geometri üretici;
- SVG, yazı ve raster işareti;
- veriye göre değer (yedek değeri yazılır);
- katmanın saydamlığı ve efektler; simgenin efektleri;
- çizginin paralel kaydırması, dalgası, yumuşak kenarı, gölgesi, kesik başlangıcı;
- işaretin kaydırması, bağlantı noktası, yüksekliği, deliği;
- kitaplıkta olmayan simge; aralıkların sınır kuralı.

Okurken söylenenler:
- başka simge katmanı sınıfları ve fırça desenleri;
- veriye bağlı değerler (sabit değerleri alınır);
- etiketler, katmanın saydamlığı;
- çizginin kaydırması ve kırpması;
- işaretin kaydırması, bağlantı noktası, alana göre ölçeklenmesi;
- kapalı aralık, boya göre aralıklı görünüş, ikinci NULL sınıfı;
- çevrilemeyen süzgeç; aralıkların sınır kuralı.

### 5. Komutlar

`file.export.qml` ve `file.import.qml` SLD'nin komutlarıyla aynı biçimdedir (ADR 0516 §4):
- girdiler `path`, `layer`, `overwrite`; içe almada `path` ya da `text` ve `expectedRevision`;
- retler aynı sırada; kod `invalid_qml`;
- uyarılar `export_losses` ve `import_losses`;
- dışa aktarma çizimi değiştirmez;
- içe alma `cad.layers.renderer`'ın yazmasıyla tek geri alma adımıdır (“Katman stili”), kayıtta kendi adıyla.

Komutlar başsız sunucuda, Python'da (`kentos.cad.file.export.qml`, `kentos.cad.file.import_.qml`) ve MCP'de çalışır.
İkisi de MCP'nin yıkıcı listesindedir.

Masaüstünde Katmanlar'ın sağ tıkında SLD'nin altında iki öğe vardır:
- Görünüşü QML olarak kaydet…
- QML'den görünüş al…

## Doğrulama

- `python3 scripts/fixtures/qml_cases.py --check`: 27 ortak durum, başvuru KentOS kodu olmadan yazıldı.
  - Okuma kuralı ADR'den `xml.etree` ile, QGIS'in ifadesinin ayrıştırıcısı da betiğin kendisinin.
  - Girdiler: QGIS 4.2'nin `saveNamedStyle` ile yazdığı yedi QML ve elle yazılmış dokuz dosya (QGIS 2'nin `prop`'ları,
    kayıplar, süzgeçler, retler).
- `--files` QGIS'in QML'lerini yeniden yazar.
- `KENTOS_WRITE_QML=1 cargo test -p kentos-headless --test headless qml`:
  - beş görünüş (kategorili ve Diğerleri, aralıklı, iç içe kurallı, metreli ve saydam çizgi, işaret) yazılır ve geri
    okunur;
  - dosyalar `fixtures/qml/v1/out`'a gider.
- `qml_cases.py --check` bu dosyaları QGIS 4.2'de `loadNamedStyle` ile açar ve şunları denetler: tür, alan, sınıfların
  değeri, etiketi ve rengi, kuralların süzgeci, ELSE'i ve ölçekleri, katmanların sınıfı, kalınlık ve birimi, kesikler,
  şekil, boy, açı, simgenin saydamlığı.
- `cargo test -p kentos-formats --lib qml`: çekirdeğin birim testleri (gidiş-dönüş, iki dil arası süzgeç, işaretin
  dönüşü, kayıplar).
- `pnpm py:test` içinde `python/tests/test_qml.py`.
