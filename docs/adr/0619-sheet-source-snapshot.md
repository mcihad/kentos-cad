# 0619 — Pafta çıktısının sabit kaynak sürümü

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-29.

Native plot sınırındaki `provenance::Capture`, çizimin salt okuma kopyasını,
özgün revizyonunu, anlamsal SHA-256 özetini, kullanıcı/tarih bağlamını ve bağlı
CAD resimlerinin sürümlerini bir defa yakalar. Toplu yayının ayrı PDF'leri de aynı
Capture'ı kullanır. Önizleme, PDF, PNG, SVG ve atlas aynı kaynak kuralını izler.
Çıktı DPI seçimi yalnız yakalanmış render kopyasına uygulanır; özgün çizimin
revizyonu veya birimi değiştirilmez.

Bağlı resimler başlangıçta kanonik dosya yoluyla çözülür ve sınırlı akışlardan
iki kez SHA-256 okunarak sabitlenir. İlk gerçek kullanımda içerik yeniden
karşılaştırılır; uyuşmazsa veya dosya kaybolmuşsa işlem reddedilir. İlk okumadan
sonra bütün sayfalar aynı değişmez baytları kullanır: dosyanın sonradan değişmesi
veya silinmesi sayfaları karıştırmaz. Başlangıçta eksik kaynak sonradan ortaya
çıksa da bu iş için eksik kalır. Aynı dosyanın farklı yol yazımları tek önbelleği
paylaşır. Dosya başına sözleşmedeki boyut sınırı, yüklenen ham resimler için iş
başına 512 MB sınırı vardır; kullanılmayan resimlerin ham içerikleri tutulmaz.

`SheetPublishPin` geriye uyumlu isteğe bağlı `resourcesSha256` ve
`contextSha256` korumalarını taşır. Yayın planı ikisini de verir; yürütme öncesi
kaynak, çözülmüş dosya sürümleri ve kullanıcı/tarih/font bağlamı karşılaştırılır.
Eski yalnız belge pinleri açıkça yalnız belgeyi korur. İstek tekrar oynatma
kayıtları anlamsal belge özetinden çıkarılır; önceki bir dosya çıktısı çizim
geometrisi değişmiş gibi yanlış çatışma oluşturmaz. Kanonik yollar kaynak
korumasının özel parmak izine girer; genel metadata dosya adlarını ve kullanılan
resim içeriklerinin SHA-256 özetlerini taşır.

PDF Keywords, PNG iTXt (`kentos.source`), SVG metadata ve `yayin.json.snapshot`
`kentos.sheet-source/1` verisini taşır: özgün kaynak pini, sabit kullanıcı/tarih,
kullanılan resimler, 22 paketli fontun özetleri, yüklü xref/veri kaynağı sürümleri,
çıktıya özgü DPI ve `laterEditsExcluded: true`. Atlas mevcut anahtar bilgisini
koruyup aynı snapshot verisini ekler. Xref ve veri kaynaklarının saklanan
geometrisi/satırları kendiliğinden yenilenmez. Kaynaklarıyla teslimde diskteki
dosya yüklü SHA'dan farklıysa eski geometriyle yeni kaynak birlikte verilmez;
kullanıcıdan kaynağı yenilemesi veya bağı çözmesi istenir.

Revizyon çalışma zamanı kimliğidir; çizimi yeniden açınca sıfırlanabilir.
Aynı anlamsal çizimin SHA'sı korunur, fakat PDF yeni oturumun gerçek revizyonunu
söyler. Bu nedenle yeniden açma regresyonu metadata eşitliği yerine bağımsız
okunan bütün PDF akışlarını (font/resim/sayfa talimatları) ve gerçek önizleme
piksellerini birebir karşılaştırır.

Masaüstü denetim sırasında çizim değişirse sonucu yakalanan eski revizyonla
etiketler. Yayın düğmesi ve olay işleyicisi eski denetimden yayınlamayı engeller;
yeni çizim yeniden denetlenmelidir. Sonradan gelen iş sonucu yeni çizimin
kayıtlarına uygulanmaz. Web arayüzü ADR 0228 uyarınca ertelidir.

Çalıştırılmış native kabul:

- Python hashlib ile hazırlanmış 23 bağımsız olgu; çizim/başlık/değişken/katman
  düzenleme, mm/cm/m tercihi, resmin ilk okumadan önce/sonra değişmesi veya
  silinmesi, eksik kaynağın ortaya çıkması, yol takma adları, aynı adlı iki dosya,
  bağlam ve kaynak pinleri. PDF Info bağımsız okuyucu ve mevcutsa Poppler ile
  okunur. PNG iTXt/SVG XML/PDF metadata aynı kaynak verisiyle karşılaştırıldı.
- 42 PDF/atlas/önizleme/kalibrasyon entegrasyonu, 16 plot birimi, 10 dosya/paket
  birimi geçti. İstek kaydından sonra pinin değişmemesi, kaynak dosyası
  değiştiğinde çatışma ve eski xref ile daha yeni kaynağın paketlenmemesi geçti.
- 28 yayın olgusu, 35 tam kaynak teslimi ve 51 native ön denetim regresyonu geçti.
- Dört masaüstü kabulü ve MCP kaynak pini senaryosu geçti. Açık/koyu 13 ve koyu
  18 yazı boyutunda GPU ekranları açılıp incelendi. İlk incelemedeki yanıltıcı
  “hazır” ön eki kaldırıldı; eski kaynak uyarısı tema rengiyle belirginleşti.
  Son üç ekran yeniden üretildi ve açılarak doğrulandı.
- Katalog üç kontrolü, SDK/879 satır manifest ve 47 crate/61 hedef bağımlılık
  yönü geçti. Native/masaüstü/MCP clippy `-D warnings` ile geçti.

Tam Python takımının **246 testi** geçti; kaynak pini/manifest/PDF metadata
ve değişen resim sürümü çatışması tipli Python üzerinden de doğrulandı.
**875 TS sözleşme üretimi** geçti. Optimize ortak WASM derlemesi ve mevcut
smoke koşucusu geçti; yeni web UI veya bağlayıcı eklenmedi. Kabul tamamlandı.
