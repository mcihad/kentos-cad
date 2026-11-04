# ADR 0746: Boyutlandırılan, tam ekran yapılan ve boyunu hatırlayan pencereler

- **Durum:** kabul edildi (2026-10-29); köşe tutamacının görünüşü [ADR 0804](0804-one-control-height-cells-and-frames.md) ile değişti (ayrı işaret yok, çerçevenin kendisi).
- **İstek:** sahibin isteği:
  > pencere bileşenimizi genişletilebilir büyütülebilir yap … büyüttük pencere o büyüklüğü saklasın, bir sonraki
  > açılmasında aynı büyüklükte açılsın … tam ekran yapılabilsin, arkasındaki pencereyi doldursun ama kenarlarda
  > birazcık boşluk kalsın
- **Ek:** form pencereleri “daracık” açılıyordu.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Masaüstünün bütün pencereleri (200'den çok) KentOS UI'ın `Dialog`'udur. `Dialog` genişliği sabit (12 piksellik yazıya
göre), yüksekliği içeriği kadar ya da `max_height` kadar olan bir kutudur. Kutu ortaya konur ve kullanıcı boyunu
değiştiremez. Büyük formda gövde 380 piksellik bir kaydırma alanına sıkışıyordu.

Başvurulan yollar (29 Ekim 2026'da okundu):

- QGIS'in `QgsGui::enableAutoGeometryRestore`'u: pencerenin konumu kapanınca saklanır ve açılınca geri yüklenir. QGIS'in
  iletişim kutuları boyutlandırılır ve kullanıcının ayarlarında (`QgsSettings`) boyunu tutar.
  <https://qgis.org/pyqgis/master/gui/QgsGui.html>
- Masaüstü pencerelerinin alışkanlığı: kenardan ve köşeden boyutlandırma, başlığa çift tık ile büyütme ve önceki boya
  dönme.

## Karar

### 1. Her `Dialog` boyutlandırılır ve tam ekran yapılır

KentOS UI'ın yeni `Sizable` bileşeni (`widget/sizable.rs`) kutuyu sarar:

- **Kenar ve köşe:** kutunun kenarlarının içindeki 6 piksellik bant ve köşelerin 16 piksellik kısmı fareyle çekilir;
  imleç yönünü gösterir. Kutu ortada kaldığı için iki yana birlikte büyür ve kenar imleci izler. Sağ alt köşede iki
  kısa çizgi çekilebilir köşeyi gösterir; çekilirken vurgu rengine döner.
- **Tam ekran:** başlığın sağındaki düğme (⤢, tam ekrandayken ⤡) ya da başlığa çift tık kutuyu arkadaki pencereyi
  dolduracak boya getirir. Kenarlarda 16 piksel boşluk kalır. Yeniden basmak önceki boya döndürür.
- **Sınırlar:** en küçük boy 280 × 160'tır (yazı boyutuyla büyür). En büyük boy pencere eksi boşluktur; pencere
  küçülünce kutu da sığar.

`Dialog::fixed()` kutuyu boyutlandırılmaz bırakır; kısa sorular içindir. `Confirm` zaten ayrı bir kutudur ve
değişmez.

### 2. Boy pencere başına ve kişiye göre hatırlanır

Kutunun boyu ve tam ekran olup olmadığı anahtarıyla tutulur (`sizable::sizes`). Anahtar öntanımlı olarak başlıktır;
başlığı değişen kutu `Dialog::key` ile tek anahtar verir. Örneğin Formla düzenle'nin anahtarı kaydedilmemiş iş işareti
“●” olmadan formun adıdır, Formlar'ınki ise yalnız “Formlar”dır.

Boylar yazı boyutuna bölünmüş tutulur, yazı büyüyünce kutu da büyür.

Masaüstü kayıtları kullanıcının ayar klasöründeki `pencereler.json`'da tutar (`dialog_sizes.rs`):

- açılışta okur;
- bir boy değişince sonraki mesajda yazar;
- yanındaki dosyadan yeniden adlandırarak yazar, böylece yarım dosya kalmaz;
- okunamayan dosya ya da sayı olmayan boy atlanır; pencere o zaman kendi boyunda açılır.

### 3. Boyu verilmiş kutu gövdesini büyütür

Hatırlanan boyu olan kutu `Fill` ile o boya yerleşir. `Fit`'e verilen yükseklik içeriğinden fazlaysa kalan yer son
esnek çocuğa (kayan gövde) gider. Esnek çocuk yoksa boşluk son çocuğun (düğmeler) üstüne konur. Böylece düğmeler hep
altta durur.

Formların pencereleri bunu kullanır:

- **Formla düzenle:** sayfaları kutunun kalan yerini alır (`Fill`). Öntanımlı boyu 620 × 380 yerine 760 × 460'tır.
- **Formlar (tasarımcı):** üç bölmesi kalan yeri alır.

Öbür pencereler bu dilimde kendi iç sabit yüksekliklerini korur: kutu büyür, `Fill` olan parçaları onunla büyür.

## Doğrulama

- `cargo test -p kentos-ui --features snapshot --lib -- sizable fit`:
  - gerçek fare olaylarıyla kenar 100 piksel çekilince kutu 200 piksel büyür;
  - tam ekran düğmesi kutuyu pencere eksi 16 piksel yapar, ikinci basış önceki boya döndürür;
  - `Fit` verilen yüksekliği kayan gövdeye verir, düğmeler altta kalır.
- `cargo test -p kentos-desktop --bin kentos-cad dialog_size -- --include-ignored`:
  - Formla düzenle ve Formlar fareyle köşeden büyütülür ve tam ekran yapılır;
  - `pencereler.json`'un metni gidip gelir;
  - uygulama yeniden kurulunca pencere aynı boyda açılır.
  - Resimler `.run/shots/pencere-*` (1440 × 900).
- Masaüstünün bütün testleri (1388) ve KentOS UI'ınkiler (292) geçer.

## Kalan

- Öbür pencerelerin iç sabit yükseklikleri (listeler, tablolar) kutuyla büyüsün: dokunuldukça.
- Kutunun yerinin (ortadan kaydırma) saklanması: kutular modal ve ortadadır.
- Web: aynı davranış TODOS.md §0.2'de.

## Ek (4 Kasım 2026): büyütülmüş kutuda düğmeler alt kenarda

Sahibin sözü: "pencereler genişletildiğinde veya tam ekran yapıldığında alttaki butonlar aşağı doğru insin, yani
footer olsun."

- **Kök neden.** Kutuyu ortalayan modal kap çocuğuna gevşek sınır verir, yani en küçük yükseklik 0'dır. `Fit`'in
  "boyu verilmiş kutu" kuralı en küçük yüksekliğe bakıyordu. Bu yüzden büyütülmüş kutuda hiç işlemedi: düğmeler gövdenin
  hemen altında kaldı, altta boş bir alan açıldı.
  - Dever'de düğmelerin dibe yakın görünmesi tesadüftü: formu uzun olduğu için gövde zaten pencereyi dolduruyordu.
- **Karar.** Kutunun tutulan bir boyu varsa (`kept`: elle büyütülmüş ya da tam ekran), `Dialog` içindeki `Fit`'e
  `fill(true)` verilir, `Fit` de verilen en büyük yüksekliği hedefler.
  - Artan yer son esnek parçaya gider: `scroll` ile verilen gövde büyür.
  - Esnek parça yoksa ve kutunun alt çubuğu (`action`/`aside`) varsa, artan yer düğme satırının üstüne gider.
  - Alt çubuğu olmayan kutuda hiçbir şey aşağı itilmez: düğmeleri gövdesinin içinde olan eski pencerenin gövdesi dibe
    kaymaz.
- **Pencerelere düşen.** Düğmeler `Dialog::action`/`aside` ile verilir, büyüyebilecek gövde `scroll` ile verilir.
  Gövdenin içine konan düğme satırı alt çubuk olmaz. Ölçüm (1920×1080, tam ekran): Enkesit'in düğmeleri ilk görüntüde
  pencerenin ortasında (y ≈ 560) asılıydı; alt çubuğa taşınınca alt kenardan 30 px yukarıdalar (iç pay 18 px).
  Genişletme önerisi ve Dever de aynıdır.
- `Dialog::footer(satır)`: pencerenin kendi düğme satırı (arasında katman seçicisi olan) alt çubuk olur, `aside`/`action`
  gibi davranır.
- 4 Kasım'da düğmeleri gövdede olan 105 pencere sayıldı. 21 Yol penceresinin düğme satırı alt çubuğa taşındı:
  Enkesit, Boykesit, Güzergâh, Koridor, Brükner, Arazi yüzeyleri, Otokorkuluk, Menfez, İstinat duvarı, Platform,
  Araç izi, Drenaj, Yüzey analizi, Aplikasyon, Güzergâh arama, Kavşak, Görünüşler, Paftalar, Nokta bulutu, Hakediş,
  aks ve boykesit uydurma.
  - Düzenleyicisi sabit yükseklikli kendi kaydırma alanında olanlar (Enkesit 360, Boykesit 440, Güzergâh 420,
    Koridor 330, Brükner ve Arazi 360 px) artık gövdeyle birlikte kayar. Böylece tam ekranda büyürler; normal boyda da
    Boykesit'in PVI tablosu iç kaydırma olmadan görünür.
  - Çizim satırları (Çizime, Km yazısı, Eş yükselti) gövdeden çıkıp alt çubuğun hemen üstüne alındı.
  - Aynı gün Yol dışı pencereler de taşındı. Düğme satırı gövdenin sonunda olan altısı alt çubuğa geçti (olay bağları,
    birleşimler, katman dizinleri, tablo stilleri, katman kuralları, ilişkiler), ayrıca form denetimi ve bina
    programı (iç kaydırması 380 px'ti). Gövdesi zaten esnek (`fill`) olan form pencerelerinin son satırı ve
    `.push(footer(..))` zincirli 13 pencere `footer`'a çevrildi (işleme toplu, tasarımcı, stil yöneticisi, SVG
    düzenleyicinin altı penceresi, katman stili, ifade, 3B sahne).
  - Hesap pencereleri (kutupsal, kestirme, poligon) zaten `action` kullanıyordu. Kalanlar tuval çizimidir ya da
    pencere değildir. Başlangıç ekranı kendi düzenini korur: boyutlanmaz, alt satırı bir anahtardır.

Bekçi: `widget::fit::tests::a_kept_box_fills_loose_limits_and_its_footer_goes_down`.

