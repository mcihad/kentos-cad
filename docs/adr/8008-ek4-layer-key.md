# 3008 — Katman anahtarı: mimari nesne türünün EK-4 katmanına yazılır

Tarih: 2026-11-07. Durum: kabul (M9, ADV-14; plan S5, G4).

## Sorun

- **Her şey etkin katmandaydı (S5).** Bütün mimari nesneler etkin katmana yazılıyordu.
  - Gerçek pencerede fareyle çizilen 2+1 dairede 54 nesnenin hepsi "Çizim" katmanındaydı
    (`.run/gui-evidence/mimari-m9/once/`). M0'da da aynı sayı ölçülmüştü.
  - Ölçü, Yazı ve Tarama katmanları boştu. Duvar ölçüleri "Ölçü katmanı: Çizim", Hepsini etiketle "Etiket katmanı:
    Çizim (sabit)" diyordu (G4).
- **Yönetmelik bunu istemiyor.** 2026 Dijital Hazırlama Yönetmeliği'nin (RG 05.08.2026/33331) EK-4'ü katmanları
  disipline, öğeye ve malzemeye göre ayırır. DXF'i açan belediye ya da ofis duvarı kapıdan, ölçüyü yazıdan ayıramıyordu.

## Araştırma (7 Kasım 2026)

- **Mevzuat:** 2026 Dijital Hazırlama Yönetmeliği, Md. 7(1) ve ekleri. Sayfa görüntüleri okundu, OCR'ın bozduğu kodlar
  sayfadan düzeltildi.
  - **EK-4 Tablo 4.1** (mimarlık, iç mimarlık, harita, zemin): disiplin (zorunlu, 2 harf), öğe (zorunlu, 4 harf),
    malzeme/tip/gösterim (zorunlu, en çok 15 harf ya da rakam). Tablo 4.3 (peyzaj) ve 4.4 (blok) üçüncü alanı "isteğe
    bağlı" yazar; mimari katmanda alan zorunludur.
  - **4.11:** büyük harf; Ç, Ğ, İ, Ö, Ş, Ü kullanılmaz; alanlar arasında boşluksuz orta tire.
  - **4.12.1:** silinemeyen "0" katmanı gibi katmanlarda çizim yapılmaz.
  - **4.14.3:** plan ve kesitte kesite giren yerlerin katmanı malzemesiyle adlandırılır.
  - **4.14.5:** taranacak öğe, katman adının sonuna "-T" eklenmiş ayrı bir katmanda taranır.
  - **4.29:** aks çizgileri noktalı kesikli çizilir.
  - **EK-2 Tablo 2.2:** Mimari "MM". **Tablo 2.3:** mimari öğe kodları, 53 satır. Örnekler: Duvar DUVA, Kapı KAPI,
    Pencere PENC, Döşeme DOSM, Kolon KOLN, Merdiven MERD, Rampa RAMP, Aks AKSS, Baca BACA, Tefriş TEFR,
    Genel XXXX, Diğer DIGR.
  - Tabloda "Mahal" öğesi yoktur. Mahal, ölçü, etiket ve çizelge "Genel (XXXX)" öğesiyle ve gösterim alanıyla
    adlandırıldı.
- **AutoCAD Architecture 2026–2027, *About Working With Layer Keys*:** "A layer key maps an object that you draw to a
  defined layer. When you create an object, the layer key associated with it automatically places the object on the
  layer to which it is mapped." AIA ve BS1192 anahtar stilleri ürünle gelir. AIA 2nd Edition'ın alanları discipline,
  major, minor 1, minor 2 ve status'tur; aralarına tire konur (*About Creating and Editing Layer Standards*).
- **Revit, *About Layer Mapping for Export*:** "a Revit door object automatically maps to A-DOOR layer in AutoCAD."
  Eşleme AIA, ISO 13567, Singapur ve BS 1192 standartlarından yüklenebilir.
- **Archicad 27, *Wall Tool Settings*:** her aracın varsayılan ayarı bir katman taşır. Resmî görselde duvar aracının
  katmanı "ENZ - Structure - Wall"dur (`.run/gui-evidence/mimari-m8/referans/archicad-ChooseComposite.png`).
- **Ortak yol:** nesne türü bir katmana bağlanır, kullanıcı her çizimde katman seçmez; DXF/DWG'ye aynı katmanlarla gider.

## Karar

- **Ad** (`kentos_contracts::ek4`): `MM-<öğe>-<malzeme/tip/gösterim>`. Türkçe harf ASCII'ye çevrilir ("Tuğla" →
  TUGLA, "Alçı levha" → ALCILEVHA); harf ve rakam dışı atılır, 15 karakterde kesilir. Adı olmayan üçüncü alan GENEL'dir.
  `is_ek4_layer` adın kurala uyduğunu söyler. Tablo 2.3'ün 53 kodu `EK4_ELEMENTS`'tedir.
- **Anahtar** (`kentos_interaction::arch_layer::Kind`):

  | Nesne | Katman |
  |---|---|
  | Duvar | MM-DUVA-<çekirdeğin malzemesi>: Dış/İç duvar TUGLA, Gazbeton GAZBETON, Betonarme perde BETONARME, Bölme duvar 10 ALCIPAN; stilsiz GENEL |
  | Kapı, pencere | MM-KAPI-GENEL, MM-PENC-GENEL |
  | Boşluk, niş | MM-DUVA-BOSLUK, MM-DUVA-NIS |
  | Baca | MM-BACA-GENEL |
  | Döşeme | MM-DOSM-<malzemesi>, yoksa GENEL |
  | Kolon | MM-KOLN-GENEL (kolon malzemesi M41'de) |
  | Merdiven, rampa | MM-MERD-GENEL, MM-RAMP-GENEL |
  | Aks, aks ağı | MM-AKSS-GENEL, noktalı kesikli (4.29) |
  | Mutfak dolabı | MM-TEFR-MUTFAK |
  | Mahal | MM-XXXX-MAHAL |
  | Duvar ölçüleri, iç ölçü | MM-XXXX-OLCU |
  | Etiket, Hepsini etiketle | MM-XXXX-ETIKET |
  | Kuzey oku, kesit ve görünüş işareti | MM-XXXX-SEMBOL |
  | Malzeme lejandı | MM-XXXX-LEJANT |
  | Çizelge | MM-XXXX-CIZELGE |

  - Duvarın malzemesi, stilin önceliği en küçük bileşenidir (çekirdek). KentOS'un bölme duvarı alçı levhalarıyla
    adlandırılır, çünkü çekirdeği yalıtımdır.
  - Katmanın görünüşü: kesilenler (duvar, kolon) mürekkep 0,50 mm; döşeme 0,35; açıklık, merdiven, rampa 0,25; aks
    kırmızı noktalı kesikli 0,18; ölçü yeni projenin Ölçü rengi. Kesilen ve görünen kalem ayrımı M10'dadır.
- **Katman ilk nesneyle açılır** (`arch_layer::writing`):
  - Katman yoksa nesneyle aynı geri alma adımında açılır; adım komutun kendi adını taşır ("Duvar").
  - Nesne reddedilirse katman da gider. Tek geri alma ikisini birlikte geri alır.
  - Çizimde aynı adlı bir katman varsa (içe aktarılmış bir DXF'in) kimliği ne olursa olsun o kullanılır.
  - Önizleme, katman açılana dek etkin katmanda planlanır. Önizlemesini çizimin sürümüne bağlayan araçların
    (Toplu mahal bul, Çizgiden duvar, Duvar ölçüleri, Hepsini etiketle) beklenen sürümü, açılan katman kadar ilerletilir
    (`carried`).
- **Kendi katman seçimi olan araçlar:** Duvar ölçüleri ve İç ölçü "Ölçü katmanı"nı MM-XXXX-OLCU ile başlatır. Katman
  yoksa listenin başında "MM-XXXX-OLCU (ilk ölçü yazılınca açılır)" durur. Hepsini etiketle'nin salt okunur "Etiket
  katmanı" MM-XXXX-ETIKET der.
- **Araç panelinde Katman** (`arch_start.rs`, `Session::layer_shown`): Kat satırının altında, aracın yazacağı katmanın
  adı görünür. Menüsü "Türüne göre (EK-4)" ya da "Etkin katman (Çizim)" seçer; seçim oturum boyunca sürer.
- **Program:** KentOS Mimari anahtarı açık başlatır. kentos-cad ve öbür programlar kapalı başlar, panelden açılabilir.
- **DXF** (`formats` `hatches_aside`): EK-4 katmanındaki duvar, açıklık ve yapı elemanının taramaları katman adının
  "-T"sine gider (4.14.5): MM-DUVA-TUGLA'nın tuğla taraması MM-DUVA-TUGLA-T'ye. Bu katman dosyaya Tarama görünüşüyle
  eklenir. Başka katmandaki nesnenin taraması eskisi gibi yanında kalır.
- **Kapsam dışı:** blok yerleştirme etkin katmanda kalır. KentOS blok kitaplığında kategori yok; mobilya, donatı, kot
  işareti ve antet aynı listede duruyor. Donatılar M24'te (donatı galerisi) TEFR'e bağlanacak.

## Ölçüm

- **Testler:**
  - `ek4` (sözleşme, 3 durum): Türkçe harfsiz büyük harf ("Tuğla" → TUGLA); üç alanlı ad ve "-T"; 53 öğe kodu dört büyük
    harf ve birer kez.
  - `arch_layer` (etkileşim, 2 durum): her türün adı EK-4'e uyar; KentOS'un beş duvar stili TUGLA, TUGLA, GAZBETON,
    BETONARME ve ALCIPAN olur.
  - `arch_start_tests` (masaüstü):
    - KentOS Mimari'de panelin Katman'ı MM-DUVA-TUGLA der;
    - ilk duvar katmanı açar ve ona yazılır;
    - tek geri alma ikisini birlikte geri alır;
    - "Etkin katman" duvarı Çizim'e yazar;
    - kentos-cad'de anahtar kapalıdır.
  - `traces::layer_keys_tests` (masaüstü): 2+1 dairenin fare ve klavye izi (`fixtures/interaction/v1/acceptance-flat.json`)
    KentOS Mimari'nin açılışıyla oynatılır.
    - Nesneler 9 EK-4 katmanına gider; Çizim'de nesne kalmaz.
    - DXF KentOS'un verisi olmadan yazılıp geri okununca aynı 9 katman ve MM-DUVA-TUGLA-T gelir; her tarama bir "-T"
      katmanındadır.
  - `dxf_write::an_ek4_walls_hatches_go_on_its_t_layer`: EK-4 katmanındaki duvarın taraması "-T"ye gider, başka
    katmandakinin taraması yerinde kalır.
- **Test takımları:**
  - kentos-contracts 1357 + 34 + 10;
  - kentos-interaction 77 + 896;
  - kentos-formats: bütün takımlar geçti.
  - Masaüstünde 1981 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels` (KentOS Harita'nın
    yol paneli), `main`'den gelen eski hata.
- **Gerçek pencere** (ADR 8000; `fixtures/interaction/real/daire-2p1.txt`, 1440 × 900, 208 adım):
  - **Önce** (M8'in ikilisi): Katmanlar panelinde Çizim 54, öbürleri 0.
  - **Sonra:** 54 nesne 9 katmanda.

    | Katman | Nesne |
    |---|---|
    | MM-DUVA-TUGLA | 5 |
    | MM-KAPI-GENEL | 5 |
    | MM-PENC-GENEL | 5 |
    | MM-XXXX-MAHAL | 5 |
    | MM-TEFR-MUTFAK | 8 |
    | MM-XXXX-OLCU | 8 |
    | MM-DOSM-GENEL | 1 |
    | MM-XXXX-ETIKET | 15 |
    | MM-XXXX-CIZELGE | 2 |

  - Araç panelinde Katman satırı: Ölçülü bölme MM-DUVA-TUGLA, Kapı MM-KAPI-GENEL, Mahal MM-XXXX-MAHAL.
  - Panelin satırları bir satır (29 px) kaydığı için senaryo üreticisi (`scripts/usage/real/scenario_2p1.py`) yeniden
    ölçüldü. Ölçülü bölmenin Stil ve Uçlar menüleri artık yukarı açılıyor.
- **Disk:** iş sırasında makinenin diski doldu (458 GB'ın 0'ı boş). `target/debug/incremental` (111 GB, yeniden üretilen
  önbellek) silindi; 108 GB yer açıldı.

## Kalanlar

- Ofis ve proje profili: anahtar tablosunun dosyadan okunması ve projede saklanması (ADV-14). Bugün KentOS'un yerleşik
  tablosu kullanılıyor ve oturum ayarı dosyaya yazılmıyor.
- Var olan nesneleri anahtara göre taşıyan bir komut ("Katmanlara dağıt").
- Kolon malzemesi (M41), donatı ve blok katmanları (M24).
- Mahal adı ve alanının DXF'e yazı olarak gitmesi (S5'in ikinci yarısı, ADR 0708 §2).
