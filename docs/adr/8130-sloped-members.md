# ADR 8130: Eğimli eleman (mertek, payanda)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B9 (BIM-09'un son kalanı: "eğimli
  eleman"), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 (IfcMember satırının kalanı).
- **Bağlam:** ADR [8070](8070-missing-building-elements.md) §8 (K46: eleman kolonun ve kirişin rolüdür; "Eğimli eleman
  (mertek, payanda): kirişin ekseni düzlemde; eğik gövde sonraki adım"), [8041](8041-beams-lintels-column-wall-joins.md) (kiriş),
  [8071](8071-geometry-accuracy.md) §6 (eğimli döşemenin `PitchAngle`'ı).
- **Dosya:** `.kcad` mimari verisi 22 (`ARCH_WITH_SLOPED_MEMBERS`; 1–21'in verisini de içerir): kirişin `rise` alanı;
  daha küçük sürüm diyen dosyadaki eğimli eleman okunurken reddedilir; belge şeması değişmez. **Komut:**
  `cad.beams.create` `rise` alır, `cad.arch.edit`'in `rise`'ı (rampanınki) kirişte de geçerli; komut sürümleri değişmez.

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-6 Tablo 6.40 ve isteğe bağlı özellikler (`mevzuat/bim/ek-6-2`, `ek-6-4`) | `IfcMember`'ın yönelimi (yatay, düşey ya da eğimli) tanımı için önemli değildir; tip listesinde RAFTER, BRACE, STRUT; `Pset_MemberCommon.Slope` "eğim açısı yataya göre"; geometri ile çelişirse geometri esastır | Eğimli eleman yine `IfcMember`'dır; Slope yazılır, gövde eğik katıdır (K304) |
| Autodesk, Revit yardımı (2014), "Sloped Beam Modeling" | Eğik kiriş başlangıç ve bitiş ucunun kot farklarıyla (Start/End Level Offset) kurulur | Uçların kot farkı: başının kotu düz elemanınki, ucu ondan `rise` kadar yüksek (K302) |
| Graphisoft, Archicad yardımı, eğik kiriş (Inclined Beam) | Eğim açısı −89°…+89°, başlangıç noktasından ölçülür; eğik kirişin gövdesi, eğik eksene dik kesitin yüksekliği her yerde aynı olacak biçimde kurulur | Elemanın derinliği eksenine diktir; düşeyde √(1 + g²) katı (K303) |
| IFC 4.3, `IfcMemberTypeEnum` | RAFTER çatı örtüsünü taşıyan, çoğu eğimli eleman; BRACE çaprazlama | Türkçe sözcükleri ADR 8070'teki gibi Mertek, Payanda |

## §2. Karar

- **K302 — Yükselme:** eleman olan kirişin (kayıt, mertek, payanda) `rise`'ı ekseninin ucunun başından ne kadar yüksek
  olduğudur (metre; eksi aşağı iner). Başının kotu düz bir elemanınki gibidir (kattan yüksekliği varsa alt yüzü, yoksa
  asıldığı yerden üstü); ekseni boyunca eşit yükselir. Yalnız elemanın, iki noktalı (düz) eksende; 0 yazılmaz (düz eleman
  yükselmesizdir), en çok ±10 m. Çerçevenin kirişi, lento ve hatıl düzdür; eleman çerçevenin kirişine dönünce düzleşir.
  Uzunluğu eğimi boyuncadır (`length`; plandaki `plan_length`).
- **K303 — Gövde:** kesit modelinde eğik katı (çatının ve rampanın eğik plağı gibi): üst yüzü ekseni boyunca eğik
  düzlem, derinliği eksenine dik ölçülür, düşeyde `yükseklik × √(1 + g²)`; uçları düşey kesilir. Planda düz eleman gibi
  izdüşümüdür. Duvar üstündeki eğimli elemanın altında o noktadaki alt yüzünde durur (`beam_underside_at`).
- **K304 — IFC:** `IfcMember` tipiyle (RAFTER, BRACE …); gövdesi eğik katının yüzleri; `Pset_MemberCommon.Slope` eğim açısı
  (eğimli döşemenin `PitchAngle`'ı gibi, düzlem açısı).
- **K305 — Araç ve Öznitelikler:** Kayıt aracının Yükselme (U) seçeneği (boş ya da 0: düz; eğimli kayıt iki noktadır,
  üç noktalıya komutun reddi söylenir); Öznitelikler'de elemanın Yükselme satırı (boş: düz) ve Eğim (projenin açı biriminde
  ve yüzde, salt okunur).

## §3. Uygulama

- `crates/shared/contracts/src/arch/beam.rs` (`rise`, `plan_length`, `grade`, `is_sloped_member`, kurallar);
  `crates/shared/commands` (`NewBeam.rise`, `ArchChange.rise`'ın belgesi); `crates/shared/kcad` (sürüm 22).
- `crates/native/application/src/arch/{arch_elements.rs, arch_edit.rs, sections.rs}`; `crates/native/domain/src/
  wall_height.rs` (`beam_underside_at`); `crates/shared/ifc/src/arch.rs` (Slope).
- `crates/native/interaction/src/arch/beam_tools.rs` (Yükselme); `apps/desktop/src/cad/properties`
  (`ArchField::MemberRise`, satırlar).
- Katalog, TypeScript bağlamaları, Python SDK yeniden yazıldı (yalnız `rise` alanı).

## §4. Sınamalar

- **Sözleşme** (`beam::a_sloped_member_is_a_straight_member_s`): 4 m'lik, 3 m yükselen mertek 5 m, eğimi 0,75; kiriş, 0,
  10,5 m, NaN ve üç nokta reddedilir. **`.kcad`** (`a_sloped_member_round_trips_under_arch_data_22`): sürüm 22 ile gidip
  gelir; çerçevenin kirişine yükselme yazılırken reddedilir.
- **Uygulama** (`arch_ifc::a_sloped_rafter_rises_along_its_axis_and_says_its_slope`, elle 3–4–5): alt yüzü başında 2,50,
  düşey derinliği 0,125, üstü 2,625'ten 5,625'e; x = 22'deki kesit 4,00–4,125; IFC'de RAFTER, Slope atan 0,75, gövdesi
  yüzler; kirişe ve eğri elemana yükselme reddi; 0 düzler.
- **Ortak durumlar** (`scripts/fixtures/arch_command_cases.py`, KentOS kodu olmadan): `cad.beams.create`'e 7 (mertek,
  aşağı inen payanda, 0'ın düz yazılması; kiriş, üç nokta, 12 m ve sonsuz yükselme reddi), `cad.arch.edit`'e 3 (merteğe
  yükselme ve 0 ile düzleme, payandanın çerçevenin kirişine dönünce düzleşmesi, kirişe yükselme reddi); var olan durumlar
  değişmedi; üreticiye bağlı sekiz üretici başvurularıyla aynı.
- **Etkileşim** (`member_tools::the_rail_tool_writes_a_sloped_member_on_two_points`): Yükselme 1,5 ile iki tıklama eğimli
  kayıt yazar; üç nokta yazılmaz ve kural söylenir; 0 düz yazar. **Öznitelikler**
  (`a_column_becomes_a_post_and_a_beam_a_rail`): Yükselme 1,25 yazılır, Eğim "15.5958 g (%25.00)"; boş düzler.
- **Resimler:** `.run/gui-evidence/egimli-eleman-8130/` (görünüş binasının doğusunda sundurma pergola: duvardaki
  kayıttan dikmelerin üstündeki kayda inen beş mertek, iki payanda; plan, doğu görünüşü, A-A, kuzeydoğudan 3B; açık ve
  koyu).

## §5. Kalanlar

- Kırık eksenli eğimli eleman (her parçası kendi eğimiyle) ve uçlarının yatay ya da açılı kesimi.
- Kesitte eğimli elemanın bir çatı yüzüne ya da duvara oturduğu yerin kendiliğinden bulunması (bugün kotlar elle verilir).
