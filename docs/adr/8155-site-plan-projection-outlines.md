# ADR 8155: Vaziyet planında çıkma izdüşümleri — PAİY 57/3

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8152](8152-projections-paiy-41.md) §6 ("Çıkmanın vaziyet planında izdüşümüyle gösterilmesi (57/3)"),
  [8032](8032-site-plan-parcel-setbacks-and-datum.md) §6.
- **Bağlam:** ADR 8032 (Parsel aracı: parsel, kenarlar, yapı yaklaşma sınırı ve vaziyet planının ölçüleri; ölçüler
  `Vaziyet` özniteliğiyle, araç her yazışta eskilerini kaldırıp yeniden çizer), 8152 (çıkmalar).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 57/3 ("çıkma izdüşümleri ve yapı yaklaşma mesafelerinin … hem mimari
  projede hem de aplikasyon projesinde … gösterilmesi zorunludur").
- **Sözleşme:** değişmedi (yeni komut yok; çizgiler `cad.entities.create` ile). `kentos_domain::Document::projections_for`.
  **`.kcad`:** değişmedi. **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Vaziyet planı yapı yaklaşma sınırını ve bahçe mesafelerini çiziyordu; 57/3'ün istediği çıkma izdüşümlerini çizmiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 57/3 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin) | Çıkma izdüşümleri ve yapı yaklaşma mesafeleri hem mimari projede hem aplikasyon projesinde gösterilir | Vaziyet planının ölçüleriyle çıkma izdüşümü (K399) |
| ADR 8032 §2 | Parsel aracının ölçüleri (bahçe mesafesi ölçüleri, ±0,00 işareti) `Vaziyet` özniteliğini taşır; araç yeniden çalışınca eskilerini kaldırır | İzdüşümler de `Vaziyet`'i taşır: aynı yolla yenilenir, sözleşmeye kimlik eklenmez (K399) |
| ADR 8152 | Çıkma: zemin katın üstündeki bir katta mahalin taban alanı izi dışındaki parçası | Çizilen izdüşüm o parçanın dış halkasıdır (K399) |

## §3. Karar

- **K399 — Çıkma izdüşümü:** Parsel aracı ölçülerle (Ö açıkken, varsayılan) her çıkmanın her parçasını kesik çizgili
  kapalı çizgi olarak etkin katmana çizer; çizgi `Vaziyet: Çıkma izdüşümü` özniteliğini ve "Çıkma izdüşümü" etiketini
  taşır, rengi katmanınki. Çıkmalar yazılmakta olan vaziyet planına göre okunur (`projections_for`: vaziyet planı
  ayarlara yazılmadan). Araç yeniden çalışınca eski izdüşümler eski ölçülerle birlikte kalkar; Ö kapalıyken çizilmez.
  Aracın sözü sayıyı söyler ("1 çıkma izdüşümü çizildi"). Aynı geri alma adımı.

## §4. Uygulama

- `crates/native/domain/src/projections.rs` (`projections_for`), `crates/native/interaction/src/map/parcel_tool.rs`
  (`draw_projections`).

## §5. Sınamalar

- **Masaüstü** (`projection_tests::parsel_draws_the_projections_outlines_with_the_site_plan`, 8152'nin sahnesi): Parsel
  balkonun taban alanı dışındaki parçasını (x 2–8, y −2–0) kesik çizgili kapalı çizgi olarak çizer, `Vaziyet` özniteliği
  "Çıkma izdüşümü"; araç yeniden çalışınca yine tek izdüşüm. Bitişik kenar, vaziyet planı, imar parseli ve ortak
  süzgeçlerle masaüstü sınamaları yeşil.
- **Resimler:** `.run/gui-evidence/cikma-izdusumu-8155/`: vaziyet planında yapı yaklaşma sınırı ve balkonun izdüşümü,
  açık ve koyu.

## §6. Kalanlar

- Aplikasyon projesi (57/4) KentOS'ta ayrı bir pafta değildir; izdüşüm aynı çizimdedir.
