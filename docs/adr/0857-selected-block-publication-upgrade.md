# ADR 0857 — Kitaplık yayınını seçilmiş yerleştirmelere yükselt

Tarih: 2026-11-02. CAD-ADV-06'nın yayın yükseltme dilimi; tip kapsamındaki
kalıcı parametreler ve birleşik kabul ayrıca açık kalır.

## Dayanak

[AutoCAD 2026 blok değiştirme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-5E78781D-AD09-4C6F-A3C0-05BD99CB410F.htm)
seçilmiş referanslara başka çizimden blok uygulayabilir.
[Revit yeniden yükleme](https://help.autodesk.com/cloudhelp/2027/ENU/RevitLT-Model/files/GUID-25A9F912-180A-4632-8DBD-52D90BF65047.htm)
tanımı güncelleme ve parametre değerlerini değiştirme kararlarını ayırır;
tip değerlerini ezmek bütün tip örneklerini etkiler. KentOS'un mevcut
kitaplık import'u ad eşleştirip tanımı değiştirir; seçili birkaç yerleştirme
için bu yol kullanılamaz. ADR 0856 aile/tip/yayını yerel sayaçtan ayırdı.

## Karar

- Ayrı otomasyon yazıcısı yerine `cad.blocks.replace` isteğe bağlı `library`
  metni alır. Verilmezse mevcut çizim içi değiştirme aynen kalır. Verilirse
  `to`, `kentos.blocks` sürüm 2 içindeki kaynak kimliktir; `from` çizimdedir.
  İki kartın sıfır olmayan aile/tip kimlikleri aynı, hedef yayın daha yeni
  olmalıdır. Etiket/tip adı eşleşmesi kimlik eşleşmesi yerine geçmez.
- Eski tanım ve onu içeren eski üst tanımlar korunur. Hedefin yalnız
  erişilen bağımlılık grafiği ayrı alınır; aynı ad veya kaynak UUID'si
  mevcut tanımı yeniden tanımlamaz. `purge=true` bu kipte reddedilir.
  Birimsiz–fiziksel değişimi ve harici referans bağımlılığı açık ret verir.
- Yeni çizim UUID'leri SHA-256 ile her kaynak tanımının sıralı, kanonik
  bağımlılık kapanışından deterministik üretilir (UUIDv8 alan bitleri).
  Böylece plan/uygula ve sonraki seçilmiş kapsam aynı hedefe yönelir;
  değişmeyen yayımlanmış çocuk başka ana yayında yeniden kullanılabilir.
  Bu, yayıncı imzası veya dışarıdan gelen içeriğin güvenilirliği iddiası değildir.
- Mevcut hedefin yalnız görünen adı değişmişse adı korunur. Diğer yerel
  hedef değişiklikleri, aynı yayın kimliğiyle farklı kaynak grafik ve
  eşlenemeyen örnek girdileri atomik reddedilir. Aynı yayına farklı içerik
  hazırlamak yerine yayıncı yeni yayın numarası vermelidir.
- Eski tanımlar silinmediği için kullanıcının yerel tanım geometrisi
  kaybolmaz. Seçilen örnekler kaynak yayına geçerken uyumlu ad/türdeki
  parametre girdileri korunur, hedef formülleri hesaplanır. Mevcut ortak
  değiştirme kuralları konum/ölçek/dönüş/katman/UID'yi korur; eşleşmeyen
  öznitelik/özel konum kaybını önizlemede gösterir. Bu dilim kendiliğinden
  etiket veya parametre yeniden adlandırma tahmini yapmaz.
- Kitaplık dosyası okunduğunda ortak komuta verilebilen kaynak metni de
  saklanır; satırlar metnin `Arc` paylaşımını tutar. Fareyle sağ tuş
  **Yayını karşılaştır…**, aynı aile/tipin eski çizim tanımını seçer;
  birden çok aday varsa alt menü kullanıcıya seçtirir. Kaynak okunduğu
  haliyle önizleme girdisidir, uygulama dosyaya sessizce tekrar bakmaz.
- Mevcut korumalı değiştirme penceresi seçili/tüm doğrudan yerleştirme
  kapsamını, mevcut/hedef yayınlarını, eklenecek tanımları ve komutun
  gerçek etkisini gösterir. Yerel sayaç yayın diye etiketlenmez. Kapat/Esc
  yazmaz; dosya açılışında otomatik yükseltme yoktur. Arka plandaki plan
  eski belge/oturum sonucunu kabul etmez; onay aynı plan kimliğini gerektirir.
- Ortak komutun tek kayıt/Undo sınırı tanımları ve örnekleri birlikte
  kapsar. `target` çözümlenen çizim hedefini, `added` gerçekten eklenen
  tanımları verir; tekrar kullanılan hedefte `added` yazılmaz. Kapsamda
  yazılabilir örnek yoksa kullanılmayan hedef tanımı eklenmez.
  Python SDK, katalog ve MCP aynı girdiyi/sonucu kullanır.

Web arayüzü ADR 0228 / TODOS §0.2 gereği ayrıca ertelenir; ortak üretilmiş
sözleşme web GUI kabulü yerine geçmez. İmzalı yayın, bağımlılıkların sürüm
aralığı çözücüsü, tip–örnek parametre kapsamı ve kullanıcı tanımlı gruplar
bu dilimin tamamlandığı iddiasına dahil değildir.

## Doğrulama

- 4 katalog ve 2 sözleşme bağı üretim testi; 5 blok değiştirme çekirdek
  testi (4 yeni yayın senaryosu), 319 application (37 ignored), 59 MCP,
  1606 masaüstü (256 ignored) ve 452 Python testi geçti. Bağımsız öznitelik /
  değiştirme / sorgu vakaları 21+51+20 olarak güncel. Clippy ve derleme geçti.
  Son arayüz açıklaması değişiminden sonra ilgili 5 pencere testi, Clippy ve
  derleme tekrar geçti. 21 Python blok/kitaplık testi ayrıca hedefli geçti.
- Çekirdek retleri: aynı/eski yayın, farklı aile/tip, sürüm 1 kaynak,
  fiziksel–birimsiz değişim, eksik bağımlılık, uyumsuz örnek, yerel hedef
  değişimi ve değişmiş kaynakla eski planı kullanma. İlk yükseltme iki
  tanım ekledi; ikinci seçilmiş kapsam aynı hedefi kullandı. Değişmeyen
  yayımlanmış çocuk, ana bileşenin sonraki yayınında yeniden kullanılabildi.
- Gerçek GUI'de ortadaki kapı seçilip Kitaplık → sağ tuş → Yayını
  karşılaştır ile 1/3 ve 3/3 kapsam görüldü. Esc sonrası kayıt başlangıçla
  tam aynı. Tek fare tıklaması yalnız ortadaki kapıyı, iç çocuğuyla birlikte
  yükseltti; diğer kapılar yayın 2'de kaldı. Liste eski yayında 2, yenide 1
  doğrudan yerleştirme gösterdi. Tek Undo bütün tanım/örnek yazısını aldı;
  dosyası başlangıçla tam aynı. Gerçek Redo kaydı uygulanmış dosyayla aynı.
- Aynı GUI dosyası bağımsız Python korumalı çağrısıyla bayt düzeyinde aynı:
  SHA-256 `1aa602435658281bb0debf835f947f2a690c97ccb9340e1bc7fba6a850c7b0bd`.
  Yeniden başlatılıp bu dosya açıldı; eski/yeni tanımlar ve geometri korundu.
- Son derlemede minimum 2200×1364 fiziksel piksel pencerede ikinci seçilmiş
  kapsam açıldı. Açıklama mevcut yayının yeniden kullanılacağını gösterdi;
  alanlar ve alt düğmeler pencere içindeydi. Gerçek uygulama kaydında blok
  tanımları hiç değişmedi, yalnız ilk kapı aynı hedefe geçti. Tek Undo
  dosyası ilk uygulanmış kayıtla tam aynı.
- Gerçek dosya kitaplığında uyumsuz yayın 4 hazırlandı; birden çok mevcut
  tanımı seçtiren alt menü kullanıldı. Yayın 3'teki genişlik girdisi hedefte
  bulunmayınca açıklamalı ret ve pasif Değiştir görüldü. Pasif düğmeye
  tıklama/Esc sonrası kayıt yükseltme öncesi dosyayla tam aynı; yarım tanım
  eklenmedi. Koyu tema ve web GUI bu dilimde ayrıca kabul edilmiş sayılmaz.

Kanıtlar `/tmp/kentos-cad-adv04-2026-11-01/` altında: 224–233 ekranları,
`block-upgrade-final-results.json`, `block-upgrade-gui-acceptance.json`,
GUI/Python KCAD'leri ve test günlükleri. İlk yerel Save As kayıtları native
ad alanındaki slash/colon dönüşümüyle kitaplık klasörüne yazıldı; bu gerçek
kayıtlar içerikleri değiştirilmeden kitaplık dışındaki kanıt adlarına
alındı ve eşleme kabul JSON'unda saklandı. Sonraki kayıtlar gerçek klasör
seçicisiyle kanıt klasörüne yazıldı. Ekranlar depoya dahil edilmez.

Bu sonuç seçilmiş yayın yükseltmesini doğrular. Kalıcı tip–örnek kapsamı,
kullanıcı grupları ve CAD-ADV-06'nın bütün birleşik kabulü açık kalır.
