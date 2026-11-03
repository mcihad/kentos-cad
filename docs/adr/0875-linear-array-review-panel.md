# ADR 0875 — Doğrusal dizi adaylarını çizimi değiştirmeden incele

Tarih: 3 Kasım 2026. CAD-ADV-07'nin masaüstü inceleme dilimi.

## Dayanak ve tasarım kararı

[BricsCAD V27 ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
seçilen kaynaklar, bulunan örüntüler ve ayrı dönüştürme eylemi sunar; tek tek
incelemede görünüşü adaya yaklaştırır. [AutoCAD for Mac 2027 Detect and Convert](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-MAC-WhatsNew/files/GUID-0876F8BC-9ED5-4EBB-A19C-FF019BAA3510.htm)
galeri, bulunan örneklerin vurgulanması ve ayrı inceleme akışı kullanır.
Bu ürünler kullanıcı akışının dayanağıdır; KentOS'un doğrusal eşleme
algoritmasının veya kapsamının onlarla eşit olduğu iddia edilmez.

KentOS Bloklar panelinde mevcut bileşen aramasının yanında **Dizileri bul…**
sunar. Aynı iş `array.find`, `DIZIBUL`, `DİZİBUL` veya `FINDARRAYS` ile açılır;
şeritte mevcut Blok grubunun taşma menüsünde bulunur. Yeni bir şerit grubu
veya sürekli açık büyük düğme eklenmez.

## Arama ve inceleme

Çizimde tek statik blok yerleştirmesi kaynak seçilir. Ayar penceresi yalnız
pozitif tolerans ve gizli/donmuş katman kapsamını gösterir. Tolerans açıkça
milimetredir: varsayılan 0,001 mm, en çok 1 mm; proje ölçeğini değiştirmez.
Kaynak yoksa kullanıcıya seçim nedeni gösterilir, Bul pasiftir.

**Bul**, ortak `cad.arrays.find` sorgusunu arka planda çalıştırır. UI'nin
ayrı geometrik eşleyicisi yoktur. Görünürlük filtresiyle seçilen aynı blok
kökleri açık `uids` girdisi olarak gönderilir; 2 048 sınırının üstü kesilip
tam tarama gibi sunulmaz. Python işlem çıktısı ayarı açıksa aynı sorgu ve
girdiler konsola yazılır. Arama çizimi ve Undo geçmişini değiştirmez.

Panel adayın aralığını, dolu/boş yer sayılarını ve kısa bir yer şemasını
sunar. Şema geometrik resim değildir: ● mevcut, ○ boş, ◇ yerinde korunan
öğedir. **Göster** adayın mevcut köklerini seçip yakınlaştırır; **İncele…**
bağımsız bağlama denetimini başlatır. Eksik yerler inceleme sırasında veya
onayda kendiliğinden doldurulmaz. Kesişen adaylar ortak öğe uyarısı taşır.

## Elle taşınmış öğe

Uygun ancak seçilen adayda kullanılmayan bloklar açık bir seçim listesidir.
Kullanıcı **Elle taşınmış öğeyi koru…** bölümünde bloğu, 1 tabanlı boş yeri
ve **Bu yere koru** eylemini seçer. Liste görünür nesne numarası ve konumla
anlaşılır; UUID yazmak gerekmez. **Bloğu göster** mevcut nesneyi yakınlaştırır.
Kaynak yer 1 ve dolu yerlerin üstüne yazılmaz. Atama yalnız taslaktır;
her değişim ortak bağlama denetiminden yeniden geçer. Yer ataması ayrı
adlı düğmeyle kaldırılır; pencereyi kapatan Vazgeç ile karışmaz.

Yerinde tutma, sapmayı toleransla gizlemek değildir: öğe açık `kept` kaydı
alır ve sonraki dizi düzenlemelerinde yerinde kalır. Kullanıcının açık
ataması olmadan eşleşmeyen hiçbir kök korunmuş öğeye dönüştürülmez.

## Onay, iptal ve belge değişimi

**Diziye bağla**, yalnız geçerli inceleme planında etkinleşir. Ortak native
Job `cad.arrays.adopt` üzerinden metadata yazar; kökler, blok tanımı, UUID'ler,
ölçü referansları ve proje ayarları korunur. Tek Undo/Redo'dur. Geçersiz yer,
başarısız inceleme veya eski belgeyle önceki geçerli plan uygulanamaz.

Arama/inceleme çalışırken Durdur; panel/pencere kapanınca iptal vardır.
Her işin tekil belirteci ve iptal bayrağı geç dönen sonucu ayırır. Belge
oturumu ve generation denetlenir: değişikliği Undo ile geri almak eski
sonucu yeniden geçerli kılmaz. Yazma işi ayrıca native plan/revizyon
korumasını kullanır. Başarısız/durdurulmuş inceleme **Tekrar incele** ile
kurtarılır; bütün işlemi baştan kurmak zorunlu değildir.

## Kabul ve açık kapsam

Otomatik kabul: gerçek sorgu/inceleme cevabı, elle atama/kaldırma, Job
uygulama, kök ve bağlı ölçü eşitliği, tek Undo/Redo, iptal, geç cevap,
belge değişimi ve Undo sonrası eskilik, geçersiz yerler, ayarlardan vazgeçme,
eski bileşen aramasına geçiş, komut adları ve Python çıktısı ayarı.
Gerçek pencere kabulünde fareyle arama, aday seçme/gösterme, eşleşmeyen blok
listesi, bloğa yakınlaşma, atama/kaldırma/yeniden atama, vazgeçme, bağlama ve
Undo/Redo kullanıldı. 2200×1364 pencerede açık/koyu tema görüldü; dar dock'taki
kırpılmış başlık sadeleştirilip tekrar kontrol edildi. Gerçek DIZIBUL girişi
aynı pencereyi açtı. Son ikilide bağlama/Undo/Redo ve aralık değişimi tekrarlandı.
Bağımsız KCAD okuyucusu bütün köklerin, altı UUID'nin, blok tanımının, ayarların
ve bağlı ölçü referanslarının korunduğunu doğruladı. Aralık 2000→3000 mm'de
iki boş yer, yerinde kalan öğe ve 3000 mm bağlı ölçü doğrudur. Vazgeçme ve
Undo dosyaları başlangıçla; Redo dosyası bağlanmış dosyayla birebir aynıdır.

Genel masaüstü koşusunda 1640 başarılı kontrol, bir envanter uyumsuzluğu
vardı. Envanter üreticisiyle düzeltildi; dokuz katalog testi geçti. Son UI
kaynağında 26 panel testi ve masaüstü tüm hedefler Clippy başarılıdır;
manifest 946 satırda tutarlıdır. Ham GUI ekranları ve ikili özetli kabul
manifesti `/tmp/kentos-cad-adv04-2026-11-01/array-panel-gui-acceptance.json`
altındadır; bu yerel geçici kanıttır, repoya resim eklenmez.

Grid, kutupsal ve yol aday bulma; metin/öznitelik/iç blok kaynak genişlemesi
bu dilimle tamamlanmaz. CAD-ADV-07 açık kalır.
