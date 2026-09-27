# ADR 0094: Masaüstünde Sembol tasarımcısı

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/STYLE.md §3, §7; ADR 0090 (stilli çizim), 0091 (Katman stili), 0092 (Stil yöneticisi)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri masaüstüne en az bire bir gelir, olabildiğince iyileşerek;
  - iyileşmeler web'e de gelir;
  - görsellik en önemli işlerdendir.

## Bağlam

- **Web:** Sembol tasarımcısı (`ui/style/SymbolDesigner.ts`, `layerForms.ts`, `designerFields.ts`) bir sembolü katman yığını olarak düzenler:
  - solda katmanlar (işaret yerleştiren katmanın işaret katmanları altında girintili);
  - ortada örnek geometride canlı önizleme;
  - sağda seçili katmanın formu, “ƒ” ile veriye bağlanan değerler;
  - kendi geri alması, Kaydet ya da (katman stilindeki sembol için) Uygula.
- **Masaüstü:** Stil yöneticisinin Düzenle ve Yeni sembol'ü, Katman stili yuvasının Düzenle'si soluktu (ADR 0092).
- **Web penceresinin kusurları:**
  - iki katman art arda hızlı eklenince (bir saniye içinde) tek geri alma adımı oluyordu; adı yazarken ilk adım değişmiş adı kaydediyordu;
  - araç düğmeleri işe yaramadıklarında da basılabiliyordu (en üstte Yukarı, tek katmanda Sil); geri alma yalnız klavyedeydi;
  - boşaltılan sayı alanı değeri 0 yapıyordu (`Number("")`);
  - 1100 piksellik pencerede önizleme çubuğunun 1:1'i sağ sütunun altında kalıyordu;
  - “ƒ” kapatılınca fallback'i olmayan zorunlu renk “yok” oluyor, sembol kaydedilemiyordu;
  - dalga biçimi değişince ilk dalganın yeri siliniyordu;
  - boş yuvada Düzenle, yuvanın gösterdiği düz görünüşten değil varsayılan bir sembolden başlıyordu;
  - koşula bağlı katmanın kutusu tıklanınca koşul siliniyordu;
  - grup sayısının eki hep “'li”ydi (3'lü, 6'lı, 9'lu yerine).

## Karar

### Model: iki platformda aynı, `fixtures/style/v1/designer.json` ile

- **Web:** `ui/style/designerModel.ts` (DOM bilmez). **Masaüstü:** `kentos-native-style` `designer` (`mod.rs`, liste düzenlemeleri `list.rs`).
- Model şunları tutar:
  - katman türlerinin adları, her sembol türünün aldığı katmanlar, yeni katmanın başlangıcı;
  - listedeki tek satırlık özet;
  - formun yamasının katmana dönüşmesi (`offsetX`, `jitterPct`, `groupCount`, `haloWidth` … yardımcı anahtarlar; silinen alan);
  - liste düzenlemeleri: ekle (sembole ya da işarete), taşı, çoğalt, sil, aç/kapat, yeni kimlik;
  - yeni taslak, yuvanın başlangıç sembolü, başlık, kaydedilen ad ve kategori, önizleme ölçeği, pencerenin sözleri.
- Semboller JSON olarak düzenlenir; bu sürümün bilmediği alanlar korunur.
- Durum dosyasını web yazar (`apps/web/scripts/fixtures/record-designer.test.ts`, `GOLDEN_WRITE=1`). İki platform geçer:
  - web: `ui/style/designerFixture.test.ts`;
  - masaüstü: `crates/native/style/tests/designer.rs`.
- Masaüstü sayıları web'in yazdığı gibi yazar: tam sayı `3`'tür, `3.0` değil.

### Pencere: `apps/desktop/src/style/designer/`

- Web'in penceresi, aynı sözlerle ve boyda (1320 × 860, en çok pencerenin %92'si). Açtığı pencerenin üstünde durur; kapanınca ona döner.
- **Solda:**
  - Katmanlar ve Katman ekle menüsü (Sembole; seçili katman işaret yerleştiriyorsa “… işaretine”);
  - satırlar: görünürlük kutusu, tür, özet;
  - araçlar ve çizim sırasının notu.
- **Ortada:**
  - örnek geometri (alan, adalı alan; düz, kırık, alan kenarı; nokta);
  - −, ölçek, +, 1:1 (96 dpi).
  - Resmi çizimin stilli boru hatları çizer (`style/thumbs.rs`): resim haritanın çizdiğinin kendisidir. Her düzenleme bir resim kurar; tasarımcı yalnız son sekizini tutar.
- **Sağda:** seçili katmanın formu, her türün alanları ve Genel (birim, saydamlık, görünür).
  - Form parçaları `style/fields.rs`'tedir (SVG düzenleyicisi de kullanacak): etiketli satır, çift, sayı, metin, açılır liste (uzun listede arama), işaret kutusu, kesik, renk, “ƒ”.
  - Sayı alanlarının tablosu `designer/numbers.rs`'tedir: etiket, birim, adım, sınırlar, varsayılan, yazılan değer. Forma ve ↑ ↓'ye aynı tablo yol gösterir.
- **Renk alanı:** renk kutusu KentOS UI'ın renk seçicisini açar. Seçici saydamlık da seçtirir; altında sembolün kendi renkleri durur. Yanında hex ya da tema jetonu yazılır, ⋯ jetonları (ve “Yok”u) örnekleriyle listeler.
  - KentOS UI'ın `ColorPicker`'ına alanın yerine verilen öğe eklendi (`anchor`); başka kullanım değişmedi.
- **Altta:** Ad ve Kategori (ya da katman stilindeki sembolün notu), son söz, Vazgeç ve Kaydet ya da Uygula.
- **Açılış:**
  - Stil yöneticisinde Düzenle, kartta çift tık ve Yeni sembol (alan, çizgi, işaret; liste düzenlenebilir bir kategorideyse orada, değilse Sembollerim'de).
  - Sistem sembolü önce Kitaplığım'a kopyalanır (Sembollerim ve kendi kategorisi altında), kopya açılır.
  - Katman stili yuvasında Düzenle… ve Kopyasını burada düzenle….
- **Kayıt:**
  - Sembol denetlenir (`validate_symbol`), ilk sorun söylenir.
  - Kitaplıktaki sembol yerinde güncellenir, yeni sembol eklenir. Kitaplığım dosyasına, projenin sembolü çizime yazılır (ADR 0092). Stil yöneticisi sembolü yerinde gösterir.
  - Katman stilindeki sembol yuvaya geri verilir; kitaplığa yazılmaz.
- **Kapatma:** değişiklik varsa web'in sorusu sorulur (Kaydetmeden kapat, Vazgeç, Kaydet ve kapat; katman stilinde Uygula).
- **Klavye:** Ctrl+Z, Ctrl+Y ve Ctrl+Shift+Z tasarımcının geri almasıdır (çizimin değil). ↑ ↓:
  - sayı alanında adım adım değiştirir, Shift ile on adım;
  - alan yokken listede satır seçer.
- **Dosya al…:** SVG (temizlenerek), PNG ya da JPEG Kitaplığım'a eklenir ve alan onu alır.

### İyileşmeler

- **İki platformda:**
  - liste düzenlemeleri her zaman kendi geri alma adımıdır; ad ve kategori değişmeden önceki hâl kaydedilir;
  - listenin altında Geri al ve Yinele düğmeleri;
  - işe yaramayan araç sönüktür, Sil nedenini söyler;
  - boş sayı alanı değeri değiştirmez;
  - önizleme çubuğu dar ortada iki satıra geçer;
  - önizlemede tekerlek yakınlaşıp uzaklaşır;
  - listede ↑ ↓ satır seçer;
  - koşula bağlı katmanın kutusu yarım görünür, tıklanınca satırı seçer (koşul Görünür'de düzenlenir);
  - “ƒ” kapatılınca zorunlu renk mürekkep olur;
  - dalga biçimi değişince öbür ayarlar kalır;
  - boş yuvada Düzenle yuvanın gösterdiği düz görünüşten başlar;
  - grup sayısı Türkçe ekiyle yazılır;
  - işaretin katmanları değişirken işaretin öbür alanları korunur;
  - Yumuşatma kendi satırındadır, gölgenin iki sayısı altında (üçü bir satıra dar sütunda sığmıyordu).
- **Masaüstünde:**
  - Enter yazılan sayıyı değerin kendi yazımına döndürür ve geri alma adımını bitirir;
  - sınır dışı ya da okunamayan sayı alanı kırmızıdır;
  - çizim seçici aranır;
  - renk seçici saydamlık ve sembolün renklerini de verir.

## Sonuçlar

- **Denetimler:**
  - `designer.json` iki platformda geçer;
  - pencere kullanıcının sürdüğü gibi `designer/tests.rs`'te sınanır: yeni sembol ve kaydı, liste düzenlemeleri ve geri alma, yazılan alanlar ve ↑ ↓, sistem sembolünün kopyası, kapatma sorusu, geçersiz sembol, katman stiline Uygula, yuvanın başlangıcı, alınan çizim, ölçek, her türün formu, Ctrl+Z ve Ctrl+Y.
- **Resimler:** `cargo test -p kentos-desktop style::designer::screens -- --ignored --nocapture` çeker (`.run/shots/sdes-*.png`):
  - web'in resimlerinin durumları: her katman türü, menüler, işaretin katmanı, “ƒ”, kitaplık sembolü, katman stilinden, adalı alan, soru, dalga;
  - iki boy, iki tema ve büyük yazıyla bir boy daha.
  - Web'in aynı resimleri `node apps/web/scripts/e2e/shots.mjs symboldesigner` ile.
- **Bilinen farklar ve kalanlar:**
  - **Çizimi seçilmemiş görüntü dolgusu:** masaüstünün önizlemesi haritanın çizdiğini, eksik görüntünün çarpılı kutusunu döşer. Web'in Canvas2D önizlemesi hiçbir şey çizmez; web haritası da çarpılı kutuyu çizer.
  - **SVG düzenleyicisi:** Yeni çizim… ve Düzenle… onu bekler (sonraki dilim).
  - **Çizginin köşe birleşimi (`join`):** modelde ve çekirdekte var ama hiçbir çizici çizmiyor. Formda gösterilmedi.
