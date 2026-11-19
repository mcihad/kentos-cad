# ADR 8163: Binanın kendi plan notları — kademe hattıyla blok blok

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8154](8154-rules-by-building-zero.md) §6, [8158](8158-plan-subasman-notes.md) §6,
  [8160](8160-plan-roof-notes.md) §6 ve [8162](8162-plan-projection-basement-notes.md) §6 ("Plan notunun binası: blok
  blok farklı plan notu okunmaz").
- **Bağlam:** ADR 8084 (plan notu katmanı), 8098 (planın bina yüksekliği ve kat adedi), 8144 ve 8145 (binanın kendi
  bölgesi ve taşıyıcı sistemi: aynı yol), 8154 (bina bina kat adedi), 8157 (tahvil), 8158, 8160, 8162 (planın öbür
  notları). Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-mm (kademe hattı), 7/7, 7/12, 7/14, 10/3, 11/6, 69/3 (RG
  3.7.2017/30113; mevzuat.gov.tr'nin 1.7.2026'ya kadar işlenmiş birleştirilmiş metni).
- **Sözleşme:** `ProgramBuilding.planNotes` (`cad.program.edit`'in `program`'ı; yeni komut yok),
  `BuildingProgram::has_building_plan_notes`, `ArchSettings::plan_notes_on`. `kentos.auditFacts` 1'in katlarına
  `planNotes` (binasının kendi notları) ve `buildingName`. `tr.kentos.mimari-mevzuat` 0.50.0. **`.kcad`:** belge şeması
  değişmedi; Mimari verisi 35 (`ARCH_WITH_BUILDING_PLAN_NOTES`, `settings.arch.program.buildings[].planNotes`).
  **Web:** dokunulmadı (ADR 0228); TS türü yeniden üretildi, web'in Bina programı'na gerekenler TODOS.md §0.2'de.

## §1. Sorun

Plan notları projenindi (Proje ayarları › Mimari › Plan notları): bir parselin blokları kademe hattının iki yanında
durduğunda (A alanı Hmax 12,50, B alanı Hmax 9,50) bütün bloklar tek değerle denetleniyordu. Mimar ya sıkı değeri
yazıp gevşek alandaki bloğu haksız yere kaldırıyor ya da gevşeğini yazıp sıkı alandaki bloğu geçiriyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-mm | Kademe hattı: birden fazla kat adedi, bina yüksekliği, yapı yoğunluğu, yapı nizamı ya da kullanma kararına isabet eden parselde farklı yapılaşma ve kullanım kararını haiz alanların sınırının plan ile belirlendiği hat | Bir bloğun durduğu alanın değerleri kendi plan notlarıdır (K420) |
| PAİY 7/7 | Kademe hattı belirlenen yerde plan aksini demiyorsa ifraz ve tevhit zorunlu değil | Bloklar aynı parselde kalır: notlar projenin değil, bloğun (K420) |
| PAİY 7/12, 7/14 | 3,50 m'den çok kot farklı ya da farklı kat adedi/yükseklik verilmiş parsellerin tevhidinde kat adedi ve bina yüksekliği her kademenin kendi içinde; tevhit edilenlere verilen yükseklik aşılamaz | Her bina kendi notlarıyla ayrı denetlenir (K422) |
| PAİY 10/3, 11/6 | Birden çok binalı parselde her bina kendi yolundan kot alır; her kademedeki bina yüksekliği planın saçak seviyesini geçemez | Binanın kendi ±0,00'ı (8154) gibi plan notu da bina bina (K422) |
| PAİY 69/3 | İdarenin yönetmeliği planda aksine hüküm yoksa uygulanır | Binanın notu yönetmeliklerin önünde; projenin aynı ölçüdeki notunun da (K420) |
| ADR 8144, 8145 | Binanın kendi bölgesi ve taşıyıcı sistemi; yoksa projeninki; olguların katları binasınınkini taşır | Aynı yol: binanın notu, yoksa projeninki; katlar binasının notlarını taşır (K420, K421) |

KentOS kademe hattını çizmez, bloğun hangi alanda durduğunu bulmaz: alanın değerlerini planı okuyan mimar binaya yazar
(8084 K91: KentOS notun sözünü okumaz).

## §3. Karar

- **K420 — Binanın plan notları:** `ProgramBuilding.planNotes`: binanın durduğu alanın plan notları (Konu, Değer,
  Kaynak; projeninkiyle aynı ölçüler ve sınırlar). Binanın yazdığı ölçü projenin aynı ölçüdeki notunun yerine geçer
  (sıkı ya da gevşek); yazmadıkları projeninkidir (`ArchSettings::plan_notes_on`). Bir katın notları binasınınkidir
  (`building_of_level`; binasız kat projeninkileri okur). Doğrulama projeninkidir (ölçü başına bir not, aralık, kaynak),
  ileti binanın adıyla: "“A Blok” binasının plan notları: …". Bina programı › Binalar'da binanın formunda "Plan
  notları" bölümü: projeninkiyle aynı satırlar, Plan notu ekle, açıklama.
- **K421 — Olgular:** `kentos.auditFacts` 1'in her katı binasının kendi plan notlarını (`planNotes`, projeninkiyle aynı
  biçim) ve binasının adını (`buildingName`; bulgunun binayı andığı ad) taşır. Projenin notları `building.planNotes`'ta
  kalır.
- **K422 — Kurallar bina bina:** hiçbir bina kendi notunu söylemiyorsa kurallar eskisi gibidir (aynı bulgu, aynı
  satır). Söyleyen varsa:
  - binanın kendi notunun satırı binanın adıyla ayrı yazılır ("Plan notu: bina yüksekliği (A Blok)"), bulgunun kaynağı
    "A Blok plan notu (UİP …)"; projenin notunun satırı onu okuyan binaların sonucudur; hiçbir bina okumuyorsa kapsam
    dışı;
  - `bina_yuksekligi` (5/11: yükseklik, kat adedi, bodrum) ve `tahvil` (9/1-c) her binayı kendi notlarıyla, ileti
    binanın adıyla; `kat_yuksekligi` (28/1) binaları ayırır (8144'ün yolu); `subasman` (4/1-yyy) her binanın zemin
    katını; `cati_olculeri` (40/4, 42/1, eğim) her çatıyı, `cikmalar` (41/1) her çıkmayı katının binasının notlarıyla
    okur. Notun bağladığı maddenin satırı, maddeyle okunan ölçü kalmadığında kapsam dışıdır.
- **K423 — `.kcad`:** Mimari verisi 35: binası kendi plan notunu söyleyen dosya; 34 ya da eskisini söyleyen dosya bu
  notlarla reddedilir.

## §4. Uygulama

- `crates/shared/contracts/src/arch/program.rs` (`ProgramBuilding.plan_notes`, doğrulama, `has_building_plan_notes`),
  `arch.rs` (`plan_notes_on`); `crates/shared/kcad/src/lib.rs`, `decode.rs` (Mimari verisi 35).
- `crates/shared/extensions/src/audit_facts/building.rs` (`StoreyHolds.plan_notes`, `building_name`),
  `crates/native/processing/src/audit_facts/building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/katman.py` (`plan_notlari`, `plan_notu`, `not_satiri`,
  `bina_adi`, `PlanNotu.bina`), `bina_yuksekligi.py`, `kat_yuksekligi.py`, `tahvil.py`, `subasman.py`,
  `cati_olculeri.py`, `cikmalar.py`.
- `apps/desktop/src/shell/project/plan_notes.rs` (satırlar ayrı: `table`), `apps/desktop/src/arch/program_window.rs`
  (binaların yazılan notları, Kaydet'te okunur), `program_view.rs` (Plan notları bölümü).
- `scripts/fixtures/program_share_cases.py` (`cad.program.edit.buildings`).

## §5. Sınamalar

- **Kural:** 9 yeni bağımsız durum, beklenenler 5/11, 4/1-n, 28/1, 9/1-c, 4/1-yyy, 42/1, 41/1 ve K420–K422'den elle:
  bina yüksekliği 17 (3'ü yeni: A Blok'un 7,00 m'lik notu A'nın 7,60 m'sini kaldırır, B projenin 9,50'siyle geçer;
  A'nın gevşek 12,50'lik notu 12,00 m'yi geçirir; iki bloğun kendi kat adedi notları varken projeninki kapsam dışı),
  kat yüksekliği 49 (1), tahvil 22 (2: yalnız A'nın notuyla uzman incelemesi; A'da iki değer varken tahvil gerekmez,
  B'ninki geçer), subasman 15 (1), çatı ölçüleri 25 (1), çıkmalar 15 (1). Eski durumlar değişmedi; bütün
  `mimari-mevzuat` sınamaları yeşil.
- **Sözleşme** (`a_building_s_own_plan_notes_come_before_the_project_s_on_its_storeys`): binanın notu projeninkinin
  yerine, binasız kat projeninkini okur; aynı ölçüde iki not ve aralık dışı değer binanın adıyla reddedilir.
- **Paylaşılan komut durumları** (`cad.program.edit.buildings`, 27): binanın notları yazılır; aynı ölçüde iki not
  reddedilir.
- **`.kcad`** (`arch_plan_notes`): binanın notları Mimari verisi 35'le yazılıp okunur; 34 diyen dosya reddedilir.
- **Masaüstü** (`building_plan_notes_tests`): iki bloklu evde A Blok'un kendi Bina yüksekliği notu Binalar'dan yazılır,
  Kaydet'le programda; Denetim'de A Blok kendi notuyla kalır, B Blok projeninkiyle geçer.
- **Resimler:** `.run/gui-evidence/bina-plan-notlari-8163/`: Binalar'da A Blok'un plan notları ve Denetim'in satırları,
  açık ve koyu.

## §6. Kalanlar

- Kademe hattının kendisi çizilmez: iki alana taşan bloğun (7/14'ün kesişme sınırındaki kademe) parçaları ayrı
  denetlenmez; blok iki bina olarak yazılmadıkça tek alanın notunu okur.
- Asansör kuralı (34/1) bütün projenin: planın kat adedini projenin notundan (yoksa künyeden) okur, binanın notundan
  okumaz (8144 §6: asansör bina bina değil).
- Binanın plan kararından (CBS) Değerlerini al'ı yok (8140 projenin plan notlarına yazar).
