# ADR 8045: Merdiven ve özel elemanlar — balansmanlı merdiven, korkuluk, asansör, otopark yeri

- **Durum:** sürüyor (11 Kasım 2026); `plan/architecture-advanced.md` ADV-21, M45. 1. bölüm yapıldı: balansmanlı
  (dengelenmiş) merdiven — çekirdek geometri, plan, model, Merdiven aracı, Öznitelikler ve PAİY 31/2-c denetimi (§1),
  kesit ve görünüşte tek tek dengelenmiş basamaklar (§2). 2. bölümde bağımsız korkuluk — model, `.kcad` 242, komut,
  plan, Korkuluk aracı (Boşluktan ile galeri boşluğu), Öznitelikler, kesit ve görünüş (§3). Sırada PAİY 38 denetimi,
  sonra asansör ve otopark yeri.
- **Bağlam belgesi:** ADR 0343 (merdivenin yürüme çizgisi, sahanlıklar, basamak çizgileri), ADR 0704 (yükseklikten
  merdiven), ADR 0709 (döner merdiven), ADR 0724 (kata bağlı merdiven, üst kat planı), ADR 8034 (PAİY 31 denetimi).

## Bağlam

KentOS'un L ve U merdiveni dönüşte her zaman sahanlık kurar. Türkiye'de konutta sık görülen balansmanlı merdivende
dönüşte sahanlık yoktur: basamaklar dönüşün iki yanında "dengelenir", yürüme çizgisinde hepsi aynı derinlikte kalır, iç
kenarda köşeye doğru daralır. PAİY bu merdiveni adıyla düzenler; KentOS onu ne çizebiliyor ne denetleyebiliyordu.

## Araştırma

### Türkiye mevzuatı

- **PAİY Md. 31/2** (RG 3.7.2017/30113, resmî metin): (a) basamak yüksekliği asansörsüz binada 0.16, asansörlüde
  0.18 m'yi geçemez; (b) "Basamak genişliği 2a+b= 60 ila 64 formülüne göre hesaplanır … Ancak bu genişlik 0.27 metreden
  az olamaz."; (c) "Balansmanlı (dengelenmiş) merdivenlerde basamak genişliği en dar kenarda 0.10 metre, basamak
  ortasında 0.27 metreden az olamaz."; (ç) basamak uçları damlalıksız.
- **PAİY Md. 31/1-a:** ortak merdiven kolu ve sahanlık konutta 1.20 m, diğer yapılarda 1.50 m, bağımsız bölüm içinde
  1.00 m (genişlik denetimi ADR 8034'te var; balansmanlı merdiven kolunu aynı ölçüyle okur).
- **PAİY Md. 38** (resmî metin): (1) "Binalarda; balkon, teras, galeri boşluğu, sahanlıkların boş kenarları 1'den
  fazla basamağı bulunan açık merdivenlerde ve rampalarda, istinat duvarı üstünde, … bulunduğu katın taban
  döşemesinden itibaren kotu 0.90 metreden az olan pencere boşluklarında, döşeme kotundan itibaren en az 1.10 metre
  yüksekliğe kadar teknik gereklere ve standartlara uygun olarak korkuluk yapılması mecburidir." (2) Son kat açık
  teraslarındaki korkuluğun ve parapetin 1.10 m'si bina yüksekliğinden sayılmaz. (3) Kırılmaz ya da kırıldığında
  dağılmayan malzeme, insan çarpması dahil tasarım yükleri. (4) "Boşluklarda, yük altındaki deformasyonlar da dâhil,
  en fazla 0.10 metre çapında geçişe izin verilir." Md. 40/6: kullanılan teras çatıda 1.10 m parapet ya da korkuluk.
  Sivas İmar Yönetmeliği 38/1 PAİY ile aynıdır (ADR 8034).

### Başka programlar (yalnız görünüş ve kurgu için)

- **Archicad, Stair Turning Type** (help.graphisoft.com, Archicad 25): dönüş türleri Automatic Landing, Winder with
  Equal Angles ve Winder with Equal Goings. "Equal Goings means that all goings in the Stair are equal, both on the
  straight and the winding steps."; düzenlenebilen tek değer "the number of winders": orta rıhtın ya da basamağın
  altında ve üstünde kaç basamağın döneceği, zincirle iki yan eşit
  (`.run/gui-evidence/mimari-m45/referans/archicad-winder-equal-goings.png`).
- Revit'in winder kolları da "number of straight treads" ile dönen bölgenin uzunluğunu verir; KentOS aynı kurguyu
  tek sayıyla alır: dönüşün iki yanında dengelenen basamak.
- **Archicad, Railing Tool** (help.graphisoft.com, Archicad 29–30): "Railing input is similar to polyline input";
  merdivene, döşemeye, duvara, çatıya bağlanabilir; segment altında küpeşte (toprail), en çok iki tutamak, dikmeler,
  çubuklar (balusters) ve paneller ayrı bileşen; planda küpeştenin sembolü çift çizgi (eksenli ya da eksensiz) ya da
  yalnız eksen (`.run/gui-evidence/mimari-m45/referans/archicad-railing-*.png`).
- **Revit, Railings:** yol çizilerek kurulur, yükseklik ve çubuk yerleşimi korkuluk türünde
  (`revit-railing-sketch-path.png`). KentOS planı balkon korkuluğunun çift çizgisini (ADR 0722 §4) sürdürür; dikme ve
  çubuk ayrıntısı planda yok, kesit ve görünüşte gelecek.

## Karar

### 1. Balansmanlı merdiven (1. bölüm)

- **Model (`StairFields.winders`, `.kcad` 241):** dönüşün iki yanında dengelenen basamak sayısı; yoksa her dönüşte
  sahanlık. Şema 240 dosya yazamaz; okuyan "merdivenin balansmanı (winders) şema 241 ister" der. Alan düzeyinde ret:
  sıfır, döner merdiven ("basamakları zaten yay boyunca dengeli"), düz merdiven ("dönüşü yok"), sahanlıkla birlikte.
- **Geometri (`geom::arch`, `winder_turns`, `riser_ends`, `winder_treads`):** sahanlık yok; rıhtlar yürüme çizgisinde
  basamak aralıkla dönüşü dolaşır. Her dönüşün bölgesi köşenin iki yanında `winders × basamak`. İç kenar (saat yönü
  tersi dönüşte sol) dönüşte `genişlik × tan(dönüş/2)` kısa (90°'de genişlik kadar); bu kayıp bölge boyunca köşeye
  doğru doğrusal artan payla dağıtılır: köşedeki basamak en dar, bölgenin uçlarındaki basamak yürüme çizgisine dik.
  Bölgedeki rıht iç kenardaki noktasından yürüme çizgisindeki noktasından geçip dış kenara uzanır. 1,00 m'lik sola dönen
  L merdivende (2,80 + 2,24 m, 0,28 m basamak, 5 balansman) iç basamaklar 0,26 → 0,22 → 0,18 → 0,14 → 0,10 → 0,10 →
  0,14 … m; köşedeki rıht iç köşeden dış köşeye iner.
- **Ret (geometri, `winder_problem`):** bölge iç kenarın kaybından kısa ("en az ⌊kayıp / basamak⌋ + 1 basamak
  gerekir"), bölge kollarına sığmıyor, iki dönüşün bölgesi çakışıyor (kısa ara kollu U merdiven: şimdilik sahanlık).
- **PAİY 31/2-c'ye yeten en az balansman (`winders_for`):** iç kenarda hiçbir basamak 0,10 m'den dar kalmayan en küçük
  sayı. 1,00 m'lik L'de 5, 1,20 m'lik L'de 7, kolları 4 m'lik 1,00 m'lik U'da 6.
- **Plan:** basamak çizgileri `riser_ends` ile; kesme çizgisi kesme rıhtının sol ucundan bir basamak ötekinin sağ ucuna;
  üst kat planı aynı. Korkuluk ve band değişmedi.
- **Merdiven aracı:** Balansman (A): boş ya da "yok" sahanlık, tam sayı dönüşün iki yanında dengelenecek basamak; düz
  ve döner merdivende yok, iki noktalı (düz) yolda komuta verilmez. Tasarım satırı en dar iç basamağı yazar; 0,10 m'den
  darsa "PAİY 31/2-c'nin 0,10 m'sinden dar; balansman basamağını artırın" der.
- **Öznitelikler:** dönen merdivende Dönüş (Sahanlık / Balansmanlı; Balansmanlı'ya geçiş PAİY'ye yeten en az sayıyla),
  Balansman basamağı (boş: sahanlık) ve En dar iç basamak.
- **Denetim › Mevzuat (`paiy_stair_step`, PAİY 31/2-c):** en dar iç basamak 0,10 m'den darsa "Balansmanlı merdivenin
  basamağı dar uçta 0.090 m < 0.10 m — PAİY 31/2-c"; düzeltmesi PAİY'ye yeten en az balansman (`cad.arch.edit`).
  Basamağın ortası yürüme çizgisidir; oradaki 0,27 m mevcut basamak denetimi.
- **Sonraya:** kısa ara kollu U merdivende birleşik bölge, iç köşesi yuvarlatılmış (gözlü) merdiven.

### 2. Kesitte dengelenmiş basamaklar (1. bölüm, ikinci adım)

Kesit, merdiveni kollar (eğik plak üstünde dik basamaklar) ve sahanlıklarla kuruyordu. Balansmanlı merdivende sahanlık
olmadığı için ilk kol yürüme çizgisinin köşesinde bitiyor, dönüşün dış yarısı boş kalıyor, ikinci kol havada duruyordu;
plandaki dengelenmiş basamakların kesitte karşılığı yoktu.

- **Kollar dönüş bölgesinde durur (`sections::stair`, `stops`):** her dönüş, bölgesinden önceki son dik rıhttan
  bölgesinden sonraki ilk dik rıhta (yoksa merdivenin son rıhtına) uzanır; kol orada sahanlıktaki gibi biter.
- **Her dengelenmiş basamak ayrı bir prizma:** tabanı iki rıhtının uçları ve aralarında kalan band köşeleri
  (`geom::arch::tread_ring`, saat yönünün tersi); köşedeki iki basamak iç köşeyi tutar ve dış köşeye uzanır. Üstü kendi
  yüksekliği (taban + rıht × sırası), altı bir rıht ve merdivenin kalınlığı aşağısı, döşemenin altına inmez; merdiveni
  tutan döşeme yoksa (kalınlık bilinmiyor) döşemeye kadar dolu, kollarla aynı.
- **Alt yüz basamaklı (karar):** betonarme balansmanlı merdivenin alt yüzü burulmuş bir yüzeydir; onu çizmek kesit
  motoruna yeni bir yüzey türü ister. Ruhsat ölçeğinde (1/50, 1/100) basamak basamak inen alt yüz okunuyor; burulmuş
  alt yüz sonraya.
- Görünüşler aynı katı modelden çıkarıldığı için dengelenmiş basamaklar görünüşte de görünür; kesit düzleminin
  ardında kalan basamakların rıhtları görünüş çizgisiyle çizilir.

### 3. Bağımsız korkuluk (2. bölüm, ilk adım)

Korkuluk yalnız merdivenin ve rampanın yanında (ADR 0705) ve balkon ile terasın serbest kenarında (döşeme kenarı,
ADR 0722 §4) vardı. PAİY 38/1'in saydığı galeri boşluğu, sahanlığın boş kenarı, istinat duvarı ya da asma kat kenarı
için çizilebilen bir korkuluk yoktu.

- **Model (`RailingFields`, `Entity::Railing`, `.kcad` 242):** çizgisi (küpeştenin ortası; açıkken en az 2, kapalıyken
  3 nokta, art arda aynı nokta ve kapalıda sonda yinelenen ilk nokta yok), `closed`, yükseklik (döşemesinden
  küpeştenin üstüne; komut verilmezse 1,10 m, PAİY 38/1), küpeşte eni (verilmezse 0,05 m, balkon korkuluğununki),
  dolgu (`balusters` dikey çubuk — yoksa —, `glass` cam, `solid` dolu), çubuk aralığı (`gap`: çubuklar arası net
  boşluk; yoksa PAİY 38/4'ün 0,10 m'si, yalnız dikey çubukta okunur), kot farkı (döşemesinin katın döşemesinden farkı).
  Sınırlar: yükseklik 3 m, küpeşte 0,5 m, aralık 1 m, kot farkı ±10 m. Blok tanımında olmaz (kata oturur). Şema 241
  türü tanımaz (`unknown_kind`); sunucu `feature` tablosu `railing` türünü alır (0040).
- **Plan (`geom::railing`):** küpeştenin iki yüzü, orta çizginin iki yanında enin yarısı kadar (iç köşeler gönye),
  açık korkuluğun uçları kapalı — balkon korkuluğunun iki ince çizgisi; 1/200 sadeleştirilmiş görünüşte yalnız orta
  çizgi. Seçim ve sınır için açık korkuluk bant, kapalı korkuluk delikli halka.
- **Komut:** `cad.railings.create` v1 (“Korkuluk”, tek geri alma adımı; ret sırası `no_objects`, `not_finite`,
  `invalid_railing`, `level_not_found`, katman ve öznitelik retleri). `cad.arch.edit` korkuluğun `height`, `width`,
  `infill`, `gap` (null: 0,10 m) ve `offset` alanlarını değiştirir. Kat ver, Denetim'in katsız nesnesi, seçim süzgeci
  (Korkuluk, son sırada; kayıtlı süzgeçler bitlerini korur), CBS türevi (çizgi), DXF, GeoJSON korkuluğu tanır.
- **Korkuluk aracı (Mimari › Merdiven ▾ › Korkuluk, KO; Revit'in dolaşım paneli gibi merdiven ve rampanın yanında):**
  Çizgi (S) Çiz — noktalar, Enter yazar, Kapat (K) ya da ilk noktaya tıklamak kapalı yazar; Boşluktan — Etkin kattaki
  döşemenin boşluğunun içine tek tıklama. Yükseklik (Y), Dolgu (D), Aralık (A, yalnız dikey çubukta), Küpeşte (E),
  Kot farkı (F). Katmanı dolgusuna göre MM-KORK-CUBUK, MM-KORK-CAM, MM-KORK-DOLU (EK-2 Korkuluk, KORK). 1,10 m'den
  alçak ya da aralığı 0,10 m'den geniş korkuluk yazılır; önizleme etiketi ve ileti kırmızıyla PAİY 38'i söyler.
- **Boşluktan'ın kararları:**
  - Yalnız serbest kenarlar: duvar yüzünün boyunca uzandığı kenara korkuluk konmaz (balkon korkuluğunun
    `free_runs` kuralı, boşluk kendi döşemesi gibi okunur). İlk denemede boşluğun bütün çevresi kapalı yazılıyordu;
    görünüş binasının merdiven boşluğunda batı ve doğu kenarlar duvarın yüzündeydi ve korkuluk duvarın içine
    giriyordu.
  - Merdivenin çıkışı açık: alt kattaki düz bitişli merdivenin son rıht çizgisi (yürüme çizgisinin son noktasında
    bandın eni) boşluğun kenarından 2 cm içindeyse o parça kesilir; yoksa korkuluk merdivenin çıkışını kapatırdı.
  - 0,10 m'den kısa parça yazılmaz: PAİY 38/4'ün 0,10 m'lik cismi o aralıktan geçmez. Görünüş binasında merdiven bandı
    boşluktan 2,5 cm dar kalıyordu; 2,5 cm'lik bir korkuluk parçası çıkıyordu.
  - Bütün çevresi serbest boşluk tek kapalı korkuluk; parçalar ayrı korkuluk, hepsi tek geri alma adımı. Yay kenarlı
    boşluk reddedilir (noktalarla çizilir).
- **Öznitelikler:** Yükseklik, Dolgu, Çubuk aralığı (dikey çubukta), Küpeşte eni, Kot farkı (düzenlenebilir), Çizgisi
  (Açık/Kapalı), Uzunluk, Kat, Tadilat durumu ve Denetim: Uyuyor ya da “1,10 m'den alçak” (PAİY 38/1), “Aralık 0,10
  m'den geniş” (PAİY 38/4).
- **Kesit ve görünüş (`sections::railing`):** döşemesinden (katın döşemesi + kot farkı) yüksekliğine; katmanı dolgusunun
  (MM-KORK-CUBUK, -CAM, -DOLU), kesiti ince ve taramasız. Dikey çubuklu ve camlı korkulukta yalnız küpeşte katıdır
  (5 cm, kesitte küçük bir kare); çubuklar (aralık + 2 cm'lik çubuk, ilki yarım adım içeride) ya da camın ayağı ve
  uçları ayak izi olmayan bir katının çizgileridir: önündeki onları gizler, onlar arkasındakini gizlemez — kapının
  duvar yüzündeki çizgileri gibi (ADR 8043). İlk sürüm korkuluğu 1,10 m'lik opak bir bant yaptı; A-A'da merdiven
  holünün penceresi korkuluğun arkasında yarım kaldı (denizliği +3,90'da, küpeştenin altı görünmüyordu). Dolu
  korkuluk tam yüksekliğinde katıdır, küpeştenin altı iki yüzünde çizgi.
- **Sonraya:** Denetim › Mevzuat'ta PAİY 38 (korkuluksuz galeri boşluğu ve balkon kenarı, alçak korkuluk, geniş
  aralık; düzeltmeleriyle), metrajda korkuluk boyu, yay kenarlı boşluk, döşemeye bağlı (döşeme değişince izleyen)
  korkuluk, dikme ve tutamak ayrıntısı.

## Doğrulama

- Bağımsız başvuru (`scripts/fixtures/arch_cases.py`, kesirlerle tam): 6 balansman durumu (sola dönen 1,00 m L, sağa
  dönen 1,20 m L, U, üç ret); çekirdek testi her rıhtın iki ucunu, her dengelenen basamağın iç ve orta genişliğini,
  reddi ve en az sayıyı 1e-9 içinde karşılaştırır.
- KCAD şema 241 örneği (`stair-winders.kcad`), şema 240'ta ret, şema 242'nin reddi; KCAD testleri 250; başvuru
  `--check` 1790 dosya. Ortak komut durumları: `cad.stairs.create` (2 geçerli: L ve U; 7 ret: düz, döner, sıfır,
  sahanlıkla birlikte, dönüşün kaybından kısa, kola sığmayan, iki dönüşte çakışan), `cad.arch.edit` (balansman verilir,
  sahanlık kalkar, null sahanlığa döner; kola sığmayan ve düz merdivende ret). Etkileşim: Merdiven aracında A 5 ile
  1,20 m'lik L'nin tasarım satırı PAİY uyarısı, A 7 ile "(PAİY 31/2-c: en az 0,10 m)", düz yolda balansman yazılmaz,
  A "yok" sahanlığa döner. Masaüstü 66 (seçenek harfleri, Öznitelikler, Mevzuat).
- Gerçek pencere (Xvfb 1920 × 1080, gerçek fare ve klavye; `fixtures/interaction/real/merdiven-balansman.txt`):
  görünüş binasının sağına 14,5,1 → 17,1 → 17,4 yazılarak 1,20 m'lik L merdiven, Balansman 5: tasarım satırı "en dar
  iç basamak 0.062 m: PAİY 31/2-c'nin 0,10 m'sinden dar"; planda köşede dönen basamaklar; Denetim › Mevzuat'ta
  "Balansmanlı merdivenin basamağı dar uçta 0.061 m < 0.10 m — PAİY 31/2-c" (29 bulgu, 5 düzeltilebilir); Düzelt
  "Balansmanı 7 basamak yap", bulgu düştü (28, 4). Balansman 7 ile çizilen merdivenin Öznitelikler'i: Dönüş
  Balansmanlı, Balansman basamağı 7, En dar iç basamak 0.119 m, Denetim Uyuyor. Eski ikili aynı yolu dönüşte
  sahanlıkla çiziyor.
- Kesit (§2): çekirdek testi düz basamağın dikdörtgenini ve 1,00 m'lik L'nin köşesindeki iki dengelenmiş basamağın
  rıht uçlarıyla birlikte iç (2,3; 0,5) ve dış (3,3; −0,5) köşeyi tuttuğunu, saat yönünün tersine döndüğünü denetler.
  Kesit modeli testi: 1,00 m'lik L, 5 balansman, 18 rıht × 0,17 m: 2 kol ve 10 dengelenmiş basamak (6. ile 15. rıhtların
  üstü, üstleri 1,02 … 2,55 m), sahanlık yok, köşedeki ikisi iç ve dış köşeyi tutuyor, kalınlık olmadan hepsi döşemede.
- Gerçek pencere (`fixtures/interaction/real/merdiven-balansman-kesit.txt`): 1,20 m'lik L, Balansman 7, 18 rıht ×
  0,167 m; A-A kesiti ilk kolu ve dönüşü 0,10 m yürüme çizgisinin dışından kesiyor. Eski ikili: ilk kolun 9 basamağı
  yürüme çizgisinin köşesinde (x = 17,00) bitiyor, dönüşün dış 0,60 m'si boş, ikinci kol +1,50'de havada. Yeni ikili:
  gövde merdivenin dış kenarına (17,60) kadar dolu, dengelenmiş basamaklar tek tek kesiliyor, ardındakilerin rıhtları
  görünüyor; ±0,00'dan son basamağın +2,83'üne kesintisiz.
- Korkuluk (§3): sözleşme testi (uzunluk, aralık, alan retleri, varsayılanların yazılmadığı JSON); çekirdek testi
  (açık korkuluğun iki yüzü ±0,025 m, 1/200'de tek çizgi, kapalı korkuluğun delikli halkası); KCAD şema 242 örneği
  (`railing-objects.kcad`: açık dikey çubuklu ve kapalı cam), şema 241'de `unknown_kind`, şema 243'ün reddi; KCAD
  testleri 250, başvuru `--check` 1792 dosya. Ortak komut durumları: `cad.railings.create` 35 (bağımsız başvuru
  `scripts/fixtures/arch_command_cases.py`), `cad.arch.edit` 75 → 83 (korkuluğun alanları, null aralık, retler,
  türünde olmayan alan). Etkileşim: Korkuluk aracı 4 test (Çiz ve kapatma, katmanlar MM-KORK-CUBUK/CAM, Boşluktan
  serbest çevre, duvarlı kenarlar ve merdiven çıkışı: üç parça tek adım, alçak korkuluğun uyarısı).
- Gerçek pencere (`fixtures/interaction/real/korkuluk-galeri.txt`): görünüş binasının 1. katında merdiven boşluğu
  (2,85 × 3,94 m), Boşluktan ile tek tıklama: batı ve doğu kenar duvar yüzünde, korkuluk yok; kuzey kenarı duvardan
  duvara 2,850 m; güney kenarı batı duvarından merdivenin çıktığı yere (6,20 m'de biter, merdivenin 1,20 m'lik çıkışı
  açık; üstüne gelince kart “Uzunluk 1.625 m”, “Alan 0.08 m²”), 2,5 cm'lik artık yazılmadı. Öznitelikler: Yükseklik 1.100, Dolgu Dikey çubuk, Çubuk aralığı 0.100,
  Küpeşte eni 0.050, Çizgisi Açık, Uzunluk 2.850, Denetim Uyuyor. Eski ikilide menü araması “Korkuluk menülerde yok”
  diyor.
- Korkuluk kesitte (§3): kesit modeli testi (bölüm binası, 1. kat, 2,40 m'lik üç korkuluk): çubuklu olan küpeşte katısı
  +4,05–+4,10 ve 20 çubuk (+3,00'ten +4,05'e, ayak izsiz katının çizgileri), camlı olan küpeşte ve camın ayağı ile iki
  ucu, dolu olan +3,00–+4,10 tek katı; A-A'nın bloğunda üç katman. Gerçek pencere (`korkuluk-kesit.txt`,
  `korkuluk-kesit-a.txt`): görünüş binasında Boşluktan'dan sonra B-B'de iki korkuluğun küpeştesi kesilen küçük birer
  kare, çubukları +3,00'e inen tek çizgi; A-A'da merdiven holünde küpeşte bandı ve 0,12 m'de bir çubuk, arkadaki
  pencere bütün. Önceki ikilide (korkuluk var, kesitte yok) iki kesitte de korkuluk görünmüyor.
