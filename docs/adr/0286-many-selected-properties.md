# ADR 0286: Büyük seçimde Öznitelikler

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-22`: “Büyük seçimde property paneli “çeşitli”
  değerleri, toplu değişikliği ve kilitli nesne sonuçlarını gecikmeden ve açık biçimde sunsun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0063 ve 0066 (Öznitelikler), 0120 (değişmeyen karede panel yeniden kurulmaz), 0278 (kilitli katman kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Birden çok nesne seçiliyken panel ortak özellikleri ve toplamları gösteriyordu; seçimde kilitli katmanda tek bir nesne olsa bütün panel
düzenlenemez oluyor, Katman satırı yalnız “Kilitli katman içeriyor” diyordu. Öznitelikler (alan değerleri) çoklu seçimde hiç
görünmüyordu: toplu öznitelik değişikliği yoktu. Büyük seçimde panelin kurulma süresi ölçülmemişti.

## Karar

1. **Kilitliler atlanır, sayılır:** ADR 0278'in kuralı panelde de geçerlidir. Seçimde kilitli katmandakiler varken öbürleri düzenlenir:
   küme komutları (`cad.entities.set`: Katman, Renk, Kalınlık, Çizgi tipi, ölçeği, Saydamlık, Sembol, öznitelikler) bütün seçimi alır
   ve kilitlileri `layer_locked` uyarısıyla sayar; geometri ve yazı düzenlemeleri (`cad.entities.edit`: Kot, yazı, kılavuz, ölçü satırları)
   yalnız kilitsizlere gider. Ortak özellikler'in ilk satırı uyarı renginde kaçının kilitli olduğunu söyler (“1 nesne kilitli katmanda:
   atlanır.”); hepsi kilitliyse satırlar yalnız gösterir.
2. **Katman satırı:** ortak katmanın yolu, değilse “Çeşitli”.
3. **Ortak öznitelikler:** seçimdeki her öznitelik anahtarı (ilk görüldüğü sırayla, en çok 200), her nesnede aynı değerse değeri, değilse
   “Çeşitli (k/n nesnede)” (hepsinde aynı ama bazılarında yoksa değeriyle); hücreye yazılan değer bütün seçime `cad.entities.set` ile
   tek adımda (“Değiştir”) yazılır.
4. **Süre:** panel seçim değişince bir kez kurulur (ADR 0120); 100 000 nesnelik, iki öznitelikli seçimde 48 ms (Apple M5, release),
   bütçe 100 ms (`a_hundred_thousand_objects_build_the_panel_quickly`). Katmanların kilidi seçimdeki her katman için bir kez sorulur.

## Sonuçlar

- Karışık seçimde Öznitelikler'den yapılan her değişiklik kilitsizlere yazılır; ne atlandığı panelde ve günlükte söylenir.

## Doğrulama

- `cargo test -p kentos-desktop properties::` (karışık seçim, ortak öznitelikler, Kot'un kilitsizlere yazılması);
  `cargo test --release -p kentos-desktop a_hundred_thousand_objects_build_the_panel_quickly -- --ignored --nocapture`.
- Resimler: `KENTOS_SHOTS_ONLY=oznitelik-coklu,oznitelik-ortak cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
