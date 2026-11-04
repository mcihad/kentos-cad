# 0935 — KentOS Yol: düşey kurp boyunun görüşten, konfordan ve bisektristen önerilmesi

Tarih: 2026-11-04. plan/road.md Y31 (ROAD-PROFILE-13). Durum: Y31.1 kabul; Y31.2–Y31.3 bu belgeyle ilerler.

## Dayanak

- **AASHTO Green Book (metrik):** durma görüş mesafesi tepki yolu ile fren yolunun toplamıdır:
  `0,278·V·t + V²/(254·(a/9,81 ± G))`.
  - t = 2,5 s, sürücülerin yaklaşık %90'ını kapsar. a = 3,4 m/s² için sürücülerin yaklaşık %90'ı bundan sert yavaşlar.
    İkisi de yüzdeliktir, ortalama değildir.
  - Göz 1,08 m (eski baskılarda 1,07 m), engel 0,60 m yüksekliktedir (1994 öncesi 0,15 m). Tepe için
    `L = A·S²/658` (S ≤ L), 658 = 200(√1,08 + √0,6)².
  - Çukurda far yüksekliği 0,60 m, ışın 1°'dir: `L = A·S²/(120 + 3,5·S)`. Konfor için `L = A·V²/395`.
  - Kaynaklar: [Wikipedia, Stopping sight distance](https://en.wikipedia.org/wiki/Stopping_sight_distance), NYSDOT
    çalışma notları, [FHWA'nın durma görüş mesafesi notu](https://webarchive.library.unt.edu/eot2008/20080916011951/http:/safety.fhwa.dot.gov/geometric/mitigationstrategies/chapter3/3_stopdistance.htm).
    Green Book'un metni çevrimiçi okunamadı.
- **Civil 3D:** ölçüte dayalı tasarımda tasarım hızına göre en küçük K tablosu kullanılır (tepe için durma görüşü, çukur
  için kurp boyu). Uymayan kurp K değiştirilerek ya da boy ve eğim ayarlanarak düzeltilir
  ([yardım](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-FFE978DE-6282-4D40-8C16-7A1D6288D7A1.htm)).
- **Novapoint:** çukurda durma görüşü ile üst engel için ayrı K değerleri, tepede geçme görüşü K'sı vardır. Küçük eğim
  farklarında K'yı alt sınırın epey üstünde tutmayı önerir.
- **NETPRO ve KGM:** bisektris, açık ve kapalı kurp katsayısı, DGM ve GGM ölçütleri kullanılır (ROAD-PROFILE-13'ün dili).
  Tebliğin tabloları bu oturumda çevrimiçi bulunamadı.

## Karar

### §1 Çekirdek (`kentos_road::profile::lengths`, Y31.1)

- **`stopping(V, t, a, G)`:** `v·t + v²/(2(a + g·G))`, v = V/3,6, g = 9,80665 (Y27 ile aynı). AASHTO'nun 254'ü 2·9,81·3,6²'nin
  yuvarlanmışıdır; fark %0,1'dir. Eğim frenin tümünü alıyorsa (`a + g·G ≤ 0`) mesafe yoktur.
- **`crest(A, S, h₁, h₂)`:** parabol teğetinden `A·x²/(200L)` düşer, görüş çizgisi parabolü teğet keser. S ≤ L için
  `L = A·S²/(200(√h₁ + √h₂)²)`, S > L için `L = 2S − 200(√h₁ + √h₂)²/A`. Eğimler görüşü kendi başına veriyorsa sonuç 0'dır:
  kurp istenmez.
- **`headlight(A, S, h, β)`:** S ≤ L için `L = A·S²/(200(h + S·tan β))`, S > L için `L = 2S − 200(h + S·tan β)/A`.
- **`comfort(A, V, a_v)`:** `R = 100L/A` ve `v²/R = a_v` olduğundan `L = A·V²/(1296·a_v)`. AASHTO'nun 395'i
  a_v ≈ 0,3'e karşılık gelir.
- **`bisector(A, E)`:** `L = 800E/A`.
- Hiçbir değer KentOS'tan gelmez: hız, tepki süresi, yavaşlama, yükseklikler, ışın açısı, ivme ve bisektris
  standarttan ya da kullanıcıdan gelir.

**Fizik sınaması:** önerilen boydaki tepede, Y20'nin bağımsız görüş hesabı (`Vertical::sight`) bütün göz yerlerinde en
kısa görüşü ölçer. Sonuç istenen DGM'ye 5 cm içinde eşittir. Örnekler: 80 km/sa ve A %6 için DGM 128,18 m, L 149,81 m,
en kısa görüş 128,18 m. 100 km/sa ve A %3 için DGM 182,92 m, L 146,50 m, en kısa görüş 182,92 m. A %1 ve A %4 (50 km/sa)
için kurp istenmez; eğimler 384,5 m ve 82,3 m görüş verir.

## Doğrulama

33 ortak durum vardır (`scripts/fixtures/road_vertical_length_cases.py`, 30 basamak). Referans kapalı formülleri
kullanmaz; her ölçütü ileri yönde yazar ve L'yi çözer:

- Tepe: her göz yerinden görüş çizgisinin yola teğet olduğu nokta ve nesnenin çizginin altına indiği yer ikiye bölmeyle
  bulunur. En kötü göz yeri altın oranla, L de ikiye bölmeyle aranır.
- Çukur: ışının yolu kestiği yer bulunur.
- Durma, konfor ve bisektris ileri formülleriyle çözülür.

Referansın ilk sürümü nesnenin göründüğü yanı ters ikiye böldü ve 95,5 m yerine 57,4 m buldu. Y20'nin görüş hesabı
çekirdeği doğruladığı için hata referansta arandı.

Bekçi: `tests/all/vertical_lengths.rs`.
