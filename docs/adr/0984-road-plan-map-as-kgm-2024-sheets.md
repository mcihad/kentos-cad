# 0984 — Plan haritası KGM'nin 2024 örnek paftaları gibi: some kutusu, ana noktalar, proje uçları

Tarih: 2026-11-06. Durum: kabul (Y78). Kapsam ROAD-SEC-11'in plan haritası; ADR 0982 ve 0983'ün kutusu. ADR 0983'ün
T ve L kararını düzeltir.

## Sorun

ADR 0982 ve 0983, KGM 2019 şartnamesinin 110 dpi'lık ek taramalarına dayanıyordu. Kurumun sahibi eski belgelere
dayanmayı istemedi, son ihale belgelerine bakılmasını istedi. Ayrıca Y77'nin resimlerinde ve Y78'in ilk resminde kutu
haritanın kenarından taşıyor, son satırları kırpılıyordu.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı ve KGM'nin güncel belgeleri

- **KGM Etüt, Proje ve Çevre Dairesi Başkanlığı, "Çalışmalar" sayfası** (kgm.gov.tr, 6 Kasım 2026'da okundu):
  - Proje teknik şartnamesi hâlâ 2019 baskısıdır (`tekniksartname.pdf`, oluşturma 22 Nisan 2019).
  - 2013–2026 birim fiyat listeleri ve ayrı bir **"Örnek Paftalar"** belgesi yayımlanmıştır.
- **2025 ve 2026 Yılı Birim Fiyat Listesi ve Tarifleri** (EP1003, EP1007): pozları "Proje Yatay ve Düşey Hattının
  Belirlenmesi ve Raporlarının Hazırlanması Teknik Şartnamesi" ile "Proje Paftaları ve Proje Sonu Raporunun
  Hazırlanması Teknik Şartnamesi"ne bağlar. Bunlar 2019 şartnamesinin bölümleridir (s. 134–135, 991). Yani 2026 ihale
  fiyatları da 2019 şartnamesine dayanıyor; daha yeni bir proje şartnamesi yok.
- **Örnek Paftalar** (`Ornekpaftalar.pdf`, oluşturma 12 Haziran 2024, 12 sayfa A1, 300 dpi): Hırfanlı Barajı
  Varyantı – Şanlıkışla 3. Bl. Hd. Devlet Yolu ve Yüksekova–Esendere örnekleri. Bu ADR'nin dayanağı. Plan–profil
  paftalarında (s. 8–12) şunlar var:
  - **Some kutusu** (S-1, S-2, S-17, S-23, S-24, S-7):

    ```text
    S - 1   (Sol)
    V = 80
    D = 19.0937
    R = 1500.00
    L = 449.88
    T = 226.64
    Se = % 2.20
    Lr = 22.00 - 22.00
    Lt = 20.00 - 20.00
    La = 36.42 - 36.70
    Gn = 0.00
    ──────────────
    X =4 349 283.7346
    Y =  542 198.1906
    ```

    Dış çerçeve, solda çift dikey çizgi, başlığın altında ve koordinatların üstünde yatay çizgi var. Başlık kırmızı.
  - Sapma **D** (grad, dört ondalık), dever **Se** ("% " ile), uzunluklar iki ondalık. Önce X (kuzey), sonra Y
    (doğu); dört ondalık, binlik boşluklu. Deverlenmemiş büyük yarıçaplı kurpta (S-7, R 3500) `Se = % -2.00`.
  - **Ana noktalar** aksa dik bir çizgi ve yanında `TO1=0+100.42`, `TF17=13+035.78`: kod, kurp numarası, `=`, iki
    ondalık km. Siyah.
  - **PI** üçgenle işaretli, adı çerçeve içinde (`S - 1`).
  - **Aks** kırmızı kesik-noktalı.
  - **Proje başı/sonu**: `PROJE BAŞI`, `KM : 0+000.00`, `X=…`, `Y=…`. 2019 şartnamesi s. 5 de "Proje başı ve sonunda
    kot ve koordinat değerleri yazılacaktır" der.
- **Karayolu Tasarım El Kitabı 2022:** KGM'nin ücretli yayını (yayınlar sayfasında 1000 TL). Okunamadı; ikincil
  kopyalar (Scribd) kullanılmadı.

### Değerlerin doğrulanması (elle, KentOS kodu yok)

- 2024 örnekleri:
  - S-1: 1500·19,0937 grad = 449,88 ✓; 1500·tan(D/2) = 226,64 ✓.
  - S-7: 3500·20,2368 grad = 1112,58 ✓; T = 561,02 ✓.
  - Bu kurplar klotoidsiz (TO/TF).
- **Lr = Se·10 m** her kurpta (22, 48, 41, 31; Se 2,2, 4,8, 4,1, 3,1). Lt = 20.
  - Bu, 6 m dönen genişlik ve 1/167 (%0,6) göreli eğimle uyuşur: 6·0,022/0,006 = 22 ve 6·0,02/0,006 = 20.
  - Bu, bizim çıkarımımızdır; El Kitabı'nın tablosu okunamadı.
- **La − Lt**, klotoidsiz kurplarda Lr'nin 0,74–0,76'sı:
  - S-1: 16,42/22; S-2: 35,44/48; S-23: 30,54/41; S-24: 22,87/31.
  - Rampanın yaklaşık dörtte üçü aliymanda. Netcad'in "La: Alinmanda uygulanan dever" tanımı ve ADR 0983'ün La = TS −
    NC hesabı doğrulanıyor.
- **Klotoidli kurp (2019 EK-10 ve EK-13):**
  - Kutuda `A=` dairesel kısmın sapmasıdır. Satırları KGM şöyle sürdürür: `S-A=` (toplam sapma), `S-Y=`, `S-X=` (some,
    yani PI).
  - EK-10 S1: S-A = 35.60832 (ADR 0983'te okunamamıştı), A = 27.45958. Fark 8.14874 grad = 0,128000 rad = 2·Ls/(2R) =
    76,8/600, tam.
  - EK-13: A = 18.98047, S-A = 27.64527, R = 325, Lr = 44,5-….
  - Kutunun **Y, X**'i SC ve CS teğetlerinin kesiştiği nokta, yani dairesel kısmın PI'sı. EK-10'da some ile arası
    taramada ≈10,45 m. Hesap: E_toplam − E_dairesel = 24,67 − 14,24 = 10,43 m ✓.
  - **ADR 0983'ün düzeltmesi:** 0983 kutunun A'sını toplam sapma sanıp "L = R·Δ_toplam, T = R·tan(Δ_toplam/2)"
    demişti. Bu klotoidsiz kurpta doğru, klotoidli kurpta yanlıştı: L dairesel yaydır (Y76'nın `L=`'si doğruydu), T ise
    R·tan(D_dairesel/2).

### Yabancı yazılımlar

- **Civil 3D / OpenRoads:** eğri etiketleri kullanıcı tanımlıdır; etiketlerin çakışmasını yerleşim kuralıyla ya da elle
  çözerler. KGM'nin örneğinde kutular haritanın boş yerine, PI'ya bağ çizgisi olmadan konmuştur.

## Karar

- **Kutu, 2024 örneği gibi:** `S - n (Sol|Sağ)` | `D` | `R` | `L` | `T` | `Se = % …` | `Lr = a - b` | `Lt` | `La` |
  `Gn` | `X` | `Y`. Çerçeve, çift dikey çizgi, iki yatay çizgi.
  - Klotoidli kurpta D, L, T dairesel kısmındır; X, Y dairesel kısmın PI'sıdır. Ardından `S-D`, `S-X`, `S-Y` gelir.
    Bu üç satırın adı bizimdir: 2019'un `S-A`'sı, 2024'ün `D` adıyla. Değerler KGM'nindir.
  - `Se` her zaman yazılır; deverlenmemiş kurpta eksi çıkar (`curve_super` artık hep bir oran verir).
  - V yazılmaz, çünkü modelde tasarım hızı yok.
- **Ana noktalar:** `TS1=1+194.71` biçimi, siyah. **PI:** 2,5 mm'lik üçgen; ad çerçeveli, üçgenin kutunun gitmediği
  yanında. **Aks:** kırmızı kesik-noktalı. **Proje uçları:** `PROJE BAŞI KM : 1+000.00`, `X = …`, `Y = …`. Satırlar,
  yazı yarım tur çevrilse de okuma sırasıyla dizilir. Kot yazılmaz: proje uçlarının kotu boykesitindir.
- **Kutunun yeri** (`Room`): harita, kopyaya kendi merkezini ve boyunu verir. Adaylar sırayla denenir:
  1. PI'nın dışı;
  2. yolun öbür yanı, bu kurbun aksının ötesi;
  3. son çare: kutu haritanın içine kaydırılır.
  - Aday, haritanın 2 mm içinde ve her akstan ve ana nokta çizgi/yazısından 6 mm uzakta olmalıdır.
  - Yalnız PI'sını gösteren harita kutuyu yerleştirir. Aynı paftadaki boykesit haritası, göstermediği S-1'in kutusunu
    kendi içine çekiyordu.
  - Önbellek anahtarına, aks çizen haritalar için merkez ve boy eklendi (`kentos_plot::book`, masaüstü `sheets.rs`).

## Ölçüm

- **Uygulama testleri** (`tests/all/road_plan_axes.rs`, 4 test), örnek çizimin PI'larından elle:
  - Dc = Δ − Ls/R ile `D`, `L`, `T`, `S-D`.
  - Satırlardan `Se = % 6.00`, `Lr = 35.29 - 35.29`, `Lt = 14.71 - 14.71`, `La = 44.71 - 0.00`.
  - X, Y açıortay üzerinde, PI'dan (R + p)·sec(Δ/2) − R − (R·sec(Dc/2) − R) içeride, 1 mm içinde.
  - Ana noktalar `TS1=1+…`, iki ondalık; proje başı km ve koordinatları.
  - 80 m'lik haritada kutunun her satırı içeride; uzak bir harita kutuyu çekmez.
- **Çekirdek:** deverlenmemiş kurpta `Se` −0,02 (`superelevation_pi`, 6 durum).
- **Birim:** `grouped` (`4 349 283.7346`).
- **Testler:** `kentos-road` 133, uygulama 518 + 137 (kitaplık), plot 86, masaüstü 1852 (356 yok sayılan). Masaüstü,
  proje kuralı gereği `RUST_MIN_STACK=67108864` ile koşturuldu. ADR 0983'teki "masaüstü pafta 18" ölçümü bu ayar
  olmadan alınmıştı ve bir testin yığın taşmasını göstermemişti: o koşu tam değildi.
- **Resimler** `.run/gui-evidence/yol-y78/`: 1:2000 A1 KGM paftası, PDF'i ve kutu çevresi.
  - İlk resimde ad kutusu paralel aksın üstündeydi; üçgenin yanına alındı.
  - İkincide kutu haritadan taştı; "içine kaydır" onu yolun üstüne bıraktı. Üçüncüde yan aks "öbür yan" adayını
    haritanın dışına itti. Dördüncüde ana nokta yazılarının altına girdi. Beşincide boykesit haritası ikinci bir kutu
    gösterdi.
  - Son resimde kutu yolun öbür yanında, akslardan ve yazılardan uzak, bütün satırları görünür.

## Açık

- ~~**V:** tasarım hızı alanı yok.~~ Y79, ADR 0985: aksın proje hızı ve kurbun kendi hızı.
- **Birden çok dever çizelgeli aks** (ADR 0983).
- **Kutunun PI'ya bağ çizgisi:** 2024 örneğinde yok, biz de çizmiyoruz. Kalabalık paftada hangi kutunun hangi kurba
  ait olduğu adla okunur.
- ~~**Proje uçlarının kotu.**~~ Y81, ADR 0987: boykesitte, 2024 örneği gibi.
- **El Kitabı 2022**'nin Lr/Lt tablosu okunamadı.
