# 3760 — Hata elipslerinin etiketi

Tarih: 2026-11-09. plan/map-cad.md MAP-05, yedinci tur. Durum: taslak. ADR 2960 ve 3360'ın “Sonra”sındaki “elipsin
etiketi (yarı eksenler mm): yazı nesnesinin boyu ve yeri için ölçek kuralıyla”.

İlgili:
- [ADR 2960](2960-network-error-ellipses-drawn.md): hata elipslerinin çizimde gösterimi;
- [ADR 3360](3360-network-height-bars-drawn.md): kotun yarı aralığının çubukla gösterimi;
- [ADR 0905](0905-road-corridors.md) §6: şevli planın kotları (kâğıttaki yazı boyu × ölçek).

## Dayanak

- **Trimble Business Center**, [Network Adjustment Options](https://help.fieldsystems.trimble.com/tbc/3338.htm)
  (9 Kasım 2026'da okundu): elipsler grafik görünümlerde noktaların hatalarının büyüklüğünü ve yönünü gösterir; hata
  büyüdükçe elips büyür. Sayısal değerler raporda okunur, belge elipsin etiketinden söz etmez.
- **KentOS bugün** (ADR 2960, 3360):
  - elipsler büyütülerek çizilir (1000 kat: 1 cm, 10 m). Gerçek boylar elipsin özniteliğindedir;
  - paftada okunan çizimde, kâğıtta milimetreleri söyleyen bir yazı yoktu. Okumak için nesnenin özniteliğine bakmak
    gerekiyordu.
- **Yazı boyu:** KentOS'un çizdiği açıklamalar kâğıttaki boyla yazılır, çizimde bu boy × ölçeğin paydası / 1000'dir
  (ADR 0905, şevli planın kotları 2,5 mm).

## Karar

1. **`labels`** (`map.survey.edit network`'ün `ellipses`'ine ek alan; komut sürümü değişmez): her elipsin yanına bir
   yazı nesnesi yazılır, elipsle aynı geri alma adımında ve aynı dayanan kaydında (`kb-N`):
   - metin “a 1.4 · b 1.1 mm”: yarı eksenler milimetrenin onda biriyle. `heights` ile “· h ±2.2 mm” de eklenir;
   - boyu `labelHeight` (kâğıtta mm; verilmezse 2,5) × projenin ölçek paydası / 1000. 1/1000'de 2,5 m olur;
   - yeri: sol ortası, noktanın yerinden doğuya büyütülmüş büyük yarı eksen artı bir yazı boyu. Dönük hiçbir elips
     bundan doğuya uzanmaz;
   - öznitelikleri: İşaret, Dengeleme. Elipsle birlikte nokta yeniden üretilince güncel değildir.
2. **Cevap:** `labels`, yazılan etiket sayısı.
3. **Ret:** `labelHeight` 0'dan büyük olmalıdır, değilse `invalid_survey_value` (`operation.ellipses.labelHeight`).
   Büyütmenin retlerinden sonra, katmanınkilerden önce gelir.
4. **Masaüstü** (Ağ dengelemesi): Hata elipsleri satırında **Etiket** kutusu. Kabul iletisi yazılan etiketleri sayar.
5. **Kapsam işareti:** ADR 2960'taki gibi **Harita**; etiketler CBS'de yazı olarak okunur.

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/map_survey_network_cases.py`, `map_survey_cases.py`; numpy, scipy ve PROJ,
  KentOS kodu yok; etiketin metni, yeri ve boyu betikte):
  - elipsler, çubuklar ve etiketlerle kabul: üç etiket, 1/1000'de 2,5 m, yarı eksenler ve kot yarı aralığı;
  - 2000 kat, 5 mm etiket: yalnız yarı eksenler;
  - etiket yüksekliği 0 reddedilir;
  - `map.survey.edit` 121 durum (118'den). Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/network_ellipses.rs`, `the_ellipses_labels_are_the_case_s_over_mcp`): durumun
  arşivi `.kcad` olarak açılır; kabul üç elips ve üç etiket der; kaydedilip okunan çizimdeki her etiketin metni, yeri,
  boyu ve öznitelikleri durumunkidir.
- **Masaüstü** (`network_ellipses_tests::etiket_writes_each_ellipse_s_semi_axes_beside_it`, sapmaları 30° dönük
  durum, 2000 kat): Etiket işaretli kabul üç etiket yazar. Metin çizilen elipsin yarı eksenlerinden okunur, boy
  1/1000'de 2,5 m, yer elipsin doğusunda; dayanan kaydı elipslerle altı nesnedir; ileti “3 etiket yazıldı” der. Ağ
  dengelemesi'nin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/elips-etiketi-3760/`
  (pencere ve N1'in yakını).

## Sonra

- Etiketin komşu elipsle ya da başka bir yazıyla çakışınca kaydırılması (bugün yer kuralı tek).
- Robust kestirim; seyrek çözüm; ağırlıklı benzerlik (MAP-05'in kalanı).
- Web (TODOS §0.2).
