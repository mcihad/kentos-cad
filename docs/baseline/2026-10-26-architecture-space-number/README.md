# Mahal numaralandırma — ARCH-SPACE-02 / ADR 0653

26 Ekim 2026. **İlk numaralandırma dilimi doğrulandı; ana madde kısmen açık.**

## Davranış ve sınır

**Mimari → Düzenle → Mahal numarala**, seçili mahalleri seçim sırasıyla alır;
sonraki tıklama ekler/çıkarır. İlk numara tam metindir (`Z-009 → Z-010`).
Yön izdüşümü seçeneği iki açık noktaya göre sıralar. Numara UUID değildir;
aynı nesnenin etiketi ve çizelgesi mevcut follower yoluyla güncellenir.
Komut yalnız `number` alanını değiştirir, tek undo/redo adımıdır.

`cad.spaces.number` aynı hesabı Python ve MCP'ye sunar. Plan ve validate
salt okunur; `SpacesEdited.spaces/kept` numaralandırma sırasındadır.
`expectedRevision` eski önizlemenin yazılmasını önler. Aynı girdinin tekrarı
boş undo üretmez. Yeni KCAD alanı veya şema sürümü gerekmez.

- 1–10000 farklı mahal; baş/son boşluksuz, kontrol karaktersiz, en çok 256
  karakter ve ASCII sayısal son ek. Mevcut Artır yordamı rakamları sayıya
  çevirmeden artırır; büyük tam sayılar ve sıfır dolgusu korunur.
- Yön sırası mevcut etiket ankrajının yön izdüşümü, eşitlikte sol normal
  izdüşümü, sonra UUID'dir. Eş ankrajların UUID sırası yön tersinde de sabittir.
  Bu bir satır/sütun veya en yakın komşu rotası değildir.
- Seçim dışındaki mahalın aynı tam numarası, gizli/başka katta da olsa,
  bütün yazmayı reddeder. Seçimdeki eski yinelenenler birlikte düzeltilebilir.
  Açıklık numaraları ayrı alandır. Otomatik atlama veya kaynak genişletme yoktur.
- Kilit, yanlış tür, tekrar UUID, eksik kaynak, geçersiz numara/yön, aşırı sayı
  ve eski revision hiçbir kısmi düzenleme yazmaz.

## Bağımsız doğrulama

`python3 scripts/fixtures/spaces_number_cases.py --check`: **68 durum**.
Python tam sayısı/sıfır dolgusu ve açık dikdörtgen konumları bağımsız başvurudur;
KentOS cevapları yakalanıp beklenti yapılmamıştır. Beş çizim birimi, yerel ve
500000/4400000 koordinatı, tıklama/+X/−X/+Y sırası, uzun son ek, Unicode önek,
çakışma/kat/gizlilik, atomik ret, plan, tekrar, tek undo/redo kapsanır.
Bunlar ortak koşucunun içindeki durumlardır; 68 ayrı Rust test fonksiyonu değildir.

Native ek test, çakışan ankrajlarda UUID son kararını ve sonlu noktalardan taşan
yön farkının reddini denetler. Gerçek komut günlüğü dört çağrıyla protokolden
oynatılır; yaratılan UUID'ler eşlenince tam sonuç eşittir. İki mahalın adı,
kullanımı, sınırı ve kaynakları korunur; etikette özel `at/of/content/leader`
ve çizelge bağları tek adımda güncellenir. KCAD gerçek encode/decode/aç kabulü vardır.

`contract-audit.json`: yalnız `cad.spaces.number` eklenmiştir; mevcut komut
şeması/tanımı değişmemiştir. Ortak kod sözlüğünde iki yeni ret kodu vardır.
Katalogdan Python SDK üretildi; manifest aynı komutu Python/MCP'ye bağlar.
Yeni TS/TSX uygulama kaynağı yoktur. TS sözleşme üretimi ignore edilen
`.run/architecture-contours/contracts-ts` içindedir. Web uygulaması ADR0228
uyarınca ertelenmiştir; TODOS.md §0.2 kaydı vardır.

## Gerçek arayüz dosyası ve dış okuyucu

`prepare.py` bağımsız dört 6×4 m eksen dikdörtgeni/0,2 m duvar kaynağını
hazırlar; public tipli Python SDK dört bağlı mahal, dört etiket ve bir canlı
çizelge oluşturur. Üç mahal Zemin, biri Birinci kat; dördüncünün `Z-002`
numarası seçim dışı çakışma denemesi içindir. Her net alan **22,04 m²**.
İlk etiket başlangıçta kaynağından ayrı `(-2,2)` konumuna yerleştirilmiştir;
bu, GUI'de sürükleme yapılmış olduğu iddiası değildir.

`verify.py`, bağımsız KCAD okuyucusu ve Decimal halka alanıyla gerçek
kaydedilmiş modelleri denetler. Bütün UUID'ler, geometri, kaynak/kat bilgisi,
ayarlar, etiket konumu/ankrajı/şablonu/bağı ve çizelge üyeleri korunmalıdır.
Yalnız beklenen numara, türetilmiş etiket metni ve çizelge numara hücresi değişir;
yeni numara glifleri için yalnız ilk çizelge sütunu yeniden boylanabilir.
Ham JPEG baytları değiştirilmeden SHA256 ile kaydedilir; ekranlar boyanmaz.

İlk paket `binary.json`, 01–05 ham görüntüler ve `diagnostic-cancel.kcad`
tanısal kayıttır: 03'te üç iç tıklama sayı0 bırakır, 04'te kenar tıklaması
sayı1 yapar. 05'te Escape/Kaydet sonrası bütün model başlangıçla aynıdır.
İlk bench merkez tıklamaları mahal yazısına denk geldiği için kusuru görmemiştir.

Düzeltme yalnız bu aracın görünür dizin adaylarını ortak analitik dış/delik
sınırıyla sınar; genel Seç kenar tercihi korunur. Yeni regresyon, yazı ve
kenardan uzak noktayla kusuru kapsar; delik, gizlilik, aktif katman ve tür
süzgeci de denetlenir. İkinci paket `binary2.json` ve 06–42 görüntüleri bu
düzeltmenin gerçek fare/klavye kaydıdır. Başarısız/tanısal görüntüler de tutulur.

### Gerçek işlem kabulü

- **07–13:** üç mahalın boş içinden seçim, yeniden tıklayarak çıkarma, araç içi
  Geri al ve tek Escape. İlk çakışma reddinde model görünümü değişmez.
- **14–18:** üçüncü→birinci→ikinci tıklama, ilk numara K-101; tek Enter sonucu
  mahal sırasıyla **K-102/K-103/K-101/Z-002**. Etiket ve canlı çizelge aynı anda
  güncellenir; tek Cmd-Z tam başlangıcı, tek Cmd-Shift-Z aynı sonucu getirir.
  Gerçek dosya seçicisinden yeniden açıldığında numaralar ve bağlar kalır.
- **19–22:** N isteminden D-009; görünür sıralama menüsü, klavyeden iki nokta
  `0,0 → 10,0`; ilk tıklama sırasından bağımsız yön önizlemesi. **20'de üstteki
  Kaydet'e gerçekten basılırken araç hâlâ önizlemededir**; disk modelinde eski
  K numaraları kalır. Sağ tuş yalnız menüyü açar; bir Onayla/bitir sonucu
  **D-010/D-011/D-009/Z-002**, araç kapanır. İkinci onay gerekmez.
- **23–31:** açık tema, normal/büyük yazı, dar/kısa pencere; iki mahal için
  L-001/L-002 önizlemesi, Diğer→Uygula, gerçek Kaydet. Büyük yazıda uzun hata
  metninin tamamına kaydırarak ulaşılır (30); tek Escape belgeyi korur.
- **32–37:** odaklı komut satırında MAHALNUMARALA, koyu/açık büyük yazı ve
  normal yazı; B-099→B-100 geçişi, dar menüden tek Uygula ve gerçek Kaydet.
  Diğer kattaki Z-002 ve seçilmeyen mahal değişmez.
- **38–42:** gerçek geçmişte `cad.spaces.number v1`, tam ilk numara/UUID/sıra,
  türetilen Python kodu görünür. Mevcut Duvar'ın dört görünür ayarı ve
  etkinleştirme/Escape korunur; bu bütün duvar çizme matrisi değildir.
  **42'de çakışma Enter ile reddedildikten sonra araç açıkken gerçek Kaydet**
  yapılır; `collision-toolbar.kcad` tam modeli korur.

`models.json` **20 disk anlık görüntüsü** içerir; hepsi bağımsız
okuyucudan geçer. Bazıları tanısal mevcut-disk kopyasıdır; 20 bağımsız işlem
veya 20 taze Kaydet sayılmaz. 12/14/19'da etkin araçta **Cmd-S komut satırına
`s` göndermiştir**; eski açıklamalar düzeltildi, dosyalar tanısal adla saklandı.
Gerçek önizleme-kayıt ve ret-kayıt kanıtı 20/42'dir. Genel Mac kısayol yönlendirmesi
bu komutta değiştirilmedi; ayrı regresyon işi olarak açık tutulur.

### Gözlenen pencere boyutları

`screenshot-dimensions.json` ham JPEG'in gerçek ölçülerini taşır; 2× Retina.
İstenen boyut ile gözlenen çerçeve birbirinin yerine yazılmaz.

| Tema / arayüz yazısı | Geniş pencere (mantıksal nokta) | Dar/kısa pencere | Kanıt |
|---|---|---|---|
| Koyu / 13 px | 1440×870 | 1100×682 | 07/09, 37 |
| Açık / 13 px | 1440×870 | 1100×682 | 23, 25–27 |
| Açık / 18 px | 1512×872 | 1100×682 | 32, 28/30/31 |
| Koyu / 18 px | 1512×872 | 1100×682 | 33, 35/36 |

Bu sekiz görünüm ikinci paketin ayar/seçim/menü kontrolüdür. 33/35'te küçük
zoom ve büyük arayüz yazısında uzun **eski→yeni** önizlemeler komşu mahalda
birbirine değmiştir. Üçüncü paket yalnız atanacak numarayı `→ B-099` olarak
gösterir; özgün mahal numarası çizimde kalır. Ortak yazı ölçeği, kamera,
geometri veya numaralandırma hesabı değiştirilmez. Son paketin gerçek ekran
kabulü aşağıda kayıtlıdır. Bu kayıt bütün mimari araçların genel matrisini
veya tam 1440×900 / 1100×650 çerçeve testini tamamlamaz.

### Son paket: kısaltılmış önizleme yazısı

`binary3.json`, kaynak/ikili hashleri ve **43–52** ham görüntüler son kodu
eşler. 44/45/46: koyu 13 px, iki iç seçim, yalnız atanacak `→ T-099` /
`→ T-100`, tek Enter/gerçek Kaydet ve tek undo/gerçek Kaydet.
`final-confirm.kcad` ve `final-undo.kcad` tam bağımsız model kontrolünden geçer.
47–49: koyu 18 px, geniş 1440×872 ve dar 1100×682; fare tekerleğiyle
uzaklaştırılan sahnede komşu yeni numaralar ayrı okunur. Home tuşu bu etkin
araçta kamerayı sığdırmamıştır; 48 bir sığdırma kabulü değildir. Tek Escape
sonrası gerçek Kaydet, `final-dark18-cancel.kcad` modelini korur.
50–52: açık 18 px, 1512×872 ve 1100×682; aynı uzak sahnede iki yeni numara,
Diğer→tek Uygula, gerçek Kaydet; `final-light18-apply.kcad` tam modeli doğrular.

Bu görsel düzeltme yalnız önizleme metninden eski numara tekrarını kaldırır.
Kamera/yazı ölçeği/ortak renderer, mevcut oda yazıları ve etiket konumu
değişmez. Deneme dosyasında bazı etiketler oda yazısına özellikle yakın
bırakılmıştır; onların çakışmaması bu aracın kabul iddiası değildir.

## Test kayıtları ve kalan kapsam

`test-runs.json` ve `testlogs/` gerçek komut/çıkış/tarih/hash taşır. Son tam
application çalışması **51 lib + 219 integration = 270 başarılı, 22 atlanan**.
Atlananlar başarı sayılmamıştır. Altı araç etkileşim testi, üç desktop geçmiş testi ve yedi gerçek MCP
host testi geçmiştir. Katalog üç testi, 300 Python testi ve dört paket/all-targets clippy geçmiştir. İç seçim düzeltmesi ve son önizleme metninden sonraki desktop/interaction clippy, altı etkileşim testi ve üçüncü build ayrıca kaydedilir. Web/WASM ve envanter ADR0228 gereği çalıştırılmadı; web borcu kayıtlıdır.

İlk derlemede başlık dizisinin boyu, ilk katalog denemesinde yeni girdi türünün
örnek doğrulama kaydı eksikti; düzeltildi. İlk clippy, doğrulanmış son ek için
kullanılan `expect`'i reddetti; hata yoluna çevrildi ve tam uygulama testleri ile
clippy tekrar geçti. İlk iç seçim regresyonunun derlemesinde katman iç seçim alanının `style` altında olduğu düzeltildi. Başarısız günlükler tanısal kayıttır, kabul sayılmaz.

ARCH-SPACE-02 bütünü açık kalır: boş/yinelenen numara denetim listesi,
kat/ad/kullanım listeleri, daha zengin etiket şablonları, gerçek etiket taşıma
sonrası duvar değişimi/silme/yenileme ve farklı pafta oranları tamamlanmış değildir.
Son paketin kabulü 43–52 ile yukarıda sınırlandırılmıştır; önceki paketin sekiz
görünümü bütün araçların genel kabulü değildir. Diğer mimari maddeler bu dilimle kapanmaz.
