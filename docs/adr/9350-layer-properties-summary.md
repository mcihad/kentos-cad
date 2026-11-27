# ADR 9350: CBS katman menüsünün sonunda Katman özellikleri: katmanın bilgi amaçlı özeti

- **Durum:** kabul edildi (2026-11-27). Kaynak: sahibin sözü, 27 Kasım 2026: “Katman sağ tuşa da Katman Özellikleri
  elemanı ekle en alta cbs projesinde ve burada qgis gibi veritabanı bağı tipi projeksion stil alanlar formlar
  hepsinin özetini gösteren bir pencere açılsın, Qgis katman özellikleri gibi ama burası bilgi amaçlı ve çok daha
  estetik ve sade”.
- **İlgili:**
  - [ADR 9195](9195-module-separation.md) §5 (katman menüsü modül × katman türü);
  - [ADR 9101](9101-data-layers.md) (veri katmanı);
  - [ADR 9340](9340-data-layers-in-select-by-location-and-the-inspector.md) (kaynağın kendi sayımı);
  - [ADR 9349](9349-alanlar-list-and-settings.md) (Alanlar).

## Bağlam

- **Dağınık bilgi:** bir CBS katmanının bağlantısı, tablosu, sistemi, stili, alanları ve formları ayrı pencerelerde
  duruyordu. Bunlar Kaynaklar, Koordinat sistemi, Katman stili, Alanlar ve Formlar'dı.
- **Özet yok:** katmanın neyi okuduğunu tek bakışta söyleyen bir yer yoktu.

## Araştırma

- **QGIS, katman özellikleri:** pencerenin solunda sekmelerin listesi durur (Bilgi, Kaynak, Sembol, Alanlar, Öznitelik
  formu …). Bilgi sekmesi katmanın sağlayıcısını, kaynağını, geometri türünü, sistemini, kapsamını ve alanlarını
  salt okunur bir özet olarak gösterir
  ([Information Properties](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#information-properties)).
- **KentOS kararı:**
  - Katman özellikleri bir okuma penceresidir; değiştirmek katmanın menüsündeki kendi pencerelerinin işidir.
  - QGIS'in düzenini (solda bölümler) izler; her bölüm ad–değer çiftleriyle sade bir özettir.

## Karar

### 1. Menü

- **Yer:** katmanın sağ tık menüsünün son öğesi, Sil'den sonra ayrı bir bölümde: “Katman özellikleri…” (bilgi simgesi).
- **Yerleşim** (`LayerEntry::Properties`):
  - Modülde yalnız CBS'de.
  - Katman türünde çizim, açıklama, veri tablosu, bağlı, servis, raster ve nokta bulutunda. Grubun kendi kaynağı,
    stili ve alanları olmadığı için grupta yoktur.
  - Matris sınaması (`layer_menu_tests`) her hücreyi sınar.

### 2. Pencere

```text
Katman özellikleri
 ⓘ  bakim
    [Veri katmanı] [GeoPackage] [Salt okunur] [Düzenleniyor]
┌ bölümler ───────────┐│┌ Kaynak ───────────────────────────────────────┐
│ ⓘ Genel             ││ │ Bağlantı        GeoPackage dosyası            │
│ ▤ Kaynak            ││ │ Dosya           /…/Mahalle-Verileri.gpkg      │
│ ⊕ Koordinat sistemi ││ │ Tablo           bakim                         │
│ ◈ Stil              ││ │ Anahtar sütunu  fid                           │
│ ≣ Alanlar           ││ │ Geometri        POINT · sütun “geom”          │
│ ▭ Formlar           ││ │ Satır           6                             │
└─────────────────────┘│└───────────────────────────────────────────────┘
Bilgi içindir; değiştirmek için katmanın menüsündeki pencereleri kullanın.  Kapat
```

- **Baş:** katmanın adı ve çipleri: türü; veri katmanında GeoPackage ya da PostgreSQL; salt okunursa “Salt okunur”;
  düzenleniyorsa “Düzenleniyor”.
- **Bölümler** (solda, kitin `ListDetails`'i; sağda ad–değer çiftleri, kitin `Pairs`'i):
  - **Genel:** ad, grup yolu, türü, görünür, kilitli, etkin katman. Veri katmanında düzenleme ve kaynağa
    yazılmamış kayıt sayısı; çizim katmanında nesne sayısı.
  - **Kaynak:**
    - Veri katmanında bağlantı, dosya ya da veritabanı adresi (parola yazılmaz), tablo, anahtar sütunu, geometri
      türü ve sütunu, kaynağın kendi saydığı satır (sayılırken “sayılıyor…”, sayılamazsa nedeni), yazma biçimi,
      projeye göre yolu.
    - Serviste servis türü, adresi ve servis katmanları.
    - Bağlı katmanda tablosu.
    - Öbür katmanlarda “Proje dosyası”.
  - **Koordinat sistemi:** projenin ve tablonun sistemi. Aynıysa satırlar olduğu gibi çizilir; değilse çizilirken
    projenin sistemine dönüştürülür, kaynaktaki koordinatlar değişmez; sistem yazılı değilse projenin sisteminde
    sayılır.
  - **Stil:** çizim biçimi (Tek sembol, Kategorilere göre, Sınıflara göre, Kurallara göre …), renk ve dolgu örnekleri,
    etiketler açık mı, katman süzgeci.
  - **Alanlar:** her alan adı ve takma adıyla, türü (metinde uzunluğu) ve kuralları (zorunlu, tekil, değerli liste,
    aralık).
  - **Formlar:** katmanın formları; yayının sürümü, taslak ya da yayından kaldırılmış. Form yoksa kayıtların
    varsayılan formla açıldığı söylenir (ADR 9346).
- **Yazmaz:** pencere hiçbir şey yazmaz; satır sayısı kaynağın kendi sayımıdır (ADR 9340), satırlar okunmaz.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Katman özellikleri | yok: sahibin sözü CBS projesi içindir; CAD katmanının kaynağı proje dosyasıdır | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Bilgi penceresidir; betik aynı bilgiyi `cad.layers.list`, `cad.layers.rows` (`count`) ve `cad.forms.list` ile okur. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Kapsam:** tablonun kapsamı (kutu) ve satırların son okunma anı gösterilmiyor.
- **Bölümden pencereye:** bölümlerden ilgili düzenleme penceresine geçiş yok; pencere bilgi içindir.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `layer_properties_tests::the_menus_last_entry_opens_the_layers_summary` geçti:
  - pencere Genel'de açılıyor;
  - Kaynak'ta `bakim`'in 6 satırı kaynağın sayımıyla geliyor;
  - Kapat pencereyi ve durumu kaldırıyor.
- **Programlar:** `cargo test -p kentos-cad --test programs layer_menu`: 3 geçti.
  - Matris `LayerEntry::Properties`'i yalnız CBS'de ve “davbsrn” türlerinde buluyor.
  - Bütün `kentos-cad` sınamaları 37 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9350/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-kaynak`: GeoPackage dosyası, tablo, anahtar, geometri, 6 satır;
  - `02-alanlar`: kod, bedel, mahalle (Mahalle; zorunlu · 2 değerli liste);
  - `03-koordinat-sistemi`: proje TM36, tablo TM33, dönüşüm sözü;
  - `kullanim-9301/menu-cbs-veri-tablosu`: menünün sonunda “Katman özellikleri…”.
