# ADR 8144: Binanın kendi bölgesi — blok blok PAİY 28/1

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8085](8085-zone-storey-heights.md) §6 ("Blok başına bölge: birden çok bloklu parsel, ADR 8072").
- **Bağlam:** ADR 8085 (Bölge, 28/1'in bentleri), [8072](8072-buildings-shared-basement-location.md) (bina programının
  binaları, katları, ortak katın sayıldığı bina), [8143](8143-zone-from-plan-use.md) (bölge kararın kullanımından).
  Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 28/1, 4/1-mm (kademe hattı: bir parsele birden çok kullanım kararı isabet
  edebilir).
- **Sözleşme:** `ProgramBuilding.zone` (`cad.program.edit`'in `program`'ı; yeni komut yok), `ArchSettings::zone_on`.
  `kentos.auditFacts` 1'in katlarına `building` ve `zone`. `tr.kentos.mimari-mevzuat` 0.34.0. **`.kcad`:** belge şeması
  değişmedi; mimari verisi sürüm 30 (`settings.arch.program.buildings[].zone`). **Web:** dokunulmadı; TS türü yeniden
  üretildi, web'in Bina programı'na gerekenler TODOS.md §0.2'de.

## §1. Sorun

Bölge projenindi: bir parselin blokları farklı kullanımlı kararlarda durduğunda (kademe hattı, PAİY 4/1-mm; 8072'nin
iki kararlı parseli) bütün katlar tek bölgeyle okunuyordu. 28/1-d'nin "zemin dahil 3 katı geçmeyen" binası da bütün
projenin katlarıyla sayılıyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-mm | Bir imar parseline birden fazla kat adedi, yükseklik, yoğunluk, nizam ya da kullanım kararı isabet edebilir (kademe hattı) | Bir bloğun kendi bölgesi olabilir (K369) |
| PAİY 28/1 | Kat yükseklikleri bölgeye göre: a ticaret, b ticaret + konut, c konut, ç zemin katında ticaret yapılabilen konut alanı | Binanın katları kendi bölgesinin bendiyle (K370) |
| ADR 8072 §1 | Bir kat, onu kapsayan tek binanın ya da ortak katın seçildiği binanın katıdır (`building_of_level`) | Katın bölgesi o binanınki, yoksa projeninki (K369) |

## §3. Karar

- **K369 — Binanın bölgesi:** `ProgramBuilding.zone`: binanın kendi bölgesi; yoksa projeninki (Proje ayarları › Mimari ›
  İmar durumu). Bir katın bölgesi katın binasınınkidir (`ArchSettings::zone_on`). Bina programı › Binalar'da "Bölge"
  seçimi ("Projeninki" ya da 28/1'in bölgeleri). Mimari verisi 30.
- **K370 — 28/1 bina bina (`kat_yuksekligi`):** olguların her katı binasını (`building`) ve binasının kendi bölgesini
  (`zone`) taşır. Bir binası kendi bölgesini söylüyorsa katlar binalarına göre ayrılır, her binanın katları kendi
  bölgesinin bendiyle denetlenir (28/1-d'nin kat sayısı da binanınkidir); aynı maddenin satırları birleşir (durumun
  önceliği ve notları). Hiçbir bina söylemiyorsa kural eskisi gibi bütün projenin katlarını birlikte okur.

## §4. Uygulama

- `crates/shared/contracts/src/arch/program.rs` (`ProgramBuilding.zone`, `has_building_zones`), `arch.rs` (`zone_on`);
  `crates/shared/kcad/src/lib.rs`, `decode.rs` (`ARCH_WITH_BUILDING_ZONES`).
- `crates/shared/extensions/src/audit_facts/building.rs` (`StoreyHolds.building`, `zone`),
  `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kat_yuksekligi.py` (`kontrol` binalara ayırır, `_bina`).
- `apps/desktop/src/arch/program_window.rs` (`BuildingZone`), `program_view.rs` (Bölge seçimi).

## §5. Sınamalar

- **Kural** (`test_kat_yuksekligi.py`, 46 durum; 2'si yeni, 28/1'den elle): A bloğu kendi bölgesi ticaret alanı (zemin
  4,80 > 4,50 kalır, 1. kat 4,00 tutar), B bloğu projenin konut alanı (iki katlı blok, 12.8.2023'ün 4,00'ı; 3,80 tutar):
  28/1-a kaldı, 28/1-c geçti; B'nin kendi bölgesi öbür kullanımsa B kapsam dışı. Eski 44 durum değişmedi. Bütün
  `mimari-mevzuat` sınamaları yeşil.
- **Ortak durumlar:** `cad.program.edit.buildings` 23 (1'i yeni: binanın bölgesi yazılır).
- **`.kcad`:** `a_building_s_zone_round_trips_under_arch_data_30` (yazılır, okunur; 29 diyen dosya reddedilir); `arch_`
  sınamaları (31) yeşil. Olguların altı değişmedi (örnek binanın programında bina yok).
- **Masaüstü** (`program_scenes::tests::two_blocks_get_codes_zeros_and_a_shared_level_through_the_window`): Binalar'da A
  Blok'a Ticaret alanı seçilir, Kaydet'le programda; `zone_on("kat-1")` ticaret alanı; olgularda kat-1'in binası "a",
  bölgesi "commercial". Ortak süzgeçlerle 280 sınama yeşil.
- **Resimler:** `.run/gui-evidence/bina-bolgesi-8144/` (`tools_screens::screens`'in `bina-ortak-kat`'ı): Binalar'da A
  Blok'un Bölge'si; açık ve koyu.

## §6. Kalanlar

- **Binanın kendi taşıyıcı sistemi** (8094 §6): emsal tablosunun geleneksel binası (22/1-m, 5/8, 20/8-j) ve 28/1-d bina
  bina; bir sonraki dilim.
- Öbür bina bina kurallar (otopark, sığınak, asansör) bugün bütün projenindir.
