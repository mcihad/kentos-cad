# 0613 — Pafta önizlemesi ve ortak native çıktı

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-23.

## Karar

Masaüstünün gerçek Kaydet/Yazdır yolu, pencere önbelleğine bağlı ikinci PDF
orkestrasyonu yerine `kentos_plot::book::book_pdf_options` kullanır. Aynı hazırlık,
`file.export.pdf` ile CLI, Python ve MCP çıktısının da kaynağıdır. Seçili paftalar,
GeoPDF/katman seçenekleri ve kullanıcı çözünürlüğü açık girdilerdir.

Pafta düzenleyicisinin kitabı ve gömülü resimleri çizimin salt okunur kopyasına
birlikte alınır. Yalnız çıktı kopyasındaki dpi değişir; çizim, açık pafta ve undo
geçmişi değişmez. Native hazırlık bütün kaynak indeksini yeniden kurar; aktif
viewport'un seçim indeksini ya da ekranın görünür katman önbelleğini kullanmaz.

`kentos_plot::preview::page` aynı Export display list, stil kitaplığı, annotation
ölçeği, katman override'ı, yakalanmış baskı profili ve fiziksel kamera ile SVG/PNG
üretir. PDF ve SVG aynı `print_setup::plan_for` sonucunu kullanır: beyaz kâğıttan
sonra çıktı alanı kırpması, kaynak dönüşümü ve kaynak alanı kırpması. Yazılar aynı
paketli fontlardan okunur; taşınabilir SVG kullanılan ailelerin fontlarını taşır.
PNG/SVG'nin gerçek masaüstü kayıt yolu da bu fonksiyondur. Gömülü harita ve
resimlerin çizilmesi için resvg'nin raster-images desteği açıktır; mevcut lockfile
bağımlılıkları kullanılır. Üretilen SVG, usvg 0.45'in okuduğu SVG 1.1 xlink resim
başvurularını taşır; resim atlama başarılı çıktı sayılmaz. Hatalı hesap veya
okunamayan kaynak başarılı gri harita kutusuna çevrilmez.

PDF penceresindeki Baskı önizlemesi ilk seçili paftayı uygulama içinde 96 dpi'de
gösterir. Bu ekran fiziksel çıktı boyunun ekran cetveli değildir. Çıktı
seçeneklerine dönerek seçim yapılır; bütün seçili paftalar PDF'e gider. Tam sayfa
resim 16384 piksel/120 MP sınırını aşarsa çözünürlük düşürme önerisiyle reddedilir.
Model harita resmi ayrıca mevcut 8192 piksel/32 MP sınırını kullanır.

Iced 0.14, 2 MB üzerindeki RGBA dokuları asenkron yükler. İlk GPU karesinin boş
kalmaması için yakalanan önizleme 512×512 piksel dokulara ayrılır; harita yeniden
hesaplanmaz, orijinal çıktı pikselleri korunur. Her parça 1 MB'tır ve aynı karede
yüklenir. Test hem yazılım hem GPU çizicide tam A4 resmini ve modal yerleşimini
kontrol eder; yalnız 2×2 bir resmin görünmesi yeterli değildir.

## Doğrulama

20 bağımsız page-setup senaryosunda PDF, Poppler ile 96 dpi'ye çizilir; önizlemenin
renkli çizgileri karşılıklı bir piksel konum toleransıyla karşılaştırılır.
İnce çizginin kesilen yarısındaki rasterizer yumuşatma farkları için düşük kromalı
kenar örnekleri de sayılır. Her yönde çizginin en az %90'ı eşleşmeli; tek bir çizginin
kaybolması testin hatasıdır. Dönüşüm ve kaynak snapshot'ının değişmemesi de sınanır.
Gerçek masaüstü kaydı ve yazdırma dosyası ortak native PDF ile bayt bayt karşılaştırılır.
Tema ve büyük yazı görüntüleri ayrıca açılarak incelenir.

Fiziksel yazıcıya gönderme ev sahibinin işi olarak kalır. Web uygulaması ADR 0228
uyarınca ertelidir; ortak SVG çekirdeği WASM ile derlenmeye devam eder.

Kabul: native plot 12+35, mevcut UI 46+44+7 ve yeni iki resim/yerleşim testi
GPU'da geçti. Gerçek düğme tıklamasıyla üç ekran ve Kaydet/Yazdır karşılaştırması
geçti; ekranlar açıldı. 240 Python testi güncel native modülle geçti. Clippy,
SDK drift, 47 crate/61 hedef bağımlılık denetimi ve ortak pafta WASM smoke geçti.
