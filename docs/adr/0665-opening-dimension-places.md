# ADR 0665: Açıklığın gerçek noktalarına bağlı ölçü

- Durum: mevcut açıklık noktaları ve ilişkili ölçü dilimi kabul edildi (26 Ekim 2026); otomatik ölçü sırası ayrı açık iştir.
- Taban: `ea7d1baee`, aynı mimari dal; ARCH-DIM-01'in ilk altyapı dilimi.

## İnceleme

Autodesk'in [AEC ölçü noktaları](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-085E70B2-9AE9-4091-8341-C1EC4F8FAD2C.htm)
açıklık genişliği/merkezi ile ayrı ölçü sıralarını tanımlar. [2026 destek notu](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/DIMASSOC-not-associative-with-AEC-Objects-like-walls-in-AutoCAD.html)
standart AutoCAD ölçüsünün AEC nesnesini otomatik izlemesini varsaymamayı,
bu nesneler için AEC ölçülerini kullanmayı söyler. [İlişki kopması](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT-DidYouKnow/files/GUID-7C7B6F58-2E17-4A77-9579-D42E205800BF.htm)
görünür olmalı ve yeniden bağlama ayrı eylemdir. [FreeCAD Window](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Arch_Window.md)
kapı/pencerenin host ve kaynak geometrisini ayrı tutar. Bunlar ürün yönü
karşılaştırmasıdır; KentOS için standart/mevzuat uygunluğu iddiası değildir.

KentOS'ta duvar yüzlerinin `vertex/mid` başvurusu vardır; açıklığın dört
kalıcı jamb köşesi ve başlangıç/bitiş/merkez noktaları `ops::subref::point`
tarafından okunmuyordu. Native Ölçülendirme keneti bu nedenle açıklık
köşesini serbest koordinat olarak yazabiliyordu. Otomatik sıra üretmekten
önce mevcut ölçü sisteminin bu gerçek kaynak noktalarını izlemesi gerekir.

## Karar

- Yeni komut veya kalıcı yer türü eklenmez. Mevcut `SubPlace` içinde açıklık:
  `vertex(part=0,ring=0,index=0..3)` sırasıyla başlangıç-sol, başlangıç-sağ,
  bitiş-sol, bitiş-sağ jamb köşeleridir. Yaprak, açılım yayı veya kasa çizgisi
  sırası değildir. Sol/sağ host duvarının yönüne göredir.
- `start/end` iki jambın orta noktaları, `center` bunların orta noktasıdır.
  Dairesel duvarda bunların doğrusal ölçüsü kiriş mesafesidir; host boyunca
  nominal açıklık eni veya doğrama net geçiş eni diye sunulmaz.
- Yalnız dört geçerli köşesi olan, kopuk olmayan açıklığın yeri ölçülebilir.
  Host kaybı/duvar dışına çıkma/çakışma eski koordinatları ölçüye sessizce
  geçerli diye vermez; mevcut `no_such_place` ve kopuk bağ davranışı kullanılır.
- Açıklık aynı tür ailesidir; kapı/pencere/boşluk ve el/yön değişimi kaynak
  kimliğini korur. Başka nesneye dönüşüm/silme kopar. Duvar→açıklık→ölçü
  mevcut tek işlem sırasında izlenir, undo/redo birlikte döner.
- Gerçek GUI eski sürümde jamb kenedinin duvar kesim parçasını adlandırdığını
  ve ölçünün `refs` olmadan kaydedildiğini gösterdi. Ortak store yalnız
  Endpoint kazananının görünür/keneti açık/geçerli aynı host açıklığının
  dört köşesinden biri olması halinde kaynak kimliğini açıklığa bağlar.
  Koordinat/tür/sıralama aynıdır; noktanın uzamsal komşuları sorgulanır,
  tüm proje açıklıkları taranmaz. Diğer nesne/türler ve katman maskeleri korunur.
- Genel Zincir/Baz/Hızlı ölçü, birim/ölçek/render ve dosya şeması değişmez.
  Mevcut ölçü oluştur/yeniden bağla, Python ve MCP aynı noktayı okuyacaktır.

## Kabul kapıları ve açık kapsam

- Bağımsız köşe/orta nokta beklentileri; bütün açıklık türleri, ters/eğik/yay
  host, büyük koordinat; genişlik/yer/kalınlık/değişen host/bölme/silme.
- Mevcut native kenetle ilişkili ölçü, önizleme/tek onay/geçmiş/ret/iptal;
  kaynak ve ilgisiz nesneler tam kayıt karşılaştırmasıyla korunmalı.
- Python/MCP plan/uygula/undo/kaydet-aç; eski KCAD baytları korunmalı.
- Genel otomatik ölçü sıraları, çekirdek/iç-dış yüz seçimleri, metin çakışma
  önizlemesi ve çoklu görünüş/pafta kapsamı henüz yapılmış sayılmaz.

## Doğrulanan dilim

[Kabul kaydı](../baseline/2026-10-26-architecture-opening-dimensions/README.md):
son native takımlar, 809 commands, 342 yeniden derlenmiş Python ve 76 MCP
komut testi; desktop build, ilgili paketlerin all-targets clippy denetimi,
WASM check ve SDK/katalog eşitliği geçti. Gerçek eski/yeni GUI, tek onay,
undo/redo, genişlik/konum/kalınlık, kaynak silme/kopuk bağ, Esc ve yeni süreçte
açma doğrulandı. 36 ham ekran (iki tanısal),21 tam model ve dört kabul dışı
kopya saklandı; 1828 önceki KCAD dosyası byte aynı. Açık13/koyu13-16 ve
geniş/dar pencereler sınandı; tüm görünüm/iş akışı matrisi tamamlanmadı.
Kaynak slot-id beklentisi, yanlış kanıt kopyalama yolu ve yazı boyutu meta
verisi düzeltmeleri kanıt kaydında açıklanır; raw veriler korunur.
Genel otomatik sıra üretimi/üyelik ve çoklu görünüş/pafta bu kabulün dışındadır.
