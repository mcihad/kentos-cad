# ADR 8091: Malzemenin yanıcılık sınıfı — kaplamalar BYKHY 26/2, 26/4 ve Ek-2/Ç ile

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, BIM-07; ADR
  [8089](8089-suspended-ceiling-fire-resistance.md) §6 ("26/4: malzemenin yanıcılık sınıfı beyanı gelince tavan
  kaplaması ve asma tavan en az zor alevlenici"); ADR [8068](8068-fire-performance-material-declarations.md) Kalan
  ("malzeme … için pencere alanları").
- **Bağlam:** ADR [8068](8068-fire-performance-material-declarations.md) (malzemenin dayanımı, `cad.arch.performance`),
  [0785](0785-material-identity-and-wrap-to-frame.md) (Malzemeler penceresi), [8070](8070-missing-building-elements.md) §10 (kaplama),
  [8089](8089-suspended-ceiling-fire-resistance.md) (asma tavan).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/ccc, 26/2, 26/4, 29 ve Ek-2/Ç (10.8.2009-2009/15316
  ile değişik, RG 9.9.2009/27344); TS EN 13501-1.
- **Web:** TS türleri yeniden üretildi (`ArchMaterial`, `PerformanceOperation`). **`.kcad`:** belge şeması değişmedi;
  mimari verisi sürüm 14 (`settings.arch.materials[].reaction`). **Sözleşme:** `ArchMaterial.reaction`,
  `reaction_problem`; `cad.arch.performance` v1'in malzeme işlemi `strength` ve `reaction` (verilmezse kalır, null
  kaldırır); `kentos.auditFacts` 1'in `building` bölümüne `coverings`.

## §1. Araştırma (17 Kasım 2026)

Ek-2/Ç tablosu yönetmeliğin PDF'inin 89. sayfasından resim olarak okundu (düz metin satırları karıştırıyordu; ADR 8081'in
kuralı).

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 26/2 | Döşeme kaplamaları en az normal alevlenici, yüksek binalarda en az zor alevlenici malzemeden | IFC türü FLOORING olan kaplamanın malzemesi; yüksek bina 4/ccc ile (ADR 8087) |
| BYKHY 26/4 | Ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanları en az zor alevlenici | IFC türü CEILING olan kaplamanın malzemesi; nizam modelde yok (yalnız CBS planında `Nizam`): tek konutlu binada tutmayan tavan uzman incelemesi |
| Ek-2/Ç (döşeme dışı) | Hiç yanmaz A1; zor yanıcı A2-s1,d0; zor alevlenici B, C-s1,d0, A2-s2,d0, A2/B/C-s3,d0, -s1,d1, -s1,d2, en az A2/B/C-s3,d2; normal alevlenici D-s1…s3,d0, E, D-s1…s3,d2, en az E-d2; kolay alevlenici F | Sınıf önce tam yazımıyla, yoksa harfiyle eşlenir: harfin en zayıf satırı (A2 yalnız: zor alevlenici) |
| Ek-2/Ç (döşeme) | Hiç yanmaz A1fl; zor yanıcı A2fl-s1; zor alevlenici Bfl-s1, en az Cfl-s1; normal alevlenici A2fl-s2, Bfl-s2, Cfl-s2, Dfl-s1, Dfl-s2, en az Efl; kolay alevlenici Ffl | Döşeme kaplamasına döşeme tablosu; öbür tablonun sınıfı eksik veri |
| Ek-2/Ç (çatı) | Dış alev yayılımına dayanıklı Broof (TS EN 13501-5) | Bu dilimde okunmaz (§6) |
| TS EN 13501-1'in yazımı | Sınıf harfi, döşemede `fl`, duman `s1–s3`, damla `d0–d2` | `reaction_problem`: A1 ve F'de duman, damla yok; döşemede damla yok; damla dumandan sonra virgülle, dumansız tireyle (E-d2) |

## §2. Karar

- **K120 — Malzemenin yanıcılık sınıfı beyandır** (`ArchMaterial.reaction`, `Declared`: değeri ve dayanağı); yoksa
  eksik. `cad.arch.performance`'ın malzeme işlemi `strength` ve `reaction`'ı birlikte ya da ayrı yazar: verilmeyen kalır,
  null kaldırır, ikisi de verilmezse `nothing_to_set` (ADR 8068'de verilmeyen dayanım siliniyordu; komut iki günlük ve
  dışarıda kullanılmıyor: anlam değişti, sürüm 1 kaldı). Okunmayan sınıf `invalid_declared` (`operation.reaction.value`).
- **K121 — Malzemeler penceresi:** "Dayanım" ve "Yanıcılık sınıfı" alanları; Uygula malzemenin öbür alanlarını
  `cad.materials.edit` ile ("Malzeme"), beyanları `cad.arch.performance` ile ("Yangın ve performans") yazar; okunmayan
  sınıf Uygula'dan önce söylenir.
- **K122 — Kaplamaların kuralı** (`kaplamalar`, paket 0.15.0): 26/2 FLOORING en az normal alevlenici (yüksek binada zor
  alevlenici), 26/4 CEILING en az zor alevlenici; Ek-2/Ç'nin tabloları kural setinde veri. Malzemesi yok, sınıfı beyan
  edilmemiş ya da öbür tablonun sınıfı: eksik veri.
- **K123 — Kural seti:** 26/2 ve 26/4'ün metni 2007'den, Ek-2/Ç'nin tablosu 9.9.2009'dan: hâl 9.9.2009; daha eski
  başvuruda kaplama varsa uzman incelemesi.
- **Dosya:** mimari verisi 14 (`ARCH_WITH_MATERIAL_REACTIONS`).

## §3. Uygulama

- **Sözleşme:** `materials.rs` (`reaction`), `performance.rs` (`reaction_problem`), `cad_arch_performance.rs`.
- **Dosya:** `kentos_kcad::{ARCH_WITH_MATERIAL_REACTIONS, uses_material_reactions}`.
- **Uygulama:** `arch_performance` malzeme dalı. **Masaüstü:** `materials_window.rs` (`declared_of`,
  `materials_declare`, alanlar ve önizleme).
- **Olgular:** `CoveringFacts` (kimlik, kat, IFC türü, malzeme, malzemenin sınıfı; malzeme kimliğiyle ya da adıyla).
- **Paket:** `kaplamalar.py` (`ozellik`, `_denetle`), kural setine 26/2 ve 26/4 (Ek-2/Ç veri), Denetim'in `kaplamalar`
  katkısı.

## §4. Sınamalar

- **Sözleşme:** `a_reaction_class_is_written_as_ek_2_c_writes_it` (tablonun bütün sınıfları okunur; A3, B-s4, F-d2,
  Bfl-s1,d0, B,d0 okunmaz). **Dosya:** `a_material_s_reaction_class_round_trips_under_arch_data_14`.
- **Komut durumları** (`cad.arch.performance.json`, 9 yeni, 61 durum; üretici ADR'den bağımsız sınıf okuyucusuyla):
  sınıf, belgeli döşeme sınıfı, sınıf ve dayanım birlikte, yalnız sınıf verilince dayanım kalır, sınıf kaldırılır, A3,
  damlalı döşeme sınıfı, boş sınıf, ne dayanım ne sınıf.
- **Paket** (`fixtures/standards/v1/kaplamalar.json`, 16 durum, metinden elle): parke Dfl-s1 geçer, halı Ffl kalır,
  yüksek büroda Dfl-s1 kalır, Cfl-s1 geçer, Bfl-s2 tam yazımıyla normal alevlenici, sınıfsız ve malzemesiz eksik veri,
  döşemeye döşeme dışı sınıf eksik veri; asma tavan A2-s1,d0 geçer, D-s2,d0 kalır, yalnız A2 geçer, Bfl-s1 eksik veri;
  tek konutlu evde D-s2,d0 uzman incelemesi, B-s1,d0 geçer; 2008 başvurusu uzman incelemesi; tarihsiz.
- **Masaüstü** (`covering_reaction_tests`, dört daireli binanın 1. katı): olgularda iki kaplama malzemeleriyle; beyansız
  ikisi de eksik veri; Malzemeler'de Alçıpan D-s2,d0 ve Parke Dfl-s1 yazılınca 26/2 geçer, 26/4 kalır; A3 Uygula'dan önce
  söylenir; A2-s1,d0 ile 26/4 geçer. **Resimler:** `.run/gui-evidence/yanicilik-8091/`.

## §5. Arayüz

- Malzemeler › seçili malzeme: "Dayanım" (C25, S355…) ve "Yanıcılık sınıfı" (TS EN 13501-1: A2-s1,d0, B-s1,d0, Bfl-s1…).

## §6. Sıradaki adımlar

- **Broof (TS EN 13501-5):** çatı kaplamasının dış alev yayılımı sınıfı ve çatıyla ilgili maddeler.
- **Duvar ve tavan kaplamaları** (mahalin kaplama kalınlıklarının malzemesi; kaçış yolunun kaplamaları, 41 ve 36): mahal
  kaplamasının malzemesi modelde yok ([ADR 8138](8138-room-finish-coverings.md)'de yapıldı: kaplamanın adı malzemedir,
  20/3, 26/2, 26/4; 36 ve 41'in bugünkü metninde kaplamanın yanıcılığı yok).
- **Isı yalıtımı (26/3, 27):** katmanın malzemesinin sınıfı katman katman okunabilir; cephe kuralı ayrı dilim.
- **Nizam:** projenin nizamı İmar durumu'na yazılırsa 26/4'ün tek konutlu binası kesinleşir.
