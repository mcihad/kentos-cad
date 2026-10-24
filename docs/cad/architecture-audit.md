# Mimari CAD: ev planı iş akışı ve referans incelemesi

Araştırma tarihi: 23 Ekim 2026. Kullanıcı hedefi: insanların bir evin planını
kolayca çizebildiği, kurumsal üretimde kullanılabilen, uluslararası çizim
kurallarını uygulayan 2B CAD. Bu belge test sonucu veya uygunluk sertifikası değildir.

## Referanslarda ne incelendi?

AutoCAD'in genel çizim araçlarıyla Architecture toolset'in yapı nesneleri ayrı
kapsamdır. Duvar/kapı/pencere/mahal hedefinde esas karşılaştırma Architecture
olmalıdır. Ticari uygulamalar bu makinede çalıştırılmadı; aşağıdakiler üreticinin
birincil dokümanlarından incelendi. FreeCAD tarafında kaynak kod da okundu.

| Referans | Doğrulanan yaklaşım | KentOS'ta karşılığı / araştırılacak fark |
| --- | --- | --- |
| [AutoCAD Architecture: duvar birleşimi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-33F8B759-ECFE-4359-B186-C95C98B30D36.htm) | Referans çizgisi ve birleşim grubu/yarıçapı açık; arıza teşhis görünümü ve canlı önizleme var. | Birleşim çözücüsünün varlığı yetmez: L/T/X köşeyi kullanıcı anlayabilmeli, başarısız köşe sahnede gösterilmeli. Sessizce geometri değiştiren büyük snap toleransı kullanılmamalı. |
| [AutoCAD Architecture: ilişkili AEC ölçüsü](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-80E8DF2E-1722-4957-BABF-910A0072EF81.htm) | Duvar, açıklık ve kesişimlerden stilin seçtiği noktalarla ölçü zincirleri üretilir; açıklık değişimi ölçüyü günceller. İç içe xref ilişkileri de ele alınır. | Duvara bağlanan tek ölçü ile otomatik dış/iç ölçü zinciri aynı özellik sayılmamalı. Açıklık eni, duvar kalınlığı ve toplam cephe ayrı zincirlerde sınanmalı. |
| [AutoCAD Architecture: doğrudan düzenleme](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-CDA9AF6E-AE7F-47A1-A19A-82EAA0196138.htm) ve [eylem tutamaçları](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-B1BC9BF6-7F3E-40C6-A448-D81A85DD2F43.htm) | Sayısal tutamaç girişi ve yön/kanat gibi tek hareketlik eylemler; seçime ait bağlamsal düzenleme. | Kapıyı çevirme ve açıklık enini değiştirme görünür olmalı; yalnız gizli harf seçenekleriyle erişilen iş akışı yeterli değil. Aynı eylem mevcut ürün komutunu çağırmalı. |
| [AutoCAD Architecture: ilişkili mahal](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-3845A9D8-FC9E-4234-892B-CC5BB875770E.htm) | Sınır değişince mahal güncellenir; 2B, ekstrüzyon ve serbest 3B ayrıdır. Xref'te ilişkinin yönü tanımlıdır. | KentOS'un bugünkü 2B mahal hesabı 3B hacim doğruluğu diye sunulmamalı. Duvar taşımada alan, isim etiketi ve çizelge birlikte izlenmeli. |
| [Archicad: otomatik dış ölçülendirme](https://helpcenter.graphisoft.com/user-guide/77042/) | Toplam boy, dış geometri, yapı ve açıklık ölçüleri seçilebilir; açıklık merkez/uç ve duvar dış/çekirdek yüzü ayrılır. | Kullanıcı bir cepheyi seçip ölçü üretmeli; ölçü geometrisinin hangi fiziksel yüzü izlediği anlaşılmalı. Bu sayfa eski sürüm kılavuzudur; güncel sürümde her ayrıntı var sayılmaz. |
| [Archicad: mahal oluşturma](https://help.graphisoft.com/AC/27/INT/_AC27_Help/040_ElementsVB/040_ElementsVB-137.htm) | Kapalı alana tıklama; iç yüz veya referans çizgisiyle farklı alan hesabı. | Net/eksen/brüt seçeneğinin sonucu çizimde önizlenmeli. Kapı boşluğu yüzünden mahalın dışarı taşması özel kabul senaryosu olmalı. |
| [BricsCAD BIM: QuickDraw](https://help.bricsys.com/en-us/document/command-reference/b/bimquickdraw-command?version=V27) | Dikdörtgen/L mekânı etkileşimli çizme; duvar hizası, sayısal boylar, komşuya hizalanma, kat işlemleri aynı akışta. | Önce kolay 2B oda/duvar çizimi; bu yaklaşımın bütün 3B işlevlerini bugünkü 2B çekirdeğe taşımak gerekmez. Oda net ölçüsü ile duvar referans boyu açık seçilmeli. |
| [FreeCAD ArchWall](https://github.com/FreeCAD/FreeCAD/blob/7f643c369dee071a9fa5058c831572ec48bbdd50/src/Mod/BIM/ArchWall.py), [ArchWindow](https://github.com/FreeCAD/FreeCAD/blob/7f643c369dee071a9fa5058c831572ec48bbdd50/src/Mod/BIM/ArchWindow.py), [ArchSpace](https://github.com/FreeCAD/FreeCAD/blob/7f643c369dee071a9fa5058c831572ec48bbdd50/src/Mod/BIM/ArchSpace.py) | Kaynak çizgi/kalınlık ve üretilen şekil ayrıdır; açıklığın `Hosts`, `MoveWithHost`, delik hacmi ve ayrı yeniden hesap yolu vardır. | Kalıcı duvar/açıklık ilişkisi korunmalı; çizilmiş sembol delik/host ilişkisinin yerine geçmez. Kod okumasıdır; FreeCAD yerelde çalıştırılmadı. |

CADCraft ve OpenCADStudio'nun birim, blok ve anotasyon kaynak incelemesi
[ADR 0624](../adr/0624-cad-units-direct-open-and-property-input.md)'tedir.
Bu uygulamalardaki genel CAD altyapısı, incelenmeden bir mimari BIM araç setine
eşdeğer kabul edilmez. Yeni kaynak taramasında referanslar OpenCADStudio
`3edb433eb44af41eb8251dd53fd3b27a66ca98a4`, CADCraft
`dd1f0531ac553b2c614dd1bc7d80406c3d29d767` olarak kaydedildi.

## 2B CAD ve yapı modeli karşılaştırması

- [QCAD'in güncel baskı kılavuzu](https://qcad.org/en/tutorials/printing/printing_and_pdf_export/)
  modelin gerçek boyutta çizilmesini, çizim birimi ile kâğıt biriminin ayrı ayarlanmasını
  ve çıktı oranının önizlemede seçilmesini anlatıyor. KentOS'ta da 10 m duvarın
  fiziksel uzunluğu pafta 1:50 veya 1:100 olduğunda değişmemeli. Sıradan bir model
  yazısını sonradan büyütmek, çıktı oranını değiştirmekten ayrı bir eylem olmalı.
- [LibreCAD anotasyon kılavuzu](https://docs.librecad.org/en/latest/guides/annotate.html)
  hizalı/doğrusal/açısal/radyal ölçüleri ve çizim tercihlerindeki boy ayarlarını
  belgeler. Bu, temel 2B ölçü kapsamı için karşılaştırmadır; duvara bağlı AEC ölçü
  zincirinin kanıtı değildir. Belge kendisini geliştirilmekte olan kılavuz olarak tanımlar.
- [LibreCAD çıktı akışı](https://docs.librecad.org/en/latest/guides/completion.html)
  pafta, ölçek, antet ve baskı önizlemesini birlikte ele alır. KentOS'ta çıktı
  kabulü yalnız ekrandaki bir pafta resmine bakılarak yapılmaz: PDF'deki uzunluk,
  kâğıt boyu ve yazının basılı boyu bağımsız ölçülür.
- [AutoCAD Architecture çizelgeleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-77F8B132-6A45-483B-AB92-AD3BD5D47660.htm)
  nesne üzerinde tutulan özellikleri okur; etiket ve tablo bunların görünümüdür.
  KentOS'ta da kapı numarası/eni birden fazla bağımsız metin kopyasına dönüşmemeli.
  Hesap alanını değiştirmek ile kaynak nesneye yazmak ayrı, açık işlemler olmalı.
- [BricsCAD BIMSECTIONUPDATE V27](https://help.bricsys.com/en-us/document/command-reference/b/bimsectionupdate-command?version=V27)
  kaynak ilişki verisi üretimini bir ayara bağlar; ilişkisiz anotasyonu gösterebilir.
  Buradan çıkarılan KentOS kararı: kopan ölçü/etiket bağı sessizce eski değerde
  bırakılmamalı. Kaynak kimliği ve ilişkinin durumu incelenebilir olmalı.
  Bu komutun etiketleri kendiliğinden üretmediği de belgede açık; “tek komutla tüm
  belgeleme otomatik” gibi kapsamı aşan bir çıkarım yapılmamalı.

## KentOS için teknik karşılıklar ve kabul kanıtı

Bunlar yapılacak/doğrulanacak işlerdir; araştırma maddeleri tamamlanmış özellik
olarak işaretlenmez. Özellikle mevcut çalışan mekanizma incelenmeden yenisi yazılmaz.

| İş akışı | İncelenecek mevcut kaynak | Kabul / eksikse yapılacak iş |
| --- | --- | --- |
| Duvar üretimi | `interaction/wall.rs`, `application/walls_create.rs`, `geometry-core/geom/wall.rs` | Eksen/sol/sağ ve öteleme çizim yönüyle açık; kalınlık gerçek uzunluk; çok katmanlı stil sabit kalınlık veriyorsa kullanıcıya kilit nedeni gösterilir. Düz/yaylı, ters yönlü ve kapalı yol aynı fiziksel sonuca göre ölçülür. |
| Köşe birleşimi | `application/walls_joins.rs`, `geometry-core/store/wall_joins.rs` | L/T/X, dar açı, farklı kalınlık ve katman sınırlarında birleşim; başarısızlık vurgusu; sıfır alan/kendi kesişimi/sıçrama yok. Kaynak duvar referansı türetilmiş köşe çizgisinden ayrı kalır. |
| Kapı/pencere | `interaction/opening_tools.rs`, `application/openings_{create,edit}.rs` | Duvar kenarından sayısal mesafe ve merkezleme; nominal en, boşluk eni ve kanat yönü ayrı anlamlar. Host taşıma, duvar bölme/silme, çakışma, kilitli katman tek işlem ve geri al ile sınanır. |
| Doğrudan düzenleme | `properties/rows/arch.rs`, mevcut tutamaç oturumu | En, kalınlık ve uzaklık için görünür sayısal giriş; menteşe/yön için anlaşılır eylem. Klavyede her basış belgeye komut yazmaz; Enter/odak kaybı bir kez uygular, Escape vazgeçer. Önizleme ile sonuç aynı doğrulamayı kullanır. |
| Mahal | `application/spaces_{create,edit,quantities,finish}.rs` | Net/eksen/brüt sınır önizlemesi, kapalı sınır tanısı, delikler/kolonlar ve açıklık geçişi. Birim gösterimi değişince m² alan değişmez. Duvar değişiminde sınır, alan ve etiket beraber izlenir. |
| Ölçü | `interaction/dimension_chain.rs`, `application/dimension_create.rs`, `dimensions_reassociate.rs` | Mevcut Zincir/Baz aracı korunur. Cepheyi otomatik analiz ederek açıklık/duvar/toplam için çok sıralı AEC ölçüsü üretmek ayrı kabul edilir. Nokta bağı, silinen host, birden çok aday ve ilişkiyi yeniden kurma sınanır. |
| Çizelge | `application/schedules.rs`, `domain::schedules` | Mevcut canlı kaynaklı tablo kullanılır; yeniden tablo motoru yazılmaz. Kapı/mahal/duvar değişimi satıra yansır; gruplama, kat filtresi, toplamlar ve salt hesap hücresine yazma reddi doğrulanır. |
| Başlangıç şablonu | `catalog/v1/templates/mimari.json`, `application/templates_apply.rs` | Şablonun mevcut cm/1:50 seçimi görünürdür; yeni kullanıcı fark etmeden birim değiştirilmez. Boş ve dolu projeye uygulama ayrıdır. Etkin kat, nesneye uygun katman ve yeni yazı/ölçü varsayılanı anlaşılır olur. |
| Dış dosya | `formats/src/dxf`, `native/dwg`, `cad_open` | Kaynak birim, INSERT çarpanı, model/pafta uzayı ve aktif stiller korunur. Düzenlenebilir veri kaybı açık raporlanır; küçük/büyük görünüşe bakarak birim tahmin edilmez. Kaynak dosya açma ve mevcut projeye aktarma varsayılanları ayrılır. |
| Pafta | mevcut sheet/plot komutları | Aynı model iki farklı viewport oranında; A3 ölçüsü, kırpma, antet, kalem, yazı/ok boyu ve font gömme kontrolü. 10 m uzunluk 1:50'de 200 mm, 1:100'de 100 mm çıkmalı; model verisi değişmemeli. |
| Python / AI | mevcut ürün komut kataloğu ve üretilmiş SDK | Arayüz eylemiyle aynı doğrulama, önizleme, atomik sonuç, undo, kimlik ve kayıp raporu. Ekrana koordinat tıklatmak yerine duvar/açıklık/mahal anlamsal komutu kullanılmalı. |

### Boy ve gösterim sözleşmesi

1. Fiziksel geometri, gösterim birimi, içe alma dönüşümü, çıktı oranı ve anotasyon
   ölçek ilişkisi ayrı kavramlardır. Bir kullanıcı tercihi bunlardan birkaçını
   sessizce değiştirmemeli.
2. Duvar 10 m, kapı 900 mm, metin modelde 125 mm olabilir. Aynı model 1:50'de
   metni 2.5 mm basar. İlişkisiz model metni 1:100 seçilince modelde yine 125 mm
   kalır; otomatik kâğıt boyunu izleme yalnız açıkça ölçeğe bağlı nesnelerde uygulanır.
3. Kaynak stil aktifse yeni ölçünün boyu kaynaktan belirlenmeli; genel 2.5 mm
   varsayılanıyla kaynak 140 mm stili birbirine karıştırılmamalı. Kâğıt mm alanının
   sınırını artırmak bu model eksikliğini çözmez.
4. Görüntüleyici kaynak boyu/koordinatı değiştirmez. Büyük koordinatta kamera
   farkı ve metin çerçevesi hassas kalmalı; okunurluk için büyütme CAD'de fiziksel
   model görünümünden ayrı ve açık seçenek olmalı.
5. İnç/ayak testleri metrik çarpanların yuvarlanmış benzeriyle yapılmaz. 1 inç =
   25.4 mm dönüşümü, blok iç içe ölçekleri ve çıktı oranı bağımsız hesaplanır.

### AutoCAD ile birim ve ilişki karşılaştırmasındaki önemli ayrımlar

- [INSUNITS](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm)
  blok/resim/xref yerleştirme birimini bildirir; tek başına binanın doğru fiziksel
  boyda çizildiğinin kanıtı değildir. Sıfır “birimsiz”dir. İnç/ayak/mm/cm/m farklı
  değerlerdir; tüm kaynakları mm saymak uygun değildir.
- [Blok birimi ve ekleme çarpanı](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
  ayrı girdilerdir. Kaynak/hedef birim dönüşümü ile kullanıcının INSERT çarpanı
  üst üste uygulanabilir. KentOS'un SI depolamasında aynı dönüşüm ikinci kez
  uygulanmamalı; iç içe blokta dönüşüm sırası korunmalıdır. Birimsiz kaynak için
  kullanılan varsayım raporda açıklanır. Annotative blok yolu ayrıca sınanır.
- [İlişkili ölçü](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-3559D5D2-AD15-4167-9BD9-EE5FCEB4F8CE.htm)
  ile tek nesne olarak duran fakat geometriye bağlı olmayan ölçü farklıdır.
  Patlatılmış çizgi/yazı üçüncü durumdur. KentOS'un kayıp raporu ve kabul sayıları
  bu üç durumu ayrı göstermeli; görünüşün eşitliği ilişki korunumu sayılmamalı.
- [Architecture anotasyon ölçeği](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-017F80EA-AF4B-4CD2-A374-9EAF103A7E5B.htm)
  model ve kâğıt viewport'una bağlıdır. KentOS'ta mevcut tek `plotScale` alanı
  kullanıcıya “bütün çizimin fiziksel ölçeği” gibi sunulmamalı. Çok viewport'lu
  paftada nesnenin hangi temsili gösterildiği ve ölçek bulunamadığında davranış
  ayrı sözleşmedir; zoom görünürlük hatası anotasyon filtresiyle karıştırılmamalı.

### Ev planı etkileşiminde kaydedilecek ölçümler

- Başlangıçtan ilk dış duvara kadar eylem sayısı ve geçen süre; gizli harf
  seçeneği bilmeden çalışabilme. Deneyimli kullanıcının klavye akışı korunur.
- Bir sayı yazarken tuş→ekran gecikmesi ve ilk önizleme gecikmesi. Boş çizim,
  10 bin nesneli plan ve uzak koordinat ayrı ölçülür; debug ölçümü üretim hızı diye sunulmaz.
- Duvar taşıma sonrası beklenen değişen nesneler kümesi, revision/undo adımı,
  kaynak/çıktı alanları. İlgisiz nesnenin değişmesi başarısızlıktır.
- Her ekran görüntüsünde tema, pencere boyu, yazı boyu, belge, komut ve aşama
  belirtilir. Gözle bakılan sonuç ile otomatik doğrulanan geometri ayrı kayıtlanır.
- Geri al/yinele ve KCAD yeniden açma sonrası kimlikler/ilişkiler/gerçek boylar
  karşılaştırılır; yalnız toplam nesne sayısının eşitliği yeterli değildir.

## KentOS'ta şu anda kodda bulunan altyapı

Aşağıdakiler kaynak kod bulgusudur; bu oturumda hepsinin GUI kabulü tamamlanmadı.

- `interaction/wall.rs`, `path.rs`: duvar stili, hiza, genişlik, öteleme, yaylı yol;
  önizleme/yazma `walls_create` üzerinden. Beş yerleşik duvar stili sunuluyor.
- `interaction/opening_tools.rs`, `application/openings_create.rs`: duvara bağlı
  kapı/pencere/boşluk, tip, en, menteşe, yön, uzaklık, ortalama; çakışma ve duvar
  sınırı hataları. Önizleme ve yazma aynı komutta.
- `interaction/space_tools.rs`, `application/spaces_create.rs`: tohumdan veya
  çizilmiş sınırdan mahal; net, eksen, brüt; numara, ad ve kullanım seçenekleri.
- `interaction/arch_tools.rs`: aks, kolon, merdiven, rampa; proje kat bilgisi
  `settings.arch` içinde. Bunların varlığı kolay kullanılabildiklerini kanıtlamaz.
- `properties/rows/arch.rs`: tipli mimari özellik satırları. Araç öncesi ayar ile
  seçili nesne düzenlemesinin aynı kullanıcı kavramlarıyla sunulduğu sınanmalı.
- `wall_scenes.rs`, `opening_scenes.rs`, `space_scenes.rs`, `arch_scenes.rs`:
  mevcut sahne/etkileşim örnekleri. Kendi başlarına üretim kabulü değiller.
- Sağ tuş açık bitirmesi ve baskı ölçeğinin normal nesneleri büyütmesi
  ayrı regresyon incelemesinde ele alınıyor; araştırma bunların kabulünün yerine geçmez.

## Bağımsız kat planı dosyasıyla somut bulgular

[CADCraft apartment.dxf](https://github.com/storytold/cadcraft/blob/dd1f0531ac553b2c614dd1bc7d80406c3d29d767/examples/apartment.dxf)
yalnız inceleme için indirildi; kaynak değiştirilmedi ve projeye bağımlılık eklenmedi.
`ezdxf` kaynaktaki 58 model nesnesini, `INSUNITS=4` (mm) bilgisini ve Standard
ölçü stilinde `DIMTXT=140`, `DIMSCALE=1` değerlerini doğruladı. KentOS bu dosyayı
135 nesne olarak açıyor; yedi DIMENSION çizgi/yazı görünümüne patlatılıyor ve
raporda düzenlenebilir ölçü kaybı bildiriliyor. Dolayısıyla görünüşü alınmış bir
ölçü, ilişkili mimari ölçü desteği tamamlanmış sayılmamalı.

Aynı açılışta yeni açıklama varsayılanı kaynak aktif stilinden türetilmiyor:
proje `plotScale=1`, açıklama varsayılanı yok. Kaynakta 140 mm ölçü yazısı bulunurken
KentOS'un genel yeni ölçü varsayılanı 2.5 mm. Bu bulgu, kaynak aktif yazı/ölçü
ayarlarının korunması ve normal model boyu ile kâğıt anotasyonunun ayrı ele
alınması gerektiğini gösteriyor. Rastgele medyan yazı boyu veya tahmini ev ölçüsü
ile otomatik düzeltme yapılmamalı. Henüz bu iki konu tamamlandı sayılmadı.

### Dosya ve yeni anotasyon varsayılanları için teknik iş paketi

Aşağıdaki tasarım önerisi henüz uygulanmadı. Bugünkü içe alma görünümünü bozarak
ölçü desteğini varmış gibi göstermemek için geometri, stil ve ilişki ayrı kabul edilir.

1. **Doğrudan açılışta aktif kaynak ayarları:**
   [TEXTSIZE](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-DD548317-95BA-4AEA-B54D-0D917F10D3C1.htm)
   ile sabit yüksekliği olan yazı stilinin önceliği korunmalı. Kaynaktaki aktif
   yazı/ölçü stili, fiziksel model boylarıyla yeni nesne üretimine taşınmalı.
   Mevcut projeye sıradan içe alma, hedef projenin varsayılanlarını kendiliğinden
   değiştirmemeli. `Standard` adının ayrılmış olması kaynak değerlerin atılmasına
   yol açmamalı; ad ve stil kimliği eşlemesi açık olmalı.
2. **Ölçü görünüş boyu ile ölçülen uzunluk ayrımı:**
   [DIMSCALE](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-AEA309F0-B831-4432-8085-FB6CD49CDC78.htm)
   ölçülen uzunluğu değiştirmez. Sıfır değer körlemesine sıfır boy veya doğrudan
   1:1 demek değildir; annotative stil/viewport bağlamı da incelenmeli. Kaynağın
   model yüksekliği, mevcut kâğıt yüksekliği alanına tahminle doldurulmamalı.
3. **Yabancı temel ölçülerin düzenlenebilir geometriye alınması:**
   `formats/src/dxf/dimension.rs::foreign` şu anda özel koordinat/yay uzunluğu/
   kırıklı yarıçap yollarını ele alıyor; diğerlerinde blok görünümüne dönüyor.
   [Doğrusal/döndürülmüş DXF ölçüsü](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DXF/files/GUID-F0004556-493C-48D5-8619-61D6ADF05C04.htm)
   için 13/14 WCS tanım noktaları, 50 dönüşü ve 52 eğik uzatma açısı gibi alanlar
   ayırt edilmeli. [Ortak ölçü gruplarındaki](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DXF/files/GUID-EDD54EAC-A339-4EBA-AEA6-EC8066505E2B.htm)
   11 OCS metin noktası WCS noktası gibi okunmamalı.
   Desteklenmeyen eğiklik/düzlem/özel ok/görünüşte mevcut açık kayıp yolu korunmalı.
4. **İlişkilerin ayrıca alınması:**
   [DXF DIMASSOC nesnesi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DXF/files/GUID-C0B96256-A911-4B4D-85E6-EB4AF2C91E27.htm)
   ölçü ve geometrinin handle/alt nesne/snap ilişkisini taşır. Salt DIMENSION
   noktalarını okuyup `refs` boş bırakmak düzenlenebilir geometri sağlayabilir,
   fakat ilişkili ölçü desteği sayılmaz. Handle→kalıcı kimlik çözümü, kesişim,
   xref ve model–pafta ilişkileri için ayrı destek/kayıp raporu gerekir.
5. **Kalıcılık ve yüzeyler:** aktif varsayılanların tipli sözleşmesi, KCAD
   kaydet/aç, undo sınırı, dışa verme ve Python/MCP okuma/yazması birlikte
   tanımlanmalı. Yalnız pencere belleğine yazılmış bir varsayılan yeterli değildir.
6. **Bağımsız kabul:** kaynak mm/cm/m/inç/ayak, sıfır/pozitif DIMSCALE,
   sabit/değişken metin stili, nesneye özel override, döndürülmüş görünüm,
   büyük koordinat ve dış yazılımda değiştirilmiş dosya ayrı örneklerdir.
   Kaynak blok görünümüyle karşılaştırma, ölçü değeri ve düzenleme sonrası bağ
   ayrı ayrı ölçülmeli; stil kaybını örten gevşek tolerans kullanılmamalı.

## İlk sayısal ev planı deneyi — GUI kabulü henüz yok

Mevcut Python ürün komutlarıyla ayrıca küçük bir bağımsız deney yapıldı. Dış
kontur 10 × 8 m, dış duvar 250 mm, ortadaki bölme 150 mm; 900/800 mm kapılar ve
1200 mm pencere eklendi. Saat yönünün tersindeki dış konturda `justify=right`
ile sağ yüz referans alınarak duvar gövdesi içte bırakıldı. Bölme iki iç yüze
ulaşır; iki ayrı seed ile mahal üretildi. Dış duvar ve bölme yedi ayrı sıradan
çizgi olarak taklit edilmedi; iki duvar, üç açıklık, iki mahal kaynağı kullanıldı.

- Bağımsız beklenen oda alanı: `((10 − 2×0.25 − 0.15) / 2) × (8 − 2×0.25)`
  = **35.0625 m²**. Halkaların alanı, büyük koordinatta önce yerel başlangıca
  taşınarak Python'da ayrıca hesaplandı ve ürünün metraj sonucuyla karşılaştırıldı.
- mm, cm, m, inç ve ayak gösterimlerinin her biri `(0,0)` ve `(500000,4400000)`
  konumunda denendi: **10 varyant geçti**. Alan ve açıklıkların fiziksel eni
  değişmedi; KCAD baytlarından yeniden açmada aynı kaldı. Uzak koordinattaki
  fark yaklaşık `9×10⁻¹¹ m²`; bu dar deney bütün geometrinin hassasiyet kanıtı değildir.
- Dış duvar 250 → 300 mm düzenlendiğinde oda alanları bağımsız beklenen
  **34.225 m²** değerine geldi, açıklık enleri kaldı; geri al 35.0625 m² alanı
  geri getirdi. Bu adım da aynı 10 varyantta geçti.
- Bu yalnız komut/kalıcılık deneyi. Tıklamayla kolay çizim, köşe görünüşü, etiket,
  ölçü, çizelge ve baskı henüz bu deneyle doğrulanmış değildir.
- Hiza sözcüğünün hangi yüzü referans aldığı önemlidir: “Sol” gövdenin sola
  taşması diye okunursa yanlış sonuç beklenebilir. Arayüz, referans yüzünü ve
  gövdenin kalacağı tarafı çizim yönü okuyla göstermeli; mevcut geometri kuralı
  sessizce ters çevrilmemeli.

## Kolay ev planı için kabul senaryosu

Örnek yapı, mimari öneri değil yazılım test verisidir: 10 × 8 m dış kontur,
250 mm dış duvar, 150 mm bölme, iki oda ve bir servis hacmi. Kapılar 900/800 mm,
pencereler 1200 mm; bir açıklık köşeye yakın, biri ortalanmış. Aynı fiziksel
model mm/cm/m gösterimlerinde ve yerel/büyük koordinatlarda denenir.

1. Boş CAD aç; birimi ve gerçek boyu tek anlamlı seç. Yeni başlayan kişi ayrıca
   geometriyi büyüten bir proje ölçeği öğrenmek zorunda kalmasın.
2. Dış konturu tam boylarla çiz. İç/dış/eksen referansı ve duvar kalınlığı
   önizlemede açık olsun. Dört ayrı duvarın ve tek kapalı yolun köşeleri sınansın.
3. Bölmeleri tam uzaklıkla yerleştir. T birleşimi otomatik temizlensin; çakışan
   duvar veya gereksiz parça açık hata/denetim sonucu versin.
4. Kapı/pencereyi duvara taşı; yerleşmeden önce en, köşeye uzaklık, menteşe ve
   açılma yönü görülsün. Bir klik gerçekten bir yerleşim yapsın; sağ tuş yerleştirmesin.
5. Kapı enini değiştir ve aynı duvarda kaydır. Host değişimi açık olsun. Duvarı
   taşı, kalınlığını değiştir, böl; açıklık ilişkisi tek changeset'te korunsun.
6. Odalara tıkla; net alan, numara ve ad üret. Yanlış yerde kapalı alan bulunamazsa
   kullanıcıya hangi sınırın açık olduğu gösterilsin. Sessiz boş sonuç olmasın.
7. Dış/iç ölçü zincirlerini yerleştir; kapı/pencere eni ve duvar yer değiştirdiğinde
   aynı referanslar güncellensin. Nominal ölçü ile duvar boşluğu ayrı anlamlar taşısın.
8. Mahal ve kapı/pencere çizelgesini üret. Kaynak nesne düzenlemesi satıra yansısın;
   hesaplanan alanın üzerine yanlışlıkla yazılamasın.
9. A3 paftayı 1:50 ve 1:100 üret; aynı çizgiyi ölçerek fiziksel çıktı oranını doğrula.
   Yazılar/ölçüler zoomda kaybolmasın; yerleşim oranı model boyunu değiştirmesin.
10. Her adımda geri al/yinele, KCAD kaydet/aç, DXF/DWG alışverişi ve aynı anlamsal
    Python/MCP komutuyla sonuç karşılaştırması yap. Kayıp desteklenmiyorsa açık rapor ver.

Her aşama açık/koyu temada, 1440×900 ve 1100×650'de, normal/büyük yazıda denenir.
Gerçek pencere etkileşimi ve GPU resmi alınır; otomatik test sonucu ile gözle
kontrol edilen görüntü ayrı ayrı kaydedilir. Hız, ilk tıklama/ilk önizleme ve
klavye gecikmesi ölçülerek değerlendirilir; yalnız FPS sayısı kabul değildir.

## Uluslararası standartların kapsamı

Aşağıdakiler ISO'nun kamuya açık kapsam/sürüm bilgilerinden doğrulandı. Tam normatif
maddelerden bir uygulama matrisi ve madde bazında test olmadan genel “ISO uyumlu”
iddiası yapılmaz. Standart sürümü, kurum profili ve uygulanan maddeler açık tutulur.

| Kaynak | Yazılımda ele alınacak kapsam |
| --- | --- |
| [ISO 7519:2025](https://www.iso.org/standard/89718.html) | Bina/mimari genel düzen ve montaj çizimlerinin sunumu; plan takımı, basitleştirme, kapı/pencere/merdiven gösterimi. 2024 sürümü geri çekilmiş, 2025 sürümü yayımlanmıştır. |
| [ISO 128-2:2022](https://www.iso.org/standard/83355.html) | Çizgi türlerinin anlamı ve sunumu. Modelin fiziksel genişliği ile ekrandaki kalem kalınlığı birbirine karışmamalı. |
| [ISO 129-1:2018](https://www.iso.org/standard/64007.html) | Ölçü sunumunun genel ilkeleri; 2020 değişikliği ve devam eden revizyon bilgisi profil kaydında izlenmeli. İnşaat özel ayrıntısı için ilgili normatif referanslar ayrıca değerlendirilmeli. |
| [ISO 6284:2023](https://www.iso.org/standard/82512.html) | İnşaat belgelerinde sınır sapmalarının gösterimi. 1996 sürümü yerine güncel sürüm izlenmeli; tolerans yazımı hesap toleransından ayrı tutulmalı. |
| [ISO 3098-1:2015](https://www.iso.org/standard/65679.html) | Teknik çizim yazılarının genel gereksinimleri. Bir ekran fontunun güzel görünmesi tek başına teknik yazı uygunluğu değildir; çıktı fontu/harf ölçüsü ve okunaklılık sınanmalı. |
| [ISO 4157-1:1998](https://www.iso.org/standard/26189.html) | Bina, mahal ve eleman adlandırma sistemi; numara ile nesnenin kalıcı kimliği ayrılmalı. |
| [ISO 5455:1979](https://www.iso.org/standard/11500.html) | Teknik çizim ölçeklerinin adlandırılması; model boyu ve çıktı oranı ayrımı. 2025'te teyit edilmiş sürüm. |
| [ISO 5457:1999](https://www.iso.org/standard/29017.html) | Çizim sayfası boyutu/yerleşimi; değişiklikleriyle sürüm takibi. 2026'da teyit edilmiş. |
| [ISO 7200:2004](https://www.iso.org/standard/35446.html) | Antet ve belge başlığının veri alanları; kimlik, revizyon ve belge yönetimi alanlarının tipli modeli. |
| [ISO 13567-2:2017](https://www.iso.org/standard/70182.html) | İnşaat CAD katmanlarının organizasyonu ve adlandırılması. Kurumun katman profiliyle eşleme gerekir; bütün projelerin katmanları zorla yeniden adlandırılmaz. |
| [IFC / ISO 16739-1:2024](https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/) | İleride nesne/ilişki alışverişi. Resmî IFC 4.3 ADD2 (4.3.2.0), geliştirme sürümlerinden ayrılır. 2B DXF vermek IFC desteği sayılmaz; desteklenen sürüm/görünüm ve kayıplar belirtilir. |

Alan hesabında ayrıca sürüm kontrolü önemlidir: [ISO 9836:2017](https://www.iso.org/standard/73149.html)
ISO sayfasında geri çekilmiş olarak listeleniyor. Güncel zorunlu standartmış gibi
sunulmaz. KentOS'un net/eksen/brüt hesabı fiziksel sınırı açıkça tanımlar; seçilen
kurumun alan hesabına eşitliği ayrıca doğrulanır.

Çizim sunum standardı ile bir ülkenin imar, yangın, erişilebilirlik ve ruhsat
kuralları ayrı profillerdir. Kullanıcıya otomatik mevzuat uygunluğu sonucu
verilecekse ülke/kurum/kural sürümü ayrıca tanımlanır; bu belge böyle bir sonuç vermez.

## Öncelik sırası

1. Geometriyi değiştiren ölçek regresyonu, kaybolan ölçü ve zoom/hassasiyet.
2. Yukarıdaki ev planının mevcut araçlarla baştan sona gerçek GUI kabulü.
3. Kabulde kanıtlanan boşluklara göre duvar/açıklık/mahal ayarlarını görünür kılma,
   referans ve sayısal doğrudan düzenleme; yeni görünüm aynı komutları kullanır.
4. Ölçü zinciri, çizelge ve standart pafta çıktısı; sonra kat/merdiven ayrıntıları.

Eski `CAD-ARCH` işaretleri burada yeniden üretim kabulü sayılmadı. Tamamlandı
kararı çalışan kod, bağımsız hesap, gerçek arayüz ve fiziksel çıktı kanıtıyla verilir.
