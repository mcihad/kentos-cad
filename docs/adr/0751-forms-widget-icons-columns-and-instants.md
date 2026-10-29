# ADR 0751: Formda bileşenin ikonu, geniş pencerede iki sütun ve saat dilimli tarihin seçicisi

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü:
  > formlarımız çok zayıf; çok büyük formlarda çok küçük pencere oluştuğu için çalışmak zor oluyor
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez. **Komut:** değişmez (formun
  bileşen sözleşmesi aynı; saat dilimli tarih yine `check_value`'nun kuralıyla yazılır).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0745–0750'den sonra sahibin `kbs.ilceler` tablosundan üretilen form (ADR 0750'nin resimleri) üç zayıflık gösterdi:

- **Tasarımcının ağacı her alanı aynı ikonla gösteriyordu.** Ağaçta on sekiz alanın hepsi “T” idi. Tarih, sayı,
  evet/hayır, liste ya da JSON ağaçtan ayırt edilmiyordu; ne olduğunu görmek için her alanı seçmek gerekiyordu.
  Özellikler'deki Bileşen seçiminde de seçenekler yalnız adlarıyla duruyordu.
- **Büyük pencerede alanlar tek sütundu.** ADR 0746 pencereyi büyütmeyi ve tam ekranı getirdi. Ama 1440 piksellik bir
  pencerede bile alanlar tek sütunda kaldı: sağ yarı boştu, on sekiz alanlık form aşağı kayıyordu.
- **Saat dilimli tarih düz metindi.** `olusturma_tarihi` ve `guncelleme_tarihi` (PostgreSQL `timestamptz`, ADR 0629'un
  `instant`'ı) elle `2026-10-28T16:05:12+03:00` diye yazılıyordu. Tarih ve saat alanlarının seçicisi (ADR 0412) bu
  türe açılmıyordu.

Başvurulan yollar (30 Ekim 2026'da okundu):

- QGIS'in kullanıcı kılavuzu, öznitelik formunun sürükle-bırak tasarımcısı: bir kap, içindeki alanların kaç sütuna
  dağıtılacağını söyler (*Number of columns the embedded fields should be distributed over*). Tarih/Saat bileşeni bir
  satırdır ve tarih, saat ya da ikisi için takvim açar.
  <https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html>
- ArcGIS Field Maps, formu yapılandırma: alan öğesinin giriş türleri (açılır liste, radyo düğmeleri, anahtar, tek ve
  çok satırlı metin, tarih ve saat) ayrı türlerdir; zorunlu öğe adının yanında yıldızla gösterilir. Field Maps mobil
  uygulaması saat dilimi kaymalı zaman damgası alanını (*Timestamp offset*) desteklemez; böyle bir öğe formu açmaz.
  <https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-form.htm>

## Karar

### 1. Ağaçta ve Bileşen seçiminde bileşenin ikonu

Formun ağacındaki her alan, kaydın onu göstereceği bileşenin ikonunu taşır (`widget_glyph`). Bileşen formda verilmemişse
alanın türüne göre olanıdır (`kentos_forms::widget_of`). Tür, aynı bileşenin anlamını ayırır:

| Bileşen | İkon |
|---|---|
| Metin | “T”; büyük tam sayıda `#`, saat dilimli tarihte takvim |
| Çok satırlı metin | satırlar (web setinin `mtext`'i) |
| Sayı | `#` |
| Kaydırıcı | kaydırıcı |
| Tarih | takvim; saatte saat |
| Onay kutusu | işaretli kutu |
| Anahtar | açık anahtar |
| Açılır liste | okuyla bir alan |
| Radyo | seçili radyo düğmesi |
| Parçalı seçim | ilki seçili üç parça |
| Renk | damla (web setinin `color`'ı) |
| İşaretli liste | işaretli iki satır |
| Arama | büyüteç |
| Etiketler | iki etiket |
| Ağaç | bir dal ve iki yaprak |

- JSON alanı her bileşende süslü parantezdir `{ }`, ikili alan belgedir.
- Yeni ikonlar (işaretli kutu, anahtar, açılır liste, radyo, parçalı seçim, işaretli liste, etiketler, ağaç, süslü
  parantez) web setinin 20 × 20'lik ızgarasında ve 1,4 birimlik çizgisiyle çizildi. Bulma ikonu (hedef) kullanılmadı:
  o Sahneden seç'indir.
- Özellikler'deki Bileşen seçiminin her seçeneği aynı ikonla gelir; “Alanın türüne göre” türün seçeceği bileşeninkiyle.
- Yansıma (ADR 0437) ağaçta bağlantı ikonunu korur.

### 2. Geniş pencerede alanlar iki sütunda

Formla düzenle'de bir bölümün ardışık alanları, sayfanın genişliği yazı boyuyla büyüyen 880 pikseli (12 piksellik yazıya
göre) geçince iki sütuna akar (`TWO_COLUMNS`):

- **Sıra yukarıdan aşağıya, sonra yana:** ilk yarı solda, ikinci yarı sağda. Formun sırası ve Tab'ın sırası değişmez.
- **Bölümlerin sınırı korunur:** gruplar, notlar, ilişkili kayıtlar, ekler ve bölücüler kendi yerlerinde kalır; yalnız
  aralarındaki alan dizileri ikiye bölünür. Formun iki sütunlu grubu (ADR 0426) kendi düzeniyle kalır.
- **Dar pencere ve dar yerler tek sütundur:** 880 pikselin altında, tasarımcının önizlemesinde ve alt paneldeki form
  görünümünde (ADR 0416) tek sütun kalır.
- Genişlik pencerenin kendisinden ölçülür (KentOS UI'ın `Responsive`'i). Pencere büyütülünce ya da tam ekran olunca
  form yeniden akar; alanların odağı ve açık seçicisi kaybolmaz.

### 3. Saat dilimli tarihin seçicisi

Saat dilimli tarih alanı (`instant`) metin kutusunun yanında tarih ve saat seçicisini açar:

- Seçici değerin tarihini ve saatini gösterir; seçilen tarih ve saat değerin kendi saat dilimiyle yazılır
  (`2026-10-28T16:05:12+03:00` → seçilen gün, `+03:00` korunur).
- Boş değer cihazın saat dilimini alır (`LOCAL_OFFSET`; sıfırsa `Z`).
- Kesirli saniye seçilince düşer; seçici saniyeye dek seçer.
- Kutuya elle yazmak eskisi gibidir; değer ADR 0629'un kuralıyla denetlenir. Zorunlu olmayan alanda seçici değeri
  temizleyebilir.

### 4. Seçicinin “Şimdi”si ve bugünü

KentOS UI'ın seçicisinin “Şimdi”si ve takvimde işaretli bugün UTC'ydi; formlar ona saati vermiyordu. Türkiye'de
“Şimdi”ye basılan tarih ve saat alanına üç saat önceki zaman yazılıyor, gece yarısından sonra bugün dün görünüyordu.

- **Saat dilimli tarih:** “Şimdi”, değerin saat dilimindeki şu andır (`+03:00`'lü değerde Türkiye saati). Yazılan an
  gerçekten şu andır.
- **Saat dilimsiz tarih, tarih ve saat, saat:** “Şimdi” ve bugün cihazın saatiyledir (`LOCAL_OFFSET`).

## Doğrulama

- `cargo test -p kentos-desktop --bin kentos-cad forms::entry::tests`:
  - saat dilimli değerin tarihi, saati ve dilimi (`Z`, `+03:00`, `-05:30`, kesirli saniye);
  - seçicinin geri yazdığı metin;
  - boş ve okunamayan değerde cihazın dilimi;
  - dilimin dakikaya çevrilmesi (“Şimdi”nin anı) ve metne geri dönmesi.
- `cargo test -p kentos-desktop --bin kentos-cad district_form_screens -- --ignored --test-threads=1`: sahibin
  `kbs.ilceler` katmanının formu, 1100 × 650 ve 1440 × 900, `.run/shots/form-ilce-*`:
  - `1-tasarim`: ağaçta her alan bileşeninin ikonuyla (metin, JSON, saat dilimli tarih, sayı, onay kutusu);
  - `1b-oge-turu`: evet/hayır alanının Öğe türü listesi, her bileşen kendi ikonuyla;
  - `3-formla-duzenle`: kendi boyunda tek sütun;
  - `4-tam-ekran`: tam ekranda 1440 × 900'de iki sütun, 1100 × 650'de tek;
  - `5-zaman-secici`: `olusturma_tarihi`'nin açık seçicisi, değerin günü ve saati seçili, “Şimdi” Türkiye saatiyle.
- `every_forms_window_keeps_its_buttons_in_a_low_window`: formların 92 sahnesinde düğmeler 1100 × 650 pencerenin
  içinde (ADR 0749).
- Masaüstünün bütün testleri (1390); `clippy -D warnings`.

## Kalan

- Formun yeni kaydında katman alanının varsayılanının gösterilmesi (ADR 0750'nin kalanı).
- Sütun sayısının formda seçilmesi (QGIS'in kabı gibi); şimdilik genişlikten bulunur.
- Saat dilimli tarihin seçicide başka bir dilime çevrilmesi.
