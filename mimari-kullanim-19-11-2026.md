# KentOS Mimari kullanım incelemesi · 19 Kasım 2026

## Deneme ortamı ve kanıt

macOS üzerinde 18 Kasım 21.52 tarihli yerel `target/release/kentos-cad` kullanıldı. Zaten çalışan çıplak yürütülebilir dosya masaüstü denetiminde seçilemediği için aynı ikili, projenin macOS paketleme aracıyla geçici bir Mimari test penceresinde açıldı. Yeni proje metre, yerel koordinatlar ve 1:100 çizim ölçeğiyle başladı. Görsel gözlemler bu ikiliye ait. İncelenen kaynak ağacının başlangıç revizyonu `0bd37f998`. Derlemenin bu revizyona birebir karşılık geldiği doğrulanmadı.

## Planda görülenler

| Araç | Nasıl denendi ve ne görüldü | Geliştirme talimatı ve kabul ölçütü |
|---|---|---|
| Oda | Yeni proje → Mimari → Oda. İki karşı köşeye tıklanınca kapalı duvar, mahal ve ortasında 372,71 m² alan etiketi oluştu. Araç ikinci oda için ilk köşeyi bekledi. 2880 piksel genişlikte görünür duvar yaklaşık 1000 pikselken etiket yaklaşık 42 piksel genişliğindeydi. Sağda İç net, Dış duvar 25 ve Var olanı kullan seçenekleri görüldü. | Ekranda etikete gelince oda adı, iç net en × boy ve alanı okunur bir bilgi kartında verin. Baskıdaki metin yüksekliğini koruyun. İki köşeyle oluşturma ve bitişik odada ortak duvar akışını koruyun. Kabul örneği olarak 4 × 3 m iç net odada 12 m² gösterimini ve 1:50, 1:100 çıktısını karşılaştırın. |
| Duvar | Odanın dışında iki nokta verilip Enter ile bitirildi. Durum satırı 24,251 m duvar eklendiğini bildirdi. Zincir bittiğinde araç yeni ilk noktayı bekledi. Şeritte kalınlık 0,250 m, orta hiza ve sıfır öteleme görüldü. | Başarı iletisine yeni zincirin başlayabileceğini ekleyin. İlk noktayı vermeden Esc ve zincirin ortasında Esc davranışını ayrı sınayın. |
| Kapı | Odanın üst duvarında Kapı 90 kullanıldı. K2 otomatik sayacı, duvarda salınım yayı ve imleçte 75,566–76,466 m ile 75,596–76,496 m aralıklarını karşılaştıran kırmızı çakışma kutusu görüldü. Aynı bölgeye tekrar giriş yapıldığı için ilk yerleştirmenin yanlış uyarı verdiği bu gözlemle kanıtlanamaz. | Kapalı duvar çevresi boyunca birikimli iki mesafeyi mimarın gönyeden ölçebileceği yerel bilgiye çevirin. Örneğin “K1 ile 87 cm çakışıyor” ve en yakın uygun yönü gösterin. Ayrıntıda ham aralıklar kalabilir. Çakışan gerçek ikinci kapı reddedilmeli, geçerli ilk kapı korunmalı. |
| Pencere | Aynı üst duvarın başka bir noktasında Pencere 120 ile yerleştirme denendi. Durum satırında 66,165–67,365 m ile başlayan çakışma metni kesildi. | Çakışmanın karşı nesnesini vurgulayın, sayısal yerleştirmede en yakın söveye net mesafeyi gösterin. Uzun uyarının tamamı tıklamayla açılabilmeli. |

## Benzer uygulamalarla karşılaştırma

- **Mahal sınırı ve yüzeyler.** AutoCAD Architecture 2B mahali ilişkili veya serbest oluşturabiliyor. Vectorworks, otomatik sınırlı mahalleri duvar kalınlığı ve bileşen değiştikçe güncelliyor ve kapı/pencere nişlerinin net sınıra etkisini seçilebilir kılıyor. KentOS'ta mahal kaynağı ve net alan zaten var. Geliştirme hedefi, denetçi ve çizerin aynı alanın neden değiştiğini görmesi. Sınır kenarına kaynak eleman, ölçülen yüz ve kesinti nedeni ekleyin. Bir duvarı 25 cm'den 30 cm'ye çıkarıp hangi odanın hangi yüzünden ne kadar kaybettiğini karşılaştırın. [AutoCAD Architecture – Space Types](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-80F67B48-94E9-418B-9161-8C9B325FAD84.htm) · [Vectorworks – Space settings](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Spaces/Space_settings.htm)
- **Döşeme kenarları.** Vectorworks tek döşemede duvara bağlı ve elle çizilmiş kenarları birlikte kullanabiliyor. KentOS için sınama örneği üç kenarı cepheye bağlı, dış kenarı serbest balkondur. Cephe 30 cm kayınca yalnız bağlı kenarlar yenilenmeli. Korkuluk, kaplama ve boşluk konturları ile net/brüt alan ayrıca kontrol edilmeli. [Vectorworks – Automatically bounded slabs](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Floors_slabs/Creating_automatically_bounded_slabs_from_existing_walls.htm)
- **Merdiven.** Archicad çizim sırasında geometriyi gösteriyor, kural çelişkisinde çözüm seçenekleri sunuyor. KentOS'ta merdiven, döner merdiven, denetim ve rıht ayarı bulunuyor. Geliştirme hedefi kısa kalan yürüyüş hattını sorun oluştuğu anda düzeltmek. Kilitli merdiven kovası ve sabit kat yüksekliğinde uygulanabilir rıht sayısını veya gerekli kol uzamasını hesaplayıp önizletin. Uygulama ve geri alma tek işlem olmalı. [Archicad – Stair Tool](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-178.htm) · [Archicad – Solver](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-185.htm)
- **Ölçülendirme.** AutoCAD Architecture stil üzerinden hangi zincirin açıklıkları, kesişimleri veya çekirdekleri ölçtüğünü belirleyip açıklık değişince ölçüyü yeniliyor. KentOS'ta plan, duvar ve iç ölçü zinciri ayrı komutlar. Kullanıcı hangi yüzlerin ölçüldüğünü zinciri yerleştirmeden görebilmeli. 90 cm kapıyı 100 cm'ye çıkarınca söve ölçülerinin, dış toplamın ve pafta görünümünün birlikte doğrulandığı bir kabul çizimi hazırlayın. [AutoCAD Architecture – Associative Dimensions](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Architecture/files/GUID-80E8DF2E-1722-4957-BABF-910A0072EF81.htm)

## Öncelik sırasıyla düzeltilecek işler

1. **P1 · Plan ölçülerini ikinci çalıştırmada üst üste yazma.** `acceptance-flat` çizimini mutfak yerleştirilene kadar açın. 28 nesne varken Planı ölçülendir → Enter, ardından aynı kat ve aynı seçeneklerle işlemi tekrarlayın. İlk uygulamada 114, ikincide 200 nesne vardı. İkisinde de 4 dış dizi ve 10 iç zincir bildirildi. Tek geri alma sayıyı 114'e döndürdü. Paftada iki takım tam üst üste durabildiği için kullanıcı çoğalmayı çizime bakarak kaçırabilir. `crates/native/interaction/src/arch/plan_dimensions.rs` içindeki üretim dış takımı ve her iç zinciri yeniden oluşturuyor. Üretim takımına kat, görünüm, kapsam ve kaynak kimliklerini kaydedin. Aynı kapsam bulunduğunda mevcut takımın vurgulandığı **Güncelle** ve **Ayrı takım oluştur** seçenekleri sunun. Güncellemede elle yerleştirilmiş ölçüleri ve taşınmış metinleri koruyun. Kabul için aynı komutu üç kez çalıştırın, bir kapıyı büyütün, güncelleyin, geri alın ve yeniden açın. Tek takım, doğru söve ölçüsü ve sabit dış toplam okunmalı.
2. **P1 · Boş doğrama listesinin dolu çizimde olağan sonuç gibi görünmesi.** On açıklıklı örnekte Kapı ve pencere çizelgesi on satır verdi. Doğrama listesi doğrudan yerleştirilince sıfır tip ve sıfır açıklık yazdı. Aynı başlangıçtan önce Doğrama kodları, sonra Doğrama listesi ile GK01, K01, P01 ve on açıklık geldi. İlk listeden “çizimde açıklık yok” sonucu çıkarılabilir. Önizlemede toplam, kodlanmış ve kod bekleyen açıklıkları ayrı sayın. **Kodları oluştur ve listele** eylemi önerin. Gerçekten boş kat, kodsuz dolu kat, kısmen kodlanmış kat ve tamamlanmış kat ayrı kabul durumlarıdır. Kullanıcı kodları koruyarak listeyi yenileyebilmeli.
3. **P2 · Önkoşulu eksik hesapların sonuç dili tutarsız.** Bölümsüz örnekte beş mahal varken mahal çizelgesi beş satır üretti. Metrekare cetveli ve bölüm çizelgesi sıfır satırlı tablo yerleştirdi. Otopark ve sığınak ise bina programında bağımsız bölüm gerektiğini açıkladı. Otopark verili örnekte tabloyu sahneye koymak mümkün oldu. Hesaplanamadı, gerçek sıfır ve kapsam dışında durumlarını tabloda da farklı gösterin. Kaynak sayısı, hesap kapsamı ve **Bölüm bağla** eylemini yerleştirmeden önce sunun. Boş tablo bilinçli olarak paftaya konduğunda eksik veri durumu PDF'de de okunmalı.
4. **P2 · Aynı kapı için iki ayrı “tip” anlamı var.** Bir Kapı 90 seçip aynı tiptekileri bulunca beş açıklık seçildi. Aynı kaynakla Tipe uygula, Tipten ayır ve Tipe döndür komutları açıklığın bir *proje tipini* izlemediğini bildirdi. Dahili hazır tür ile bağlı proje tipi farkını özniteliklerde iki açık satırla gösterin. Proje tipi gerektiren düğmede **Proje tipi ata** eylemi bulunsun. Bir örnekte yerel istisna oluşturup tipi güncellerken etkilenecek açıklık sayısını ve korunacak istisnaları önizletin.
5. **P2 · Türetilmiş görünüşün güncelliğini paftadan görmek güç.** A-A kesiti, batı cephesi ve salonun dört iç görünüşü oluşturuldu. Komut tamamlandığında metin, plan değişince Öznitelikler'de “eski” durumunun görüneceğini ve yeniden üretmek gerekeceğini söylüyor. Paftayı basan kişi bu nesneleri tek tek seçmeyebilir. Kaynak revizyonu ile son üretim revizyonunu görünüm ve pafta listesine taşıyın. Bir duvarın veya pencere parapetinin yüksekliği değiştiğinde etkilenen görünümleri adlarıyla gösterin. Toplu yenileme elle eklenen açıklama ve işaretlerin durumunu açıklamalı.
6. **P2 · Araç ailesinin başlığı son kullanılan işlemi açıklamıyor.** Canlı şeritte Aks yazısına tıklayıp tek bir çizgi beklerken etkin komut Aks ağı oldu. Durumda 4 × 3 aks, 18 × 10 m yazdı. Boşluk ailesinde de işlem satırı Baca 20 gösterdi. Kullanıcının tek aks veya tek boşluk beklediği anda bir tıklamanın etkisi önemli ölçüde değişiyor. Aile düğmesinin ana yüzünde son seçilen işlemin adı ve doğru simgesi kalmalı. Ok yalnız listeyi açmalı. Menüde etkin öğeyi işaretleyip bir sonraki açılışta neyin çalışacağını klavye ve ekran okuyucu adıyla da aynı verin.
7. **P2 · Şeritte yoğunluk ve eski yön tarifleri.** Yerel masaüstü testinde `architecture_ribbon::tests::mimari_s_tab_shows_at_most_thirty_buttons_and_its_main_tools_large` 32 öğe saydı ve 30 öğelik sınırı aştığı için başarısız oldu. Bu, o test ikilisinin sonucu. Görünüm bağlantıları penceresi de boşken “Mimari › Görünüm” yolunu tarif etti. Canlı şeritte ilgili yol **Mimari → Belgele → İşaret**. `apps/desktop/src/arch/viewmark_refs.rs` içindeki sabit metni katalog yoluna bağlayın. Açılış ve belge teslimi gibi seyrek kullanılan komutları gruplayın. Oda, Duvar, Kapı ve Pencere tek hareketle erişilir kalsın. 1100 ve 1440 mantıksal piksel genişlikte büyük yazıyla hem erişim hem ad doğrulansın.
8. **P3 · Çatı yardım metni desteklenen geometriyle çelişiyor.** Katalogdaki Çatı açıklamasının sonu L ve T saçakların çizilemediğini söylüyor. Köşeleri çiz yöntemiyle altı köşeli L için kırma çatı ve +4,14 mahya üretildi. Komut sözleşmesi de içe dönen saçakta dere tarif ediyor. Yardımı gerçek sınır koşullarına göre düzeltin. L ve T örneklerini kaydet–aç ve kesitle regresyona alın. Bu denemede T geometri ayrıca doğrulanmadı. [Araç açıklamaları](catalog/v1/additions.json) · [Çatı komut sözleşmesi](crates/shared/commands/src/catalog/arch.rs)
9. **P2 · Erişilebilirlik ağacında şerit ve sahne araçları görünmüyor.** Canlı Mimari ana penceresinde erişilebilirlik sorgusu pencere ve sistem kontrollerini döndürdü. Şerit üzerindeki Oda ve Kapı fareyle çalıştı. Buna karşılık macOS IFC dosya seçicisinin düğmeleri, alanları ve seçimi erişilebilirlik ağacında ayrı ayrı okunabildi. KentOS UI şerit düğmelerine, açılır listelere ve etkin komut istemine rol, ad, seçili durum ve klavye odağı ekleyin. VoiceOver ile Oda'yı bulma, ikinci köşeye ölçü girme ve sonucun okunması ayrı kullanıcı testi gerektirir.

10. **P3 · Farklı kaydet sonrası aktif dosyayı ayırt etmek güç.** Yeni proje, Mimari-1911-canli-deneme.kcad adıyla kaydedildi. Pencere başlığı ve çizim alanındaki dosya adı “Yeni proje” olarak kaldı. Kaydedildi iletisi gerçek yolu kısaltarak gösterdi. Kaydedilen KCAD ayrı okuyucuda doğrulandı ve 26 nesne içeriyordu. Proje adı ile dosya adını ayrı tutun. Aynı proje adına sahip iki dosya açıldığında başlık veya sekme ipucu gerçek dosya adını ve yolu göstermeli.

## Mimari tasarım için eksik veya parçalı iş akışları

Aşağıdaki öneriler 2B plan üretimine yönelik. Bir komutu eksik sayarken Mimari şeridi, bağlamsal sekmeler, komut kataloğu, mimari etkileşim kodu ve önceki mimari plan incelendi. Örneğin KentOS'ta merdiven, asma tavan, kaçış yolu, tasarım seçenekleri, oda modülleri ve bina programı zaten bulunuyor. Aşağıdaki her öneri bunlardan birinin sağlayamadığı belirli bir kararı hedefliyor.

| Öncelik ve öneri | Mimarın yapacağı iş | Uygulama ve kabul çizimi |
|---|---|---|
| 1 · Oda programı ve komşuluk kontrolü | Henüz duvar çizmeden salonun verandaya doğrudan açılması, mutfağın girişe yakın olması, yatak odalarının geçiş mahali olmaması gibi kararları plana bağlamak. | Bina programına oda hedefi, hedef net alan aralığı ve komşuluk ilişkisinin türünü ekleyin. Duvar paylaşma ile kapıdan geçebilme iki farklı ilişki olsun. Bir kapıyı silince ortak duvar korunmalı, doğrudan geçiş kaybolmalı. Gerçekleşmeyen ilişkiye tıklayınca iki ilgili mahal görünmeli. |
| 2 · Kapı manevra alanı ve donatı çakışması | Bir kapının duvara sığdığını bilmenin yanında, kullanıcının yaklaşabildiğini ve kanadın tezgâha çarpmadığını görmek. | Kapı tipine çekme/itme tarafı için farklı serbest alan ve kanat süpürme zarfı tanımlayın. Ebatları proje veya erişilebilirlik profilinden okuyun. 90 cm kapının önüne dolap koyun, menteşeyi ters çevirin, dolabı taşıyın. Üç durumda da etkilenen alan, engel ve en dar geçiş birlikte okunmalı. |
| 3 · Engelsiz dolaşım güzergâhı | Antreden banyoya mobilya çevresinden erişilip erişilemediğini denemek. | Kaçış yollarının mahal ve açıklık bağlantılarından yararlanın. Başlangıç/bitiş, kullanıcı profili, engel sınıfları ve dar geçitleri kapsayan ayrı plan analizi ekleyin. İki kapılı odada bir yolu koltukla kapatınca diğer güzergâh veya ilk geçilemeyen kesit gösterilmeli. Hesap sonucu “mevzuata uygun” anlamına gelmemeli. |
| 4 · Bağlı ve serbest kenarlı balkon/döşeme düzenleyicisi | Üç kenarı cepheyi izleyen, dördüncü kenarı konsol ölçüsüyle belirlenen balkon çizmek. | Her kenarda kaynak duvar, hangi bileşen yüzünün izlendiği, öteleme ve kilit durumu tutulmalı. Cephe hareketi, kaynak duvarın silinmesi, balkon içerisine şaft açılması, geri alma ve KCAD yeniden açma aynı örnekle sınanmalı. |
| 5 · Yansıtılmış tavan planı | Asma tavan ve mahal ızgarasını armatür, menfez ve bakım kapağıyla tek paftada koordine etmek. | Mevcut tavan ve kaplama nesnelerine yukarı bakışlı görünüm reçetesi, görünüş aralığı, cihaz sembolü ve ızgaraya hizalama ekleyin. Aynı odanın kat planı ve tavan planı farklı ayrıntı göstermeli. Asma tavan kotu değişince sembol görünürlüğü ve mahal kaplama çizelgesi tutarlı kalmalı. |
| 6 · Bağımlı pafta ve görünüşleri tek noktadan güncelleme | Bir açıklığı değiştirdikten sonra kesit, iç görünüş, doğrama listesi ve paftalardan hangisinin güncel olduğunu teslimden önce görmek. | Var olan Gezgini, Pafta setini ve Doğrama kodlarını kullanın. Üretilen her çıktının kaynağını, son model değişikliğini ve yeniden üretim sonucunu listeleyin. Bir pencerenin parapetini yükseltince ilişkili çıktılar işaretlenmeli. İkinci güncellemede yeni kopya doğmamalı. |
| 7 · Ölçülü mobilya ve ıslak hacim yerleşimi | Bir yatağı başucundan duvara, lavaboyu ekseninden tesisat duvarına oturtmak ve kalan geçişi mobilyayı bırakmadan okumak. | Mevcut blok kitaplığına gerçek en × boy ve hangi birimden dönüştürüldüğünü gösteren yerleştirme önizlemesi ekleyin. Tutma noktası, döndürme, arka yüz ve boşluk zarfı seçilebilsin. Aynı 1,60 × 2,00 m yatağı metre ve milimetre belgede 3 × 4 m odaya yerleştirin. Çıktıda ve ölçümde fiziksel boy aynı kalmalı. |

Archicad, nesne yerleştirmeden önce tutma noktasını belirlemeye ve mobilyayı köşeye veya başka bir nesneye oturtmaya izin veriyor. KentOS için yedinci önerinin değeri, kitaplıktaki hazır yatağın ev planına hangi fiziksel boyla gireceğini mimarın o anda görebilmesi. [Archicad – Placing an Object](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-283.htm)

Vectorworks mahal ilişkilerini grafikle ve yakınlık puanıyla temsil ediyor. Archicad tanımlı kapı yaklaşım alanını planda dolgu ve konturla gösterebiliyor. Revit iki nokta arasında seçilebilir engelleri dikkate alan rota üretiyor. Bunlar ilk üç öneri için karşılaştırma noktaları. Tavan görünümündeki yukarı bakış ve görünüş aralığı beşinci öneriye örnek. [Vectorworks – Bubble diagram](https://app-help.vectorworks.net/2023/eng/VW2023_Guide/Programming/Creating_a_bubble_diagram.htm) · [Archicad – Minimal Space of Doors](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-40.htm) · [Revit – Path of Travel](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-Analyze/files/GUID-F9943131-87F1-4D88-AC69-80BEB05E6A69.htm) · [Revit – View Range](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-DocumentPresent/files/GUID-58711292-AB78-4C8F-BAA1-0855DDB518BF.htm)

## Araç bazında deneme defteri

Yedi şerit grubunda, açılır menüler ve grup altındaki ek komutlarla birlikte **121 farklı komut** var. Son grup, bu 121 içinde bulunmayan **11 mimari komutu** içeriyor. Paylaşılan genel CAD komutları (örneğin Kopyala) bu sayılara dahil edilmedi. Aynı komut birden çok sekmede göründüğünde bir kez sayıldı. [Şerit kataloğu](catalog/v1/ribbon.json) · [Ek komut kataloğu](catalog/v1/additions.json)

Canlı pencerede oda, duvar, kapı, pencere, baca seçeneği, kolon, kiriş, aks ağı, merdiven, 3B ve IFC akışları kullanıldı. Araç bazındaki geniş taramada aynı release ikilisinin **masaüstü etkileşim oynatıcısı** kullanıldı. Bu yol uygulamanın komutlarını, Iced fare/klavye olaylarını, özellik panelini ve çizim görünümünü çalıştırıyor. Ayrı bir macOS penceresinde her düğmeye fiziksel tıklama kapsamı bu yöntemle ölçülmez. Her komutta ayrı bir çizim durumu kullanıldı. Tablolardaki “pencere açıldı”, “reddedildi” ve “yerleştirildi” ifadeleri birbirinden farklı sonuçlardır. Harici kuruma teslim, buluta yayın ve hukuki uygunluk bu denemenin parçası değildi. [Etkileşim izlerinin çalışma biçimi](fixtures/interaction/README.md)

Tekrar için 2+1 dairenin 135. adıma kadarki durumunda dört bölme, kapılar, pencereler, beş mahal ve mutfak vardır. Başlangıçta 28 nesne görünür. Temel sahne `target/release/kentos-cad snapshot /tmp/daire.png --tema acik --iz acceptance-flat --adim 135` ile üretilebilir. İki kat, eğri plan ve DXF tanıma için ayrı örnekler kullanıldı. [Daire](fixtures/interaction/v1/acceptance-flat.json) · [İki katlı ev](fixtures/interaction/v1/acceptance-house.json) · [Eğri plan](fixtures/interaction/v1/acceptance-curved.json) · [Tanınan DXF](fixtures/arch/v1/recognize/kesit-binasi-zemin.dxf)


### Duvar

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Duvar <br>`tool.wall` | İki nokta ve Enter ile bağımsız duvar yazıldı. Canlı denemede 24,251 m. | Duvarı bitirme ile yeni zincire başlama durumlarını ayırt edin. |
| Dikdörtgen duvar <br>`tool.wallRectangle` | İç net referansı ile dört kenarlı duvar oluşturuldu. | Önizlemede aynı anda iç net, eksen ve dıştan dışa değerlerden hangisinin kilitlendiğini gösterin. |
| Ölçülü bölme <br>`tool.wallPartition` | Referans kenar, mesafe ve taraf verildi. Bölme yazıldı. | Ölçünün referans verdiği yüz ile bölmenin oturacağı yüzü iki ayrı çizgiyle belirtin. |
| Giydirme cephe <br>`tool.curtainWall` | İki uç ve Enter ile 4,020 m eleman üretildi. Başarı iletisi “Duvar eklendi”. | İletiye panel sırası, bölme sayısı ve artan son panel enini ekleyin. |
| Çizgiden duvar <br>`tool.wallFromLines` | Dönüşümde bir duvar eklendi, bir kaynak çizgi silindi. | Kaynakların korunması seçeneğini ve silinecek sayı önizlemesini onaydan önce verin. |
| Planı tanı <br>`tool.planRecognize` | Kesit binasının patlatılmış DXF katı içe aktarıldı. Tümünü seç → Planı tanı → Enter ile 8 duvar, 3 kapı, 4 pencere ve 8 kolon oluştu. İleti 104 kaynak nesnenin silindiğini bildirdi. | Kaynakta çizgi/yay adedini ve uygun katmanları ilk adımda gösterin. Patlatılmış DXF'de aday, güven ve reddetme nedeni birlikte görülebilmeli. |
| Oda <br>`tool.room` | İki köşe ile duvar, mahal ve etiket geldi. | İlk iki köşede gerçek oda büyüklüğünü imleç yakınında okunur tutun. |

### Yerleştir

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Kapı <br>`tool.door` | Kapılar duvarlara yerleşti, örnek izde K5'e kadar numara verildi. | İç/dış açılış, menteşe ve net geçişi yerleştirmeden önce planda işaretleyin. |
| Pencere <br>`tool.window` | Örnek duvarlarda P5'e kadar yerleştirildi. | Denizlik ve lento kotunu plan kesme düzlemiyle birlikte önizletin. |
| Boşluk <br>`tool.opening` | İlk konumda 5,00–6,00 m boşluk bir pencereye taştı ve reddedildi. Başka konumda B1 yazıldı. | Engelleyen açıklığı vurgulayın, en yakın boş aralığı seçilebilir yapın. |
| Niş <br>`tool.niche` | Aynı duvarda niş yazıldı. | Niş derinliği ve kalan duvar kalınlığını karşı yüzdeki nişle birlikte ölçün. |
| Baca <br>`tool.flue` | Duvara Baca yerleştirildi. | Duvar içi baca ve çatı üstü baca adlarını birbiriyle ilişkilendirin. Aynı şaftta düşey süreklilik sınanmalı. |
| Kolon <br>`tool.column` | Tıklanan konuma bir kolon geldi. | Tıklamanın merkez, köşe veya aks kesişimi olduğunu işaretleyin. |
| Dikme <br>`tool.post` | Bir dikme yazıldı. | Dikme ve kolonun sınıfını, kesitini ve bağlı olduğu sistemi seçimden önce gösterin. |
| Kiriş <br>`tool.beam` | İki nokta ve Enter sonunda 25/50 kiriş oluştu. | Kiriş alt kotunu ve serbest geçiş yüksekliğini kesitten önce okunabilir yapın. |
| Kayıt <br>`tool.rail` | İki nokta ve Enter sonunda 5/10 kayıt oluştu. | Kaydın hangi taşıyıcıya veya cepheye bağlanacağını önizletin. |
| Çatı <br>`tool.roof` | Daire için kırma çatıda +4,71 mahya, elle çizilen L saçakta +4,14 mahya elde edildi. | Saçak, dere ve mahyayı kenar türüne göre vurgulayın. Katalog yardımını güncelleyin. |
| Çatı bacası <br>`tool.roofChimney` | Baca tepesi +5,51 olarak gösterildi. Mahyadan 0,80 m yüksek olduğu yazıldı. | Referans mahyayı ve çatıdan çıktığı noktayı kesitte eşleştirin. |
| Çatı penceresi <br>`tool.roofWindow` | Mahyaya taşan konum reddedildi. İç konumda %33 eğimli yüzde 0,780 × 1,180 m pencere yazıldı. | Reddedilen kenarı ve pencerenin sığacağı yüz bölgesini renkle gösterin. |
| İniş borusu <br>`tool.roofDownpipe` | Oluğa yakın tıklamada Ø100 boru yazıldı. | Yalnız geçerli oluk kenarlarını yakalanabilir olarak gösterin. |
| Aks <br>`tool.grid` | İki noktayla tek aks üretildi. | Tek aks ve aks ağı düğme yüzlerinin adları farklı olmalı. |
| Aks ağı <br>`tool.gridSystem` | 4 × 3 aks, 18 × 10 m ağ oluştu. | İlk tıklamadan önce toplam en, boy ve kolon üretimi açık/kapalı özetini gösterin. |
| Merdiven <br>`tool.stair` | Düz kol yerleştirildi. Son kolun son rıhtta bittiği bildirildi. | Tıklanan uç ile hesaplanan gerçek bitişi iki işaretle gösterin. |
| Döner merdiven <br>`tool.stairCurved` | Merkez, yarıçap ve yön ile döner merdiven üretildi. | Yürüme çizgisi yarıçapını, açıyı ve sığan rıht sayısını aynı önizlemeye alın. |
| Rampa <br>`tool.ramp` | İki uç ve Enter ile rampanın son kolu eğime göre bitti. | Tıklanan yatay boy, gerçekleşen kot ve gereken kalan mesafeyi beraber gösterin. |
| Korkuluk <br>`tool.railing` | 4,020 m korkuluk, 1,100 m yükseklik ve 0,100 m çubuk aralığı bildirildi. | Merdiven ve serbest döşeme kenarı boyunca izlemeyi, dönüş ve bitiş birleşimini ayrı sınayın. |

### Mahal

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Mahal <br>`tool.space` | Farklı iç noktalarda adlandırılmış mahaller oluştu. Listede beş mahal okundu. | Alan etiketinden sınırı ve hesaplanan yüzleri tek hareketle gösterin. |
| Toplu mahal bul <br>`tool.spaceDiscover` | Beş mahalli örnekte toplu arama sıfır yeni mahal buldu ve mevcutları korudu. | “Zaten tanımlı” ile “sınır bulunamadı” bölgelerini ayrı işaretleyin. |
| Mahal ayırıcı <br>`tool.spaceSeparator` | 8,010 m ayırıcı yazıldı. | Ayırıcı kapı eşiğinde veya duvar yüzüne erişmeden biterse açık uçları önizletin. |
| Döşeme <br>`tool.slab` | Konut kabul izinde odaya tıklayarak kaynağa bağlı döşeme yerleştirildi. | Bağlı kenarı, serbest kenarı ve şaft boşluğunu ayrı seçilebilir yapın. |
| Kaplama <br>`tool.covering` | Mahale tıklamada 36 m² asma tavan oluşturuldu. | Gerçek tavan kotu ile yalnız yüzey desenini iki ayrı özellik olarak gösterin. |

### Donatı

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Mutfak <br>`tool.casework` | Dört modül, 2,970 × 0,600 m tezgâh, 0,270 m dolgu üretildi. Eviye ikinci, ocak dördüncü modülde. | Dolgunun hangi uca gideceğini ve kapak açılışlarını imleçle değiştirilebilir yapın. |
| Kitaplık <br>`block.library` | Kitaplık açıldı. Masa, sandalye, kanepe ve çift kişilik yatak aileleri görüldü. | Önizlemede fiziksel en × boy, taban noktası ve duvara dönük yüzü gösterin. |

### Düzenle

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Duvar birleşimi <br>`tool.wallJoins` | Duvar ucu seçilince birleşim türü seçimi açıldı. | Her seçenek için temizlenmiş bileşenleri planda canlı gösterin. |
| Duvarı böl <br>`tool.wallSplit` | Duvar bir noktadan bölündü. | Yeni eleman kimliklerinin hangi açıklık, ölçü ve mahali devraldığını vurgulayın. |
| Açıklığı taşı <br>`tool.openingMove` | Kapı seçilip aynı duvarda yeni konuma tıklanınca açılış taşındı. | Taşırken referans söve ve uçtan mesafeyi sabitleyebilmeliyim. |
| Mahali böl <br>`tool.spaceSplit` | Mahal ve iki kesme noktası ile iki parça oluştu. | Eski numarayı alan parçayı ve yeni numarayı onaydan önce gösterin. |
| Aksları numarala <br>`tool.gridNumber` | Önce aks ağı üretildi, tıklanan bir aks numaralandı. | Birden çok aks için sıra okları ve eski → yeni etiketler önizlensin. |
| Açıklıkları numarala <br>`tool.openingNumber` | Enter ile dokuz açıklığa numara verildi. | On açıklıklı çizimde değişen, korunan ve atlanan sayıları ayrı gösterin. |
| Mahali yeniden sınırla <br>`tool.spaceRebound` | İç nokta ve sınır güncellendi. | Eski ve yeni sınır ile alan farkını m² olarak önizletin. |
| Mahal kaynakları <br>`tool.spaceSources` | Mahal seçilince kaynak düzenleme penceresi görüldü. | Kaynakları hariç tutunca hangi kenarın ve hangi mahalin etkilendiğini vurgulayın. |
| Açıklık dağıt <br>`tool.openingArray` | Kaynak korundu, üç yeni açıklık oluştu. | Net aralık ile eksenden eksene aralığı küçük bir şema üzerinde gösterin. |
| Duvarı kaydır <br>`tool.wallShift` | Kenar −0,275 m kaydı, bağlı bir duvar izledi. Salon 33,80 m² göründü. | Değişecek bitişik mahal alanlarını onaydan önce okuyabilmeliyim. |
| Mahal numarala <br>`tool.spaceNumber` | Seçilen iki mahal numaralandı, etiket ve çizelge bağlarının korunduğu bildirildi. | Var olan numaralarla çakışanları listeleyin, boş numaraları atlama seçeneği sunun. |
| Tek duvara çevir <br>`tool.wallJoin` | Uç uca duvarları seçme ve Enter ile birleştirme akışı çalıştırıldı. İzin sonundaki durum geri almaydı. | Birleşmeyi engelleyen uç aralığını veya farklı stili ölçüyle açıklayın. |
| Duvarı uzat <br>`tool.wallExtend` | Uzatılan ucun yönü hedefi bulmadığında işlem reddedildi. | Uzatma ışınını ve neden kesişmediğini çizim üstünde gösterin. |
| Duvarı buda <br>`tool.wallTrim` | İki yol kesişmediğinde budanacak yer olmadığı açıklandı. | Kalacak ve kesilecek parçaları iki farklı çizgi tipiyle önizletin. |
| Duvarı çevir <br>`tool.wallFlip` | Duvar yönü çevrildi. | Dış yüzü, katman sırasını ve kapı elini işlem öncesi/sonrası karşılaştırın. |
| Stili değiştir <br>`tool.wallRestyle` | Stilsiz duvar seçilince “stilli bir duvara tıklayın” yanıtı geldi. | Bu noktada doğrudan ilk stili atama yolu sunun. |
| Açıklığı çevir <br>`tool.openingFlip` | Kapının açılış yanı değişti. İstemde Shift ile menteşe seçeneği gösterildi. | Fareyle kullanılabilir iki ayrı eylem ve küçük açılış diyagramı sunun. |
| Mahalleri birleştir <br>`arch.spaceMerge` | Önseçimsiz çalıştırmada birbirine değen en az iki mahal seçilmesi istendi. | Araç açıkken iki mahal toplatın, ortak kenarı ve korunacak kimliği gösterin. |
| Döşeme deseni <br>`tool.spaceFloor` | Mahale bağlı döşeme deseni yazıldı. | Desenin net alanı, yönü ve başlangıcını düzenlenebilir gösterin. |
| Tavan ızgarası <br>`tool.spaceCeiling` | Mahale bağlı tavan ızgarası yazıldı. | İki karşı duvardaki kesik plaka enini önizletin. |

### Belgele

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Planı ölçülendir <br>`tool.planDimensions` | 28 → 114 → 200 nesne. İkinci üretimi geri almak 114'e döndü. | Mevcut takımı güncelleme seçeneğini ilk sıraya alın. |
| Duvar ölçüleri <br>`tool.wallDimensions` | Açıklıklı duvarda sekiz ölçü yerleştirildi. | Çekirdek, kaba boşluk ve net geçiş referanslarını ayırın. |
| İç ölçü zinciri <br>`tool.interiorDimensions` | Bir kesen ve yerleşim noktasıyla yedi ölçü seçili kaldı. | Kesilen duvar veya kolon yüzlerini sıralı vurgulayın. |
| Etiket <br>`tool.tag` | Kaynağa bağlı etiket yazıldı. | Etiket ile kaynak arasında geçici bağlantı çizgisi gösterin. |
| Hepsini etiketle <br>`tool.tagAll` | Toplu etiketleme çalıştı ve yeni/yenilenen etiket bilgisi döndü. | Yinelenen çalıştırmada yeni, mevcut ve yer bulamayan etiketleri ayırın. |
| Doğrama kodları <br>`tool.openingCodes` | Kapı 90/210, kapı 100/210 ve pencere 120/130 için üç kod üretildi. | Kaynak değiştiğinde kullanıcı tarafından verilen kodları koruma politikasını gösterin. |
| Doğrama listesi <br>`tool.joineryList` | Kodlardan önce boş liste, kodlardan sonra üç tip ve on açıklık. | Kodsuz açıklıkları önizlemede sayın. |
| Metraj <br>`arch.quantities` | Metraj penceresi daire ve kesit binası üzerinde açıldı. | Satırdan kaynak nesnelere ve alan formülüne çift yönlü erişim sağlayın. |
| Metrekare cetveli <br>`tool.scheduleAreas` | Beş mahal içeren fakat bölümsüz çizimde sıfır satır yazıldı. | Kaynak kapsamını tabloyu koymadan açıklayın. |
| Emsal ve TAKS tablosu <br>`arch.zoningTable` | İmarsız dairede ayarlara yönlendirdi. Verili binada tablonun sol üst noktası istendi. | Eksik girdiler ve kaynağını tek formda gösterin. |
| Kat irtifakı tablosu <br>`tool.scheduleLandShares` | Bölümsüz çizimde sıfır satır ve eksik bağımsız bölüm uyarısı geldi. | Pay toplamını ve hesaplanamayan payları ayrı özetleyin. |
| Otopark hesabı <br>`arch.parkingTable` | Bölüm eksikliği bildirildi. Otopark binası örneğinde tıklanınca tablo yazıldı. | İhtiyaç, sağlanan ve fark satırlarından kaynak bölümlere gidilebilsin. |
| Sığınak hesabı <br>`arch.shelterTable` | Boş program uyarısı görüldü. Kesit binası üzerinde tablo sahneye yerleşti. | Varsayılan veya eksik kişi/oda bilgisini tabloda görünür tutun. |
| Kapı ve pencere çizelgesi <br>`tool.scheduleOpenings` | Kapı-pencere çizelgesinde on satır elde edildi. | Aynı numaralı iki doğramayı satırdan planda bulunabilir yapın. |
| Mahal çizelgesi <br>`tool.scheduleSpaces` | Beş mahal satırı yazıldı. | Net ve brüt kolonlarının hesap yüzünü başlıkta belirtin. |
| Mahal listesi <br>`tool.scheduleRoomList` | Mahal listesinde beş satır üretildi. | Mahal çizelgesi ile bu listenin sütun farkını iki küçük örnekte gösterin. |
| Donatı çizelgesi <br>`tool.scheduleFixtures` | Donatı çizelgesi üç satır üretti. | Hangi cihaz veya mobilya sınıflarının toplama girdiğini satıra bağlayın. |
| Bölüm çizelgesi <br>`tool.scheduleUnits` | Bağımsız bölümü olmayan planda sıfır satır yazıldı. | Boş mahal-bölüm ilişkilerini topluca tamamlama eylemi sunun. |
| Malzeme lejandı <br>`tool.materialLegend` | Stilsiz daire reddedildi. Malzemeli örnek binada beş malzemeli lejant oluştu. | Lejant satırından örnek duvar ve bileşene geçiş sağlayın. |
| Kesit <br>`tool.viewSection` | İki nokta ve Enter ile A kesit işareti üretildi. | Bakış yönünü kesilen eleman önizlemesiyle belirtin. |
| Kesit çıkar <br>`tool.sectionDraw` | İşaret seçilip yer verilince A-A kesiti üretildi. | Kaynak değişince paftada güncellik durumu gösterilsin. |
| Görünüş <br>`tool.viewElevation` | İşaret ve bakış yönü verildi, görünüş işareti 1 yazıldı. | Kuzey ve plan döndürmesi altında cephe adını kontrol edin. |
| Görünüş çıkar <br>`tool.facadeDraw` | İşaret üzerinden batı görünüşü çıkarıldı. | Açıklık ve saçak kotlarının kaynak elemana bağlantısını koruyun. |
| Detay <br>`tool.viewDetail` | Merkez, sınır ve Enter ile detay işareti 1 üretildi. | Hedefi olmayan işareti baskı kontrolünde tıklanabilir bulguya dönüştürün. |
| Kuzey oku <br>`tool.northArrow` | Plan kuzeyi ile ok yerleştirildi. | Gerçek kuzey ve plan kuzeyi farklıysa yön açısını okunur verin. |
| Etiketleri denetle <br>`arch.tagsCheck` | Numarasız mahaller adları ve katlarıyla listelendi. | Bulgu grubuna “bu mahalleri numarala” eylemi ekleyin. |
| Birleşimleri denetle <br>`arch.wallJoinCheck` | Sekiz birleşimin tamamının birleştiği yazıldı. | Satır veya nokta üzerinden birleşim stilini denetleme olanağı sunun. |
| Başvurular <br>`arch.viewRefs` | İşaret veya hedef görünüm bulunmadığı açıklandı. Tarif edilen şerit yolu eski. | Yolu güncel katalogdan getirin, hedef üretme düğmesini boş duruma koyun. |
| Oda iç görünüşü <br>`tool.roomElevation` | Salondan dört iç görünüş üretildi. | Her yanın plandaki bakış yönünü ve sıra numarasını eşleyin. |
| Revizyon işaretle <br>`tool.revisionMark` | Dikdörtgen iki köşeden revizyon 1 bulutu ve üçgeni yazıldı. | Üçgen ile pafta revizyon tarihinin bağını seçimde açıklayın. |
| Revizyon listesi <br>`arch.revisionList` | İşaretsiz çizimde gerekçe geldi. İşaret oluşturulunca liste tablo olarak kondu. | Listeyi sonradan değişen bulut ve üçgenlerle yenileme durumunu gösterin. |

### Proje

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Üst kat <br>`view.levels.up` | Tek katlı dairede üstünde kat olmadığı bildirildi. | Mesajdan kat listesini açtırın. Yeni kata geçerken etkin kat adı sürekli görünmeli. |
| Alt kat <br>`view.levels.down` | Zemin katın altında kat olmadığı bildirildi. | Alt kat ekleme ile mevcut kata gitmeyi görsel olarak ayırın. |
| Bütün katlar <br>`view.levels.all` | Bütün katlar görünümüne geçildi. | Görünen kat sayısını ve çizimin hangi kata yazılacağını birlikte gösterin. |
| Yalnız etkin kat <br>`view.levels.active` | İki katlı konut izinde yalnız 1. kat gösterildi, gizli nesnelerin seçilemeyeceği ve kenetlenmeyeceği açıklandı. | Bu açıklamayı kat durumu yardımında da koruyun. |
| Etkin kat ve altındaki <br>`view.levels.below` | Zemin katta altında kat olmadığı ve yalnız zeminin çizileceği bildirildi. | Bir üst katta altlıkla etkin katın çizgi yoğunluğunu ayrı ayarlatın. |
| Tadilat: Tümü <br>`arch.phase.all` | Bütün evreler görünümü çağrıldı. | Yeni çizimin atanacağı evreyi görünüm filtresinin yanında gösterin. |
| Tadilat: Mevcut durum <br>`arch.phase.existing` | Mevcut görünüm seçilince yeni çizilenlerin Mevcut olacağı bildirildi. | Yeni nesneye atanacak evreyi araç ayarlarına taşıyın. |
| Tadilat: Yıkım planı <br>`arch.phase.demolition` | Yıkım görünümünde yeni çizilenlerin Mevcut olacağı bildirildi. | Bir elemanı yıkıma işaretleme ve yıkım görünümünde çizme arasındaki farkı açıkça gösterin. |
| Tadilat: Yeni durum <br>`arch.phase.proposed` | Yeni durum görünümünde yeni elemanın İlave olacağı bildirildi. | Görünüm değiştikten sonra kapının ana duvarla evre uyumunu önizletin. |
| Tadilat: Tadilat projesi <br>`arch.phase.renovation` | Tadilat görünümünde de yeni nesnenin İlave olacağı bildirildi. | Seçili evreyi lejantla aynı görsel dilde, yazıyla da belirtin. |
| Tadilat lejantı <br>`tool.renovationLegend` | Mevcut düz, ilave taralı ve iptal kesikli gösterimleriyle lejant yerleşti. | Pafta reçetesi ve basılı siyah-beyaz lejantı eşleştirin. |
| Mimari denetim <br>`arch.audit` | Kat planından denetim açıldı, kural seti seçilemeyen ve eksik verili satırlar görünüyordu. | Aynı eksik girdiden türeyen bulguları gruplayın ve ilgili giriş alanını açtırın. |
| 3B görünüm <br>`scene3d.view` | Canlı 26 nesneli planda ayrı 3B görünüm açıldı. 11/11 gövde ve 1 gösterilemeyen bildirildi. Esc ile aynı 26 nesneli plana dönüldü. | “Gösterilemeyen” öğeyi seçince planda karşılığı ve eksiklik nedeni görülsün. |
| IFC modeli <br>`ifc.view` | Canlı dosya seçicisinden kesit-binasi-tam.ifc açıldı. IFC4X3_ADD2, iki kat, 68/68 gövde ve 1 gösterilemeyen görüldü. | Çizilen gövde ve temsil edilemeyen kayıt sayılarını aynı kapsam ve birimle açıklayın. |
| Mimari ayarları <br>`arch.settings` | Mimari ayarlarda plan kesme yüksekliği ve mahal sınır yöntemi görülüyor. | Bir ayarı değiştirmeden önce mevcut görünümlere etkisinin örneğini gösterin. |
| Duvar stilleri <br>`arch.wallStyles` | Stillerde tuğla, iç/dış sıva ve ısı yalıtımı, kalınlık ve birleşim öncelikleri görüldü. | Örnek L ve T köşesini 1:20 ile 1:100 arasında aynı pencerede karşılaştırın. |
| Ruhsat takımı <br>`arch.permitSet` | Ruhsat künyesi açıldı. Zorunlu alanlar ve pafta antetine aktarılacak bilgiler görüldü. | Eksik alanı ilgili pafta ve antet hücresiyle ilişkilendirin. |
| Bina programı <br>`arch.program` | Bölümsüz dairede program boştu, Bölüm ekle ile başlanacağı belirtildi. | Mevcut mahalleri bölümle ilişkilendirmek için plandan toplu seçimi görünür yapın. |
| Parsel ve bahçe mesafeleri <br>`tool.siteParcel` | Açık çizgi reddedildi. Ayrı denemede gerçek dikdörtgen seçilince 119,88 m² ve dört kenar önizlendi. | Kapalı görünen açık çizgiyi uç aralığıyla teşhis edip kapatma seçeneği sunun. |
| Kaçış yolları <br>`tool.escapeRoutes` | Kat çıkışı tanımsız örnekte yol üretilemeyeceği ve gereken kaynaklar açıklandı. | Eksik çıkışı planda tanımlama eylemi sunun. Engelsiz güzergâh analizini bu komuttan ayrı tutun. |
| Mimari şablonu <br>`arch.template` | Şablon önizlemesinde metre → metre, 1:100 → 1:100 karşılaştırması ve birimi koruma seçimi göründü. | Mevcut çizimin ne kadarının hangi ayardan etkileneceğini maddeler halinde gösterin. |
| Pafta seti <br>`arch.sheetSet` | Paftasız örnekte 0 pafta ve 0 seri bildirildi. Yeni pafta yolu metin olarak gösterildi. | Boş durumda doğrudan ilk kat paftasını oluşturma eylemi verin. |
| Model kalite kontrol formu (EK-9) <br>`arch.qualityForm` | Model kalite formunda sol üst köşe seçilerek tablo yerleştirildi. | Hesap sürerken, hesap bittiğinde ve sonuç güncelliğini yitirdiğinde ayrı durum gösterin. |
| Teslim kaydı <br>`arch.deliveries` | Teslim geçmişi boş açıldı. Paket ve iade akışı tarif edildi. | Henüz yayın yoksa ilk paketin nasıl hazırlanacağını tek eyleme bağlayın. |
| Altlık modeller <br>`arch.underlays` | Altlıklar penceresinde dosyasız durum ve IFC ekleme yolu göründü. | Eklemeden önce kat ve koordinat eşlemesini iki işaretle önizletin. |
| IFC revizyonlarını karşılaştır <br>`arch.ifcCompare` | İki IFC revizyonunu seçme penceresi açıldı. | Aynı model ve farklı koordinat başlangıcı durumlarını değişmiş geometri olarak raporlamadan önce açıklayın. |
| BCF sorunları <br>`arch.bcfIssues` | BCF başlık, tür, durum ve ilgili nesnelerin okunacağı pencere açıldı. | Çözülemeyen nesne kimliklerini karşı dosya ve bakış noktasıyla gösterin. |
| Gelen paketi incele <br>`arch.reviewPackage` | Paket klasörü veya IFC seçimi, idarenin koordinat sistemi ve İncele eylemi görüldü. | Yüklenmeyen paket ile yanlış koordinat sisteminin hata mesajları ayrı olmalı. |
| Dijital proje kuralları <br>`arch.digitalRules` | Kural, karar ve şema farkları penceresinde 12 açık karar bildirildi. | Belirsiz karardan etkilenen çıktıların durumunu teslim öncesi rapora bağlayın. |
| Gezgin <br>`arch.navigator` | Seçili nesne, adlı görünüm ve pafta bulunmadığı ayrı gösterildi. | Eksik görünüm veya paftayı buradan oluşturmayı sağlayın. |
| Kırpma çerçevesi <br>`tool.viewCrop` | Detay görünümünün 4 × 3 m çerçevesi ve sürükleme tutamaçları gösterildi. | Çerçeve değişirken paftada kaybolacak içeriği önizletin. |
| Kapı ve pencere tipleri <br>`arch.openingTypes` | Proje tipi olmayan dairede Yeni tip yolu göründü. Çizimde yerleşik Kapı 90 bulunuyordu. | Hazır tür ile proje tipini örnek bir açıklık üzerinden ayırt edin. |
| Eleman tipleri <br>`arch.elementTypes` | Kolon, kiriş, döşeme, çatı, korkuluk ve merdiven tipleri penceresi açıldı. | Bir tipi uygulamadan önce değişecek örnekler ve istisna alanları sayılsın. |
| Malzemeler <br>`arch.materials` | Projede tanımlı malzeme bulunmadığı ve yeni malzeme açılabileceği görüldü. | Duvar stili bileşenlerindeki metin adlarını proje malzemesine eşleme yolu sunun. |
| Oda modülleri <br>`arch.modules` | Oda nesnelerini seçme, tanımlama ve örnek yerleştirme adımları görüldü. | Modül taban noktasını koyarken duvar, kapı ve etiket bağlılığını önizletin. |
| Modül yerleştir <br>`tool.modulePlace` | Tanımsız projede yerleştirme denendi. Önce kaynak odayı tanımlama gereği açıklandı. | Buradan tanım yapıp aynı yerleştirme noktasına dönülebilmeli. |
| Tasarım seçenekleri <br>`arch.options` | Boş tasarım seçeneği kümesinin nasıl başlayacağı görüldü. | Seçeneğe alınan oda ile ana modeldeki kapı/duvar bağının etkisini önizletin. |

### Bağlamsal araçlar ve diğer mimari komutlar

| Araç | Deneme ve gözlenen sonuç | İyileştirme veya kabul kontrolü |
|---|---|---|
| Mahal kaplama çizelgesi <br>`tool.scheduleFinishes` | Mahal kaplama çizelgesi ayrı komutla yerleştirildi, beş satır yazıldı. | Mahal çizelgesinden yüzeylerin boş veya atanmış olduğunu karşılaştırın. |
| Yerel istisna yap <br>`arch.moduleExcept` | Modüle ait olmayan nesne seçilince yerel istisna uygulanmadı. | Kaynak örneği seçme eylemini gerekçenin yanına koyun. |
| Kaynağa döndür <br>`arch.moduleRestore` | Aynı nesnede kaynak modül olmadığı için dönüş uygulanmadı. | Dönecek özellikleri yalın bir fark listesinde gösterin. |
| Örneğe ekle <br>`arch.moduleAddMembers` | Sıradan bir nesne seçiliyken örneğin bir nesnesinin de seçilmesi gerektiği açıklandı. | İki seçim rolüne ayrı ad ve vurgu verin. |
| Modülden çıkar <br>`arch.moduleRemoveMembers` | Örneğe ait olmayan nesne için çıkarma reddedildi. | Diğer örneklerden kalkacak elemanları onaydan önce sayın. |
| Aynı tiptekileri seç <br>`arch.openingSameType` | Kapı 90 örneğinden etkin kattaki beş açıklık seçildi. | Süzgeç kapsamına kat ve tür kaynağını yazın. |
| Tipten ayır <br>`arch.openingDetach` | Seçili açıklık bir proje tipini izlemediği için reddedildi. | Hazır türün parametrelerini proje tipi olarak kaydetmeye yönlendirin. |
| Tipe uygula <br>`arch.openingPush` | Aynı açıklıktaki değerlerin proje tipine yazılması önkoşulda durdu. | Yazılacak alanlar ve etkilenecek açıklık sayısı önizlensin. |
| Tipe döndür <br>`arch.openingReset` | Proje tipine bağlı olmayan kapının sıfırlanması reddedildi. | Yerel istisna ve tipten alınan değerler birbirinden okunabilsin. |
| Tipe çevir <br>`arch.elementTypeFrom` | Önseçim yokken desteklenen eleman sınıfları tek tek bildirildi. | Komut açıkken plandan geçerli elemanı seçmeye izin verin. |
| Aynı tiptekileri seç <br>`arch.elementSameType` | Önce tipi izleyen elemanın seçilmesi istendi. | Örneğe uygulanmış bir proje tipi yoksa ilk uygun adayları vurgulayın. |
