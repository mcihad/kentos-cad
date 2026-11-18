# ADR 8127: Kaplama mahalini örter (`IfcRelCoversSpaces`)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B9 (BIM-09'un kalanı: "kaplamanın mahale
  bağı"). Bağlam: ADR [8070](8070-missing-building-elements.md) §10 (K47: kaplama döşemenin bir rolüdür, mahalden türer
  ya da elle çizilir; "mahale bağ sonraki adım").
- **Dosya, komut, `.kcad`:** değişmedi. IFC dosyasına `IfcRelCoversSpaces` eklendi.

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| buildingSMART IFC 4.3, `IfcRelCoversSpaces` | Bir mahali ona bakan ya da ona atanan bir ya da daha çok kaplamaya bağlar: `RelatingSpace`, `RelatedCoverings` | Kaplama kendi katında içerilir, mahaline bu ilişkiyle bağlanır |
| buildingSMART IFC 4.3, `IfcCovering` | Kaplamanın mahale bağı `IfcRelCoversSpaces`, elemana bağı `IfcRelCoversBldgElements`; hangisinin birincil olduğu görünüm tanımına göre | Mahalden türeyen kaplama mahalinindir; elle çizilen, iç noktasının durduğu mahalin |
| EK-6 Tablo 6.20 dipnotu (ADR 8070 §6) | Kaplamanın malzemesi ve kalınlığı ya mahalin setinde ya da `IfcCovering` ile | İki yol aynı dosyada çelişmez: mahalin seti kalır, kaplama ona bağlanır |

## §2. Karar

- **K293 — Mahalin kaplamaları:** mahale bağlı kaplama (sınırı mahalin halkaları, ADR 0771 §4) o mahalindir. Elle çizilen
  kaplama, kendi katındaki mahallerden iç noktasının (gövdesinin ilk dış çizgisinin, `label_at`) içinde durduğu mahalin
  kaplamasıdır; hiçbir mahalde değilse bağlanmaz (bina dışındaki bir zemin kaplaması gibi). Her mahal için bir
  `IfcRelCoversSpaces`; kaplama katında içerilmeye devam eder.

## §3. Uygulama ve sınamalar

- `crates/shared/ifc/src/building.rs` (`Building::covers`, `IFCRELCOVERSSPACES`), `arch.rs` (mahalin kaplamaları).
- `arch_ifc::a_ceiling_from_a_room_and_a_floor_covering_are_ifc_coverings`: mahale bağlı asma tavan o mahali örter, bina
  dışında çizilen zemin kaplaması hiçbirini; dosyada bir `IfcRelCoversSpaces`.
- **BIM-09'un öbür kalanları** ayrı dilimlerdir: giydirme cephenin panel yerine kapısı ve penceresi, eğimli eleman,
  rampa ve merdiven korkuluğunun kesit gövdesi. Duvar kaplamasının kalınlığı ADR 8076 K57 ve 8124 K281 ile yapıldı.
