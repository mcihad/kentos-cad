# 0913 — Gerçek ölçü, kaynak ilişkileri ve anotasyon yerleştirme

Tarih: 4 Kasım 2026. Kapsam: `plan/cad-advanced.md`, CAD-ADV-09.

## Dayanak ve ürün kararı

- [AutoCAD 2026: Associative Dimensions](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-DidYouKnow/files/GUID-7C7B6F58-2E17-4A77-9579-D42E205800BF.htm): tam, kısmi ve serbest ilişki ayrılır; kopuk ilişkiyi göstermek ve doğru kaynağa yeniden bağlamak gerekir.
- [AutoCAD 2026 Mac: About Associative Dimensions](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-3559D5D2-AD15-4167-9BD9-EE5FCEB4F8CE.htm): anotasyon monitörü kaybolmuş ilişkiyi bildirir; işaret ölçünün doğru olduğunu kanıtlamaz.
- [BricsCAD V26: Editing dimensions](https://helpcenter.bricsys.com/en-us/document/bricscad/annotate/dimensions/editing-dimensions?version=V26): `<>` ölçülen değeri tutar, sabit yazı onun yerine geçer; yazı hareketi ölçü geometrisinden ayrı değerlendirilir.
- [BricsCAD V26: Creating linear dimensions](https://helpcenter.bricsys.com/en-us/document/bricscad/annotate/dimensions/creating-linear-dimensions?version=V26): zincir ölçü önceki uçtan, baz ölçü ortak başlangıçtan devam eder; bazların aralığı ayrı yerleşim davranışıdır.
- [Vectorworks 2026: Unconstrained linear dimensioning](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Dimensions/Unconstrained_linear_dimensioning.htm): zincir ve baz ayrılır; yazı çakışması ayrıca ele alınır.

KentOS'ta geometri, ölçülen değer, ekranda yazılan değer ve kaynak bağı ayrı gerçeklerdir. Uyarıyı kapatmak veya bir sayıyı elle yazmak bunları eşitlemez. Mevcut ölçü, stil, anotatif temsil, zincir/baz ve yeniden bağlama motorları korunur. Mimari ölçü tariflerinin ikinci bir kopyası oluşturulmaz.

## Uygulama

1. `cad.drawing.inspect` üçüncü denetim grubu `annotations` alır. Sonucun `dimensions` alanı doğrudan `cad.dimensions.query` okuyucusunun `measured`, `shown`, `overridden`, `bond` verileridir. Kısmi, kopuk ve serbest ilişki; sabit yazı ve sonlu olmayan değer ayrı bulgulardır. `<>` içeren metin otomatik olarak override sayılmaz. Açısal açıklama proje açı biçimini kullanır.
2. Kılavuzun ilk kolu ve bütün ek kolları denetlenir. Kaybolan UUID bulguda korunur; onun yerine sessizce yeni nesne bulunmaz. Bulgudaki ilk UUID anotasyon, sonraki benzersiz UUID'ler kaynaklardır. Göster yalnız anotasyonu seçer; Kaynağa git yaşayan kaynaklara gider. Kısmi ölçünün işareti serbest uçta, kopuk ölçü/kılavuzunki sorunlu uçtadır.
3. Masaüstündeki CAD denetimi KentOS UI'ın mevcut seçim, sayı, tablo ve onay bileşenleriyle çalışır. Kapsam, seçili nesneler, durdurma, eski sonuç uyarısı ve kaynak onarımı korunur. Yeniden bağla seçilmiş ölçüyü doğrudan alır; Enter mevcut ucu korur, Esc yazmadan çıkar. Kılavuz ucunu bağla önce mevcut kolu, sonra yeni kenetlenen kaynağı ister; boşlukta tıklama bağ oluşturmaz.
4. `cad.leaders.edit` bağ doğrulaması artık tam blok parça yolunu okur. Kılavuz ucu iç içe blok taşındığında ve tanım değiştiğinde ortak `follow_path` kuralıyla izler. Parça kalmadığında uç son yerinde kalır ve bağı kopuk olur. İşlem ve türeyen değişiklikler aynı geri alma adımındadır.
5. Birden fazla nesnenin aynı koordinatı paylaşması bağ kurmayı engellememelidir. Normal kenet/Tab sırası değişmez. `cad.snap.find(all=true, sources=true)` aynı noktadaki farklı kaynak UUID'lerini de döndürür. Kılavuz aracı kendisini ve kalıcı alt nesne referansı veremeyen adayları atlayarak geçerli kaynak arar. Python/MCP aynı sorguyu kullanabilir.
6. Ölçü yerleştirirken mevcut gerçek konumlu yazı önizlemesi korunur; zincir/baz ölçüsüne de eklenir. Ortak referans ve baz aralığı mevcut komutun hesabıdır. Yakındaki ölçü/yazı kutularıyla olası çakışma kırmızı kutu ve açıklamayla görünür; uyarı ölçüyü taşımaz, değeri değiştirmez ve onayı engellemez.
7. Öznitelikler'deki hesaplanan sayı açıkça **Gerçek ölçü**, sabit yazının durumu **Elle değiştirilmiş** adını alır. Pafta ön denetimi gerçek/gösterilen farkını ve kısmi ilişkiyi bildirir; kopuk ilişki taraması kılavuzun ek `arms` alanını da kapsar. Tanı işaretleri PDF'ye basılmaz.
8. `drawing.measure` ve kutuyla nesne sorgusu blokların yalnız ekleme noktasını ölçmez. `blocks::placed_box_in` ortak render genişletmesini kullanır: etkin parametreler, iç içe dönüşümler, aynalama, farklı X/Y ölçeği, kırpma ve parça katman kapıları. Sonuç dünya eksenlerine göre kutudur; GUI'de **Kutu eni (Y)** ve **Kutu boyu (X)** aynı yardımcıdan gelir. Bloğun yerel eksenlerindeki **En × boy** bilgisi de aynı görünür parça ve kırpma kuralını okur; dönüş bu ikinci okumada kaldırılır. Seçim vurgusu artık gizli katmandaki blok parçalarını ortaya çıkarmaz. Bir toplu API sorgusunda bloklar bir kez hazırlanır; her nesne için çizimin bütün deposu kurulmaz. Kaydedilmiş koordinatlar metre olduğu için blok birimi ikinci kez çarpılmaz.

## Sınırlar

- Önizlemenin çakışma denetimi uzamsal indeksteki en çok 128 yakın nesneyi inceler. Döndürülmüş yazılarda eksenlere paralel kutu muhafazakârdır; bu nedenle “çakışabilir” der. Çizimin bütün anotasyonlarını kapsayan yerleşim optimizasyonu veya standart uygunluk doğrulayıcısı değildir. Blok içindeki yazılar ve kılavuz metinleri bu yerel ipucunun kapsamı dışındadır.
- Kaynak denetimi mevcut domain motorunun sakladığı kopma nedenlerini ve kaynak nesnenin varlığını okur; her denetimde bütün alt nesne yollarını baştan genişletmez.
- Görünen blok kutusu çizilen geometri içindir. Hiç çizilen parçası olmayan/tanımsız blok API'de ekleme noktasına düşer; bu durum çizilmiş içerik varmış gibi alan veya uzunluk üretmez. Seçim tutamacının/zoom kapsamının mevcut ekleme noktası davranışı değiştirilmez.
- Birincil/alternatif birim, yuvarlama, limit ve çift taraflı tolerans, çap/yarıçap ve teknik çerçeve özellikleri mevcut ortak biçimleme/stil motorunda kalır. Bu çalışma ISO/ASME sürüm sertifikasyonu iddia etmez. Seçilen standardın kuralları ayrıca doğrulanmadan yalnız sembol görünüşü uygunluk kanıtı değildir.

## Doğrulama kaydı

Otomatik ve gerçek arayüz kabulünün son durumu `plan/cad-advanced.md` altına, yalnız çalıştırılan kontrollerle yazılır. Ham ekranlar, kabul çizimi ve iki ölçekli PDF `/tmp/kentos-cad-adv08-09-2026-11-03/` altındadır; Git'e alınmaz.
