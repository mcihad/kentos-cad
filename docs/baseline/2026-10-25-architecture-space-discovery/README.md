# Toplu mahal bul — kabul kaydı

25 Ekim 2026, `codex/architecture-2d-workflows`; [ADR 0642](../../adr/0642-bulk-room-discovery.md).
ARCH-SPACE-01 [~] sürer. Kolon, kalıcı sanal ayırıcı/kaynak alt kümesi,
açık sınırı kaynak uçlarıyla tanıma ve tam GUI matrisi bu dilime dahil değildir.

## Davranış ve bağımsız beklenti

Mimari/Çiz içindeki **Toplu mahal bul** aracı, seçilen katın veya **Tüm katlar**
duvar ağının kapalı boşluklarını önizler. Kat verilmezse eski bütün çizim
semantiği sürer; aynı kot farklı kat kimliklerini birleştirmez. Duvar gövdesinin
kapalı hücreleri mahal olmaz. Analitik yaylar sınıflandırma için düzleştirilmez.

`before.kcad` dört kaynak duvar içerir: Zemin katın 10×8 m merkez çizgili
çevresi ve x=5 m bölmesi, 1. katın 12×10 m çevresi ve ayrı fiziksel açık U.
Kalınlık 0,2 m; iki katın kotu da 0 m; başlangıç E=500000, N=4400000 m.
Zemin katın **iki adet 4,8×7,8=37,44 m²** odası, üst katın tek
**11,8×9,8=115,64 m²** odası vardır. Açık U kapatılmaz. Tüm katlar bilinçli
olarak legacy birleşik ağı arar; katlara göre ayrı toplu gruplama değildir.
GUI girdileri SDK ile hazırlanmış sabit dosyalardır; elle çizilmiş gibi sunulmaz.

Bölge dikdörtgeni bir odanın doğrulanmış iç temsil noktasını seçer; kaynak veya
oda sınırını kesmez. Mevcut mahal seed/UUID'si korunur, isim kimlik sayılmaz.
Elle çizilmiş 18 m² dikdörtgenin sol odayla kısmi örtüşmesi kırmızı çakışmadır;
sağda geçerli yeni oda olsa da bütün yazma reddedilir. Sağlıklı seed'li diğer
odanın Brüt duvar offseti ikinci odaya çakışma sayılmaz.

`verify.py`, gerçek GUI kayıtlarını açık KCAD okuyucusuyla okur; SDK veya ürünün
geometri hesabını kullanmaz. Yerel başlangıca taşınmış Decimal shoelace ile
alanı hesaplar; geçici sayısal id dışında her kalıcı UUID/kaynak özelliği ve
proje ayarını karşılaştırır. İki mahalin birlikte undo/redo'su, boş tekrarın
history eklememesi, bölgenin tam sağ odayı koruması, çakışmada sıfır yazma,
400 mahal ve son dar pencere iptal/kayıt/açılışı denetlenir.

## Gerçek pencere sırası

Ham CUA görüntüleri `shots/` içinde JPEG olarak değiştirilmeden saklanır;
`gui-events.json` zamanı/sırayı, `gui-binary-*.json` program ve kaynak hash'lerini
tutar. Kullanıcının çizim penceresi kullanılmadı; ayrı yerel QA uygulamaları ve
yalnız bu kabul dosyaları kullanıldı. Bunlar yeni Git çalışma kopyaları değildir.

- 01–02: d2ba5f7ef temel sürümündeki aynı dört kaynak. Eski tek Mahal önizlemesinde
  **37440000.00 mm²**, fiziksel olarak **37,44 m²** görülür. Görüntüde m²
  yazıyormuş gibi raporlanmaz; önizleme iptal edilmiştir.
- 03–06: yeni şerit düğmesi, birleşik ağ önizlemesi, açık Zemin kat seçimi,
  tek Enter ile iki mahal. Başlık yalnız yazma sonrası kirlenir.
- 07–12: tek undo iki mahalı kaldırır, tek redo aynı UUID'leri döndürür.
  Tekrar 0 yeni/2 mevcut gösterir. Gerçek sağ tuş menüsünde bir
  **Onayla / bitir** seçimi çıkar; bundan sonraki undo önceki asıl oluşturmayı
  kaldırır, boş tekrar adımı bulunmaz.
- 13–15: birinci bölge köşesinden sonra Enter yazmaz. İkinci köşe yalnız
  önizlemedir. Son Enter küçük kutuya rağmen tam sağ odayı oluşturur.
- 16–17: elle çizilmiş oda kısmi çakışması kırmızı; Enter açık ret,
  hiçbir mahal yazılmaz. `conflict-before/after.kcad` kalıcı içerikleri aynıdır.
- 18–23: 39 kaynak duvar, **400 oda** önizlemesi, tek Enter ile **439 nesne**.
  Tek undo 39 kaynağa, redo 439 nesneye döner. Yakınlaşınca 14,44 m² etiketleri
  okunur; bütün katı sığdırınca küçük etiketler okunur boyutta değildir.
- 24–27: ilk panelde kesilen **Yeni / Var / Çakışan** başlığı, üç kısa
  salt okunur satıra ayrıldı. Yeni derleme gerçek pencerede okunur. Üst kat
  bir oda önerir; Zemin kata dönüp tek sağ tuş onayı iki oda oluşturur.
- 28–30: **1100×650 iç pencere**, koyu tema/18 yazı. Şerit düğmesi ve Kat
  seçimi erişilir; panel kaydırılabilir, sayaçlar ve sahne özeti kullanılabilir.
  İptal belgeyi değiştirmez. Kaydedilmiş dosya uygulamadan yeniden açılır:
  aynı iki mahal ve dört kaynak korunur.
- 31–35: son katalog sırası düzeltmesiyle yeniden derlenen aynı uygulama
  açılır; üst katın 115,64 m² mahali eklenir. Geçmişte **Toplu mahal bul**
  başlığı, tek `cad.spaces.discover` çağrısı ve Python bölümü görülür.
  Python komut kimliği aynı çağrıya aittir. 34 yalnız kaydırma denemesidir;
  35 açılmış Python bölümünün kimliğini gösterir. Üst kat oluşturma tek undo
  ile kaldırılır, alttaki iki mahal ve bütün kaynaklar korunur.
- 36: kilit açıldıktan sonra aynı derleme tekrar kullanılır. Araç görünür
  düğmeden açılır, Kat menüsünden Zemin kat seçilir; üç sayaç ve sahne etiketi
  **0 yeni / 2 mevcut / 0 çakışan** gösterir. Escape ile yazmadan çıkılır.
- 37: Zemin kat önizlemesinde yöntem **Brüt** yapılır; dış yüzler önizlenir
  ama mevcut iki net mahal yinelenmez, 37,44 m² etiketleri değiştirilmez.
  Yöntem seçimi var olan mahalı düzenleme komutu değildir. Escape ile çıkılır.
- 38–39: capabilities metadata düzeltmesini de içeren son derleme yeniden
  açılır (`gui-binary-delivery.json`). Zemin kat yine 0 yeni/2 mevcut gösterir;
  Escape sonrası 6 nesne kalır. Kaydet düğmesiyle dosya yazılır;
  `delivery-catalog-cancel.kcad` bağımsız okuyucuda önceki iki mahalın ve bütün
  kaynak/ayarların aynı kaldığını doğrular.

İlk geniş pencereler ham 2880×1726, sonraki 2880×1730 pikseldir; yaklaşık
1440×831–833 iç boyut. **1440×900 tam kabulü değildir.** Açık tema/13/16 ve
öteki boyutların tam gerçek pencere matrisi açıktır. Mevcut genel başlıkların
dar pencerede kısalması bu dilimde bütün uygulama yerleşimi düzeltilmiş sayılmaz.

## Otomatik kapılar

Kesin sonuç/sayı/atlama ve ham log hash'leri `test-results.json` ile
`gates-delivery.json` içinde tutulur. Önceki başarısız koşular son koşunun yerine
geçmez; aşağıdaki açıklamalar ile birlikte saklanır.

Son geniş koşu **4.007 Rust testi başarılı / 0 hata / 279 atlanmış** ve
**285 Python testi başarılı / 0 hata / 0 atlanmış** sonucunu verdi. Rust sayısı
komut/sözleşme, masaüstü, domain, geometri, etkileşim, MCP ve uygulama
kümelerinin toplamıdır; 34 yeni senaryo tek ortak fixture testinin alt
durumlarıdır, bu toplama ayrıca eklenmez. Atlanan elle ekran/perf testleri
başarılı sayılmadı; bu dilimin gerçek CUA kayıtları ayrıca tutulur.
Ham derleyici loglarındaki boşluklar/satır sonları değiştirilmez; kaynak ve
belge `git diff --check` kontrolünde yalnız bu ham `.log` kayıtları hariçtir.

- Çekirdek: iki oda, açık U, T/çapraz, ters yön, uzak koordinat, iç avlu,
  400 oda, analitik daire, Net/Eksen/Brüt, eski tek seed sonucu eşitliği.
- Native: beş çizim birimi, aynı kot farklı kat, salt okunur plan, revision,
  kilitli hedef, bilinmeyen kat, bölge, yinelenen/eşit elle çizilmiş ve kısmi
  çakışan mahal; tek undo/redo, snapshot ve bölme değişince iki odanın takip hesabı.
- Etkileşim: önizleme yazmaz, tek confirm oluşturur, boş tekrar undo eklemez;
  bölge iki köşe gerektirir, ikinci köşe önizlemedir, iptal kaynakları korur.
- Python: tipli plan/execute/journal, miktar, gerçek KCAD kayıt/açılışı,
  kalıcı kimlik ve kaynak korunumu; m/mm/ft ve uzak analitik yaylı odanın üç alanı.
- MCP: gerçek sunucu ve dosyayla plan/execute/tekrar, tek undo/redo ve metraj.
- Eski bağımsız 87 space durumu değiştirilmedi. Katalog birim alanları, SDK,
  889 satırlı manifest, şerit ve bağımlılık yönü denetlenir. Core WASM hedefinde
  derlenir; web UI/TS uygulaması bu dilimde eklenmez.

İlk katalog kontrolü `bounds` uzunluk birimini yakaladı; alan düzeltildi.
Clippy üretim `unwrap` kullanımını yakaladı; kalıcı UUID iteratorü kullanıldı.
SDK üreticisinin union varyantına özel alanı bağımsız çıktı türü olarak
desteklememesi, guard kaldırılarak aşılmadı: çıktı ortak `SpaceFields` oldu.
İlk iki geniş native koşusu yeni komutun geçmiş adı ve sonra katalogdaki
sırası nedeniyle durdu. Üçüncü koşu eksik capabilities kaydı ve ortak
fixture dosyasını yakaladı. 34 bağımsız komut senaryosu ve komut yetenek
tanımı eklendi; bozuk kaynak duvar ve tipli/zorunlu alan retleri de kapsanır.
Yinelenen duvar noktasının mevcut `invalid_wall` kodu korunur. Sonradan eklenen
metadata, görüntülerdeki UI/geometri/etkileşim kodunu değiştirmez.
Beklenti gevşetilmedi; liste katalogla eşitlendi,
önce hedef test çalıştırıldı, sonra geniş koşu yeniden başlatıldı.
Depodaki önceden var olan genel format farkları için `cargo fmt --all --check`
geçmez. Yeni uygulama/etkileşim/geometri dizini dosyaları, yeni Rust
testleri ve ölçüm örneği bağımlı modüllere yayılmadan `rustfmt --check`
ile denetlendi; iki ayrı ham format kontrolü logu boştur ve başarılıdır.

## Çoklu sorgu performansının kapsamı

`legacy-plan-equality.json` ve son FFI için `legacy-plan-equality-delivery.json`:
değişiklik öncesi FFI ile kaydedilen **400 seed'in
tam plan JSON'u**, yeni FFI planıyla birebir eşittir. Bu bir süre ölçümü değildir.

`space_queries.rs` release örneği 16/144/400 seed için eski her seed'de
yeniden kurulum ve yeni tek ortak sorgu dizinini karşılaştırır. Her iki sürede
dizin kurulumu dahildir; 9 dönüşümlü örnek ve tam analitik sınır eşitliği,
medyan/p95 raporlanır. Bu **çekirdek sorgu** ölçümüdür: yeni toplu komutun
doğrulaması/yazması, Python, GUI, GPU veya follower uçtan uca süresi değildir.
Kesin ölçümler ham `release-benchmark.log` ve ayrı JSON kaydındadır.

Apple M5 üzerinde release ölçümü (`machine.json`, milisaniye):

| Oda/seed | Eski medyan / p95 | Ortak dizin medyan / p95 |
|---:|---:|---:|
| 16 | 2,003 / 2,338 | 0,124 / 0,147 |
| 144 | 141,122 / 145,763 | 1,024 / 1,370 |
| 400 | 1128,548 / 1154,410 | 3,055 / 3,452 |

Her boyutta 9 dönüşümlü örneğin tam analitik sınırları eşittir. Bunlar yalnız
aynı ağda çoklu çekirdek sorgusudur; yeni toplu komutun veya genel uygulamanın
bu oranlarda hızlandığı anlamına gelmez. QA uygulamaları açık kalmıştır;
uçtan uca/geçiş başına RAM ve kullanıcı gecikmesi ayrı kabul kapısıdır.

Bir çağrıda 10.000 kaynak duvar ve 10.000 oda sayım kotası vardır; bu kesin
RAM/süre garantisi değildir. İşlem sırasında farklı yakın kaynak kümeleri
ayrı dizin kurabilir. Kalıcı önbellek yoktur; immutable belge ödüncünün bitmesiyle
sorgu dizini de biter. Farklı yakın kümelerin dizinleri aynı geçişte bellekte
birikebilir; büyük belgede uçtan uca süre/RAM kabulü açık kalır.
Kolon/sanal sınır bağımlılıkları henüz eklenmemiştir.
