# 4160 — Hata elipsi etiketinin boş yere konması

Tarih: 2026-11-09. plan/map-cad.md MAP-05, sekizinci tur. Durum: taslak. ADR 3760'ın “Sonra”sındaki “Etiketin
komşu elipsle çakışınca kaydırılması”.

İlgili:
- [ADR 2960](2960-network-error-ellipses-drawn.md): kabul edilen ağın hata elipsleri;
- [ADR 3360](3360-network-height-bars-drawn.md): kot çubukları;
- [ADR 3760](3760-network-ellipse-labels.md): elipslerin etiketleri, doğuda;
- [ADR 0145](0145-text-extras.md): yazının hizası ve kutusu.

## Dayanak

- **QGIS 3.40 kullanıcı kılavuzu**,
  [Label settings › Placement](https://docs.qgis.org/3.40/en/docs/user_manual/style_library/label_settings.html)
  (9 Kasım 2026'da okundu):
  - noktanın “Cartographic” yerleşiminde aday yerlerin sırası: sağ üst, sol üst, sağ alt, sol alt, sağ orta, sol orta,
    üst (biraz sağda), alt (biraz solda). Kılavuz bu sırayı Krygier ve Wood'a (2011) ve öbür kartografya kitaplarına
    dayandırır;
  - uzaklık noktadan ya da simgenin sınırından ölçülebilir;
  - “Features act as obstacles”: bir katmanın nesneleri etiketlere engeldir;
  - “Allow overlaps if required”: başka yer yoksa etiket çakışarak da çizilir.
- **KentOS bugün** (ADR 3760): etiket hep elipsin doğusundadır. Yakın iki noktada etiket komşunun elipsinin ya da
  etiketinin üstüne düşebilir; okunmaz.

## Karar

1. **Aday yerler, sırasıyla:**
   - önce doğu: ADR 3760'ın yeri, QGIS'in sağ ortası. Boşken etiketler yerinden oynamaz, eski çizimler aynı kalır;
   - sonra QGIS'in kartografik sırası: sağ üst, sol üst, sağ alt, sol alt, batı (sol orta), üst, alt.

   Her yer noktadan bir yön ve oradaki hizadır. Doğu `middleLeft`; sağ üst `bottomLeft`, sol üst `bottomRight`, sağ alt
   `topLeft`, sol alt `topRight`; batı `middleRight`, üst `bottomCenter`, alt `topCenter`. Etiketin noktası o yönde,
   büyütülmüş büyük yarı eksen artı bir etiket boyu uzaktadır: dönük hiçbir elips o çemberi aşmaz. Uzaklık QGIS'teki
   gibi simgenin sınırından ölçülür.
2. **Engeller:** etiketin kutusu çizimin ölçtüğü gibidir. Genişliği çizim yazı tipiyle harflerin ilerleyişleri çarpı
   boyudur; yüksekliği tabanın 0,2 × 1,15 boy altından 1,15 boy üstünedir; hizasıyla yerleşir (ADR 0145).
   - Elips: kutu, elipsin kendi ekseninde birim çembere ölçeklenince merkez kutunun içindeyse ya da bir kenarı 1'den
     yakınsa elipse değer. Yassı elips büyük eksenidir.
   - Kot çubuğu: parçaları kutunun içinden geçerse (Liang–Barsky).
   - Daha önce konmuş etiket: kutular örtüşürse.
   - Kenara değmek çakışma sayılmaz.
3. **Sıra:** istasyonlar ağın sırasıyla. Her etiket bütün elipslere ve çubuklara, ama yalnız kendinden önce konmuş
   etiketlere bakar.
4. **Boş yer yoksa** etiket doğuda kalır ve çakışıyor sayılır (QGIS'in “Allow overlaps if required”ı).
5. **Cevap:** `labelsMoved` (doğudan başka yere konan), `labelsCrowded` (boş yeri olmayan). Komutun sürümü aynı.
   Eskiden sırayla yazılan elips, çubuk ve etiket sırası korunur.
6. **Masaüstü** (GNSS ağ dengelemesi): kabul iletisi söyler: “4 etiket yazıldı (1'i elipse ya da etikete düşmesin
   diye başka yana kondu)”; boş yeri olmayanı “doğuda çakışıyor” diye sayar.
7. **Kapsam işareti:** ADR 3760'taki gibi **Harita**.

## Sınırlar

- Etiket, ağın kendi elipslerine, çubuklarına ve etiketlerine bakar. Çizimin öbür nesneleri (noktaların adları, öbür
  katmanlar) engel değildir.
- Konmuş bir etiket sonradan konan için yer değiştirmez: sıra açgözlüdür, en iyi bütünü aramaz.
- Yer sekiz adaydır. Uzaklık artırılmaz; QGIS'in en büyük uzaklık seçeneği yok.

## Kanıt

- **Birim testleri** (`network_ellipses::tests`):
  - 45° dönük ince elips kendi çerçevesiyle ölçülür (dış dikdörtgeni değmese de); yassı elips ekseniyle;
  - doğusu dolu etiket sağ üste, ikisi dolu sol üste gider; dört yanı dolu etiket doğuda kalır, çakışıyor sayılır.
- **Bağımsız başvuru** (`scripts/fixtures/map_survey_network_cases.py`; KentOS kodu yok):
  - yazının genişliği fontTools'la çizimin varsayılan yazı tipinden (Barlow 500) okunur;
  - kutu, elips, parça ve kutu sınamaları elle yazılmıştır;
  - yeni durum: N6, N1'in 15 m doğusunda; N1'in etiketi doğuda N6'nın elipsine düşer, sağ üste geçer; öbür üçü doğuda.
    Cevapta `labelsMoved` 1;
  - ADR 3760'ın etiket durumları değişmez.
- **Rust ve Python** ortak durumları geçer.
- **Python** (`python/tests/test_network_label_places.py`), testin kendi ölçümüyle (fontTools):
  - hiçbir etiket bir elipse ya da başka etikete değmez;
  - N1'inki doğuda N6'nın elipsine değerdi, sağ üsttedir.
- **MCP** (`crates/native/mcp/tests/network_label_places.rs`): cevap 4 etiket, 1 taşınan; `drawing.entities` ile
  okunan etiketlerin hizaları N1 `bottomLeft`, öbürleri `middleLeft`. ADR 2960 ve 3360'ın MCP testleri geçer.
- **Masaüstü** (`network_ellipses_tests::a_label_that_would_cover_a_neighbour_goes_to_a_free_place`): N1'in etiketi
  sağ üstte, ileti sayar.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/etiket-yeri-4160/`.

## Sonra

- Robust kestirim; seyrek çözüm; ağırlıklı benzerlik (MAP-05'in kalanı).
- Çizimin öbür nesnelerini (nokta adları) de engel sayma seçeneği.
- Web (TODOS §0.2).
