# ADR 0870 — Kapalı çoklu çizginin son kenarı ölçü hesabında kaybolmaz

Tarih: 2026-11-03. CAD-ADV-07'nin ölçü/alt nesne koruma önkoşulu.

## Dayanak ve teşhis

[AutoCAD 2027 Edit Polylines](https://help.autodesk.com/cloudhelp/2027/ENU/OARX-DevGuide-Managed/files/GUID-7B4A7B69-8780-4F8E-A1C6-A195CA1F122E.htm)
kapalı durumu, köşe/kenar bulge'u ve uç genişliklerini ayrı özellikler olarak
tanımlar. KentOS kararı: kapanış durumu, ölçünün son köşeden ilk köşeye
giden kenarı bulabilmesi için gerçek geometri bilgisidir.

Native belge ile ölçü çekirdeği arasındaki JSON okuması, eski WASM
sınırının çoklu çizgi varsayılanlarını kullanıyordu: `closed` ve `widths`
veride korunuyor ama okunan `Shape` açık/genişliksiz kalıyordu. Bu nedenle
kök nesnedeki son yay kenarının ölçü bağı taşımada kopuyor; blok içinde
public yeniden bağlama komutu geçerli kapanış kenarını `no_such_place`
ile reddediyordu. Hata düzeltme öncesi domain testi ve gerçek GUI Python
konsoluyla tekrarlandı; kaynak dosyanın geometri verisi kayıp değildi.

## Karar

- Native belge geometrisi için `Entity::from_document_json` çoklu çizginin
  kapalı durumunu ve uç genişliklerini okur. Geçersiz genişlik veya kapalı
  durum tipi hata verir. Belge zaten contract doğrulamasından geçmiştir.
- `Blocks::from_document_json` aynı tanım okuyucusunu kullanır; her iç içe
  düzeyde nesneleri belge okuyucusuyla alır. Tanım, taban noktası, yerel nesne
  kimliği, öznitelik ve dinamik veriler için ikinci bir model kurulmaz.
- Ölçünün kök nesne ve blok tanımı okuması bu yolu kullanır. Public
  `cad.dimensions.reassociate` planı ve yazması, blok alt nesne noktaları ve
  belge içindeki sonraki takip aynı kapanış kenarını görür.
- Eski `FromJson`/`Shape` sınırının varsayılanları değiştirilmez; regresyon
  testi bu davranışı ayrıca sabitler. Proje birimi, çizim motoru ve dosya
  şeması değişmez; köşeye yapay tekrar noktası eklenmez.
- Çizgi gerçekten açılırsa son kenar artık yoktur: ilgili ölçü noktası
  son konumunda kalıp kopma nedeni taşır. Olmayan kenar uydurulmaz.

## Kabul

Bağımsız geometri: (0,4)→(0,0), bulge 0,25 kapanış yayının orta noktası
(-0,5;2)'dir. Kök nesne ve bir/iki düzey blok içinde bağın taşımayı izlemesi,
canlı kalması ve tek Undo/Redo ile snapshot eşitliği sınanır. Public komut
planı yazmadan aynı değeri verir; yazma aynı sonucu üretir. Python kabulü
sonraki taşıma ve tanımı gerçekten açınca yalnız kapanış kenarı bağının
kopmasını da içerir. Gerçek konsolda eski ret tekrarlandı; son sürümde kapanış yayının
(800,425) mm orta noktasına canlı bağ kuruldu ve blokla taşındı. Taşıma Undo,
bağlama Undo ve Redo kayıtları kendi önceki/sonraki KCAD dosyalarıyla bayt
bayt aynıdır. Test sonuçları planın ilerleme kaydındadır; ham görüntüler depoya
konmaz.

Bu düzeltme adayların genel blok/dizi dönüşümünü tamamlamaz; döngüsel/ters
köşe sıralamasının dönüşümdeki yer eşlemesi hâlâ ayrıca gereklidir.
