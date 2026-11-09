# 2940 — Detay eşleştirmede büyük aday kümesi için ızgara dizini

Tarih: 2026-11-09. plan/map-cad.md MAP-04, beşinci tur. Durum: taslak. ADR 1760'ın, 2140'ın ve 2540'ın “Sonra”sındaki
“büyük aday kümesinde ızgara dizini (bugün tüm çiftler karşılaştırılır)”.

İlgili:
- [ADR 1760](1760-asset-candidate-reconciliation.md): adayların eşleştirilmesi (yarıçap, Hausdorff, gruplar);
- [ADR 2140](2140-area-candidates-by-overlap.md): alan adayları, örtüşme oranı;
- [ADR 2540](2540-reconcile-field-rules-and-entry.md): alan kuralları.

## Dayanak

- **CLAUDE.md §6.2:** uzamsal sorgu dizinden geçer; optimizasyon aynı veri ve aynı sonuçla ölçülür.
- **KentOS bugün** (ADR 1760, 2140):
  - her aday çifti karşılaştırılıyordu: n adayda n(n − 1)/2 deneme;
  - her grubun “var olan” varlığı için çizimin bütün nesneleri baştan geziliyordu: grup sayısı × nesne sayısı;
  - ölçüm (aşağıda): 10 000 kümede (22 000 aday) bir plan 38 s.
- **Doğruluk koşulu:** bir çiftin yakın olması kutularının buluşmasını gerektirir:
  - iki nokta yarıçap içindeyse yarıçap kadar genişletilmiş kutuları buluşur;
  - iki çizginin Hausdorff uzaklığı yarıçap içindeyse her birinin köşeleri öbürünün yarıçap kadar genişletilmiş
    kutusundadır;
  - iki alanın örtüşme oranı sıfırdan büyükse ortak alanları vardır, kutuları buluşur.

  Bu yüzden yalnız kutuları buluşan çiftleri denemek, bütün çiftleri denemekle aynı cevabı verir.

## Karar

1. **Kutu ızgarası** (`kentos_native_application::box_grid`):
   - her kutu kapladığı bütün hücrelere girer;
   - bir çift, iki kutunun kesişiminin sol alt köşesini tutan hücrede bir kez bildirilir; iki kutu da o hücrededir;
   - hücre, kutuların yayıldığı alanda ortalama bir kutu düşecek karenin kenarıdır, kutuların ortalama boyundan küçük
     değildir;
   - bir sorgunun buluştuğu kutular sırayla verilir; sorgu bütün hücrelerden büyükse kutular tek tek okunur.
2. **Eşleştirme** (`map.features.reconcile`, sözleşme değişmez):
   - yalnız uzanımları buluşan uygun aday çiftleri denenir. Uzanım: noktanın ve çizginin kutusu yarıçap kadar
     genişletilmiş (iki yanda da, payla), alanın kendi kutusu;
   - gruplar birleşim-bul ile kurulur. Bir kümenin kökü en küçük sırasıdır, çiftlerin sırası grupları değiştirmez;
   - bir grubun var olan varlığı, sınıf katmanının nokta, çizgi ve alan nesneleri arasında kutusu temsilcilerinin
     uzanımıyla buluşanlarda aranır. Katmanın listesi ve ızgarası bir kez yapılır. Çizimin sırasındaki ilk yakın nesne
     seçilir, eskisi gibi.
3. **Kapsam işareti:** ADR 1760'taki gibi **Harita** (pencere ve yazma); komut Python ve MCP'de.

## Ölçüm

macOS, Apple M5, `cargo test` debug derlemesi, `reconcile_large::timing`; tek koşu, 9 Kasım 2026. Veri: ortak durumların
çizimi ve doğusunda kümeler. Her kümede LiDAR ve ortofoto direği, beşte birinde üçüncü aday, onda birinde yanında
var olan direk; kümeler 5 m arayla. Plan süresi:

| Küme (aday) | Bütün çiftler (önceki) | Izgara |
| --- | --- | --- |
| 2 000 (4 400) | 1,09 s | 0,054 s |
| 10 000 (22 000) | 38,2 s | 0,246 s |
| 40 000 (88 000) | ölçülmedi (saatler) | 1,02 s |

Süre artık aday sayısıyla doğrusal büyüyor. İki kod aynı grup sayısını verdi. Sürüm derlemesinde ölçülmedi.

## Kanıt

- **Izgara** (`box_grid` birim testleri): belirlenimci rastgele 1500 kutuda (uzun kutularla da) çiftler bütün çiftlerin
  buluşanlarıyla aynı, her biri bir kez; aynı yerdeki noktalar, boş küme, değen kutular. Sorgu küçük ve büyük kutuda
  bütün kutuların buluşanlarıyla aynı.
- **Ortak durumlar:** `map.features.reconcile`'ın 35 durumu aynen geçer.
- **Büyük küme** (`reconcile_large.rs`): 3000 küme (6600 aday): grup sayısı küme sayısı artı ortak durumunkiler, boylar
  2 ve 3, onda biri `existing`, gerisi `agreed`, ortak durumun grupları aynı.
- **MCP, Python, masaüstü:** cevap ve arayüz değişmedi.

## Sonra

- Ayrı inceleme kaydı; ~~yaylı aday~~ (ADR 3340); AI çıkarımı (MAP-04'ün kalanı).
- Sürüm derlemesinde ve gerçek bir çıkarımın adaylarıyla ölçüm.
- Web (TODOS §0.2).
