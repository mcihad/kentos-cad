# ADR 8123: Emsal zonu çatının altında; zonlar görüntüleyicide ayrı ve kendiliğinden kapalı

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-08 (emsal zonları), BIM-06;
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12. Sahibin bildirimi: kendi “Yeni proje” çiziminin 3B görünümünde çatıda
  projede olmayan, yarı saydam kocaman bir küp vardı. Kesit alınınca köşelerden çapraz yarı saydam yüzeyler görünüyordu.
- **Bağlam:** ADR [8069](8069-room-zone-far-model.md) §2 karar 6: emsal zonunun yüksekliği katın en yüksek mahalinin
  tepesidir. ADR [8071](8071-geometry-accuracy.md) §11 karar 34: tavanı başka yoldan bilinmeyen mahal kendi katındaki
  çatının alt yüzüne kadar yükselir. Bunlarla birlikte [8119](8119-building-3d-in-the-model-viewer.md) (3B görünüm
  dosyanın modelini gösterir) ve [8044](8044-roof-and-roof-plan.md) §8 (oluk, iniş borusu) dayanaktır.
- **Web, `.kcad`, komut:** değişmedi. IFC dosyasında zonun gövdesi değişti.

## §1. Bulgu (sahibin çizimi, 18 Kasım 2026)

Çizimin modeli bellekte yazıldı. Gövdeler boyutlarına göre sıralandı ve resimlendi (`scene3d_quality_tests::probe_bodies`,
`KENTOS_PROBE_KCAD`):

- **Küp, zemin katın emsal zonuydu:** 12,50 × 10,40 m'lik düz bir prizma, ±0,00'dan 4,72 m'ye. Duvarlar 3,00 m'de
  bitiyor, kırma çatı 3,00'den 4,97 m'ye çıkıyor. Mahaller tavansız olduğu için çatının alt yüzüne yükseliyor (karar 34).
  En yüksek mahal mahyanın altında 4,72 m'ye varıyor. Zon “katın en yüksek mahali” kuralıyla bu yükseklikte düz bir
  kutu oluyor ve çatının yamaçlarından dışarı taşıyordu. Zon dosyada vardır ama çizimde ayrıca çizilmiş bir nesne
  değildir; sahibin “projede olmayan” dediği budur. Eski 3B görünüm zonları göstermiyordu, ADR 8119'dan beri gösteriyordu.
- **Kesitteki çapraz yüzeyler mahallerin tavanlarıdır:** tavansız mahaller kırma çatının alt yüzüne göre kesilmiştir.
  Kesilince bu eğik tavanlar köşelerden mahyaya kalkan yarı saydam yüzeyler olarak görünür. Bu EK-9 satır 14'ün
  (mahallerin üst sınırları doğru) bilinçli karşılığıdır (karar 34) ve kalır.
- **Köşelerden yere inen ince turuncu çizgiler çatının iniş borularıdır:** çatıda oluk ve iniş borusu tanımlı (ADR 8044 §8).
  Bunlar çizimdedir.

## §2. Karar

- **K269 — Zon çatının altında:** bir katın mahalleri bir çatının alt yüzüne yükseliyorsa o katın emsal zonu da aynı
  yüzlerle kesilir (`under_roof::pieces`, mahallerinki gibi). Gövdesi tabanından alt yüze kapalı parçalardır, tepesi alt
  yüzün en yükseğidir. Zonun tabanının bir kısmı hiçbir yüzün altında değilse ya da bir parçasında delik varsa eskisi
  gibi en yüksek mahaline kadar prizma kalır. Zon böylece mahallerini sarar ama çatının dışına çıkmaz.
- **K270 — Görüntüleyicide zon ayrıdır:** `IfcSpatialZone` artık mahal değil, kendi kategorisidir (`Category::Zone`,
  parça “Zon”, soluk yeşil, saydam). Kendiliğinden gizlidir. Şeritteki “Zonlar” anahtarı (yalnız modelde zon varken)
  gösterir. Zon gövde sayılmaz (durum çubuğundaki “gövde” sayısına girmez). Tıklama mahallerdeki gibi içinden geçer.
  Ağaçta katının altında durur, katla gizlenir (ADR 8119 K250).
- **K271 — Mahallerin eğik tavanı kalır:** karar 34 değişmez. Kesitteki eğik yüzeyler mahallerin doğru üst sınırıdır.

## §3. Uygulama

- `crates/shared/ifc/src/arch.rs`: katın çatı yüzleri mahallerden toplanır (`storey_roofs`); emsal zonu onlarla kesilir.
  `crates/shared/ifc/src/building.rs`: `Zone::{body, shapes}`; yazıcı zonu `shapes` ile yazar.
- `crates/shared/model-view`: `Category::Zone`, `parts::ZONE`, `View::zones`; sayımda zon yok.
  `crates/shared/ifc/src/read/view.rs`: `IfcSpatialZone` → `Zone`.
- `crates/model-view-ui`: `Message::Zones`, şeritte “Zonlar”. Masaüstünün izleri: “Zonlar”.

## §4. Sınamalar

- `roof_sections::a_room_with_no_slab_over_it_rises_to_the_roof`: kesit binasının teras döşemesiz, kırma çatılı 1. katında
  emsal zonları kesilmiştir. Hacimleri çatının alt zarfının zonun tabanı üstünde 1 200 × 1 200 ızgarada tümlenmesine
  %0,5 içinde eşittir, aynı yükseklikteki kutudan küçüktür, parçaları kapalıdır. Zemin katın zonları prizmadır.
- `scripts/fixtures/curved_ifc_check.py` (IfcOpenShell): `cati-alti-mahal.ifc`'nin 1. kat zonları kapalı, hiçbir noktası
  çatının alt yüzünün üstünde değil, hacimleri ızgaranınki. Eski dosyada bu denetim düşer: zonlarda alt yüzün üstünde 16
  ve 4 nokta (7,44 m'ye kadar), hacimler 317,12'ye karşı 248,98 ve 108,86'ya karşı 91,47 m³.
- Fikstür: `fixtures/arch/v1/ifc/cati-alti-mahal.ifc` bugünkü yazıcıyla yeniden yazıldı (zonların parçaları; sınama çatıyı
  her koşuda yeniden çizdiği için çatının kimliği de değişti). Başvuruları IfcOpenShell'le yeniden üretildi:
  `fixtures/ifc/v1/okuma/cati-alti-mahal.json` (zon hacimleri ve tepesi, çatının kimliği, kayıt numaraları) ve
  `fixtures/commands/v1/cad.ifc.inspect.json`. `katmanli-cati.ifc` anlamca değişmediği için yazılmadı.
- Görüntüleyici çekirdeği ve IFC okuma sınamaları; masaüstü 3B, görüntüleyici ve altlık sınamaları.
- Resimler: sahibin çiziminde düzeltmeden sonra kuzeydoğudan, 2,00 ve 3,60 m kesitleri (`.run/gui-evidence/zon-8123/`).
  Önceki hâlin resimleri bu oturumun bulgusunda (§1) incelendi, ayrıca saklanmadı.
