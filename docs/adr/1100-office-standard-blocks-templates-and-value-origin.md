# ADR 1100: Ofis standardının blok kitaplığı, pafta şablonu, araç paleti ve değerin kökeni

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12`'nin ADR 1002 §7'de açık kalan işi:
  blok kitaplığı yayınının kurulması, onaylı pafta şablonu sürümünün çizime kaydı, yayımlanan ofis araç paleti, “bu
  değer nereden geliyor” açıklaması ve iki ofis profilinin uçtan uca kabul senaryosu. Sahibin sözü: maddeyi alt
  maddeleriyle birlikte tek dilimde bitir.
- **Bağlam belgesi:** ADR 1002 (`cad.standards.install` v1), [docs/specs/drawing-standard-package.md](../specs/drawing-standard-package.md)
  (sürüm 2 paketin `resources.blocks` ve `sheetTemplates` alanları), ADR 0314 §1 (görünüm penceresi katman
  geçersiz kılması), ADR 0312 §4 (görünüşün kaynağı), ADR 0318 §4 (KentOS blok kitaplığı).

## 1. Araştırma

- **Blok yeniden tanımı.** AutoCAD'de çizimde aynı adlı blok varken başka bir dosyadan blok alınırsa “Block already
  exists. Redefine it?” sorulur; evet denince tanım değişir ve **var olan bütün yerleştirmeler yeni tanımla çizilir**
  ([AutoCAD LT 2026 — -INSERT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-BB831F94-6385-4490-8DE9-7C565CD1B639.htm)).
  DesignCenter'ın “Redefine Block” seçeneği yalnız aynı adlı tanım varken çıkar; düz sürükle-bırak çizimdekini korur.
  Yeniden tanım ayrı, açık bir eylemdir.
- **CAD Standards (DWS).** Standart denetimi adlandırılmış kaynakları karşılaştırır ve eşleme tablosuyla düzeltir;
  eşlenmeyeni kendiliğinden ezmez ([AutoCAD 2027 — CAD Standards](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-D64F8076-4978-44B7-B056-D921C77FEA88.htm)).
  ADR 1002'nin missing/same/differs ve add/keepLocal/replace/renameIncoming düzeni bunun karşılığıdır.
- **Pafta oluşturma şablonu.** Sheet Set Manager'da takımın **Sheet Creation Template**'i (DWT ve içindeki düzen)
  yeni paftaların hangi şablondan yapılacağını söyler; alt kümeler kendi şablonunu taşıyabilir; şablonu değiştirmek
  var olan paftaları değiştirmez ([AutoCAD Architecture 2024 — To Create a Sheet Set Template](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Architecture/files/GUID-F252AB08-A017-4FEF-B03F-73673B031B09.htm);
  [Autodesk blog — Becoming a Master of Sheet Sets](https://www.autodesk.com/blogs/autocad/create-new-sheets-autocad/)).
- **Ofis araç paleti.** AutoCAD palet ve palet gruplarını XTP/XPG dosyası olarak dışa/içe aktarır; içe aktarma
  mevcut gruplarla birleşir, silinen grup geri gelmez ([AutoCAD LT 2027 — Customize Dialog Box](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT/files/GUID-0893BB38-5B13-435B-A157-2271D449BF55.htm)).
  Civil 3D'de paletler bir **araç kataloğuna** konur, katalog paylaşılan konuma yayımlanır; katalogdan bırakılan araç
  kataloğa bağlı kalabilir ya da bağsız olup yerel düzenlemeyi korur ([Civil 3D 2022 — Adding Content to a Tool Catalog](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-ContentBrowser/files/GUID-7A2B22C3-5064-4500-A53F-F4AB901850C0.htm);
  [Civil 3D 2016 — Editing Tool Catalog Properties](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-ContentBrowser/files/GUID-BD64F249-C7BE-4972-B386-86150A066E08.htm)).
- **Değerin kökeni ve geçersiz kılma sıfırlama.** Blok parçalarının rengi, çizgi tipi, kalınlığı ve saydamlığı
  yerleştirmede katmandan, bloktan (BYBLOCK) ya da kendi değerinden gelir; iç içe blokta aynı kural her düzeyde işler
  ([AutoCAD LT 2020 — Block Object Properties Reference](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-LT/files/GUID-25E9F20C-D146-426C-8815-37DF48D2D33F.htm)).
  Görünüm penceresi katman geçersiz kılması “yalnız bu pencerede” ya da “bütün pencerelerde” kaldırılabilir; kapsam
  sıfırlamadan önce seçilir ([AutoCAD LT 2019 — To Remove an Override From a Layer for All Layout Viewports](https://help.autodesk.com/cloudhelp/2019/ENU/AutoCAD-LT/files/GUID-E90A8FA1-A25D-4F7F-8424-E400951063D2.htm);
  [AutoCAD Architecture 2024 — To Remove Viewport Layer Overrides](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Architecture/files/GUID-02C58E45-05B7-469E-A511-FBDE1F379989.htm)).

**KentOS kararı:** standarttaki blok bir dosya ya da yerel ad değil **yayın kimliğidir** (aile, tip, yayın); okunabilen
kitaplıklardan çözülür ve ADR 1002'nin aynı tablosuna satır olur. Yeniden tanım yalnız açık “Değiştir” seçimiyle,
çizimdeki ad ve kimlik korunarak yapılır; yerleştirmeler kimlikle izler. Pafta şablonu için kopya değil onaylı sürümün
**kaydı** tutulur; var olan paftalar değişmez. Değerin kökeni özellik panelinde düzey düzey gösterilir, sıfırlamanın
kapsamı yazmadan önce sayılır.

## 2. Blok kitaplığı yayınının kurulması

`cad.standards.install` v1'in satır türlerine `block` ve `sheetTemplate` eklendi (`StandardResourceKind`). Eski
girdiler aynı cevabı alır: blok ve şablon başvurusu olmayan standartta satır sayısı değişmez; çıktıya her durumda
`unavailable` sayacı eklendi (eski 25 ortak durumda `0`).

**Çözümleyici** (`crates/native/application/src/standard_libraries.rs`): `Libraries` sıralı blok ve şablon
kitaplıklarıdır. Bir blok başvurusu, yayınının aile, tip ve yayın numarası eşit olan **ilk** kitaplık bloğuna çözülür;
başvuru birim veriyorsa birim de eşit olmalıdır. Bugün KentOS'un kendi kitaplıkları okunur: `kentos-temel` ve bu ADR
ile eklenen, yayın kimlikli `kentos-ofis` (`catalog/v1/blocks/kentos-ofis.kblok`, `scripts/blocks/office_library.py`;
kapının 1. ve 2. yayını yan yana, kuzey oku, kot işareti). Proje, ofis klasörü ya da bulut kitaplığı konağın okuduğu
`BlockLibrary` olarak `Libraries::with_blocks` ile önüne takılır; komutun kuralları değişmez (`plan_with`,
`execute_with`, `validate_with`). Kayıttan çalışan komut (Python, MCP, başsız) bugün yerleşik kitaplıkları okur.

Durumlar ve seçimler:

| Durum | Ne zaman | Varsayılan | İzinli |
| --- | --- | --- | --- |
| missing | Kitaplıkta var, çizimde o adda blok yok | add | add, keepLocal |
| same | Çizimdeki o adlı blok kitaplıktakiyle aynı | keepLocal | keepLocal |
| differs | Aynı adlı blok farklı (`fields`: base, entities, attributes, units, dynamic, publication) | keepLocal | keepLocal, replace, renameIncoming |
| unavailable | Hiçbir okunabilen kitaplık yayını tutmuyor | keepLocal | keepLocal |

- Karşılaştırma kimliği, adı, açıklamayı ve kartı (yayın kimliği dışında) dışarıda bırakır; içteki yerleştirme
  bloğunun adıyla karşılaştırılır; sayılar 1e-9 içinde eşittir.
- `unavailable` satırı **açık bulgudur**: `fields` en yakın girdinin farkını söyler (`release`: kitaplıkta tipin başka
  yayını var; `units`: yalnız birim farklı; boş: tip hiç yok). Sessizce atlanmaz; `unavailable` sayacına ve `kept`'e
  girer. Kurulmak istenirse `invalid_install_choice`.
- `add`: kitaplık bloğu başvurunun adıyla eklenir; kimliği çizimde yoksa kitaplığın kimliğini alır, varsa yenisini.
- `replace`: çizimdeki **ad ve kimlik korunur**, tanım kitaplığınkidir; bütün yerleştirmeler (çizimdekiler ve başka
  tanımların içindekiler) kimlikle izler; dinamik yerleştirmenin önbelleği aynı adımda yenilenir
  (`block_params::refresh_definitions`). Çizimdeki blok dış başvuruysa değiştirilmez (`invalid_install_choice`).
- `renameIncoming`: kitaplık bloğu yeni adla eklenir; ad çizimde ya da standartta blok adıysa `duplicate_name`.
- Kitaplık bloğunun yerleştirdiği iç tanım çizimde o adla varsa o kullanılır, yoksa aynı kitaplıktan bir kez eklenir;
  kitaplıkta da yoksa ya da sonuç blok kurallarını bozuyorsa `block_refused` (yeni ret kodu; KentOS kitaplıkları
  kurallara uyduğu için ortak durumla ulaşılamaz, `office_profiles::a_library_block_that_breaks_the_block_rules_is_refused`
  sınar).
- Hiçbir tanım silinmez; yerleştirmesi olan tanım hiçbir yolda kaldırılmaz. `uses` satırda çizimdeki o adlı
  tanımın yerleştirme sayısını verir (değiştirilirse izleyecek olanlar).

## 3. Onaylı pafta şablonu sürümü

Sürüm 2 paketin `sheetTemplates` başvurusu (`templateId`, `revision`), şablon kitaplıklarında aranır: bugün sistem
şablonları (`sistem`: `crates/shared/sheet/templates/*.json`); kullanıcının ve ofisin şablon kitaplığı
`Libraries::with_templates` ile önüne takılır. Kayıt yeri yeni proje ayarı
`approvedSheetTemplates: [{templateId, revision, standard, standardRevision}]`'dir (**KCAD şema 183**, CAD-ADV-12
dalında geçici numara; birleşmede yeniden numaralanır): yeni paftaların hangi şablon sürümünden yapılacağının
projedeki notu ve onu onaylayan standardın kimliği/revizyonu.

- `missing` (kayıt yok) → add; `same` (aynı revizyon) → keepLocal; `differs` (`revision`) → keepLocal ya da replace
  (kayıt yerinde güncellenir, onaylayan standart da yazılır). Şablon kimliğiyle kaydedildiği için `renameIncoming`
  reddedilir (`invalid_install_choice`).
- Kitaplıkta bulunmayan kimlik ya da revizyon `unavailable` bulgusudur (`fields`: `revision` başka revizyon listeleniyorsa).
- **Var olan paftalar değişmez.** `uses` o şablondan yapılmış pafta sayısıdır; paftanın `origin`'i olduğu gibi kalır
  ve `cad.standards.check` paftanın kökeni ile onaylı sürüm arasındaki farkı bildirmeye devam eder.
- Undo: kayıt yeni `Op::ApprovedSheetTemplates` ile aynı “Standart kaynaklarını kur” adımındadır; blok yazımı ve
  stil/desen tabloları da bu adımın içindedir (`Document::transact`), tek Undo hepsini geri getirir.
- Dosya sınırı: en çok 256 kayıt, kimlik 1–128 bayt, büyük-küçük harf ayrımsız tekil, revizyon ≥ 1, standart kimliği
  ve revizyonu 1–128 bayt (`approved_sheet_templates_problem`; Rust kodlayıcı/okuyucu, `tools/kcad/kcad.py`,
  `scripts/fixtures/kcad_v2_reference.py`). Şema 182 alanı bilinmeyen alan sayar.

## 4. Yayımlanan ofis araç paleti — bu dilimde yok, gerekçe ve gereken model

Bugünkü kişisel palet (ADR'siz, plan/cad-advanced.md'deki Şablonlar paneli) stil kitaplığının **nesne şablonlarının**
favori/geçmiş listesidir ve kullanıcı ayar klasöründeki `sablon-paleti.json`'da durur. Ofis paletinin yayımı için
veri modelinde üç şey yok:

1. **Araç tarifi:** paletin bir öğesi bugün yalnız bir nesne şablonuna işaret eder; ofis paletinin öğesi ise “bu
   komutu bu hazır girdiyle çalıştır” demelidir (`{id, ad, açıklama, kategori, önizleme, command, commandVersion,
   input}`; girdi komutun şemasıyla doğrulanır, eksik alan kullanıcıdan istenir). Komut kaydına bağlı olmayan simge
   yayımlamak planın “yüzlerce şerit ikonu yerine komut kaydına bağlı tarif” kuralına aykırı olurdu.
2. **Paket alanı:** sürüm 2 paketin `resources`'ı `deny_unknown_fields`'tır; `toolPalettes: [{id, ad, öğeler}]` paket
   sürümü ve doğrulamasıyla eklenmelidir (spec + paket okuyucusu + çakışma/kaynak önceliği).
3. **Bağlılık kararı:** Civil 3D kataloğundaki gibi bırakılan araç kataloğa bağlı mı kalır, yoksa kişisel kopya mı
   olur; kişisel favori anahtarı ofis paletinin öğesine (`standart:<kimlik>:<öğe>`) bağlanmalı, ofis revizyonu
   değişince kişisel favori sessizce başka araca kaymamalı.

Bu üçü, araç tarifinin ortak komut girdisi doğrulamasıyla birlikte ayrı bir dilimdir (TODOS.md'ye yazıldı); bu ADR
yalnız modeli sabitler. Kişisel palet ile belge standardı karışmaz kuralı korunur: ofis paleti çizime kurulmaz,
pakette okunur.

## 5. “Bu değer nereden geliyor?”

`kentos_native_application::appearance_origin::explain(doc, slot)` salt okunur bir açıklamadır; masaüstünde
Öznitelikler panelinin **Bu değer nereden geliyor?** bölümü onu gösterir. Renk, çizgi tipi, kalınlık ve (yazıda) yazı
stili için:

- **Etkin değer ve kökeni:** Nesne (kendi değeri), Katman (ByLayer), Varsayılan; yerleştirmede bloğun ByBlock
  parçalarının bu değeri aldığı ayrı satırdır. Çözüm `resolve_look`'un, pencere görünüşü paftanın `map_layers`'ının
  kuralıdır: çizen kodla aynı.
- **Standart:** bağlı standart katmanı (ya da yazı stilini) tanımlıyorsa kaynağı ve revizyonu, katmanla aynı mı farklı
  mı (farklıysa “Standardı uygula düzeltir”, yazı stilinde “Kaynakları kur değiştirir” ve farklı alanlar).
- **Görünüm penceresi:** nesnenin kendi değeri yoksa, katmanı başka çizen her pafta penceresi `Pafta · pencere: değer`
  olarak uyarı satırıdır (katman geçersiz kılması ve katman durumu teması dahil). Nesnenin kendi değeri pencere
  geçersiz kılmasından önce gelir (AutoCAD'deki gibi).
- **Sıfırlama kapsamı:** nesne geçersiz kılması için “bu nesne” ve “bu katmanda kendi değeri olan bütün nesneler”
  (sayısıyla), pencere geçersiz kılması için “bütün pencerelerde” (pencere sayısıyla), yazıda elle ayrılan stil
  alanları; her biri sonraki değeriyle. Sıfırlamanın kendisi var olan komutlardır (`cad.entities.edit`, paftanın
  pencere düzenlemesi); açıklama hiçbir şey yazmaz.

Sınır: blok içindeki tek bir parçanın kökeni (iç içe seçim) ve plot profilinin (renk tablosu) baskıdaki etkisi bu
açıklamada yok; seçim üst düzey nesnedir.

## 6. Masaüstü

Standart kaynaklarını kur penceresinde Tür sütunu Blok ve Pafta şablonu satırlarını, yeni **Kitaplık** sütunu
çözüldüğü kitaplığı (`kentos-ofis`, `sistem`), **Kullanım** sütunu yerleştirme ya da pafta sayısını gösterir.
`Kitaplıkta yok` satırı uyarı rengindedir ve işlem yerine “Kurulamaz; çizimdeki kalır” yazar; şablon satırında Yeni
adla seçeneği yoktur. Özet satırı kitaplıkta bulunmayanları sayar.

## 7. Doğrulama

- `scripts/fixtures/standards_install_library_cases.py` KentOS kodu kullanmadan, kitaplıkları veri dosyalarından okuyup
  24 ortak durum yazar (`fixtures/commands/v1/cad.standards.install.library.json`): varsayılanlar, plan/doğrulama,
  yerinde değiştirme (ad/kimlik korunur, model ve PLAN içindeki yerleştirmeler izler), yeni adla ekleme, kitaplık
  kimliği çizimde varken yeni kimlik, eksik bloğun kitaplık kimliğiyle eklenmesi, atlama, onaylı şablon revizyonu
  değişimi, kitaplıkta olmayan blok/şablonun bulgu kalması, Türkçe büyük-küçük harfle ilk standart önceliği
  (`KAPI`/`kapı`), Undo/Redo ve 11 ret. İlk koşuda 24/24 geçti; ADR 1002'nin 25 durumu `unavailable: 0` ile geçti.
- `crates/native/application/tests/all/office_profiles.rs` iki ofis profili kabulünü komutlarla koşar: ofis A
  (kırmızı A-DUVAR, GAZ 2/−1, A-NOT 2,5, kapının 1. yayını, mimari şablon) ve ofis B (mavi, GAZ 9/−9, A-NOT 3,5, kapının
  2. yayını). Her profilde plan cevabı yazma cevabına eşittir; aynı adlı farklı blok yerinde değiştirilir (kimlik aynı,
  iki yerleştirme izler), aynı adlı farklı çizgi tipi GAZ-B olarak eklenir, nesnenin kendi rengi ve iki pencerenin
  farklı geçersiz kılması korunur, paftalar değişmez, tek Undo kurulumu geri alır; aynı tarif yeni projede ve
  kaydedilip yeniden açılmış projede aynı çizimi üretir. `block_refused` ayrı testtir.
- MCP: `crates/native/mcp/tests/office_standard.rs` (araç yıkıcı işaretli; plan = sonuç; kitaplıkta olmayan yayın
  `unavailable`). KCAD: şema 183 örneği ve üç bozuk dosya bağımsız yazıcı/okuyucuyla.
- Masaüstü görüntüleri (`standart-kitaplik-kur`, `standart-kitaplik-kuruldu`, `deger-kokeni-katman`,
  `deger-kokeni-nesne`) repo dışındadır.

## 8. Sınırlar

Ofis araç paleti yayımı (§4), tablo stili kaynağı, blok içi parçanın ve plot profilinin görünüş kökeni, kitaplıkların
proje/bulut kaynağının masaüstünde seçilmesi ve web karşılığı (ADR 0228) bu ADR'de yok.
