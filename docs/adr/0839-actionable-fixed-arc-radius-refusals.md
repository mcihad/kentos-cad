# 0839 — Sabit yay uçlarında anlaşılır yarıçap/çap reddi

2026-11-02. CAD-ADV-05. Durum: native komut, Python/MCP ve gerçek Mac arayüz kabulü geçti.

## Gerekçe

ADR 0837'nin gerçek arayüz testinde, 4 m sabit kirişe 1.5 m yarıçap
verilince belge güvenle korunuyordu; fakat genel tekillik açıklaması
nedenini söylemiyordu. [W3C'nin hata önerisi açıklaması](https://www.w3.org/WAI/WCAG22/Understanding/error-suggestion.html)
bilinen düzeltme yolunun kullanıcıya verilmesini önerir.
[AutoCAD DCRADIUS](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-C0FE6334-7C41-4382-8C28-3EB776CDCACB.htm)
yarıçapı süren ölçü olarak tanımlar. Bu iş KentOS'un mevcut komut
sonucunu açıklayıcı yapar; başka çözücü, otomatik bağ kaldırma veya
modeli yaklaşık düzelten ikinci bir yazma yolu eklenmez.

## Uygulama

- Normal çözücü önce çalışır. Başarılı çözüme gösterim birimindeki
  karşılaştırmayla yeni ret konmaz; özellikle in/ft dönüşümünde sınır
  değerin yuvarlanması uygulanabilir isteği engellemez. Kilit nedeniyle
  çelişki için mevcut kilit önceliği korunur.
- Başarısızlıkta ancak aynı kenarın **iki gerçek köşesine ayrı kalıcı
  Fix bağı** varsa bilinen alt sınır açıklanır. Referans ölçüsü sürücü
  sayılmaz. Geçici tutuş veya yalnız koordinatı aynı başka nokta kalıcı
  Fix varsayılmaz. Bir uç serbestse kiriş kısalabilir.
- `subref::edge_vertex_indices` mevcut tek halka/parça otoritesidir;
  kapanış kenarında ikinci köşe sıfıra döner. Native adaptör aynı
  köşelerin gerçek metre konumlarını okur; kullanıcıya proje biriminde
  kiriş ve sınır söyler. Şekiller UID başına bir kez alınır.
- Yarıçapın sınırı kiriş/2, çapın sınırı kiriştir. Mesaj parametreyi,
  verilen değeri, sabit uçlar arasındaki mesafeyi ve gereken en küçük
  değeri söyler; değeri büyütme veya ilgili Sabit bağını kaldırma yolunu
  gösterir. Bilinen bu çatışma `constraint_conflict` olarak döner;
  mevcut kısıt hata düzeniyle sürücü ve iki Fix'in kimlikleri taşınır.
- UI, Python ve MCP aynı `cad.parameters.edit` / `cad.constraints.edit`
  sonucunu alır. Dosya biçimi veya komut girdi şeması değişmedi.
  Mesaj üretimi durdurma isteğini korur ve belgeye yazmaz.

## Kabul

Native testler m/mm/cm/in/ft için mesajın gerçek birim ve sınırını,
plan–execute aynı hatasını ve kaynağın korunmasını doğrular. Tam sınırda
2 m yarıçap beş birimde de normal çözücüyle kabul edilir; uçlar aynı,
gerçek yarıçap mevcut çözücü toleransında 2 m, tek Undo tam geri döner.
Tek ucu sabit yayda 1.5 m isteği kirişi kısaltarak geçer. Delik kapanış
kenarı 2→0 için çap 3 m isteği, 4 m kiriş / asgari 4 m olarak açıklanır.

- Native application tam paketi 306 geçti, mevcut 37 ignored;
  son odaklı yedi yaylı kenar testi de geçti. Reddedilen girişin Undo
  geçmişine adım eklemediği ayrıca doğrulandı. MCP kabulü bir test,
  Python kısıt dosyaları 14 test; application/MCP/desktop all-targets
  Clippy ve güncel masaüstü derlemesi geçti.
- Gerçek GUI'de büyük koordinatlı, altı bağlı mm çizimde `1500mm`
  girişi tek Enter ile reddedildi. 4000 mm kiriş / en az 2000 mm
  açıklaması tam okunuyor; sürücü ve iki Fix kırmızı işaretleniyor.
  Pencerenin üst teşhisi saklanan geçerli modeli, kırmızı satırlar
  uygulanamayan öneriyi gösterir. Dosyaya öneri yazılmaz.
- Aynı çizimde `3000mm` kabul edildi; altı bağ yeşile döndü, hata
  temizlendi ve teğet çizgi izledi. UI/Python geometri farkı 0.
  Reddedilmiş dosya ve başarılı değişimden sonra tek Undo dosyası,
  başlangıç dosyasının baytlarıyla tam aynı. Python ve MCP aynı
  `constraint_conflict` ve açıklamayı alır.
- Ekranlar `119-fixed-arc-radius-actionable-mm-error.png`,
  `120-fixed-arc-radius-corrected-mm.png`; sayısal kanıt
  `arc-limit-gui-acceptance.json`. Bunlar ve günlükler yalnız
  `/tmp/kentos-cad-adv04-2026-11-01/` altındadır; repoda görsel yoktur.

Bu açıklama, dolaylı bağlardan çıkarılan bütün sabitlikleri veya başka
bir çatışmanın asgari bağ kümesini hesaplayan yeni bir teşhis değildir.
İki açık köşe Fix'i yoksa normal açıklama korunur.
