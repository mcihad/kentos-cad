# 0990 — Boykesitte kot taşıma

Tarih: 2026-11-06. Durum: kabul (Y84). Kapsam ROAD-SEC-11 (plan–profil paftası), ADR 0908 §10 (pafta görünümleri).

## Sorun

Boykesit çerçevesi çizginin kot farkı kadar yüksekti. Kot farkı büyük bir yolda görünüm paftanın boykesit haritasına
sığmıyor, `road.sheets.plan` yalnız `road_sheet_outside` uyarısı veriyordu. Kullanıcı ya ölçeği ya abartmayı
küçültüyordu; ikisi de paftayı KGM'nin istediğinden uzaklaştırır.

## Araştırma (6 Kasım 2026)

- **KGM 2019 teknik şartnamesi** (2025 ve 2026 birim fiyatları hâlâ ona bağlı, ADR 0984), boykesit maddeleri:
  "Profillerin paftalarda ayrılan genişliğe sığmaması durumunda kot taşıma çizgisi yapılacaktır."
- **KGM Haziran 2024 örnek paftaları:**
  - Sayfa 8: çerçeve iki dik çizgiyle üç parçaya bölünmüş. Kot ölçekleri 955–980, 975–1000, 990–1015 (25 m'lik
    çerçeve, 5 m'lik ızgara). Her çizginin iki yanında iki parçanın kotları yazılı ("980 | 1000").
  - Sayfa 11: tek kırılma, 860–885'ten 870–895'e.
  - Kıyas kotları ızgara adımının katlarıdır.
- **Civil 3D** ([Profile View Height](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-742535A5-8C1B-4A3D-8B0E-D68C147840B4.htm),
  [About Profile Views](https://help.autodesk.com/cloudhelp/2019/ENU/Civil3D-UserGuide/files/GUID-C2C782D2-899C-4B0B-9E61-9D72A80AFC80.htm)):
  - Bölme yalnız kullanıcı yükseklik verdiğinde vardır ("User Specified").
  - Otomatik bölme görünümü o yüksekliğe sığdırır, bölme yerini ve kıyas kotunu ızgaraya yuvarlar.
- **Netcad:** bu araştırmada kaynak bulunamadı.
- **Matematik:** hesap yok, karşılaştırma var. Bir parçaya sığması gerekenler:
  - kırmızı kotun tam değerleri (parça uçlarında, aradaki eleman uçlarında ve kurp tepe/çukurlarında);
  - siyah kotun noktaları ve uçlardaki doğru üstü değeri.
  - Parçanın tuttuğu kot aralığı sonuna doğru yalnız büyür (monoton). Bu yüzden en uzak sığan son ikiye bölmeyle
    bulunur.

## Karar

- **`road.profiles.draw` `span`** (m): çerçeve `span × exaggeration` yüksektir.
  - Görünüm baştan başlayarak açgözlü bölünür. Her parça, çizgilerin sığdığı en uzak **tam metreye** kadar sürer.
  - Parçanın kıyas kotu, en alçak kotun bir metre altının ızgara adımına aşağı yuvarlanmışıdır. Üstü bu kotun `span`
    fazlasıdır ve en yüksek kot üstten en az bir metre içeride kalır.
  - Izgara adımı `span`'ın en çok 10 çizgi verdiği yuvarlak adımdır (eski kuralla aynı).
  - `datum` ile birlikte verilmez. 0'dan büyük olmalıdır. Tek bir metreye sığmayan kot sıçramasında `invalid_label`
    ("… km ile … km arasındaki kot farkına yetmiyor").
  - Çıktıda `splits` (from, to, datum, top); `datum` ve `top` ilk parçanınkidir. `span` yoksa çıktı bayt bayt eskisi
    gibidir.
- **Çizim:**
  - Her kırılmada çerçeve kalınlığında dik bir çizgi. Solda önceki parçanın, sağda sonrakinin kotları yazılır; yazılar
    maskelidir (bkz. Ölçüm).
  - Izgara, siyah kot ve kırmızı kot her parçada kendi kıyas kotuyla çizilir. Çizgi kırılmada kesilip doğru üstü
    değeriyle yeniden başlar.
  - Kurp yazısı ve proje uçları bulundukları parçanın kotuyla yerleşir. Satırlar (ADR 0989) ve bantlar değişmez.
- **`road.sheets.plan`:** görünüm, satırları ve bantlarıyla boykesit haritasından yüksekse
  `span = ⌊(harita − satırlar − bantlar) / abartma⌋` ile yeniden çizilir. Bölünemiyorsa eskisi gibi tek parça kalır
  ve `road_sheet_outside` uyarır. Komutun girdisi değişmedi.
- **Boykesit penceresi:** Çizime satırında "Kot aralığı (m)"; boşsa eskisi gibi.

## Ölçüm

- **Bağımsız başvuru** `road_profile_command_cases.py` (`split_parts`, KentOS kodu yok):
  - Tam metreleri tek tek tarar; KentOS ikiye böler. Aynı parçaları bulmaları monotonluğun da sınamasıdır.
  - Nesne sayısını da üretir.
- **`road.profiles.draw`** 56 durum, 8'i yeni:
  - MAIN 12 m: 4 parça, 0–274 (100), 274–557 (104), 557–604 (106), 604–669 (108).
  - 15 m KGM bantlarıyla 2 parça; 20 m tek parça; PART 5 m 2 parça; km 100–500 görünümü 8 m 3 parça.
  - Üç ret: 0, `datum` ile, 6 m (km 1+013–1+014 arasındaki arazi sıçraması).
  - Bir parça bitişini 1 m değiştirmek durumu düşürüyor; çıktı gerçekten karşılaştırılıyor.
- **`road.sheets.plan`** 36 durum. Yeni durum 1:1000 ve 10 kat abartmada:
  - 1. pafta 7 m'ye bölünür ve sığar.
  - 2. pafta 7 m'ye bölünemez (km 1+557–1+558 sıçraması) ve uyarır.
  - "Doğrulama" durumunda dışarı taşan pafta sayısı 2'den 1'e indi.
- **Resimler** `.run/gui-evidence/yol-y84/`:
  - KGM A1 paftası, 1:2000, 40 kat abartma (`y84-pafta.pdf`): dört parça.
  - Pencerede "Kot aralığı 12" (1440 ve 1100 px, taşma yok) ve çizim (`y84-cizim-*.png`).
  - İlk resimde kırılmadaki kotların üstünden ızgara çizgisi geçiyordu ("112.00" çizik). Kırılma yazıları maskelendi
    (`y84-kirilma.png`).

## Açık

- **Paftanın varsayılan yeri** "0; -500" sabit. Harita kadar yüksek bir çerçeve oradan plan nesnelerinin üstüne
  çıkabiliyor; resim testinde "0; -3000" verildi. Varsayılan yer çizimin altından seçilmeli.
- **Sağ kenarda kotlar** yazılmıyor (2024 örneğinde iki yanda var).
- **Bölme yeri:** KGM ve Civil 3D'de bölme yeri elle de seçilebiliyor; burada yalnız otomatik.
- **Tek metrede sığmayan sıçrama** reddediliyor; bir metreden ince bölme yok.
