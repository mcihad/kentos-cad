# ADR 9131: Katman süzgeci veri katmanında kaynağın satırlarını sayar

- **Durum:** kabul edildi (2026-11-22); adım VY-30. Plan: [plan/veri.md](../../plan/veri.md) VY-30.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  19. maddesi (P1):
  - altı nokta görünürken Veri › Katman süzgeci › bakim › `bedel >= 100` önizlemesi “0 / 0” ve “hiçbir nesne
    görünmeyecek” dedi;
  - kaydedince haritada beklenen üç nokta kaldı ama bildirim yine “0 / 0 nesne görünür” dedi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** `cad.layers.filter` değişmedi.

## §1 Bağlam

- Haritaya uygulanan süzgeç doğruydu (ADR 9101 §6: veri katmanının süzgeci kaynağa sorgu olarak gider).
- Önizleme ve bildirim ise `layers_filter::count` ile katmanın çizimdeki nesnelerini sayıyordu. Veri katmanında bu
  nesneler yoktur.

## §2 Karar

1. **Önizleme**, veri katmanında süzgeci kaynağın satırları üzerinde sayar (`layers_filter::count_rows`).
   - Satırlar İstatistik'in okuduğu yerden gelir (`data_rows`, ADR 9130).
   - Okunurken “Kaynağın satırları okunuyor; sayı birazdan.” der, okununca kendiliğinden yeniden sayar.
   - Okunamazsa nedenini söyler.
2. **Bildirim** veri katmanında satır sayar: ““bakim” katmanının süzgeci yazıldı: 3 / 6 satır görünür.”
   - Satırlar henüz okunmadıysa “… satırlar kaynaktan okununca sayılır.” der.
   - Süzgeç kaldırılınca “… 6 satırın hepsi görünür.” der.
   - Çizim katmanının cümleleri değişmedi.

## §3 Kanıt

- **Masaüstü** (`gis/data_rows_tests.rs`), raporun bakım verisi: `bedel >= 100` yazılınca önizleme “3 / 6 nesne
  süzgeçten geçiyor”, Kaydet “3 / 6 satır görünür” diyor.
- Katman süzgecinin izi (`layer-filter`) ve İstatistik sınaması geçiyor.

## §4 Kalan

- Gerçek bir sıfır eşleşmenin ayrı sınaması (raporun kabulündeki 6 → 3/6 → 6/6 dizisinin son adımı; Süzgeci kaldır
  “6 satırın hepsi” diyor ama sınamaya alınmadı).
- Web (TODOS.md §0.2).
