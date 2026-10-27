# ADR 0696: Modelde yakınlaştırmaya göre duvar ayrıntısı

- Tarih: 27 Ekim 2026.
- Durum: aşağıdaki dar GPU ve gerçek GUI vakaları geçti; son ortak test kapıları ve kapsam sınırları nedeniyle nihai özellik kabulü henüz verilmedi.
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
