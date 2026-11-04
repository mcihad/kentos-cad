# 0932 — KentOS Yol: kurpta tasarım taşıtına göre genişletme

Tarih: 2026-11-04. plan/road.md Y28 (ROAD-DEVER-05). Durum: Y28.1 kabul; Y28.2–Y28.3 bu belgeyle ilerler.

## Dayanak

- **AASHTO Green Book (metrik).** Kurpta kaplama genişliği W_c = N(U + C) + (N − 1)·F_A + Z'dir. Genişletme
  w = W_c − W_n'dir (W_n teğetteki genişlik).
  - Taşıtın kurptaki izi U = u + R − √(R² − ΣLᵢ²)'dir; u teğetteki iz genişliğidir (lastik dışından dışa).
  - Ön çıkıntının genişliği F_A = √(R² + A(2L + A)) − R'dir; A ön çıkıntı, L çekicinin dingil aralığıdır.
  - Sürüş güçlüğü payı Z = 0,1·V/√R'dir (V km/sa, R m). ABD birimlerinde Z = V/√R'dir.
  - Yanal açıklık C, teğetteki 6,0 / 6,6 / 7,2 m genişliğe karşı 0,6 / 0,75 / 0,9 m'dir.
  - Denklemlerin ABD birimli hâli [Open Highway Curve Widening](https://www.hwyplans.com/HTMLPage30.html) sayfasında
    terim terim yazılıdır. Green Book'un kendisi (Denklem 3-34, 3-35) çevrimiçi okunamadı. Metrik Z ve C ikincil
    kaynaklardan ve ders notlarından doğrulandı.
- **Kararlı durum izi (Heald, [TRR 1052](https://onlinepubs.trb.org/Onlinepubs/trr/1986/1052/1052-010.pdf); sayfa
  görüntüleri incelendi).** Kararlı dönüşte her sabit dingilin uzantısı kurbun merkezinden geçer ve dik üçgenler zinciri
  kurulur. WHI formülü OT = R − (R² − ΣL²)^½'dir.
  - Formül en büyük değeri verir. Kararlı duruma ulaşmayacak kadar kısa dönüşte formül taşıtın izini abartır.
  - Çok uzun taşıt kısa yarıçapta (merkez en arka dingilin izi ile kurp arasında kalınca) formül çöker.
  - ΣL² dingil aralıklarının kareleri toplamı değildir. Mafsalın ötelemesi eksi sayılır.
- **Civil 3D:** tasarım ölçütü dosyasında iki yöntem vardır: AASHTO Standard (3-31, 3-32, 3-33) ve AASHTO Simple
  (yalnız 3-31). Varsayılan taşıt WB-19'dur
  ([tasarım ölçütü dosyaları](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-UserGuide/files/GUID-6EEBA97D-7732-4099-9BA2-E830A2EE4E23.htm)).
- **OpenRoads Designer:** Create Curve Widening, `.wid` tablosundan yarıçapa göre iç ve dış genişliği ve geçiş boyunu
  okur. Tabloda olmayan yarıçap bir küçük satırın değerini alır, ara değer bulunmaz. Genişlik üç yatay denetimle
  uygulanır: rampa, sabit ve rampa
  ([Calculating Widening](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-2FB67443-F5BE-2C14-8842-661815C398D4.html)).
  Taşıt girdisi yoktur; tablo kurumun standardıdır.
- **Civil Designer:** iç, dış ya da iki yana uygulanır. Geçiş eğrisindeki yüzde, en büyük genişlik, AASHTO Simple,
  Standard ya da tablo seçilir. Girdiler normal şerit genişliği, sol ve sağ şerit sayısı, taşıtın dingil aralığıdır
  ([Curve Widening Settings](https://help.civildesigner.com/91/Roads_Mode/Dialogs/Alignment/Curve_Widening_Settings.htm)).
- **WSDOT Tasarım Kılavuzu 1240 (Eylül 2024):** şerit başına 0,5 ft'ten (0,15 m) ya da toplam 2 ft'ten (0,6 m) az
  genişletme gözardı edilebilir. Genişletme içe ya da iki yana eşit yapılır, yalnız dışa yapılmaz. Kurbun tamamında
  sürer; 6 ft'e kadar 1:25, üstünde 1:15 rampayla bağlanır.
- **Türkçe ders kaynakları ve Netcad NETPRO:** klasik formül n·L²/(2R) + k·V/√R'dir. Geçiş eğrisi varsa genişletme
  klotoidin başında sıfırdır ve dairede tam değere ulaşır; yoksa dever uygulamasıyla birlikte gelir. NETPRO'nun
  genişletme hesabına dair çevrimiçi belge bulunamadı.
- **Matematik:** n·L²/(2R), R − √(R² − L²)'nin ilk terimidir. Kalan yaklaşık L⁴/(8R³)'tür. R − √(R² − Σ) farkı büyük
  yarıçapta iki yakın sayının farkıdır ve hane kaybeder. Σ/(R + √(R² − Σ)) cebirsel olarak aynıdır ve kayıpsızdır.

## Karar

### §1 Çekirdek (`kentos_road::widening`, Y28.1)

- `WideningVehicle { track, front, wheelbase, trailer: Option<(kingpin, wheelbase)> }`. `sum_of_squares` ΣLᵢ² =
  L₁² − k² + L₂²'yi verir. `kingpin`, Y17'nin `swept::Trailer`'ındaki anlamdadır: arka dingilin arkasında artı, önünde
  eksi. Karesi alındığı için iki yönde aynı sonucu verir.
- `offtracking(R, Σ) = Σ/(R + √(R² − Σ))`'dir; R² ≤ Σ ise yoktur (`Unturnable`). `front_overhang` aynı biçimdedir.
  `allowance` 0,1·V/√R'dir.
- `widening(R, taşıt, yol)` U, F_A, Z, W_c ve w'yi verir. `WideningMethod::Standard` tam formüldür, `Simple` N(U + C)'dir.
  Hız yoksa Z sıfırdır. Eksi w, teğetteki genişliğin yettiğini söyler ve olduğu gibi döner.
- `rounded(w, step, least)`: `least`'ten az ya da artı olmayan genişletme yazılmaz. Kalan `step`'in katına yukarı
  yuvarlanır. Kat olan değer 1e-9 payla kendinde kalır (0,6 → 0,7 olmaz).
- **Hiçbir sayı KentOS'tan gelmez.** u, A, L, C, adım ve en az genişletme kurumun taşıtı ve standardıdır.

### §2 Kararlı durum ile sürüş (ölçüm)

Y17'nin tractrix sürüşü (`swept::drive`, 0,25 m adım), WB-19 boyutlarında bir çekici ve römorkla gerçek kurplarda
sürüldü (L₁ 6,6, mafsal −0,8, L₂ 12,5 m). Sonuç her durumda kararlı durumun altında kaldı (1 cm sürüş toleransıyla):

| R (m) | Klotoid (m) | Sapma | Sürüş (m) | Kararlı (m) | Oran |
|---|---|---|---|---|---|
| 60 | 0 | 15° | 0,984 | 1,683 | %58 |
| 60 | 0 | 45° | 1,590 | 1,683 | %94 |
| 60 | 0 | 90° | 1,680 | 1,683 | %100 |
| 150 | 0 | 10° | 0,523 | 0,665 | %79 |
| 150 | 0 | 30° | 0,662 | 0,665 | %99 |
| 150 | 60 | 30° | 0,630 | 0,665 | %95 |
| 150 | 60 | 60° | 0,665 | 0,665 | %100 |
| 300 | 0 | 5° | 0,261 | 0,332 | %79 |
| 300 | 80 | 20° | 0,323 | 0,332 | %97 |

Öneri AASHTO gibi kararlı durumdan yapılır, çünkü güvenli yandadır. Kısa kurpta formülün ne kadar abarttığı Y28.3'te
pencerede sürüşle birlikte gösterilir.

Bekçi: `tests/all/widening.rs::the_steady_state_bounds_the_drive`.

## Doğrulama

26 ortak durum (`scripts/fixtures/road_widening_cases.py`) 40 basamakla üretilir. Referans dönüşü dingil dingil
kurar. Ön dingil R çemberindedir, her dingilin doğrusu merkezden geçer. Arka dingil ve römork dingili ikiye bölmeyle
bulunur. İz, en arka dingilin merkeze uzaklığından okunur. Kapalı formül kullanılmaz.

Durumlar şunları kapsar: WB-19 (60–20 000 m), mafsalın iki yönü, WB-12, SU-9, BUS-12, binek taşıt, hızsız ve basit
yöntem, 1–4 şerit, çıkıntısız taşıt, sınıra yakın yarıçap, iki kararlı dönemez durum ve beş yuvarlama.

Büyük yarıçapta hane korunur: R = 10⁶ m'de fark biçimi 10⁻⁸'den fazla bağıl hata verir, KentOS'unki 10⁻¹²'nin
altındadır.

## Açıklar

- Dış kenara ya da iki yana genişletme. Bugün dever çizelgesinin genişletmesi iç yandadır.
- Rampa oranıyla (1:25, 1:15) geçiş boyu. Bugün genişletme dever geçişini izler.
- OpenRoads'un `.wid` tablosu gibi kurum tablosundan okuma.
- Sürüşle (geçici izle) öneri.
