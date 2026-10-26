# Görünür etiket ayarları — ADR 0685

26 Ekim 2026. **Bu dar dilim kabul edildi.**

01 ham görüntüde Etiket etkin olmasına karşın Öznitelikler yalnız seçimi/
belgeyi gösterir. Şablon/Biçim/Kol alttaki komut seçeneklerindedir.
`before-binary.json`, ADR0684'in değişmez paketidir; yeni derleme sayılmaz.

Yeni etkin araç paneli şablon, çerçeve, kol ve kağıt yazı boyunu mevcut
native alanlarla sunar. Seçim özelliklerinden ayrı başlıktadır. Mevcut
`cad.tags.create` önizleme ve yazmayı yapar; yeni ürün komutu/şema/SDK,
TS/TSX, birim veya render değişikliği yoktur. Şablon ortak `tag_template`
doğrulamasından geçer. Kağıt mm sayısı çizim birimine çevrilmez; mevcut
NewTag.size anlamı kullanılır. Boş şablon/boy varsayılanı geri getirir.

Komutlar/çıkış kodları/zaman/SHA256 `test-runs.json` içindedir.

- `red-settings`: eski kodda dört başarısız; panel/ayar yolu eksiktir.
- `green-settings`: üç başarılı, yeni testteki çok satırlı önizlemeyi tek
  text primitive sanan beklenti nedeniyle bir başarısız. Gerçekte mevcut
  renderer her satırı ayrı primitive yapar. Yalnız testte satırlar birleştirildi;
  renderer/ürün hesabı değişmedi. Ara başarısız günlük korunur, kabul değildir.
- `interaction-final2`: **697 başarılı, bir atlanan**. Beş yeni test, 20 açık
  şablon/çerçeve bileşimi ile gerçek hedef/yer tıklaması, önizleme, tam kaynak/
  UUID/tek undo/redo; yanlış değer/boş sıfırlama, menü/eski tuş döngüsü,
  konsol sorusundan görünür onay ve Escape/korunan seçimi denetler.
  Ayrıca beş çizim birimi × dört çizim ölçeğinde **20 eşleşme**, 3.5 mm kağıt
  boyu için bağımsız `3.5 * scale / 1000` önizleme yüksekliğini kontrol eder.
- `desktop-settings-final2`: mevcut host ayar/eskimiş olay ve gerçek snap testi
  **üç başarılı, iki GPU testi atlanan**. Bunlar yeni test değildir; gerçek
  GUI kabulüyle birlikte okunur.
- `tag-reference`: 165 bağımsız ortak durum aynı; `sdk-check` geçti.
- `python-tags`: mevcut kamuya açık yerleşim/izleme testleri **üç başarılı**.
  Komut/FFI/SDK değişmedi; ADR0684'teki tam 303 Python testi bu panel
  diliminde yeniden çalıştırılmış gibi sunulmaz.

## Gerçek GUI'nin yakaladığı takip kusurları

02/03 tanısal görüntülerdir: ekran yardımcısı eski uygulama referansını
closure içinde tuttuğu için eski File penceresini yeniden açtı. Yardımcıya
hedef uygulama artık açık argümanla verilir. Bu resimler başarı kanıtı değildir;
ham baytlar ve eski paket hash'i korunur. 04–06 ilk panel paketindendir.

05'te çerçeve menüsü ve 3.5 mm kabulü görülür; ilk hızlı yapıştırma şablona
`v` yazdığı için başarılı şablon kabulü değildir. Ayrı gözlenen odakta ikinci
yapıştırma doğru metni getirdi. 06 doğru şablonu ve mevcut komut düğmesinde
çok satırlı yazının kesilmesini gösterir. Düğme artık tek satır Özel/Türün
özeti taşır; tam şablon panelde kalır.

`red-clipboard`: eski widget'ta KeyPressed kendi Cmd bayrağını taşıdığı halde
önceden ModifiersChanged gelmeyince yeni odaklı hücrenin A/C/V işlevi
çalışmaz; bağımsız pano beklenen `room` yerine eski `copied` kalır. Düzeltme
CommandLine/EditCell'in iç metin kutusuna her basıştan önce o basışın kendi
bayraklarını eşitler. Genel host onayı/kısayolu değiştirilmez.

`ui-final2`: **19**, `cells-final2`: **5**, `saving-final2`: **6**,
`keys-final2`: **10** başarılı test. İki yeni widget regresyonu, kaçırılan
modifier bildirimi ve eski Cmd bayrağının düz `v` tuşuna taşınmamasını sınar.
Önceki A/C/X/V/AltGr/Cmd-S ve normal metin testleri de geçer. Toplam bu dört
kümede **40**; önceki tekrarlar eklenmez.

`clippy-final2` (UI/interaction/desktop, bütün hedefler, snapshot özelliği,
`-D warnings`) ve `build-final2` geçti. Günlüklerde mevcut ts-rs ve gelecek
uyumluluk bildirimleri korunur. Önceki `interaction-final`, `desktop-settings`,
`clippy`, `build` ilk panel paketine aittir; son düzeltmelerin kabulü final2'dir.

## Son pakette gerçek fare/klavye ve bağımsız dosya kabulü

`after2-binary.json` son değişmez uygulamayı ve sekiz kaynak dosyasının
SHA256'sını bağlar. 07–26 bu paketten, ham ve düzenlenmeden alınmıştır.
Kullanıcının normal profili yerine ayrı QA profili kullanıldı; açık/18 ve
koyu/13 değerleri gerçek Ayarlar ekranından kaydedildi ve JSON'ları arşivlendi.

- 07–09: Mimari > Etiket'te dört alan; yeni boş şablon alanına ilk hızlı
  yapıştırma/tek Enter, `Mahal {number}\\n{name}\\nNet {area}`, yuvarlak köşe,
  Kol Var ve `3,5 mm`. Yerleştirmeden Cmd-S: önceki 13 nesne tam aynı.
- 10–12: gerçek Salon kaynağı seçiliyken bilinmeyen token ve sıfır yazı boyu
  reddedildi; geçerli şablon/boy ve hedef korundu. 10'da Kaydet geçici durum
  uyarısını örttüğü için görünür ret ayrıca 11/12'de kaydedildi.
- 13–16: serbest önizlemede Kaydet henüz nesne eklemez. Komut alanına `16,2`
  ve **tek Enter**, sonra Cmd-S: yalnız bir yeni anlamsal etiket, doğru kaynak
  UUID/şablon/kol/çerçeve/3.5 mm ve konum `(16,2)`. Escape tekrar yerleştirme
  aracından çıkar; bir Cmd-Z eski 13 nesne, bir Cmd-Shift-Z aynı 14 nesne.
- 17/18: kaynak mahalın gerçek Ad hücresinde `Oturma`/tek Enter ile iki bağlı
  etiketin metni ve çizelge Ad hücresi güncellenir; elle konum değişmez.
  Bir undo hepsini geri getirir.
- 19–22: **1100×683 pt açık18**; dört alan ve yukarı açılan altı çerçeve
  seçeneği sığar. Gerçek kaynak tıklama/tekerlek önizlemesi, hedefi bırakıp
  araçtan çıkmak için iki Escape; Kaydet yeni nesne eklemez.
- 23/24: **1100×683 pt koyu13**; yeni şablon girişi, `2,75 mm` ve Kol Var
  menüsü kullanılır; bu ikinci denemede nesne yerleştirilmez.
- 25: **1440×863 pt koyu13**, aynı alanlar ve yardım; 07–18 geniş açık18.
  Gerçek piksel/pt boyutları `screenshot-dimensions.json` içindedir.
- 26: Cmd-Q sonrasında aynı son paket yeni süreçte açıldı. Home/Kaydet:
  14 nesne, mevcut şablon/elle yer/3.5 mm ve kaynak/çizelge ilişkisi korunur.

`models.json` içindeki **11 gerçek disk kopyası**, uygulama komutunu veya
geometri hesabını çağırmayan `verify.py` ile karşılaştırıldı. Beklentiler bütün
model, UUID sırası, katman/ayar ve eski nesneler içindir; yalnız açık işlemin
beklenen farkı eklenir. `independent-final` geçti; 26 ham resim, üç değişmez
paket, son kaynaklar ve test günlüklerinin hash'leri de denetlenir.

## Açık sınırlar

Önceki nesne özelliklerinde aşağı kaydırılmış Öznitelikler paneli yeni araca
geçince eski kaydırma konumunda kalabilir. 19 öncesi elle yukarı kaydırıldı;
bu dilim otomatik olarak en üste gelmesini düzeltmez. Yeni ayrı takip işi
planlandı. Dar panelde uzun şablonun tamamı tek satır görünmeyebilir; düzenleme
alanında saklanan değer ve alt komut satırının tek satır özeti ayrı tutulur.
Bütün tema/font/boyut çapraz matrisi, diğer metin bileşenleri, toplu etiket
Koru/Yenile ve döşeme/tavan kotları açık kalır. Bu kayıt üretim bütünlüğü
veya bütün mimari planın tamamlandığı iddiası değildir.
