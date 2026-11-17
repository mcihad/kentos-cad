# ADR 8099: Teslim kaydı — paketin idareye teslimi, iadenin nesneye bağlı gerekçesi ve öncekiyle karşılaştırma

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-15 (uygulama sırasında
  B16). İlk kabulün dört adımı sağlandı (§4); "yetkisiz kullanıcı künyeyi göremez" ADR
  [8100](8100-kunye-personal-particulars-by-role.md)'de.
- **Bağlam:** ADR [0617](0617-complete-drawing-set-delivery.md) (yayının `yayin.json`'u ve SHA-256'sı),
  [1320](1320-sheet-sources-and-update-publication.md) (yayının kaynakları, `kaynaklar/cizim.kcad`),
  [1000](1000-review-notes-and-taking-revision-differences.md) (inceleme notları, nesneye bağlı, eskime),
  [0576](0576-per-view-time-scale-link-and-revisions.md) ve [0179](0179-data-compare.md) (kalıcı kimlikle karşılaştırma),
  [8066](8066-2d-permit-package-pdfa.md) ve [8075](8075-bim-permit-package.md) (ruhsat paketi),
  [0031](0031-cloud-file-projects.md) (bulut dosya projesinin revizyonu).
- **Dayanak:** e-PYS Yönetmeliği (RG 5.8.2026/33331) Md. 4–7; Dijital Proje Hazırlama Yönetmeliği Md. 4/4 ve EK-2
  (dosya adının revizyon numarası); TS EN ISO 19650-1/2'nin ortak veri ortamı.
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`Delivery*.ts`, `ArchSettings.ts`); pencere ve Mimari verisi 17'nin
  okuyucusu TODOS.md §0.2'de. **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 17 (`settings.arch.deliveries`).
  **Sözleşme:** yeni `Delivery`, `DeliveryPackage`, `DeliveryResponse`, `DeliveryChannel`, `DeliveryOutcome`; komut
  `cad.delivery.edit` v1. **Bağımlılık:** yok.

## §1. Araştırma (17 Kasım 2026)

e-PYS maddeleri [araştırma notunun](../research/2026-11-15-bim-ifc-yonetmelik.md) §1.2'sinden; ortak veri ortamı ve
iletim kaydı aşağıdaki kaynaklardan.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| e-PYS 4/1–2 | Teslim, inceleme, eksik bildirimi, onay ve tebligat Bakanlığın ya da yetkilendirdiği idarenin e-PYS'sinde | Kanal "e-PYS" varsayılan (K147) |
| e-PYS 4/3, 7/7 | İdarelerin kendi yazılımları; oradaki işlemler e-PYS'ye aktarılır ve kayda alınır | Kanal "İdarenin yazılımı" |
| e-PYS 5/1–2 | Dosyayı müellif kaydeder; eksik ya da hatalı dosya yeniden yüklenir | İadeden sonra düzeltilmiş paket yeni bir teslimdir; önerilen revizyon bir artar (K151) |
| e-PYS 5/3 | İdare eksikleri gerekçesiyle bildirir | İadede gerekçe zorunlu; gerekçenin nesnedeki karşılığı inceleme notu (K148) |
| e-PYS 5/4, 5/6 | Kimlik doğrulama, yöntem ve kılavuz Bakanlıkça; 15 Kasım 2026'da kılavuz yok | Yükleme yapılmaz; sistemin makbuz numarası elle yazılır |
| e-PYS 6 | E-imzasız PDF/A ya da IFC tesliminde ayrıca basılı teslim | Kanal "Basılı (elden)" |
| EK-2 (dosya adı) | Revizyon numarası dosya adının alanı: üç rakam, 000 revizyonsuz | `revision` üç rakam; modelin adı revizyonla değişir (K150) |
| [UK BIM Framework, Guidance Part C (2021)](https://www.ukbimframework.org/wp-content/uploads/2021/02/Guidance-Part-C_Facilitating-the-common-data-environment-workflow-and-technical-solutions_Edition-1.pdf) | ISO 19650'nin ortak veri ortamı: çalışma → paylaşım → yayın → arşiv; her geçiş bir onay; bilgi kabının durum ve revizyon kodu; dışarıya gidenin iletim kaydı | Teslim kaydı yayımlanmış paketin iletim kaydıdır; paket yayının özetiyle anılır |
| [Durum kodları (ISO 19650 UK ulusal eki)](https://help.autodesk.com/cloudhelp/ENU/Docs-Files/files/file-naming-standard/Status_Codes.html) | S0–S7, A1–A5, B1–B5, CR: paylaşım ve yayın amaçları | Türkiye'nin ulusal eki ve EK-2 bu kodları tanımlamıyor: eklenmedi, değer uydurulmaz |
| [ISO 19650'nin amacı](https://bimfix.blogspot.com/2025/12/what-is-purpose-of-iso-19650.html) | Bilginin kim tarafından, ne zaman, hangi durumda paylaşıldığının izlenebilirliği | Kim (yayımlayan), ne zaman (gün), ne (paket özeti), kime (kurum), ne geldi (cevap) |
| ADR 0617, 1320 | Yayın klasörü `yayin.json` ile kendini tanıtır; kaynaklarıyla yayımlanan paket çizimi `kaynaklar/cizim.kcad`'da taşır | Paket `yayin.json`'un SHA-256'sıyla anılır; çizimler oradan karşılaştırılır |

## §2. Karar

- **K147 — Kayıt çizimde** (`settings.arch.deliveries`, Mimari verisi 17, en çok 1000): her teslim kimlik, sıra numarası (en
  büyüğün bir fazlası), EK-2 revizyonu (üç rakam), paket (yayının `yayin.json`'unun SHA-256'sı, dosya sayısı, klasör,
  yayımlandığı çizim revizyonu), kurum, kanal (e-PYS, idarenin yazılımı, basılı, öbür), gün (YYYY-AA-GG), yayımlayan, makbuz
  ve cevap taşır. Dosyanın okuyucusu yapıyı, komut çizimi (notun varlığını) denetler. Çizimle gider: bulut dosya
  projesinde projenin revizyonuyla (ADR 0031).
- **K148 — Cevap:** Kabul ya da İade, günü (teslimden önce olamaz), gerekçesi (iadede zorunlu), makbuzu ve gerekçeyi
  nesnelere bağlayan inceleme notları (ADR 1000; kimlikleriyle, her biri bir kez, çizimde var olmalı). Not nesnenin o
  günkü damgasını tutar: düzeltilince "eskimiş" görünür. Kayıt silinince notlar durur.
- **K149 — Komut `cad.delivery.edit` v1:** `add` (paket yazıldı), `respond` (cevap, öncekinin yerine), `remove` (yanlış
  kayıt); her biri tek geri alma adımı "Teslim kaydı"; çizimin geometrisi değişmez. Ret sırası: `invalid_uid`,
  `delivery_not_found`, `duplicate_delivery`, `review_note_not_found`, `invalid_delivery` (alanın yolu),
  `too_many_deliveries`, revizyon. Python (`kentos.cad.delivery`) ve MCP aynı komutu çağırır.
- **K150 — Öncekiyle karşılaştır** (`kentos_native_application::delivery_compare`): seçilen teslimin paketi, aynı kuruma
  ondan önceki son teslimin paketiyle. İki klasör yeniden okunur; `yayin.json`'un SHA-256'sı kayıttakinden farklıysa ret
  ("klasör sonradan değişmiş"). Dosyalar önce taşıdıkları paftalarla, sonra adla, sonra içerikle, en son paftasız
  dosyalardan bir uzantıda her yanda tek kalanla eşlenir (modelin EK-2 adı revizyonu taşır); Aynı, Değişti, Adı değişti,
  Yeni, Kalktı. İki paket de çizimi taşıyorsa nesneler kalıcı kimlikle karşılaştırılır (ADR 0576'nın çekirdeği, 1 mm) ve
  önceki iadenin her notu için bağlı nesnelerden kaçının değiştiği, kaçının silindiği yazılır. Pencerenin iş parçacığı
  dışında çalışır, yazmaz. Salt okuyan bir komut olarak Python ve MCP'ye açılması sıradaki iştir (§6); çizimlerin farkı
  bugün `cad.drawing.compare` ile alınabilir.
- **K151 — Pencere** `arch.deliveries` "Teslim kaydı" (Mimari › Proje ▾, Model kalite kontrol formunun altı): solda
  teslimler (No, Rev., Kurum, Gün, Cevap), sağda seçilenin kaydı, İdarenin cevabı ve Öncekiyle karşılaştır (DESIGN.md §7.9).
  Teslim ekle yayın klasörünü seçtirir; önerilen revizyon son teslimin revizyonu, son teslim iade edildiyse bir fazlası;
  kurum ve kanal son teslimden. Seçilenlere bağla yazılan notu seçili nesnelere bağlar ve cevabın notlarına ekler; Cevabı
  yaz notlarla birlikte yazar.

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/arch/deliveries.rs` (`Delivery::list_problem`, `Delivery::previous`),
  `ArchSettings::deliveries`, kodlar `invalid_delivery`, `delivery_not_found`, `duplicate_delivery`, `too_many_deliveries`.
- **Dosya:** `kentos_kcad::{ARCH_WITH_DELIVERIES, uses_deliveries}`; eski KentOS teslim kaydını taşıyan dosyayı "mimari
  verisi sürüm 17" diye reddeder.
- **Komut:** `crates/shared/commands/src/arch/cad_delivery.rs`; uygulama `arch/delivery_edit.rs`; alan
  `Document::{deliveries, set_deliveries}`, geri alma `ArchOp::Deliveries`; katalog, yetenek, MCP'nin yıkıcı listesi,
  geçmişin adı.
- **Karşılaştırma:** `crates/native/application/src/arch/delivery_compare.rs` (`read`, `files`, `compare`).
- **Masaüstü:** `apps/desktop/src/arch/deliveries_window.rs`; ikon `delivery` (belge ve gönderme oku).

## §4. Sınamalar

- **Sözleşme:** `a_delivery_is_written_and_checked_as_the_record_of_a_package`. **Dosya:**
  `the_delivery_record_round_trips_under_arch_data_17`.
- **Komut durumları** (`fixtures/commands/v1/cad.delivery.edit.json`, 40; bağımsız başvuru
  `scripts/fixtures/delivery_cases.py`, KentOS kodu yok): ekleme, numara, kırpma, cevap, iade gerekçesi, notların varlığı
  ve tekliği, gün sırası, silme, sınırlar, revizyon. 1000 kayıt sınırı (`too_many_deliveries`) ortak durumun çiziminde bin
  kayıtla `delivery_limit` testinde sınanır (17 Kasım 2026, ADR 8102'nin işinde eklendi): bin birincisi `change`'te reddedilir,
  çizime bir şey yazılmaz.
- **Karşılaştırma:** dosyaların eşlenmesi (pafta, ad, içerik, tek uzantı; iki adayda tahmin yok), klasörün okunması.
- **Masaüstü** (`deliveries_tests`, ilk kabul elle): `dated_sizes_tests`'in evi; ilk paket 2.11.2026'da e-PYS ile gider,
  9.11.2026'da iade gelir; gerekçesiz iade reddedilir; gerekçe WC kapısına ve girişe bağlanan bir notla yazılır; WC kapısı
  1,10 m'ye genişletilir, ikinci paket 001 olarak gider; karşılaştırma: zemin kat paftası değişti, kesit aynı, model adı
  revizyonla değişip eşlendi; çizimde 1 nesne değişti, eklenen ve silinen yok; gerekçenin iki kapısından biri değişti.
  Cevap ve silme tek geri alma adımı; klasörü sonradan değişen paket karşılaştırılmaz.
- **Resimler:** `.run/gui-evidence/teslim-8099/` (iade, yeni teslim, karşılaştırma, şeritteki yeri; açık ve koyu).

## §5. Arayüz

- "1. teslim · Rev. 000", "İdarenin cevabı": Sonuç (Kabul | İade), Gün, Gerekçe, Notlar, Yeni not + Seçilenlere bağla,
  Makbuz, Cevabı yaz.
- Karşılaştırma: "Dosyalar: 1 aynı, 2 değişti." tablosu (Dosya, Önceki adı "…_000.ifc", Durum); "Çizim: 13 nesne aynı,
  1 değişti, 0 eklendi, 0 silindi."; gerekçelerin tablosu (Not, Gerekçe, Nesne, Değişen, Silinen).

## §6. Sıradaki adımlar

- ~~**Künyenin görünürlüğü**~~ (ilk kabulün son cümlesi): yapıldı, ADR [8100](8100-kunye-personal-particulars-by-role.md).
- **Ortak veri ortamının durumları:** çalışma, paylaşım, yayın, arşiv; kabul edilen teslimin paketinin arşive geçmesi.
- **Salt okuyan komut** `cad.delivery.compare`: K150'nin cevabı Python ve MCP için.
- **e-PYS pilotu:** Bakanlığın arayüzü, imza yöntemi ve deneme ortamı yayımlanınca (e-PYS 4/4, 5/4, 5/6).
