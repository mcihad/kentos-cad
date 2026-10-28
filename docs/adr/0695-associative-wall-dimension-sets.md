# ADR0695 — Kalıcı ilişkili duvar ölçü dizileri
Durum: Live-3 gerçek GUI ve bağımsız model kontrolleri tamamlanan dar vakalar aşağıdadır; son Python/API, clippy ve WASM kapıları beklenir. Nihai özellik kabulü verilmedi (2026-10-27).
Ön koşul: ADR0694 açıklık yerleri ve mevcut belge ölçü stili. Bu karar ARCH-DIM-01'in ilk uçtan uca dilimidir; bütün otomatik mimari ölçülendirme tamamlandı demek değildir.

## Araştırma ve kullanıcı akışı
[Archicad 30](https://help.graphisoft.com/AC/30/INT/Topics/070_Documentation/070_Documentation-49.html) seçilmiş duvar, seçenek, yön ve yerleştirme sırasını; açıklık ucu/merkezi ve birden çok sırayı açık sunar.
[Vectorworks 2026](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Dimensions/Dimensioning_exterior_walls.htm) ölçüleri duvara bağlar; duvar/yapı, yüz/merkez ve açıklık uçları/merkezini ayırır, belge ölçü standardını kullanır.
[Archicad düzenleme](https://help.graphisoft.com/AC/30/INT/Topics/070_Documentation/070_Documentation-45.html) sıra, parça ve metnin ayrı yerleşimini destekler.
[AutoCAD Architecture 2027](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-799D5B1E-CD7D-4773-A34A-1F460E65DC4F.htm) sıra/metin/uzatma yerlerini ayrı düzenler.
[Bastırılan noktayı geri getirme](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-DB9E2BF5-ECB1-4C33-A019-3ABD3E9FBBC3.htm) kaynak silmekten ayrıdır.
Bu kaynaklar tasarım gerekçesidir; rakip yazılımlar yerelde çalıştırılmış veya standart uygunluğu belgelenmiş sayılmaz.

[Microsoft Windows 7 Ribbons](https://learn.microsoft.com/en-us/windows/win32/uxguide/cmd-ribbons)
aynı komutun yakın varyantlarını split button altında toplamayı, sık kullanılan
eylemi doğrudan erişilebilir tutmayı anlatır. Bu **tarihsel Windows 7 tasarım
kılavuzudur**; sayfanın kendi uyarısı yeni Windows sürümleri için
güncellenmediğini belirtir. Güncel platform standardı veya KentOS'un tam
uygunluk kanıtı olarak kullanılmaz. KentOS'ta ilişkili ölçü/sıra seçeneklerini
gruplayıp araç adını görünür tutma kararı için karşılaştırma dayanağıdır.

- Kullanıcı “Duvar ölçüleri” aracını bulur; seçili uygun duvar kullanılır veya duvar seçilir.
- Yüz, açıklık ucu/merkezi, açıklık/toplam sıraları, hedef katman ve kağıt mm sıra aralığı görünürdür.
- Fareyle yer seçilirken sayı ve yazı dahil gerçek ölçü önizlenir. Tek tıklama/Return/sağ tuş Onayla bir defa işler; Escape hiç yazmaz.
- Kaynak, sıra ve grubun seçimi belirgindir. Sıra yerini değiştirme, yazı yerini sıfırlama, gizlenenleri geri getirme, bağımsızlaştırma ve grubu silme ayrı eylemlerdir.
- Bir ölçünün sayısını elle değiştirmek kaynak duvarı değiştirmiş gibi sunulmaz.

## İlk dilimin kapsamı
1. Tek açık, iki noktalı, doğrusal duvarın gerçek sol/sağ yüzü ölçülür. Yay, kapalı çokgen, çok parçalı duvar, çoklu cephe, çekirdek ve mahal henüz desteklenmez; işlem öncesi açıklamalı ret zorunludur.
2. Açıklık uçları veya merkezleri ile açıklık aralıkları ve isteğe bağlı toplam sıra üretilir. Görüntü üçgenlerine değil kalıcı duvar/açıklık yerlerine bağlanır.
3. Dünya geometrisi metre olarak kalır. Kağıt aralığı mevcut plotScale ile bir kez çevrilir; proje birimi, renderer veya mevcut ölçek davranışı değiştirilmez. Belge ölçü stili ve yazı sığdırması kullanılır.
4. Kalıcı DimensionSet tarifi duvar kimliği, yüz, sıralar, açıklık politikası, katman, yer, aralık, varsayılan görünüş, üye kaynak çiftleri ve bastırılmış parçaları tutar.
5. Aynı sıra/kaynak çifti aynı ölçü kimliğini korur; sonradan açıklık ekleme/silme/yön değiştirme üyeliği de yeniden üretir. Kullanıcının sıra yeri ve aynı parçanın metin/görünüş düzeni korunur.
6. Kaynak kaybolursa veya desteklenmeyen topolojiye dönüşürse son geometri görünür kopuk durumla korunur. Sessiz sağlıklı görünen eski ölçü kabul edilmez.
7. Tek ölçü silmek parçayı bastırır; kaynak silmez. Geri getirme açıktır. Bağımsızlaştırma tarifi kaldırır, sıradan ilişkili ölçüleri bırakır. Grup silme tarif ve üyeleri tek adımda kaldırır.
8. Grubun yönettiği ham uç/bağ geometri düzenlemesi atomik reddedilir; metin ve görünüş düzenlenebilir. Kopyalama/bloklaştırmada tarifin sessiz kaybı kabul edilmez.
9. Oluşturma/düzenleme/sorgu GUI, Python ve MCP'de aynı kayıtlı komutları kullanır. Plan belgeye/kimlik sayacına/geçmişe yazmaz; eklenen, değişen ve kaldırılan takipçileri gösterir.
10. Yerel KCAD 113 yalnız bu özellik varsa kullanılır; eski belgelerin baytları aynı kalır. Bağımsız codec tam modeli okur. Bulut metadata+üyeler+kaynakları aynı transaction içinde taşır; özelliksiz sunucuya yazmadan önce açık ret, eski v1 kuyruklara uyum gerekir.
11. Üyenin sıra uzaklığı tarifteki yer/sıra aralığı ve güncel plotScale'den türeyen önbellektir. Mevcut ayar geçmişi korunur: ölçek ayarı geri alma dışında kaldığında undo/redo da güncel ölçeğe göre bu uzaklığı yeniden çözer. Elle taşınmış rowPlaces, yazı yüksekliği/görünüş/yazı yeri ve üye kimliği korunur. Değişen üye geometrisi API özetinde, damgalarda, plan kimliğinde ve bulut dirty kaydında görünür; geometri değişmese de ölçek planı kaynak tarifin damgasına bağlıdır. Boş eski belgelerin plan kimliğine yeni alan eklenmez.
12. Yeni segmentin yazı yüksekliği güncel belge stilinden çözülür: `look.dimStyle` yoksa mevcut `Standart`/proje Ölçü kağıt yüksekliği ve güncel ölçek, tanımlı stil kimliği varsa güncel stilin `height_at` değeri kullanılır. Böylece 1:50'den 1:100'e geçişten sonra eklenen açıklığın yeni ölçüleri eski 1:50 yüksekliğiyle doğmaz. Aynı aktif üyedeki elle yazı yüksekliği, görünüş ve yazı yeri korunur. Tarifin `height` alanı başlangıç template'idir ve artık bulunamayan adlandırılmış stil için fallback'tir; güncel Standart veya var olan stil önceliklidir. Public grup komutunda ayrı sabit model yüksekliği politikası yoktur; bir üyeye elle yükseklik vermek desteklenir, `metadata.height` ile bütün gelecekteki Standart üyelerini sabitleme sözü verilmez.

## Kabul kapıları
- Bağımsız sayısal örnek: 10 m duvar, 20 cm kalınlık, merkez 4 m/genişlik 1 m açıklık: 3,5 + 1 + 5,5 ve toplam 10. Genişlik 2: 3 + 2 + 5. Merkez 6: 5 + 2 + 3.
- Yeni açıklık, silme, rehost, ters yön, büyük koordinat, sol/sağ yüz ve merkez politikası; değişmeyen kaynak çiftlerinin kimlikleri ve tek undo/redo.
- Sıra/yazı yerleri, gizleme/geri getirme, stil ve kilitli katman/veri giriş rolü; yarım değişiklik kalmaması.
- Salt okunur plan, kaynak oluşturma/düzenleme/silme önizlemesinde üye ekleme/kaldırma, scratch tekrarlarında kimlik sayacı.
- KCAD yeniden açma, bağımsız okuma, eski dosya baytları, bulut sürüm el sıkışması ve toplu değişiklik atomikliği.
- Gerçek fare/klavye testi: seçili duvar ve önce duvar seçme, görünür ayarlar, tek onay, Escape, geri al/ileri al, dar pencere, iki tema, büyük yazı.
- Ham ekranlar yalnız .run altında; repo içinde metin raporu ve bağımsız model/manifest özeti. Doğrulanmayan maddeler işaretlenmez.

## Sunucuda son kaynak ve ölçü tutarlılığı

Son incelemede yalnız tam tarifin gönderildiğini ve üye başvurularını
denetlemenin yeterli olmadığı görüldü: duvar ucu 8 m'den 10 m'ye giderken
aynı tarif ve eski 8 m ölçü geometrisi birlikte gönderilebiliyordu.
Sürüm kontrolü, güncel revizyona karşı verilmiş bu eksik işlemi tek başına
ayırt etmez. Alıcı aynı transaction'ın son durumunda ortak saf
`wall::lines`, `opening::jambs` ve `dimension_set::plan` hesaplarını kullanır;
yeni solver, nesne üretimi veya kimlik tahsisi yapmaz. Güncel açıklık
pervazları, aralık üyeliği, kaynak çiftlerinin korunan kimlikleri ve ölçünün
uç/açı/uzaklık geometrisi uyuşmalıdır. Metin yeri, görünüş, elle yükseklik
ve tarifte açık sıra yerleri korunur. Gerçek eksik/desteklenmeyen kaynakta
son geometriyi dondurma politikası ayrıca korunur.

[PostgreSQL 18 transaction belgesi](https://www.postgresql.org/docs/18/tutorial-transactions.html)
birden çok yazının birlikte görünür olmasını ve hata halinde hiçbirinin
kalmamasını açıklar. [Microsoft optimistic concurrency](https://learn.microsoft.com/en-us/ef/core/saving/concurrency)
güncel sürümle yazma ve çatışma sonrası yeniden okuma ilkesini belgeler.
Bunlar atomiklik/sürüm dayanağıdır; mimari geometri doğrulaması KentOS'un
ayrı ürün invariantıdır. Yeni stale-source/üyelik/uzak koordinat DB
regresyonlarının gerçek PostGIS turu geçmeden bu ek denetim kabul edilmiş
sayılmaz; önceki dört başarılı DB testi bu son kodun kanıtı değildir.

## Tamamlanan dar doğrulamalar ve açık kapılar

[QA kaydı](../baseline/2026-10-27-architecture-wall-dimensions/README.md),
bağımsız okuyucu sonuçlarını, model kopyalarını, build kayıtlarını ve önceki
başarısız tanıları ayırır. Son dondurulmuş Live-3 ikilisinde 142–155 gerçek
fare/klavye akışı kaynak seçimi, görünür ayarlar, tek sağ tuş Onayla, tek
toolbar Undo/Redo ile tekrarlandı. 153'te 100 cm sabit uzaklık **1 m**,
aralık ise değiştirilmemiş varsayılan **7,5 mm**'dir. PlotScale50'de açıklık
ölçülerinin uzaklığı 1 m, toplam sıranınki 1,375 m; uzunluklar
3,5 / 1 / 5,5 / 10 m'dir. Hedef katman `olcu`, gerçek sol yüz ve kaynak
başvuruları bağımsız tam model okumasıyla doğrulandı. Kaynaklar ve ilgisiz
metadata aynı kalır; 154 tam başlangıcı, 155 tam 153'ü geri getirir.
156 yeni süreçte açılan tam belgenin 155'e eşitliğini doğrular. Önceki
Live-2'nin 10 mm aralığı bu Live-3 vakasına aktarılmaz.

Son tam masaüstü turu varsayılan paralellikte **1341 geçti / 0 başarısız /
228 ignored**; UI turu **289 geçti / 0 başarısız**. Üç uygulama derlemesi
başarılıdır; son GUI kaydı Live-3'tür. Bunlar yeni Python/API turu, son
clippy/WASM kapısı veya bütün etkileşim matrisinin yerine geçmez; nihai
kabul bu kapılar kapanana kadar açık kalır. Önceki Live-1/2 yaşam döngüsü
vakaları QA'da kendi ikilileriyle korunur; hepsi son ikilide yeniden
çalıştırılmış gibi sunulmaz.

Mevcut açıklık merkez tutamacı yolu `select_opening::preview` sonucundaki
`followers`'ı taşımıyor; çağıran da yalnız ilk Shape'i alıyor. Bu yolda tam
Add/Update/Remove ölçü önizlemesi henüz kabul değildir. Genel WallEdit tam
önizlemesi, bütün grup/sıra/segment eylemlerinin son ikilide tam tema/yazı/
pencere matrisi ve bütün kaynak topolojileri açık kapsamdır. İlk A3 PDF
vektör boyu/görsel kontrolü temiz PDF metin semantiği anlamına gelmez:
önceden var olan halo boyaması ham ölçü metnini iki kez çıkarabiliyor.
Yeni stil görünüşüyle tam taze PDF çıktı matrisi yapılmadı.

## Dışa aktarma sınırı
DXF/DWG, seçilmiş sıradan ölçü geometrisini taşır; KentOS kaynak tarifi ve otomatik üye ekleme/kaldırma davranışı taşınmaz. Dışa aktarma raporu yalnız seçilen aktif üyesi bulunan grupları sayarak açık uyarı verir ve KCAD ile saklamayı önerir. Kaynak duvarın tek başına aktarımı, seçilmemiş ölçü grubu için kayıp uyarısı üretmez; grup bulunmayan eski aktarımın girdisi değişmez.

WBLOCK/`file.export.block` seçimindeki aktif ölçü üyesi, seçim kaynak duvarı da içerse, dosya yazılmadan veya mevcut dosya değiştirilmeden `dimension_set_owned` ile reddedilir. Seçim paketi henüz kalıcı ölçü grafiğini taşımadığından önce açık, geri alınabilir bağımsızlaştırma gerekir. Bağımsız ölçü ve mevcut blok aktarımı çalışmaya devam eder. Bu sınır gerçek headless validate/plan/execute, değişmeyen dosya baytları ve bağımsızlaştırma sonrası açma testleriyle tutulur; tam grafik kopyalama sonraki dilimdedir.

## Sonraki dilimler
Kapalı/dikdörtgen ve çok parçalı duvarda açık cephe seçimi; çoklu duvar/mahal; çekirdek/yapı/ara duvar sıraları; çoklu pafta; çakışma ve sığmayan yazıyı açıklayan yerleştirme; tam grafik kopyalama ve bütün kabul matrisi. Bunlar ARCH-DIM-01 açık kapsamıdır.


## Ek (27 Ekim 2026): açıklıksız kenarda tek ölçü

Kullanıcının bildirdiği kusur: Duvar ölçüleri, açıklığı olmayan duvar kenarına aynı boyu iki kez yazıyordu. Açıklık sırası
açıklık yokken kenarın bütün boyudur ve toplam sırası onu ikinci sırada tekrar ediyordu. Kural: bir ölçü, sıraların
sırasıyla kendinden önce çizilen bir ölçüyle aynı iki yeri ölçüyorsa çizilmez. Açıklıksız kenarda kullanıcının tıkladığı
ilk sırada tek ölçü kalır; kenara açıklık eklenince açıklık sırası parçalanır ve toplam yeniden çizilir (eski toplam ölçünün
yerine yeni üye). Kullanıcının kaldırdığı (bastırılmış) parça çizilmediği için öbür sıranın aynı ölçüsü kalır. Kural
belgenin planındadır (`dimension_set_plan`), çekirdeğin sıra hesabı değişmedi. Eski dosyalardaki yinelenen toplam ölçü,
duvar ya da dizi yeniden planlanınca kalkar; açılışta kendiliğinden silinmez. Doğrulama: `dimension_sets_command_cases.py`
(109 ortak durum, kural başvuruda da), belge ve araç testleri (bir kenar tek ölçü, pencere silinince 4 üye gider 1 gelir,
odanın dört kenarı dört ölçü).

## Ek (28 Ekim 2026): kapanan kabul kapıları

- **Tutamaçla önizleme:** açıklığın merkez tutamacı (`select_opening::preview`) ve duvarın Esnet/WallEdit tutamacı
  sürüklenirken belgenin izleyicilerinin aynı hesabıyla (`Document::preview_followers`) bütün ölçü dizisi üyelerini
  eklenen, değişen ve kalkan olarak önizler; değerler çizimin yazısıyla görünür, kalkanlar tehlike rengindedir; önizleme
  hiçbir şey yazmaz (`wall_dimensions::an_opening_grip_previews_the_dimensions_that_follow_it`,
  `a_wall_grip_previews_the_dimensions_that_follow_it`; etkileşim takımının 807 testi).
- **PDF metni:** yazının halesi aynı harflerin altına konturla basılır; boş `ActualText`'li işaretli içerik (PDF 1.5)
  okuyucuların onu ikinci kez almasını önler. `pdftotext -raw` eski ifraz paftasında 225 sözcük (hale yazıları iki
  kez), yenisinde 161 sözcük verir; 64 halenin hepsi sarılıdır. Paftanın altın kaydı yenilendi
  (`fixtures/sheet/v1/pdf/ifraz.json`), pafta ve çizici takımları geçer.
- **Veritabanı:** `kentos-application`'ın bütün takımı gerçek PostGIS'te (`KENTOS_TEST_DB=required`) 189 testle geçer;
  cephe ve mahal tarifinin sunucu denetimi ADR 0729 §8'dedir.

Çoklu pafta, sığmayan yazının yeri, toplamın yuvarlama kuralı ve tam grafik kopyalama ADR 0730'dadır.
