# ADR 8145: Binanın kendi taşıyıcı sistemi — 28/1-d, 22/1-m, 5/8 ve 20/8-j blok blok

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8094](8094-structural-system-traditional-cut-stone.md) §6 ("Blok başına taşıyıcı sistem: bina programının binasında;
  bugün projenin bütün binalarının").
- **Bağlam:** ADR 8094 (taşıyıcı sistem, geleneksel kesme taş yığma; 22/1-m ve 28/1-d), [8095](8095-thick-stone-walls-out-of-emsal.md)
  (kalın taş duvarlar 5/8, 20/8-j), [8096](8096-sivas-22k-thick-walls-within-30.md) (Sivas 22/k),
  [8144](8144-zone-per-building.md) (binanın kendi bölgesi: aynı yol). Kurallar Python'dadır, Rust olguları verir (bim-plan
  K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 5/8, 20/8-j, 22/1-m, 28/1-d.
- **Sözleşme:** `ProgramBuilding.structure` (`cad.program.edit`'in `program`'ı; geleneksel öğeler yalnız kesme taş
  yığmada, `program.buildings[i].structure.traditional`'da reddedilir), `ArchSettings::structure_on`,
  `traditional_cut_stone_on`, `any_traditional_cut_stone`. `kentos.auditFacts` 1'in katlarına `structuralSystem`,
  `traditional`. `tr.kentos.mimari-mevzuat` 0.35.0. **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 31
  (`settings.arch.program.buildings[].structure`). **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Taşıyıcı sistem projenindi: bir parselde geleneksel kesme taş yığma bir blokla betonarme bir blok birlikte duruyorsa
bütün katlar tek beyanla okunuyordu. Taşlık, dış sofa ve eyvan (22/1-m) ve kalın taş duvarlar (5/8, 20/8-j) bütün
projede emsal harici sayılıyor, 28/1-d ve 22/1-m'nin "zemin dahil 3 katı geçmeyen" binası bütün projenin katlarıyla
sayılıyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 22/1-m, 28/1-d (8094) | Taşıyıcı sistemi kesme taş yığma, geleneksel mimari öğeler barındıran, zemin dahil 3 katı geçmeyen binalar | "Bina" programın binasıdır: kat sayısı binanınkidir (K371, K372) |
| PAİY 5/8, 20/8-j (8095) | Böyle binanın taşıyıcı taş duvarlarının 25/12,50 cm'yi aşan kısmı | Yalnız geleneksel binaların katlarındaki duvarlar okunur (K371) |
| ADR 8072 §1 | Bir katın binası `building_of_level` | Katın taşıyıcı sistemi binasınınki, yoksa projeninki (K371) |

## §3. Karar

- **K371 — Binanın taşıyıcı sistemi:** `ProgramBuilding.structure`: binanın kendi sistemi ve geleneksel öğeleri; yoksa
  projeninki. Bir katın sistemi binasınınkidir (`structure_on`); geleneksel kesme taş yığma katı
  `traditional_cut_stone_on`. Emsal ve TAKS tablosu taşlık, dış sofa ve eyvanı yalnız geleneksel binanın katlarında,
  kalın taş duvarları yalnız geleneksel binaların katlarındaki duvarlardan sayar. Bina programı › Binalar'da "Taşıyıcı
  sistem" (Projeninki ya da sistemler; kesme taş yığmada "Geleneksel mimari öğeler barındırıyor"). Mimari verisi 31.
- **K372 — Kurallar bina bina:** olguların katı binasının kendi sistemini taşır. `kat_yuksekligi` bir binası kendi
  bölgesini ya da sistemini söylüyorsa katları binalarına ayırır (28/1-d'nin kat sayısı binanınki). `emsal`'in 22/1-m'si
  her binanın mahallerini kendi kat sayısıyla okur (sınırların mahalleri binalara, alanları mahallerinden); satır
  birleşir.
- **K373 — Kalın taş duvarlar:** 5/8 ve 20/8-j'nin duvar toplamı bina bina ayrılmaz (olgular toplamı verir). Bir
  geleneksel bina varsa kat sayısı onunki; birden çok geleneksel binanın biri 3 katı aşıp öbürü aşmıyorsa bulgu uzman
  incelemesi (duvarlar bina bina ayrılmıyor), hepsi aşıyorsa kaldı.

## §4. Uygulama

- `crates/shared/contracts/src/arch/program.rs` (`ProgramBuilding.structure`, `has_building_structures`, denetim),
  `arch.rs` (`structure_on`, `traditional_cut_stone_on`, `any_traditional_cut_stone`); `crates/shared/kcad/src/lib.rs`,
  `decode.rs` (`ARCH_WITH_BUILDING_STRUCTURES`).
- `crates/native/domain/src/zoning.rs`, `zoning/masonry.rs` (`walls`'a katın geleneksel olup olmadığı).
- `crates/shared/extensions/src/audit_facts/building.rs` (`StoreyHolds.structural_system`, `traditional`),
  `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kat_yuksekligi.py`, `emsal.py` (`_binalar`, `_geleneksel_mi`).
- `apps/desktop/src/arch/program_window.rs` (`BuildingStructure`, `BuildingTraditional`), `program_view.rs`.

## §5. Sınamalar

- **Kurallar** (28/1-d, 22/1-m'den elle): kat yüksekliği 47 (1 yeni: A bloğu kendi beyanıyla geleneksel kesme taş yığma,
  iki katı 4,40 m 28/1-d'yle geçer; B bloğu projenin betonarmesi, 4,40 m'lik zemin katı 28/1-c'nin 4,00'ından kalır),
  emsal 41 (1 yeni: iki geleneksel blok, iki katlı A'nın taşlığı uzman incelemesi, dört katlı B'nin eyvanı kaldı; satır
  kaldı). Bütün `mimari-mevzuat` sınamaları yeşil; eski durumlar değişmedi.
- **Ortak durumlar:** `cad.program.edit.buildings` 25 (2'si yeni: binanın kesme taş yığma ve geleneksel sistemi yazılır;
  betonarmede geleneksel öğeler `program.buildings[0].structure.traditional`'da reddedilir).
- **`.kcad`:** `a_building_s_structure_round_trips_under_arch_data_31` (yazılır, okunur; 30 diyen dosya reddedilir);
  `arch_` sınamaları (32) yeşil. Olguların altı değişmedi.
- **Masaüstü** (`program_scenes::tests::two_blocks_get_codes_zeros_and_a_shared_level_through_the_window`): Binalar'da B
  Blok kesme taş yığma, geleneksel öğeli; programda; A Blok'un kat-1'i geleneksel değil, proje bir geleneksel bina
  tanıyor. Kalın taş duvar, kesme taş ve Sivas sınamalarıyla (`stone_wall_tests`, `cut_stone_tests`) ve ortak süzgeçlerle
  281 sınama yeşil.
- **Resimler:** `.run/gui-evidence/bina-tasiyici-8145/` (`tools_screens::screens`'in yeni `bina-tasiyici-sistem`'i):
  Binalar'da B Blok'un Taşıyıcı sistem'i ve geleneksel öğe anahtarı; açık ve koyu.

## §6. Kalanlar

- Kalın taş duvarların bina bina toplamı (olgularda binaya göre): birden çok geleneksel bina uzman incelemesinden çıkar.
- Künye: ruhsat takımının `tasiyici_sistem` metni boşsa binanın beyanıyla dolabilir (8094 §6).
