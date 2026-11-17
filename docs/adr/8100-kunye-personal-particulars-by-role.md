# ADR 8100: Künyenin kişisel bilgileri role göre — bulut projesinde yalnız nesneleri yazabilenlere

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-15'in ilk kabulünün son cümlesi
  ("Yetkisiz kullanıcı künyeyi göremez"; uygulama sırasında B16). ADR [8099](8099-delivery-record.md) §6'nın ilk maddesi.
- **Bağlam:** ADR [0015](0015-project-ownership-and-access.md) (proje rolleri ve izinler), [0550](0550-one-reach-resolver.md)
  (formların "örneklerde gizle"si: okumada ve kopyada kesme, saklanan kopyanın kesilmemesi §2–§3, indirilmiş kopyanın geri
  alınamaması §5), [8037](8037-turkish-title-block-sheet-sets-pdfa.md) (Ruhsat takımının künyesi, `kentos_sheet::permit::PARTICULARS`),
  [8099](8099-delivery-record.md) (teslim kaydının yayımlayanı).
- **Dayanak:** 6698 sayılı Kişisel Verilerin Korunması Kanunu md. 3/1-d (kişisel veri: kimliği belirli ya da
  belirlenebilir gerçek kişiye ilişkin her türlü bilgi), md. 4/2-ç (işlendikleri amaçla bağlantılı, sınırlı ve ölçülü
  olma); e-PYS Yönetmeliği md. 4/5 (idare müellifin izni olmadan veriyi çoğaltamaz, üçüncü kişiye veremez).
- **Web:** dokunulmadı (ADR 0228). Sunucu ortak: web istemcisi kesilmiş künyeyi zaten alır. Paylaşım penceresinin rol
  ipuçları ortak fikstürde (`fixtures/cloud/v1/share.json`) değişti; web'in `sharing.ts`'i TODOS.md §0.2'de.
- **Sözleşme:** izin eklenmedi; `ProjectAccess::sees_particulars` (`feature.write`). `kentos_contracts::{PERSONAL_PARTICULARS,
  CUT_PARTICULAR}`, `ProjectSettings::{has_particulars, cut_particulars}`, `ProjectSheets::{has_particulars, cut_particulars}`.
  **`.kcad`:** değişmedi.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| 6698 md. 3/1-d | Kişisel veri: kimliği belirli ya da belirlenebilir gerçek kişiye ilişkin her bilgi | Yapı sahibi, müteahhit, müellif, büro (tek kişilik büronun adı ve vergi numarası kişiyi belirler), tasarlayan, çizen, kontrol eden, denetçiler, tapu kaydı, teslim edenin adı |
| 6698 md. 4/2-ç | Amaçla bağlantılı, sınırlı ve ölçülü | Künyeyi işleyen (projeyi hazırlayan) görür; izleyen, yorum yazan ve form doldurana gerekmez |
| e-PYS md. 4/5 | İdare veriyi müellifin izni olmadan çoğaltamaz, üçüncü kişiye veremez | Paylaşılan bulut projesinde de künye yalnız projede çalışana |
| ADR 0015 | Roller: İzleyici, Yorumcu, Veri girişi, Düzenleyici, Yönetici, Sahip; izinler sabit | Yeni izin yok: `feature.write` (Düzenleyici ve üstü) projede çalışmanın işaretidir |
| ADR 0550 §2–§3 | Okuma ve kopya kesilir; saklanan kopya kesilmez, tam hakkı olmayana verilmez | Aynı kalıp: okumada ve anlık görüntüde kes; künye taşıyan saklı kopyayı verme |
| ADR 0550 §5 | İndirilmiş kopya geri alınamaz | Rolü düşürmek önce verilmiş künyeyi geri almaz; paylaşım penceresindeki kalıcı not bunu zaten söyler |
| `crates/native/cloud/src/open.rs` | İstemci indirdiği revizyonun SHA-256'sını listeyle karşılaştırır | Dosya revizyonu kesilmiş verilemez (SHA değişir): künye taşıyorsa ret |

## §2. Karar

- **K152 — Kimin:** künyenin kişisel bilgilerini projenin nesnelerini yazabilen (`feature.write`: Düzenleyici, Yönetici,
  Sahip, kurumun politikasıyla yönetici) görür. İzleyici, Yorumcu ve Veri girişi (misafir de aynı rollerle) kesilmiş görür.
  Ayrı izin eklenmedi: roller sabit izin kümeleridir (ADR 0015) ve künyeyi yazan zaten nesneleri yazandır.
- **K153 — Ne:** künyenin 13 değişkeni (`PERSONAL_PARTICULARS`: `yapi_sahibi`, `tapu_kaydi`, `muteahhit`, `mimar`,
  `sicil_no`, `btb_no`, `buro_adi`, `buro_adres`, `vergi_no`, `tasarlayan`, `cizen`, `kontrol_eden`, `denetciler`) projenin
  ayarlarında, pafta kitabında ve her paftada; teslim kaydının yayımlayanı. Metin değer "Gizli" okunur, öbür türler boş
  (değişkenin türü korunur, ayarlar geçerli kalır). Ada, parsel, il, ilçe, proje ve bina kodu, unvan kişiyi belirlemez: kalır.
  Paftaya düz yazı olarak yazılmış ad değişken değildir, kesilmez.
- **K154 — Nerede kesilir:** projenin bilgisi (`projects::info`: açılış ve her `meta` olayından sonra yeniden okuma) ve
  anlık görüntü (`snapshot::take`'in kesilen kopyası: `GET …/snapshot`, `project.export`). Olaylar yalnız "değişti" der
  (`meta: bool`); yeniden okuma kesilmiş gelir.
- **K155 — Saklanan kopya:** kontrol noktası, dosya projesinin revizyonu ve dışa aktarma işinin dosyası olduğu gibi
  saklanır; künyeyi göremeyen biri için baytları okunur, künye taşıyorsa 403 ("künyenin kişisel bilgilerini taşır; olduğu
  gibi saklandığı için kesilemez"). Taşımıyorsa verilir. Paftanın atlas PDF'i okunamaz (künyeyi basabilir): künyeyi
  göremeyene verilmez. İşleme işinin GeoPackage'ı yalnız nesneleri taşır: verilir.
- **K156 — Arayüz:** Projeyi paylaş penceresinin rol ipuçları: İzleyici ve Veri girişi "Künyenin kişisel bilgileri ona
  gizlidir.", Düzenleyici "künyenin kişisel bilgilerini görür".

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/core/particulars.rs`.
- **Sunucu:** `access::ProjectAccess::sees_particulars`; `projects::{info, require_particulars}`; `snapshot::take`;
  `files::download`, `checkpoints::download`, `jobs::artifact`.
- **Masaüstü:** `shell/cloud/share_plan.rs` (`role_hint`).

## §4. Sınamalar

- **Sözleşme:** `the_kunye_s_personal_particulars_are_cut_and_the_rest_kept` (kesilen değer türünün değeri; ikinci kesme
  bir şey kesmez), `the_sheet_book_s_and_each_sheet_s_particulars_are_cut`. **Pafta:**
  `every_personal_particular_is_a_variable_of_the_kunye`.
- **Sunucu, gerçek PostGIS** (`tests/project_particulars.rs`): veritabanı projesinde Düzenleyici künyeyi tam, İzleyici
  kesilmiş okur (bilgi ve anlık görüntü); kontrol noktası ve sahibin dışa aktarması Düzenleyicinin, İzleyicinin değil;
  dosya projesinde künyesiz ilk revizyon İzleyicinin, künyeli ikincisi değil, sahibe olduğu gibi (SHA-256 aynı).
  `kentos-application`'ın bütün sınamaları (215) geçti; `kentos-api`'de üç denetim işi sınaması macOS'ta Landlock ve seccomp
  olmadığı için düşüyor (değişiklikten önce de; Linux'ta koşar).

## §5. Bilinen sınırlar

- Dosya projesinde künyesi dolu revizyonu İzleyici açamaz (ADR 0550 §6'nın sert reddi gibi). Yolu: revizyon yazılırken
  künyesi kesilmiş ikinci bir kopya saklamak ve listeyi rolüne göre vermek (§6).
- Önce verilmiş kopya, ekranda görülen ve rol düşürülmeden önce indirilen künye geri alınamaz (ADR 0550 §5).
- Yayımlanan PDF/A paftalar ve IFC modeli künyeyi taşır: bunlar müellifin teslim ettiği belgelerdir, bulut projesinin
  okuması değildir.

## §6. Sıradaki adımlar

- Dosya projesinin revizyonunun künyesi kesilmiş kopyası (yazılırken), listesi ve indirmesi role göre.
- Paylaşımda künyeyi ayrıca açma: rolü değiştirmeden bir kişiye "künyeyi görür" vermek (izin kümesinin genişlemesi, ADR
  0015'in yeniden açılması).
