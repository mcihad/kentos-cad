# 0679 — Açık plan için kalıcı mahal ayırıcı

25 Ekim 2026. **İşlev dilimi doğrulandı; genel performans ve tam mimari matris açıktır.** ARCH-SPACE-01.

## Ürün davranışı

Aynı salon içindeki yemek alanı gibi duvarsız bölümler, ayrı bir **Mahal
ayırıcı** aracıyla sınırlandırılır. Ayırıcı fiziksel duvar, kapı host'u veya
duvar metrajı değildir. Çizginin adına, rengine ya da katmanına bakarak rol
tahmin edilmez. Mevcut çizgiler ve kaydedilmiş dosyalar dönüştürülmez.

- [Revit Room Separation Lines](https://help.autodesk.com/cloudhelp/2027/ENU/Revit-ArchDesign/files/GUID-6B8BDC71-D0A4-4ABA-8561-34F7AD6BAB55.htm)
  ayrı oda sınırlayıcı aracı tanımlar; duvarsız açık plan bölümleri bununla ayrılır.
- [AutoCAD Architecture ilişkili mahaller](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-F51C80FE-3B37-43E8-8A20-2850D07AFDD9.htm)
  nesnede açık sınır özelliği ve kaynak değişiminde güncellenme kullanır;
  sürümler arası dolaşımda ilişkilerin ayrıca kontrol edilmesini belirtir.
- [BricsCAD BIM spaces](https://help.bricsys.com/en-us/document/bricscad-bim/building-data/creating-and-editing-bim-spaces?version=V26)
  2B çizgisel nesneler için açık sınırlayıcı sınıflandırma sunar.

## Teknik yön ve kapılar

1. Analitik `BoundaryIndex` ayırıcı kenarlarını yüz ağına ekler; fiziksel
   malzemenin winding hesabına katmaz. Yapay epsilon genişlikli duvar yoktur.
   Net/eksen/brüt yöntemi fiziksel duvarları öteler, ayırıcı kalınlığı sıfırdır.
   Ayırıcısız eski kurucular aynı yolu boş listeyle çağırır.
2. Model adayı mevcut polyline üzerinde isteğe bağlı tipli
   `spaceSeparator: ArchInfo` rolüdür. Kat/evre/disiplin açık veridir. İlk
   kapsam tek parçalı, düzlemsel, genişliksiz analitik açık/kapalı path olur;
   uygunsuz polygon/genişlik/çok parça/ölçü verisi sessiz kabul edilmez.
3. Kaynak UUID seçimi, kat/kapsam, eski+yeni etki alanı ve follower aynı
   geometriyi kullanır. Kaynak değişince mahal → etiket → çizelge tek undo'dur.
   Türetilmiş rol dizini dosyaya yazılmaz, kaynak değişince güncellenir.
4. Oluşturma/düzenleme anlamsal komutları, plan/execute, katalog, bağımsız en
   az 20 ortak durum, tipli Python ve gerçek MCP aynı davranışı sunmalıdır.
   Görünür araç önizlemesi ikinci hesap üretmez.
5. KCAD yeni alan için opt-in şema; eski şema/kanonik baytlar sabit. DXF
   semantik whole ve import UUID eşlemesi rolü ve seçilmiş kaynakları korur.
   Eksik kaynak son sınır/uyarıyla kalır. Web uygulama işi ADR0228 uyarınca ertelenir.
6. Gerçek GUI: çizim/kat/ayar, kaynak seçimi, önizleme/ret/Escape/tek onay,
   undo/redo, kayıt/aç, kolon, etiket/çizelge ve zoom/pan; bağımsız kaydedilmiş
   dosya karşılaştırması. Pencere/yazı/tema kabulü gerçek boyutuyla raporlanır.

## Bağımsız geometri kabulü

10×8 m dış duvar ekseni, 0,2 m kalınlık: net **76,44 m²**. x=5 ayırıcı
**38,22 + 38,22 = 76,44 m²** verir. Aynı yerde fiziksel 0,2 m bölme
**37,44 + 37,44 = 74,88 m²** verir; iki davranış karıştırılmaz.

Düz/yaylı/kapalı ayırıcı, T ve çapraz birleşim, ters/çift kenar, sarkan ve
dışa uzayan uç, kapı açıklığı, kolon düşümü, uzak koordinat, beş çizim birimi,
aynı kotta farklı kat ve açık kaynak alt kümesi ayrı ayrı doğrulanır.

Bu bölümün bağımsız başvuruları, aşağıdaki kabul kaydıyla birlikte değerlendirilir;
ana mimari plan tamamlanmış değildir.

## Uygulanan model ve güvenli dönüşümler

- `cad.spaces.separators` v1: `operation=create|set|clear`, salt okunur plan,
  beklenen revision, tek kaydedilmiş komut ve undo. `clear` çizgiyi silmez;
  yalnız açık ayırıcı rolünü kaldırır. Sıradan çizginin adı/katmanı/çizgi tipi
  sınır rolü sayılmaz. Araç mevcut path çizgi/yay dilbilgisini kullanır;
  global çoklu çizgi genişliği ayırıcıya fiziksel kalınlık vermez.
- Kaynak seçimi duvar/kolon yanında ayırıcı UUID'sini de kabul eder. Rolün
  kaldırılması veya kaynağın silinmesi açık kaynak listeli mahalı bütün
  çizime genişletmez; son geçerli sınır `sourceUnavailable` ile korunur.
- Genel edit/taşı/kopya, desteklenen çizgi/yay geometrisinde rolü korur.
  Rolü taşıyamayan ellipse, genişlik, kot/M veya çöken kenar atomik reddir.
  Tam kopyada kaynak kimlikleri yeni gruba eşlenir.
- Blok içindeki parçalar ilk kapsamda mahal kaynağı değildir. Tanımla,
  yeniden tanımla, içerik güncelle ve blok olarak yapıştır, kök ayırıcıyı
  bloğa çevirmeyi açıkça reddeder; normal yapıştır rolü korur. Dışarıdan
  yüklenmiş bir blok/nested blok bu rolü içeriyorsa Patlat sessiz rol kaybı
  yerine `not_explodable` verir. Böyle blok verileri KCAD'de okunup
  kaydedilebilir; bu, blok içinden mahal hesabı desteği anlamına gelmez.
- KCAD opt-in şema **111** (birleştirmede 106’dan kaydı), path sütun bit 7: tipli ArchInfo rolü. Önceki
  şemaların kanonik dosyaları değişmez; sıradan polygon/polyline rol almaz.
  DXF whole metadata açıkken rol ve açık kaynak kimlikleri korunur. Eksik
  veya yinelenen dış kimlik, başka bir kaynağa keyfî bağlanmaz. Metadata
  kapalı düz çizim aktarımında ayırıcı rolü kaybı raporda açıkça belirtilir.
  Gerçek DWG roundtrip bu dilimin kabulü değildir.

## Performans bulgusu — açık takip

200 dış duvar/kolon/ayırıcı, 400 mahal, 400 etiket, 400 satırlı çizelge ve
20.000 ilgisiz çizgiyle public Python → native denemesi yapıldı. Uzak
koordinatta bağımsız 34,22/38,22 m² sonuçları, tek ayırıcı hareketiyle
42,02/30,42 m² ve tek undo'da tam KCAD eşitliği kontrol edildi. İlk debug
ölçümü, eşzamanlı derleme altında: plan ~69 ms, execute ~3,44 s, bütün
metraj ~9,75 s, tek ayırıcı + takipçileri ~23 ms. Bunlar release/GPU veya
etkileşim süresi kabulü değildir. Metrajın her mahal/yöntem için ayrı
`space_found` kurması ölçülmüş darboğazdır; ortak immutable arama oturumunun
sonuç eşitliğiyle uygulanması ayrı takip işidir. ARCH-SPACE-01/ARCH-PERF-01
bu nedenle tamamlanmış sayılmaz.

## Gerçek arayüzde bulunan rapor ve yerleşim kusuru

59–62 numaralı ham görüntüler, kolon düşümü/kaynak alt kümesi gibi başka
seçenekleri olmayan bir mahalın sınırı koptuğunda, modeldeki `lostEnclosure`
uyarısının Metraj'da eksik olduğunu gösterdi. Son alanı saklamak doğrudur;
onu güncel ölçü gibi yeşil onayla göstermek doğru değildir. Public Python
denemesi sanal 4×3 sınır silinince 12 m² için aynı eksikliği doğruladı.

`boundaryBreak` artık her kopuk mahalda mevcut nedenini taşır ve `rule`
son geçerli sınırı açıklar. Önceden geçerli satırların biçimi değişmez;
kopuk eski satırlar bilinçli olarak mevcut opsiyonel uyarı alanını kazanır.
Alan/çevre/hacim ve toplamlar son geometriyi içerir; API istemcisi neden
alanını okuyabilir. Dosya şemasına veya model geometrisine yeni alan eklenmez.

Metraj bütün satırlardaki kopuklukları sayar; ilk mahal geçerli olsa da
uyarır. Yalnız kopuk satır varken açılan Durum sütunu aynı bilgiyi kopya ve
dosya çıktısına da koyar. Bu sütun geçerli raporların eski çıktı sütunlarını
değiştirmez. 18 px dar pencerede bulunan başlık çakışması için sütunlar
seçili yazı tipinin ölçülmüş genişlikleriyle kurulur; uzun adlar sarılır,
sayılar yatay kaydırmada bütün kalır, alt düğmeler görünür tutulur.

Son derlemede 67–75 ham ekran, iki gerçek CSV ve bağımsız kaydedilmiş model
karşılaştırması bu düzeltmeyi doğruladı. Bir Undo kaynak duvarı onarıp
uyarıyı kaldırır; ikinci Undo eski kolon tercihine/34,22 m²’ye tam döner.

## Kabul ve kalan sınır

[Kabul dosyası](../baseline/2026-10-25-architecture-space-separators/README.md):
105 bağımsız ortak durum, 267 uygulama testi/22 atlanan, 300 Python testi,
6 gerçek MCP host testi, metraj görünüm/dosya regresyonu; KCAD/DXF/etkileşim,
WASM, clippy, katalog/SDK/manifest ve paketli desktop build kontrolleri.
35 gerçek GUI kaydı, 76 değiştirilmemiş JPEG; kaynak/model/ayar korunumu,
Decimal alan, tek onay/undo/redo/kayıt-aç ve gerçek CSV doğrulanmıştır.
Yeni TS/TSX uygulama kaynağı yoktur; JSON katalog sözleşmeden üretilmiştir.

Mahal ayırıcı ayarlarının iki tema × iki yazı × iki pencere görünümü
doğrulanmıştır. Bu, bütün araçların tam iş akışı matrisi değildir. Metraj
yatay ray tıklaması çalışır; tekerlek/thumb sürükleme denemeleri sonuç
vermediğinden başarılı kabul sayılmaz. Genel üst araçların tüm ret paritesi,
blok içi mahal kaynağı ve DWG roundtrip bu dilimde tamamlanmamıştır.
ARCH-SPACE-01 ve ARCH-PERF-01 [~] kalır.
