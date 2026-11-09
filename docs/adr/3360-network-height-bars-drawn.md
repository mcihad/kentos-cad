# 3360 — Ağın kot yarı aralığının çizimde çubukla gösterilmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-05, altıncı tur. Durum: taslak. ADR 2960'ın “Sonra”sındaki “kotun aralığının
çizimde gösterimi (düşey çubuk ya da etiket)”.

İlgili:
- [ADR 2560](2560-network-confidence-regions.md): noktaların güven bölgeleri (elips ve kotun yarı aralığı);
- [ADR 2960](2960-network-error-ellipses-drawn.md): hata elipslerinin çizimde gösterimi.

## Dayanak

- **Trimble Business Center, Network Adjustment Options** ([belge](https://help.fieldsystems.trimble.com/tbc/3338.htm),
  9 Kasım 2026'da okundu):
  - grafik görünüşlerde noktaların hatalarının büyüklüğü ve yönü resimle gösterilir;
  - sonuç sekmesindeki elips resimleri dengelemenin yatay ve düşey artıklarını birlikte gösterir.

  Belge düşey bileşenin hangi şekille çizildiğini söylemez.
- **ADR 2560:** her noktanın güven bölgesi yatay elips ve kotun yarı aralığıdır (√χ²₁ · σ yukarı), aynı güvende.
  ADR 2960 elipsi çizer; kotun yarı aralığı yalnız cevapta ve raporda kalıyordu. Haritada noktanın kot belirsizliği
  görünmüyordu.

## Karar

1. **`map.survey.edit network`'ün `ellipses.heights`'i** (ek alan; komut sürümü değişmez): elipslerle birlikte her
   noktanın kot yarı aralığı da çizilir.
2. **Çubuk:**
   - noktanın içinden geçen düşey bir çoklu çizgi;
   - yarı uzunluğu u, yarı aralık × elipsin büyütmesi (elipsle aynı ölçek, karşılaştırılabilir);
   - uçlarında u/5'lik çentikler (hata çubuğunun “I” biçimi): (x − u/5, y + u) → (x + u/5, y + u) → (x, y + u) →
     (x, y − u) → (x − u/5, y − u) → (x + u/5, y − u);
   - düşey, haritanın yukarısına. Kot bir yön değildir, yalnız büyüklüğü gösterilir.

   Bu biçim KentOS'un seçimidir; dayanaktaki yazılım şeklini söylemiyor.
3. **Öznitelikler:** İşaret, Dengeleme, Güven, Kot yarı aralığı (m) (dört basamak), Büyütme.
4. **Kayıt:** çubuklar elipslerle aynı geri alma adımında ve aynı dayanan kaydında (`Hata elipsleri (da-N)`) tutulur.
   Nokta yeniden üretilince onlar da güncel değildir.
5. **Cevap:** `ellipses` elips sayısı, `bars` çubuk sayısı (ek alan).
6. **Masaüstü** (Ağ dengelemesi › Hata elipsleri):
   - büyütmenin yanında **Kot çubuğu** kutusu, elipsler seçiliyken açılır;
   - kabul iletisi ikisini sayar: “3 hata elipsi ve 3 kot çubuğu 2000 kat büyütülerek çizildi.”
7. **Kapsam işareti:** ADR 2960'taki gibi **Harita**; çubuklar ve öznitelikleri CBS'de okunur.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_survey_network_cases.py`, `map_survey_cases.py` yazar; numpy, scipy,
  pyproj, QGIS'in Python'u ve PROJ verisiyle):
  - yeni durum: N1, N4 ve N5'in elipsleri ve kot çubukları 1000 kat; çubukların köşeleri ve öznitelikleri betiğin
    kuralıyla;
  - eski durumlar aynı;
  - `map.survey.edit` 118, `map.survey.network` 22 durum. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/network_height_bars.rs`): kabulün cevabı 3 elips ve 3 çubuk; kaydedilen dosyadaki
  çubuklar durumunkiler (köşeler 10⁻⁶ m içinde).
- **Masaüstü** (`network_ellipses_tests::kot_cubugu_draws_the_height_bars_with_the_ellipses`):
  - üç çubuk, uzunlukları yarı aralığın büyütülmüşünün iki katı;
  - dayanan kaydında altı nesne; ileti ikisini sayar;
  - resimler `.run/gui-evidence/kot-cubugu-3360/` (pencere ve çizim).

  Elips testleri ve çapraz denetimler geçer.

## Sonra

- Elipsin etiketi (yarı eksenler mm): yazı nesnesinin boyu ve yeri için ölçek kuralıyla.
- Robust kestirim; seyrek çözüm; ağırlıklı benzerlik (MAP-05'in kalanı).
- Web (TODOS §0.2).
