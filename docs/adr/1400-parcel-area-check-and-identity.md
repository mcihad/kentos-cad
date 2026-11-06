# 1400 — Parsel alan denetimi ve bağlamlı parsel kimliği (`cad.parcels.check`)

Tarih: 2026-11-06. plan/map-cad.md MAP-08 (alt maddeler: geometrik ve kayıtlı alan ayrı; kimlik il/ilçe/mahalle/ada/parsel
bağlamında). Durum: taslak. İlgili: [ADR 0067](0067-desktop-measure-and-parcel.md) (Parsel oluştur), CLAUDE.md §7, §23.

## Dayanak

- **Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik** (Resmî Gazete 27.09.2022, sayı 31966;
  dayanağı 3402 sayılı Kadastro Kanunu m. 41):
  - **Madde 4/v:** yüzölçümü hesabı yanılma sınırı, ilk tescile esas hesapla arşiv bilgilerinden yeniden üretilen köşe
    koordinatlarıyla yapılan ikinci hesap arasındaki farkın kabul edilebilir en yüksek sınırıdır.
  - **Madde 7:** sınır içindeki fark hata sayılmaz; tapu yüzölçümünün doğruluğu karinesi sürer.
  - **Madde 8(2):** arşiv onaylı geometride sınır, desimetrekare mertebesindeki alandan, tablodaki değerlerle artan
    oranlı hesaplanır. **Madde 8(5):** geçici koordinatlı parsellerde köşelere nokta konum doğruluğu yarıçaplı hata
    çemberleri çizilir, 400n alan hesaplanır.

  Okunan metinler: [Lexpera'daki özgün metin](https://www.lexpera.com.tr/resmi-gazete/metin/tapu-planlarinda-yanilma-sinirinin-belirlenmesi-hakkinda-yonetmelik-31966)
  (tablo resim olarak duruyor); mevzuat.gov.tr PDF'i bu makinede sertifika hatası verdi, okunamadı. Tablonun değerleri
  üç bağımsız ikincil kaynaktan alındı, üçü de aynı ([Lider Yazılım](https://portal.lideryazilim.com/books/ornek-uygulama-yanilma-siniri-hesabi/page/yanilma-siniri-hesabi),
  [Bozdoğan](https://www.bozdogan.co/bilgilendirme/yanilma-siniri-hesabi-nedir-nasil-hesaplanir/) ve arama dökümünde
  görülen mevzuat.gov.tr satırı). Tablo dilim sınırlarında süreklidir (0,5; 2,3; 6,3; 8,8; 24,8; 84,8), bu da okumayı
  doğrular. **Resmî metinle karşılaştırma bekleyen iş olarak kalır.** Kaynaklardaki örnekler: 100 m² → ±2,3; 950 m² →
  ±8,55; 300 m² → ±4,30; 100,75 m² → küsurat alt dilimden ±2,32.

  | Alan (m²) | Oran | Sınır (m²) |
  |---|---|---|
  | 0–10 | 0,05 | Alan × 0,05 |
  | 11–100 | 0,02 | 0,5 + (Alan − 10) × 0,02 |
  | 101–500 | 0,01 | 2,3 + (Alan − 100) × 0,01 |
  | 501–1000 | 0,005 | 6,3 + (Alan − 500) × 0,005 |
  | 1001–5000 | 0,004 | 8,8 + (Alan − 1000) × 0,004 |
  | 5001–25000 | 0,003 | 24,8 + (Alan − 5000) × 0,003 |
  | > 25000 | 0,0015 | 84,8 + (Alan − 25000) × 0,0015 |
- **Netcad:** Netmap'te tapu alanı “baz alan” seçilip alan özeti alınır (ikincil ders notu; sürümün kendi belgesiyle
  doğrulanmalı). ArcGIS parsel ağında kayıtlı ölçü ile koordinat ayrıdır (plan/map-cad.md [S17]).
- **KentOS bugün:**
  - Parsel, altı öznitelikli bir alandır (K19). Tapu alanı yazıldığı gibi tutulur; Bilgi kartı onu hesap alanının
    yanında gösterir.
  - Karşılaştırma ve yanılma sınırı yoktur.
  - `cad.parcels.create` yinelenen numarayı yalnız Parsel numarasına bakarak uyarır: Ada 101'in 7'si ile Ada 102'nin
    7'si “yinelenen” sayılır. Sıradaki numara da bütün çizimden verilir.

## Karar

1. **`cad.parcels.check` v1, salt okunur (Harita + CBS).**
   - **Kapsam:** verilen alanlar ya da parsel katmanının bütün alanları.
   - **Hesap alanı:** çekirdeğin `entity_area`'sı (yay ve delikler dahil).
   - **Kayıtlı alan:** öznitelik (ilk değer `Tapu alanı (m²)`) yazıldığı gibi tutulur. Düz ondalık (`118,5`,
     `723.525`) okunur; binlik ayırıcılı ya da başka yazı `invalidRecord` olur, tahmin edilmez.
   - **Fark ve sınır:** fark hesap − kayıttır. Sınır, Madde 8(2) tablosuyla hesap alanından bulunur; küsurat alt
     dilimden alınır. Durum `within`, `outside`, `noRecord` ya da `invalidRecord`'dur.
   - **Kural adı:** cevap kuralın adını taşır (`rule: tkgm-2022-m8-arsiv`, CLAUDE.md §23.4).
   - **Sınırlar:** hesap alanı kaydın üstüne yazılmaz. Sonuç resmî uygunluk iddiası değildir. Geçici koordinatlı
     parsel yöntemi (Madde 8(5), hata çemberleri) bu dilimin dışındadır.
2. **Kimlik:** İl, İlçe, Mahalle, Ada ve Parsel birliktedir. Aynı kimliği iki parsel taşıyorsa `duplicates`'ta
   bildirilir; Ada ya da Parsel'i olmayan parsel `noIdentity` olur.
3. **K19 düzeltmesi, `cad.parcels.create`:** yinelenen numara uyarısı ve sıradaki numara yeni parselin yerine bakar
   (İl, İlçe, Mahalle, Ada). Verilmeyen parça her şeyle eşleşir, böylece Ada verilmeden çizilen parsel eskisi gibi
   bütün parsellerle karşılaştırılır. Uyarının sözü ve kodu değişmedi; üç yeni ortak durum eklendi.
4. **Pencere Parsel alan denetimi…** (`parcels.check`, ikon `parcelArea`):
   - **Yeri:** Harita'da Plan › Ada ve parsel, CBS'de Veri › Tablo. Yol ve Mimari sunmaz.
   - **İçerik:** özet kutusu, süzgeç (Tümü, Sınır dışında, Kaydı yok / okunamadı, Yinelenen kimlik) ve kitin veri
     tablosu (Ada/Parsel, Mahalle, Hesap alanı, Tapu alanı yazıldığı gibi, işaretli fark, ±sınır, Durum).
   - **Düğmeler:** Dışındakileri seç sınır dışındaki parselleri seçer. CSV kaydet sayıları hesaplandığı gibi yazar.
     Kapat.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/parcels_check_cases.py` 21 durum üretir. Alan rasyonel shoelace'le ve
  işaretli yay parçasıyla, sınır tablodan elle hesaplanır. Durumlar şunları kapsar:
  - dört durum ve yinelenen kimlik;
  - başka katmandaki verilen alan, yaylı ve delikli parsel;
  - dilim sınırları (10, 5000, 25000, 90000 m²);
  - küsurat ve alt dilim: 10,5 m²'de ±0,525; 100,75 m²'de 2,31'lik fark ±2,315 içinde, üst dilime göre (±2,3075)
    dışında olurdu;
  - binlik ayırıcı, başka öznitelik, retler.

  Rust ve Python geçer. `cad.parcels.create`'in 26 durumu (3'ü yeni) de geçer.
- **Masaüstü:** `parcels_check_window_tests.rs` denetimi ve seçimi, salt okunurluğu, seçimin kapsamını, CSV'yi ve
  modül işaretlerini sınar. Resimler `.run/gui-evidence/parsel-alan-1400/`.

## Sonra

- Madde 8(2) tablosunu Resmî Gazete'nin özgün metniyle karşılaştırmak.
- Geçici koordinatlı parsellerin hata çemberi yöntemi (Madde 8(5), Madde 9 nokta konum doğruluğu tablosu).
- Kalıcı parsel kaydı:
  - kaynak belge, geçerlilik zamanı ve önceki/sonraki parsel bağı;
  - mantıksal ortak sınır kimliği;
  - erişim denetimli hak ve pay kayıtları (MAP-08'in kalanı).
