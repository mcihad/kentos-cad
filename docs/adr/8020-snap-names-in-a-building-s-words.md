# 8020 — Kenet bir binanın diliyle

Tarih: 2026-11-08. Durum: kabul (M20, ADV-11; plan S11).

## Sorun

- **Kenet işareti duvarın hangi çizgisinde olduğunu söylemiyordu.** Yazı "{kenetin türü} · {nesnenin türü}" idi:
  "Orta nokta · duvar", "Uç nokta · duvar" (`snap_cycle.rs::snap_marker_note`).
- **Mimar için duvarın çizgileri ayrı şeylerdir:** yüz, eksen, çekirdeğin yüzü, katmanların arası. Mesafeyi hangisinden aldığı
  ölçünün anlamını değiştirir (net mesafe, aks mesafesi). Açıklığın ortası ile söve kenarı da öyle.
- **"duvar" küçük harfle yazılıyordu.** Nesne türünün adı cümle içinde kullanılan biçimdeydi.

## Araştırma (8 Kasım 2026)

- **Vectorworks yardımı:**
  - *SmartCursor cues*: imlecin yanında kenetin yerini adlandıran ipucu.
  - *Object snapping*: "Walls and slabs with components can have master snap points located on the component sides."
  - Plan ADV-11 "Mimari kenet türleri"ni bu ipuçlarıyla tarif ediyor: "Kenetin türü imlecin yanında yazı ve şekille görünür".
- **KentOS'ta kenet zaten doğru yere oturuyor.** Depo duvarı çizilen çizgileriyle kenetler (`geom::wall::pieces`; sembol
  parçaları hariç, ADR 8014). Eksik olan çizginin adıdır: `SnapHit` yalnız türü, noktayı ve nesneyi taşır.
- **Duvarın çizgileri tanımlı** (ADR 0340 §5, `geom::wall::boundaries`): 0 sol yüz, son sağ yüz, aradakiler bileşenlerin
  sınırları. Çekirdek önceliği 1 olan bileşendir (ADR 0340 §1). Öznitelikler duvarın yolunu "Taban çizgisi" diye adlandırıyor
  (Sabit kalan çizgi).

## Karar

- **Ad, kenetlenen noktadan bulunur** (`kentos_interaction::snap_names::name`). Depodaki şeklin geometrisinden okunur;
  kenet tam çizginin üstünde durduğu için 0,1 mm içindeki en yakın çizgi alınır.
- **Duvar** (`Shape::Wall`):
  - ilk ve son çizgi: **Duvar yüzü**;
  - iki bileşenin arası, biri çekirdekse: **Çekirdek yüzü**; değilse: **Katman sınırı**;
  - yüzlerin ortası: **Duvar ekseni**;
  - yol, hiçbirine denk gelmiyorsa: **Taban çizgisi**.
  - Önce yüz ve sınırlar sınanır: sola hizalı duvarın yolu sol yüzdedir ve "Duvar yüzü" denir.
- **Açıklık** (`Shape::Opening`, dört söve köşesi):
  - sövelerin ortalarının ortası: **Açıklık ortası**;
  - bir sövenin üstü: **Açıklık kenarı**.
  - Kanat ve yay adlandırılmaz; onlarda nesnenin türü kalır.
- **Kenet işareti:** "Orta nokta · Duvar yüzü", "Uç nokta · Açıklık kenarı". Adı bulunmayan noktada (kanat, yay, başka bir
  nesne) eskisi gibi nesnenin türü yazılır.
- **Kenetin kendisi değişmez:** aynı noktalar, aynı öncelik. Yalnız adı söylenir.

## Ölçüm

- **Gerçek pencere** (`fixtures/interaction/real/mimari-kenet.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Dikdörtgen duvarla oda ve Duvar ile bölme çizildi, sonra Duvar aracı çalıştırıldı.
  - Bölmenin yüzünün ortası: "Orta nokta · duvar" → "Orta nokta · Duvar yüzü".
  - Ucu: "Uç nokta · duvar" → "Uç nokta · Duvar yüzü".
  - Resim: `.run/gui-evidence/mimari-m20/m20-once-sonra.png`.
- **Testler:**
  - `snap_names::tests` (geometri):
    - sola hizalı 20 cm'lik duvarda yüzler ve eksen; ötelenmiş duvarda eksen ve taban çizgisi;
    - dört katmanlı duvarda çekirdeğin iki yüzü ve yalıtım ile sıvanın arası;
    - açıklığın ortası ve iki sövesi; kanadın yanı adsız.
  - `building_context_tests::the_snap_marker_says_a_wall_s_face_and_axis_in_a_building_s_words`: masaüstünün kenet yazısı,
    Dış duvar 25'in iki yüzü ve ekseni.

## Sonraki adımlar

- Kolonun yüzü ve ortası, merdivenin basamak burnu, mahalin sınırı.
- Kenetin türüne mimari tür eklemek: "yalnız duvar yüzü", "yalnız eksen" süzgeci (Vectorworks'ün bileşen kenet noktaları
  gibi). Bugün ad söylenir, süzgeç yoktur.
