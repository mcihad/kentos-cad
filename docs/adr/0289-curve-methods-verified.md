# ADR 0289: Daire, Yay ve Elips yöntemlerinin doğrulanması ve `cad.curves.construct`

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-03`: “Daire/yay/elips araçlarında iki/üç nokta,
  merkez, açı/uzunluk, tangent ve radius/diameter yöntemlerini doğrula.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0032 (Daire, Yay), 0057 (Elips), 0270 (`cad.points.construct`, salt okunur yapım komutu), 0287 (yöntemlerin
  AutoCAD'le karşılaştırılması), 0244 (işlem manifesti), 0253 (komut protokolü), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0287 Daire'nin altı, Yay'ın on bir ve Elips'in üç yöntemini AutoCAD'le eşledi; ama yöntemlerin hesabı doğrulanmamıştı ve manifest
Yay'ı ve Daire'nin dört yöntemini `geometry` borcunda sayıyordu: araç hesabı tıklamalardan kendisi yapıyor, komut hazır merkezi, yarıçapı
ve açıları alıyordu; Python ve ajan iki noktadan daireyi ya da teğet daireyi kendileri hesaplamak zorundaydı. Bağımsız bir başvuruyla
yöntem yöntem yapılan denetim şu eksikleri gösterdi:

1. **Paralel iki doğruya teğet daire yoktu.** TTY'de iki doğru paralel ve aralarındaki uzaklık 2r iken sonsuz çözüm vardır; çekirdek
   ötelenmiş iki doğrunun kesişimini aradığı için hiç bulamıyor, “teğet daire yok” diyordu. Başka yarıçapta neden de söylenmiyordu.
2. **Neredeyse aynı doğrudaki üç nokta** mutlak 10⁻¹² m² eşiğiyle sınanıyordu: 1 km'lik noktalarda milimetrenin altındaki sapma
   10⁹ m yarıçaplı daire veriyordu, küçük ölçekte de doğru üç nokta reddedilebiliyordu.
3. **Başlangıç–merkez–bitiş'te bitiş başlangıcın doğrultusundaysa** tam daire yay olarak yazılıyordu.
4. **Başlangıç–merkez–açı'da 360° ve üstü** reddedilmiyor, yazılan açı tam tur sayılıyordu.
5. **Başlangıç–bitiş–yarıçap** açıyı dereceye çevirip geri alıyordu (iki yuvarlama).
6. **Devam** saat yönünde verilmiş bir yayda kullanıcının son noktasından değil, yayın saklanan bitişinden (başladığı uçtan) sürüyordu;
   kapalı çoklu çizginin son köşesinde kapanan kenarı değil, son kenarı izliyordu.
7. Uyarılar genel ve yönteme göre farksızdı (“Bu değerlerle yay oluşmuyor …”), elipste yazılan sıfır sessizce yok sayılıyordu.

## Karar

### 1. `cad.curves.construct` v1

Salt okunur ürün komutu (`effect: query`, geri alma adımı yok), `cad.points.construct`'ın (ADR 0270) kardeşi: yapımların eğrilerini
sırasıyla verir. Cevap `cad.circle.create`'in, `cad.arc.create`'in ve `cad.entities.create`'in elipsinin aldığıdır. Noktalar koordinat ya da
nesnenin yeri (`MeasuredPoint`), teğet nesneler kimlik ve yakın nokta (`EdgeNear`), Devam'ın nesnesi kimlik ve uç (`ObjectEnd`: `start`,
`end`, açık çoklu çizginin ilk ya da son köşesi); açılar doğudan saat yönünün tersine radyan.

| Yapım | Yöntem (araçta) | Eğri |
|---|---|---|
| `circleTwoPoints {a, b}` | Daire 2N | çapın ortası, yarı uzaklık |
| `circleThreePoints {a, b, c}` | Daire 3N | çevrel daire |
| `circleTangentTangentRadius {first, second, r}` | Daire TTY | iki nesneye r yarıçaplı, değme noktaları yakın noktalara en yakın olan; 2r arayla paralel iki doğruda merkez yakın noktaların ortasının karşısında |
| `circleTangents {first, second, third}` | Daire TTT | üç nesneye teğet (Apollonius), değme noktaları yakın noktalara en yakın |
| `arcThreePoints {start, mid, end}` | Yay 3 nokta | ortadaki nokta önce geliyorsa saat yönünün tersine |
| `arcStartCentreEnd {start, centre, end}` | M, BMS, önce merkez (M), B+M | başlangıçtan merkez çevresinde bitişin doğrultusuna |
| `arcStartCentreAngle {start, centre, angle}` | BMA, MBA, açı | işaretli açı, tam turdan küçük |
| `arcStartCentreChord {start, centre, chord}` | BMU, MBU, kiriş | kiriş; eksi kiriş büyük yay |
| `arcStartEndAngle {start, end, angle}` | BSA, B+A | işaretli açı |
| `arcStartEndDirection {start, end, direction}` | BSY, B+Y | başlangıçtaki teğet doğrultusu |
| `arcStartEndRadius {start, end, r}` | BSR, B+R | yarıçap; eksi büyük yay |
| `arcContinue {from, end}` | Devam | nesnenin ucundan teğet: bitişinde yönüyle, başlangıcında tersine |
| `ellipseAxis {a, b, other \| rotation, startAngle?, endAngle?}` | Elips eksen–uç, Döndürme, Yay | diğer yarı eksen ya da `a·cos(rotation)`; açılar büyük eksenden kutupsal |
| `ellipseCentre {centre, axisEnd, …}` | Elips Merkez | aynı, merkezden |

Merkez–yarıçap ve merkez–çap yapım istemez: `cad.circle.create` alır (r = çap / 2). Retler yapım yapım sırayla, önce sayılar ve kimlikler,
sonra türün istedikleri: `no_constructions`, `not_finite`, `invalid_uid`, `entity_not_found`, `no_such_place`, `piece_gone`,
`same_points`, `collinear_points`, `invalid_angle`, `invalid_radius`, `radius_too_small`, `invalid_distance`, `chord_too_long`,
`invalid_half_axis`, `not_tangent_kind`, `no_tangent_circle`, `not_an_end` (yer `constructions[i]…`). Yedisi yeni kod (`codes.rs`).

### 2. Araçlar komutun hesabını kullanır

Yay, Daire (2N, 3N, TTY, TTT) ve Elips yazacakları eğriyi `curves_construct::curve`'den alır, `cad.arc.create`, `cad.circle.create` ve
`cad.entities.create` ile yazar; önizleme aynı hesabın çizimsiz yoludur (`curve_at`; teğet dairenin önizlemesi çekirdeğin aynı
işlevi). Komutun reddi aracın uyarısıdır: araç ve betik aynı girdiyi aynı sözlerle reddeder. Yay Devam'da en yeni çizgi, yay ya da çoklu
çizginin yolunun bittiği yerden sürer: saat yönünde verilen yay saklanan başlangıcında biter (`Memory::arc_drawn_back`), AutoCAD'in ARC'ı
gibi son verilen noktadan devam edilir. Manifestte Yay `semantic` oldu (`via`: `cad.curves.construct`, `cad.arc.create`); Daire'nin
dört yöntemi ve Elips aynı yolla.

### 3. Çekirdeğin düzeltmeleri

- `circle_through` (ve onu kullanan 3 noktalı yay ve büküm): aynı doğruda, ilk noktadaki açının sinüsü 10⁻⁹'u geçmiyorsa
  (`COLLINEAR`), ölçekten bağımsız.
- `tangent_tangent_radius`: 2r arayla paralel iki doğruda yakın noktaların ortasının karşısındaki daire; `parallel_gap` reddin nedenini
  söyletir.
- `arc_start_center_end`: bitiş başlangıcın doğrultusundaysa yay yok. `arc_start_center_sweep` ve `arc_start_end_sweep` radyanla, tam tur
  ve üstü reddedilir; dereceli eski işlevler onları çağırır (WASM işlemleri aynı kalır). `arc_start_end_radius` dereceye dönmez.
- `end_tangent`: kapalı çoklu çizgi kapanan kenarın ucunda, uzunluksuz son kenar yok; `start_tangent` başlangıçtan tersine.

### 4. Doğrulama yolu

Bağımsız başvuru `scripts/fixtures/curve_method_cases.py` KentOS kodu olmadan 70 basamaklı ondalıklarla (karekök, atan ve sinüs kendi
serilerinden) yöntemlerin tanımından hesaplar; girdiler çift duyarlıklı sayılardır ve tam alınır. Değeri tam bir çift olan sonuç tam
karşılaştırılır; aşkın değerli (açılar, karekökler) ya da yinelemeyle çözülen (Apollonius) sonuç `within` taşır: 10⁻⁹ m ya da radyan,
çekirdeğin yuvarlama zinciriyle başvurunun tek yuvarlaması arasındaki pay. Ortak durum çalıştırıcıları (Rust ve Python) `within`'i okur.

| Yöntem | Başvurunun hesabı | Komut durumları | Araç durumları (tıklanan, yazılan) |
|---|---|---|---|
| Daire 2N | orta nokta, yarı uzaklık (tam) | 4 (yerleriyle, aynı nokta, olmayan nesne) | tıklanan, yazılan koordinat |
| Daire 3N | Cramer'le çevrel merkez | 5 (genel, aynı nokta, aynı doğru, neredeyse aynı doğru) | genel, aynı doğru uyarısı |
| Daire TTY | ötelenmiş doğru ve çemberlerin kesişimleri, en yakın değme | 9 (dik doğrular iki çeyrek, doğru–daire, içine alan daire, paralel doğrular, ret nedenleri, tür) | dik doğrular, paralel doğrular ve uyarısı |
| Daire TTT | işaretli doğrusal sistem; doğru–doğru–daire ikinci derece | 7 (iç ve dış teğet, paralel ve kesen, doğru ve daire, üç paralel, tür) | iç teğet daire |
| Daire merkez, çap, Merkez, çap | — (komut almaz) | — | tıklanan, yazılan yarıçap, Ç ve MÇ |
| Yay 3 nokta | çevrel merkez ve yön | 6 (saat yönünde verilen, alttaki yarım, olmayan yer) | tıklanan, genel |
| Yay başlangıç–merkez–bitiş | açılar | 5 (yerleriyle, doğrultu ve aynı nokta retleri) | M, BMS, önce merkez, B+M |
| Yay –açı | işaretli tam tur altı | 4 | BMA, MBA eksi açı |
| Yay –kiriş | 2 asin(k / 2r) | 4 | BMU, MBU eksi kiriş, çaptan uzun uyarısı |
| Yay başlangıç–bitiş–açı | merkez = kiriş ortası + sol normal · (d/2) / tan(θ/2) | 3 | BSA |
| Yay –yön | θ = 2 · yönden kirişe açı | 3 | BSY yazılan ve tıklanan yön |
| Yay –yarıçap | θ = 2 asin(d / 2r) | 4 | BSR, kirişin yarısından küçük uyarısı |
| Yay Devam | nesnenin ucundaki teğet | 11 (çizginin iki ucu, yay, açık ve kapalı çoklu çizgi, retler) | çizgiden, saat yönünde yaydan, doğrultuda uyarısı |
| Elips eksen–uç, Merkez, Döndürme | uzun eksen seçimi, a·cos | 12, eliptik yayla (yarı eksen, döndürme ve açı retleri) | yazılan, gösterilen yarı eksen, Döndürme 60 ve 95 uyarısı |
| Eliptik yay | t = atan2(sin φ / oran, cos φ) | (üsttekilerde) | yazılan ve gösterilen açılar |

Komutun 81 durumu (`fixtures/commands/v1/cad.curves.construct.json`; 43 ret) ve araçların 37 durumu
(`fixtures/curve-methods/v1/tools.json`, `crates/native/interaction/tests/all/curve_methods.rs`) aynı başvurudan.

## Yapılmadı

- Elipse ve eğriye teğet daire: AutoCAD'de de yalnız doğru, yay ve daire; `not_tangent_kind` reddeder.
- Eliptik yayda Parametre (P) ve İçerilen açı: başlangıç ve bitiş açısı yeter (ADR 0287).
- Elips açılarının AutoCAD'deki gibi ilk verilen eksenden ölçülmesi: KentOS'ta büyük eksenden (istemin dediği gibi).

## Sonuçlar

- Python ve ajan iki noktadan, üç noktadan, iki ya da üç nesneye teğet daireyi; on bir yay yöntemini ve Devam'ı; elipsi ve eliptik
  yayı geometriyi hesaplamadan ister, cevabı oluşturma komutuna verir (`kentos.cad.curves.construct`, MCP'de yıkıcı olmayan araç).
- Manifestin `geometry` borcundan Yay'ın on bir ve Daire'nin dört yöntemi çıktı.
- Araçların uyarıları yönteme özgü ve komutla aynı; izler değişmedi.

## Doğrulama

- `python3 scripts/fixtures/curve_method_cases.py --check`; `cargo test -p kentos-native-application --test all` (81 komut durumu,
  yetenekler); `cargo test -p kentos-interaction --test all curve_methods` (37 araç durumu); `cargo test -p kentos-interaction`;
  `cargo test -p kentos-geometry-core --lib`; `pnpm py:test`; `python3 scripts/cad/manifest.py --check`; bütün izler
  (`every_trace_passes`).
- Resimler: `KENTOS_SHOTS_ONLY=egri-tty-paralel,egri-tty-uyari,egri-devam,egri-bsy,egri-eliptik-yay,egri-dogru cargo test -p
  kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-egri-*`).
