# İlişkili duvar ölçü sıraları — 27 Ekim 2026

**Durum: kabul taslağı; test kapıları bekleniyor.** Bu kayıt ARCH-DIM-01'in
bir dilimidir. Otomatik ölçü sıraları, mimarinin tamamı veya bütün CAD/GIS
ürünü kabul edilmiş sayılmaz. `featureAccepted` ve
`wholeArchitectureAccepted` değerleri [evidence.json](evidence.json) içinde
`false` kalır. Kaydedilen seri native turunda 3.645 başarılı/43 ignored,
son gerçek PostGIS turunda yedi başarılı test vardır. Son tam masaüstü turu
1.341 başarılı/0 başarısız/228 ignored; UI turu 289 başarılı/0 başarısızdır.
Taze Python native tam takımı 366 başarılı/0 başarısızdır. Önceki 366 testlik
turun iki başarısızlığı ve kesilmiş turlar kayıttan çıkarılmamıştır. API
health hedefi bir başarılı testtir; bütün API takımı değildir. Build-4 ve
kalan ortak kapılar beklenir;
Live-3'ün yalnız aşağıda listelenen akışları doğrulanır. Tamamlanmamış
kapılara başarı yazılmaz.

Karar ve veri modeli [ADR0695](../../adr/0695-associative-wall-dimension-sets.md),
açık plan [architecture.md](../../../plan/architecture.md) içindedir. Çalışma
`main` üzerinde, dondurulmuş GUI kayıtlarının tabanı
`b793dd770c85d7357b0e7f2ad6277600df79f81d` ve o anda kaydedilmiş kaynak
karmalarıyla tanımlanır. Bu taban tek başına yeni üretim değişikliklerini
veya güncel çalışma ağacını doğruladığı anlamına gelmez.

## Bu taslakta bağımsız doğrulanan model davranışı

[models/](models/) altında bağımsız beklentisi doğrulanmış **42 tam KCAD**
(192.823 bayt; biri GUI öncesi bilinen uzun stil seed'i) ve ayrıca dört korunmuş Live-3 model kopyası bulunur.
Ek dört kopyanın yalnız byte/SHA/bağımsız KCAD okuması doğrulanır; geometrik
vaka kabulüne henüz eklenmez (`decodedSupplementalModels`). Beklenen aralıklar bağımsız
[tools/kcad/kcad.py](../../../tools/kcad/kcad.py) okuyucusuyla, 10 m duvar ve
bilinen açıklık ankraj/enlerinden hesaplanır. Uygulama, komut planı veya
native Python uzantısı beklenen geometriyi üretmek için kullanılmaz.

| Kayıt | Denetlenen somut sonuç |
|---|---|
| 22, 23, 24 | Dört ölçülü tek grup; geri alma üç kaynak nesnesini ve tarifsiz başlangıcı getirir; yeniden yap tam 22 belgesini getirir |
| 28 | Pencere eni 200 cm: 3 / 2 / 5 / 10 m; tarif, UUID'ler ve sıra yerleri korunur |
| 29 | Açıklığı silme: ilgisiz çizgi/duvar korunur, açıklık sırası yeniden kurulur, genel ölçünün UUID'si kalır |
| 33 | Yeni açıklığın gerçek serbest tıklama ankrajı 8,000242248062015 m'dir; aralıklar bu değer ve 1 m enden hesaplanır; tam 8 m denmiş sayılmaz |
| 35 | Ayrı sayısal konum düzenlemesi 7 m: 3 / 2 / 1,5 / 1 / 2,5 / 10 m; bütün üye UUID'leri ve tarif korunur |
| 40 | Genel sıranın uzaklığı gerçek sol yüz y=0,1'den 3 m: dünya çizgisi y=3,1; açıklık sırası değişmez |
| 41, 44 | Tek üyeyi silme tam bir anlamsal aralığı bastırır; geri getir tek yeni UUID oluşturur, kalan üyeleri değiştirmez |
| 46, 48 | Merkez modu doğrusal izdüşümleri 4 / 3 / 3 / 10 m; genel UUID ve özel sıra korunur; yeni süreçte açılan tam belge 46 ile aynıdır |
| 51 | Yalnız pafta kitabı eklenir; model/tarif/kimlikler 48 ile birebir; A3 yatay, görünüm 1:50 |
| 58 | İkinci dondurulmuş ikilide kaynak seçimi/önizleme iptali tam başlangıç belgesini korur |
| 64, 65, 66 | Gerçek 100 cm uzaklık, 10 mm kağıt aralığı ve 1:50: offset'ler 1 / 1 / 1 / 1,5 m, dünya sıra çizgileri 1,1 ve 1,6 m; geri al tam 58, yeniden yap tam 64 |
| 70 | Sabit yerden imleç kipine geçip önizlemeyi iptal ettikten sonra tam belge 66 ile aynıdır |
| 73, 76 | 1:100 kaydı dondurulmuş özgün fixture ile tam aynıdır; 1:200 kaydında yalnız `settings.plotScale` değişir |
| 78 | Desteklenmeyen çok parçalı kaynak reddi tam 76 belgesini değiştirmez |
| 80 | Yalnız pafta kitabı eklenir; kaynak model/tarifler 76 ile aynıdır; A3 yatay, görünüm 1:200 |
| 83 | Pafta/model ve yakın/uzak görünüm gezinmesinden sonra tam belge 80 ile aynıdır |
| 88, 89 | Bağımsızlaştırma yalnız `dimensionSets` tarifini kaldırır; yedi nesnenin UUID/ref/geometrileri aynı; undo tam 64 belgesini getirir |
| 90, 92 | Duvarı silme yalnız hostu kaldırır; açıklık ve tarif `hostGone`, altı nesne ve son ölçü geometrileri korunur; undo tam 64 belgesini getirir |
| 96, 97 | Bütün grubu silme tam 58 başlangıç belgesini getirir; undo tam 64 belgesini getirir |
| 125, 130 | Gezinme özgün 1:100 belgeyi korur; açık stil adı kaydında yalnız önceden belirtilen ad değişir |
| 137, 141 | Eski ada elle dönüş ve ayrı gezinme tam 125 belgesine eşittir; 137 Undo değildir |
| 153, 154, 155 | Tek Onayla: 100 cm uzaklık ve **varsayılan 7,5 mm** aralıkta offset1/1/1/1,375 m; tek toolbarUndo tam 58, tek toolbarRedo tam 153 |
| 156, 164 | Yeni native süreçte açılan tam belge 155 ile aynı; geniş13 px stilde Projeye al yalnız taslak, Vazgeç+Kaydet sonrası tam belge 156 ile aynı |
| 165 seed, 176 | 125 özgün belgesinden bağımsız tarif edilen 26 stil/16 bileşenli seed; iç/dış kaydırma ve son bileşen taslağı sonrası Vazgeç+Kaydet tam aynı KCAD baytlarını korur |

### Live-3 envanteri ve ayrılan tanılar

- 125 tam özgün 1:100 modele eşittir. 130, önceden yazılmış 129 beklentisiyle
  ve 125 modelinin yalnız açık stil adını `Dış duvar 25 QA` yapma işlemiyle
  tam eşittir. 128 stil iptali tam 125 modelini korur.
- 131/132/133/135, başarısız Undo/Redo denemelerinin tanısıdır. Dosya
  adındaki `undo`/`redo` sözcükleri başarı iddiası değildir. 133 ve 135
  yalnız KCAD kaydıdır; bu numaralara ham ekran atanmamıştır.
- 137, eski ada **elle** geri dönüştür ve tam 125 modeline eşittir; Undo
  kabulü değildir. 136 taslak, 138 mevcut görünüm gözlemidir. 141 gezinme
  kaydı da tam 125 modelini korur.
- 116 dosyası 2852 piksel genişliğinde ara görünümdür; adında `narrow`
  bulunmasına rağmen dar pencere kabulü olarak kullanılmaz.

156 kaydı, önce uygulama kapatılıp süreç kalmadığı gözlendikten sonra açılan
38939 numaralı yeni native süreçte alınmıştır; tam belge 155 ile aynıdır.
Yeniden açılan pencere 2880×1720 ve 18 px geniş görünümüdür. 157–159 bu
ayarın gözlemleridir. 160 yalnız 13 px taslağı, 161 uygulanmış 13 px'tir.
162 geniş13 px hazır stil penceresi, 163 Projeye al düzenleme **taslağı** ve
164 Vazgeç+Kaydet kaydıdır. Bu görünüm/tercih/draft işlemleri ölçü tarifini
ve proje modelini değiştirmemiştir. Mevcut bütün tema/yazı/pencere matrisi
sınanmış sayılmaz.

165 kısayol denemesi ayarları açmamıştır ve kabul değildir. 166 gerçek
KentOS menüsüdür. 167'de kaydırma alanı dışındaki teker hareketi değişiklik
yapmamıştır; 168 alan içinden gerçek kaydırma ile yazı boyutu alanını getirir.
169 bilinen uzun seed'in 18 px geniş görünümde açılmasıdır; 170'te pencere
gerçek 2204×1364'e küçültülür. 172 uzun stil formunu, 173 sol iç listenin
QA24/hazır stillere kaymasını ve sağ formun üst konumunu korumasını gösterir.
174 sağ dış kaydırmanın son `Ek katman 12` alanına ulaşmasıdır; başlık,
önizleme ve alt eylemler görünür kalır. Sol gövdenin bu dış kaydırmada ekran
dışında kalması beklenendir. 175 `SonbileşenQA` adını girip Tab ile çıkılmış
bir **taslaktır**; etkin Kaydet düğmesi kaydın yapıldığı anlamına gelmez.
176 gerçek Vazgeç+CmdS işlemidir. Kaydedilen dosya, GUI öncesinde kaydedilmiş
`89b83eeb2140ebc5fd76c6fecbd354c331020f06fdb8d42e147d7c77ce8b2aad`
seed SHA'sıyla ve bütün baytlarıyla aynıdır. Doğrulayıcı seed'i 125'ten bir
uzun stil, 24 liste stili ve özgün stili bağımsız kurarak da karşılaştırır;
üç özgün kaynak ve diğer bütün proje alanları korunur. Bu vaka stil değişimi
kaydetme veya Undo başarısı değildir.

KCAD dosyası yerel `entity.id` slotlarını yeniden numaralar. Farklı adımlar
arasındaki nesne korunumu **kalıcı UUID ve diğer bütün nesne alanlarıyla**
karşılaştırılır; yalnız dosya yerel slot karşılaştırmadan çıkarılır. Tam
belge eşitliği istenen undo/redo/reopen/ret durumları bu istisnayı kullanmaz.
Kaynak noktaları ve `refs` ayrıca aynı anlamsal yerlere bağlanmalıdır.

Host silme beklentisi, sonuçtan türetilmemiştir: `opening_follow::placed_on`
eksik duvarda açıklığı silmeden `hostGone` yapar; `dimension_sets` follower
hata dalı tarifi/üyeleri ve son geometriyi korur. Artık bulunmayan duvara
giden üye başvuruları `object_gone`, var olan açıklığa gidenler
`no_such_place` olur. 91 ham ekranı görünür kopuk kaynak uyarısının
kısmi GUI kaydıdır; 90 modeli bu durumun tam veri kaydıdır.

153–155 için ilk geçici denetimde yanlış 10 mm aralık beklentisi ve wire
alanı `layerId` yerine `layer` kullanılmıştı. Bu bir üretim kusuru değildir.
Bu gerçek GUI tekrarında aralık değiştirilmedi: belgelenmiş varsayılan
7,5 mm, plotScale50'de 0,375 m sıra farkıdır; ilk offset1 m, genel offset
1,375 m ve dünya sıra çizgileri y=1,1/1,475 olur. Kaynaklar ve ilgisiz proje
alanları tam korunur. Düzeltme `verificationCorrections` ve 155 bağımsız
receipt'inde açık tutulmuştur; kaynak kodu beklentiye uydurmak için değişmez.

Merkez modunda gerçek kaynak merkezleri y=0, duvar yüzü y=0,1 olabilir.
Ölçü stili doğrusal ve açı 0 olduğundan 4/3/3 değerleri duvar doğrultusundaki
izdüşümdür; eğik iki kaynak noktası arasındaki Öklid kirişi değildir.
Dünya uzunlukları metre, `gapMm` kağıt milimetresidir. İlk serbest tıklama
sırası hiçbir zaman sayısal uzaklık girişinin kanıtı diye kullanılmaz.

## Görsel kayıtlar ve dondurulmuş ikililer

Ham görüntü/PDF baytları Git'e kopyalanmamıştır. Yeni görsellerin politikası
[üst kılavuzda](../README.md) açıklanır. Varsayılan yerel kaynak dizini:

```text
.run/gui-evidence/2026-10-27-wall-dimensions/
```

[evidence.json](evidence.json), ilk 183 adımla ilişkili **177 ekranın** adını,
gerçek MIME türünü, piksel boyutunu, bayt sayısını, SHA256'sını ve kapsamını
saklar. `.png` uzantılı dosyaların çoğunun gerçek içeriği JPEG'dir; biçim
uzantıdan değil başlıktan okunur. Ayrı bir PDF çıktısının adı/SHA/bayt
metaverisi de vardır; PDF veya render görüntüsü depoya eklenmemiştir.
Görüntü karmasının doğru olması tek başına görüntünün görünüşünün veya
etkileşimin doğru olduğunu kanıtlamaz.

- 01–09 tarihsel tanı; mevcut üretim kaynağının kabulü değildir.
- 10–12 ayrı stil karşılaştırmasıdır; bunlara kesin GUI ikilisi atanmaz.
- 16 başarısız kaynak seçiminin tanısal kaydıdır.
- 21 ve ilk 22 konumu, eski imleç/sayısal uzaklık kusurunu korur. Bu
  kayıtlar düzeltilmiş uzaklık kabulüne eklenmez. Düzeltilmiş davranış ayrı
  live-2 ikilisinde 64/65/66 ile ölçülür.
- 93–97 açık tema/18 px/dar pencere kısmi gözlemleridir. 98, menü alt
  bölümünün pencere dışına taşması tanısıdır; kabul değildir. Sonraki Live-3
  düzeltme gözlemleri ayrı kayıtlardır; eski ekran başarı diye yeniden etiketlenmez.
- 99 Mimari sekmesinin açık tema/18 px/dar görünümüdür. 100/101 aynı
  görünümde 1:50 stil çiziminin fit/yakın/Home/kaydet gözlemleridir.
  `observations/101-style50-navigation-verification.json` kaydı tekrar
  bağımsız okundu: tam belge özgün 1:100 fixture'ın yalnız `plotScale=50`
  değişimidir. Yeni bir dondurulmuş GUI model kopyası varmış gibi sayılmaz.
- 102/103 stil penceresinin Kaydet/Vazgeç alt bölümünün dışarıda kaldığı ve
  sağ panel wheel'in etkisiz olduğu tanıdır; 104 Esc ile çizime dönüştür.
  ADR0698'in `.scroll(body)` düzeltmesi sonrası ayrı Live-3 geniş13 ve dar18
  uzun form kayıtları vardır; bunlar bütün tema/pencere matrisini kapsamaz.
- Diğer ara ekranlar gözlenmiş kısmi akışlardır; bütün özellik veya tam
  tema/yazı boyutu/pencere matrisi kabulü değildir.

[builds/](builds/) dört metin kaynak/ikili kaydını korur. Live-1 ikilisi
`496758b7624dd7026507938fc71fdb247d506f284c3fdd6becf21951cfd6a03c`,
live-2 ikilisi
`748a754e019f6cd7c7db2756f9601a886d47798986f82829792af69525578533`
SHA256 ile ayrılır. Her iki kayıtta o andaki 159 kaynak dosyasının hash'i
vardır. Eski binary kaydı yalnız tarihsel tanı içindir; kaynak listesi yoktur.
Son Live-3 ikilisi
`65a2fd393468dbb6bfb2a4e526cee97f092aa24cd1a6498119965ca33d379406`
SHA256 ile ayrı dondurulmuştur; 171 kaynak dosyası kaydı içerir.
`--check-current-sources` yalnız bu son build'i esas alır; eski Live-1/2
kaynağını güncel kaynakmış gibi kullanmaz. Mutlak yakalama yolları tarihsel
metaveridir; doğrulayıcı yerel ikilileri manifestteki depo köküne göre
kayıtlı yollarla arar.

Live-3'ten sonra sunucunun v2 kaynak-geometri önbelleği koruması değişmiş ve
yedi testlik gerçek PostGIS turu tamamlanmıştır.
Bu nedenle güncel kaynak karşılaştırmasında sapma **beklenir ve hata olarak
korunur**. Gerçek yeni ikili, Build-4 kaynak kanıtı ve GUI smoke gelmeden
Live-3 karmaları güncellenmez veya bu denetim atlanmaz.

## Yeniden doğrulama ve eksik kanıt

Depo kökünden, hiçbir app/Cargo çalıştırmadan:

```sh
# Tam model + mevcut yerel ekran/PDF/ikili baytları. Eksik tek ham dosya hata verir.
python3 docs/baseline/2026-10-27-architecture-wall-dimensions/verify.py

# Açıkça yalnız 42 tam model ve kayıtlı build proof metaverisi.
# verifiedImages=0; bu sonuç GUI/görsel kabul değildir.
python3 docs/baseline/2026-10-27-architecture-wall-dimensions/verify.py --models-only

# Yalnız manifest ve bulunabilirlik; hiçbir görüntü/model/ikili baytı doğrulanmaz.
python3 docs/baseline/2026-10-27-architecture-wall-dimensions/verify.py --metadata

# Taşınmış yerel ham kanıt dizini.
python3 docs/baseline/2026-10-27-architecture-wall-dimensions/verify.py --evidence-root /tam/yerel/kanıt/dizini

# İstenirse son GUI build'inin bütün kaynak hash'lerini güncel ağaçla ayrıca strict kıyasla.
python3 docs/baseline/2026-10-27-architecture-wall-dimensions/verify.py --models-only --check-current-sources
```

Varsayılan denetim eksik görselleri sessizce atlamaz; `incomplete-evidence`,
eksik sayıları/adları ve sıfırdan farklı çıkış kodu verir. `metadata-only`
ve `models-only` ayrı kapsamlı sonuçlardır, görsel kabul gibi sunulmaz.
Yeni bir klonda yalnız metin/KCAD modelleri bulunur; bu yeni ham ekranlar
hiç Git'e girmediği için eski görüntülerin Git'ten `restore` mekanizmasıyla
geri getirilemez. Yerel arşiv gerekiyorsa açık `--evidence-root` kullanılır.

Kaynak hash'leri güncel ağaçla farklıysa `--check-current-sources` bunu
`source-drift` ve hata olarak bildirir. Kaynakların sonradan değişmiş olması,
eski ikilinin gözlemini güncel kodun GUI kabulüne dönüştürmez. Bu taslak
eski kabul betiklerinin kaynak/hash kontrolünü gevşetmez.

## Biçim denetiminin gerçek kapsamı

[format-scope.json](format-scope.json) ilk dar biçim aşamasında 162 dokunulan
Rust dosyasını HEAD ile salt okunur kıyaslar. O aşamanın 20 yeni dosyasının
tamamı strict rustfmt biçimindedir.
İlk genel denetimde 57 mevcut dosyada fark vardı; 633 hunk aynen eski,
150 hunk yeni/değişmiş bağlamdaydı. Ardından yalnız yeni Git satırlarıyla
kesişen 114 güvenli parça, 35 dosyada dar biçimde düzenlendi. Her dosyanın
tam rustfmt sonucu önce/sonra birebir aynı kaldı; 633 eski hunk korundu.
16 karışık ve değişen satır dışında kalan 20 parça uygulanmadı. Dolayısıyla
40 mevcut dosyanın strict genel farkı sürer; tüm deponun formatı temiz
denmez. Menü sınır düzeltmesi ve MaybeSend dahil korunan üç menü kaynağının
hash'leri değişmemiştir. Mevcut dosyalar bütünüyle formatlanmamıştır.

Son salt okunur envanter [format-current-audit.json](format-current-audit.json)
içindedir: 165 dokunulan dosya, 21 yeni dosya, yeni dosyalarda sıfır strict
biçim hatası ve denetimde sıfır kaynak değişimi. 41 mevcut dosya hâlâ tam
strict biçim karşılaştırmasında farklıdır. Aynı 12 dosyada kalan 36 yeni
veya değişmiş bağlam parçası aşağıdaki kapsamını korur. Son envanterin 672
biçim parçasının 636'sı HEAD'de aynıdır; ilk dar aşamanın 633 parçasına ek
üç parça yeni kapsanan mevcut dosyalardandır. Bu son denetim hiçbir kaynak
yazmamıştır ve tam rustfmt başarısı anlamına gelmez.

Kalan 36 parça aşağıdaki dar kapsamda bırakıldı. “Karışık” sütunu eski
biçim borcuyla birleşen veya eşlenik taşımanın yarısını uygulamadan ayrı
normalize edilemeyen parçadır. “Dışında” sütunu Git'in yeni/değişen satır
aralığına doğrudan temas etmeyen parçadır. Bu tablo, bu alanların biçiminin
tamamının eski olduğu iddiası değildir; geniş format uygulanmayacaktır.

| Dosya/kapsam | Karışık | Dışında |
|---|---:|---:|
| `apps/desktop/src/architecture_settings.rs`: eski `column!` zinciriyle birleşen yeni genişlik | 1 | 0 |
| `native/application/src/lib.rs`: yeni ölçü modülü/exportu, mevcut modül sırası ve komut tuple bağlamları | 2 | 6 |
| `native/application/src/registry.rs`: yeni importların taşınması ve eşlenik dispatch satırları | 2 | 2 |
| `native/application/tests/all/capabilities.rs`: yeni ölçü ret notlarının mevcut sıkışık tuple listesi | 1 | 0 |
| `native/application/tests/all/fixtures.rs`: yeni ölçü importu ve eski import sırası | 1 | 1 |
| `native/application/tests/all/main.rs`: yeni ölçü test modüllerinin modül sırası | 1 | 1 |
| `native/domain/src/lib.rs`: yeni ölçü modülünün modül/import sırası | 2 | 2 |
| `native/domain/tests/all/main.rs`: yeni ölçü test modülünün modül sırası | 1 | 1 |
| `native/interaction/src/lib.rs`: yeni ölçü modülünün eski modül grubuyla sırası | 1 | 1 |
| `native/interaction/tests/all/main.rs`: yeni ölçü test modüllerinin eşlenik taşıması | 0 | 2 |
| `shared/commands/src/lib.rs`: yeni ölçü sözleşmesi modülü/export sırası | 2 | 2 |
| `shared/contracts/src/lib.rs`: yeni kalıcı ölçü/derived modül/export sırası | 2 | 2 |
| Toplam | 16 | 20 |

Tablodaki `native/` ve `shared/` yolları `crates/` altındadır. Menü sınır
hesabı, `MaybeSend`, çalışma mantığı veya eski 633 biçim parçası bu kalan
kapsamı temizlemek için değiştirilmemiştir.

Kök ajanının eşitlediği 33 TypeScript dosyası kanonik ts-rs çıktısıdır.
Üreticinin satır sonu boşlukları global `git diff --check` uyarısı verir;
bu, bu 33 dosyanın üreticinin birebir çıktısından saptığı anlamına gelmez;
üretilen baytlar elle kırpılmamıştır. Bu kapsam, deponun bütün biçim
kontrollerinin temiz olduğu iddiası değildir.

## İlk dilimin geçici grip önizleme sınırı

`select_opening::preview`, anlamsal planın yalnız `openings + added`
Shape'lerini döndürür, `output.followers` deltasını tüketmez; seçimin
çağıranı ayrıca ilk Shape'i alır. Bu yüzden genel seçimin açıklık merkez
tutamacında bağlı ölçülerin eklenen/güncellenen/kalkan geçici görüntüsü
bütünüyle garanti edilmez. Son `openings_edit::execute` anlamsal komutu ve
kalıcı follower güncellemesi doğrudur; bu sınır veri kaybı veya son ölçü
geometrisinin yanlışlığı olarak sunulmaz. Oluşturma/ayrı açıklık düzenleme
aracının follower önizleme yolu, bu eksik grip yoluyla karıştırılmaz.
İlk ADR0695 kabulü bütün grip önizlemelerinin kabulü olarak genişletilmez.

## Kaydedilen native turu ve bekleyen kapılar

[test-runs.json](test-runs.json), gerçek `native-20261026T230528Z.log`
çıkışını 41 hedef ve paket bazında saklar. Çıkış kodu 0; log SHA256
`e7d5445ef961f0d53901859135fa6b98f661a1d9c6684197ea92958947e5e128`.
Komut contracts, commands, domain, native-application, interaction, KCAD,
headless, MCP ve cloud paketlerinin `--lib --tests` hedeflerini birlikte
çalıştırmıştır. Harici stdio MCP oturumu veya gerçek PostGIS turu değildir.

| Paket | Başarılı | Ignored |
|---|---:|---:|
| cloud | 113 | 1 |
| commands | 854 | 0 |
| contracts | 1.093 | 1 |
| domain | 115 | 0 |
| headless | 81 | 1 |
| interaction | 785 | 6 |
| KCAD | 218 | 2 |
| MCP | 78 | 0 |
| native-application | 308 | 32 |
| Toplam | 3.645 | 43 |

Gerçek PostGIS turunda `project_dimension_sets` hedefinin dört testi
başarılı, sıfır ignored/başarısızdır. Log SHA256
`03b3ab9cbc82755b2cd0ea2036dd84f67fb97475088dd1998e70748b490d050a`.
Atomik tarif/üye aktarımı, eski v1 isteğin sessiz kaybı reddi, ölçek/sıra
önbelleği ve v2 sahiplikten sonra eski idempotent isteğin tekrarı denetlenir.
Yalnız bu çalışma için oluşturulmuş `kentos-cad-architecture-qa-751e437a`
geçici kabı kullanılmış ve testlerden sonra kaldırılmıştır; mevcut
5432/5433 veritabanlarında değişiklik yapılmamıştır.
Bu dört testlik kayıt, sonradan eklenen v2 kaynak duvarı/açıklığı ile ölçü
geometrisi tutarlılığı korumasından öncedir. Yeni koruma ayrı yedi testlik
turda sınanmıştır; eski dört test yeni kaynakta yeniden çalışmış gibi sayılmaz.

Son gerçek PostGIS turu `server-20261027T031302Z.log`: yedi başarılı,
sıfır başarısız/ignored, çıkış 0, duvar saati 16,974 sn. SHA256
`9044cf30cac4000d5fc8085dbe2fce2db9ffc8bdf419d2d408547c90eab2f92e`.
Tam tarifle gönderilse bile eski kaynak geometrisinin reddi, eklenen/silinen
açıklıklarda aralık üyeliği, dönük/büyük koordinatlı sağ yüz ve merkez
geometrileri, plotScale satır önbelleği ve eski idempotent tekrar sınanır.
Kök ajan kendi geçici DB kabının kaldırıldığını, mevcut 5432/5433
veritabanlarının değiştirilmediğini bildirmiştir.

Önceki `server-20261027T012044Z.log` çıkış 101, altı başarılı/bir
başarısız olarak korunur. Dönük büyük koordinat fixture'ı ideal
53,13010235415599° beklerken saklanan gerçek yüz uçlarının `atan2` yönü
53,1301023558235°'dir; fark yaklaşık 1,6675×10⁻⁹°.
Yalnız testin beklenen açısı gerçek saklanan uçlardan hesaplanacak biçimde
düzeltilmiştir; üretim toleransı genişletilmemiştir. Başarısız tur ayrı
kalır; son yedi testlik tam tekrarın başarısı onun kaydını silmez.

Son masaüstü ve UI kayıtları da aynı `test-runs.json` içindedir:

| Tur | Çıkış | Başarılı | Başarısız | Ignored |
|---|---:|---:|---:|---:|
| İlk tam desktop, 23:51:57 | 101 | 1.337 | 4 | 228 |
| İki seri layout hedefi, 00:01:49 | 0 | 2 | 0 | 0 |
| Son UI, 00:07:51 | 0 | 289 | 0 | 0 |
| Son tam desktop, 00:17:00 | 0 | 1.341 | 0 | 228 |
| İlk tam Python, 00:38:36 | 1 | 364 | 2 | 0 |
| Son tam Python, 00:48:48 | 0 | 366 | 0 | 0 |
| API health hedefi, 00:38:48 | 0 | 1 | 0 | 0 |
| İlk yeni server guard, 01:20:44 | 101 | 6 | 1 | 0 |
| Son yeni server guard, 03:13:02 | 0 | 7 | 0 | 0 |

Zamanlar log dosyasındaki UTC damgalardır. İki hedefin tek başına geçmesi,
önceki tam turun dört başarısızlığını silmez; ayrı son tam tur sonucu ayrıca
korunur. Son desktop log SHA256
`9bb42a0126adc924c6d753798f3615201da7ed3b5e04e766a712d3e223177e3b`,
UI log SHA256
`9d2172b9d006d90617fb4300595b26185622b0897cf3f9530f2c0cfcbf3d25f5`.

228 masaüstü ignored kaydın ana nedenleri: 162 elle başlatılan
resim/snapshot üretimi, 27 açık GPU/seri kabul hedefi, 23 ölçüm/performans,
5 gerçek sunucu/API/PostGIS, 5 Python/MCP ortamı ve 6 kullanıcı dosyası/probe/
çıktı hazırlığıdır. Bazı testlerin ikincil performans veya dosya ihtiyacı da
vardır; bu ayrık ana sınıflar toplam 228 eder. Gerekçesi logda boş olan
`topology::tests::screens`, kaynak yorumunda ve Snapshot çağrısında
`.run/shots/topoloji-*` görüntü üreticisidir. Bunlar çalıştırılmış test veya
gerçek fare/klavye GUI kabulü olarak sayılmaz.

43 ignored kaydın tamamı logda ayrı süre/performans ölçümü olarak
adlandırılmıştır; isimleri ve gerekçeleri `test-runs.json` içindedir.
Bunlar başarılı sayıya eklenmemiştir. Bu eski native tur tek başına son masaüstü UX
kaynağının GUI kabulünü kapsamaz; son desktop/UI turları yukarıda ayrıdır. `desktop-20261026T234254Z.log` turu kök ajan tarafından SIGINT ile
durduruldu; çıkış `-2` ve tanısaldır, başarı sayılmaz. Bu kesilmiş tur başarı sayılmamıştır. Daha sonra tamamlanan son masaüstü
turu ayrı exit0 kaydıyla yukarıda gösterilir; canlı log hashlenmemiştir.

Taze Python native abi3 tekeri ve editable kurulum tamamlandı;
`pybuild-20261027T002447Z.log` SHA256
`5b161246bba7b58db362efd576fcc42a7c97634773063909f6e9aece4bd8521c`.
Bu derleme tek başına tam Python takımının başarılı olduğu anlamına
gelmez; onun sonucu ayrı kaydedilmiştir. İlk tam Python turu 366 testin
ikisinde başarısızdır: ortak vaka okuyucusu `dimensionSets` beklentisini
tanımıyordu ve yeni test isteğe bağlı `row_places` için `UNSET` yerine `[]`
bekliyordu. Bu iki test/okuyucu düzeltmesinden sonraki tam tur 366/0,
çıkış 0'dır; duvar saati 272,891 sn, unittest'in bildirdiği süre 270,116 sn.
Son log SHA256
`a2221c1a3135e9077311e4176416eae6ea624abc99b3254683f108a86f6bf09c`;
önceki başarısız log da zorunlu envanterde kalır. Logdaki ResourceWarning
gibi uyarılar sıfır uyarı iddiasına dönüştürülmez.

API health turu `api-20261027T003848Z.log`, çıkış 0 ve 2504,68 sn;
SHA256 `ce8318428d713fa121577b6466548c3ae12edb93d0cb023fb35ecc50e21ca41c`.
Gerçekte yalnız `health_answers_without_a_database_and_carries_a_request_id`
testi çalışmıştır. Diğer iki hedefte sıfır test seçilmiş, toplam 96 test
filtrelenmiştir; bu sonuç tüm API veya veritabanı entegrasyon takımı kabulü
değildir. Sonuç yalnız tamamlanan logdan kaydedilir.

Build-4 ve ilgili clippy/build/format/katalog/SDK kapıları tamamlanınca
son kaynakla ilişkileri ayrıca kayda alınmalıdır.

Bu model kayıtları; genel kavisli/çok parçalı duvar desteğini, otomatik
mahal/cephe ölçülendirmesini, metin çakışması çözümünü, bütün mimari araçları,
farklı platform/GPU'ları veya tam mobil/web uyumunu kabul etmez. Çoklu
pafta/görünüş bağlamlarının ve tema/büyük yazı/dar pencere kombinasyonlarının
tam matrisi açıktır. PDF'nin dosya bütünlüğü ve pafta modelinin saklanması,
bütün fiziksel baskı/ölçek/çıktı kabulünün tamamlandığı anlamına gelmez.

177–183 devam denemesinde uzun iki sayfalık teker hareketi çizimin tamamını yaklaşık12 piksele küçültmüştür. 178/179 ilk bakışta kayıp sanılmıştır; imleç dışarı alınınca182 tüm üç duvarı gösterir. Bu bir renderer kusuru olarak kaydedilmez. 181 normal çeyrek sayfa hareketi ve183 geri dönüş bileşen/tarama gözlemidir;183 kaydı tam125 modeline ve özgün fixture baytlarına eşittir. Son strict lint turu beş hata nedeniyle başarısızdır; kural gevşetilmeden düzeltilen üç domain kaynağı için yeni tur ve Build4 beklenir.
