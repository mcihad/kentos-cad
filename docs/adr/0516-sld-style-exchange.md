# ADR 0516: Görünüşün SLD alışverişi (OGC SLD 1.1 / SE 1.1)

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-17'nin ilk dilimi (“dış stile ait desteklenen SLD/SE … alt
  kümesinin açık import/export profili ve çevrilemeyen kuralların raporu”); CBS-44'ün QGIS 4.2 ile görünüş alışverişi.
- **Bağlam belgesi:** ADR 0090–0094 (stil sistemi, `.kstil`), 0213 (`cad.layers.renderer`), 0100 (ifade dili), 0514
  (dışa aktarmada kayıpların söylenmesi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - QML (QGIS'in kendi biçimi) ve MapLibre stili;
  - etiketlerin TextSymbolizer'ı;
  - tematik işleyiciler;
  - stilin sağlayıcının alan kimliklerine bağlanması (CBS-17'nin öbür yarısı).

## Bağlam

Kurumlar görünüşü QGIS, GeoServer ve MapServer arasında SLD ile taşır. KentOS'un görünüşü kendi JSON'udur
(`model/style.ts`: tek simge, kategorili, aralıklı, kurallı görünüş ve dokuz tematik işleyici; simgeler dolgu, çizgi ve
işaret katmanları). Plan desteklenen alt kümenin açık olmasını ve çevrilemeyenin sessizce varsayılana düşmemesini ister.

## Karar

### 1. Profil

`kentos_formats::sld` SLD 1.1'in Symbology Encoding 1.1 ile yazılan, QGIS 4.2'nin, GeoServer'ın ve MapServer'ın ortak
okuduğu alt kümesini yazar ve okur.

**Kurallar:** OGC süzgeci, `ElseFilter`, Min/MaxScaleDenominator.

**Simgeleyiciler:**

| Simgeleyici | Taşınanlar |
|---|---|
| PolygonSymbolizer | `Fill` + `Stroke` |
| LineSymbolizer | renk, saydamlık, kalınlık, kesikler, uç, köşe, `PerpendicularOffset` |
| PointSymbolizer | iyi bilinen işaretler (`circle`, `square`, `triangle`, `star`, `cross`, `x`): dolgu, kenar, boy, dönüş |

**Görünüşler:**

| KentOS | SLD |
|---|---|
| tek simge | süzgeçsiz tek kural |
| kategorili | her sınıf alanın değerine eşitlik süzgeci; Diğerleri `ElseFilter` |
| aralıklı | `alt ≤ v < üst` (sonuncusu `≤ üst`) |
| kurallı | süzgeçleri ve ölçek aralıklarıyla; iç içe kurallar üstlerinin süzgeciyle `And`'lenir |
| işleyicisi olmayan katman | basit görünüşü (renk, çizgi tipi, kalınlık, dolgu, nokta simgesi) tek simge olarak |

**Süzgeç dili:** KentOS'un ifadesinden SLD'ye çevrilen alt küme:

- alan (`[Ad]` ya da çıplak sözcük; `"…"` KentOS'ta metindir) ile sayı ya da metin karşılaştırması
  (`= <> < <= > >=`);
- `arasında … ve …`, `boş`, `boş değil`, `gibi`;
- `ve`, `veya`, `değil`, parantezler.

Bunun dışındaki ifade (işlev, `$` değeri, iki alan karşılaştırması) çevrilmez.

**Birimler:**

- Kâğıt mm, SE 1.1'in 0,28 mm'lik standart pikseliyle piksel yazılır; QGIS de böyle yazar (10 mm → 36 px).
- Metre `uom` metre, ekran pikseli piksel yazılır.
- Okurken piksel piksel, metre metre olur; QGIS de pikseli piksel okur.
- Renkler `#rrggbb`; saydamlık `*-opacity`, okununca `#rrggbbaa`.
- Dönüş KentOS'ta saat yönünün tersine, SE'de saat yönündedir; işareti çevrilir.

### 2. Söylenenler

SE'nin söyleyemedikleri yazılmaz ve bir cümleyle söylenir (`export_losses`):

- tarama, degrade, desen ve resim dolgusu, işaretli çizgi, geometri üretici;
- SVG, yazı ve resim işareti; KentOS'un SE'de olmayan şekilleri;
- kenarın halka seçimi, dalga, gölge kaydırması, yumuşak kenar;
- simgenin saydamlığı ve efektleri;
- tematik işleyiciler;
- kitaplıkta olmayan simge;
- SLD'nin süzgeciyle söylenemeyen kural (yazılmaz).

Veriye göre değerin yedeği varsa yedeği yazılır, söylenir.

KentOS'un alamadıkları söylenir (`import_losses`):

- dış resim (`ExternalGraphic`), desenli dolgu (`GraphicFill`), işaretli çizgi (`GraphicStroke`), alanın kaydırması;
- KentOS'ta olmayan işaret (kare yazılır);
- etiket ve raster simgeleyicisi;
- çevrilemeyen süzgeç (kural alınmaz);
- ikinci ve sonraki `FeatureTypeStyle`.

### 3. Okuma

QGIS, GeoServer ve KentOS'un SLD 1.0 ve 1.1'i okunur (`CssParameter` de).

- Bir alanın değerlerine eşit kurallar (ve tek bir `ElseFilter`) kategorili görünüş olur.
- Bir alanın `≥/>` ve `</≤` çiftleri ya da `PropertyIsBetween` kuralları aralıklı görünüş olur.
- Süzgeçsiz ve ölçeksiz tek kural tek simge olur.
- Öbürleri kurallı görünüş olur: süzgeçler KentOS'un ifadesine yazılır (`[alan] > 100 ve [ada] <> 'B'`; `gibi`'nin joker
  karakterleri `%` ve `_` olur).
- Kuralın adı `Title`'dan, yoksa `Name`'den alınır.

Görünüş `cad.layers.renderer`'ın yazmasıyla katmanın olur: tek geri alma adımı “Katman stili”; aynısıysa adım yazılmaz.

### 4. Komutlar

**`file.export.sld`** başsız sunucunun dosya komutlarındandır:

- Simgeleyiciler katmanın nesnelerinin geometrisi için yazılır (nesnesizde üçü de).
- Retler sırasıyla: `invalid_file_path`, `file_exists`, `layer_not_found`, `not_a_layer`, `nothing_to_export`,
  `file_unwritable`.
- Uyarı `export_losses`.

**`file.import.sld`** belgenin komutudur:

- `path` ya da `text` alır.
- Retler sırasıyla: `invalid_input`, `file_unreadable`, `invalid_sld` (yeni kod), `revision_conflict`,
  `layer_not_found`, `not_a_layer`, `service_layer`.
- Uyarı `import_losses`.

**Ortak:** Python ve MCP'de katalogdan gelir; ikisi de MCP'nin yıkıcı listesindedir. Masaüstünde Katmanlar'ın sağ
tıkında Katman stili…'nin altında **Görünüşü SLD olarak kaydet…** ve **SLD'den görünüş al…** vardır (ikonlar: sayfa,
SE'nin çarpısı, dışa ve içe ok).

## Doğrulama

- **Ortak durumlar:** `fixtures/commands/v1/file.import.sld.json`, 24 durum, `scripts/fixtures/sld_cases.py`'nin
  xml.etree'yle, KentOS kodu olmadan yazılmış okuyucusundan. Kaynaklar:
  - QGIS 4.2'nin yazdığı beş SLD: kategorili, aralıklı, kurallı, kesikli çizgi, döndürülmüş üçgen;
  - elle yazılmış SLD'ler: GeoServer'ın SLD 1.0'ı, metre birimi, kayıplı, desenli, süzgeçler, bozuk ve boş.

  Durumlar yol ve metin, plan, aynısı ikinci kez, geri alma ve bütün retleri de kapsar.
- **Başsız sunucuda gidiş-dönüş:** kategorili, aralıklı ve tek simgeli görünüş SLD'ye yazılır. Plan yazmaz. Veriye
  göre renk söylenir, çizgi katmanına alan simgeleyicisi yazılmaz. Geri okunanlar aynı türdedir; `file_exists` ve
  `layer_not_found` retleri sınanır.
- **QGIS 4.2:** KentOS'un SLD'lerini (`KENTOS_WRITE_SLD=1`) açar.
  - QGIS her SLD'yi kurallı görünüş olarak okur, kendi kategorili SLD'sini de. KentOS'unki QGIS'in kendisininkiyle
    aynı kuralları, süzgeçleri ve renkleri verir.
  - Tek simge çizgisi QGIS'te 2,857 piksellik `SimpleLine` olur.
- **Biçim çekirdeği:** süzgeçlerin iki yönlü çevrilmesi, kategorili görünüşün gidiş-dönüşü, söylenen kayıplar.
- **Resimler:** `sld-*` (Katmanlar menüsü).

## Sonuçlar

- KentOS'un görünüşü QGIS'e, GeoServer'a ve MapServer'a SLD ile gider, onlarınki KentOS'a gelir.
- Taşınamayan her parça adıyla söylenir. Görünüş sessizce varsayılana düşmez.
