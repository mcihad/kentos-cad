# ADR 0696: Modelde yakınlaştırmaya göre duvar ayrıntısı

- Tarih: 27 Ekim 2026.
- Durum: kabul edildi (29 Ekim 2026); bölünmüş görünümde görünüş başına ayrıntı, taze PDF matrisi ve son kapılar
  aşağıdaki 29 Ekim ekinde. Kanıtlanmayan sınırlar o bölümde ayrıca yazılıdır.
- Kapsam: model alanındaki otomatik duvar gösterimi ve yoğun sıfır genişlikli GPU taraması. Geometri, proje birimi, ölçü yüksekliği ve pafta ölçeği değiştirilmez.

## Bulgular ve araştırma

Kullanıcının bildirdiği yakınlaştırma sorunu önceki kabul edilen uygulamada
tekrarlandı: otomatik ayrıntılı 1:100 projede duvar ekranda yaklaşık 120 piksel
kalınlığa getirildiğinde de bileşenleri görünmüyordu. Ham kanıtlar yerel
`.run/gui-evidence/2026-10-27-wall-dimensions/05-before-style-100.png` ve
`06-before-style-100-close.png` dosyalarındadır; görüntüler repoya eklenmez.

Stil penceresinin örneği her zaman ayrıntılı/1:20 gösterilirken, modelin
otomatik gösterimi yalnız proje `plotScale` değerinden seçiliyordu.
`plotScale` 100 veya 200 olduğunda kamerayla yaklaşmak, şematik görünüşü
değiştirmiyordu. Ayrı bir GPU hatasında, taramanın sıfır genişliği ekranda
bir piksellik çizgi sayıldığı halde sıklaşınca ortalama kapsam sıfır
genişlikten hesaplanıyor ve alfa sıfıra düşüyordu.

[Vectorworks 2026](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Preferences/Setting_document_preferences.htm?agt=index)
tasarım katmanlarının otomatik ayrıntısını açık bir gösterim seçeneğiyle
tanımlar; duvar bileşenleri orta ve yüksek ayrıntıda görünür.
[AutoCAD Architecture gösterim sistemi](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-8512B736-FC8A-4559-BFF5-7F097517A1C2.htm)
gösterimi viewport ve çizim işi üzerinden ayırır.
[Archicad](https://helpcenter.graphisoft.com/user-guide/76859/)
yapı gösterimini görünüşte saklayabilir. Bu kaynaklar ekran gösterimi ile
çıktı gösteriminin ayrılmasını destekler. KentOS'un model kamerasıyla otomatik
ayrıntı artırması kendi kullanıcı deneyimi kararıdır; bu programların aynı
kamera davranışını kullandığı iddia edilmez.

[PBRT 4, Texture Sampling and Antialiasing](https://pbr-book.org/4ed/Textures_and_Materials/Texture_Sampling_and_Antialiasing)
tek tek çözülemeyen yüksek frekanslı desenlerde alan ortalamasının korunmasını
açıklar. GPU tarama düzeltmesinde bu ilke kullanılır; geçersiz koordinatı
veya geometriyi toleransla gizleme yapılmaz.

## Karar

- `arch.detail` açıkça Şematik/Uygulama/Ayrıntılı seçilmişse aynı seçim
  korunur. Kalıcı ayar veya dosya şeması eklenmez.
- CAD model alanındaki otomatik seçim, kameranın mevcut 96 dpi ekran
  paydasını kullanır. İlk ayrıntı eşikleri mevcut 1:30 ve 1:75 değerleridir.
  Ayrıntı yalnız `wall::View.detail` alanını etkiler.
- `wall::View.scale` proje `plotScale` değeri olarak kalır. Bileşen taraması,
  yazı, ölçü veya mimari geometri kamerayla yeniden ölçeklenmez.
- Ortak `walls::screen_view` yardımcı işlevi açık ayrıntı seçimini ve
  geçersiz ekran paydasında eski görünüşe dönüşü korur. Eşiklerde %10
  dönüş aralığı, küçük ileri/geri yakınlaştırmaların ayrıntıyı titreştirmesini
  önler. Masaüstü mevcut 150 ms gezinme sakinleşmesini kullanır; tekerlek
  hareketi sırasında her olayda duvarlar yeniden üretilmez.
- Modelin `Spatial` indeksi gösterim değişikliğini kaynağa yazmadan alır.
  Mevcut styled cache, `wall_view` değişince yalnız duvar içeren parçaları
  yeniden kurar. Proje geçmişi/revizyonu değişmez.
- Pafta viewport'u, pafta düzenleme kapsamı ve PDF/çıktı yolları mevcut
  `view_at` kuralını kullanır. Pafta ile proje paydası aynı olsa bile model
  indeksi daha ayrıntılıysa `SheetMaps` kendi pafta indeksini oluşturur;
  aynı model indeksi sessizce tekrar kullanılmaz. Mevcut cache anahtarının
  kapsam bayrağı bu farkı izler.
- Proje ayarlarındaki mevcut otomatik seçimin açıklaması, modelde
  yaklaşınca bileşenlerin görünmesini ve paftanın kendi ölçeğini kullandığını
  söyler. Yeni seçenek grubu veya kullanıcıya yeni ölçek sistemi eklenmez.
- GPU `hatchFs` yoğunluk dalında yalnız sıfır genişlikli taramanın bir
  cihaz pikseli kapsamı korunur. Pozitif fiziksel genişliğin mevcut kapsam
  geçişi ve dosyadaki/vektör çıktıdaki çizgi genişlikleri değiştirilmez.

## Kabul kapıları

- Ortak yardımcı: 1:100/1:200 projede ekran 1:200 → 1:50 → 1:20;
  Şematik → Uygulama → Ayrıntılı; `View.scale` ve ayarlar aynı kalmalı.
- Açık ayrıntı seçimi, geçersiz kamera değeri ve iki eşikte küçük dönüşler.
- Gerçek store: bileşen sınırlarının/taramalarının gelmesi, desenin kağıt
  ölçeğinin korunması; hiçbir kaynak/kimlik/geçmiş yazısı olmaması.
- Masaüstü: hareket boyunca ayrıntının sabit kalması, sakinleşince
  güncellenmesi; model/pafta geçişi; aynı paydadaki paftanın kendi indeksini
  kullanması ve model yakınlaştırmasının pafta cache'ini yeniden üretmemesi.
- Gerçek GPU readback: yedi sık desen seviyesi, tek örnek ve 4× MSAA;
  sıfır genişlikli desen kaybolmamalı, pozitif genişlikli %2 kapsam korunmalı.
- Yeniden derlenmiş gerçek arayüzde 1:50/1:100/1:200 karşılaştırması, stil
  penceresiyle bileşen eşleştirmesi, yaklaş/uzaklaş ve pafta/PDF kontrolleri
  aşağıdaki kanıt kapsamıyla ayrılır; doğrulanmayan çıktı kabul edilmez.

## Tamamlanan kontroller ve sınırlar

[QA kaydı](../baseline/2026-10-27-architecture-wall-dimensions/README.md)
önceki kaybolan bileşen tanısını ve yeni gösterim vakalarını ayrı tutar.
Apple M5 / Metal üzerinde yedi yoğunluk seviyesinin iki MSAA kipindeki
**14 sürekli, sıfır genişlikli tarama** örneği ve pozitif fiziksel genişliğin
**iki** kapsam örneği gerçek GPU readback ile geçti. Bu sonuç bütün kesikli
yoğun desenler, bütün motifler veya bütün GPU/işletim sistemleri için
genellenmez.

Gerçek uygulamada otomatik 1:50/1:100/1:200 projelerde yaklaşma/uzaklaşma ve
bileşenlerin gelmesi denendi; 1:200 pafta kendi ölçeğinde korundu. 73/76/80/83
model kayıtları ve ayrı 101/125 gezinme doğrulamaları gösterimin kaynak
çizimi değiştirmediğini denetler. Son Live-3'te 125 özgün 1:100 belgeyle
aynıdır. Yeni stil görünüşleriyle tam taze PDF/çıktı matrisi yapılmış sayılmaz.

Son varsayılan paralel desktop turu 1341/0/228, UI turu 289/0 ve üç uygulama
derlemesi geçti. Son Python/API, clippy ve WASM kapıları henüz açık olduğu
için bütün özellik/yayın kabulü verilmez. Duvar stili adını kaydetmenin
önceden var olan geri alma sorunu bu gösterim düzeltmesinden ayrıdır;
[ADR0698](0698-scrollable-wall-style-dialog.md) 130 ile başarısız 131–135
denemelerini açıkça ayırır. Bunlar başarılı stil undo/redo kanıtı değildir.

İlk uygulama etkin model indeksini izler. Aynı kaynak indeksini paylaşan
ikincil karşılaştırma görünümleri için birbirinden bağımsız otomatik ayrıntı
iddiası bu kabulün kapsamında değildir; ayrı görünüş indeksleri ayrıca ele
alınmalıdır. Genel WallEdit tam önizlemesi ve tüm mimari iş listesinin
tamamlanması bu düzeltmenin sonucu sayılmaz.

## 29 Ekim eki: bölünmüş görünümde görünüş başına ayrıntı, taze PDF matrisi

### Bulgu

İlk uygulama yalnız etkin model görünümünü izliyordu: bölünmüş çizim alanının
bütün görünümleri deponun tek açılımını paylaştığından, 1:150'deki görünümün
yanında 1:15'e yaklaşılan görünüm de şematik duvar çiziyordu (ya da tersi).
Yukarıdaki “ayrı görünüş indeksleri ayrıca ele alınmalıdır” sınırı budur.

### Araştırma

- [Revit, Detail Levels](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-Customize/files/GUID-0BB861D5-27CB-4276-93AE-F2E949B0D466.htm):
  yeni görünümün ayrıntı düzeyi görünümün ölçeğinden gelir; her görünümün
  Detail Level parametresi onu istendiğinde ezer. Ayrıntı görünüşündür.
- [Archicad 29, Partial Structure Display](https://help.graphisoft.com/AC/29/INT/_AC29_Help/050_ViewsVB/050_ViewsVB-94.htm):
  yapı gösterimi o anki pencereye uygulanır ve her görünüş için ayrı saklanabilir.
- KentOS kararı: otomatik ayrıntı her görünümün kendi kamerasından gelir; Proje
  ayarları › Mimari'deki açık seçim bütün görünümlerindir. Görünüşe özgü kalıcı
  ayar, dosya alanı ya da şema eklenmez; bölünmüş görünüm oturumundur (ADR 0523,
  0576, 0637). Bu programların kamera eşiklerini kullandığı iddia edilmez.

### Karar

- Depo her nesnenin bir açılımını tutar: etkin görünümünkini (seçme ve kenet
  onunla). Öbür görünümlerin duvarları deponun birleşmiş duvar çiziminden kendi
  `wall::View`'larında yeniden parçalanır (`Store::expanded_at`); yalnız görünüşe
  bağlı türler (duvar, mahal, aks, kolon, merdiven, rampa, etiket, görünüm
  işareti) ayrı açılır, öbürleri deponunkini paylaşır. İkinci depo, ikinci
  birleşim ya da açıklık çözümü yoktur.
- Stil çekirdeğinin `View.walls`'ı bir görünümün kendi duvar görünüşüdür:
  stilli katmanın geometrisi de parça stilleri de (`kentos-native-style`'ın
  `layer_call`'u) aynı görünüşün açılımlarını okur. İlk denemede yalnız parça
  stilleri değişmişti; tarama sınırları deponunkinden kaldı, test yakaladı.
- Her görünümün stilli önbelleği yalnız kendi görünüşü değişince duvar içeren
  parçaları yeniden kurar. %10 dönüş aralığı ve gezinirken sabit kalma görünüş
  başınadır.
- Başka görünüme basınca o görünümün çizdiği ayrıntı depoya geçer
  (`Spatial::set_screen_wall_scale_from`): geçişte iki görünümden hiçbiri
  ayrıntı değiştirmez.
- Kaydırmalı karşılaştır'da (ADR 0523 §3) öbür görünüm etkinin ölçeğinde
  durduğundan etkinin ayrıntısıyla çizer; ayırıcı iki ayrıntıyı ayırmaz.
  Revizyon görünümü kendi belgesinin ayarlarını kullanır.
- Pafta, pafta düzenleme kapsamı ve PDF değişmez: `view_at` ile haritanın
  ölçeği.

### Doğrulama

- Çekirdek: `store::walls::another_views_pieces_are_those_of_a_store_at_that_view`
  (L duvar, kapı, çizgi; üç ayrıntı × 1:50/1:200 o görünüşteki depoyla aynı
  parçalar, depo değişmez). Stil: `wall_views::a_layer_at_a_views_own_detail_is_what_a_store_at_it_draws`.
- Masaüstü `split_wall_tests` (4): her görünüm kendi kamerasının ayrıntısı
  (1:200 şematik dolgu, 1:20 tarama); etkinleşen görünüm 1:32'de çizdiği
  ayrıntılıyla sürer; açık proje ayrıntısı bütün görünümlerin; Kaydırmalı
  karşılaştır etkinin ayrıntısı. Gerçek GUI sahneleri `bolunmus-duvar-ayrintisi`
  ve `-etkin` (1440×900, 1100×650; koyu, açık): 1:150 şematik, 1:50 uygulama,
  1:15 ayrıntılı yan yana, etkin görünüm değişince üçü de yerinde.
- Taze PDF matrisi `wall_detail_sheet_tests::each_maps_walls_are_drawn_at_its_own_scales_detail_on_paper`:
  Dış duvar 25'ten L, dört paftada 1:20, 1:50, 1:100 (projenin ölçeği),
  1:200 haritası; masaüstünün pafta kipinden Dışa aktar › PDF ve başsız kitap
  (`kentos_plot::book`). poppler `pdftoppm` 600 dpi'da duvarın ortasından 4 mm
  genişliğinde şerit keser; uçtan uca mürekkepli satır dizileri sayılır:
  ayrıntılı beş çizgi, uygulama üç, şematik iki yüz arasında duvar kalınlığında
  yarı saydam gövde (#5A5A5A %45, gri 180). Sonuçlar:
  - Otomatik, model 1:500: ayrıntılı, uygulama, şematik, şematik; başsız
    kitap aynı.
  - Otomatik, model 1:15 (depo ayrıntılı): sayfalar değişmez; projenin kendi
    ölçeğindeki 1:100 haritası da şematik kalır.
  - Uygulama: dört sayfa uygulama, iki yolda. Şematik: dört sayfa şematik.
  - Ayrıntılı: 1:20 ve 1:50'de beş ayrı çizgi. 1:100 ve 1:200'de sıvanın kenarı
    yüze bir çizgi kalınlığından yakındır (2 ve 3 cm: 1:100'de 0,2 ve 0,3 mm)
    ve onun üstüne basılır; dizi sayısı Uygulama'nınkinden çok değil, mürekkep
    satırları en az 4 fazla, gövde yok. Bu kâğıdın doğrusudur, kusur sayılmaz.
  - Otomatiğe dönünce ilk sonuç.
  Kanıt yerel `.run/gui-evidence/2026-10-29-wall-detail-pdf/` (PDF'ler,
  kırpıntılar, `pdf-matrisi.png`); repoya eklenmez.
- Kapılar: clippy `-D warnings` (geometry-core, style-core, native-style,
  interaction, desktop; bütün hedefler) temiz; `kentos-geometry-wasm` wasm32
  derlemesi; geometry-core store 82 + duvar 22, style-core 44, native-style 75,
  interaction duvar gösterimi ve Spatial; masaüstünün split, wall, navigation,
  sheet ve style:: süzgeçlerinde 172 geçti, 1 düştü:
  `style::svgedit::tests::the_sources_top_edge_is_dragged_and_a_double_click_gives_the_first_share`
  (SVG düzenleyicisinin ekransız sürükleme testi) paralel yükte düştü, tek
  başına geçer; duvar koduna dokunmaz, ayrı kayıtlıdır. `pnpm py:test`:
  SDK üreticisi güncel, yerli modül derlendi, 375 Python testi geçti,
  `kentos-python` clippy temiz.

### Sınırlar

- Gerçek GPU readback yalnız Apple M5 / Metal'de; öbür donanımlar ve bütün
  kesikli yoğun desenler genellenmez.
- Görünüşe özgü kalıcı ayrıntı ayarı (Revit'in görünüş parametresi gibi) yoktur;
  gerekirse ayrı karar.
- Web'in modeli ve WebGL2 taraması TODOS.md §0.2'de.
