# 3040 — Düşey model: duvarın tepesi, kat yüksekliği zinciri, döşeme katmanları

Tarih: 2026-11-09. Durum: kabul (M40; ADV-18).
- §1: duvar üst döşemenin altında biter, katın yüksekliğini geçmez; bir katın yüksekliği değişince üstündeki katlar
  kayar.
- §2: döşeme katmanları (kaplama, şap, yalıtım, betonarme, sıva); duvar betonarmeden betonarmeye; KCAD 220.
- Kalan: mahalin döşeme kotu, asma tavan ve iç yükseklik (§3).

## Sorun

- **Duvar döşemenin içinden geçiyordu.** Kendi yüksekliği olmayan duvar katın yüksekliğini (döşeme üstünden üst katın
  döşeme üstüne) alıyordu. Üst katın döşemesi çizilse de duvar onun içinden geçiyordu:
  - Metraj duvarın net yüzüne döşemenin kalınlığı × duvarın boyu kadar fazla alan katıyordu. Döşemenin hacmi ayrıca
    sayıldığından o kısım iki kez sayılıyordu.
  - İç görünüşler duvarı döşemenin içine kadar çiziyordu.
- **Kat yüksekliği değişince üstteki katlar yerinde kalıyordu.** Proje ayarları › Mimari'de kot ve kat yüksekliği ayrı
  ayrı yazılıyordu. Zemin katı 3,00'ten 3,24'e çıkarmak 1. katın kotunu 3,00'te bırakıyordu; katlar üst üste
  oturmuyordu, 1. katın döşemesi ve duvarları da kıpırdamıyordu. Plan maddesi ADV-18'in ilk kabulü ("kat yüksekliği
  değişince duvar tepeleri izler") bu yüzden tutmuyordu.

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı

- **Planlı Alanlar İmar Yönetmeliği:**
  - 4(1)(rr) **kat yüksekliği:** binanın bir katının döşeme üstünden bir üstteki katın döşeme üstüne mesafe.
  - 4(1)(ıı) **iç yükseklik:** taban döşeme kaplaması üzerinden tavan kaplamasına net mesafe (§3).
  - 4(1)(yyy) subasman kotu: zemin kat taban döşemesinin üst kotu, ±0,00 ile +1,20 arası.
  - 28(1) kat yükseklikleri en çok (döşeme üst kotundan döşeme üst kotuna): konutta 3,60 m, bodrum hariç 3 katı geçmeyen
    binada 4,00 m; ticarette zemin 4,50 m. 28(4) iskân edilen katın iç yüksekliği en az 2,60 m; 28(5) ıslak hacim,
    kiler, geçit ve iskân edilmeyen bodrumda 2,20 m.
  - Sonuç: kat bir yığın: her katın kotu altındakinin kotu + yüksekliği. Duvar ise iki döşeme arasındaki boşluğu doldurur.
- **Birim fiyat tarifleri:** kârgir duvar "yapılan duvarın" ölçüsüyle ödenir (Kültür ve Turizm Bakanlığı 2022 birim
  fiyat listesi; Çevre ve Şehircilik Bakanlığı 2026 inşaat birim fiyatları). Tuğla duvar alttaki döşemenin üstünden
  üstteki döşemenin altına örülür; döşemenin içinde duvar yoktur. Ayrı bir "döşeme altına kadar" cümlesi bulunamadı;
  kural, yapılan duvarın sınırından çıkar.

### Döşeme katmanları (§2)

- TS 825 yapı elemanını kalınlığı ve ısıl iletkenliği olan katmanlarla hesaplar.
- Ara kat döşemesinde Türkiye'de ısı yalıtımı yaygın değildir.
- Teras döşemesinin dizilişi yukarıdan: koruma ve kaplama, su yalıtımı, eğim betonu ya da şap, ısı yalıtımı, buhar
  kesici, betonarme (DergiPark, "Teras çatılarda ısı yalıtımı", makale 202817).
- PAİY'nin sözcükleri: "taban döşeme kaplaması" ve "tavan kaplaması" (4(1)(ıı)). "Kaba kot" ve "ince kot" bu
  araştırmada doğrulanamadı, kullanılmadı.

### Başka uygulamalar (yalnız görünüş için)

- Revit: duvarın "Top Constraint"i bir kata ya da yüksekliğe bağlıdır; döşemeyle birleşince duvarın tepesi kırpılır.
- Archicad: kat yüksekliği değişince üstteki katların kotu kayar (Story Settings).

## Karar

### 1. Duvarın tepesi ve kat zinciri

- **Duvarın yüksekliği** (`Document::wall_height`, tek kural): kendi yüksekliği varsa o. Yoksa üstündeki en alçak
  döşemenin alt yüzüne kadar, katın yüksekliğini hiç geçmeden. Üstünde döşeme yoksa katın yüksekliği (eskisi gibi).
  - "Üstündeki döşeme": halkası duvarın yolunun ortasını içeren ve alt yüzü duvarın tabanından 10 cm'den yukarıda olan
    döşeme. Kendi katının döşemesi altında kalır, hesaba girmez.
  - Katın yüksekliğini geçmemesi: döşemesi iki kat yukarıda olan galeri boşluğundaki duvar kendi katında biter.
  - Taban: katın kotu (katı yoksa 0).
- **Okuyanlar aynı kurala bağlandı:** Metraj (`spaces_quantities`: net yüz, hacim), değer sütunları ve etiketler
  (`arch_values`: yükseklik, alan, hacim), iç görünüşler (`elevations`).
- **Dosya ve komut değişmedi.** `height` boşken anlamı değişti; şema aynı. Eski bir dosyada üstünde döşeme olan duvar
  artık döşemenin içinden geçmez; metraj döşeme kalınlığı × duvarın boyu kadar azalır. Bu bir düzeltmedir.
- **Öznitelikler'in Yükseklik satırı** yüksekliğin nereden geldiğini ve sayıyı söyler: "Üst döşemenin altı · 2.850",
  "Katın · 3.000". Sayı yazılınca sabit yükseklik olur (eskisi gibi).
- **Kat zinciri:** Proje ayarları › Mimari'de bir katın yüksekliği değişince, o katın tepesinde ya da üstünde duran
  katların kotu aynı farkla kayar, yazarken canlı. Kot milimetreye yuvarlanır. Silip yeniden yazınca kayma son geçerli
  yükseklikten ölçülür. Kotu sayı olmayan kat kımıldamaz. Döşemenin üstü katının kotuna göre olduğundan üst katın
  döşemesi ve duvarları onunla gelir.
- İki katı geçen duvar (bir üst kata bağlı duvar) bu bölümde yok; plan istemiyor.

### 2. Döşeme katmanları

- **Model:** `SlabFields.thickness` taşıyıcının (betonarme) kalınlığı olarak kalır; eski dosyalar aynı anlamda okunur.
  Yeni `layers`'da yukarıdan iki liste var:
  - `over`: taşıyıcının üstündekiler (kaplama, şap, ısı yalıtımı, su yalıtımı).
  - `under`: altındakiler (sıva).
  - Her katmanın türü, kalınlığı (0 < t ≤ 1 m) ve isteğe bağlı malzemesi var; bir yanda en çok 8 katman olur.
- **Kotlar:**
  - Döşemenin üstü (katın kotu + kot farkı) bitmiş döşemedir; PAİY 4(1)(rr)'nin kat yüksekliği oradan ölçülür.
  - Taşıyıcının üstü: üst − üst katmanlar (`structure_top`).
  - Taşıyıcının altı: üstü − kalınlık (`structure_underside`).
  - Tavan: altı − alt katmanlar (`ceiling`).
  - Toplam kalınlık: `whole_thickness`.
- **Duvar betonarmeden betonarmeye (§1'in güncellemesi):**
  - Tuğla, şap ve kaplamadan önce betonarmenin üstüne örülür. Tavanın sıvası duvarı da döşemeyi de örter.
  - Duvarın tabanı, kendi katındaki döşemenin taşıyıcı üstüdür; döşemenin üstü katın kotundan yarım metre aşağıdan
    10 cm yukarıya kadar olabilir (alçaltılmış ıslak hacim dahil).
  - Duvarın tepesi, üstündeki döşemenin taşıyıcı altıdır.
  - Katmansız döşemede ikisi de eskisi gibidir.
- **Dosya:** `.kcad` şema 220. Bir döşemenin katmanı varsa dosya 220 olur; yoksa eski şemasıyla bayt bayt aynı.
  - Şema 219 dosyasında katman `unknown_field`.
  - Bağımsız Python okuyucusu (`tools/kcad/kcad.py`) ve yazıcısı (`kcad_v2_reference.py`) aynı kuralla.
  - Çok yeni şema örneği 221'e kaydı.
- **Komutlar:**
  - `cad.slabs.create` ve `cad.slabs.edit` (Değiştir) `layers` alır; düzenlemede boş liste katmanları kaldırır.
  - "Değiştirilecek bir alan verilmedi" iletisi katmanları da sayar.
  - Metraj (`cad.spaces.quantities`) döşeme satırında `layers` verir: her katmanın türü, malzemesi, kalınlığı, net alanı
    ve hacmi. `volume` taşıyıcınındır.
- **Öznitelikler:**
  - Katmanlar satırı dört hazır dizilişten birini bütün olarak verir. Kalınlıklar olağan değerlerdir, hepsi
    değiştirilebilir:
    - Yalnız taşıyıcı.
    - Ara kat döşemesi: kaplama 2, şap 5 cm; altında sıva 2 cm.
    - Zemine oturan döşeme: kaplama 2, şap 5, ısı yalıtımı 5, su yalıtımı 1 cm.
    - Teras döşemesi: kaplama 3, su yalıtımı 1, şap 5, ısı yalıtımı 8 cm; altında sıva 2 cm.
  - Dizilişin adı ya da katmanlar tek tek yazılır ("Kaplama 0.020 · Şap 0.040 · Taşıyıcı 0.150 · Sıva 0.020").
  - Her katmanın kalınlığı kendi satırında yazılır, 0 yazılan katman kalkar. Ardından Toplam kalınlık gelir.
- **Metraj penceresi:** Döşemeler sekmesine beş sütun geldi: Kaplama, Isı yalıtımı, Su yalıtımı ve Sıva alanla, Şap
  hacimle; aynı türden katmanlar toplanır. Tablo hesap tablosunda toplanabilsin diye tür başına bir sütun var.
- **Ortak durumlar** (`slab_cases.py`, `space_command_cases.py`):
  - Oluşturmada ara kat dizilişi ve metrajın katman satırları; sıfır kalınlıklı katman reddi.
  - Düzenlemede teras dizilişi; boş katmanlar katmanları kaldırır.

## Doğrulama

- Domain (`wall_height.rs`): döşemesiz 3,00; üst katın 15 cm döşemesi altında 2,85; sabit yükseklik önce gelir; üst katın
  duvarı 3,00; döşemenin dışındaki duvar 3,00; iki kat yukarıdaki döşeme katı aşmaz (3,00); iki döşemeden alçağı (2,55).
- Öznitelikler (`a_wall_s_height_row_says_it_stops_under_the_slab_over_it`): "Katın · 3.000", döşeme çizilince "Üst
  döşemenin altı · 2.850".
- Proje ayarları (`a_storey_s_height_moves_the_levels_above`): zemin 3 → 3,24 yazılınca 1. ve 2. kat 3,24 ve 6,24; bodrum
  ve kendi kotu yerinde; silip 3 yazınca 3,00 ve 6,00'a döner.
- Masaüstü (`wall_height_tests.rs`), iki katlı bina:
  - Döşemesiz zemin duvarları 3,00.
  - 1. katın 15 cm döşemesi çizilince zemin 2,85, 1. kat 3,00 (üstünde döşeme yok).
  - Her zemin duvarının net yüzü boy × 0,15 kadar azaldı.
  - Proje ayarları'nda zemin 3,24 yapılıp kaydedilince 1. kat 3,24'e çıktı, zemin duvarları 3,09.
- Gerçek pencere (iz `fixtures/interaction/real/duvar-tepesi.txt`, 1920×1080). İki katlı bina, 15 × 6 m, döşemesiz.
  Zemin katın 5 m'deki bölmesi seçildi:
  - Öznitelikler'de Yükseklik "Katın · 3.000".
  - 1. katta Döşeme aracı dış yüzlerle döşemeyi çizdi ("Döşeme oluşturuldu: 90.00 m²"). Zemin katta aynı bölme "Üst
    döşemenin altı · 2.850".
  - Proje ayarları › Mimari'de Zemin kat'a 3.24 yazılırken 1. katın kotu 3.24 oldu. Kaydedilince bölme "Üst döşemenin
    altı · 3.090".
  - ÖNCE (M39'un ikilisi): üç adımda da "Katın"; 1. katın kotu 3'te kaldı.
- Testler: 13 paket 7523 geçti; masaüstü 2041 geçti (tek kırmızı işten önce de kırmızı olan
  `module_surface_tests::each_program_docks_its_own_panels`).
- Döşeme katmanları, §2:
  - Domain (`a_wall_runs_from_concrete_to_concrete`): iki kat da ara kat dizilişli. Zemin duvarı 2,78 − (−0,07) =
    2,85 m. Üstünde döşeme olmayan 1. kat duvarı kendi betonarmesinden bir üst kata 3,07 m.
  - Öznitelikler (`a_slab_s_layers_are_picked_whole_and_typed_one_by_one`):
    - "Yalnız taşıyıcı" → Ara kat döşemesi: Kaplama 0.020, Şap 0.050, Sıva 0.020, Toplam kalınlık 0.240.
    - Şap "4 cm" yazılınca satır katmanları tek tek söyler.
    - Kaplama 0 yazılınca kalkar.
  - Masaüstü (`wall_height_tests.rs`): 1. katın döşemesi ara kat dizilişini alınca zemin duvarları 3,24 − 0,07 − 0,15
    = 3,02 m. Zemin katın döşemesi de aynı dizilişle çizilince duvarlar 3,09 m (betonarmeden betonarmeye).
  - KCAD: `slab-layers` örneği ve şema 219'daki bozuk dosyası; 1723 dosya bağımsız okuyucuyla tutarlı; Rust 251
    geçerli örnek.
  - Ortak durumlar: `cad.slabs.create` 49, `cad.slabs.edit` 40 durum. Rust ikisinde de bağımsız başvuruyla aynı
    (`every_command_case_matches_the_desktop_handlers`).
  - Gerçek pencere (iz `fixtures/interaction/real/doseme-katmanlari.txt`):
    - İki katlı binada 1. katın döşemesi çizildi ve pencereyle seçilip Öznitelikler'de Döşeme (1) seçildi.
    - Katmanlar "Yalnız taşıyıcı" iken liste dört dizilişi gösterdi.
    - Ara kat döşemesi seçilince Kaplama 0.020, Şap 0.050, Sıva 0.020, Toplam 0.240.
    - Zemin katın bölmesi "Üst döşemenin altı · 2.780".
    - Metraj › Döşemeler: Kaplama 90.00, Şap 4.500. Duvarların toplam net yüzü 307,13 m²'de kaldı, çünkü zemin
      duvarları 7 cm kısaldı, 1. katınkiler kendi betonarmesine oturduğu için 7 cm uzadı.
    - ÖNCE (M39'un ikilisi): satır yok; aynı tıklama Kenar satırına düştü.
  - Testler: 13 paket 7528 geçti; masaüstü 2042 geçti (aynı tek kırmızı).

## Sınırlar

- Metrajın "kural" yazısı değişmedi ("taban çizgisinin uzunluğu × yükseklik …"); yükseklik artık bu kuralla bulunur.
- Döşemenin tadilat durumu duvarın yüksekliğine bakılmıyor: mevcut duvarın üstüne ilave döşeme de onu kısaltır.
- Döşeme aracı katman sormuyor: diziliş Öznitelikler'de verilir. Projenin varsayılan dizilişi yok.
- Katmanlar kesitte ve 3B'de çizilmiyor (kesit türetimi M42). Yerden ısıtma gibi özel dizilişler elle yazılır.
- Metraj penceresinin özeti ("1 döşeme, net … m²") katmanları saymaz; katmanlar Döşemeler sekmesindedir.
