# 0829 — Gerçek elipsin merkez ve uç kısıtları

2026-11-02. Durum: bu adaptör ve gerçek GUI kabulü doğrulandı. CAD-ADV-05 açık.

## Karar ve sınır

Elipsi segmentlere dönüştürmeden mevcut dual türevli çözücüye katıyoruz.
Bilinmeyenleri merkez x/y, pozitif iki yarı eksen uzunluğu ve eksen
istikametidir. Eliptik yay iki açılmış parametreyle yedi; tam elips beş
bilinmeyen taşır. Merkez sayıları yerel orijine alınır; eksen uzunlukları
ve açılar koordinat gibi kaydırılmaz. Açılar büyük yarı eksenle ağırlanır.

Mevcut `Point(center/start/end)` sözleşmesi kullanılır; yeni dosya şeması,
komut veya bağımlılık yok. Tam elipsin Start/End'i mevcut SubRef'in dikiş
noktasıdır; tam eğriyi iki bağımsız uçla açmayız. Eliptik yay uçları gerçek
`c + a·u·cos(t) + b·v·sin(t)` noktasından dual sayılarla hesaplanır.
İki yay ucu ve simetri doğrusu 18 türev bileşeni gerektirdiği için dual
kapasitesi 16'dan 20'ye yükselir; kapasite aşımı kontrolü kalır.

Pozitif eksenler, `b <= a` ve aynı açık yay dalında `0 < t1-t0 < 2π`
korunur. Geçersiz deneme sonlu bir geometri gibi kabul edilmez; mevcut
adım küçültme/tekillik teşhisi devrededir. Tam/kısmi değişimi farklı
bilinmeyen düzenidir. Salt merkez değişiminde eksen vektörü, oran ve
saklanan açılar tekrar trigonometrik hesapla yuvarlanmaz.

Bu dilim genel elips teğeti, eksen uçları için yeni referans türü,
elipsleri eşitleme veya bütün konik kesit çözücüsü değildir. Dairesel
`Circle` elemanı elipste reddedilmeye devam eder; yarıçap/çap uydurulmaz.
Geçici edit tutuşu uç elemanının kullandığı sayıları tutar; elips ucunda
bu, bütün elips sayılarıdır. Kısmi serbestlikleri ayrı tutan elips grip
hedefi ve yaylı path yarıçapı CAD-ADV-05'in sonraki işleridir.

## Görünür seçim

Kısıt aracı nokta isteyen adımda uzamsal adayların gerçek, desteklenen
noktalarını sınar. Daire/elips merkezine tıklamak için kenarın aynı
pikselde olması gerekmez. Genel SubRef otomatik başvuru seçicisinin merkez dışlama
politikası regresyonla doğrulanıp korunur; merkez yalnız kısıt
aracının nokta seçicisine eklenir. Süzgeç, görünürlük ve salt yakalamaya açık nesne politikası
korunur; iç bölgeyi rastgele bir nokta kabul etmeyiz.

Nokta zorunluysa, bir dairesel/eliptik kenarın üzerine gelmek de merkezini
belirtebilir; önizleme merkezdeki işareti gösterir. Bu, merkezi yayın
kutu kapsamı dışında kalan eliptik yaylarda da çalışır. Eğri alabilen
Çakışık/Sabit adımı bu kısa yolu kullanmaz: daire gövdesinden Sabit,
eskisi gibi merkez ve yarıçapı birlikte tutar.

Parametreler penceresinde hiç geometrik kısıt yoksa “Tam kısıtlı” yerine
“Geometriye bağlı kısıt yok” gösterilir. Serbestlik halen yalnız çözücüye
katılan nesneler içindir.

## Araştırma

[AutoCAD GEOMCONSTRAINT](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-D79452FB-DDA7-44C0-A9EA-F4EA656022D2.htm)
merkez/eksen noktalarını ve eş merkezlilikte elipsi ayırır; eşit yarıçapı
ise daire/yaya sınırlar.
[BricsCAD 2D constraints](https://helpcenter.bricsys.com/en-us/document/bricscad/drawing-accurately/2d-constraints?version=V26)
gerçek geometrik bağ, sürücü ölçü ve yalnız ölçen referansı ayırır.
KentOS'un bu dilimdeki kapsamı yukarıdaki mevcut sözleşmeyle sınırlıdır.

## Kanıt

- Altı çekirdek testi: 3e6/1e7 koordinatta tam elips merkezi; dikiş
  noktasından eksen sürme; bir uç/merkez sabitken diğer ucu değiştirme;
  her bilinmeyenin türevinin bağımsız geometrik formülün merkezî farkıyla
  karşılaştırılması; eksen/dal sınırı; iki eliptik yayın simetrisi.
- Üç ürün komutu testi: eksen 4→6 ile bağlı çizginin izlemesi, gerçek
  elips/UID/dosya/tek Undo; kilitli katmanda çelişkinin atomik reddi;
  geçici tutulmuş elipsin yerinde kalması.
- Fare/klavye etkileşiminde merkez, eksen uzaklığı ve boş iç bölgenin
  reddi; yay gövdesinden merkez seçimi; daire gövdesinden Sabit regresyonu.
- Typed Python ve MCP: aynı eksen düzenlemesi/teşhis, planın yazmaması,
  analitik elipsin ve kapalı biçimin korunması, dosya ve tek Undo.
  Sorgunun yeniden çözüm adım sayısı ile yazma işleminin adım sayısı
  farklı olabilir; geometrik teşhis alanları karşılaştırılır.

## Birleşik kabul — 2026-11-02

Gerçek uygulamada boş elips/çizgi çiziminden fareyle merkez Sabit,
yatay merkez–dikiş bağı, tek Enter ile `d1 = 4` ve çizgi ucuna Çakışık
kuruldu. Parametre penceresinde `4 → 6` tek Enter ile uygulandı.
Büyük yarı eksen 6 m, küçük yarı eksen 2 m, merkez (0,0), tam kapalı
elips ve bağlı çizginin başlangıcı (6,0), bitişi (7,0) bağımsız kaydedilmiş
dosya okumasıyla doğrulandı. Tüm nesne/UID/kısıt kimliği/parametre ve
teşhis, aynı başlangıç dosyasına typed Python Set uygulanınca birebir
aynıdır. Üç serbestlik kalır: küçük eksen ve çizginin serbest bitişi.
Serbest geometriyi göster iki nesneyi seçip kapsamına getirir.
Tek görünür Undo düğmesi aynı kimlik ve değerleri 4 m durumuna döndürür.
Görseller 66–70 ve sayısal karşılaştırma repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altındadır.

Çekirdeğin bütün test hedefleri (888 başarılı), yerel ürün komutları
(286), etkileşim (71 birim + 863 bütünleşik), desktop (1577), yeni
Python (1) ve MCP (1) kabulü başarılı. İlgili dört crate ve desktop tüm
hedefler Clippy temiz; bağımlılık yönü 51 crate/67 hedef için temizdir.
Ortak SubRef'in eski merkez dışlama beklentisi önce regresyonda yakalandı;
seçim genişlemesi kısıt aracına alınarak eski testler korundu.

GUI, iki sonraki UX/çözücü işini de gösterdi: başarılı tek parça kısıttan
sonra aynı noktadaki hover hemen tekrarlayan kısıt uyarısı gösterebiliyor;
yatay dikişli serbest elipsi doğrudan düşeye zorlayan simetrik başlangıç
adımı eksen çöküşüne yönelip güvenli tekillik reddi verebiliyor.
Bunlar çözülmüş sayılmaz; CAD-ADV-05'in dal/önizleme kabulünde izlenir.


Takip: başarı/tekrar hover sorunu [ADR 0830](0830-constraint-success-feedback-and-parameter-units.md),
merkez–uç çeyrek dönüş sorunu [ADR 0831](0831-ellipse-ray-direction-without-axis-collapse.md)
ile gerçek arayüzde doğrulanarak kapatıldı. CAD-ADV-05'in diğer açık
kabul işleri destek matrisinde sürer.
