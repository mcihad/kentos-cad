# CAD dosya değişimi: karar, kapsam ve koruma kuralları

22 Ekim 2026. CAD-IO-01/02/03/06/08/16/18/19 için kaynak ve kabul kaydı. Uygulanan sağlayıcı seçimi [ADR 0586](../adr/0586-dwg-opencadcodec-git.md). “Sağlayıcı destekliyor”, KentOS'ta denenmiş veya kayıpsız demek değildir. Bu belge sağlayıcı lisansının yerine geçmez.

## Sağlayıcı karar matrisi

| Yol | Windows | macOS | Linux | ARM64 | Web | Rust bağlantısı | Headless/offline | Dağıtım ve maliyet | Bakım / karar |
|---|---|---|---|---|---|---|---|---|---|
| OpenCADCodec Git | Rust hedefi | Rust hedefi; Apple Silicon testli | Rust hedefi | hedefe göre denetlenir | upstream clock JS ister; KentOS web adaptörü ertelendi | saf Rust; FFI yok | yerel, ağsız | MPL-2.0; SDK ücreti yok; lisans bildirimleri korunur | kaynak revizyonu ve lock sabit; yedi DWG hedefi korpusu; seçilen yol |
| Autodesk RealDWG 2027 | Windows 11 ilanı | bu sürüm sayfasında ilan edilmemiş | ilan edilmemiş | ilan edilmemiş | doğrudan ilan edilmemiş | C++/.NET için C ABI köprüsü veya ayrı süreç gerekir | SDK uygulaması; sunucu/dağıtım koşulları sözleşmeyle | Tech Soft 3D lisansı/evaluasyonu; fiyat teklifi gerekir | platform ve kapalı SDK bakım maliyeti; bu çalışma alanına kurulmaz |
| ODA Drawings | x86/x64 tabloda | x64/arm64 tabloda | x86/x64/ARM64 tabloda | platform tablosuna bağlı | ayrı inWeb ürünü | native köprü veya ayrı süreç | yerel SDK; SaaS üyelik kademesine bağlı | Commercial ilk yıl 3.000 USD, yenileme 2.250; 100 koltuk sınırı. Sustaining ilk yıl 7.500, yenileme 4.500; sınırsız dağıtım ve web/SaaS. Fiyatlar kontrol tarihindeki liste | aylık bakım; üyelik kesilince dağıtım hakkının sürdüğü varsayılmaz; yedek seçenek |
| GNU LibreDWG | derleme/CLI denenmeli | derleme/CLI denenmeli | C/CLI yolu | toolchain'e bağlı | KentOS entegrasyonu yok | C FFI veya izole CLI | yerel ve offline | GPL-3.0-or-later; SDK satın alma ücreti yok, ürün lisans uyumu ayrıca değerlendirilir | bağımsız karşılaştırma adayı; KentOS runtime bağımlılığına eklenmez |
| Yetkili uzak dönüşüm | istemci bağımsız | istemci bağımsız | istemci bağımsız | istemci bağımsız | servis arayüzü | sürümlü HTTP/iş sözleşmesi | headless olabilir; offline değil | sağlayıcı/iş başına maliyet, aktarım ve saklama sözleşmesi | bu projede etkin değil; dosya/servis/hedef/izin kullanıcı tarafından açıkça seçilmeden çağrılmaz |

RealDWG'nin C++/.NET arayüzü ve güncel Windows gereksinimi [Autodesk'in ürün sayfasından](https://aps.autodesk.com/developer/overview/realdwg-api), ODA platformları [resmî platform tablosundan](https://www.opendesign.com/supported-platforms/drawings), ODA kademeleri/fiyatları [resmî fiyat sayfasından](https://www.opendesign.com/pricing?language=en) doğrulandı. LibreDWG lisansı ve C kütüphanesi niteliği [GNU ürün sayfasında](https://www.gnu.org/software/libredwg/) belgelenir. Codec için pazarlama tablosu yerine [kilitlenen kaynağın Cargo.toml'u](https://github.com/HakanSeven12/opencadcodec/blob/ff75fddc322bda4cf817c5f68535c5c2b316b9c4/Cargo.toml), kaynak kodu ve KentOS testleri esas alınır. Tabloya yazılmayan sağlayıcı platformu destekli kabul edilmez; işletim sistemi desteğinden CPU mimarisi desteği çıkarılmaz.

## Biçim ve özellik kabul matrisi

| İçerik | KentOS okuma | KentOS yazma | Semantik/kayıp koşulu |
|---|---|---|---|
| Model-space temel 2B | LINE, POINT, CIRCLE, ARC, ELLIPSE, POLYLINE/LWPOLYLINE, SPLINE | yerel CAD entity'leri | Z/OCS/projeksiyon, çizgi genişliği ve dönüşümler ayrı sayısal korpusla sınanır |
| Çoklu çizgi bütünlüğü | düzlemsel genişlik/yay/kot, ±Z extrusion; X/Y ölçeği korunmuş INSERT | standart planar LWPOLYLINE veya 3B düz POLYLINE | 2B modelin taşıyamadığı dönüşüm varsayılan polyline_geometry_loss; yalnız açık allowPolylineLoss ile raporlu yaklaşık sonuç; genel eğik/afin yerel düzenleme henüz yok (ADR 0588) |
| Katman | LAYER, açık/kapalı, donmuş, kilit, plot, renk/kalınlık | LAYER | ad çakışması ve grup düzleştirmesi raporlanır |
| LTYPE/LIN | basit örüntü, karmaşık yazı/shape tanımı | LTYPE | dosyası bulunmayan SHX görünümü eşdeğer kabul edilmez |
| STYLE | font adı, yükseklik, genişlik/hiza ve mevcut model alanları | STYLE | bulunmayan yüz için kullanılan alternatif raporlanır |
| DIMSTYLE + override | mevcut ölçü modeli; dosyadaki yerel görünüş | DIMENSION + blok | desteklenmeyen ok/font/değişken raporlanır; eşit piksel görünümü garanti edilmez |
| MLEADERSTYLE | desteklenen kılavuz görünüşü ve özellikleri | LEADER/MTEXT; bazı kılavuzlar blok | tam MLEADERSTYLE düzenleme semantiği yok; dönüşüm kaybı belirtilir |
| HATCH/PAT | örüntü, ada/sınır, gradient ve mevcut ilişkiler | HATCH/PAT | desteklenmeyen 3B yüzey/ilişki yeniden kurulmuş sayılmaz |
| TABLE | KentOS tablo metadata'sı ve görünüş | blok görünüşü + KentOS metadata | foreign ACAD_TABLE tam yerel tablo düzenlemesi olarak iddia edilmez |
| Nesne override | ACI/truecolor, ByLayer/ByBlock çözümleme, kalınlık, line type/scale/transparency | modeldeki alanlar ve KentOS metadata | blok içinde çözülmüş görünüşle kaynak kalıtımı ayrılır |
| Annotation | desteklenen ölçü/yazı/kılavuz; kendi model alanları | yerel entity veya görünüş bloğu | çoklu annotative scale ve gelişmiş ilişki kaybı raporda |
| Xref | BLOCK path/flags + INSERT transform; yükleme yalnız açık seçimle | BLOCK path/attach/overlay + INSERT | referans dosya otomatik çalıştırılmaz veya ağdan alınmaz; bulunmayan kaynak açıkça unloaded |
| Layout/viewport/paper-space | gerçek KentOS pafta kitabı ve görünüm öğesi | LAYOUT/VIEWPORT/paper-space | desteklenen 2B ortografik görünüm ve pafta öğeleri; perspektif/3B clipping ayrı kayıp |
| Dynamic block | görünüş ve desteklenen KentOS parametreleri | çözümlenmiş görünüş / plain block | foreign AutoCAD evaluation graph düzenlenebilir sayılmaz |
| Constraint | çözümlenmiş geometri | çözümlenmiş geometri | foreign constraint ağının düzenlenebilirliği ve export ilişkisi garanti edilmez |
| AEC/proxy/özel sınıf | tanınan görünüş veya raporlanan ret | görünüşe dönüşüm ya da ret | kaynak sınıf korunmadan “kayıpsız” etiketi verilmez |
| Handle/XDATA/dictionary | kaynağa ait ilişki kimliği ve seçilmiş opaque veri | yalnız doğrulanan referans yeniden eşlemesi | ham handle başka dosyanın graph'ına aynen yazılmaz |

Bu tablo tam yetenek iddiası değildir; satırların uygulanmış davranışı, kayıp/ret ve testleri `crates/shared/formats/tests/all/cad_io.rs`, DWG adaptör korpusu, native ortak durumlar ve dosya entegrasyonu ile birlikte değerlendirilir. Komut kataloğundaki makinece okunabilir format kapsamı aynı ayrımları Python/AI'a sunar.

## Desteklenmeyen nesne ve opaque veri politikası

1. **Kaynağı sakla:** özgün dosya değişmez; SHA-256 ve biçim/sürüm makbuzu kaydedilir. Tanınmayan nesne kaynakta kalır; içe aktarma raporu alınmadığını söyler. Kaynak tutulması, hedef dosyada yeniden yazılabilme garantisi değildir.
2. **Görünüşü kullan:** yalnız gerçekten mevcut ve güvenle çözülebilen grafik temsil kullanılır. Özgün AEC/proxy tipi olarak düzenleme açılmaz. Grafik yoksa kullanıcıya boş bir “başarılı nesne” verilmez.
3. **Düzenlemeyi engelle:** opaque kaydın binary grafiği, işletilebilir payload'ı, URL'si veya uygulama sınıfı otomatik yüklenmez. Metaveri sınırlı boyutta veri olarak tutulur; komut/script çalıştırılmaz.
4. **Basic entity'ye dönüştür:** desteklenen çizgi/yay/yazı/blok görünüşü açık kayıp raporuyla alınır; orijinal parametre/ağ ilişkisi korunmuş sayılmaz.
5. Handle yalnız kaynak dosyanın kimlik uzayındadır. Hedef yeni handle üretir; owner/reactor/soft-hard pointer geçişleri whitelist ve yeniden eşleme gerektirir. Eşlenemeyen pointer ham olarak hedefe taşınmaz.
6. XDATA app-id adı ve group-code tipleri, 1004 binary boyutu, dictionary derinliği ve toplam opaque baytları doğrulanır. İzinli olmayan sınıf/proxy/ACIS payload'ı domain nesnesi gibi yorumlanmaz. Toplam boyut, nesting, örnek sayısı ve kayıp raporu kotaları belirlenir; limit aşımı açık uyarı/ret verir.
7. Düzenleme sonrasında kaynağın eski geometrisini geri getirebilecek opaque veri otomatik olarak yeni geometriye uygulanmaz. Kaynak hash'i, nesne kimliği ve geometri sürümü tutmuyorsa opaque round-trip kapatılır.
8. Metadata kaybı olan hedef için “kayıpsız kaydet” seçeneği gösterilmez. CAD-IO-18 bu stratejinin kabulüdür; her bilinmeyen üretici formatının uygulaması veya lisans kabulü değildir.

## ASCII, binary, sürüm ve kodlama

DXF okuma: AC1006, AC1009, AC1012, AC1014, AC1015, AC1018, AC1021, AC1024, AC1027, AC1032 ASCII. Başlık/sürüm yoksa açıkça belirtilir; dosya yapısı yine SECTION/ENDSEC/EOF ile tam olmak zorundadır. Bilinmeyen sürüm `unsupported_cad_version`, binary/UTF-16 veya eski sürümde desteklenmeyen codepage `unsupported_cad_encoding`, yanlış DWG seçimi `unsupported_cad_format` verir. Eski ASCII dosyalarda ANSI_1252 ve ANSI_1254, AC1021 ve sonrasında UTF-8 yolu vardır. UTF-8 BOM desteklenir. Okuma-yazma bozuk harfleri sessizce başarı saymaz.

DXF yazma: ASCII AC1021, UTF-8; hedef DWG bunun üzerinden açıkça seçilen yedi binary imzaya yazılır. Binary DXF yazma/okuma bu sürümde ilan edilmez. DWG okuma/yazma AC1014, AC1015, AC1018, AC1021, AC1024, AC1027, AC1032; R14 birim/çizgi kalınlığı sınırı raporda. Ürün yılı yerine dosya imzası kontrol edilir.

## Platform ve kanıt kayıtları

- Gerçek çalıştırma: mevcut macOS ARM64; native okuyucu/yazıcı, komut/undo, Python 3.14 ve MCP. Sayısal testler mevcut ASCII bağımsız başvurularını kullanır. Aynı codec ile üretilen DWG, bağımsız DWG uyumluluk kanıtı olarak adlandırılmaz.
- Diğer Rust hedefleri için derleme kontrolü ve CI işi, runtime denemesi yerine geçmez. İlan edilen sağlayıcı platformu ile KentOS'ta çalıştırılan platform ayrı tutulur.
- UI klavye/fare, açık/koyu tema ve yazı ölçeği kendi kabul kaydıyla doğrulanır. Arayüz eklenmesi tek başına kabul değildir.
- CAD-IO-20, 22 Ekim'de kullanıcının açık isteğiyle kapsam dışında bırakıldı; AutoCAD kontrolü yapılmış sayılmaz.
