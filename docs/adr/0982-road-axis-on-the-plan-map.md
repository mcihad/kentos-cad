# 0982 — Pafta plan haritasında aks

Tarih: 2026-11-06. Durum: kabul (Y76). Kapsam ROAD-SEC-11'in plan haritası; ADR 0908 §Sonra ("plan haritasının aksı …
modelin nesnesi olmadan kendisi çizmesi") ve 0981 §Açık.

## Sorun

Aks modelde nesne değildir: ekran onu kendi katmanında çizer (`alignment_layer.rs`, ADR 0901). Paftanın plan haritası ise
yalnız modelin nesnelerini çizer. Bu yüzden Y10'dan beri plan–profil paftasının planında yolun kendisi yoktu, yalnız
ölçüm noktaları görünüyordu. Y75'in resminde açıkça görüldü.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı

- **KGM 2019, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi** (basılı sayfa = PDF sayfası
  − 2):
  - s. 17/64, madde 2: "Yol ve kavşak kolu eksen kilometrajı uygun aralıkta ve normda paftalara işlenecektir." Aralık
    verilmez.
  - s. 17, madde 10: çizgi kalınlığı ve biçimi "teknik resim kaideleri, varsa bilinen KGM normları ve İdare'nin
    istekleri"ne bırakılır; renk, kalınlık, yazı yüksekliği yazılmaz. KGM normları bulunamadı.
  - s. 17, madde 3: mevcut topografya "açık renkte"; madde 9: km ve kot yazıları tasarımı boğmamalı.
  - s. 5/64, some bilgileri: R, **grad cinsinden sapma açısı**, T, L, S, Gn, Lr/Lt/La, PI'nın X=Kuzey, Y=Doğu
    koordinatları; klotoid zorunlu. Proje başı ve sonu kot ve koordinatla verilir.
  - s. 18: km eşitliği İLERİ/GERİ kesriyle yazılır; kavşaklar numaralı daire içinde.
  - Kuzey oku, eşleşme çizgisi ya da some tablosunun yerleşimi için kural yok; tablolar eklere bırakılır.
- **Örnek paftalar** (EK-10–13, s. 45–48, ~110 dpi taramalar):
  - plan aks paftanın boyunca uzanacak biçimde döndürülür;
  - tam km aks boyunca, ona paralel yazılır (`6+000`), hektometreler küçük tek rakamla;
  - kurp ana noktalarında platformu kesen, aksa dik uzun bir çizgi ve onun boyunca yazı: `TF S11 5+983.306` (km üç
    ondalık);
  - PI `S11` adlı bir nokta; some bilgisi kurbun dışında, başlığı `S11 (SAĞ)`, satırlar V, A (grad, beş ondalık,
    `36.56916`), R, T, L, S, `Lr=76.800-76.800`, Lt, La, Gn, Y, X;
  - `PROJE BAŞI, KM: 0+000.000`; eşleşme çizgisi yok, yol çerçeveye kadar sürer.
  - Okunamadı: 20 m'lik ara işaret var mı, aks noktalı mı düz mü.
- **Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği** (RG 26.06.2018, 30460 Mük.), md. 80/1-h: aksın
  karakteristik noktaları "aliyman üstü noktalar, some noktası, To, TF". md. 77: ızgara koordinatları 2,5 mm yazıyla.
- **KGM Karayolu Teknik Şartnamesi 2013:** aksın çizimine dair bir şey bulunamadı.

### Türk uygulaması

- **İTÜ (Kesten), Karayolu Ödev Projesinin Hazırlanması**, s. 1–2: "Geçki ekseni: Kırmızı, kalın"; noktalı-çizgili
  eksen; kesit işaretleri 2 cm, adlar (TO, TF …) solda, km sağda; kurp elemanları kurbun dışında; yazılar tek yönde.
- **MEB MEGEP Yol Projesi**, s. 8, Şekil 1.4: ana noktalarda aksa dik çizgi, km onun boyunca; "Proje başı 0+000";
  PI S₁, blok "S₁=sağ, Δ₁=65ᵍ, T₁=…, L₁=…".
- **Netcad:** kodlar TO (Tanjant Orijin), TF (Tanjant Final), TS, SC, CS, ST; paftalar aks boyunca döner, ardışık
  paftalar bindirmeyle (eşleşme çizgisi değil).

### Yabancı yazılımlar

- **Civil 3D:** Major/Minor Station ve Geometry Point etiketleri ("Perpendicular With Tick And Line"); view frame
  "Along Alignment"; eşleşme çizgisi yalnız planda, istasyon yuvarlanır.
- **OpenRoads:** eşleşme çizgisi "a line and the respective stations"; km aralıkları aksın özellik tanımından.

### Matematik

Kabul görmüş, standart:

- km'den istasyon: bölgenin `s = at + dir·(km − ahead)` bağıntısı (ADR 0901'in km eşitlikleri).
- Teğet t = (cos θ, sin θ), sol normal n = (−sin θ, cos θ); işaretler n boyunca.
- Sapma açısı grad: Δ_g = |Δ|·200/π (1 dik açı = 100 grad).
- Okunur yön: yazının paftaya göre açısı (−90°, 90°] aralığına çekilir; yarım tur çevrilen yazının dayanağı öteki ucuna
  geçer.
- Döndürülmüş pencereye kırpma (Liang–Barsky 1984, Cyrus–Beck 1978) gerekmedi: haritanın kendi kırpması nesneleri
  kırpar.

## Karar

- **Yol:** Harita, yol projesinde çizimin bir **okuma kopyasını** çizer. Aks bu kopyaya `yol-aks-pafta` ("Aks
  (pafta)") katmanında nesne olarak eklenir. Model değişmez; kopya haritanın ölçeği ve dönüşüyle önbellekte tutulur.
  Ekrandaki pafta (`apps/desktop/src/sheets.rs`) ve PDF/baskı (`kentos_plot::book::MapDrawer`) aynı işlevi çağırır:
  `kentos_native_application::road_plan_axes::with_axes_on`.
- **Çizilenler** (boyutlar kâğıt mm × ölçek):
  - aks kırmızı 0,5 mm (İTÜ); paralel aks 0,25 mm; PI çizgisi ince kesikli;
  - her hektometrede aksı kesen 3 mm'lik işaret; yuvarlak km'de tam km (`1+000`) aks boyunca, ötekilerde hektometrenin
    rakamı;
  - kurp ana noktaları aksa dik 12 mm'lik kırmızı çizgi ve onun boyunca `TS S1 1+184.713`. Kodlar klotoidsiz TO ve TF
    (BÖHHBÜY md. 80, Netcad), klotoidli TS, SC, CS, ST;
  - PI 0,75 mm'lik daire ve adı (`S1`); some bilgisi kurbun dışında, paftaya dik:
    `S1 (SOL)`, `A=52.26690` (grad), `R=`, `T=` (iki yan farklıysa `T=a-b`), `L=` (dairesel kısım), klotoidle
    `Lr=a-b`, `Y=` (doğu), `X=` (kuzey);
  - `PROJE BAŞI KM: 1+000.000` ve `PROJE SONU KM: …`;
  - yazılar 2,5 mm (bant yazılarıyla aynı, ADR 0980) ve paftada soldan sağa okunur.
- **Bantlardaki Δ:** Y73'ün yatay geometri bandı kurbu `R= Δ=…g L=` diye yazıyordu. Çizim yazı tipi Barlow'da Δ yok ve
  PDF yazıcısı `map_text_unsupported` ile paftanın tamamını reddediyordu. Bant artık KGM'nin yazdığı gibi `A=…g`
  yazar; klotoid `Kl. A=` olarak ayrışır.

## Ölçüm

- **Uygulama testi** (`tests/all/road_plan_axes.rs`): örnek çizimin PI'larından elle hesaplanan sapma
  |atan2(300, 260) − atan2(10, 280)| = 47,04° = **52,26690 grad** kutudaki `A=` ile aynı. Yön SOL. Hektometre
  rakamları 1–6 iki aksta birer kez. Ana noktalar üç ondalık km'li. Bütün yazılar 29,847° dönük paftada okunur. Çizimin
  revizyonu ve nesne sayısı değişmez; aksı olmayan çizimde kopya yok.
- **Masaüstü testi:** KGM paftası PDF'e yazılır. Önce `map_text_unsupported` (Δ) ile düşüyordu, test bunu artık
  bekçiler.
- **Testler:** masaüstü 1841 geçti (önceden bilinen `each_program_has_its_own_bottom_tabs` dışında), plot ve sheet-ui
  geçti, `road.` ortak durumları geçti.
- **Resimler** `.run/gui-evidence/yol-y76/`: 1:2000 A1 paftası ve PDF'i. Ana aks haritaya sığar; aks, ana noktalar
  ve some kutusu okunur. 1:1000 KGM paftası `.run/gui-evidence/yol-y75/y75-pafta.pdf`'te.

## Açık

- ~~**Some bilgisinde S, Gn, Lt, La:**~~ Y77'de, ADR 0983: aksın dever çizelgesinden. Aynı işte `T=` ve `L=` KGM'nin
  basit kurbuna, `Lr=` klotoid boyundan rampa boyuna döndü. V (hız) yazılmıyor: projede tasarım hızı yok.
- **Klotoid kutuları** (KGM'nin `S1/1`, `S1/2` A, Ls, Y, X kutuları).
- **Km eşitliğinin kesir yazısı** (İLERİ/GERİ).
- **Kuzey oku** (paftada ayrı bir öğe olarak var) ve aksın noktalı-çizgili gösterimi: KGM'de okunamadı.
- **Harita katman listesi:** yalnız seçili katmanları gösteren bir haritada `yol-aks-pafta` listede yoksa aks
  çizilmez.
- **Eşleşme çizgileri:** KGM bindirmeyi kullanıyor (Netcad'in "Bindirme"si gibi). KentOS'ta `overlap` var, çizgi yok.
