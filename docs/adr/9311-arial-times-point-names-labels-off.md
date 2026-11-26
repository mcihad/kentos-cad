# ADR 9311: Çizim yazı tipi Arial ve Times New Roman; nokta adları yazı, etiketler kapalı başlar

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “varsayılan font olarak Arial veya Times New Roman olması gerekiyor”;
  - “Hayır arial olacak, arial kullanacaksın times new roman” (ikame yazı tipi değil);
  - noktalara ad verildiğinde kenar yazıları gibi köşeye Metin nesnesi; “etiket motoru ile olmaz bu”;
  - “Etiketleri varsayılan olarak aktif etme”: kişi katman özelliklerinden açarsa açılır;
  - varsayılan yazı tipi ayarlardan, tasarlanan her şeyde de değiştirilebilir.
- **İlgili:** [ADR 0055](0055-desktop-drawing-text.md) (çizim yazı tipleri), [ADR 0183](0183-text-and-dimension-styles.md)
  (yazı stili), [ADR 0212](0212-label-engine.md) (etiket motoru), [ADR 0324](0324-paragraph-layout-and-fonts.md) (DXF'te yazı tipi),
  [ADR 0914](0914-cad-text-layout-and-live-field-delivery.md) (gömülü yüzler), [ADR 9310](9310-processing-tools-by-module.md),
  [ADR 9312](9312-install-layout.md) (`fonts` klasörü).
- **Web:** iptal edildi. **`.kcad`:** şema değişmedi (§1). **Bağımlılık:** yok; Liberation Serif 2.1.5 (OFL 1.1) eklendi.

## Bağlam

- Çizimin yazı tipi öntanımı Barlow'du. Yedi yazı tipinin hepsi web'in açık lisanslı yüzleriydi; Arial ve Times New
  Roman yoktu. Arial, metrik ikizi Arimo olarak çiziliyordu (DXF `arial.ttf` → Arimo).
- Etiket motoru, katmanda etiket ayarı yokken bile etiket metni olan her nesneyi türünün öntanımlı stiliyle
  etiketliyordu (`layer_texts`'in “kinds' defaults”'ı). CBS veri tabloları bu kuralın dışındaydı: stil yoksa
  etiketsizdi (`data_view::labelled`).
- Köşe noktalarını numarala'nın öntanımı “Nokta”ydı: noktanın adı etiket motoruyla görünüyordu. Hazır kadastro ağacının
  ada, parsel, kot, poligon ve pafta katmanlarında etiket açıktı.

### Araştırma (26 Kasım 2026'da okundu)

- **Lisans:**
  - Microsoft'un yazı tipi SSS'si: belgeye gömme hakları dışında Windows yazı tipleri yeniden dağıtılamaz, başka
    bilgisayara ya da sunucuya kopyalanamaz, değiştirilemez ([Font FAQ](https://learn.microsoft.com/typography/fonts/font-faq),
    [Arial font family](https://learn.microsoft.com/en-us/typography/font-list/arial)).
  - Bu Mac'teki dosyaların kendi lisans alanı: Arial ve Times New Roman (sürüm 5.01) “ürünün EULA'sının izin verdiği
    gibi kullanılabilir” der; `fsType` 8'dir (düzenlenebilir gömme: belgeye alt küme olarak gömülebilir).
  - Sonuç: Arial ve Times New Roman kullanılır ama depoya ve kurulum paketine konamaz. Pakete koymak Monotype'tan
    dağıtım lisansı ister.
- **Metrik ikizler:**
  - Liberation Sans, Serif ve Mono, Arial, Times New Roman ve Courier New'in metriklerine göre çizilmiştir
    ([Red Hat](https://www.redhat.com/en/about/blog/liberation-fonts), [Liberation fonts](https://en.wikipedia.org/wiki/Liberation_fonts)).
  - Liberation 2.x SIL OFL 1.1'dir; dosyanın kendi alanı da bunu söyler (sürüm 2.1.5).
- **Ölçüldü (bu makinede, fontTools):** 32–383 arasındaki 319 ortak karakterin her birinde Arimo'nun genişliği
  Arial'ınkiyle, Liberation Serif'inki Times New Roman'ınkiyle aynıdır. Yazının yerleşimi ikizle ve gerçek yazı
  tipiyle birebir aynı olur.

## Karar

### 1. İki yazı tipi: Arial ve Times New Roman

- **Sözleşme:** `DrawingFont::Arial` (`arial`) ve `DrawingFont::TimesNewRoman` (`times-new-roman`) eklendi.
  - Proje dosyası yazı tipini `DrawingFont::ALL`'daki sırasıyla saklar. İkisi sona eklendi (7 ve 8), var olan bütün
    dosyalar aynı okunur.
  - Şema numarası yükselmedi: dağıtılmış bir eski sürüm yoktur. İlk dağıtımdan sonra yeni bir değer şema ister.
- **Seçim sırası:** pencereler `DrawingFont::CHOICES` sırasıyla gösterir; Arial ile Times New Roman başta.
  - Proje ayarları › Çizim yazı tipi; Uygulama ayarları › Yeni projelerin çizim yazı tipi;
  - yazı ve ölçü stillerinin yazı tipi; nesnenin yazı stili.
- **Yeni proje Arial'dır:**
  - `newProjects.drawingFont` öntanımı `arial`; `DrawingFont::NEW_PROJECT`;
  - Python ve MCP'nin yeni proje isteği yazı tipi vermezse Arial;
  - kullanıcı öntanımı ayarlardan değiştirir; proje kendi yazı tipini saklar.
- **Eski dosyalar:** yazı tipi yazılmamış eski dosya Barlow'la açılır (`id_of(None)`), örnek çizimler değişmez.

### 2. Gerçek Arial ve Times New Roman çizer

- **Açılışta masaüstü** (`launch::program_main` → `drawing_fonts::give_system_faces`) iki yazı tipinin dosyalarını
  sırayla arar:
  1. kurulumun `fonts` klasörü (lisanslı kopya buraya konur, ADR 9312);
  2. kullanıcının `kentos-cad/fonts` klasörü;
  3. işletim sisteminin yazı tipi klasörleri: macOS `/System/Library/Fonts/Supplemental`, Windows `%WINDIR%\Fonts`,
     Linux `msttcorefonts`.
- **Doğrulama:** dosya, ailesinin adı (Arial, Times New Roman) yazıyorsa alınır.
- **Kullanım:** bulunan yüzler geometri çekirdeğine verilir (`font_data::give`). Şunlar gerçek yazı tipiyle çizer:
  - çizimin yazısı (GPU glifleri);
  - etiketler ve Iced metni;
  - PDF (kullandığı harflerin alt kümesi gömülür; `fsType` 8 izin verir).
- **İkiz:** bulunmayan anahtar, gömülü ikiziyle çizer. Arial'ınki Arimo, Times New Roman'ınki Liberation Serif'tir (400,
  700, italik 400 ve 700). Testler ve başsız sunucu `give` çağırmaz; her makinede aynı resmi çizerler.
- **Dosyayı bütün yazanlar her zaman ikizle yazar:** SVG'nin gömülü yüzleri, raster önizleme, pafta paketinin
  `kaynaklar/fontlar`'ı ve kaynak kaydı. Böylece KentOS Arial'ın dosyasını hiçbir yere kopyalamaz.

### 3. Ölçü tabloları ikizlerden

- **Geometri çekirdeği** (`text/metrics_more.rs`, `scripts/fonts/drawing_metrics.py --check`):
  - Arial'ın satırları Arimo'nun kayıtlı satırlarıyla aynıdır;
  - Times New Roman'ınki Liberation Serif'in kendi genişliklerindendir (500 → 400, 600 → 700, tarayıcının seçtiği
    gibi).
- **Pafta çekirdeği** (`data/font-metrics.json`, `scripts/fonts/sheet_metrics.py`): sekiz yüz eklendi.
- **Yüz numaraları:** eklenen yüzler yedek yüzlerden sonra numaralanır (`ADDED_FACES`). Var olan yüzlerin numarası
  değişmez.

### 4. DXF ve dosya adları

- **Yazma:** Arial `arial*.ttf`, Times New Roman `times*.ttf` (`timesbd`, `timesi`, `timesbi`). Arimo da `arial.ttf`
  olarak yazılır: başka program onu temsil ettiği yüzle çizer.
- **Okuma:** `arial*.ttf` ve `times*.ttf` artık kendi yazı tiplerine gider. Önce Arimo'ya eşdeğer sayılırdı; Times
  eksikti.
  - Helvetica ve Liberation Sans Arial'ın eşdeğeridir;
  - Liberation Serif ve Tinos Times New Roman'ınkidir.

### 5. Etiketler kapalı başlar

- **Kural:** `LayerStyle::labels_on`. Katmanın etiketlemesi açıksa (tek etiket ya da kurallı) ya da etiketleme ayarı
  olmadan etiket stili varsa etiketler. İkisi de yoksa etiketlemez.
  - CBS veri tablolarının kuralı zaten buydu; çizim katmanları da artık ona uyar.
  - Aynı kural: `layer_texts` (sahne), çekirdeğin etiket motoru (`read_labelling`), Etiketleri yazıya çevir, etiket
    adayları, katman menüsü (“Etiketler… (etiketsiz)”).
- **Etiketler penceresi** etiketsiz katmanda Yok'ta açılır. Tek etiket seçilince açılır.
- **Hazır ağaç ve yeni katmanlar:** standart kadastro ağacı (ada, parsel, ana eş yükselti, kot, poligon, pafta) ve
  aracın etiket stilli yeni katmanı (`NewLayerStyle::over`), koordinat listesinin nokta katmanı:
  - etiket stillerini saklar, etiketleri kapalı başlar (`labels_off`);
  - kullanıcı Etiketler'den açınca tasarlanmış stil gelir.

### 6. Nokta adları yazıdır

- **Köşe noktalarını numarala:** öntanımı “Nokta ve yazı”dır. Adlı nokta nesnesi ve köşenin dışına, açıortayına
  oturan Metin nesnesi yazar. Yazı, Kenar uzunluklarını yaz'ın yazısı gibidir: projenin kenar ve köşe yazıları
  yüksekliği, çizimin yazı tipi.
- **Parsel ölçü yazıları modeli** de köşe adlarını yazı olarak yazar: adımı öntanımı kullanır.
- **Seçenekler:** “Nokta” (yalnız adlı nokta; ad, etiket açılınca görünür) ve “Yazı” yerindedir.

### 7. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | var | Yazı tipi, kapalı başlayan etiketler, köşe adları yazı (araç Harita'nın, ADR 9310) |
| Yol | var | Aynı; köşe numaralandırma Yol'da da var (kamulaştırma parseli) |
| Mimari | var | Yazı tipi ve etiket kuralı ortak çekirdektir; köşe numaralandırma Mimari'de yok |
| CBS | var | Yazı tipi ve etiket kuralı (veri tablosunda değişiklik yok); köşe numaralandırma CBS'de yok |

### 8. Kanallar

| Kanal | Var / yok | Nasıl |
|---|---|---|
| Python | var | Yeni proje isteği yazı tipini açıkça alır (`Document.new`'un öntanımı hâlâ `barlow`; ADR 9313 §Açık kalanlar); `cad.processing.run` köşe adlarını yazıyla yazar; etiket kuralı belgenindir. Başsız Python ikizle ölçer, aynı genişlikler. |
| MCP | var | Aynı komutlar; başsız. |
| Yapay zekâ ajanı | var | MCP ile aynı. |
| Eklenti | var | Aynı komutlar. Eklentinin yeni katman stili etiket verirse etiket kapalı başlar. |

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/core/{document,annotation,layer,settings/schema}.rs`.
- **Geometri çekirdeği:** `text/{font_data,shaping,fonts,mod,metrics_more}.rs`, `labels/style.rs`.
- **Pafta:** `crates/shared/sheet/src/{text,svg}.rs`, `data/font-metrics.json`; `crates/sheet-ui/src/fonts.rs`.
- **Çizim:** `crates/native/plot/src/{fonts,images,preview,provenance}.rs`, `crates/render/wgpu/src/scene.rs`.
- **Dosya:** `crates/shared/formats/src/dxf/writer/styles.rs`, `crates/shared/kcad/src/{decode,encode/names}.rs`.
- **Etiketler:** `crates/native/scene-query/src/label_texts.rs`, `application/src/core/annotation_labels.rs`,
  `interaction/src/core/labels_to_text.rs`, `apps/desktop/src/gis/labelling/{mod,candidates}.rs`,
  `apps/desktop/src/cad/layering/submenus.rs`.
- **Araç:** `crates/native/processing/src/builtin/vertex_numbering.rs`, `types.rs` (`NewLayerStyle::over`);
  `crates/native/project/src/new_project.rs`.
- **Masaüstü:** `apps/desktop/src/cad/drawing_fonts.rs`, `launch.rs`, `cad/labels/gpu.rs`,
  `cad/annotation_styles.rs`, `shell/project/settings.rs`, `shell/project/wizard/mod.rs`,
  `shell/exchange/coord_import.rs`.
- **Yazı tipi dosyaları:** `apps/desktop/assets/fonts/drawing/LiberationSerif-*.ttf` ve `LiberationSerif-OFL.txt`.
  Bu Mac'teki LibreOffice'in kendi kopyasından alındı, indirilmedi, değiştirilmedi.
- **Üreticiler:** `scripts/fonts/drawing_metrics.py`, `scripts/fonts/sheet_metrics.py`.
- **Ortak durumlar:** `fixtures/processing/v1/cases.json` güncellendi:
  - varsayılan çıktı `both`;
  - nokta yolunu sınayan iki durum açıkça `points` der;
  - yeni katmanların stilinde etiket kapalı.

  Komut durumları üreticilerden yeniden yazıldı. `dialog.json`'un seçenek ipuçları güncellendi.

## Doğrulama (26 Kasım 2026)

- **Geometri çekirdeği:** 372 bütünleşik sınama geçti. Yazı tipi bildirgesinin dondurulmuş 29 yüzü değişmedi; eklenen 8
  yüz sona yazıldı. Ad çözümleme (`fonts::resolve`) sınamaları yeni kurala göre güncellendi.
- **Pafta çekirdeği:** sınamalar ve glif örnekleri geçti. Hiçbir yüzde olmayan karakter örneği `≤` yerine `⟨` oldu;
  `≤` artık Times New Roman'dan çizilir.
- **DXF:** biçimler sınamaları geçti. Bağımsız Python başvurusu `dxf_write_reference.py --check` 11 çıktının
  hepsini doğruladı.
- **kcad:** sınamalar geçti. Bağımsız başvuru `kcad_v2_reference.py --check` 1893 dosyada tutarlı; bozuk örnek
  bilinmeyen yazı tipi olarak artık `helvetica` adlandırır.
- **İşleme ve komutlar:** ortak durumlar, komut durumları (`cad.processing.*`, `cad.fonts.report`) geçti.
- **Masaüstü:**
  - çekirdeğin bütün sınamaları koşuldu (2 496 geçti);
  - etiket penceresi, veri etiketleri, nokta görünümü ve kullanım izleri (Etiketleri yazıya çevir) yeni kurala göre
    katmanın etiketini açarak sınanır;
  - Harita paketi 195, programlar 37 sınama geçti.
- **Resimler:** `.run/gui-evidence/kullanim-9310/kose-numarala-pencere.png`, `kose-adlari-yazi.png`. Köşe adları
  köşelerin dışında, gerçek Arial'la (bu Mac'in kendi dosyası); katmanda 12 nesne (6 nokta, 6 yazı), etiket kapalı.
- **Koşulmayanlar:** Windows'ta ve Linux'ta sistem yazı tipi araması (yalnız macOS'ta denendi); gerçek pencerede fare
  ve klavyeyle kabul.

## Açık kalanlar

- **İçe almada adlar:** koordinat listesinden ve GNSS'ten alınan noktaların adları için Metin nesnesi yazılmadı. Bu iş
  ortak içe alma komutunda yapılmalıdır; nokta katmanının etiketi şimdi kapalı başlar.
- **Kurulu olmayan makine:** Linux'ta ve başsız sunucuda ikizle çizilir. Gerçek dosya kurum lisansıyla `fonts`'a konur.
