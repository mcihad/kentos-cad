# ADR 0876 — Mevcut bloklarda dünya X/Y grid örüntülerini inceleme

Tarih: 3 Kasım 2026. CAD-ADV-07'nin dikdörtgen örüntü dilimi.

## Dayanak ve mevcut model

[AutoCAD Web ARRAY](https://help.autodesk.com/cloudhelp/ENU/AutoCAD-Web-Help/files/List-Commands/ARRAY_COMMAND.html)
dikdörtgen/kutupsal/yol düzenini ayırır; ilişkili dizide sayı, aralık ve tekil
öğeler sonradan düzenlenebilir. [BricsCAD V27 Arraying entities](https://help.bricsys.com/en-us/document/bricscad/modifying-entities/arraying-entities?version=V27)
dikdörtgende satır/sütun sayısı ve ayrı aralıkları, yol/kutupsalda ayrı parametreleri
belirtir. [ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
bulunan örüntüyü inceleme ve seçileni bağlama için dayanak olmaya devam eder.
Bunlar ürün akışının kaynaklarıdır; kapalı kaynak tanıma algoritmaları incelenmiş değildir.

KentOS'un `contracts::ArrayLayout::Grid` ve `geometry-core::ops::assoc_array`
modeli satırları dünya Y, sütunları dünya X yönünde öteler. `AssocArray.rotation`
bloğun yönüdür; grid eksenlerini döndürmez. Bu dilim modele gizli bir dönük
çerçeve yüklemez, dosya şemasını değiştirmez. Dönük/eğik grid tanıma açık kalır.

## Geriye uyumlu açık seçim

`cad.arrays.find` v1 girdisine opsiyonel `kind: line | grid` eklenir. Alan
yoksa önceki doğrusal davranış korunur. `grid` yalnız dünya X/Y dikdörtgenlerini
arar. Python `ArrayFindKind`, tipli SDK, TS ve MCP şeması aynı katalogdan gelir.
Çıktı önceki `ArraysFound` ve `ArraysAdopt` tipidir; gizli uygulama yapılmaz.

Kaynak/tanım/veri/katman/görünüş/ayna/ölçek/yön koşulları ADR0874 ile aynıdır.
Eşleşmeyen blok kendiliğinden `kept` olmaz. Kaynak gridin içinde olabilir;
çıktının kaynağı seçilen dolu sınır köşesine eşlenir. Bütün kökler aynı UUID ile kalır.

## Aralık ve üyelik doğrulaması

- Float64 yerel farklar seçili kaynağa göre alınır. Aynı satır/sütun çevresindeki
  gerçek blok çiftlerinin uzaklıkları aralık önerisidir. Bir boşluğu ikiye
  bölerek gözlenmemiş satır/sütun icat edilmez.
- Her aday aralık çiftinde kökler tam sayılı satır/sütun yerlerine mutlak model
  toleransıyla oturmalıdır. En az iki satır, iki sütun, dört dolu yer ve her
  yönde aynı satır/sütunda gözlenen komşu çift gerekir.
- Aynı yere iki UUID düşerse biri rastgele seçilmez; aday reddedilir.
  Eşleşmeyen kökler ve farklı şablonların gerekçeleri önceki açık listelerde kalır.
- Mevcut bağlama sözleşmesi ilk öğenin kaynak olmasını gerektirir. Bu nedenle
  sınır köşelerinden biri dolu olmalıdır. Önce alt-sol, yoksa alt-sağ, üst-sol,
  üst-sağ seçilir; gerekli aralık işareti kullanılır. Boş köşeye nesne eklenmez,
  eksik bütün köşeler için tam grid önerisi verilmez.
- Yerler satır satır, her satırda sütun sırasındadır. En çok 10 000 yer;
  eksikler açık boş kayıttır. Her öneri mevcut `arrays_adopt::inspect` ve
  gerçek yerleşim motorundan yeniden geçer. Hiçbir mevcut kök toleransı
  gizlemek için taşınmaz.
- Önce en dolu açıklamalar gelir; öz alt kümeler bastırılır. Kesişen alternatifler
  `overlaps` taşır. Benzer yerleşim, anlamca aynı bileşen olduğunun kanıtı değildir.

## İş sınırı ve iptal

Kapsam 2 048 kök ve toplam iş bütçesi iki milyon sayılan denemedir. Aralık
keşfi 500 000 çift veya bir eksende 128 aralık adayını aşınca keşif durur;
kalan bütçeyle keşfedilen öneriler incelenir. Sonuç bu durumda açıkça kısmi
`truncated` ve `recognition_truncated` uyarısıdır. Bu sınırlar tam tarama
kanıtı diye sunulmaz. Arama/yer oluşturma döngüleri iptali kontrol eder;
ortak bağlama denetimi de iptal edilebilir. Kaynak dizisi ve öneri yerleri
sınırlıdır; arama UI iş parçacığında çalışmaz.

## Masaüstü

**Dizileri bul… / DIZIBUL** penceresi **Düzen türü** seçimi sunar:
Doğrusal veya **Dikdörtgen (X/Y)**. Grid sınırı pencerenin içinde açıklanır.
Kart satır/sütun sayısını, dolu/boş yerleri ve iki bağımsız aralığı gösterir.
İncelemede şema satır satırdır; bu geometrik sahne resmi değildir.

Elle taşınmış bloğun korunacağı **Satır** ve **Sütun** ayrı alanlardır;
kaynak köşeden 1 ile başlarlar. Dolu yer ve kaynak yeri değiştirilmez. Atama
taslağı ortak native incelemeden geçer. Hatalı atamada onay pasiftir; hata,
kaydırılan formun altında saklanmaması için düğmelerin üstünde sabit gösterilir.
Bul/Göster/İncele/Vazgeç/Job/Undo ve eskilik kuralları ADR0875 ile aynıdır.

## Doğrulama kapısı

Bağımsız beklenen yerleşimlerle iki boş yer, iç kaynak, eşleşmeyen blok,
işaretli aralık, boş köşe, bütün köşelerin eksikliği, gizli yarım aralık,
yalnız üç köşe, yalnız sıra, dönük kare, çakışan UUID, ters kapsam, büyük
koordinatta küçük aralık, arama sınırı ve iptal sınanır. Ortak JSON durumları,
tipli Python/Job ve MCP aynı komuta ulaşır. Grafik kabulde satır/sütunla açık
atama, ret sonrası düzeltme, vazgeçme, bağlama, aralık ve Undo/Redo dosya
karşılaştırmaları yapıldı. Gerçek uygulamada içteki bloktan 3×4 grid, kaynak yere
atama reddi, geçerli sütuna düzeltme ve bağlama denendi. Kaydırılan formda
ret mesajının gizlenmesi görülüp sabit geri bildirimle düzeltildi ve tekrar
fareyle doğrulandı. Bağlama on kökü, tanımı, proje ayarlarını ve bağlı ölçü
referanslarını değiştirmez; iki boş yer ve tek korunan öğe kalır. Gerçek
menüde sütun 2000→2500 mm, satır 3000→4000 mm yapıldı. Korunan öğe yerinde,
ölçü 2500 mm'dir. Bağımsız KCAD okuyucusu beş byte eşitliği çiftini ve bütün
UUID/konum/ref koşullarını doğruladı. Ekranlar ve ikili özetli kabul manifesti
`/tmp/kentos-cad-adv04-2026-11-01/array-grid-gui-acceptance.json` içindedir;
ham ekranlar repoya eklenmez.

Native odak koşusunda 15 tanıma testi; geniş koşuda 99 birim ve 400
entegrasyon başarılıdır. Ortak durum filtresi masaüstüne özel fixture
klasörünü boş bıraktığı için o tek kontrol ayrıca filtresiz çalıştırılıp
başarılı bulundu. Bu filtre hatası ürün başarısı sayılmamıştır. 31 bağımsız
ortak durum native ve Python'dan geçti. Python dizi modülünde 13, MCP'nin
tüm gruplarında toplam 128, masaüstü panelinde 28, geçmişte 12, UI kataloğunda
9 test başarılıdır. Şema/katalog ve 1050 TS bağlama kontrolü, SDK/fixture/
manifest tutarlılığı başarılıdır. Komut/native/desktop/MCP/Python tüm
hedefler Clippy son kaynakta temizdir. İlk Clippy koşusundaki tek iç içe
koşul uyarısı birleştirilerek giderilmiştir.

Dönük/eğik grid, kutupsal/yol tanıma ve kaynak/ilişki genişlemesi açık kalır;
CAD-ADV-07 bu dilimle kapanmaz.


Son teslim kaynağında dokuz dizi arayüz testi tekrar başarılıdır. Son derlemeyle
arayüzden grid arama/inceleme/açık atama/bağlama tekrarlandı; bağımsız okuyucu
özgün köklerin aynılığını ve incelenen üyeliğin korunduğunu doğruladı.
