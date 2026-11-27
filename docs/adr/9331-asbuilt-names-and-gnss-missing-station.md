# ADR 9331: Uygulama kontrolünde ad alanı katmandan seçilir, satır noktaya götürür; GNSS ağında eksik istasyon satırıyla söylenir

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  14, P3). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 1700](1700-asbuilt-check.md) (Uygulama kontrolü), [ADR 2420](2420-field-record-checked-with-its-package.md)
  (saha paketi), [ADR 1380](1380-gnss-baseline-network-adjustment.md) (GNSS baz ağı).

## Bağlam

- **Uygulama kontrolü:**
  - Ölçülen `qa_noktalar`, tasarım çizgileri `qa_yollar`, yatay tolerans 0,01 m ile üç satırın farkı 0,000 m ve durumu
    İçinde doğru çıktı.
  - Sonuç tablosunun Nokta sütunu boştu. Formda serbest metin “Ad” vardı, katmanın sütunu `ad` idi; denetim adı “Ad”
    özniteliğinde aradı, bulamadı.
- **GNSS ağı:** `ag.txt` örneğinde tanımsız N1 için hata verdi.
  - Dosyada N1 baz uçlarında vardı, istasyon kaydı yoktu; ret doğruydu.
  - Ama ileti yalnız ““N1” istasyonu verilmedi.” diyordu: hangi satır, neyin eklenmesi gerektiği söylenmiyordu.
- **İncelemenin kabulü:**
  - Alan isim yazdırılmadan katman şemasından seçilsin; bulunamazsa satır kimliği gösterilip nedeni açıklansın.
  - Satırla harita konumu arasında Yakınlaştır bağlantısı olsun; CSV aynı kimliği yazsın.
  - GNSS dosya hatasında satır numarası, eksik istasyon ve doldurulabilir örnek gösterilsin.

## Karar

### 1. Ad alanı katmanın alanlarından seçilir

- Uygulama kontrolünün “Ad alanı” bir seçicidir. Seçenekleri ölçülen katmanın alanlarıdır: katmanın kendi alanları,
  sonra noktalarının taşıdığı öznitelikler, her biri bir kez.
- **Tahmin:** pencere açılınca ve ölçülen katman değişince alan tahmin edilir:
  - büyük-küçük harf ve Türkçe işaretlerden bağımsız `Ad` olan;
  - yoksa adında nokta, name, isim, adı geçen;
  - yoksa hiçbiri (“—”, denetimin kendi `Ad`'ı).
- **Katmanda olmayan alan:** seçicide “(katmanda yok)” diye görünür, başka bir alanın adıyla görünmez.
- **Komut:** girdi aynıdır. Denetimin kendi alanı (`Ad`) gönderilmez, betiğin bıraktığı gibi.

### 2. Adı olmayan satır

- **Tablo ve CSV:** ad alanı boş olan nokta nesne kimliğinin ilk sekiz karakteriyle gösterilir (`#1a2b3c4d`). CSV'nin
  Nokta sütunu aynısını, Nesne sütunu kimliğin tamamını yazar.
- **Özet:** “N noktanın “Ad” alanı boş: satırları nesne kimliğiyle (#…) gösterildi, eşleme adsız yapılır. Ad alanını
  katmanın alanlarından seçin.”
- **Ölçülmeyen tasarım noktaları:** adı olmayan aynı biçimde kimliğiyle sayılır.

### 3. Satır haritaya götürür

- Sonuç tablosunda bir satıra basınca harita ölçülen noktaya gider. Bu, görünüm geçmişinin bir adımıdır; satır işaretli
  kalır.
- Özetin altında “Bir satıra basınca harita o noktaya gider.” yazar.
- Nokta çizimde yoksa (sonuç eskimişse) bu söylenir.

### 4. GNSS ağında eksik istasyon

- Metinle verilen ağda bir bazın ucu için K ya da N satırı yoksa dengeleme yapılmaz. İleti satırı, ucu ve eklenecek
  satırı söyler: “6. satırdaki bazın bitişi “C”: dosyada onun K ya da N satırı yok. Yeni bir istasyonsa “N;C”, bilinen
  bir noktaysa “K;C;X;Y;Z” satırını ekleyin.” (kod `unknown_station`, yol `text`).
- Listeyle verilen ağda ileti çekirdeğinki gibi kalır (yol `baselines[i].to`).
- **Arşiv:** GNSS arşivinin baz metni içe alması K/N satırı istemez; bu denetim yalnız dengelemededir.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Uygulama kontrolünün ad alanı, adsız satır, satırdan haritaya | yok: Mimari'de Uygulama kontrolü yoktur | var | var | var |
| GNSS ağında eksik istasyonun satırıyla söylenmesi | yok: GNSS ağı yoktur | var | yok: Yol'da GNSS ağı yoktur | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.survey.baselines` metinle aynı reddi verir (ortak durum “Metinde baz ucunun istasyon satırı yok”); `cad.asbuilt.check`'in `field`'ı aynıdır. Alan seçicisi ve satır pencerenin sunumudur. |
| MCP | var | Aynı komutlar. |
| Yapay zekâ ajanı | var | Aynı komutlar. |
| Eklenti | var | Aynı komutlar. |

## Açık

- **Ad alanı tasarımda:** denetim aynı ad alanını ölçülen ve tasarım noktalarında arar. İki katmanın ad alanı farklıysa
  adla eşleme yapılmaz, arama yarıçapı kullanılır. Ayrı alan çekirdeğin girdisini genişletir.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Ortak durum:** `fixtures/commands/v1/cad.survey.baselines.json`'a 32. durum eklendi.
  - Üretecin hesaplı durumları numpy, scipy ve pyproj ister; bunlar bu makinede kurulu değil. Ret hesap içermediği için
    durum üretecin kendi `refused` biçimiyle ve dosyanın kendi JSON biçimiyle yazıldı; önce dosyanın o biçimde olduğu
    denetlendi. Üreteç de aynı durumu yazar.
  - Rust: `KENTOS_CASES_ONLY=cad.survey.baselines … fixtures::every_command_case` geçti.
  - Python: yerel modül yeniden derlenip `test_command_cases.py` geçti.
- **Harita paketi:** `cargo test -p kentos-desktop-map -- asbuilt baselines` ve resimleri: 22 geçti.
  - `the_name_field_is_the_layers_and_a_row_goes_to_its_point`:
    - `ad` alanlı katmanda alan tahmin ediliyor, her satır adlı;
    - katmanda olmayan alanla satırlar `#…`, CSV'nin ilk sütunu aynı kimlik;
    - satıra basınca harita o noktanın üstünde, satır işaretli.
  - `a_station_without_its_line_is_said_with_the_line_to_add`: C'nin N satırı çıkarılmış örnekte ileti satırı, “N;C”
    ve “K;C;X;Y;Z”yi söylüyor.
  - `the_review_kept_from_the_window_is_the_case_s`: öntanımlı alan gönderilmediği için girdi ortak durumunkiyle aynı.
- **Resim** (`.run/gui-evidence/kullanim-9331/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-ad-alani-ve-satir`: Ad alanı `ad`, işaretli satır;
  - `02-adsiz-satirlar`: Ad alanı “—”, uyarı ve “Ölçülmeyen: P5, P6, P7.”;
  - `03-gnss-eksik-istasyon`: satır, uç ve eklenecek satır.
