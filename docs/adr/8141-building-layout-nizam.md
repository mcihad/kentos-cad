# ADR 8141: Yapı nizamı — İmar durumu'nda; PAİY 5/6, BYKHY 26/4 ve 28/3 onunla

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8140](8140-plan-decision-values-into-zoning.md) §6 ("Nizam"), [8139](8139-roof-seat-slab.md) §6 ("Bitişik nizam"),
  [8138](8138-room-finish-coverings.md) §6 ("26/4'ün nizamı"), [8134](8134-roof-covering-fire-class.md) (28/3-b'nin
  uzman incelemesi).
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (İmar durumu), [8085](8085-zone-storey-heights.md)
  (Bölge: aynı biçimde bir seçim), 8140 (Değerlerini al), [8091](8091-material-reaction-to-fire.md) (26/4),
  8134 ve 8139 (28/3); `tr.kentos.mevzuat`'ın `imar-durumu` kuralı (5/6). Kurallar Python'dadır, Rust olguları verir
  (bim-plan K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-pppp, 5/6 (RG 1.7.2026/33297 ile değişik), 8/5; Binaların Yangından
  Korunması Hakkında Yönetmelik 26/4, 28/3.
- **Sözleşme:** `ArchSettings.layout` (`BuildingLayout`: `detached`, `attached`, `block`, `twin`; `cad.settings.set`'in
  `arch`'ı, yeni komut yok). `kentos.auditFacts` 1'e `building.layout` ve `architecture.zoning.layout`.
  `tr.kentos.mimari-mevzuat` 0.32.0, `tr.kentos.mevzuat` 0.8.0. **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm
  28 (`settings.arch.layout`). **Web:** dokunulmadı (ADR 0228); TS türleri yeniden üretildi (`ArchSettings`,
  `BuildingLayout`), web'in İmar durumu'na gerekenler TODOS.md §0.2'de.

## §1. Sorun

Binanın yapı nizamı projede yoktu. Üç kural onu bekliyordu: PAİY 5/6 TAKS verilmeyen parselde ayrık ve blok nizamda %40,
öbüründe %60 der; KentOS nizamı bilmediği için %40 ile %60 arasını uyarıyla bırakıyordu. BYKHY 26/4 ayrık nizamdaki
müstakil konutu dışarıda bırakır: tek konutlu binada tutmayan tavan uzman incelemesiydi (8091). BYKHY 28/3 yüksek binada
ve bitişik nizamda uygulanır: yüksek olmayan binada tutmayan çatı uzman incelemesiydi (8134, 8139). Planın kararı nizamı
yazıyor (profilin `Nizam` alanı) ama Değerlerini al onu almıyordu (8140 §6).

## §2. Araştırma (18 Kasım 2026)

PAİY'nin metni derlenmiş resmî metinden (pdftotext; 8098'in kopyası) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-pppp | Yapı nizamı: uygulama imar planıyla belirlenen ayrık, blok ve bitişik nizamdan biri; ayrık hiçbir yanından komşu parseldeki binaya bitişik olmayan, bitişik bir ya da birden çok komşu binaya bitişik olan; bir yanı bitişik öbür yanı ayrık ikili ya da ikiz nizamlar blok nizam sayılır | Dört seçenek: Ayrık, Bitişik, Blok, İkiz; ikiz planın kendi yazımı olarak saklanır, kurallar blok sayar (K355) |
| PAİY 5/6 (1.7.2026) | Ayrık veya blok nizamda planda açıkça belirlenmemişse TAKS %40'ı geçemez; KAKS verilip TAKS verilmeyen parselde TAKS %60'ı geçmemek şartıyla yaklaşma mesafelerine göre uygulama yapılır | Ayrık, blok, ikiz %40; bitişik %60 (K357) |
| PAİY 8/5 | İmar planında yapı nizamı belirlenmeyen hallerde ayrık nizam uygulanır | "Yazılmamış" planda belirlenmemiş demek değildir: KentOS ayrık saymaz; seçimin ve uyarının sözü 8/5'i söyler, müellif seçer (K355) |
| BYKHY 26/4 | Ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanları en az zor alevlenici | Ayrık nizamdaki müstakil konut kapsam dışı; başka nizamdaki tek konutlu binada uygulanır (K358) |
| BYKHY 28/3 | Yüksek binalarda ve bitişik nizam yapılarda çatının oturduğu döşeme yatay yangın kesici (a), çatı taşıyıcısı ve kaplaması yanmaz (b) | Bitişik nizamda yüksek olmayan binada da uygulanır; öbür nizamlarda yüksek olmayan binada kapsam dışı (K359) |
| `imar-plani` profili, `yapi-nizami` alanı | A Ayrık, I İkiz, B Bitişik, BL Blok | Değerlerini al kodu ya da etiketi okur; başka yazım (kat adediyle "A-3" gibi) okunmaz, çevrilmez (K356) |
| BIM yönetmeliğinin ekleri (EK-6, EK-9) | Nizam özelliği yok | IFC'ye yazılmaz |

## §3. Karar

- **K355 — Yapı nizamı:** `ArchSettings.layout` (Ayrık, Bitişik, Blok, İkiz). Proje ayarları › Mimari › İmar durumu'nda
  Bölge'nin altında "Yapı nizamı" seçimi; boşu "Yazılmamış (plan belirlemiyorsa ayrık uygulanır)". Yazılmamış nizam ayrık
  sayılmaz: plan nizam belirlemiyorsa müellif Ayrık'ı seçer (8/5). Bölge gibi kaynağı yoktur. Mimari verisi 28.
- **K356 — Değerlerini al:** kararın Nizam'ı profilin kodu (A, I, B, BL) ya da etiketiyse (büyük-küçük harf Türkçe
  kuralıyla bir: İ'nin küçüğü i, I'nınki ı) Yapı nizamı'na yazılır; söz "Emsal 1,20, TAKS 0,40 ve nizam ayrık İmar
  durumu'na, kat adedi 3 Plan notları'na.". Form o nizamı tutmuyorsa düğme açık kalır.
- **K357 — PAİY 5/6 (`imar-durumu`, `tr.kentos.mevzuat` 0.8.0):** TAKS verilmemişse ayrık, blok ya da ikiz nizamda %40'ı
  aşan taban alanı hata ("yapı nizamı ayrık, 5/6'ya göre %40'ı geçemez"); bitişik nizamda %60'ı aşan hata, altı bulgusuz.
  Nizam yazılmamışsa eskisi gibi: %60'ı aşan hata, %40 ile %60 arası uyarı; uyarı nizamın nereye yazılacağını ve 8/5'i
  söyler. Olgular `zoning.layout`.
- **K358 — BYKHY 26/4 (`kaplamalar`):** müstakil konut tek konutlu ve başka bölümü olmayan binadır (8091). Ayrık nizamda
  madde kapsam dışı; bitişik, blok ya da ikiz nizamda tek konutlu binanın tavanı da okunur; nizam yazılmamış tek
  konutlu binada tutmayan tavan uzman incelemesi, söz nizamın nereye yazılacağını söyler.
- **K359 — BYKHY 28/3 (`catilar`):** yüksek olmayan binada bitişik nizamda a ve b bentleri yüksek binadaki gibi okunur
  (döşeme yoksa ya da sınıf beyan edilmemişse eksik veri, tutmazsa kaldı; söz "bitişik nizamda"); ayrık, blok ya da ikiz
  nizamda madde kapsam dışı ("bina yüksek değil, yapı nizamı ayrık"). Nizam yazılmamışsa eskisi gibi uzman incelemesi.
  Olgular `building.layout`.
- **K360 — Örnek sınırı:** paketleyici bir kuralın en çok on örneğini alır; `imar-durumu`'nun 0.7.0'daki kaynaklı TAKS
  örneği çıktı (kaynaklı emsal örneği aynı yolu gösterir; kaynaklı TAKS bulgusu `apps/jobrun`'un sınamasında kalır).

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`BuildingLayout`: `label`, `word`, `name`, `from_plan`;
  `ArchSettings.layout`, `has_layout`); `crates/shared/kcad/src/lib.rs`, `decode.rs` (`ARCH_WITH_LAYOUT`, `uses_layout`,
  eski sürümün reddi).
- `crates/shared/extensions/src/audit_facts/building.rs`, `architecture/zoning.rs`; `crates/native/processing/src/
  audit_facts/building.rs`, `architecture.rs`; `python/kentos/ext.py` (`ZoningFacts.layout`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/catilar.py` (`NIZAM`, `uygulanir`), `kaplamalar.py` (26/4);
  `catalog/standards/tr-mevzuat/src/tr_mevzuat/imar.py` (5/6).
- `apps/desktop/src/shell/project/arch.rs` (`layout_choices`, Yapı nizamı satırı, `take`, `holds`), `settings.rs`
  (`ArchLayout`), `apps/desktop/src/map/zoning_badge.rs` (`PlanTake.layout`).

## §5. Sınamalar

- **Sözleşme:** `the_layout_is_written_by_its_name_and_read_from_the_plan_s_codes`: adıyla yazılır, yazılmamışsa alan
  yok; A, " bl ", I, b, BİTİŞİK, İKİZ, AYRIK, Blok okunur; "A-3", "ayrık nizam" ve boş okunmaz.
- **`.kcad`:** `the_layout_round_trips_under_arch_data_28`: mimari verisi 28 ile yazılır, okunur; aynı dosya 27 derse
  reddedilir (8140'ın sürüm düzeltme yardımcısı ortak). Bütün `arch_` sınamaları (29) yeşil.
- **Olgular:** processing'in `audit_facts`'i (20) nizamın `zoning.layout`'a geçtiğini sınar; olguların altı değişmedi;
  SDK taslağı ve şemalar yeniden yazıldı (24 yeşil).
- **Kurallar:** çatılar 37 durum (8'i yeni, 4/1-pppp ve 28/3'ten elle: bitişik nizamda döşemesiz eksik veri, R 60 ve çam
  mertek kaldı, tam olan geçer; ayrık, blok, ikiz nizamda kapsam dışı; yüksek binada ayrık nizam yüksekliğiyle eksik
  veri), kaplamalar 34 durum (4'ü yeni: ayrık nizamda müstakil konut kapsam dışı, bitişik ve ikiz nizamda tek konutlu
  ev kalır, ayrık nizamda apartman kalır); bütün `mimari-mevzuat` sınamaları yeşil. `tr.kentos.mevzuat`'ın on örneği
  kurala karşı tek tek tutar; `apps/jobrun`'un `mevzuat_tests`'i (14, `KENTOS_AUDIT_CONFINE=report`) ayrık, blok ve
  ikizde %40'ın hatasını, bitişikte bulgusuzluğu sınar.
- **Ortak durumlar:** `cad.settings.set` 137 (2'si yeni: bitişik ve ikiz yazılır).
- **Masaüstü:** `zoning_parcel_tests`: K1'in Nizam'ı "A" ayrık, K2'ninki "BL" blok alınır, söz "…ve nizam ayrık İmar
  durumu'na"; "A-3" okunmaz. `roof_fire_tests::the_building_s_layout_decides_28_3_in_a_building_that_is_not_high`: 6 m'lik
  binanın çam mertekli çatısı nizam yazılmamışken uzman incelemesi; Proje ayarları'nda Bitişik seçilip kaydedilince
  olgularda `layout` "attached", 28/3 kaldı ("28/3-b bitişik nizamda"); Ayrık seçilince kapsam dışı, bulgu yok. Ortak
  süzgeçlerle 281 sınama yeşil.
- **Resimler:** `.run/gui-evidence/yapi-nizami-8141/` (`roof_fire_tests::layout_screens`): İmar durumu'nda Yapı nizamı
  Bitişik; Denetim'de 28/3'ün bulgusu; açık ve koyu. `.run/gui-evidence/plan-degerleri-8140/` nizamlı sözle yeniden.

## §6. Kalanlar

- **PAİY 39/4:** bitişik ve blok nizamda komşu parsel sınırındaki bitişik duvarda pencere ve kapı açılamaz: vaziyet
  planının kenarları (ADR 8032) ve duvarların açıklıkları çizimde; bir sonraki dilim.
- **PAİY 41:** bitişik nizamda çıkma bitişik olduğu komşu sınırına 2,00 m'den fazla yaklaşamaz (çıkmalar plan notunun
  da konusu, ADR 8084 §6).
- **PAİY 6:** parselin cephesi nizama ve kat adedine göre (parsel düzeyinde; harita modülünün işi olabilir).
- **Kullanım → Bölge:** kararın Kullanım'ını (profilin kullanım kodları) Bölge'ye (28/1) almak kaynaklı bir eşleme ister.
