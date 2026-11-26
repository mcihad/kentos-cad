# ADR 9322: Form alanının seçenek kaynağı adlarıyla sunulur; değer listesi tasarımı kesmeden yapılır ve bağlanır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  3, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0430](0430-project-value-lists.md) (değer listeleri), [ADR 0404](0404-form-lookups-and-related-records.md) §1 (başka katmandan
  liste), [ADR 0438](0438-record-rules-and-field-extras.md), [ADR 0439](0439-sources-references-and-migration.md).

## Bağlam

- **Kusur:** form taslağına alan eklendi. Taslak kaydedilmeden Forma ekle › Liste › Değer listeleri… ile `Saha Durumu`
  listesi yapıldı (`A / Aktif`, `B / Bakımda`).
  - Liste yöneticisi tasarımcının yerine geçti; kaydedince ana ekrana dönüldü. Tasarımcı yeniden açıldığında yeni form
    öğesi ancak Geri yükle ile döndü.
  - Kod girilince alttaki uyarı görünüp kayboluyor, pencere dikeyde yaklaşık 30 piksel oynuyordu.
  - Form öğesinin Değer listesi menüsü yalnız “Alanın kendi değerleri”ni ve katman adlarını gösteriyordu. Yeni liste
    orada seçilemiyordu; çalışan yol Veri › Alanlar › durum › Liste… › Projenin listesi'ydi.
- **Kod:**
  - `OpenDomains` tasarımcının durumunu bırakıp bağımsız liste yöneticisini açıyordu.
  - Form öğesinin özellikleri ile liste yöneticisi farklı bağlama kavramları sunuyordu.

## Araştırma

İncelemenin verdiği kaynaklar:

- **QGIS:** formun alan düzenleyicileri sabit değer eşlemesini (değer haritası) ve başka katmandan değer ilişkisini ayrı
  yapılandırmalar olarak sunar
  ([vektör formu düzenleyicileri](https://docs.qgis.org/4.4/en/docs/user_manual/working_with_vector/vector_properties.html#edit-widgets)).
- **ArcGIS Pro:** aynı alan kısıtı (domain) birden çok alana ve alt türe bağlanır; kısıtlar veri tabanında yönetilir
  ([domain yönetimi](https://pro.arcgis.com/en/pro-app/3.6/help/data/geodatabases/overview/create-modify-and-delete-domains.htm)).
- **KentOS kararı:** listenin üç kaynağı tek denetimde adıyla sunulur: katman alanının listesi, proje listesi, başka
  katman. Proje listesi KentOS'ta katman alanına bağlanır (ADR 0430). Tasarımcı da bunu Alanlar'ın kullandığı
  `cad.domains.edit` `link` işlemiyle yapar.

## Karar

### 1. Seçenek kaynağı

Seçili form alanının “Değer listesi” denetimi “Seçenek kaynağı” olur. Seçenekler adlarıyla sunulur:

- **Katman alanının listesi:** neyse o söylenir: “· Saha Durumu (proje listesi)”, “· kendi 3 kodu” ya da “· liste yok”.
- **Proje listesi: <ad> (<n> kod | <en az> – <en çok>):** projenin, alanın türündeki her listesi. Adın yanındaki kod
  sayısı ya da aralık listeleri ayırır; iki listenin aynı adı taşıması zaten reddedilir.
  - Seçilince katman alanı listeye bağlanır (`cad.domains.edit` `link`, tek geri alma adımı) ve taslaktaki başka katman
    listesi bırakılır.
- **Başka katmandan: <katman>:** ADR 0404'ün listesi; anahtar (Saklanan) ve görünen alan (Gösterilen) sorulur.
- **Altında listenin özeti:** alanın türü, etkin kodlar, kullanımdan kalkmış kodlar (eski kayıtta okunur, seçilmez) ve
  alanın boş bırakılıp bırakılamayacağı. Örnek: “Metin · 2 etkin kod (A Aktif, B Bakımda) · boş bırakılabilir”.

### 2. Liste oluştur ve bağla

- Değer listeleri penceresi, seçili alanın adıyla ve türüyle yeni bir listeyle açılır. “Kullanan alanlar” satırı
  “<katman> › <alan> (Kaydet'te bağlanır)” der.
- **Kaydet:** listeyi yapar ve alana bağlar; ikisi tek geri alma adımıdır.

### 3. Tasarıma dönüş

- Değer listeleri Formlar'dan açılınca (Forma ekle › Liste › Değer listeleri…, Liste oluştur ve bağla) Kaydet de Vazgeç
  de tasarımcıya döner.
  - Kaydedilmemiş taslak, seçili öğe ve sekme durumda kaldığı için aynıdır.
  - Özellikler bölmesi kaydırma konumuna döner: konumu izlenir ve dönüşte geri verilir.
- Şeritten açılan Değer listeleri eskisi gibi kapanır.

### 4. Pencere oynamaz, eksik hücre işaretlenir

- Değer listeleri penceresinin uyarı satırı, söyleyecek bir şey olmasa da yerini tutar; kod yazılırken pencere oynamaz.
- Seçili listede kodu ya da etiketi boş ilk satırın boş hücresi kırmızı çerçeveyle işaretlenir.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Seçenek kaynağı, Liste oluştur ve bağla, tasarıma dönüş | yok: Mimari formsuzdur | var: çizim katmanının formu | yok: Yol formsuzdur | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Bağlama `cad.domains.edit` `link`, liste `create` ile yapılır; masaüstüyle aynı komut ve aynı denetim (tür uyumu, `link_problem`). |
| MCP | var | Aynı komut. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Açık

- Başka katmandan alınan listede anahtar ve görünen alan ADR 0404'teki gibi sorulur; örnek kayıtla önizleme ve yayın
  öncesi özet incelemenin 6. maddesinin işidir.
- Gerçek pencerede fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- form domain layer_fields traces module_tests ribbon_tests catalog
  history::text`: 296 geçti; tek kırmızı önceden var olan bağımlılık tabanı (`shell/approved_layer.rs`,
  `shell/gpu_frame.rs`, `gis/data_selection.rs`).
- **`list_source_tests` (4 sınama):**
  - yalnız alanın türündeki proje listesi sunuluyor; seçilince katman alanı bağlanıyor, kodları listeninki oluyor,
    taslak ve seçim kalıyor, tek geri almayla çözülüyor;
  - Liste oluştur ve bağla alanın adıyla liste yapıyor; Kaydet bağlıyor ve tasarımcıya aynı öğe, kaydedilmemiş taslak
    ve kaydırma konumuyla dönüyor; Vazgeç hiçbir şey yapmadan dönüyor;
  - Forma ekle › Liste › Değer listeleri… Vazgeç'te taslağı ve seçimi koruyarak dönüyor; şeritten açılan eskisi gibi
    kapanıyor;
  - aynı adlı ikinci liste reddediliyor, hiçbir şey yazılmıyor.
- **Resim** (`.run/gui-evidence/kullanim-9322/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-secenek-kaynagi`: “Katman alanının listesi · Saha Durumu (proje listesi)”, özet ve Liste oluştur ve bağla;
  - `02-liste-olustur-ve-bagla`: alanın adıyla yeni liste, boş etiket kırmızı, “Saha › durum (Kaydet'te bağlanır)”.
