# 0995 — Boykesitin altında hafriyat satırları (denge kesimleri)

Tarih: 2026-11-07. Durum: kabul (Y89). Kapsam ROAD-SEC-11, ROAD-HAUL (Brükner, ADR 0907) ve ADR 0980 (KGM bantları).

## Sorun

KGM 2019 şartnamesi denge kesimlerinin kazı, dolgu ve faktörlü kazısının paftada yazılmasını zorunlu tutar. KentOS
Brükneri hesaplıyor (`road.hauls.*`) ama plan–profil paftasında bunları yazmıyordu.

## Araştırma (7 Kasım 2026)

- **KGM 2019 teknik şartnamesi**, Brükner ve dengeleme hesapları:
  - "Denge kesimlerinin başlangıç ve bitiş kilometrelerinin diyagramda gösterilmesi gereklidir. Bunlara ait kazı,
    dolgu, faktörlü kazı, taşıma momenti gibi hesaplar veya sonuçlarının paftada yazılması zorunludur."
  - Ariyet ve depo gösteren kesimler belirtilir.
- **KGM Haziran 2024 örnek paftaları**, s. 8–12: boykesitin hemen altında beş satır, HAFİR, HAFİRLİ FAK., İMLA,
  ARİYET, DEPO.
  - s. 9: tek bir denge kesimi (DN 0+000.00 – DN 15+458.18) her paftada aynı toplamlarla yazılı: 602114, 661737,
    661737, -, -. Faktörlü kazı dolguya eşit, ariyet ve depo yok.
  - Denge noktasında satırları kesen dik çizgi ve yanında yukarı okunan "DN: …".
  - Seçme malzeme (SM) ve dolguda kullanılamayan yarma (DKY) kesimleri ayrı işaret ve notlarla
    ("TOPLAM 9805 m3 Dolguda Kullanılamayan Yarma … 1 Nolu Depoya").
- **Matematik:**
  - Brükner ordinatı birikimli düzeltilmiş kazı eksi dolgudur (ADR 0906, 0907). Denge noktası, ordinatın denge
    çizgisine değdiği yerdir.
  - İki denge noktası arasında ordinat, alanlar dahil, çizgiye döner. Bu yüzden kesitin ham neti
    (N = faktörlü kazı − faktörlü dolgu) o kesime getirilen ödünç (N < 0) ya da oradan götürülen dökümdür (N > 0).
    Uçtaki dengesiz kesimde aynı fark ariyet ya da depodur.
  - Bir kübaj satırı (iki kesit arası) içinde ordinat doğrusaldır; denge noktası satırı uzunlukla böler.

## Karar

- **`road.profiles.draw` ve `road.sheets.plan` `haul`:** Brüknerin kimliği ya da adı.
  - Yalnız `bands: kgm` ile; değilse `invalid_label`.
  - Brüknerin kübajının koridoru boykesitin aksında olmalı; değilse `invalid_label`.
- **Kesimler:** kübajın ilk istasyonundan sonuncusuna, Brüknerin denge noktalarıyla bölünür.
- **Değerler:**
  - HAFİR: ham kazı.
  - HAFİRLİ FAK.: kazı × kabarma katsayısı.
  - İMLA: ham dolgu.
  - DEPO = max(0, N), ARİYET = max(0, −N).
  - Tam m³ yazılır; 0,5'ten küçükse "-" (2024 gibi).
  - Görünüme düşen her kesim **bütün toplamlarıyla** yazılır (2024'te bir kesim birçok paftada aynı sayılarla).
- **DN:**
  - Denge noktalarında.
  - Başta, denge çizgisi 0 ise (ordinat 0'dan başlar).
  - Sonda, ordinat alanlarıyla birlikte çizgiye dönüyorsa.
  - Dik çizginin solunda, yukarı okunur, maskeli.
- **Yer:** satırlar çerçevenin hemen altında, KGM bantları onların altında. Pafta görünüm derinliğine satırları ekler.
- **Pencereler:** Boykesit ve Yol paftaları pencerelerinde KGM seçimlerinin yanında "Hafriyat" (aksın Brüknerleri,
  varsayılan "Satır yok").

## Ölçüm

- **Bağımsız başvuru** `scripts/fixtures/road_earthwork_band_cases.py` (KentOS kodu yok):
  - Kübaj tablosu `road_volume_command_cases.py`'nin, denge noktaları `road_haul_command_cases.py`'nin kendi
    diyagramından.
  - `fixtures/road/v1/earthwork-bands.json`; `sections_are_the_references` 1e-3 m³ ve 1 µm içinde aynı.
- **`road.profiles.draw`** 62 durum, 6'sı yeni:
  - Ana Brükner tek kesim 0–600: 25878 / 31612 / 3507 / - / 28116 (fazla kazı döküme).
  - Yan Brükner, 300'de 6000 m³ döküm alanı: dört kesim, DN 0+050, 0+278.99, 0+300, 0+323.61.
    - 278.99–300 DEPO 2429 ve 300–323.61 DEPO 3571, toplamı alanın 6000 m³'ü.
    - Sondaki 7722 dengesiz fazla.
  - Görünüm parçası ve üç ret.
- **Resimler** `.run/gui-evidence/yol-y89/`:
  - KGM A1, 1:1000, Yan aks (`y89-satirlar-*.png`); 1100 px pencerede Hafriyat seçimi.
  - İlk resimde ayırma çizgisi DN yazısının içinden geçiyordu ve satır çizgileri onu kesiyordu. Yazı çizginin soluna
    alındı ve maskelendi.

## Açık

- **SM ve DKY işaretleri ile notları:** uygunsuz malzeme (ADR 0944) ve seçme malzeme kesimleri, ödünç/döküm alanının
  adı ve uzaklığı yazılmıyor.
- **Taşıma momenti** satırı yok (şartname "taşıma momenti gibi" der; 2024 örneğinde satır olarak yok).
- **Satır içi bölme doğrusal:** bir kübaj satırının kazı ve dolgusu satır boyunca düzgün varsayılır.
