# ADR 0843 — Birleşik parametrik plaka ve okunabilir kısıt çubukları

2026-11-02. CAD-ADV-05 birleşik kabulü tamamlandı.

## Dayanak ve bulunan eksik

[AutoCAD grip düzenlemesi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-D3DE4062-EAFB-4CB3-9111-2EF19AD4C2D3.htm)
kalıcı bağları koruyan düzenlemeyi açık gevşetmeden ayırır.
[Geometrik bağları görüntüleme](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-56EA46E3-5425-4368-99A1-AD159B28DE4D.htm)
çubukların geometriyle ilişkisinin hover ile anlaşılmasını ve kalabalıkta
görünüşünün yönetilmesini anlatır. [BricsCAD 2D constraints](https://helpcenter.bricsys.com/en-us/document/bricscad/drawing-accurately/2d-constraints?version=V26)
ölçüsel, geometrik ve yalnız ölçen parametreleri aynı çalışma içinde ayırır.

Tek tek geçen eğri kabulü, birden fazla kısıtlı grubun birlikte çalıştığını
kanıtlamıyordu. Gerçek birleşik çizimde yakın ama farklı noktalara ait
çubuklar, uzun ölçü etiketlerinin üzerine çiziliyordu. Aynı noktadaki
çubukları gruplayan eski düzen bu çakışmayı yakalamıyordu.

## Sınırlı değişiklik

- Mevcut dört piksel ortak nokta grubu, tam yazı, birim, simge, kırpma ve
  hover ile kaynak geometriyi vurgulama yolu korunur.
- Bağımsız çubuklar en yakın boş dikey sıraya yerleşir. Adaylar kararlı
  kısıt sırasıyla denenir. Dikey şerit dizini yalnız ilgili satırlardaki
  dikdörtgenlere bakar; her aday bütün çizim işaretleriyle karşılaştırılmaz.
- Hover boyalı son dikdörtgeni kullanır. Aynı kamera aynı yerleşimi verir.
  Bu yalnız ekran yerleşimidir; nesne, kısıt, tolerans, geometri, pafta ve
  dosya şeması değişmez. Çözücü yeni bir teşhis üretmez.
- Ekran gerçekten doluysa üst üste işaret üretilmez; tam küme mevcut
  Parametreler penceresinde kalır. Keyfi büyüklükte bir kümenin her
  işaretini aynı anda göstermek veya kullanıcıya ait kalıcı çubuk
  konumu saklamak bu düzeltmenin kapsamı değildir.

## Tekrar kullanılabilir kabul çizimi

`fixtures/interaction/v1/constraint-technical-plate.kcad`, büyük koordinatta
(E=487000, N=4420000) dokuz nesne, 24 bağ, yedi parametre ve dört kalıcı
serbestlik içerir. Resim değildir; bağımsız KCAD okuyucusuyla incelenebilir.

- Dört ayrı çizgiden 12×8 m plaka; çakışık köşeler, yatay/paralel/dik
  kenarlar, sabit köşe, genişlik/yükseklik ölçüleri.
- Eşit iki daire; sabit ilk merkez, yatay merkezler, 6 m merkez uzaklığı
  ve 1 m yarıçap. Üstteki çizgi daireye teğet, yatay ve uçları merkezlere
  düşey hizalıdır. Aynı geometriden gereksiz ikinci teğet bağı eklenmez.
- `taban=6`, `genislik=taban*2`; yalnız ölçen `kosegen` sürücü değildir.
- Merkez/dikiş uzaklığı tutulan 4/2 m tam elips ve dikişe bağlı çizgi;
  elipsin serbest diğer boyutu sürüklenen eksene uyum sağlar (ADR 0842).

## Kabul adımları

1. `delik_yaricapi` 1→1.5 m: iki daire eşit büyür, üst teğet çizgi birlikte
   yükselir. Merkezlerin sağ/sol ve teğetin üst dalı korunur.
2. `taban` 6→7 m: bağımlı genişlik 12→14 m; başvuru köşegeni gerçek yeni
   uzaklığı ölçer. `delik_araligi` 6→8 m ikinci deliği ve kılavuz ucunu taşır.
3. Elipsin kısa ekseni 2→3 m: plakanın yedi nesnesi değişmez; izleyen çizgi
   hareket eder. 24 bağ geçerli, kalıcı dört serbestlik korunur.
4. Çizim katmanı kilitliyken yarıçap değişikliği `layer_locked` ile reddedilir;
   son geçerli geometri, parametre ve dosya korunur. Kilit açmak açık bir
   katman eylemidir; katman görünüş/lock yolu mevcut sözleşmede Undo adımı değildir.
5. Sabit plaka köşesini 1 m kaydırmak normalde `constraint_conflict` ile
   atomik reddedilir. Açık gevşetmeyle yalnız bozulan üç bağ kalkar: bir
   çakışık nokta (iki denklem), Fix (iki), genişlik (bir); 4→9 serbestlik.
   Kalan 21 bağ geçerlidir. Geometri ve bağ değişikliği tek Undo'dur.
6. UI/Python dosya eşitliği, salt okunur plan, kaydet/aç ve tek Undo
   başlangıç baytlarının tam karşılaştırmasıyla doğrulanır. MCP aynı
   çizimde aynı senaryoları çalıştırır; başka test geometrisiyle ikame edilmez.

## Sonuçlar

İlk gerçek GUI/Python dosya karşılaştırmaları geçti: yarıçap, elips grip,
gevşetme ve üç tek Undo; kilitli ret parametreyi korudu. MCP birleşik
senaryosu geçti. Yerleşim testinde gerçek çizimin bütün işaretleri üç
pencere boyutunda içeride, çakışmadan ve tekil hover hedefiyle kaldı.
Temiz adayda masaüstü kitaplığının 1583 testi (mevcut 256 ignore), MCP'nin
49 testi, Python kısıtlarının 19 testi ve bağımsız KCAD doğrulaması geçti.
Masaüstü derlemesi ve desktop/MCP tüm hedef Clippy kontrolü geçti.

Son temiz derlemenin gerçek Mac penceresinde uzun ölçü etiketleri ve
bağımsız simgeler çakışmadı. Taşınmış Teğet simgesinin hover'ı doğru daire
ve çizgiyi vurguladı. Uzak zoom ve 2204×1452 fiziksel piksel dar pencere
kontrol edildi. Dar pencerede taban 6→7 m tek Enter'la genişliği 14 m yaptı;
yirmi dört bağ/dört serbestlik korundu. Kaydedilen sonuç Python'la tam
aynı; tek Undo başlangıç baytlarını aynen getirdi. Kamera/yerleşim dosyayı
etkilemedi. Son kısıt satırlarına kaydırılarak Teğet/Düşey bağları da
yeşil görüldü.

Ekranlar repo dışındaki `/tmp/kentos-cad-adv04-2026-11-01/144-…153-*.png`
dosyalarıdır; sonuçlar `combined-plate-clean-results.json` ve ilgili
loglardadır. Ürüne görüntü veya yeni dış bağımlılık eklenmedi.

Bu kabul, yayımlanmış destek matrisindeki aşamalı 2B kapsamı kapatır.
Genel NURBS kontrol/G2, genel elips teğeti ve kalıcı bağımsız elips eksen
başvuruları uygulanmış sayılmaz; sınırlar `docs/cad/parametric-support.md`
üzerinde açık tutulur.
