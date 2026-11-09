# 3040 — Düşey model: duvarın tepesi, kat yüksekliği zinciri, döşeme katmanları

Tarih: 2026-11-09. Durum: kabul (M40; ADV-18).
- §1: duvar üst döşemenin altında biter, katın yüksekliğini geçmez; bir katın yüksekliği değişince üstündeki katlar
  kayar.
- Kalan: döşeme katmanları (§2), mahalin döşeme kotu, asma tavan ve iç yükseklik (§3).

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

## Sınırlar

- Metrajın "kural" yazısı değişmedi ("taban çizgisinin uzunluğu × yükseklik …"); yükseklik artık bu kuralla bulunur.
- Duvarın tabanı katın kotudur; döşemenin şap ve kaplaması (§2) gelince taban betonarmenin üstüne inmeli.
- Döşemenin tadilat durumu duvarın yüksekliğine bakılmıyor: mevcut duvarın üstüne ilave döşeme de onu kısaltır.
