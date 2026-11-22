# ADR 9157: İşlemin girdisi seçimin katmanını ve katmanın sayısını söyler

- **Durum:** kabul edildi (2026-11-23); adım VY-56. Plan: [plan/veri.md](../../plan/veri.md) VY-56.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 11. maddesinin
  kalanı (ADR 9144 §4): seçim varken girdinin satırında “Yol ekseni · 1 seçili / 1 kayıt”.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- İşlem penceresinin nesne girdisi okuduğunu tek satırda söyler (`kentos_processing::features`):
  - katmanda: “3 kapalı alan; “Parsel” katmanında”;
  - seçimde: “1 kapalı alan; seçili nesneler”. Seçimin hangi katmandan olduğu ve katmanın ne kadarı olduğu
    yazmıyordu.
- ADR 9144 seçimi aracın ilk nesne girdisine verir; kişi seçimin doğru katmandan geldiğini satırdan görebilmeli.

## §2 Karar

1. **Tek katmandan seçim.** Seçimin nesneleri tek katmandaysa satır şöyledir: “1 kapalı alan; seçili, “Parsel”
   katmanından (katmanda 1 nesne)”.
2. **Birden çok katman.** Seçim birden çok katmandaysa kaç katmandan geldiği yazar: “3 nesne (…); seçili nesneler, 2
   katmandan”.

## §3 Kanıt

- **Masaüstü** (`gis/processing/result_name_tests.rs`): parsel seçiliyken Tampon'un girdisi “seçili, “Parsel”
  katmanından (katmanda 1 nesne)” diyor.
- İşlem çekirdeği sınamaları ve masaüstü işlem sınamaları (130) geçiyor.
- Aynı turda ADR 9154'ün paylaşılan adres izi (`fixtures/interaction/v1/address-search.json`) yeni eşitlik sözüyle
  yenilendi (`scripts/fixtures/address_scene.py`, `--check` tutarlı). İz sınamaları (39) geçiyor.

## §4 Kalan

- ADR 9144 §4'ün öbür kalanları: türetilmiş sonuç adı, iş kaydının bütün alanları, CRS/iptal/bozuk geometri sınamaları.
- Web (TODOS.md §0.2).
