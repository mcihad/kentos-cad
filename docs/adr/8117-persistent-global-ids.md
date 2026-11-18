# ADR 8117: IFC'de türetilen parçaların kalıcı GlobalId'si

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) §7 "Kimlik" kuralı ve sınama tablosunun
  "Yeniden dışa aktarma, yeniden adlandırma, sil ve geri al, kopya, tip değişikliği" satırı; IFC-04 (ilişkiler) ve IFC-10
  (revizyon farkı).
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (yazıcı ve GlobalId tohumu), [8103](8103-ifc-revisions-and-bcf.md)
  (GlobalId'yle karşılaştırma), [8116](8116-ek9-from-the-delivered-model.md) K225 (teslim edilen modelle karşılaştırmanın
  zaman damgası geçici çözümü ve sınırı).
- **Web, `.kcad`, komut, kural paketi:** değişmedi. Dosyaların GlobalId'leri değişir (§3 K232).

## §1. Bulgu

ADR 8116'nın sınaması aynı kesit binasını başka bir zaman damgasıyla yazınca 23 eklenen, 23 silinen ve 59 değişen nesne
gördü. Neden: yazıcı KentOS kimliği olmayan her parçanın GlobalId'sini dosyanın adı ve zaman damgasından ve yazım
sırasından üretiyordu (`Step::gid`: `"{dosya}|{zaman}"` tohumu ve sayaç). Bu parçalar şunlardı: tek dosyalık çizimde proje,
saha ve bina; katlar; açıklıklar; emsal zonları; bağımsız bölüm grupları; özellik ve miktar setleri; bütün ilişkiler.
Sonuçları:

- Teslim edilen model çizimle karşılaştırılamıyordu (ADR 8116 zaman damgasını dosyadan okuyarak geçici olarak çözdü).
  Sıra numarası olduğu için açıklık, kat ya da zon ekleyen bir değişiklik o geçici çözümü de bozuyordu.
- `cad.ifc.compare` (ADR 8103) farklı günlerde yazılmış iki teslimde bu parçaları eklenmiş ve silinmiş sayıyordu. Kata bağlı
  her eleman da "taşıyıcısı değişti" diye listeleniyordu. ADR 8103'ün ilk kabulü bundan etkilenmedi; çünkü ikinci revizyon
  aynı dosyanın IfcOpenShell'le düzenlenmiş hâliydi ve GlobalId'ler korunuyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| buildingSMART, IFC 4.3 "Software Identity" kavramı ([IFC4.3.x-development](https://github.com/buildingSMART/IFC4.3.x-development/blob/master/docs/templates/Object%20Attributes/Software%20Identity/README.md)) | IFC-GUID genellikle kendiliğinden üretilir ve kalıcı olmalıdır; STEP örnek numaraları yalnız o dosyanındır, revizyonlar ve uygulamalar arasında güvencesi yoktur | GlobalId dosya adından, zamandan ve yazım sırasından türemez |
| buildingSMART, [IfcRoot](https://github.com/buildingSMART/IFC4.3.x-development/blob/master/docs/schemas/core/IfcKernel/Entities/IfcRoot.md) | GlobalId bütün yazılım dünyasında tekil bir kimliktir | Bir dosyada hiçbir GlobalId iki kez geçmez (K231) |
| Autodesk Developer Blog, [Unique Id and IFC GUID Parameter](https://blog.autodesk.io/unique-id-and-ifc-guid/) | Elemanın IFC GUID'i kendi kalıcı kimliğinden türer; sonraki dışa aktarım aynısını kullanır | KentOS kimliği olan nesne o kimliği taşır (bugünkü gibi); olmayan parça sahibinin kimliğinden türer |
| Archicad, [Detect IFC Model Changes](https://help.graphisoft.com/AC/23/INT/_AC23_Help/115_IFC/115_IFC-9.htm) | İki sürüm GlobalId'yle karşılaştırılır; aynı GlobalId yoksa her öğe yeni ya da silinmiş sayılır | Karşılaştırma kalıcı GlobalId ister |
| [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562) §5.8 | Sürüm 8 UUID, uygulamaya özgü (custom) biçimdir | Türetilen kimlikler sürüm 8; KentOS'un kendi kimlikleri sürüm 7 (eski dosyanınkiler 5) |
| plan §7 "Kimlik" | Türetilen kök nesnede kimlik sahibinin kimliğinden ve rolünden türer; dosya yolu ve sıra numarası kullanılmaz | Uygulandı (K229) |

## §3. Karar

- **K228 — KentOS kimliği olan nesne** kendi kalıcı kimliğini GlobalId olarak taşır (değişmedi).
- **K229 — Türetilen parçalar:** GlobalId, rolünden ve sahibinin anahtarından türer (`kentos_ifc::step::derived_id`).
  Dosya adı, zaman damgası ve yazım sırası kullanılmaz.
  - Kat: katın kimliği. Açıklık: onu açan kapı ya da pencere. Emsal zonu: kat ve sayılış biçimi (ikincisi `+` alır).
    Bağımsız bölüm grubu: bölümün kimliği. Tek dosyalık çizimde proje, saha ve bina: projenin kodu (yoksa işin adı). Bir
    binanın kendi dosyası (ADR 8072) eski formülünü korur.
  - Özellik ve miktar setleri: sahibinin GlobalId'si ve setin adı.
  - İlişkiler: bağladıkları nesnelerin GlobalId'leri ve rol. Parçalama üst nesneden ya da parçadan. Kata ya da binaya bağlama
    taşıyıcıdan. Boşluk ve doldurma açıklıktan. Zon başvurusu zondan. Grup üyeliği gruptan. Tip ilişkisi ve tipin malzemesi
    tipten. Malzeme ve katman seti bağlantısı bina ve adından ya da katmanlarından. Bildirim proje ve binadan. Aktörler proje,
    rol ve kişiden.
- **K230 — Sürüm 8:** türetilen 128 bit sürüm 8 UUID biçimindedir (`is_derived`). EK-9/8'in beklenen nesne listesi bunları
  eskisi gibi saymaz: türetilen parça çizimin bir nesnesi değildir.
- **K231 — Tekil:** aynı rol ve sahiplerle ikinci kez istenen anahtar `+` alır. Bir dosyada hiçbir GlobalId iki kez geçmez.
- **K232 — Eski dosyalar:** bu karardan önce yazılmış bir dosyada (katlarının GlobalId'si sürüm 8 değil) türetilen
  parçaların kimliği farklıdır. Teslim edilen model böyleyse Denetim onu çizimle nesne nesne karşılaştırmaz ve bunu söyler
  ("Model, KentOS kalıcı kimlikleri yazmadan önce yazılmış…"). ADR 8116 K225'in zaman damgası kaldırıldı.
- **K233 — Fixture'lar:** yalnız yazıcının çıktısıyla bayt bayt karşılaştırılan iki dosya yeniden yazıldı
  (`kesit-binasi-tipli.ifc`, `kesit-binasi-eleman-tipli.ifc`). Fark betikle denetlendi: yalnız GlobalId (358 ve 364 satır).
  Yeniden yazmada dokuz fixture'ın yazıcının bugünkü çıktısı olmadığı ortaya çıktı (`kesit-binasi`, `-tam`, `-kunye`,
  `-kompartiman`, `blok-a`, `cati-alti-mahal`, `egimli-doseme`, `katmanli-cati`, `yerel-sistem`). Duvar tipleri (ADR 8105)
  ve sonraki zonlardan önce yazılmışlar. Olgu, IDS sonucu, form ve okuma başvuruları, inceleme durumları, revizyon çifti ve
  BCF durumları bu eski dosyalardan hesaplanmış. Bu dilimin farkı olmadığı için olduğu gibi bırakıldılar; tazelenmeleri
  ayrı adımdır (§6). `blok-b`'nin duvarları sınamada her koşuda yeni kimlik alıyor, onun da tazelenmesi gürültü üretir.

## §4. Uygulama

- `crates/shared/ifc/src/step.rs`: `derived_id`, `is_derived`, `Step::derived` (sahiplerin GlobalId'si eklenen satırdan
  okunur); `crates/shared/ifc/src/building.rs`: bütün setler ve ilişkiler; `crates/shared/ifc/src/arch.rs`: katlar, açıklıklar,
  emsal zonları, tek dosyalık proje, saha ve bina; `crates/shared/ifc/src/expected.rs`: türetilenler beklenmez.
- `crates/native/application/src/arch/arch_ifc.rs`: bağımsız bölüm grupları. `arch/delivered_model.rs`: `Counted`
  (fark, model yok, eski dosya); zaman damgası yok. Masaüstü `arch_audit/standards.rs`: eski dosyanın sözü.
- `crates/native/application/src/lib.rs`: `delivery_compare` `mod-arch` arkasında (modül sınaması ADR 8099'dan beri düşüyordu).

## §5. Sınamalar

- **Kalıcılık** (`arch_ifc::a_drawing_written_another_day_keeps_every_global_id`): giydirme cephe ve kitaplıktan asansör
  eklenmiş kesit binası iki ayrı günde yazılınca bütün GlobalId'ler aynı ve tekil. Uzakta bir duvara kapı eklenince önceki
  bütün GlobalId'ler yerinde.
- **Bağımsız okuyucu:** IfcOpenShell 0.9.0'ın doğrulayıcısı (şema ve EXPRESS kuralları) yeniden yazılan iki dosyada 0 bulgu
  verdi; 430 ve 442 kök nesne, iki kez geçen GlobalId yok. `ifc_types_check.py` (her stil bir IfcWallType, her eleman tipi
  sınıfının tipi) uyuyor.
- Uygulamanın bütün sınamaları, kentos-ifc'nin sınamaları, Denetim'in teslim modeli sınamaları.

## §6. Sıradaki adımlar

- Dokuz eskimiş fixture'ın ve bağımlılarının tazelenmesi; her farkın açıklanması ve bağımsız başvuruların IfcOpenShell ve
  IfcTester'la yeniden üretilmesi.
- Yol hizalaması dosyasının (`alignment.rs`) kimlikleri hâlâ "dosya | zaman" ve sırayla üretiliyor.
- Plan §7'nin kalan sınamaları: kopya yeni kimlik alır; bölme, birleştirme, silip geri alma.
