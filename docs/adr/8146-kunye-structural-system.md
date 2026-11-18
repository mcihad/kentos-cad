# ADR 8146: Künyenin taşıyıcı sistemi beyandan

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8094](8094-structural-system-traditional-cut-stone.md) §6 ("Künye: ruhsat takımının `tasiyici_sistem` metni boşsa
  beyanın adıyla dolabilir").
- **Bağlam:** ADR [8037](8037-turkish-title-block-sheet-sets-pdfa.md) (ruhsat takımı, künye değişkenleri), 8094 (taşıyıcı sistem beyanı),
  [8145](8145-structure-per-building.md) (binanın kendi sistemi).
- **Sözleşme:** değişmedi (`cad.permit.set`'in künyesi aynı değişkenler). **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Projenin taşıyıcı sistemi beyan edilmişken (Proje ayarları › Mimari › Taşıyıcı sistem) ruhsat takımının künyesi
"Taşıyıcı sistem"i ayrıca istiyordu; boşsa paftalar onu eksik yazıyordu.

## §2. Karar

- **K374 — Beyanın sözü (`permit_set::structure_words`):** projenin beyanı ("Betonarme", geleneksel kesme taş yığmada
  "Kesme taş yığma (geleneksel mimari öğeli)"), yoksa beyan eden bütün program binalarının aynı sistemi; binalar
  farklıysa yok (künye proje bütünündür, müellif yazar).
- **K375 — Künye (`with_particulars_of`):** çizimde hiç "tasiyici_sistem" değişkeni yoksa ruhsat takımı onu beyanın
  sözüyle yazar (işin adının çizimin adından gelmesi gibi); pencere alanı onunla açılır, müellif değiştirebilir; çizimin
  kendi değişkeni varsa dokunulmaz. Böyle bir çizimde künye onu eksik saymaz.

## §3. Uygulama

- `crates/native/application/src/arch/permit_set.rs` (`structure_words`, `with_particulars_of`, `missing_particulars`;
  `cad.permit.set` yeni işlevle); `apps/desktop/src/shell/exchange/permit_set.rs` (alanın açılış değeri, takımın
  künyesi).

## §4. Sınamalar

- **Uygulama:** `the_kunye_s_structure_is_the_declaration_s`: beyansız çizimde yok; iki binanın aynı geleneksel kesme taş
  yığması "Kesme taş yığma (geleneksel mimari öğeli)"; farklı binalar yok (eksik sayılır); projenin betonarme beyanı
  "Betonarme", eksik sayılmaz, künyeye yazılır; yazılan "Betonarme karkas" kalır.
- **Masaüstü** (`permit_set_tests::the_kunye_s_structural_system_is_the_declaration_s`): betonarme beyanlı binada Ruhsat
  takımı alanı "Betonarme" ile açılır, takım onu yazar. Ruhsat takımının öbür sınamalarıyla 11 sınama yeşil.
- **Resim:** alanın açılış değeri sınamayla doğrulandı; ayrı resim çekilmedi.
