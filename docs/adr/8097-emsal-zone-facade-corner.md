# ADR 8097: Emsal zonlarının cephe köşesi — ortak duvarın ekseninden bölünür

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8079](8079-emsal-article-22-limits.md) §6 ("Emsal zonlarının cephe köşesi … köşeyi ara duvarın ekseninden bölmek
  kaldı").
- **Bağlam:** ADR [8069](8069-room-zone-far-model.md) (emsal zonları mahallerin konturundan, K41: zonların alanı ve
  açıklamalardaki paylar birlikte tablonun emsale esas alanı), [8030](8030-emsal-taks-zoning-badge-and-table.md)
  (mahal mahal brüt kontur, cephe köşesinin yarı yarıya bölünmesi).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-ğ-1 (brüt alan: dış duvar dış yüzü, ortak duvar ekseni); Dijital Proje
  Yönetmeliği EK-6 Tablo 6.60 (emsal zonları).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** değişmedi.

## §1. Sorun

Mahalin konturu dış yüzlere kadardır, öbür mahallerin eksen çizgisi konturu düşülür (ADR 8030). İki mahal arasındaki duvar
cephe duvarına girdiği yerde, duvarın ucunun cephe duvarının dış yarısında kalan parçası (ara duvar kalınlığı × dış
duvarın yarısı) iki konturda da kalır. Tablo bu köşeyi brüt alanda yarı yarıya böler; zonlar bölmüyordu: farklı sayılan
iki komşunun (daire ve merdiven evi gibi) zonlarında köşe iki kez vardı (ADR 8079'un deneme planında 4 × 0,075 × 0,125 =
0,0375 m²).

## §2. Karar

- **K142 — Köşe ortak duvardan bölünür** (`kentos_domain::zoning::corners`): iki konturun ortak parçası, köşenin
  ortasına en yakın iki net kontur noktasının orta dikmesinden bölünür (düz duvarda duvarın ekseni); her mahalin şekli
  (emsal zonlarının yapıldığı) kendi yanını tutar. Brüt alan değişmez (yarı yarıya bölme simetrik köşede eksenden bölmeyle
  aynıdır); zonun alanı artık mahallerinin brüt alanıdır.
- **Net konturu olmayan** mahalin köşesi bölünmez (eski davranış).

## §3. Uygulama

- `zoning/corners.rs` (`split`); `zoning_measured` köşeleri konturların çakışmasından alır, şekilleri böler. Vaziyet
  planının oturumu (şekillerin birleşimi) değişmez.

## §4. Sınamalar

- **Birim:** `a_shared_facade_corner_is_split_at_the_wall_between_the_rooms` (40 cm duvar, 50 cm cephe duvarı: 0,40 ×
  0,25 m'lik köşe x = 5'ten 0,05 + 0,05 m²).
- **Masaüstü** (`emsal_limits_tests`): zemin katın emsale dahil zonu iki dairenin 60 m²'lik brüt alanı (önceden 60,0375);
  zon ve adlandırılan pay tablonun emsale esas alanı (60 + 17,14375), köşe düzeltmesiz.

## §5. Arayüz

- Yeni pencere yok; IFC modelinin emsal zonlarının çokgenleri.

## §6. Sıradaki adımlar

- **Eğri ya da açılı ortak duvar:** orta dikme düz duvarın eksenidir; eğri duvarda köşe yaklaşık bölünür.
