# 8011 — Kolon, merdiven, rampa, döşeme kenarı ve mobilya ölçeğe göre

Tarih: 2026-11-08. Durum: kabul (M12, ADV-15; plan S4).

## Sorun

- **Kolon, merdiven, rampa, döşeme kenarı ve mobilya her ölçekte aynı çiziliyordu.** Yalnız yazıların boyu kâğıtta
  sabitti.
- **1:200'de birbirine giriyordu** (`.run/gui-evidence/mimari-m12/m12-once.png`):
  - basamak numaraları "123456" diye birleşiyordu (28 cm'lik basamak kâğıtta 1,4 mm, numara 2 mm);
  - korkuluk çizgileri bandın kenarına yapışıyordu;
  - "S1" kolondan büyük basılıyordu;
  - rampanın eğim yazısı "%6.00" bandın kenar çizgisine biniyordu.
- **1:50 uygulama paftası, 1:100'ün istemediği hiçbir şeyi yazmıyordu.** Kolonun ölçüsü ve merdivenin rıht/basamak
  bilgisi yoktu.
- Kapı ve pencere M11'de ölçeğe bağlandı (ADR 8010); öbür mimari nesneler bağlı değildi.

## Araştırma (7–8 Kasım 2026)

- **TMMOB Mimarlar Odası İstanbul Büyükkent Şubesi, *Mimari Proje Çizim ve Sunuş Standartları*.** Sayfa Chrome ile
  basıldı: `.run/gui-evidence/mimari-m11/referans/mimarist-cizim-ve-sunus-standartlari.pdf`.
  - §14.3.3, ön proje planları (1/100):
    - j) "İnşai elemanlar, kolon, perde duvar pano vb. ayrı, çizim tekniği ile çizilir."
    - k, l) hacimler tefriş edilir;
    - m) "Merdiven ve rampaların çıkış okları çizilir." Rampaların eğimleri ve başlangıç–bitiş kotları yazılır.
  - §14.5.3, uygulama projesi planları (1/50):
    - i) "Taşıyıcı elemanlar (kolon, perde, duvar, pano vb.) ayrı çizim tekniği ve gerçek boyutları gösterilir, içleri
      koyulaştırılır."
    - o) "kolon isimleri ve ölçüleri yazılır."
    - q) "merdiven numarası, basamak adedi, genişlik ve rıht yüksekliği yazılır. Merdiven ve sahanlık aksını gösteren
      çizginin basamakları kestiği noktalar çıkış yönünde numaralanır … korkuluklar çizilir."
  - §9: fikir projesi 1/200, ön ve kesin proje 1/100, uygulama projesi 1/50.
- **TMMOB Mimarlar Odası, *Mimarlık Hizmetleri Şartnamesi*** (28.12.2011, MYK 42/31): fikir projesinde vaziyet,
  planlar, kesitler ve görünüşler vardır; tefriş ön projeyle gelir.
- **Archicad 30 yardımı, *Model View Options: Detail Level of Stair and Railing Symbols*.** Merdiven ve korkuluk
  simgeleri dört grupta (Grid, Structure, Walking Line, Railing) ayrı ayrı açılıp kapanır. Ölçek başına ayrı bir model
  görünümü kurulur. Archicad'in ölçeğe duyarlı kütüphane nesneleri küçük ölçekte sade simgeyle çizilir.
- **Revit 2024 yardımı** (Revit-Customize, GUID-0BB861D5): aile parçaları kaba / orta / ince düzeyde ayrı görünür
  (ADR 8010).

## Karar

- **Düzey duvarınkidir** (`wall::View`), M11'deki gibi: paftada haritanın ölçeği ya da görünüş tarifinin kendi
  ayrıntısı, ekranda yakınlık (yakınlık yalnız düzeyi seçer, ölçek projenin çizim ölçeğidir).
  - **En sade düzey** (`wall::View::simple`, `SIMPLE_BEYOND` = 150): şematik ve 1:150'den küçük, yani 1:200 ve daha
    küçük.
  - M11'in yay sınırı (`opening::SWING_UP_TO`) artık bu tek sabittir.

  | | 1:200 (en sade) | 1:100 (şematik) | 1:50 ve 1:20 (uygulama, ayrıntılı) |
  |---|---|---|---|
  | Kolon | dış çizgi ve dolgu | ve adı (etiket, yer) | ve ölçüsü: 30/50, yuvarlakta Ø40 |
  | Merdiven | bant, basamaklar, kesme çizgisi, ok | ve korkuluklar, basamak numaraları (açıksa) | ve "11 × 17.5/28" |
  | Rampa | bant, sahanlık, ok, eğim, kotlar | ve korkuluklar | aynı |
  | Döşeme kenarı | dış çizgisi | ve korkuluğun ya da parapetin iç çizgisi, uçları | aynı |
  | Mobilya | dış çizgisi | bütün | bütün |

- **Kolonun ölçüsü** santimetreyle, eni/derinliği olarak yazılır (30/50); kare kolonda 30/30, milimetre varsa bir
  ondalık (27.5). Çizimin birimi metre varsayılır, mahal alanında olduğu gibi. Ölçü, adı olmayan kolonda da yazılır:
  standart ölçünün yazılmasını ister.
- **Merdiven bilgisi** rıht sayısı × rıht yüksekliği / basamak genişliğidir, santimetreyle (11 × 17.5/28).
  - İlk rıhtın önüne, yürüme çizgisi boyunca, dik okunacak biçimde yazılır: sola ya da aşağı giden merdivende metin
    döndürülür ve başından dışarı doğru uzanır.
  - Ondalık nokta, çizimin öbür yazılarıyla (kot ve eğim) aynıdır.
- **Basamak numaraları** kullanıcının seçeneğidir (varsayılanı kapalı). Seçenek açıksa 1:100'de de görünür; yalnız
  en sade düzeyde gizlenir. 1:100'de 2 mm'lik numara 2,8 mm'lik basamağa sığıyor; 1:200'de sığmıyor.
- **Rampanın eğim yazısı** bandın içine sığmıyorsa bandın dışına yazılır: yazı yüksekliğinin 1,7 katı bandın yarı
  genişliğini geçerse. Önce 1:200'de 1,2 m'lik rampanın kenar çizgisinin üstüne biniyordu. Bu, bütün çizime (aracın
  önizlemesi dahil) uygulanır.
- **Mobilya:** kütüphane kartının kategorisi "Mobilya/" ile başlayan blok (`block::FURNITURE_CATEGORY`). Kategori
  çizimin blok tanımında saklanır; yerleştirme onu tanımından bilir.
  - En sade düzeyde yalnız bütünü saran kapalı parçalar kalır (`block::outline_pieces`). Bir parça, kutusu bütün
    parçaların kutusunun her iki yönde en az yarısıysa kalır: yatağın çerçevesi kalır, yastıklar gider; yuvarlak
    masa kalır, sandalyeler gider.
  - Böyle bir parçası olmayan blok olduğu gibi çizilir. İlk ve son noktası çakışan çoklu çizgi de kapalı sayılır.
  - Islak hacim ve mutfak blokları her ölçekte bütündür: içleri onları tanıtan şeydir (klozetin dış çizgisi tek
    başına okunmaz).
  - Ekranda aynı bloğu bir kez çizip yerleştirmelere taşıyan hızlandırma parça sayısını denetler ve sadeleşen
    mobilyada kendiliğinden devre dışı kalır.
- **Çizim yolları:**
  - Depo, görünüşe bağlı nesneleri tek yerden söyler (`follows_view`): duvar, açıklık, mahal, aks, kolon, merdiven,
    rampa, etiket, görünüş işareti ve mobilya yerleştirmesi. `set_wall_view` ve `expanded_at` aynı listeyi kullanır.
  - DXF: kolon, merdiven ve rampa `arch_elements::plain_pieces_in`, döşeme `spaces::slab_pieces_in` ile paftanın
    görünüşünde yazılır. Mobilya DXF'te blok yerleştirmesi olarak kalır, tanımı bütündür.
- **Değişmeyenler:**
  - `arch::pieces` (aracın önizlemesi, başvuru fikstürleri) ve Patlat'ın `plain_pieces` bütün çizimi verir; eklenen
    yazılar (ölçü, merdiven bilgisi) orada yoktur.
  - Kolonun dolgusu kullanıcının seçeneğidir. 1:20'de dolgunun yerine beton taraması malzeme kitaplığıyla gelecek
    (M14).
  - Duvar taramaları zaten düzeye bağlıydı (ADR 0339): şematikte dolgu, uygulamada bileşen taramaları.
- **Türkçe:** Proje ayarları › Mimari'nin ayrıntı seçimi "Otomatik" yazıyordu; Görünüş tarifinde aynı seçenek "Ölçeğe
  göre"dir. İkisi de "Ölçeğe göre" oldu. Açıklama metni de "standart" yerine "uygulama" der ve en sade düzeyi anlatır.

## Ölçüm

- **Üç pafta ve PDF** (`crates/native/headless/tests/headless/scales.rs`):
  - Yeni Mimari projesinde şunlar çizildi: S1 adlı 30/50 kolon; 11 rıhtlı, numaralı, iki yanı korkuluklu, 6. rıhtta
    kesilen merdiven; iki yanı korkuluklu %6 rampa; kütüphanenin çift kişilik yatağı.
  - Aynı kat 1:200, 1:100 ve 1:50 paftalara kondu; PDF'ler yazılıp KentOS'un okuyucusuyla geri okundu.

    | Pafta | Çizgi (önce → sonra) | Yazılar (sonra) |
    |---|---|---|
    | 1:200 | 53 → 45 | eğim ve kotlar |
    | 1:100 | 53 → 53 | ve S1, basamak numaraları 1–6 |
    | 1:50 | 53 → 53 | ve 30/50, 11 × 17.5/28 |

  - 1:200'den düşen 8 çizgi: merdivenin korkulukları (kesme çizgisinin iki yanında, 4), rampanın 2 korkuluğu, yatağın
    2 yastığı. Önce üç paftada da S1 ve 1–6 basılıyordu; 30/50 ve merdiven bilgisi hiçbirinde yoktu.
  - Resimler: `.run/gui-evidence/mimari-m12/m12-once.png` ve `m12-sonra.png`.
- **Gerçek pencere** (Xvfb 1440×900, gerçek fare ve klavye; `fixtures/interaction/real/kolon-merdiven-pafta.txt`):
  - El hareketleri:
    - Kolon aracında İlk etiket "S1";
    - Merdiven aracının panelinde Basamak numaraları açık, Korkuluk › İki yan; iki tık ve Enter;
    - Rampa aracında iki tık ve Enter;
    - Ekle › Kitaplık, Bloklar panelinde "kişilik" araması ve kartın çift tıkı, bir tık;
    - Model'in yanındaki + ile pafta, görünüm penceresinin Ölçek alanına 50 ve 200.
  - Aynı senaryo M12'den önceki ikiliyle de koşuldu: `.run/gui-evidence/mimari-m12/m12-gercek-pafta.png`.
    - 1:200'de önce S1, sıkışık numaralar, korkuluklar, kenara binen eğim yazısı ve yastıklar vardı; sonra yoktur.
    - 1:50'de önce yalnız "S1" yazıyordu; sonra "S1 / 30/50" ve "18 × 16.7/28".
- **Testler:**
  - `arch_levels::a_column_has_no_words_at_1_200_its_name_at_1_100_and_its_size_from_1_50`;
  - `arch_levels::a_stair_drops_its_railings_and_numbers_at_1_200_and_writes_its_risers_from_1_50`: sağa, sola ve
    aşağı giden merdivende yazının yeri, açısı ve hizası;
  - `arch_levels::a_ramp_drops_its_railings_at_1_200_and_keeps_its_slope_off_its_edge`;
  - `arch_levels::a_slab_edge_is_its_outer_line_at_1_200`;
  - `arch_levels::furniture_is_its_outline_beyond_1_150`: yatak, yuvarlak masa ve dört çizgiden bir kutu;
  - `store::inserts::furniture_is_its_outline_in_the_simplest_view`: deponun görünüşü değişince yatak 1 / 3 / 3 parça,
    mobilya olmayan blok her görünüşte bütün;
  - `arch_elements::dxf_writes_a_column_and_a_stair_at_the_files_scale`;
  - başvuru fikstürlerinin `arch::arch_elements_draw_as_the_reference_says` testi değişmeden geçer.

## Kalanlar

- Kolonun 1:20 beton taraması ve malzeme tarama kitaplığı (M14).
- Merdivenin numarası (M1 … Mn) ve merdiven, rampa yazılarının çakışmasız yerleşimi (M16, M28).
- Mobilyanın parça parça ayrıntı düzeyi (kütüphane bloklarında parçanın görüneceği düzey); DXF'te sade mobilya tanımı.
- Islak hacim ve mutfak bloklarının en sade gösterimi.
- Ölçeğe göre çizgi grubu (ISO 128-23 §5): bu işte yapılmadı, ayrı bir madde olarak plana eklenecek.
