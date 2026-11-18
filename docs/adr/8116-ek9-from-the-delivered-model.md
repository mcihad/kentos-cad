# ADR 8116: Denetim EK-9 formunu son teslim edilen modelden okuyabilir; çizimin teslimden bu yana farkı

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-08'in kalanı ("son yazılan dosyanın
  denetimi"), [bim-plan.md](../../plan/bim-plan.md) B13 ve K4 ("Formun cevabı bellekteki modelden değil, diske yazılan IFC
  ve PDF/A'dan okunur").
- **Bağlam:** ADR [8074](8074-ek9-from-the-written-file.md) §8 ("Son yazılan dosya"), [8075](8075-bim-permit-package.md)
  (BIM ruhsat paketi: model, form kaydı, `yayin.json`'un BIM bölümü), [8099](8099-delivery-record.md) (teslim kaydı ve
  paketin klasörü), [8103](8103-ifc-revisions-and-bcf.md) (iki IFC revizyonunun GlobalId'yle karşılaştırılması),
  [8104](8104-administration-package-review.md) (gelen paketin dosyalarından incelenmesi), [8115](8115-model-mapping-off-the-window-thread.md)
  (eşleme ve kurallar kuralın iş parçacığında).
- **Web, `.kcad`, komut, kural paketi:** değişmedi. Paylaşılan yeni kod `kentos_native_application::delivered_model`.

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Dijital Proje Yönetmeliği Md. 15 ([araştırma notu](../research/2026-11-15-bim-ifc-yonetmelik.md) §1.2) | Teslim edilen her model için EK-9 model kalite kontrol formu | Form teslim edilen modelindir: Denetim onu teslim edilen dosyadan okuyabilir |
| bim-plan K4 | Formun cevabı diske yazılan IFC'den okunur; aynı okuyucu idare incelemesine de hizmet eder | Paketin klasöründeki dosya `kentos_ifc::read::file_facts` ile okunur (ADR 8104'ün okuyucusu) |
| Tekla Structures, [Check the exported IFC model](https://help.trimble.com/doc/tekla-structures/2025/int_checking_exported_ifc_model) | Dışa aktarılan IFC başvuru modeli olarak asıl modele eklenir; nesne sayıları karşılaştırılır, farklıysa dışa aktarımın günlüğüne bakılır | Teslim edilen dosya çizimin şimdi yazacağı modelle karşılaştırılır; fark nesne sayısıyla söylenir |
| Archicad, [Detect IFC Model Changes](https://help.graphisoft.com/AC/23/INT/_AC23_Help/115_IFC/115_IFC-9.htm) | Aynı modelin iki IFC sürümü GlobalId'yle karşılaştırılır: yeni, silinmiş, değişmiş öğe; GlobalId tutmazsa hepsi yeni ya da silinmiş sayılır | ADR 8103'ün karşılaştırması aynı kuralla (GlobalId, 1 mm tolerans) |
| buildingSMART, [IFC Validation Service](https://buildingsmart.github.io/validate/user/index.html) | Yüklenen `.ifc` dosyasının standarda uygunluğunu denetler; ulusal ve kuruma özgü kuralları denetlemez, IDS ayrı bir denetimdir | Ulusal kural (EK-9) KentOS'un formudur ve teslim edilen dosyada koşar |
| Vectorworks, [Checking IFC models with Solibri](https://app-help.vectorworks.net/2024/eng/VW2024_Guide/IFC/Checking_IFC_models_with_Solibri.htm) | Model dışa aktarılır, denetim programı dışa aktarılan IFC'yi açar; denetim dışa aktarım bitince başlar | Denetlenen, yazılmış dosyadır; çizimin hâli ayrıca söylenir |

## §2. Karar

- **K222 — Hangi model:** teslim kaydında (`settings.arch.deliveries`) numarası en büyük olan ve listesinin (`yayin.json`) BIM
  bölümü bir model adı taşıyan teslim. Klasörü kayıtlı olmayan teslim ve yalnız paftalı paket atlanır. Klasörü olmayan ya
  da değişmiş teslim aramayı durdurur ve söylenir: tuttuğu model bilinmez.
- **K223 — Teslim edildiği gibi:** klasördeki `yayin.json`'un SHA-256'sı kayıttakiyle aynıdır. Model, form kaydı
  (`kalite-kontrol-formu.json`) ve paketin taşıdığı çizim (`kaynaklar/cizim.kcad`, listenin `resources`'ında) listedeki
  özetleriyle aynıdır. Listedeki ad klasörün kendi dosyasıdır (`kaynaklar/` altı dahil); dışına çıkan yol reddedilir. Aksi
  hâlde model okunmaz; neden, dosyasıyla söylenir.
- **K224 — Seçim:** Denetim varsayılan olarak çizimin şimdiki modelini okur (iadeden sonra düzeltilen çizim). Tablonun
  üstündeki satır son teslimi (numarası, modelin EK-2 adı, günü) ve çizimin ondan farkını söyler. "Teslim edileni
  denetle" formu teslim edilen dosyadan okutur, "Şimdiki çizimi denetle" geri döndürür. Teslim edilen modelden gelen
  bulgular dosyanın nesneleridir: çizimde artık olmayan nesne seçilemez. Düzeltme dosyayı değil çizimi değiştirir; düzeltilmiş
  model için paket yeniden yayımlanır.
- **K225 — Fark:** çizimin şimdi yazacağı model ile teslim edilen dosya GlobalId'yle karşılaştırılır (ADR 8103, 1 mm):
  eklenen, silinen ve değişen nesne sayısı. Model aynı binanın modelidir: dosyanın EK-2 adını taşıyan program binası, yoksa
  bütün proje. Yayının pini (çizimin tamamının özeti, ADR 1090) bunun için kullanılmaz: teslimi kaydetmek bile onu
  değiştirir. Sınamanın ilk koşusunda değişmemiş kesit binası başka bir zaman damgasıyla 23 eklenen, 23 silinen ve 59
  değişen nesne gösterdi. Yazıcı KentOS kimliği olmayan parçaların GlobalId'sini dosyanın adı, zamanı ve sırasıyla
  üretiyordu. Bu, ADR [8117](8117-persistent-global-ids.md) ile düzeltildi: türetilen parçaların GlobalId'si sahibinden ve
  rolünden türer, aynı çizim her gün aynı kimlikleri verir. Bu karardan önce yazılmış bir dosya nesne nesne
  karşılaştırılmaz; satır bunu söyler (ADR 8117 K232).
- **K226 — Beklenti:** 8., 17. ve 21. sorular kaynağın nesnelerini (beklenen nesneler, dışa aktarımın bıraktıkları) okur.
  Bunlar paketin taşıdığı çizimden alınır. Paket çizim taşımıyorsa ve model şimdiki çizimle aynıysa şimdiki çizimden
  alınır. İkisi de yoksa hiç verilmez: form üç soruyu incelemeye bırakır, satır da nedenini söyler. Boş liste verilmez;
  21. soru boş bir dışa aktarım kaydıyla yanlışlıkla geçerdi.
- **K227 — 18. soru dosyanın kendisinden:** şema ve uzantı teslim edilen dosyada cevaplanır. Bellekte yazılan dosyada bu
  soru "teslim edilen dosyayı denetleyin" diye incelemeye kalır.

## §3. Uygulama

- **Paylaşılan:** `crates/native/application/src/arch/delivered_model.rs`: `last` (teslim kaydından son model),
  `Delivered::{drift, read}`, `Drift`, `Expected`. Klasör ve özet sözleri ADR 8099 K150'nin karşılaştırmasıyla aynıdır.
  `ifc_facts::Planned::at` (binanın çizimdeki yeri, nesnesiz bulgu için).
- **Masaüstü:** `apps/desktop/src/arch/arch_audit/standards.rs` (`model`: okunan modelin olguları ve satırın cevabı;
  `DeliveredLine`), `arch_audit/mod.rs` (`delivered_model`, `delivered`, `Event::DeliveredModel`), `arch_audit/view.rs`
  (satır ve düğmesi). Dosyanın okunması ve karşılaştırma kuralın iş parçacığındadır (ADR 8115 K220).

## §4. Sınamalar

- **Birim** (`delivered_model::tests`, 3): yalnız paftalı paket model taşımaz, teslimsiz çizim; teslimden sonra yeniden
  yazılan liste, değişen model, kaybolan model ve klasör ayrı sözlerle; klasörün dışına çıkan ad reddedilir.
- **Masaüstü** (`delivered_model_tests`, 3): yazar gibi doldurulmuş kesit binasının paketi (gerçek IFC modeli, form kaydı,
  liste, taşınan çizim) teslim kaydına komutla girer. Denetim satırı çizimden okurken "1. teslimin modeli: …, 18.11.2026.
  Çizimin modeli teslim edilenle aynı." der. Teslim edileni denetleyince bulgular aynı kalır. Yarım metre taşınan duvar
  sayılır, form dosyadan okumaya devam eder. 18. soru bellekteki dosyada incelemeye kalırken teslim edilende "IFC4X3_ADD2,
  .ifc." diye geçer. Çizim taşımayan paket değişikliğe kadar beklentiyi şimdiki çizimden alır; değişiklikten sonra üç soruyu
  incelemeye bırakır ve bunu söyler. Teslimden sonra yeniden yazılan listede satır nedeni söyler ve form çizime döner.
- **Resim** (`.run/gui-evidence/teslim-modeli-8116/`): çizimden ve taşınan duvardan sonra teslim edilenden (açık ve koyu).

## §5. Arayüz

Denetim'in tablosunun üstünde, kural seti satırının altında bir satır vardır; yalnız teslim kaydında modelli bir paket
varken görünür:

- "EK-9 formu çizimin şimdiki hâlinden. 1. teslimin modeli: …_000.ifc, 18.11.2026. Çizimin modeli teslim edilenle aynı."
  [Teslim edileni denetle]
- "EK-9 formu 1. teslimin modelinden (…_000.ifc, 18.11.2026). Teslimden bu yana çizimde 3 nesne değişti, 1 nesne
  eklendi." [Şimdiki çizimi denetle]
- "1. teslimin modeli okunamadı: … EK-9 formu çizimin şimdiki hâlinden." (uyarı simgesiyle)

## §6. Sıradaki adımlar

- **Türetilen parçaların kararlı kimliği:** yapıldı, ADR [8117](8117-persistent-global-ids.md) (kat, açıklık, zon, bağımsız
  bölüm grubu, setler ve ilişkiler sahiplerinden ve rolden; K225'in zaman damgası kalktı).

- **Bulut işi:** aynı betiğin sunucuda koşması: IFC olgularının sunucuda çıkarılması, bulut portalındaki bir başvuru
  dosyasının IFC'sinin idarece denetlenmesi (ADR 9000 ile).
- Salt okuyan bir komut olarak Python ve MCP'ye açılması (son teslimin modeli ve farkı).
- Çizimdeki form tablosunun ve Ruhsat takımının teslim edilen modelden okunması (bugün bellekteki modelden; paketin form
  kaydı yayın anında dosyadan yazılıyor, ADR 8075).
- Paketin program binasını kaydetmesi: EK-2 adı sonradan değişen dosyada bina adla bulunamıyor, bütün projeyle
  karşılaştırılıyor.
