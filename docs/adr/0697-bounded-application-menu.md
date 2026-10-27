# ADR 0697: Büyük yazıda pencereye sığan uygulama menüsü

- Tarih: 27 Ekim 2026.
- Durum: dar GUI ve desktop/UI kontrolleri geçti; son ortak test kapıları beklenir, nihai özellik kabulü verilmedi.
- Kapsam: KentOS düğmesinden açılan masaüstü uygulama menüsü.

## Gerçek kusur ve neden

18px yazı ile dar native pencerede dokuz komutun toplam sabit yüksekliği
başlık ve alt eylemlere yer bırakmıyordu. Proje ayarları ve Uygulama
ayarları pencere dışında kalıyordu. Sol komut listesi kaydırılabilir
olmadığından tekerlek kullanmak son komutlara eriştirmiyordu. Tanısal ham
kanıt `.run/gui-evidence/2026-10-27-wall-dimensions/98-live2-large-menu-diagnostic.png`
altında tutulur; görüntü depoya eklenmez.

## Karar ve kaynaklar

[Apple Scroll views](https://developer.apple.com/design/human-interface-guidelines/scroll-views)
görünümden uzun içerik için alışılmış kaydırmayı, kaydırılabilir içeriğin
anlaşılmasını ve yeni klavye seçiminin gerektiği kadar görünür hale
getirilmesini önerir. Aynı yönde iç içe kaydırma alanlarından kaçınılır.
Menünün sol ve sağ bölmeleri bu nedenle birbirinin içine konmaz.

[Microsoft Windows app best practices](https://learn.microsoft.com/en-us/windows/apps/get-started/best-practices)
küçük pencere ve farklı yazı/ekran ölçeklerinde denemeyi; pencere küçülünce
dışarıda kalan denetimlere fare, klavye ve diğer giriş yollarıyla erişim
sağlayan kaydırmayı önerir. [Responsive design](https://learn.microsoft.com/en-us/windows/apps/design/layout/responsive-design)
mevcut düzenin pencere boyutuna uyarlanmasını açıklar.

Bu rehberlerin KentOS'a uygulanması bir ürün kararıdır: mevcut menü
sırası, yazı boyutu, doğal geniş/yüksek görünüm ve hover ile sağ bölme
değişimi korunur. Yeni kompakt menü tasarımı veya yeni tercih eklenmez.
Mevcut Dialog bileşeninin kullandığı `Fit`, başlığı ve alt eylemleri önce
ölçer; yalnız gövdeye pencerenin şerit altında kalan yüksekliğini verir.
Sol komutlar bağımsız kayar; sağ bölme mevcut kaydırmasını korur. Ok
tuşlarıyla seçilen sol komut, yalnız gerekiyorsa en yakın görünür konuma
kaydırılır. Normal geniş/13px menü kendiliğinden pencereyi doldurmaz.

## Kabul kapıları

- Gerçek widget düzeninde 1100×650 mantıksal pencere, 16px ve 18px:
  alt üç eylem pencere içinde; soldaki gerçek tekerlek olayı Proje
  ayarları satırını gösterir; satır ve Uygulama ayarları gerçekten açılır.
- 1440×900 / 13px: doğal menü yüksekliği ve son satır korunur;
  kaydırma gerekmiyorsa liste uzatılmaz. 18px dar düzende ok tuşlarının
  son/ilk satıra çevrimi seçimi görünür tutar.
- Menü gezinmesi ve ayarlara erişim çizim modelini değiştirmez.
- Native yeni derlemede aynı 18px dar pencere kusuru ve geniş/13px görünüm
  fare/klavye ve ham görüntülerle ayrıca doğrulanır. Otomatik widget testi
  native GUI kabulünün yerine geçmez; ekran okuyucu veya bütün tema/pencere/
  yazı bileşimleri bu dar düzeltmeyle kabul edilmiş sayılmaz.

## Sonuç ve kalan kapılar

[QA kaydı](../baseline/2026-10-27-architecture-wall-dimensions/README.md)
98 taşma tanısını son Live-3 tekrarıyla ayırır. 111 geniş/13px doğal menüyü;
118–124 açık tema/18px, 2202×1364 ham (yaklaşık 1100×682 mantıksal) pencerede
gerçek tekerlekle Proje ayarları satırına erişip tıklamayı, ok tuşlarıyla
son/ilk satıra çevrimi ve Enter'ı, alt Uygulama ayarları düğmesine erişimi
belgeler. 125 tam belgeyi özgün 1:100 dosyasıyla eşit bulur; gezinme modele
yazmamıştır. Geniş/18px ara gözlem tam görünüm matrisi kabulü değildir.

Son varsayılan paralel desktop turunda **1341 geçti, 0 başarısız, 228 ignored**;
UI turunda **289 geçti, 0 başarısız**. Üç uygulama derlemesi geçti; yeni gerçek
GUI Live-3 üzerinde yapıldı. İlk paralel başarısızlık, tanısal tek sıra turu
ve testlere özel yazı kilidi düzeltmesi [ADR0698](0698-scrollable-wall-style-dialog.md)
içinde korunur; eşikler gevşetilmedi. Son Python/API, clippy ve WASM kapıları
henüz açık olduğundan genel özellik/yayın kabulü bekler. Bütün tema/yazı/
pencere bileşimleri ve ekran okuyucu erişilebilirliği tamamlandı sayılmaz.
