# ADR 9343: Yeni proje sayfası adını taşır, iki yer ayrılır; görünüm ölçeği çizim ölçeğinden ayrılır; Seçime sığdır ve Katmana sığdır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  4 ve 8, P3, ve simge maddesi). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 9302](9302-cbs-new-project-simple.md) (CBS'de tek sayfalık yeni proje);
  - [ADR 9196](9196-storage-by-project-kind.md) (CBS verisi tablolardadır);
  - [ADR 0165](0165-project-types-cad-gis.md) §5 (görünüm ölçeği) ve
    [ADR 0626](0626-units-scale-and-true-size-texts.md) §3 (durum çubuğunda tek ölçek);
  - [ADR 9337](9337-unavailable-actions-panel-names-and-icons.md) (aç/kapa satırlarında simge).

## Bağlam

- **Madde 4:**
  - Yeni proje › CBS'nin sayfası “Ad ve konum” diye adlandırılıyordu; sayfada konum alanı yoktu, il ve koordinat
    sistemi vardı.
  - Not, projenin yerinin ilk Kaydet'te sorulacağını söylüyordu; proje dosyası ile veri tablolarının yerini
    ayırmıyordu.
  - GeoPackage alımında “Kaynağa bağla” ve “Projeye kopyala” sonuçları dosya adıyla söylenmiyordu.
- **Madde 8:**
  - Seçime yakınlaştır seçimi sığdırırken uzaklaştı (1:600'den 1:821'e).
  - Durum çubuğu görünüm ölçeğini (1:535), Denetçi çizim ölçeğini (1:1000) gösterdi; ikisi de “ölçek” diye anılıyordu.
  - Dağınık veride katmanı ekrana getirmek için şeritte komut yoktu (yalnız katmanın menüsünde Katmana yakınlaştır).
- **Simgeler:** Harita › Etiket ▾'nin Sabit etiketleri vurgula ve Yerleşmeyen etiketleri göster satırlarında simge
  yoktu.

## Araştırma

- **QGIS, durum çubuğu:** “Scale” görünümün ölçeğidir. Baskı ölçeği yerleşimin harita öğesindedir; ikisi ayrı adlarla
  durur ([Status Bar](https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_gui.html#status-bar)).
- **QGIS, gezinme:** “Zoom to Selection” ve “Zoom to Layer(s)” gezinme araç çubuğundadır; ikisi de nesneleri görünüme
  sığdırır, yakınlaştırma da uzaklaştırma da olabilir
  ([Zooming and Panning](https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html#zooming-and-panning)).
- **KentOS kararı:**
  - Sığdıran komutlar “sığdır” diye adlandırılır.
  - İki ölçek ayrı adlarla söylenir: Çizim ölçeği çıktınındır, Görünüm ölçeği ekranındır.

## Karar

### 1. Yeni proje (madde 4)

- **Sayfanın adı:** CBS sayfasının adı “Proje adı ve koordinat sistemi”dir.
- **İki yer:** sayfanın notu proje dosyası ile verinin yerini ayırır. Proje dosyası (yapı, stiller, ayarlar) ilk
  Kaydet'te sorulan yere yazılır. Katmanların verisi tablolarda durur: Veri ekle var olan bir GeoPackage'ı ya da
  veritabanını bağlar ya da içine alır, Yeni katman yeni bir tablo yapar.
- **GeoPackage alımı:** Veri katmanı seçeneğinin altındaki söz yolu dosya adlarıyla söyler:
  - “Kaynağa bağla: tablolar “qa.gpkg” dosyasında kalır; katmanlar onları okur, satırları düzenleme oturumuyla oraya
    yazılır.”
  - “Projeye kopyala: tablolar “qa.gpkg” dosyasından projenin veri yerine, “proje.gpkg” dosyasına tablo olarak yazılır;
    bu dosya bir daha okunmaz.”
  - Veri yeri henüz yoksa “(ilk Kaydet'te sorulan GeoPackage'a)” denir.

### 2. Ölçekler ve sığdırma (madde 8)

- **Görünüm ölçeği:**
  - CBS durum çubuğundaki ölçeğin ipucu “Görünüm ölçeği”dir: ekranın ölçeği, yakınlaştırınca değişir. Çıktının
    ölçeğinin ayrı olduğunu ve yerini söyler.
  - Seçim yokken Denetçi “Çizim ölçeği”nin hemen altında “Görünüm ölçeği”ni de gösterir.
- **Seçime sığdır:** `view.zoomSelection`'ın adı “Seçime sığdır”dır; İşlemler'in sonuç düğmesi de böyledir.
- **Katmana sığdır** (`view.zoomLayer`, yeni görünüm komutu):
  - Katmanlar'da seçili katmanın (seçili yoksa etkin katmanın) nesnelerini ya da veri katmanının tablosunun kutusunu
    sığdırır; seçim değişmez.
  - Boş katmanda kapalıdır; ipucu nedenini söyler.
  - Gezinme ▾'nin ilk satırıdır, CAD şeritlerinde ve CBS'de. Görünür sıraya konunca 1440 piksellik pencerede Etiket ve
    Koordinat sistemi panelleri katlanıyordu; bu yüzden ▾'dedir.
- **Önceki ve Sonraki görünüm:** sığdırmadan sonra merkezi ve ölçeği birebir geri getirir (sınandı; kural değişmedi).

### 3. Simgeler

Etiket ▾'nin aç/kapa satırları ADR 9337'den beri komutun simgesini onay işaretinin yanında taşır: Sabit etiketleri
vurgula (`labelsPinned`), Yerleşmeyen etiketleri göster (`labelsUnplaced`). Resimle doğrulandı.

### 4. Tümünü seç'in sözü (ADR 9341'in düzeltmesi)

Seçim süzgeci ya da Otomatik seçim nedeni zaten söylediyse Tümünü seç projedeki sayıyı ayrıca söylemez. Otomatik seçim
kapalıyken ADR 9341'in sözü `kentos-cad`'in `auto-select` izini bozuyordu.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Yeni proje sayfasının adı ve iki yer | yok: Mimari'nin sihirbazı kendi sayfalarıyladır | yok: aynı gerekçe | yok: aynı gerekçe | var |
| GeoPackage alımında yol sözü | yok: Mimari'de GeoPackage alımı yoktur | var: bağlı katman yolu kendi sözüyle kalır | var: aynı | var |
| Görünüm ölçeği (Denetçi) | var | var | var | var |
| Görünüm ölçeği ipucu (durum çubuğu) | yok: CAD'in durum çubuğu çizim ölçeğini gösterir | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Seçime sığdır adı, Katmana sığdır | var | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Görünüm komutlarıdır, çizimi değiştirmez; betiğin görünümü yoktur. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `scale_names_tests`:
    - Seçime sığdır ve Katmana sığdır adları;
    - Bina katmanı seçiliyken Katmana sığdır görünümü binaların ortasına getiriyor;
    - Önceki görünüm merkezi ve ölçeği birebir geri getiriyor, Sonraki sığdırılmış görünümü;
    - boş etkin katmanda komut kapalı ve nedeni;
    - Denetçi'de Görünüm ölçeği Çizim ölçeğinin altında, durum çubuğununkiyle aynı;
    - yerleri olmayan CBS projesinde alımın sözü “ilk Kaydet'te sorulan GeoPackage'a”.
  - `-- wizard properties processing::window navigation scale ribbon module_tests history::text catalog settings
    file_menu recall_tests selecting select`: hepsi geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır.
  - `kentos-project --lib wizard`: 6 geçti.
- **Takımlar:**
  - `cargo test -p kentos-cad` (izler) 37 geçti; Tümünü seç düzeltmesinden önce `auto-select` izi kırmızıydı.
  - `cargo test -p kentos-desktop-cbs` 63 geçti.
- **Kataloglar:**
  - `catalog/v1/additions.json`'a `view.zoomLayer`;
  - `python3 scripts/ui/ribbon.py --check` güncel;
  - `catalog/v1/modules.json` `KENTOS_WRITE_MODULES=1` ile yeniden yazıldı (dört modülde `view.zoomLayer`);
  - `python3 scripts/cad/manifest.py --check`: 1097 satır tutarlı.
- **Resim** (`.run/gui-evidence/kullanim-9343/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-gezinme-olcekler`: Gezinme, Denetçi'de Çizim ölçeği 1:1000 ve Görünüm ölçeği 1:535;
  - `02-etiket-menusu`: Etiket ▾'de Sabit etiketleri vurgula ve Yerleşmeyen etiketleri göster simgeleriyle;
  - `03-yeni-proje-sayfasi`: “Proje adı ve koordinat sistemi”, iki yeri ayıran not.
