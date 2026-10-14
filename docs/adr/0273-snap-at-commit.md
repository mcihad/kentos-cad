# ADR 0273: Kenet basma anında yeniden alınır

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-09`: “Pointer commit anında yeniden snap hesaplama
  kuralını koru; büyük çizim ve hızlı fare hareketinde eski aday commit edilmesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (kenet deposu; CLAUDE.md §4.7: “Kenet pointerdown/up'ta yeniden hesaplanır”), 0271 (Tab ile adaylar).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Kural yazılıydı (CLAUDE.md §4.7) ve masaüstü basma ile bırakmada kenedi yeniden alıyordu (`pointer_at`), ama sınanmamıştı: hareketsiz
basma (hızlı farede son hareket olayı başka yerdedir), son hareketle basma arasında değişen çizim (başkasının değişikliği, betik, geri
alma) ve büyük çizim. ADR 0271'in Tab dolaşması yeni bir açık getirmişti: aday listesi imleç kımıldamadıkça tutuluyordu; arada çizim
değişirse eski listenin adayı yazılabilirdi.

## Karar

1. **Basma ve bırakma kenedi kendi yerinde ve o anki çizimde alır:** son hareketin kenedi hiçbir zaman yazılmaz. Depo basmadan önce
   belgenin günlüğüyle eşitlenir (`Spatial::sync`); kaldırılan nesnenin noktası kenet olmaz.
2. **Tab'ın adayları çizimin sürümüne bağlıdır:** dolaşma adayları bulunduğu sürümü tutar (`SnapCycle::revision`); çizim değişince
   (geri alma, başkasının değişikliği, betik) dolaşma biter ve kenet yeniden alınır. İmlecin kımıldaması (0,75 px) ve tıklamanın
   bırakılması da dolaşmayı bitirir (ADR 0271 §2).
3. Büyük çizimde (20 000 nesne) aynı kural: hızlı hareketin ardından gelen uzak basma kendi yerinin kenedini alır.

## Sonuçlar

- Hızlı fare, değişen çizim ve Tab'ın eski listesi yanlış noktayı yazamaz.

## Doğrulama

- `cargo test -p kentos-desktop snap_commit_tests` (hareketsiz basma, kaldırılan nesnenin ucu, değişen çizimde Tab'ın adayları, 20 000
  nesneli çizimde 100 m ötedeki basma); masaüstü izlerinin hepsi (her tıklama basmada ve bırakmada kenedi yeniden alır).
