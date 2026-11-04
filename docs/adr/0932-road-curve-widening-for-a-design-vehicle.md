# 0932 — KentOS Yol: kurpta tasarım taşıtına göre genişletme

Tarih: 2026-11-04. plan/road.md Y28 (ROAD-DEVER-05). Durum: Y28.1–Y28.3 kabul.

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

### §3 Komut (`road.superelevations.widen` v1, Y28.2)

Salt okunur sorgudur; öneriyi çizelgeye Dever penceresi `road.superelevations.edit` ile yazar.

- **Girdi:** dever çizelgesi ve tip kesit (kimlik ya da ad). Taşıt `road.swept.query`'nin `SweptVehicleDef`'idir; Y17'nin
  taşıtı ikinci kez tanımlanmaz. `track` lastiklerin dıştan dışa iz genişliğidir, yoksa taşıtın genişliği alınır.
  `clearance` (C, şerit başına) zorunludur. `speed`, `method` (`standard` | `simple`), `step`, `least` ve `drive`
  isteğe bağlıdır.
- **N ve W_n tip kesitten gelir:** iki yanın şeritlerinin sayısı ve teğetteki genişliklerinin toplamı. Şeritsiz tip
  kesit reddedilir.
- **Çıktı:** her kurp için PI, TS, ST, R, iz, U, F_A, Z, W_c, genişletme, öneri, çizelgedeki genişletme (`current`;
  km satırlı çizelgede 0) ve `drive` ile sürüşün izi (`driven`). Kararlı dönülemeyen kurplar `unturnable`'da listelenir,
  satır açılmaz.
- **Sürüş:** taşıtın ön dingili aks üzerinde, TS'den taşıt boyunun iki katı artı 10 m önce başlar ve ST'ye kadar
  0,25 m adımla sürülür (`swept::drive`). En arka dingilin ön dingil izine en büyük uzaklığı okunur.
- **Kodlar:** `invalid_widening` (taşıt, iz, açıklık, hız, adım, en az genişletme, şeritsiz tip kesit) ve
  `widening_unturnable` uyarısı. Eskimiş çizelgede `superelevation_stale` de gelir.

27 ortak durum `scripts/fixtures/road_widening_command_cases.py`'dir. Genişletme Y28.1'in dingil dingil referansından
gelir. Sürüş referansı bisiklet modelinin diferansiyel denklemleridir: ön dingilin yeri de aksın eğriliğinden
tümlenir, dördüncü mertebe Runge–Kutta 2,5 cm adımla. KentOS tractrix'i 25 cm kirişlerde kesin sürer. İkisi 5 mm
içinde aynı çıkar. Ölçülen WB-19 durumunda (R 150 m, 40 m klotoid, 47° sapma) referans 0,6648, kararlı durum 0,6654 m
verir.

### §4 Masaüstü (Y28.3)

Dever penceresinin alt çubuğunda **Genişletme öner…** vardır; seçili çizelge yazıldığı hâliyle okunuyorsa açıktır.
Genişletme önerisi penceresi ortak düzendedir (DESIGN.md §7.19) ve Dever'in yerine açılır. Geri Dever'e döner.

- **Form:** Taşıt (dingil aralığı, ön çıkıntı, genişlik, iz genişliği, Römork Yok | Var, mafsal, römork dingili),
  Yol (tip kesit, yanal açıklık, proje hızı), Öneri (Standart | Basit, yuvarlama, en az genişletme, Sür). Taşıt Araç izi
  penceresi açıksa ondan, tip kesit ve hız Dever'in Denetim bölümünden gelir. Başka hiçbir alan doldurulmaz.
  Boş alan, formdaki sırayla adıyla söylenir ("Dingil aralığı (m) yazın.").
- **Çizim:** taşıt için genişletmenin yarıçapa göre eğrisi (yatay eksen logaritmik), aksın kurpları nokta olarak ve en
  az genişletme kesikli çizgi olarak çizilir. Eğri aksın kurplarının çevresinde çizilir: en küçük yarıçapın üçte
  birinden en büyüğün dört katına kadar, en az 1000 m.
  - İlk sürüm eğriyi taşıtın dönemediği yarıçapa (≈15 m) kadar indiriyordu. Dikey eksen 22 m'ye çıkıyor, etiketler üst
    üste biniyor, p1 (2 m) dipte eziliyordu. Şimdi 5,3 m'de biter, en çok altı ızgara çizgisi vardır.
- **Özetler:** kurp sayısı, en büyük öneri, en büyük kararlı iz, sürüşte izin kararlıya en küçük oranı.
- **Tablo:** PI, R, iz, F_A, Z, genişletme, öneri, çizelgedeki değer ve Sür ile sürüşteki iz. U ve W_c tabloda
  yoktur, çünkü öbürlerinden çıkar: on bir sütun 1100 px'te öneriyi kesiyordu.
- **Çizelgeye yaz:** önerileri Dever'in Genişleme hücrelerine yazar (sıfır hücreyi boşaltır) ve Dever'e döner.
  Yazma işini Dever'in Kaydet'i yapar. Km satırlı çizelgede bu düğme kapalıdır ve bir not söyler. Çizelgede satırı
  olmayan kurp bir uyarıyla söylenir.
- **Ölçüm** (örnek çizim, WB-19 boyutları, C 0,9 m, 80 km/sa, 0,1 m'ye yukarı): p1 (R 150 m) için kararlı iz 0,665 m,
  F_A 0,058 m, Z 0,653 m ve genişletme 2,022 m çıkar; öneri 2,10 m'dir. Sürüş kararlı izin %100'üne ulaşır (47°'lik kurp
  yeterince uzun). Çizelgeye yazılınca Dever'de en dik göreli eğim 1:148'den 1:87'ye iner: iç yan genişlerken kenarı
  da yükselir.

Bekçi: `widening_window_tests` (öneri okunur ve çizelgeye yazılır; eksik ölçü adıyla söylenir).

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
