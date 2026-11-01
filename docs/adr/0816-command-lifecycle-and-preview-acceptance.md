# ADR 0816: Komut yaşam döngüsü ve önizleme kabulü

- **Durum:** uygulandı, 2026-11-01; CAD-ADV-01 tamamlandı.
- **Başlangıç:** main `52b26da86`; ADR 0812–0815'in son kabul dilimi.
- **Kapsam:** genel 2B CAD etkileşimi, masaüstü odak/belge sınırı ve mevcut
  komutların önizlemesi. Yeni geometri motoru veya dosya şeması yok.

## Araştırma ve karar

- [Iced pencere olayları](https://docs.rs/iced/latest/iced/window/enum.Event.html)
  `Unfocused` olayını fare bırakmasından ayrı verir. Odak kaybında basılı
  düğme durumu bırakılır; sahneye yapay bir tıklama/bırakma gönderilmez.
- [AutoCAD sağ tık ayarı](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-2BD818D6-BFA6-4560-A754-17483D5F33C5.htm)
  kısa basışla Enter ve basılı tutarak menü açmayı ayırır. KentOS'un mevcut
  isteğe bağlı ayarı ve 300 ms eşiği korunur; menünün eylemi aşamadan gelir.
- [AutoCAD Fillet](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-C069AF45-ACF0-447D-A7CC-299181A42ED1.htm)
  son yarıçapın korunmasını açıklar. KentOS'ta fareyle görünen yarıçap ve
  Enter'ın uygulayacağı son değer ayrı, görünür metinlerle belirtilir.
- [AutoCAD tarama adaları](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-981679AC-7097-4724-A30D-33F1CAFDD81D.htm)
  iç sınırların dolguya etkisini tanımlar. KentOS'un mevcut sınır/follower
  motoru korunur; bağlı dolgunun değişimi aynı motorla önizlenir.
- [AutoCAD blok öznitelikleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-OnBoarding/files/ACD_FOUNDATIONS_MAIN11.html)
  yerleştirmede değişken değer girişini açıklar. Bu diyalog bir komut
  taslağıdır: Esc yazmadan kapatır, başka belgeye değer taşımaz.

Rakiplerin iç algoritmaları hakkında çıkarım yapılmaz. Önceki dört ADR'nin
durum/Enter/menü kararları sürer. Kullanıcı açıkça seçtiği komut noktalarını
pencere odağını kaybetti diye yeniden girmek zorunda kalmaz; yarım sürükleme
ve geç bırakma ise işlem uygulayamaz. Belge değişimi daha güçlü sınırdır:
taslak ve bekleyen yanıtın hedefi yeni belge olamaz.

## Gerçek pencerede bulunan ve düzeltilen farklar

1. **Kilitli nesne hayaleti:** Taşı seçimine kilitli çizgi eklenince hayalet
   taşınıyor, gerçek komut çizgiyi atlıyordu. Hayaletler de aynı katman
   kilidini süzer. Köşe yuvarla kilit gerekçesini genel bir uyarıyla ezmez;
   desteklenmeyen eğride iki düz kenar seçilmesi gerektiği belirtilir.
2. **Bağlı dolgu:** genel taşıma önizlemesi yalnız mimari ölçü kümesi
   follower'larını gösteriyordu. `preview_followers_with` artık düzenlenen
   bütün destekli kaynaklara uygulanır. Bağlı güncelleme/silme kör afin
   hayaletinin yerini alır; kopyalama özgün kaynağın follower'ını taşımaz.
   Döndür/Ölçekle'nin kopya seçeneği ve Aynala'nın kaynağı silme seçeneği
   aynı ayrımı açıkça bildirir.
3. **Ölçü yazısı:** yerleştirme önizlemesinde çizgiler ve imleç etiketi
   vardı; gerçek yazının konum/yönü görünmüyordu. Artık kayıttaki aynı
   `dimension_layout` ve biçimlendirme ile `TextGhost` üretilir. Stil/font,
   yükseklik, hizalama ve maske korunur; ölçü değeri ikinci kez hesaplanmaz.
4. **Öznitelik diyalogunda çift Esc:** odaklı metin alanı Esc'yi yalnız
   blur için tüketiyordu. Yakalanmış Esc, sadece blok öznitelik/tanım
   diyaloglarında mevcut kapatma yoluna gider. Diğer diyalogların veya
   yerinde metin düzenleyicilerinin Esc davranışı genişletilmez.
5. **Alt+Q:** odaklı komut/değer alanı arama kısayolunu tüketiyordu.
   Mac Option+Q'nun yazılabilir karakter üretmesi de dikkate alınır;
   mevcut fiziksel/Latin tuş yolu kullanılır. AltGr ve metin düzenleme
   kısayolları korunur. Komut girdisinde gerçek pencerede doğrulandı.

## Yaşam döngüsü ve veri sınırları

- `Tool::pointer_cancel`, yalnız basılı fareye bağlı taslağı bırakır.
  Modify, PickObjects, etiket, özellik eşleme, Pan, kırpma ve bölmenin
  ilgili durumları temizlenir. `Session` askıdaki üst komutlara da iletir;
  seçimin basılı durumu ve imleç bırakılır. Normal çok adımlı komutun
  açıkça girilmiş noktaları/ayarları korunur.
- Masaüstü `Unfocused` olayını widget yakalamış olsa da işler. Modifier,
  sahne menüsü, kenet çevrimi, izleme ve panning temizlenir. Viewport'un
  `Gesture` durumu odak kaybında ve belge kimliği değişiminde bırakılır;
  bekleyen sağ tık menüsü sonradan açılmaz.
- `ScopedReply` iş başlamadan belge oturumunu ve gerekiyorsa komut
  çalıştırma sayacını alır. Metin/resim dosya seçimi, nokta bulutu sorgusu
  ve ağ araç yanıtları bu kapsamda doğrulanır. Ağın belge işi komut
  değişiminde sürebilir; başka belgeye teslim edilmez. Ağ hata yanıtları
  da kapsanır. Bu yeni sarmalayıcı bütün uygulama işlerinin iptal sistemi
  değildir; mevcut dosya açma/kaydetme ve iş ailelerinin kendi kimlik
  denetimleri yerinde kalır.
- Belgeyi bırakmak bekleyen araç isteklerini, blok tanımı/öznitelik
  taslağını ve oluşturma formu isteğini temizler. Aynı nesne slotu/UUID'si
  bulunan başka belgeye geç Create/Place dosya cevabı yazamaz.
- Geçersiz girdide mevcut aşamanın beklentisi söylenir. Koordinat örneği
  sadece nokta kabul eden aşamada gösterilir. Fillet sıfır ve pozitif
  birimli uzunlukları kabul eder; negatif, sonlu olmayan ve geçersiz
  değerler yazmadan reddedilir. `200cm` gerçek pencerede 2 m uygulandı.
- Önizleme doküman snapshot'ına veya Undo'ya yazmaz. Bağlı tarama desenleri
  mevcut geometri çekirdeğinden ve sınırlı parça bütçesiyle üretilir;
  sınırı aşan desende kısmi dolgu gerçek sonuç diye çizilmez, sınır gösterilir.
  Kaynak ve follower sayıları için mevcut önizleme bütçeleri korunur.
- Son komut/son seçenek belleği, anlamsal geçmiş, ürün komutları ve
  Python/MCP şeması korunur. Yeni bir yazma veya koordinat makrosu yolu yok.
  Python çıktısı açıkken gerçek ürün çağrısı, seçim ve Undo kaydı görülür;
  kapalıyken uygulanmış taşıma dahi konsola yeni kod eklemez.

## Gerçek GUI kabul matrisi

macOS'ta kaynak koddan derlenen ayrı QA uygulaması, yaklaşık 1100×680
mantıksal piksel pencere; gerçek fare/klavye/yerel dosya diyaloğu kullanıldı.
Müşteri çizimleri değiştirilmedi. Repo fixture'larının Git dışındaki
kopyalarında yalnız çalışma alanı metadata'sı CAD/mimari olarak ayarlandı;
geometri ve birim verisi değiştirilmedi.

| Akış | Görülen sonuç ve kanıt |
| --- | --- |
| Taşı/Kopyala | Seçim → temel → hedef/delta; aşama menüleri, tek uygulama, fazladan kopyasız bitirme, Esc ve tek Undo. ADR 0812–0813, part-1/2. |
| Buda/Uzat | Sınır seçimi ve uygulama ayrımı; tek menü eylemi, geometri ve tek Undo; sınır seçimini Esc ile yazmadan bırakma. ADR 0812–0813. |
| Fillet | Geçersiz değer ret; 200cm → 2m; fare 3,6m önizlemesi yanında Enter 2m açıklaması; sağ tuş menüsünden son 2m'yi tek seferde uygulama, tek Undo. part-5/06–10. |
| Tarama | İç adalı 272m² ve açık çizgi ağı sınırından 176m² önizleme/sonuç; seçenek değişimi yazmıyor. Sayı beklerken tıklama dolgu üretmiyor. ADR 0814, part-5/12–16. |
| Blok | Basit yerleştirme ve tek bitirme önceki dilimde. Öznitelikli blokta R21/101,9 girişi, tek Enter ile yerleştirme, tek Esc ile odaklı diyalogu yazmadan iptal ve tek Undo. part-5/17–21. |
| Dinamik blok | 1,2m genişlik ve yön seçeneği önizlemede; tek yerleştirme 3→4, tek Undo 4→3, Esc fazladan örnek üretmiyor. part-5/22–24. |
| Ölçü | Hizalı ölçü ve klavyeden uzaklık ADR 0814'te. Yarıçap, açı, doğrusal, ordinat ve yay uzunluğu gerçek yazı önizlemesiyle yerleştirildi; geri alma/Esc sonunda ilk 14 nesne korundu. part-5/27–37. |
| Belge sınırı | Dosya/sekme değiştirme, tutamaç, yazı taslağı, bağımsız Undo ve bekleyen komutla kapatma. ADR 0815, part-4. |
| OS odağı | Taşı temel noktası sonrası Finder'a geçip geri dönme; açıkça girilen nokta korunuyor, hedef tek tıklamayla uygulanıyor; tek Undo. part-5/02–05. Basılı düğme sırasında geç release ayrıca otomatik testtir. |
| Bağlı tarama | İç yapı taşınırken tarama boşluğu ve desen de önizleniyor; uygulanan konum aynı; tek Undo yapı ve dolguyu birlikte geri getiriyor. part-5/39–41. |
| Kilit | Karışık iki çizgi seçiminde yalnız düzenlenebilir çizgi için taşınmış hayalet; uygulamada kilitli çizgi sabit, tek Undo; Fillet kilit sebebini koruyor. part-5/46–48. |
| Python | Açık ayarda gerçek `cad.entities.transform.execute_input`, `doc.select`, `doc.undo`; kapalı ayarda konsol boşken taşıma uygulanmasına rağmen yeni çıktı yok. part-5/38,42–45. |
| Sağ tık ayarı | Kısa basış bir kez seçimden temel noktaya geçiyor; basılı tutma “Seçimi tamamla” menüsünü açıyor, geometri uygulamıyor. part-5/44,49. |

Buradaki kabul bütün araçların her geometrik kipinin eksiksiz GUI testi
olduğu iddiası değildir. CAD-ADV-01'in saydığı ana akışlar tamamlandı;
genel araçların ilk aşamasında görünür istem ve yazmadan Esc çıkışı ayrıca
katalog üzerinden otomatik denetlenir. Eğri ailelerini genişletme, yeni
ölçek sistemi veya diğer CAD-ADV maddeleri bu kabul ile tamamlanmış sayılmaz.

## Otomatik doğrulama

- `cargo test -p kentos-interaction --test all --quiet`: **840 geçti,
  0 başarısız, 2 isteğe bağlı atlandı**.
- `TMPDIR=/tmp cargo test -p kentos-desktop --lib --quiet`: **1557 geçti,
  0 başarısız, 256 isteğe bağlı atlandı**. Mac'in uzun varsayılan geçici
  dizini Unix socket yol sınırına takıldığı için kısa TMPDIR kullanıldı;
  Python bağlantı kodu değiştirilmedi.
- `cargo test -p kentos-ui --lib --no-default-features --features fonts,snapshot --quiet`:
  masaüstünün UI profiliyle **286 geçti, 0 başarısız**. Varsayılan UI
  özellikleriyle ayrı önceki çalışmada **322 geçti**; bu iki sayı toplanmaz.
- `cargo clippy -p kentos-interaction -p kentos-desktop -p kentos-ui --lib --quiet`
  ve masaüstü uygulama derlemesi başarılı. Mevcut `ts-rs`
  `deny_unknown_fields` ayrıştırma uyarıları sürüyor.
- Yeni regresyonlar: odak kaybında üç düğmenin geç bırakması ve sağ menü;
  askıdaki Pan/üst komut; yarım seçim sürüklemesi; eski belge/eski araç
  dosya/iş yanıtları; blok diyalogunun geç Create/Place cevabı; kilitli
  nesne gölgesi; bağlı ve ayrıca seçilmiş tarama; ölçü yazısı anchor/yön/
  yükseklik ve tek Undo; Fillet geçersiz/birimli değer ve destek sınırı.
- ADR 0815'te açık kalan geçici ölçü testi incelendi: iki referans karesine
  iki bağ kontrol karesi ekleniyor. Test artık referansların gerçek dünya
  koordinatlarını ayrı doğrular ve toplam dört kontrolü bekler. Uzaklık
  ve geometri doğrulamaları korunur; renderer/hassasiyet kodu değiştirilmedi.

## Kanıtların saklanması

Görüntüler repo dışında `/tmp/kentos-cad-advanced-2026-11-01/` ve
`part-2`–`part-5` altındadır. Her dilimin manifest'i dosya özeti ve adımları
tutar. Son dilimde 49 gerçek pencere görüntüsü vardır; **11, 18, 25–26
düzeltme öncesi kanıtlardır**, başarılı sonuç diye sayılmaz. Karşılıkları
46–48, 19–21 ve 27–37'dir. Ekranlar, fixture kopyaları ve derlenmiş uygulama
Git'e eklenmez.

GUI oturumunda bir ara araç “uygulama değişti / Computer Use etkin değil”
yanıtı verdi ve pencere tekrar açıldı. Nedeni doğrulanamadı; ürün çökmesi
düzeltildi diye raporlanmaz. Temiz QA profiliyle ilgili kabul adımları
yeniden çalıştırıldı; son GUI günlük dosyasında panic kaydı yok. Geçici
klasörler kalıcı CI arşivi değildir; yeniden testte yeni kanıt üretilmelidir.
