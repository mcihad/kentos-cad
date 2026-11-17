# ADR 8103: IFC revizyonlarını karşılaştır ve sorunları BCF ile değiş tokuş et

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-10 ("Federasyon,
  karşılaştırma ve sorun değişimi"), [bim-plan.md](../../plan/bim-plan.md) B19 ve BIM-17'nin "bulgu açık sorun biçimiyle
  dışa verilir" maddesi. İlk kabul bağımsız okuyucuyla sağlandı (§4); bir masaüstü BCF görüntüleyicisinde elle açılış
  ve çakışmanın BCF'si §6'da.
- **Bağlam:** ADR [8102](8102-underlay-alignment-and-clash.md) (hizalama ve çakışma; IFC-10'un hizalama maddesi),
  [8101](8101-ifc-underlay-model.md) (altlık model), [8061](8061-ifc-viewer-and-reading-engine.md) (okuma motoru, gövdeler),
  [8073](8073-ifc-viewer-first-slice.md) §6 (`cad.ifc.inspect`, değerin dosyada yazıldığı gibi okunması),
  [8076](8076-ifc-mandatory-fields-complete.md) (kesit binasının IFC'si), [1000](1000-review-notes-and-taking-revision-differences.md)
  (inceleme notları), kılavuz §14 (IFC'ye ait her şey `kentos-ifc`'te).
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`IfcCompare`, `IfcCompared`, `IfcChange`, `Bcf*`); iki pencere
  TODOS.md §0.2'de. **`.kcad`:** değişmedi. **Komutlar:** `cad.ifc.compare`, `cad.bcf.write`, `cad.bcf.read` v1.
  **Bağımlılık:** yeni yok; `kentos-ifc` çalışma alanındaki `kentos-zip` ve `roxmltree`'yi kullanır (ikisi de saf, WASM'da
  derlenir).

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| [BCF-XML 3.0 belgesi](https://github.com/buildingSMART/BCF-XML/blob/release_3_0/Documentation/README.md) ve şemaları ([markup](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Schemas/markup.xsd), [visinfo](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Schemas/visinfo.xsd), [extensions](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Schemas/extensions.xsd), [version](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Schemas/version.xsd), [shared-types](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Schemas/shared-types.xsd), [bcf.version](https://raw.githubusercontent.com/buildingSMART/BCF-XML/release_3_0/Documentation/bcf.version)) | Zip: `bcf.version`, `extensions.xml`, konu başına küçük harfli GUID'li klasör (`markup.bcf`, `.bcfv` bakışlar), klasör girdileri merkez dizinde. Konu: Guid, TopicType, TopicStatus zorunlu; Title, CreationDate, CreationAuthor zorunlu; öğeler sıralı (Priority, Labels, ModifiedDate, DueDate, AssignedTo, Stage, Description, Comments, Viewpoints). Bakış: Components (Selection, Visibility DefaultVisibility, Coloring), tam bir kamera (Orthogonal ya da Perspective: göz, yön, yukarı, ölçek ya da görüş açısı, AspectRatio). Tarih `xs:dateTime`, bölgesiz UTC; uzunluk metre, açı derece. 2.1'de yorumlar ve bakışlar konunun yanında, kamera isteğe bağlı | 3.0 yazılır (öğe sırası şemanınki), 3.0 ve 2.1 okunur (K171–K172) |
| [IfcOpenShell IfcDiff](https://docs.ifcopenshell.org/ifcdiff.html) | Nesneler GlobalId ile eşlenir; eklenen, silinen, değişen; öznitelik ve geometri varsayılan, özellikler, kapsayan, tip, sınıflandırma isteğe bağlı; geometri gösterimiyle karşılaştırıldığından aynı gövdenin farklı yazılışı da değişiklik sayılır | Yalnız GlobalId; değişiklik sınıfı, öznitelik, kapsayan, tip, gövde ve özellik (K169); geometri gösterimle değil gövdenin kutusuyla karşılaştırılır: yazılış farkı değişiklik değildir (K170) |
| Solibri'nin model karşılaştırma kuralı ([yardım](https://help.solibri.com/hc/en-us/articles/1500004610621)) ve topluluk notları ([1](https://society.solibri.com/post/9678), [2](https://society.solibri.com/post/3355)) | Aynı modelin iki revizyonu; GUID değişirse geometri ve konumla eşleşmeye çalışır; yanlış eşleşmeden kaçınmak için "yalnız GUID ile tanı" önerilir; eski ve yeni bilinmezse eklenen ile silinen karışır | Eski ve yeni açıkça verilir; eşleşme yalnız GUID ile, tahmin yok; bir dosyada iki kez geçen GlobalId karşılaştırılmaz, listelenir (K169) |
| [Interaxo: model sürümlerini karşılaştır](https://support.interaxo.com/hc/en-us/articles/33428608748957-How-to-compare-model-versions), [BIMvision: değişiklik analizi](https://bimvision.eu/products-and-functionalities/change-analysis-comparing-revisions-in-bimvision/) | Yeni, değişen, silinen ayrı renkte; geometrik ve özellik değişikliği ayrı | Değişen, Eklenen, Silinen ayrı tablo; değişiklik türü (taşındı, boyu, özellik…) ayrı satırda (§5) |
| IfcOpenShell'in BCF kitaplığı (`bcf-client` 0.9.0, görüntüleyicisinin BCF motoru) | BCF 2.1 ve 3.0'ı okur ve yazar; bir nesne için bakış ve kamera kurar | Bağımsız okuyucu ve yazar: KentOS'un dosyasını o okur (§4), okuma durumlarının dosyalarını o yazar |

## §2. Karar

- **K169 — Eşleşme ve kapsam** (`kentos_ifc::compare`): her `IfcObjectDefinition` (proje, mekân yapısı, eleman, tip, grup)
  dosyadaki sırasıyla; aynı GlobalId aynı nesnedir, başka hiçbir şey. Yalnız yenide olan eklenen (yeninin sırasıyla), yalnız
  eskide olan silinen (eskinin sırasıyla), ikisinde olup aşağıdakilerden biri farklı olan değişendir (yeninin sırasıyla).
  Bir dosyada birden çok geçen GlobalId'nin nesneleri karşılaştırılmaz, `ifc_repeated_global_id` uyarısıyla listelenir.
- **K170 — Değişiklikler, bu sırayla:** sınıf; Name, Description, ObjectType, LongName, Tag, PredefinedType; kapsayan mekân
  yapısı ve tip nesnesi (GlobalId'leriyle); gövde (yalnız bir tarafta), kutusunun ortasının toleranstan çok kayması (yeni eksi
  eski, uzaklığıyla), bir eksende boyunun toleranstan çok değişmesi; setlerin değerleri (setin adı, tipten gelip gelmediği,
  değerin adı ile; önce yeninin sırası, sonra yalnız eskide olanlar; dokuz basamağa eşit gerçel sayı ve iki başvuru aynıdır).
  Gövde gösterimiyle değil kutusuyla karşılaştırılır: aynı kutudaki biçim değişikliği burada görülmez (§6).
- **K171 — BCF yazımı** (`kentos_ifc::bcf::write`, `cad.bcf.write`): BCF 3.0. Konu: GUID (verilmezse dosyanın yolu ve
  sırasından), başlık (tek satır, en çok 300), açıklama ve yorumlar (en çok 4000), tür (yoksa Issue), durum (yoksa Open),
  öncelik, etiketler, yazar, gün (yoksa şimdi, UTC), atanan, nesneler (en çok 1000 GlobalId), yorumlar. Nesneli konunun bir
  bakışı olur: nesneleri seçer, öbürleri görünür; kamera nesnelerin gövdelerinin birleşik kutusuna güneybatıdan ve yukarıdan
  (1, 1, −1) yönünde, 60° dikey görüş, 16:9, ortadan köşegen × 1,2 / (2·tan 30°) uzaktan bakar (köşegen en az 1 m),
  yukarısı z'nin bakışa dik parçası. Nesneli konu için model (IFC) gerekir; başlık dosyanın adını ve projesinin GlobalId'sini
  anar. Bakış ve yorum GUID'leri konunun GUID'inden kurallı türetilir (FNV-1a 128, sürüm 8 UUID): aynı girdi aynı baytları
  verir.
- **K172 — BCF okuma** (`kentos_ifc::bcf::read`, `cad.bcf.read`): 3.0, 2.1 ve 2.0; konular klasör adına göre sıralı;
  metinler kırpılır, boş olan yok sayılır; bakışın dosyası yoksa ya da XML okunmazsa `bcf_unreadable` nedeniyle. Seçilen
  nesneler GlobalId'lerine göre birer kez: model verilmişse dosyadaki nesnesi; GlobalId KentOS'un yazdığı bir kalıcı kimliğin
  sıkıştırılmışıysa (`step::unguid`) ve çizimde varsa çizimdeki nesnesi.
- **K173 — Tolerans** (karşılaştırma): metre, 0,0001 ile 0,5 arası, verilmezse 0,001 (ADR 8102 K167 ile aynı gerekçe).
- **K174 — Yer:** karşılaştırma ve BCF `kentos-ifc`'tedir (kılavuz §14); uygulama yalnız yolları denetler, okur ve sözleşmeye
  çevirir. Komutlar çizime yazmaz, geri alma adımı yoktur.

## §3. Uygulama

- **Çekirdek** (`crates/shared/ifc`): `compare.rs` (geometry özelliği: `compare`, `Change`, `same_value`), `bcf.rs` (`write`,
  `read`, `camera_for`, `guid_from`, `topic_problem`).
- **Komutlar:** `crates/shared/commands/src/arch/cad_ifc_compare.rs`, `cad_bcf.rs`; uygulama
  `crates/native/application/src/arch/ifc_compare.rs`, `bcf_exchange.rs` (`write`, `read`); kodlar `ifc_repeated_global_id`
  (uyarı), `invalid_bcf_topic`, `bcf_unreadable`, `ifc_no_body`; `exchange_files::write_bytes` (yaz, geri oku, yerine koy).
- **Masaüstü:** `apps/desktop/src/arch/ifc_compare_window.rs` (`arch.ifcCompare`), `bcf_window.rs` (`arch.bcfIssues`);
  Mimari › Proje ▾'da Altlık modeller'in altında.
- **Python:** `kentos.cad.ifc` (`cad.ifc.compare`), `kentos.cad.bcf`; MCP katalogdan.
- Ayrıca: B16'nın açığı kapatıldı: `cad.delivery.edit`'in 1000 kayıt sınırı artık bir testte sınanıyor (ADR 8099 §4).

## §4. Sınamalar ve ölçümler

- **İlk kabul** (`crates/native/application/tests/all/ifc_revisions.rs`): kesit binasının KentOS'un yazdığı dosyası ile
  IfcOpenShell 0.9.0'ın değiştirdiği ikinci revizyonu (`scripts/fixtures/revision_pair.py`: zemin kattaki iç duvar
  `01alMXGa9m080000000007` 0,50 m kuzeye, doğu duvarının `Pset_WallCommon.FireRating`'i REI 60 → REI 90): rapor tam olarak
  bu ikisi (doğu duvarı özellik, iç duvar taşındı 0,500 m), eklenen, silinen ve tekrarlı yok, kalan 90 nesne aynı. Taşınan
  duvar sorun olarak yazılır, geri okunur: bakış duvarı seçer, kamera duvarın yeni kutusunun ortasına bakar, nesne modelde
  IfcWall'dur ve çizimin duvarına (`0192f5a1-4242-7000-8000-000000000007`) götürür. Dosya her seferinde aynı baytlardır
  (`fixtures/ifc/v1/revizyon/tasinan-duvar.bcf`).
- **Bağımsız okuyucu** (`scripts/fixtures/bcf_check.py`, bcf-client 0.9.0 + IfcOpenShell 0.9.0): dosya BCF 3.0, bir konu,
  bakış `01alMXGa9m080000000007`'yi seçer, IfcOpenShell'de IfcWall "MM-DVR-İç_duvar_15", kutusu 0,105–4,445 × 4,925–5,075 ×
  −0,13–2,78 (ikinci revizyondaki yeri); kameranın ışını kutunun ortasından 6·10⁻⁸ m geçer, 5,433 m önünde; dosya adı ve
  yorum aynı okunur: "uyuyor".
- **Ortak durumlar:** `cad.ifc.compare.json` 28 (`scripts/fixtures/ifc_compare_cases.py`: dosyaları IfcOpenShell yazar,
  beklenenleri ADR kurallarıyla IfcOpenShell'in okumasından); `cad.bcf.read.json` 20 (`bcf_cases.py`: bcf-client'ın 3.0 ve
  2.1 dosyaları, Python'un zip ve XML okuyucusuyla okunur; bozuk dosyalar zipfile ile); `cad.bcf.write.json` 29 (yazılır ve
  aynı durumda okunur; GUID kuralı ve kamera ADR'den, kutular IfcOpenShell'den). Hepsi KentOS'ta geçti.
- **Çekirdek:** karşılaştırmanın üç, BCF'nin altı birim testi (geri okuma, kamera, ret, 2.1, bozuk dosya, GUID kuralı).
- **Masaüstü** (`ifc_revisions_tests`): iki revizyon pencerede karşılaştırılır (özet "2 nesne değişti · 0 eklendi · 0
  silindi · 90 aynı", değişikliklerin sözleri), iki satır işaretlenip BCF'ye yazılır, BCF sorunları'nda okunur, taşınan
  duvarın konusu çizimin duvarını seçer; işaretsiz ya da yazarsız yazma söylenir. Kesişen süzgeçler geçti (157).
- **Resimler:** `.run/gui-evidence/ifc-revizyon-8103/` (karşılaştırma, BCF yazıldı, BCF sorunları, şeritteki yeri; açık ve
  koyu); simge seçenekleri `.run/gui-evidence/ikon-ifc-karsilastir/` (ikisinde de A).
- **Yapılmadı:** BCF dosyalarının resmî XSD'lerle doğrulanması (şemalar indirilmedi; bcf-client'ın şemadan üretilmiş
  modelleriyle okunması ve öğe sırası şemadan alındı); bir masaüstü BCF görüntüleyicisinde elle açılış.

## §5. Arayüz

- **IFC revizyonlarını karşılaştır:** Eski ve yeni revizyon (Seç…), Tolerans, Karşılaştır (iş parçacığında); özet satırı;
  Değişen | Eklenen | Silinen sekmeleri, satır seçilince sağda değişikliklerin sözleri ("Taşındı 0,500 m (x 0,000 · y 0,500 ·
  z 0,000)", "Pset_WallCommon.FireRating REI 60 → REI 90", "(tipinden)"); işaret kutusu; Yazar ($USER ile dolu) ve
  "İşaretlileri BCF'ye yaz…" (kaydetme penceresi; sorunlar yeni revizyonun dosyasına bağlanır).
- **BCF sorunları:** BCF dosyası (Aç… okur), Model (IFC, isteğe bağlı), Oku; konular solda (başlık, tür, durum, nesne
  sayısı), sağda tür · durum · öncelik, yazan ve gün, atanan, etiketler, model, açıklama, yorumlar ve nesneler (modelde,
  çizimde); "Çizimde seç".

## §6. Sıradaki adımlar

- **Çakışmanın BCF'si:** Altlık modeller › Denetim'in çakışma satırlarından sorun (iki modelin nesneleri, kamera iki
  dosyanın ortak harita çerçevesinde); BIM-17'nin "bulgu kaynağına döner" maddesi.
- **Sorunların çizimde saklanması:** BCF konusunun inceleme notu olarak alınması ve notların BCF olarak verilmesi (ADR 1000'in
  modeli; nesne kimliği dosyanın GlobalId'si).
- **Görüntü:** bakışın PNG görüntüsü (3B görünümden); renklendirme ve kesit düzlemi.
- **Biçim değişikliği:** kutu içinde kalan gövde değişikliği (hacim ya da yüzey özetiyle) ve tip ile malzemenin derinliği.
- **Gerçek pencerede kabul** (ADR 8000) ve bir masaüstü BCF görüntüleyicisinde aynı nesnenin görülmesi.
