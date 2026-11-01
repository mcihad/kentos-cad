# 0825 — Elips yayı uzatmayı bütün native seçim yollarına bağla

Tarih: 2026-11-01. Durum: uygulandı; CAD-ADV-04 birleşik kabulü açık.

## Bulgu ve karar

Geometri çekirdeği `extend_ellipse` ile elips yayını aynı elips üzerinde
ilk sınıra uzatabiliyordu. Native `cad.entities.extend`, uçların tek
komutta bir kez işlenmesini denetlerken yalnız `path_of` kabul ediyor,
elipsi daha çekirdeğe ulaşmadan reddediyordu. Yetenek kataloğu ise zaten
elips desteği bildiriyordu. Yeni ayrı bir eğri hesabı yerine native
uç denetimi mevcut analitik elips hesabına bağlandı.

`extend_ends` açık yol/yay ve elips yayının gerçek uçlarını verir.
Tıklama, çit ve pencere aynı yardımcıyı kullanır. Tam elips ve kapalı
hedefin uzatılacak ucu yoktur; ret korunur. Değişen yalnız elipsin
parametre aralığıdır: merkez, ana eksen, oran, UID ve verisi kalır.
Düz sınırda analitik kesişim kullanılır; eğrisel sınır kesişimlerinin
mevcut örnekleme/iyileştirme sınırları geniş bir kesinlik iddiasıyla gizlenmez.

İkinci bulgu: native kod uç kaydını yol boyunca yarı uzunluğa göre,
çekirdek ise gerçek uçların tıklamaya mesafesine göre seçiyordu.
Katlanmış bir yolda farklı ucu kaydetmek sonraki geçerli seçimi
atlatabiliyordu. Native kayıt çekirdekle aynı mesafe/tie kuralını izler.

Komut, şema, Python türleri veya MCP araç adları değişmez. Önizleme
aynı salt okunur plan, yazma aynı komut ve tek Undo'dur. Elips yayının
sınır olarak EDGEMODE ile tam elipse genişletilmesi bu dilimde eklenmedi;
burada uzatılan **hedef** elips yayıdır. Web yüzeyi ADR 0228 gereği ertelidir.

## Birincil dayanaklar

[QCAD Trim](https://qcad.org/doc/qcad/latest/reference/en/scripts/Pro/Modify/TrimPro/doc/TrimPro_en.html)
elipsi çizgi ve yayla birlikte budama/uzatma hedefi olarak tanımlar.
[AutoCAD EDGEMODE](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Core/files/GUID-ED7D2A22-0C65-49C3-A58E-D10A3BECBE57.htm)
eliptik yayın doğal uzantısını elips olarak açıklar. KentOS için karar,
bulunan çekirdek yeteneğini doğal geometrisini koruyarak bütün native
seçim yollarına ulaştırmaktır; bu sayfalar kendi hesaplarımızın doğrulaması değildir.

## Doğrulama

- Beş native komut testi: salt okunur plan/yazma eşitliği, ilk sınır,
  kimlik/veri ve tek Undo; çit/pencere; iki uç ve tekrarlı seçim; tam
  elips reddi; katlanmış yolun iki gerçek ucunun birer kez uzaması.
- Interaction: 71 kitaplık + 857 bütünleşme testi geçti (2 ignored).
  Elipsin hover planı yazmaz; Uzat ve Buda'da Shift+tık aynı anlamsal
  planı yazar. Native kitaplık 93 test geçti (1 ignored); bütün ortak
  komut örnekleri masaüstü yürütücüsünde eşleşti.
- Typed Python ve gerçek MCP sunucusunda plan/yazma, analitik elips türü,
  kaynak verisi ve Undo sınandı. SDK yeniden tasarlanmadı; aynı komut kullanılır.
- Gerçek macOS GUI'de tıklama ve Kesişen pencereyle uzatma, kayıt ve tek
  Undo sınandı. Bağımsız KCAD okuyucusunda uç `(-2, sqrt(3))`, son
  parametre `2π/3`; merkez/eksen/oran ve UID korundu. GUI pencere sonucu
  typed Python planıyla birebir eşleşti. Ham görseller ve dosyalar Git
  dışında `/tmp/kentos-cad-adv04-2026-11-01/46-...49-...png` altındadır.
