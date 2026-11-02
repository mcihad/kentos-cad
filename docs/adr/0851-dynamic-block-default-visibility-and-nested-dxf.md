# ADR 0851 — Dinamik blokta varsayılan görünüş, etiket ve iç içe DXF

Tarih: 2026-11-02. CAD-ADV-06.

## Dayanak ve bulunan hatalar

[AutoCAD görünüş durumları](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-B30B3B47-F93B-487E-998F-2E8C72D22CBD.htm)
bir yerleştirmenin hangi geometrisinin gösterileceğini belirler.
[Varsayılan durum](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-C98C9DD3-042C-4713-88BB-C640184540B1.htm)
durum listesinin ilkidir. KentOS'un varsayılan değerler için hızlı yolu ilk
durumun gizlediği parçaları hesaba katmıyordu. Sade ilk durum yapılınca,
boş parametreli kapının açılım yayı görünüyordu.

Bu düzeltmenin gerçek GUI tekrarında ikinci hata görüldü: kapının NO etiketi
kayboldu. Store etiket kayıtları etkin varyantın parça sırasını taşırken
native Spatial etiket gövdesini değişmemiş tanımın aynı sırasından okuyordu.
Gizlenen yay, sonradan eklenen özniteliğin sırasını değiştirdiği için etiket
yerine bir çizgi bulunuyordu. `Blocks::piece_for` aynı varyant önbelleğinden
parçayı alır; yeni geometri açılımı veya tam belge taraması yapmaz. Öznitelik
etiketi ve yerleştirme değeri ayrı kalır; özel konum label kaydında korunur.

ADR 0319'un tanım içindeki dinamik yerleştirmeyi varsayılan değeriyle DXF'e
yazma sınırı da kapatılır. İç içe 1200 mm kapı, dışa aktarımda 900 mm
olmamalıdır.

## Aktarım kuralı

- Sahnedeki ve blok tanımlarındaki her referans kendi parametreleriyle
  sabitleştirilir. İlk görünüş de uygulanır. Tanım ve yerleştirme kaynakta
  değiştirilmez; yalnız DXF girdisi oluşturulur.
- Aynı kaynak ve tam türlenmiş değerler bir statik varyantı paylaşır.
  Farklı üst tanımlardaki aynı çocuk da yeniden kullanılır. İşaretli sıfır
  eşit kabul edilir; sayı, metin, nokta ve bayrak anahtarları karışmaz.
- Varsayılan statik tanım mevcut kimlik/adını korur. Özel değerli varyant
  ayrı, deterministik kimlik ve çakışmayan Vn adı alır. Kullanılmayan
  kitaplık tanımları da tutulur; dışa aktarım örtük bir temizleme değildir.
- Üst yerleştirmenin konumu, dönüşü ve ölçeği; tanımın birimi/tabanı,
  öznitelikleri ve referansın özel değer/konumları korunur. Birim çarpanı
  ikinci kez uygulanmaz.
- `Dinamik blok` uyarısı, parametre/eylem düzenlenebilirliğinin
  taşınmadığını açıklar. Rapor sayısı sahnedeki ve yazılan tanımlardaki
  referans kayıtlarıdır; tekrarlı üst örneklerin fiziksel toplamı değildir.
  Kullanılmayan dinamik tanımın kaybı sıfır yerleştirme kaydıyla da bildirilir.
- Geri alınan DXF düz bloktur. AutoCAD dinamik blok davranışı, lookup veya
  formül taşındığı iddia edilmez; yerel KCAD aynı düzenlenebilirliği korur.

Gerçek GUI kontrolünde DXF penceresinin sabit “birim metre” açıklamasının,
mm dosyanın doğru `$INSUNITS=4` kaydıyla çeliştiği de görüldü. Açıklama artık
yazıcının kullandığı birimi gösterir: yerel çizim birimi, CRS için metre.
Sayısal aktarım kuralı değiştirilmedi. Bu son metin düzeltmesinden sonra
136 exchange testi (31 mevcut ignored), masaüstü derlemesi ve Clippy
tekrar geçti; 195 numaralı gerçek dar pencere görüntüsünde `birim mm` görüldü.

Masaüstü DXF akışı ortak headless girdisini kullanır. Python
`cad.file.export.dxf.plan` / `cad.file.export.dxf` ve MCP karşılıkları aynı
kuraldadır. Yeni komut, protokol veya KCAD sürümü yoktur.

## Doğrulama

315 çekirdek (21 mevcut ignored), 317 application (37 ignored), yeni grafik
birim testi, 881 etkileşim (2 ignored), 82 headless (5 ignored), 54 MCP ve
1603 masaüstü (256 ignored) testi geçti. Bütün ilgili hedeflerde Clippy ve
masaüstü derlemesi geçti. Son temiz adayda bütün Python paketi 431 testle
ve Python native Clippy ile geçti. İlk derleme disk alanı bittiği için kesildi;
yalnız yeniden üretilebilir eski derleme önbelleği temizlendikten sonraki
aday derlemesi geçti. Disk olayı ürün kayıt/kurtarma testi sayılmaz.

Testler varsayılan görünüş/özel etiket konumu ve görünüş değiştirme–Undo;
mm/m birimlerinde iç içe 1.2 m genişlik; dönmüş/ölçekli üst referans; tekrar
kullanılan varyant ve kullanılmayan kitaplık tanımını sınar. Headless testi
beklenen dünya uçlarını ayrıca (10,20)→(7.6,20) olarak denetler. MCP de aynı
dosya/plan/execute yolunda iç içe çocuğu ve salt okunur kaynak kaydını sınar.

Gerçek GUI'de Sade ilk durum, Açılım alternatifi, NO öznitelikleri ve
formüllü 900/1200 mm bileşenler beraber açıldı. Başlangıçtaki kaybolan K-1
etiketi düzeldi. Arayüzden DXF yazıldı ve tekrar açıldı: iç içe çocuk 1.2 m,
K-1/İÇ-1/K-3 değerleri korunmuş, bütün bloklar statik. GUI ve Python DXF
baytları tam aynı; aktarım sonrası GUI kaynak KCAD kaydı başlangıçla tam
aynı. Gerçek GUI görünüş değişimi Python ile tam aynı; tek GUI Undo kaydı
başlangıçla tam aynı. Dar pencere ve Büyük yazı da görüldü.

Bağımsız ezdxf 1.4.5, gerçek GUI çıktısında `$INSUNITS=4`, iç içe çocukta
1200 mm yaprak, üç çizgi/sıfır yay ve üç öznitelik değerini doğruladı.
`block-nested-gui-acceptance.json`, `block-nested-ezdxf-acceptance.json`,
188–195 görüntüleri ve test logları `/tmp/kentos-cad-adv04-2026-11-01/`
altındadır; repo içine ekran görüntüsü alınmadı.

Bu dilim bileşen aile/yayın sürümü veya kalıcı tip–örnek politikasını
bitirmez. `drawing.measure` yerleştirme sınırının tam blok sınırı olmadığı
ayrıca görüldü; aktarım kabulü bununla ölçülmez. CAD-ADV-09 kapsamında ele
alınmalıdır. Web etkileşim kabulü yapılmış sayılmaz.
