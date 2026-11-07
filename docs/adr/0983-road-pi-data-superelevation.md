# 0983 — Some bilgisinde dever: S, Lr, Lt, La, Gn; basit kurbun T ve L'si

Tarih: 2026-11-06. Durum: kabul (Y77); **T, L ve kutu biçimi ADR 0984 ile değişti.** Bu ADR'nin "T ve L basit
kurbundur (R·tan(Δ/2), R·Δ)" sonucu klotoidli kurpta yanlıştı. KGM'nin kutusundaki A, toplam sapma değil dairesel
kısmın sapmasıdır; EK-10'un `S-A=35.60832` satırı bunu gösteriyor. L dairesel yaydır, T R·tan(D/2)'dir. Kutu artık
KGM'nin Haziran 2024 örnek paftalarının biçimindedir (`D`, `Se`, iki ondalık). Kapsam ROAD-SEC-11'in plan haritası ve ROAD-DEVER-10; ADR 0982 §Açık ("Some
bilgisinde S, Gn, Lt, La").

## Sorun

Y76'nın some kutusu A, R, T, L, Lr, Y, X yazıyordu. KGM'nin istediği deveri (S), genişlemeyi (Gn) ve rakordman
bilgilerini (Lt, La) yazmıyordu. Araştırmada iki yanlış daha çıktı:

- `Lr=` klotoid boylarını yazıyordu. KGM'de Lr bir rakordman (dever geçişi) boyudur. KGM'nin örneğinde ikisi tesadüfen
  eşittir (76,8 m), çünkü rampa klotoidin üstünde yapılmıştır.
- `T=` klotoidin kaydırmasıyla teğet boyunu, `L=` dairesel kısmı yazıyordu. KGM'nin örneği basit kurbun T ve L'sini
  yazar.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı

- **KGM 2019, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi**, s. 5 (PDF s. 7): "Yatay
  kurblar some bilgileri ile tanımlanacaktır. Some bilgileri geçiş eğrisi kullanılmayan kurblarda yarıçap (R), sapma
  açısı (Grad cinsinden), teğet uzunluğu (T), Kurb Boyu (L) uygulanan dever (S), Şerit Genişleme miktarı (Gn),
  rakordman bilgileri (Lr, Lt, La), some koordinatları (X=Kuzey, Y=Doğu) ile tanımlanacaktır. Geçiş eğrisi kullanılan
  kurblarda ise yukarıdaki bilgilere klotoid ile ilgili değerler ilave edilecektir." Aynı sayfa: dever değerleri
  Karayolu Tasarım El Kitabı'na göre hesaplanır, geçiş elemanı klotoiddir. Kısaltmaların tanımı şartnamede yoktur.
- **Örnek paftalar**, şartnamenin taramaları büyütülerek okundu:
  - **EK-10, S1:** `V=110`, `A=27.45958`, `R=600`, `T=131.445`, `L=258.800`, `S=6.00`, `Lr=76.800-76.800`,
    `Lt=25.600-25.600`, `La=25.600-25.600`, `Gn=0.000`; çizgiden sonra `Y=`, `X=` ve klotoid satırları.
  - **EK-11, S11** (klotoidsiz): `V=50`, `A=36.56916`, `R=500`, `T=147.69x` (son hane taramada okunmuyor), `L=287.213`,
    `S=KAVŞAK DEVERİ`, `Gn=0.000`, `Y=`, `X=`. Lr, Lt ve La satırları yoktur.
  - Kutu çerçevelidir: yazıların solunda dikey bir çizgi, başlığın altında ve Gn ile Y arasında yatay çizgi vardır.
- **Karayolu Tasarım El Kitabı (KGM 2005)**: Lr ve Lt'nin en küçük değerleri Tablo 2.6'dadır; klotoid varsa "Lr
  uzunluğu geçiş eğrisi uzunluğuna (Ls) eşit olması arzu edilir". Bu bilgi ikincil kopyalardan (Scribd, pdfcoffee)
  aktarılan arama özetinden geldi. El kitabının metni otomatik indirilemedi ve okunamadı; La'nın tanımı oradan
  doğrulanmadı.
- **3516 sayılı Ölçüler ve Ayar Kanunu:** uzunluk metre. Yazı biçimi için hüküm yok.

### Türk uygulaması

- **Netcad (NETPRO, Güzergâh editörü, "Some bilgileri" penceresi)**, wiki.netcad.com.tr pageId 217385448:
  - "Lt: Alinman çatı eğiminden (%-2), %0 devere kadar olan mesafe değeridir."
  - "Lr: %0 deverden max. devere ulaşmak için gerekli olan mesafe değeridir."
  - "La: Alinmanda uygulanan dever miktarıdır."
  - "Lr'nin aliymandaki oranı katsayısıyla kurp içi ve dışı bilgileri hesaplanır."
  - Klotoidli kurpta dever değişimi klotoidin üzerine konur.
- **Netcad "Some Bilgileri Kullanıcı Tanımlı"** (pageId 217385413): rapor değişkenleri `SB_Lr`, `SB_Lt`, `SB_La`,
  "dever rakortman boyu bilgileri (Lr, Lt, La)".
- **İTÜ (Kesten), Karayolu Ödev Projesi:** kısaltmaları tanımlamıyor.

### Yabancı kaynaklar

- **AASHTO Green Book:** superelevation runoff Lr (düz taçtan tam devere) ve tangent runout Lt (normal taçtan düz
  taça). Doğrusal döndürmede Lt = Lr·n/e. Klotoidsiz kurpta rampanın üçte ikisi kadarı teğette olur. Bu, Netcad'in
  "Lr'nin aliymandaki oranı"dır.

### Matematik (kabul görmüş)

- Kısa ad karşılıkları: Lr = runoff, Lt = runout (AASHTO, Netcad). La = geçişin aliymanda kalan kısmı (Netcad).
  Klotoidli kurpta La = Lt; klotoidsizde La = Lt + pay·Lr.
- KGM EK-10 bunu doğrular: 76,8·2/6 = 25,6 = Lt = La. Rampa klotoiddedir, aliymanda yalnız Lt kalır.
- **Basit kurbun elemanları:** T = R·tan(Δ/2), L = R·Δ. EK-10 S1'de 600·tan(0,215668) = 131,4445 → `131.445` ve
  600·0,431337 = 258,8004 → `258.800`. Klotoidli kurbun gerçek teğet boyu (R + p)·tan(Δ/2) + k = 169,929, dairesel
  kısmı 182,000 olurdu. S11'de 500·0,574420 = 287,2135 → `287.213`. Türk uygulamasında some bilgisi ana (basit) kurbun
  T ve L'sini verir, klotoidin değerleri ayrıca yazılır.

## Karar

- **Çekirdek:** `kentos_road::superelevation_pi::curve_super(schedule, normal, curve)`. Değerler çizelgenin kırık
  noktalarından okunur; çizelgeyi kurplar ya da km satırları yazmış olabilir. Kurbun dışında, dairesel kısmın
  ortasından uca doğru yürünür:
  - FS: eğimin oradaki değerin altına ilk düştüğü yer;
  - LC: eğimin 0'ı kestiği yer;
  - NC: eğimin normal eğime indiği yer (parçalı doğrusal, kesin).
  - Buradan Lr = |FS − LC|, Lt = |LC − NC|, La = NC'nin kurbun dışında kalan boyu (girişte TS − NC, çıkışta NC − ST,
    kurbun üstündeyse 0).
  - S, ortadaki merkeze doğru eğimdir (iki yanın küçüğü). Gn, oradaki iç yan genişlemesidir.
  - Deverlenmemiş kurpta S yoktur. Geçişi çizelgenin içinde başlayan ya da bitmeyen uçta Lr, Lt, La yoktur; yanlış bir
    sayı yazılmaz.
- **Hangi çizelge:** aksın tek dever çizelgesi. Birden çoksa harita hangisinin kastedildiğini bilemez, dever satırları
  yazılmaz (§Açık).
- **Kutu, KGM'nin sırasıyla:** `S1 (SOL)` | `A=` | `R=` | `T=` | `L=` | `S=6.00` | `Lr=a-b` | `Lt=a-b` | `La=a-b` |
  `Gn=` | `Y=` | `X=`.
  - Uzunluklar üç ondalıktır. S yüzde, iki ondalıktır.
  - Lr, Lt, La KGM gibi her zaman giriş-çıkış çiftiyle yazılır (eşit olsalar da).
  - T ve L basit kurbundur. `Lr=` artık klotoid boyu değildir; klotoid kutuları açık kalır (ADR 0982 §Açık).
- **Çerçeve:** hizalı kenarda dikey çizgi, başlığın altında ve Y'nin üstünde yatay çizgi, 0,18 mm. Yatay çizgiler
  karakter başına 0,5 h ile en uzun satırı geçer; Barlow'un rakamları PDF'te 0,465 h ölçüldü.
- **V yazılmaz:** projede tasarım hızı alanı yok. Uydurulmaz.

## Ölçüm

- **Çekirdek testleri** (`crates/shared/road/tests/all/superelevation_pi.rs`, 6 durum):
  - KGM EK-10 S1 iki dönüş yönünde: Lr 76,8, Lt 25,6, La 25,6, S %6, Gn 0. KGM'nin yayımladığı sayılarla aynı.
  - Elle kesirler: 90°'lik sağ kurp, R 100, TS 400, ST 400 + 50π, satırlar 330/400/500/570. Lt 20, Lr 50, La 70 ve
    170 − 50π.
  - Kurbun üstünde başlayan geçişte La 0.
  - Deverlenmemiş kurpta değer yok.
  - Geçişin içinde başlayan çizelgede giriş yok.
  - Kural yöntemiyle Gn iç yanda; payla La = 20 + 0,6·50 = 50.
- **Uygulama testi** (`tests/all/road_plan_axes.rs`): örnek çizimin PI'larından ve klotoid serilerinden (p, k), KentOS
  kodu olmadan:
  - `T=65.284`, `L=123.151`;
  - satırlardan `S=6.00`, `Lr=35.294-35.294` (600/17), `Lt=14.706-14.706` (250/17), `La=44.713-0.000`, `Gn=0.000`;
  - satırların sırası KGM'ninki.
- **Resimler** `.run/gui-evidence/yol-y77/`: 1:2000 A1 KGM paftası açık/geniş ve koyu/dar, PDF'i, PDF'ten 400 dpi'lık
  some kutusu (`y77-pdf-some.png`). Kutu okunur, çerçeve oturur.
- **Testler:** `kentos-road` 133 (22 + 111) geçti, uygulama 516, masaüstü pafta 18, plot 86, komutlar 1605.
- **Ön denetimin 4 hatası** Y75'ten beri aynıdır: KGM antedinin `@birim`, `@dosya_no`, `@il`, `@kontrol_kesim`
  değişkenleri örnekte boş. Y77'den gelmiyor.

## Açık

- **V (proje hızı):** yol projesinde ya da aksta tasarım hızı alanı yok.
- **Birden çok dever çizelgeli aks:** harita, paftanın seçtiği çizelgeyi (`road.sheets.plan` `superelevation`)
  bilmiyor.
- **Klotoid kutuları:** EK-10'daki `S-A`, `S-Y`, `S-X` satırları taramada tam okunamadı.
- **S'nin metin değeri:** EK-11'de `S=KAVŞAK DEVERİ` yazılıyor; KentOS'ta yalnız sayı var.
- **La'nın El Kitabı'ndaki tanımı** okunamadı. Netcad'in tanımı ve KGM örneği tutarlı.
