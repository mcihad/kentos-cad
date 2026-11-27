# ADR 9329: Nokta ve GNSS içe almada yeni katman hep yenidir; çizgiler ayrı katmana gider, rapor ve kaynak kalır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  12, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0169](0169-field-data.md) §6 (GNSS içe aktar),
  [ADR 0222](0222-kml-gpx-and-explicit-transformation.md) §6 (rotalar ve izler çizgi olarak),
  [ADR 9192](9192-one-source-catalog.md) (katmanın kaynağı), [ADR 0519](0519-vertical-crs-and-epoch.md) §6 (yüksekliğin türü).

## Bağlam

- **Gözlem:**
  - Nokta listesi `netcad.ncn`'yi doğru okudu; GNSS `sample.gpx`'ten 9 nokta ve 1 çizgi aldı, okunmayan iki kaydı
    önizlemede nedenleriyle gösterdi.
  - GPX katmanı varken NMEA da öntanımlı olarak `sample` adını önerdi. Yeni katman seçiliyken yardım satırı “Bu adda bir
    katman varsa noktalar ona eklenir” diyordu: Yeni katman denip var olan katmana ekleniyordu.
- **İncelemenin kabulü:**
  - Yeni katman için çakışmayan ad önerilsin ya da Var olan katmana ekle açıkça seçilsin.
  - Nokta ve iz geometrileri için ayrı çıktı hedefi olsun.
  - Önizlemedeki dönüşüm doğruluğu, elipsoit yüksekliği ve reddedilen satırlar sonuç raporuna geçsin.
  - Kaynak dosya kaydı ve alınan kayıt sayıları yeniden açılışta doğrulansın.

## Araştırma

- **QGIS:** GPX dosyasının yol noktaları nokta katmanı, rotaları ve izleri çizgi katmanı olarak açılır. Her tür ayrı
  bir katmandır ([GNSS/GPS verisi](https://docs.qgis.org/4.2/en/docs/user_manual/working_with_gps/plugins_gps.html)).
- **Netcad Hesap:** nokta düzenleme, dönüşüm ve raporlama aynı iş akışındadır
  ([Netcad Hesap](https://www.netcad.com/tr/urunler/hesap); inceleme bağlantısı).
- **KentOS kararı:**
  - Yeni katman hep yeni bir katmandır; var olan katmana ekleme yalnız Hedef katman'dan seçilerek olur.
  - Rotalar ve izler öntanımlı olarak kendi katmanlarına gider.
  - İçe almanın raporu günlüğe yazılır, dosyanın kaydı katmanda kalır (ADR 9192'nin `origin`'i).

## Karar

### 1. Yeni katmanın adı

- **Önerilen ad:** Nokta listesi ve GNSS penceresi, çizimde hiçbir katmanın taşımadığı bir ad önerir: dosyanın adı;
  varsa `ad_2`, `ad_3`… Karşılaştırma büyük-küçük harf ve Türkçe işaretlerden bağımsızdır (`free_layer_name`).
- **Var olan ad:** Yeni katman seçiliyken yazılan ad bir katmanınsa içe alma yapılmaz. Alanın altında yolu söylenir:
  ““sample” adında bir katman var. Onun üstüne eklemek için Hedef katman'dan seçin; yoksa başka bir ad yazın.”
- **Yardım satırı:** ad boşken “Yeni bir katman açılır. Var olan bir katmana eklemek için Hedef katman'dan onu seçin.”
- **Var olan katmana ekleme:** yalnız Hedef katman'dan seçilince olur.

### 2. Çizgilerin katmanı (GNSS)

- Dosyada rota ya da iz varsa pencere “Çizgilerin katmanı”nı sorar. Seçenekler:
  - Yeni katman (öntanımlı, adı `<noktaların katmanı>_iz`, boş adla);
  - Noktaların katmanı;
  - çizimin bir katmanı.
- Aynı kurallar geçerlidir: var olan ad reddedilir; çizgilerin yeni katmanının adı noktalarınkiyle aynı olamaz; kilitli
  katman söylenir.
- Noktalar ve çizgiler tek geri alma adımıyla alınır.

### 3. Rapor

İçe almadan sonra günlükte şunlar kalır:

- ne alındı, hangi katmana ve kaç yeni katman açıldı;
- dönüşüm ve doğruluğu (ör. “WGS 84 → TUREF / TM36: ±1 m, EPSG:5261”; resmî olmayan dönüşüm uyarıdır);
- yüksekliğin türü (elipsoit yüksekliği ya da projenin söylediği);
- sistemin kullanım alanı dışındaki konumlar;
- alınmayan kayıtlar ve nedenleri (ilk beşi, satırlarıyla, kalanın sayısı).

Önceden yalnız sayı yazılıyor, “nedenleri içe aktarma penceresinde yazılıydı” deniyordu.

### 4. Kaynak kaydı

- Pencerenin açtığı her katman dosyasını tutar (ADR 9192 `LayerOrigin`): biçim (dosyanın uzantısı, `gpx`, `nmea`,
  `ncn`), dosyanın tam yolu, SHA-256'sı, okunduğu zaman.
- Kayıt içe almanın geri alma adımının içindedir (`apply_file_import`).
- Proje yazılıp yeniden açılınca katmanlar, nesneleri ve kayıtları yerindedir.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Nokta listesi: boş ad, var olan adın reddi, kaynak kaydı | yok: Mimari'nin nokta içe alması yoktur | var | var | var |
| GNSS: aynı kurallar, çizgilerin katmanı, rapor | yok: aynı gerekçe | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Pencereler masaüstünün içe alma akışıdır. Betik dosyayı okuyup nesneleri `cad.entities.create` ile kendi seçtiği katmana yazar. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Nokta listesi:** dosyanın bir sütunundan gelen çizgi tutmaz; çizgi hedefi yalnız GNSS'tedir.
- **Kaydın doğrulanması:** kaynak dosya değişmişse (SHA-256 farklı) yeniden açılışta söylenmesi, kaynak kataloğunun
  (ADR 9192) işidir.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- exchange::gnss exchange::tests::a_coordinate`: 10 geçti.
  - `a_new_layer_is_never_one_of_the_drawings_and_the_lines_have_their_own`:
    - GPX'in 9 noktası `sample`'a, izi `sample_iz`'e gidiyor;
    - aynı adlı NMEA'ya `sample_2` ve `sample_2_iz` öneriliyor; `SAMPLE` reddediliyor, hiçbir şey alınmıyor;
    - çizgiler Noktaların katmanı'na verilince 4 konum ve 1 çizgi `nmea` katmanında;
    - kayıt `nmea`, dosyanın yolu ve 64 haneli özetle;
    - proje yazılıp okununca 9, 1 ve 5 nesne ve `gpx` kaydı yerinde.
  - `a_coordinate_list_takes_a_layer_of_the_drawing_only_when_it_is_picked`: `çizim` adı reddediliyor; Hedef
    katman'dan Çizim seçilince noktalar ona; sonraki yeni katman `netcad` ve kaydı `ncn`.
  - `a_gpx_goes_in_…`: günlükte alınmayan iki kayıt satırları ve nedenleriyle.
- **Resim** (`.run/gui-evidence/kullanim-9329/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-bos-adlar`: aynı adlı NMEA'ya `sample_2` ve `sample_2_iz`;
  - `02-katmanin-adi`: `sample` yazılınca neden ve yol, İçe aktar kapalı.
