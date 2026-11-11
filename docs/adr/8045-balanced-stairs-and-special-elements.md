# ADR 8045: Merdiven ve özel elemanlar — balansmanlı merdiven, korkuluk, asansör, otopark yeri

- **Durum:** sürüyor (11 Kasım 2026); `plan/architecture-advanced.md` ADV-21, M45. 1. bölüm yapıldı: balansmanlı
  (dengelenmiş) merdiven — çekirdek geometri, plan, model, Merdiven aracı, Öznitelikler ve PAİY 31/2-c denetimi (§1),
  kesit ve görünüşte tek tek dengelenmiş basamaklar (§2). Sırada korkuluk, asansör ve otopark yeri.
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

### Başka programlar (yalnız görünüş ve kurgu için)

- **Archicad, Stair Turning Type** (help.graphisoft.com, Archicad 25): dönüş türleri Automatic Landing, Winder with
  Equal Angles ve Winder with Equal Goings. "Equal Goings means that all goings in the Stair are equal, both on the
  straight and the winding steps."; düzenlenebilen tek değer "the number of winders": orta rıhtın ya da basamağın
  altında ve üstünde kaç basamağın döneceği, zincirle iki yan eşit
  (`.run/gui-evidence/mimari-m45/referans/archicad-winder-equal-goings.png`).
- Revit'in winder kolları da "number of straight treads" ile dönen bölgenin uzunluğunu verir; KentOS aynı kurguyu
  tek sayıyla alır: dönüşün iki yanında dengelenen basamak.

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
