# 3140 — Stok hacmi: yığının hacmi sınırından çizilen tabana göre

Tarih: 2026-11-09. plan/map-cad.md MAP-15, beşinci tur. Durum: taslak. Plan maddesi: “Kazı/dolgu sınırı, sıfır hattı,
malzeme bölgesi, kabarma/sıkışma katsayısı ve stok sahası ayrı parametre olsun”; ADR 2740'ın “Sonra”sındaki “stok
sahası”.

İlgili:
- [ADR 1600](1600-surface-volume-method-and-factors.md): iki yüzeyin hacmi (`road.terrains.compare`, TIN–TIN ve ızgara);
- [ADR 0920](0920-road-terrain-coverage.md): kapsam sınırı (`boundary`: düz kenarlı, tek parçalı kapalı nesne);
- [ADR 1840](1840-cut-and-fill-zones.md), [ADR 2740](2740-cut-fill-zone-deepest-places.md): kazı ve dolgu bölgeleri.

## Dayanak

- **Pix4D, “Menu View > Volumes > Sidebar > Objects - PIX4Dmapper”**
  ([belge](https://support.pix4d.com/hc/en-us/articles/208652926), 9 Kasım 2026'da okundu). Hacmin tabanı
  (Base Surface):
  - Triangulated: bütün köşeleri bağlar, hacmi tabanın üstünde ve altında üçgenler;
  - Fit Plane: köşelerin hepsi tabana en az uzaklıkta olacak düzlem;
  - Align with Average Altitude, Lowest Point, Highest Point: XY düzlemine paralel, köşelerin ortalama, en alçak ya da en
    yüksek kotunda;
  - Custom Altitude: XY düzlemine paralel, verilen kotta.
  - Cut Volume tabanın üstündeki, Fill Volume altındaki hacimdir; Total Volume = Cut + Fill.
- **Pix4D, “Which Base Surface is recommended for the Volume Calculation?”**
  ([belge](https://support.pix4d.com/hc/en-us/articles/217114246), 9 Kasım 2026'da okundu):
  - sınırın tamamı görünüyorsa, özellikle çevre düz değilse üçgenlenmiş taban;
  - sınır kısmen gizli, arazi oldukça düzse en alçak nokta;
  - sınır hiç görünmüyor, taban düz ve kotu biliniyorsa verilen kot.
- **KentOS bugün** (ADR 1600, 1840): iki yüzeyin farkı ve bölgeleri vardı. Tek yüzeyde bir yığının kendi sınırından
  çizilen tabana göre hacmi yoktu.

## Karar

1. **`road.terrains.stockpile` v1** (salt okunur; Harita + Yol): girdi yüzey (kimliği ya da adı), sınır (ADR 0920'nin
   kapalı nesnesi: düz kenarlı, tek parça, alanın delikleri dışarıda) ve taban:
   - `lowest`, `highest`, `average`: sınırın dış köşelerinin yüzeydeki kotlarının en alçağında, en yükseğinde ya da
     ortalamasında yatay düzlem;
   - `fit`: köşelere oturan düzlem. Pix4D “en az uzaklık” der, dik mi düşey mi söylemez; KentOS düşey farkların
     kareleri toplamını en küçükler (köşelerin ortalamasına göre merkezlenmiş normal denklemler);
   - `elevation`: verilen kotta yatay düzlem.
   - Üçgenlenmiş taban yapılmadı (Sonra).
2. **Hesap** (`kentos_road::stockpile`):
   - köşe kotları yüzeyin üçgeninin düzleminden;
   - yüzeyin her üçgeni sınırın her halkasını kırpar (Sutherland–Hodgman, pencere üçgen). İç bükey halkada pencere
     kenarı boyunca çift kenarlar kalır, alan katmazlar;
   - parça yüzey ile taban farkının sıfır olduğu doğruda bölünür; doğrusal fark parçanın alanı ve birinci momentleriyle
     kesin tümlenir;
   - dış halka saat yönünün tersine, delikler saat yönünde döner: deliğin parçaları çıkarılır;
   - koordinatlar sınırın ilk köşesinden alınır: büyük harita koordinatları basamak kaybetmez.
3. **Cevap:** taban düzlemi `z = a·x + b·y + c` (proje koordinatlarında), köşe kotları, sınırın alanı, yüzeyin örttüğü ve
   örtmediği alan, kazı (tabanın üstü, yığın), dolgu (tabanın altı), toplam (Pix4D'nin: kazı + dolgu), kazı ve dolgu
   alanları, notlar.
4. **Retler:** `invalid_stockpile` (yeni kod):
   - bilinmeyen taban;
   - kotla tabanda kot yok;
   - köşeden çizilen tabanda yüzeyin dışında kalan köşe (sırası ve yeriyle);
   - eğimli düzlemde köşeler bir doğru üstünde.

   Öbürleri ADR 0920'ninkiler: `not_a_road_project`, `unknown_terrain`, `terrain_unbuildable`, `entity_not_found`,
   `invalid_coverage`. **Uyarı** `terrain_no_data`: yüzey sınırın tamamını örtmüyor; orada hacim ölçülmez.
5. **Masaüstü: Stok hacmi…** (`terrain.stockpile`):
   - yer Arazi sekmesi › Analiz, Yüzey analizi'nin yanında;
   - simge `stockpileVolume`. Seçenek sayfası `.run/gui-evidence/stok-hacmi-3140/secenekler.png`, A seçildi;
   - Yüzey ve Sınır bir satırda: seçili kapalı nesne, yoksa çizimin ilki;
   - Taban satırı: En alçak köşe, En yüksek köşe, Ortalama, Eğimli düzlem (ilk değer), Kot. Kot seçilince kutu köşelerin
     en alçak kotuyla (cm) başlar;
   - özet: kazı, dolgu, toplam; alanlar; taban (kot ya da eğim ve ilk köşedeki kotu); köşe kotlarının aralığı; uyarı ve
     notlar. Çizim değişmez.
6. Hacim **geometriktir**: kabarma/sıkışma, malzeme ve maliyet yoktur (ADR 1600'ün katsayıları iki yüzey için).

## Kanıt

- **Çekirdek** (`stockpile.rs`, 3 birim testi):
  - düzlem üstündeki köşelere oturan düzlem o düzlemdir; doğru üstündeki köşelerde ret; yatay tabanlar;
  - iç bükey L'nin kırpılmasında çift kenarlar alan katmaz;
  - 487 600, 4 420 300 koordinatlarında 20 × 20 m tabanlı, 6 m yüksek piramit: 100'deki tabana göre 800 m³ (tabanın
    alanı × yükseklik / 3), 103'te kazı 100, dolgu 500 m³; ortadaki deliği çıkarır.
- **Ortak durumlar** (`scripts/fixtures/road_terrain_stockpile_cases.py`, uv ile `triangle`; KentOS kodu yok):
  - yüzey 4 m'lik sarsılmış ızgaranın 256 noktası (doğuya %1, kuzeye %0,5 eğimli zemin üstünde 6 m'lik yığın);
  - başvuru köşe kotlarını, taban düzlemini, kırpmayı ve tümlemeyi kesin kesirlerle kendisi yapar;
  - durumlar: beşgende beş taban; her yerin altında ve üstünde kot; ad ile yüzey; doğrulama ve yazma okur; iç bükey L
    (kapalı çoklu çizgi); delikli alan; yüzeyi aşan sınır (uyarı); retler (dışarıdaki köşe, bilinmeyen taban, kotsuz kot
    tabanı, proje türü, olmayan ve kurulamayan yüzey, olmayan sınır, açık çoklu çizgi, yaylı kenar, doğru üstündeki
    köşeler);
  - 25 durum. Yetenek testi her kodu bir durumda görür. Rust ve Python geçer;
  - eğimli taban yığının dışındaki zemin düzlemini bulur (%1,12, ilk köşede 100,220 m). Yığın 1227,1 m³: çözümsel koni
    ve tepe 1234,3 m³; 4 m'lik üçgen ağı sivri tepeyi keser.
- **MCP** (`crates/native/mcp/tests/stockpile_volume.rs`):
  - dört durumun hacimleri, alanları ve düzlemi durumlarınki (a ve b 10⁻⁹, c ile hacimler 10⁻⁶ içinde);
  - yüzeyi aşan sınırda uyarı; dışarıdaki köşede `invalid_stockpile`.
- **Masaüstü** (`stockpile_window_tests.rs`):
  - seçili beşgen sınır olur;
  - eğimli düzlem, en alçak köşe ve 101,5 kotu durumlarınki; kot kutusu köşelerin en alçağıyla başlar; L durumunki;
  - çizim değişmez; Harita ve Yol sunar.

  Çapraz denetimler (katalog, ribbon, geçmiş, ayarlar) geçer. Resimler `.run/gui-evidence/stok-hacmi-3140/`.
- **Manifest:** `terrain.stockpile` `query` düzeyinde, `road.terrains.stockpile` ile; 1031 satır.

## Sonra

- Üçgenlenmiş taban (köşelerden üçgenlenmiş yüzey, Pix4D'nin önerdiği).
- Tabanın ve hacmin çizimde gösterimi; CSV.
- Hacmin hata tahmini (yüzeyin nokta yoğunluğundan).
- Malzeme bölgesi; kesit yöntemiyle hacim (MAP-15'in öbür maddeleri).
- Web (TODOS §0.2).
