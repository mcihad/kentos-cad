# ADR 0842 — Tam elipste sürüklenen ekseni açık hedef olarak tut

2026-11-02. CAD-ADV-05; bu dilimin kabulü tamamlandı. Madde bütünü henüz açık.

## Problem ve dayanak

Bir elipste merkez ve Start arasındaki uzaklık ölçüyle tutuluyorsa kısa
ekseni büyütmek mümkündür: uzun eksen küçülerek aynı ölçüyü sağlayabilir.
Eski `update` hedefi, sürüklenmeyen Start/End dikiş noktasını da tutuyordu;
bu fazladan koşul mümkün bir düzenlemeyi çelişki diye reddediyordu.

[AutoCAD constrained grip editing](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-D3DE4062-EAFB-4CB3-9111-2EF19AD4C2D3.htm)
tutamaçla düzenlerken kalıcı bağların korunmasını ve serbest kalan
boyutların uyum sağlamasını anlatır. [ConstrainedEllipse eksenleri](https://help.autodesk.com/cloudhelp/2026/ENU/OARX-ManagedRefGuide/files/OARX-ManagedRefGuide-Autodesk_AutoCAD_DatabaseServices_ConstrainedEllipse_MinorRadius.html)
çözücüdeki eksen kimliği ile geometrik major/minor normal biçimini ayırır.
KentOS'ta major/minor normal biçimi korunur; sürüklenen kaynak eksen
normalleştirilmiş hedef baza açıkça eşlenir. İndeksin aynı fiziksel eksen
olduğu şekil farklarından tahmin edilmez.

## Komut ve çözüm

- Mevcut `cad.entities.edit operation=grip` içinde yeni değişiklik:
  `ellipseAxis {uid,index,at}`. Kaynak elipste 0:+major, 1:+minor,
  2:−major, 3:−minor. `at` dünya koordinatında metredir; proje gösterim
  birimiyle karıştırılmaz. Kaynak UID/sürüm/kilit kuralları korunur.
- Komut tam elipsi ve indeksi doğrular, mevcut `move_grip` ile geometriyi
  üretir ve +90° baz değişiminde hedef ekseni eşler. Sıfır eksen,
  sonlu olmayan hedef, kısmi elips veya başka işlem türü açıklamalı reddedilir.
- Çekirdeğe yalnız o çağrının geçici eksen hedefi verilir. Aynı beş elips
  bilinmeyeni ve dual türevlerle iki konum denklemi üretilir. Diğer yarıçap
  ve bağların serbest bıraktığı hareketler kilitlenmez. Sürüklenmeyen
  dikiş artık geçici hedef yapılmaz; kalıcı Fix hâlâ önceki konumunu tutar.
- Geçici hedefin kalıcı kısıt kimliği yoktur. Kabul edilen geometri ayrıca
  geçici hedefler çıkarılarak teşhis edilir; UI/query serbestlikleri aynı
  kalıcı bağları sayar. Yerel orijin, mevcut tolerans, saklama doğrulaması
  ve izleyen nokta kanonikleştirmesi aynı çözücüde kalır.
- Açık niyet komutun kaydedilmiş girdisidir. UI önizleme/commit, Python ve
  MCP aynı işlemi kullanır. İç içe değişiklik kümesinde niyet dış komutun
  çözümüne taşınır; aynı nesnede son geometri düzenlemesi geçerli niyeti belirler.
- Masaüstünde nesne başına tek tam elips eksen tutamacı bu yolu kullanır.
  Merkez/kısmi yay, kopya ve aynı elipsin çoklu sıcak tutamaçları mevcut
  davranışını korur. Çoklu sıcak eksenler için yeni serbestlik vaadi yoktur.
- Web komutu aynı çekirdekle yeni girdiyi geometriye indirger. Bu, webdeki
  mevcut parametrik çözücü kapsamını genişlettiği iddiası değildir.
- Kalıcı eksen bağı veya yeni KCAD alanı eklenmez. Eski genel `update`
  girdisi ve eski dosyalar yeniden yorumlanmaz. Kalıcı bağımsız eksen
  başvurusu ayrı bir tasarım işidir.

## Kabul senaryosu

4/2 m, dikişi 0.6 rad olan elipste merkez Sabit; merkez–Start uzaklığı
3.4891469627504486 m; Start'a bağlı çizgi. Kısa eksen 3 m hedeflendiğinde
uzun eksen uyum sağlamalı, üç bağ korunmalı ve kalıcı teşhis dört serbestlik
saymalıdır. 1.5 m hedefiyle eksen sırası değişimi de aynı ölçüyü korumalıdır.
Dört işaretli eksen ve büyük koordinat, gerçek Fix çatışması, salt okunur
plan, iç içe komut, kaydet/aç ve tek Undo denetlenir.

`fixtures/interaction/v1/constraint-ellipse-axis.kcad` bu kabulün 2.2 KB
belgesidir. `tools/kcad/kcad.py dump` ile bağımsız okunabilir. Görsel değildir.

## Doğrulama

- Çekirdek tam paket: 594 birim + 314 bütünleşme + 2 çağrı testi;
  geçersiz geçici hedef için eklenen testle son odaklı elips paketi 18/18.
- Application 317; interaction elips 11; commands 1057; MCP 48;
  Python elips 16; masaüstü kitaplık 1582 test geçti. Paketlerin mevcut
  ignore kapsamı değiştirilmedi. Beş ilgili crate için tüm hedeflerde
  Clippy, masaüstü/Python/WASM derlemeleri ve SDK üretim kontrolü geçti.
- Webde ilgili dokuz test geçti. Tam TypeScript kontrolünde HEAD ile
  karşılaştırılan aynı 43 mevcut tanı var; bu değişiklik yeni tanı üretmedi.
  Bu sonuç tam TypeScript kontrolünün başarılı olduğu anlamına gelmez.
- Gerçek Mac penceresinde büyük koordinatta kısa eksen 2→3 m tek Enter'la
  kabul edildi; uzun eksen 3.6959145199192953 m'ye uyum sağladı. Üç bağ
  geçerli, kalıcı teşhis dört serbestlik. Fare önizlemesi de aynı bağlı
  geometriyi gösterdi; Esc dosyayı hiç değiştirmedi.
- Kaynak uzun ekseni 1.5 m'ye indirirken eksenler yer değiştirdi;
  yeni uzun yarıçap 5.7773377801527435 m, kısa yarıçap 1.5 m.
  Dikiş uzaklığı ve bağlı çizgi korundu. Yeniden açılan dosyada üç
  geçerli bağ/dört serbestlik ve aynı geometri görüldü.
- İki GUI sonucu aynı başlangıçtan Python komutuyla üretilen dosyalarla
  bayt bayt aynı. İki işlemde tek Undo başlangıç baytlarını tam getirdi.
  Bağımsız f64 dünya koordinatı hesabında uzaklık farkı 0.2 nm'den küçük;
  gösterim yuvarlamasına dayanarak başarı kabul edilmedi.
- Dikiş noktasına ayrıca kalıcı Fix koyulduğunda aynı kısa eksen hedefi
  tek Enter'da `constraint_conflict` ile açıkça reddedildi. Komut geçmişi
  hiçbir şey yazılmadığını gösterdi; farklı kaydedilen dosya başlangıçla
  bayt bayt aynı. `validate`, `plan` ve `execute` bu çatışmada aynı ret verir.
- Dört işaretli eksen, eksen sırası değişimi, iki orijin, geçersiz indeks/
  nesne/NaN, iç içe komut, Python JSON replay ve MCP kabulü otomatik
  testlerde kapsandı. Bir genel `update` girdisinden eksen niyeti çıkarılmaz.

Ekranlar ve ham sonuçlar repo dışındadır:
`/tmp/kentos-cad-adv04-2026-11-01/137-…143-*.png`,
`ellipse-axis-intent-gui-acceptance.json`,
`ellipse-axis-intent-clean-results.json`. Bunlar ürün varlığı değildir.

Birleşik teknik plaka, kilit/gevşetme ve çözüm dalı kabulü tamamlanmadan
CAD-ADV-05 kapatılmaz. Kalıcı bağımsız eksen başvuruları, genel elips
teğeti ve web çözücü kapsam genişlemesi bu teslimde uygulanmadı.
