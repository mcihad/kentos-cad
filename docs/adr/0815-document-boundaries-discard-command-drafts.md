# ADR 0815: Belge değişiminde bekleyen etkileşimi tamamen bırak

- **Durum:** uygulandı, 2026-11-01; CAD-ADV-01 [~].
- **Başlangıç:** main `d9b6fe0fa`; ADR 0812–0814'ün devamı.
- **Kapsam:** masaüstünde dosya açma, sekmede bekletme/değiştirme ve kapatma.
- **Dosya şeması, bağımlılık, birim ve Python–MCP komut şeması:** değişmedi.

## Gerçek pencerede bulunan sorun

Tarama → Ölçek aşamasında sayı beklerken başka bir KCAD dosyası açıldı.
Yeni çizim açıldığı halde Tarama çalışmaya devam etti; sadece sayı aşamasından
bölge seçimine dönmüştü. `show_document` tam iptal yerine `App::cancel`
çağırıyordu. Bu işlev normal Esc'dir: aracın bir önceki adımına dönebilir,
kilidi bırakabilir veya şeffaf komutun altındaki komuta dönebilir.

`park_active` ve `close_tab_now` da aynı yolu kullanıyordu. Son çizimi
bekleyen değerle kapatmanın komutu açık bıraktığı otomatik testte de
doğrulandı. Seçim oturumu ayrıca başlangıç/bitiş noktalarını, yarım seçim
kutusunu ve tutamaç taslağını tutar; yalnız `Session::exit` bunları temizlemez.

Yerinde yazı ve paragraf düzenleyicileri de nesne slotu tutar. Belge
sınırında bırakılmazlarsa aynı slot numarasını taşıyan yeni çizime geç
bir Kaydet cevabı gidebilir. Aynı nesne kimliklerini kullanan iki belgeyle
eklenen regresyon, düzenleyicinin eskiden açık kaldığını gösterdi.

## Araştırma ve KentOS kararı

- [QCAD Escape](https://www.qcad.org/en/manual/scripts/edit/esc/doc/esc/)
  çok adımlı araçta Esc'nin önceki aşamaya dönebildiğini açıklar.
- [QCAD Reset / Idle](https://www.qcad.org/en/manual/scripts/reset/doc/reset/)
  bütün etkin araçları sonlandırmayı ayrı bir işlem olarak tanımlar.
- [AutoCAD komut temelleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-OnBoarding/files/ACD_FOUNDATIONS_MAIN4.html)
  komut girdisi, seçenek, iptal ve geri almayı ayrı davranışlar olarak anlatır.

Bu kaynaklar adım geri alma ile tam sonlandırmayı ayırma kararını destekler;
rakiplerin belge değiştirme iç uygulaması biliniyormuş gibi yorumlanmaz.
KentOS kararı: belgeyi bırakmak **bütün onaylanmamış taslakları bırakır**;
normal Esc adım geri almaya devam eder. Önceden uygulanmış işlemler,
belgenin seçimi, kamera ve Undo geçmişi sekmenin kendi verisi olarak korunur.

## Uygulama

- `Session::leave_document`: araç, askıdaki üst komutlar, kenar/referans/
  yapım/izleme isteği, seçim kutusu ve tutamaç taslağı bırakılır. Eski
  imleç, yutulacak bırakma ve komutun geri dönme işareti de temizlenir.
  Son komut ve çalışma sayacı korunur; tekrarlama yeni bir araç başlatır.
  `Select::reset` kullanılır; mimaride kalıcı nesne kimliğiyle tutulan
  geçici ölçü referans tercihleri için mevcut oturum politikası korunur.
- `App::leave_document_interaction`: değer alanı, kenet ve kenet çevrimi,
  bir kerelik kenet, imleç koordinatı, izleme noktaları ve kilitler; sahne
  menüsü, hover/ret kartı ve yarım komut/ölçek girdisi temizlenir.
- Yazı/paragraf alanları **yazmadan** bırakılır. Bekleyen odak/seçim isteği
  ve çift tıklama kaydı temizlenir; açık düzenleyicinin klavyesi bırakılır.
  Normal bir ribon komutuna geçerken mevcut yazıyı blur ile kaydetme
  davranışı değiştirilmez.
- Belge açma, sekmede bekletme ve kapatma bu tek yolu kullanır. Birkaç kez
  Esc çağıran, aşama sayısına bağlı bir döngü eklenmez.
- Geometri yazan yeni bir yol yoktur. Ürün komutları, Python/MCP çağrıları,
  önceden var olan geç dosya açma/kaydetme yanıtlarının belge kontrolleri ve
  ortak Undo sistemi kullanılır. Bu dilim bütün asenkron yanıt ailelerinin
  yeni baştan denetlendiği anlamına gelmez.

## Doğrulama

- Yeni belge açma ve son çizimi kapatma regresyonları düzeltmeden önce
  başarısız oldu. Yazı alanı regresyonu da ek temizlemeden önce açık kalan
  düzenleyiciyi yakaladı. Son halinde yazı ve paragraf için geç Keep,
  aynı slot/kimlikteki başka belgenin metnini ve snapshot'ını değiştirmiyor.
- `cargo test -p kentos-interaction --test all --quiet`: **832 geçti,
  2 isteğe bağlı atlandı**. Yeni iki kontrol: yarım seçim kutusunun geç
  bırakması seçmez; tutamaç önizlemesi belgeye veya Undo'ya yazılmaz.
- `TMPDIR=/tmp cargo test -p kentos-desktop --lib --quiet`:
  **1551 geçti, 1 başarısız, 256 isteğe bağlı atlandı**. Belge/sekme,
  düzenleyici, açma/kaydetme, menü, semantik komut ve bütün etkileşim
  senaryolarının üç klavye/ekran çeşidi bu çalışmada geçti.
- Tek kalan hata `temporary_scenes::tests::temporary_distances_go_to_grids_and_lengths_as_the_pictures_show`:
  2 kare beklentisine karşı 4 kare. Belge sınırındaki üç çağrı eski
  `cancel` yoluna alınarak aynı test tekrar çalıştırıldı; **aynı dört
  koordinatla yine başarısız oldu**, sonra yeni çağrılar geri kondu.
  Dolayısıyla bu hata bu dilimle çözülmüş veya bütün takım geçmiş sayılmaz.
- İlk geniş çalışmadaki Unix soket yolu testi uzun macOS geçici dizininde
  başarısızdı; kısa `TMPDIR=/tmp` ile geçti. Ürün bağlantı kodu değiştirilmedi.
- Önceki dilimlerin yeni Enter açıklamaları için **18 mevcut fixture'da
  67 prompt/options beklentisi** güncellendi. Geometri, sayılar, seçim,
  Undo ve diğer beklentiler JSON karşılaştırmasıyla aynen korundu;
  yeni onay metinleri eski kayıtlardan dolayı testleri düşürmüyor.
- `cargo build -p kentos-desktop --bin kentos-cad`: başarılı.

Gerçek macOS penceresinde yaklaşık 1100×680 mantıksal piksel:

1. Aynı Tarama sayı aşamasında başka dosyayı açma: yeni çizimde komut yok,
   12 nesne korunuyor. Önce/sonra ayrı görüntüler.
2. Taşı hedef önizlemesi sırasında sekme değiştirme: diğer çizime gölge
   geçmiyor; kaynakta geometri ve seçim korunuyor, komut yeniden başlamıyor.
3. Tutamaç önizlemesi sırasında gidip gelme: sonraki sıradan tıklama
   köşeyi uygulamıyor; kaynak 2 nesne ve ilk geometrisiyle kalıyor.
4. Yerinde Yazı alanına onaysız metin yazıp sekme değiştirme: alan kapanıyor,
   hedef 12, kaynak 2 nesne; metin hiçbirine eklenmiyor.
5. Yeni çizgi çizme: 2 → 3. Diğer sekmede Undo hiçbir şeyi değiştirmiyor;
   kaynak sekmede tek Undo 3 → 2. Yeni komut ve belge geçmişi çalışıyor.
6. Tarama sayı aşamasında normal Esc: araçta bölge seçimine geri dönüyor.
   Bekleyen değerle sekme kapatma ve son çizimi kapatma: komut kalmıyor.

Ham görüntüler, günlükler ve metin beklentilerinin değişim listesi Git dışında
`/tmp/kentos-cad-advanced-2026-11-01/part-4/` altındadır. `01`–`02` hata;
`03`–`04` düzeltilen dosya açma; `05`–`09` taşıma/tutamaç;
`10`–`12` yazı; `13`–`15` bağımsız Undo; `16`–`18` Esc/kapatma.

## Açık kalan

CAD-ADV-01 [~] kalır. İşletim sistemi odağı kaybolurken basılı fare
düğmeleri, Alt+Q'nun komut girdisi odaktayken davranışı, diğer asenkron
yanıt aileleri, kilitli/bağlı hedef matrisi ve diğer araç/kiplerin ayrıntılı
kabulü ayrı dilimlerdir. Geçici ölçü testindeki mevcut uyumsuzluk da açık
olarak kaydedildi; beklentisi bu değişikliğe uydurulmadı.
