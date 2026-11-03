# ADR 0879 — Mevcut bloklarda açık tam çember tanıma

Tarih: 3 Kasım 2026. Durum: uygulandı; native, Python, MCP ve gerçek masaüstü kabulü doğrulandı.
CAD-ADV-07'nin bir dilimidir; bütün madde tamamlanmış değildir.

## Dayanak ve kullanıcı kararı

[AutoCAD ARRAYPOLAR](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-A6E74297-2CB3-4B1C-A07B-69CD08630052.htm)
ve [BricsCAD ARRAYPOLAR](https://help.bricsys.com/en-us/document/command-reference/a/arraypolar-command?version=V27)
merkez, öğe sayısı, öğeler arası açı, dolgu açısı ve öğelerin dönmesini ayrı
kavramlar olarak ele alır. Bunlar yeni dizinin oluşturulması/düzenlenmesidir;
KentOS'ta bilinmeyen mevcut yerleştirmelerden bunların hepsinin kendiliğinden
çıkarılabildiği varsayılmaz. Ürün kararı aşağıdaki açık yorumdur.

Kullanıcı **Kutupsal (tam çember)** ailesini seçer. Tanıma 360°'lik yerleşimi
önerir; yalnız bir yayı dolu olan veri de bu açık seçimle, kalan yerleri boş
bir çember olabilir. Yarım yay mı, eksik tam çember mi olduğu tahmin edilmez.
Yerler seçilen kaynak 1'den, dünya düzleminde saat yönünün tersine ilerler.
En az dört mevcut öğe ve gerçekten dolu bir komşu yer çifti gerekir.
Üç noktanın her zaman bir çember belirlemesi, dizi niyetinin kanıtı sayılmaz.
Yalnız her ikinci açısal yer doluysa gizli ara yerler varsayılmaz.

## Algoritma ve güvence

- `cad.arrays.find` v1'in isteğe bağlı `kind` alanına `polar` eklenir.
  Eski çağrı doğrusal kalır; gridin dünya X/Y sözleşmesi değişmez.
  Yalnız bu ailede aynı statik/verisiz tanımın farklı dönüş açılarına
  sahip yerleştirmeleri aday kapsamına girebilir. Katman, görünüş, ayna,
  ölçek, kilit ve mevcut dizi üyeliği kuralları korunur.
- Merkez hipotezleri nokta çifti orta noktaları ve kaynağı içeren üçlünün
  çevrel merkezidir. Çevrel merkez mevcut geometri çekirdeğinden gelir;
  önce kaynağa göre farklar alınır ve ölçeklenir. Dünya koordinatlarının
  kareleri alınmaz, FP32 veya büyük koordinata göre bağıl epsilon kullanılmaz.
  Orta nokta toplam yerine farkla hesaplanır. Sonlu olmayan hipotez atlanır.
- Kaynağın yarıçapına mutlak toleransta uyan noktalar açısal sıraya konur.
  Gözlenen komşu açı farkları 4–1000 yerlik tam çember sayılarını önerir.
  Bunlar yalnız hipotezdir: gerçek konumlar aynı `item_placements` çekirdeğiyle,
  hem yönü koruma hem dönme düzeninde üretilir. Nokta ve dönüş açısı gerçek
  yerle karşılaştırılır. Aynı yere iki UUID düşerse keyfi seçim yapılmaz.
- Son kabul `arrays_adopt::inspect` üzerinden geçer. Tolerans büyütülmez;
  hiçbir kök taşınmaz, birim/ölçek veya tanım değiştirilmez. Yanlış açıdaki
  öğe eşleşmeyen olarak kalabilir; kendiliğinden korunan öğe yapılmaz.
  Aynı üyeliği açıklayan başka kabul edilmiş merkez daha düşük sapma
  veriyorsa o ölçülen merkez tercih edilir. Geçersiz hipotez geçerli olanı
  bastırmaz. Bu bir genel en iyi uyum veya bütün sayısal durumlarda eksiksiz
  çözüm iddiası değildir.
- İlişkili dizi, kaynak Insert'ın **taban noktasına** göre yerleşir.
  Genel bağımsız nesne kopyalama komutunun kutu merkezine ilişkin davranışı
  bu hesaba aktarılmaz. Mevcut ilişkili dizi yerleşim motoru esas alınır.
- Aynı 2048 kök/100 sonuç ve iki milyonluk ortak iş bütçesi geçerlidir.
  Döngüler durdurulabilir; sınıra ulaşmak görünür kısmi sonuçtur. Sorgu
  mevcut arka plan işiyle çalışır. Geç gelen sonuç, yeni belge veya değişmiş
  revizyon/generation üzerine uygulanmaz.

## Arayüz ve API

Mevcut Bloklar panelindeki **Dizileri bul…** kullanılır; yeni ribbon düğmesi
veya ayrı araç penceresi eklenmez. Aile seçimi niyeti açıklar. Kartta yer,
dolu/boş sayıları, açısal aralık ve yön davranışı görünür. İncelemede merkez,
tam açı, blok taban noktasından yarıçap, dönüş davranışı ve 1'den başlayan
saat yönü tersi yer sırası yazılır.
Kaymış öğe yalnız açık boş-yer atamasıyla korunur. Onay mevcut metadata-only
komuta gider; tek Undo/Redo sözleşmesi değişmez.

Aynı `ArrayFindKind.POLAR`, `PolarArrayLayout` ve öneri Python ile MCP'de
kullanılır. Python'un durdurulabilir `Document.start` sorgusu da aynı native
uygulamadır. AI bir adayın yorumunu inceleyip onaylamadan bulmayı bağlama
olarak ele alamaz. Metin/öznitelik, iç blok, dönük/eğik grid, yol tanıma ve
kısmi yay niyeti bu dilimin dışında kalır.

## Denetim durumu

İlk native koşuda mevcut 18 ve yeni sekiz test, toplam 26 tanıma testi geçti:
boş yerler, iç kaynak, iki dönüş davranışı, kayıpsız bağlama/Undo, büyük pozitif
ve negatif koordinatlar, gizli yarım aralık, kollinear/üç nokta, çakışma,
yanlış yön ve iptal. Büyük koordinat testleri önce aynı köklerin bilinen
merkezle native bağlama planında geçerli olduğunu doğrular.

Bağımsız Python `math` verileriyle ortak durumlar 33→38 oldu. Son eşleşmeyen öğeli 38 durum native ve Python koşucularında geçti.
Python dizi yüzeyinde 14, ilgili MCP kabulünde beş, masaüstü dizi panelinde
11 test başarılıdır. Komut kataloğunda dört ve TypeScript bağlama üretiminde
1050 test; SDK, bağımsız durum üretimi ve 946 satırlı CAD yüzey manifesti
denetimleri başarılıdır. Native uygulama/MCP/Python/masaüstünün bütün
hedeflerinde Clippy geçti; son görünür metin/yarıçap eklemesinden sonra
11 panel testi, masaüstü derlemesi ve Clippy tekrarlandı.

Gerçek fare/klavye kabulünde iki ayrı 2000 mm yarıçaplı çizim kullanıldı:
yönü sabit ve merkezle dönen altı koltuk, iki boş yer, ayrı kaymış yedinci
koltuk ve iki rehber çember. Düzen seçimi, kart, merkez/açı/yön/yarıçap,
boş yer şeması ve görünür yer açıklaması denendi. Dolu kaynak yerine atama
açık ret ve pasif onay verdi; doğru boş yere atama incelenip uygulanabildi.
Vazgeçme ve tek Undo başlangıç dosyasına, Redo bağlanmış dosyaya bayt bayt
eşittir. Bağımsız KCAD okuyucusunda bağlama yalnız `arrays` kaydını
değiştirdi; dokuz kök, UUID, tanım, geometri, ayar ve çizim sırası aynıdır.
Dönen örnekte eşleşmeyen kök bağlanmadı; final programda gerçek bağlama ve
Undo/Redo yine aynı eşitliği verdi.

Mevcut **Diziyi düzenle** menüsünden 8→10 adet değişiminde ilk sekiz yerin
kimliği ve boş/korunan durumu korundu; yalnız iki yeni öğe oluştu. Korunan
koltuk ve rehber çemberler yerinde kaldı. Adet Undo/Redo dosyaları ilgili
sekizli/onlu kayıtlarla birebir aynıdır. Ekranlar 415–424 ve ayrıntılı
`polar-gui-acceptance.json` depo dışındadır. Ham kayıtlar `/tmp/kentos-cad-adv04-2026-11-01/polar-*` altında;
ekranlar repoya eklenmez. Son teslim sonuçları daha sonra eklenir.
