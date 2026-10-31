# ADR 0770: Bağlantılı odalar — ortak duvarı benimseyen oda, kaydırmanın etkisi ve kalıcı mimari bağlar

- **Durum:** kabul edildi (31 Ekim 2026). Kapsam: `plan/architecture-advanced.md` ADV-01 (plan 30 Ekim; sahibin sözü
  31 Ekim: “@plan/architecture-advanced.md ile devam edelim”); madde tek parçada.
- **Bağlam belgesi:** ADV-01 (eski ARCH-DRAW-03, ARCH-EDIT-01); ADR 0656 (dikdörtgen duvar), 0657 ve 0734 (ölçülü bölme),
  0710 ve 0728 (duvarın geçici ölçüsü), 0701 (açıklığın geçici ölçüsü), 0334–0338 (kısıtlar), 0340 (birleşimler),
  0341 (açıklık duvarıyla gider), 0342 (mahal duvarıyla gider), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

“Bu odanın iç eni 3,20 olsun, koridor 1,20 kalsın” işi KentOS'ta duvar duvar yapılıyordu. ADR 0710 tek duvarın kenarını net
ölçüyle kaydırıp bağlı duvarlarını izletiyor; ama (1) bitişik oda çizen dikdörtgen duvar ortak sınıra ikinci bir duvar
koyuyordu, (2) kaydırmanın iki yandaki odalara, kapılara ve ölçülere etkisi yazmadan görülmüyordu, (3) “bu ölçüyü koru”
diye bir kayıt yoktu: dış duvar değişince koridor elle yeniden ayarlanıyordu. Kısıt motoru (ADR 0334) var ama duvar, kapı
ve yüz bilmiyor (plan kanıtı A2); bölme bir oluşturma işi, kalıcı bağ değil (A1).

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 30 — Distance Guides](https://help.graphisoft.com/AC/30/INT/Topics/030_Interaction/030_Interaction-65.html):
  seçili duvarın çevresine canlı mesafe; referans kenarı değiştirilebilir (paralel ya da dik kenarlar); duvar boyuna
  esnetilir, “Connected walls and beams will not break as a result”. Kalıcı ilişki kaydetmez.
- [Revit 2026 — ilişkiler ve kısıtlar](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-GetStarted/files/GUID-04AAFF35-A495-48EC-AA5B-35E956A4855C.htm):
  ölçü konulur ve kilitlenir (“lock it in place to apply a constraint”): bir kapı duvardan sabit uzaklıkta, pencereler
  duvarda eşit aralıkta kalır. Kilitli ölçü ile ölçülen değer ayrı kavramlardır.
- [Chief Architect — Moving walls using dimensions](https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html):
  hangi duvarın hareket edeceği ölçünün tıklanan ucuyla seçilir.
- BricsCAD BIM QuickDraw V27 (plan §2'deki gibi arama dizininden): bitişik oda çizerken var olan duvar benimsenir.

KentOS kararı: (a) Dikdörtgen duvar var olan duvarın üstüne düşen kenarı çizmez, onu benimser; (b) Kaydır ve Mesafe'nin
planı iki yanın net ölçüsünü, mahallerin alanını, giden kapıları, ölçüleri ve bağların kaydırdığı duvarları söyler;
(c) “Şimdi bu ölçüye getir” (ADR 0710, tek seferlik) ile “Bu ölçüyü koru” (kalıcı bağ, kullanıcının seçimiyle) ayrıdır;
bağlar çizimin kısıt kümesindedir (tek otorite), çözümleri mimari bağdaştırıcıdır: duvar yüz, eksen ve çekirdek çizgileri
kenarın kendine dik kaymasına çevrilir, kaymalar ADR 0710'un kuralıyla (bağlı duvarlar izler) uygulanır.

## Karar

### 1. Bitişik oda: Dikdörtgen duvarın ortak duvarı benimsemesi

`cad.walls.rectangle`'ın yeni `adopt` alanı (öntanımlı `false`; eski çağrılar bayt bayt aynı). `true` iken dikdörtgenin
her kenarı (yolun saat yönü tersine düzenlenmiş hâliyle, sol yanı odanın içi) için:

- **Referans çizgisi:** İç net → kenarın kendisi (iç yüz); Eksen → kenar (eksen); Dış ölçü → kenar (dış yüz).
- **Benimsenen parça:** aynı kattaki (ikisi de katsız sayılır, ADR 0641) bir duvarın düz kenarı bu kenara 0,001° içinde
  paralelse ve onun aynı türden çizgisi kenarın referans çizgisiyle 1 mm (`JOIN_TOLERANCE`) içinde çakışıyorsa:
  İç net için var olan duvarın odanın içine bakan yüzü (kenarın sol normaline göre konumu büyük olan yüzü), Eksen için
  ekseni, Dış ölçü için odadan uzak yüzü (konumu küçük olanı). O duvar kenarının gövdesinin (iki yüzünün kenar uçlarının)
  kenara izdüşümünün kenarla kesişen aralığı benimsenir (çizilmez): Dış ölçüde de köşede yarım duvar kalmaz. Birden çok
  duvarın aralıkları birleştirilir (1 mm'yi aşmayan boşluk kapanır; halkanın başını aşan aralık sonuncuyla birleşir).
- **Kalan yollar:** kenarlardan benimsenmeyen aralıklar halkanın sırasıyla yürünür; köşede birbirine ulaşan parçalar tek
  açık yol olur. Hiç benimsenen yoksa duvar eskisi gibi tek kapalı duvardır. Boyu 1 mm'yi aşmayan kalan parça benimsenmiş
  sayılır. Her yol ayrı bir açık duvardır: aynı genişlik, stil, hiza (referansın hizası), yükseklik, kat, renk; uçları
  öntanımlı birleşimle (benimsenen duvara T ya da köşe olarak birleşir).
- **Ret:** her kenar bütünüyle benimsenmişse `nothing_to_set` (“Dikdörtgenin bütün kenarları var olan duvarlarda;
  çizilecek yeni duvar kalmadı.”). Öbür retler eskisi gibi.
- **Çıktı:** `WallsCreated`'ın yeni `adopted` alanı: benimsenen duvarların kimlikleri, çizimin sırasıyla.

Var olan duvarlar, açıklıkları ve mahalleri değişmez (kimlikleri korunur). Mahal oluşturulmaz (ADR 0656'daki gibi);
Mahal aracı ya da Toplu mahal iki odayı bulur.

### 2. Kaydırmanın köşede aynı doğrultudaki duvar ucu

ADR 0710 §1'in kuralına ek: kaydırılan kenarın iki kenar arasındaki köşesinde başka açık duvarların uçları varsa, uçlarının
son kenarı köşenin kaydığı bitişik kenarla 1° (`INLINE_DEGREES`) içinde aynı doğrultudaysa o uçlar köşeyle aynı noktaya
gider (kendi kenarları boyunca kısalır ya da uzar). Başka doğrultudaki uç eskisi gibi reddedilir. Eksen referanslı
bitişik oda (dikdörtgenin köşesinde biten yeni duvar) böylece kaydırılabilir.

### 3. Kaydırmanın etkisi (önizleme)

`cad.walls.edit`'in Kaydır, Mesafe ve Boy işlemlerinde çıktının (planının da) yeni `effects` alanı:

- `gaps`: kaydırılan kenarın sol ve sağ yanındaki net mesafe (ADR 0710 `cad.walls.distances`'ın kuralı) önce ve sonra;
  yanda paralel duvar yoksa yok.
- `spaces`: sınırı değişen mahaller (adım sonunda mahalin izleyicisinin bulacağı gibi): kimlik, numara, ad, alan önce ve
  sonra (mahalin yönteminin net alanı, m²), `broken` (kapanmıyorsa).
- `openings`: dünyadaki yeri ya da duvarındaki `at`'ı değişen açıklıklar; `dimensions`: değeri değişen ilişkili ölçüler;
  `fixtures`: izleyen mutfak dizileri ve modülleri; `bonded`: bağların (§4) kaydırdığı duvarlar (izleyenleriyle).

Kilitli katmandaki izleyen duvar eskisi gibi bütün işlemi `layer_locked` ile reddeder; ileti katmanı adlandırır.

### 4. Kalıcı mimari bağlar

**Veri.** Bağlar çizimin kısıt kümesindeki (`ConstraintSet`, ADR 0334) kısıtlardır. Kısıtın parçasına iki yeni tür:

- `CadElement::Wall { edge, line }`: duvarın düz `edge` kenarının bir çizgisi; `line` ∈ `leftFace`, `rightFace`, `axis`,
  `leftCore`, `rightCore` (yolun yönüne göre sol ve sağ; yüzler ilk ve son çizgi, eksen ikisinin ortası, çekirdek en
  güçlü bileşen bandının sınırları (ADR 0729), bileşensiz duvarda yüzler).
- `CadElement::Jamb { end }`: açıklığın `start` ya da `end` pervazı (ADR 0694'ün pervaz orta noktası).

Mimari öğeli kısıt yalnız şu kurallardan olur; öbür kurallar ve genel öğelerle karışık kısıt `rule_problem` ile reddedilir:

| Kural | Öğeler | Anlamı |
|---|---|---|
| `distance` | duvar çizgisi `a`, duvar çizgisi `b` (paralel) | `n_a·(P_b − P_a) = ±d`; işaret `left` (b, a'nın solunda) |
| `distance` | pervaz `a`, duvar çizgisi `b` | `n_b·(J_a − P_b) = ±d`; `left`: pervaz çizginin solunda; çizgi açıklığın duvarını en az 30° ile keser |
| `distance` | pervaz `a`, pervaz `b` (aynı duvar kenarı) | `h·(J_b − J_a) = d` (`h` kenarın yönü; `a` önce gelir) |
| `parallel` | duvar çizgisi, duvar çizgisi | çizgiler paralel kalır |
| `symmetric` | `a`, `b` ve `axis` duvar çizgileri (paralel) | `axis` ikisinin ortasında: eşit aralık (n duvar için ardışık üçlüler) |
| `fix` | duvar çizgisi ya da pervaz | yerinde kalır |

`reference: true` mesafe yalnız ölçer (ADR 0334). Değer projenin parametresidir (`d1`, `d2` …, ifadeyle).

**Çözüm (mimari bağdaştırıcı).** Bilinmeyenler: bağlarda adı geçen her duvar kenarının kendine dik kayması `Δ` (kenarın sol
normali boyunca) ve her açıklığın duvar kenarı boyunca kayması `δ`. Bir kenardaki bütün çizgiler aynı `Δ`'yı paylaşır;
çizginin kenara uzaklığı (yüz, eksen, çekirdek) sabittir. Kurallar doğrusaldır:

- duvar–duvar mesafe: `ε·Δ_b − Δ_a = σd − g₀` (`ε = n_a·n_b = ±1`, `σ = ±1` işaret, `g₀` şimdiki işaretli aralık);
- pervaz–duvar mesafe: `(n_b·h)·δ_a + (n_b·n_h)·Δ_h − Δ_b = σd − g₀` (`Δ_h` açıklığın kenarının kayması, kenar bilinmeyense);
- pervaz–pervaz mesafe: `δ_b − δ_a = d − g₀`;
- simetrik: `ε_a·Δ_a + ε_b·Δ_b − 2Δ_axis = 2s_axis − s_a − s_b` (`s` ortak normal `n_axis` boyunca konum);
- sabit: `Δ = 0` (`δ = 0`);
- paralel: denklem yok; çizgiler paralel değilse çelişki.

Tutulanlar (`Δ = 0`, `δ = 0`): yazmanın oynattığı çizgiler (kenarı yer ya da doğrultu değiştiren duvarlar), dünyada kendi
duvarı boyunca yer değiştiren açıklıklar (başlangıç pervazının yazmadan önceki `place`'ine göre; duvarıyla taşınan
açıklığın `at`'ının değişmesi tutmaz), kilitli katmandaki nesneler ve eşit aralığın (`symmetric`) iki ucu (ekseni izler;
en az değişiklik yan duvarı oynatmasın diye). Bir yazmadan sonra (ADR 0337 §1'in yolu, aynı adımda): paralel olması gereken çizgiler 0,001° içinde değilse
ya da sabit bir çizgiyi yazma oynatmışsa çelişki; sonra kalan bilinmeyenler için `A·x = c` en az değişiklikle (en küçük
`Σx²`, Gauss yok etme, satır başına 1e-9 m) çözülür. Tutarsız satır kalırsa çelişki: o satırı oluşturan bağlar
`constraint_conflict` ile adlandırılır (`uids`'te kısıt kimlikleri), hiçbir şey yazılmaz. Çözüm uygulanır: her `|Δ| > 1e-9`
için ADR 0710'un Kaydır'ı (kimliklerin sırasıyla; bağlı duvarlar izler), açıklıklar pervazlarından yeni yola okunur, sonra
`at`'a `δ` eklenir. Kaydır'ın reddi bağın reddidir (`constraint_conflict`, Kaydır'ın sözüyle). Sonra bütün bağlar yeniden
ölçülür; 1e-6 m'yi aşan kalan varsa çelişki. Bir yazmanın ardından değil de kısıt kümesinin
kendisi değişince (yeni bağ, parametrenin yeni değeri) şimdi tutmayan her mesafe bağının `a`'sı tutulur; `b` ve onu
izleyenler kayar. Gevşeterek düzenleme (ADR 0337 §3) mimari bağlar için de geçerlidir. Kesen komutlar ve duvarın biçimi
(kenar sayısı) değişince adı geçen kenarın bağları ADR 0337'deki gibi gider (`constraints_removed`).

Genel çözücü mimari öğeli kısıtları görmez; mimari bağdaştırıcı genel kısıtları görmez. Tek kayıt, tek geri alma adımı,
tek ret yolu.

### 5. Komutlar

- `cad.walls.rectangle` v1: `adopt` (§1), çıktının `adopted`'ı.
- `cad.walls.edit` v1: Mesafe'nin `keep`'i (“Bu ölçüyü koru”): kaydırmadan sonra aynı adımda karşı duvarın bakan yüzü
  `a`, kaydırılan duvarın o yandaki yüzü `b`, değer `distance` olan mesafe bağı ve parametresi yazılır; çıktının `bonds`'u
  yeni kısıtın kimliği. `effects` (§3). Yeni ret yok; bağlı duvar bağlarla izler.
- `cad.constraints.edit` ve `.query` v1 mimari öğeleri alır ve verir; ölçülen değer ve parçaların anlamı aynı kuralla.
  Yeni ret: `invalid_constraint` (mimari öğe kuralına uymayan, duvar olmayan nesnede duvar çizgisi, yay kenar, kesişmeyen
  pervaz çizgisi).
- Python (`kentos.cad.walls.rectangle(adopt=True)`, `walls.edit` `keep`, `constraints.*`) ve MCP katalogdan.

### 6. Masaüstü

- **Dikdörtgen duvar:** görünür ayarlarda “Ortak duvar” (Benimse / Çiz; yeni araç oturumunda Benimse). Önizleme komutun
  planıdır: benimsenen kenar çizilmez, benimsenen duvar vurgulanır.
- **Geçici ölçü (ADR 0710):** duvar yüzüne olan her yanın sayısının üstünde (dik duran ölçüde sağında) kilit karesi.
  Bağsız yanda içi boş; basmak “Bu ölçüyü koru”dur (`cad.walls.edit` Mesafe'nin `keep`'i, değer şimdiki net ölçü: hiçbir
  şey kımıldamaz, yalnız bağ yazılır); bağlı yanda dolu kare ve yanında parametrenin adı (vurgu renginde), basmak bağı
  kaldırır (`cad.constraints.edit` `remove`). Aksa olan ölçü bağlanmaz. Bağlı yanın sayısına yazılan değer parametreyi değiştirir (`cad.parameters.edit`; karşı duvar kalır),
  bağsız yanınki ADR 0710'daki gibi Mesafe'dir. Sayı yazılırken önizleme kartı §3'ün etkisini söyler (iki yanın net ölçüsü,
  mahallerin alanı, kapılar, ölçüler, bağların kaydırdığı duvarlar; ret kırmızı). Enter tek adım yazar, Esc hiçbir şey.
- **Bağların işareti (ADR 0338):** Kısıtları göster açıkken mimari bağ, bağlı duvar çizgisinin ortasında ADR 0338'in
  işaretiyle (türün ikonu, `d1 = 2.6`) çizilir; tutmayan tehlike renginde. Çelişkide komutun iletisi reddi ve bağı
  kaldırma ya da gevşeterek düzenleme yolunu söyler.

### 7. Dosya

Mimari öğeli kısıt `.kcad` şema 137 ister (opt-in; ADR 0265): yazıcı ancak bir kısıt `wall` ya da `jamb` öğesi adlandırınca
137 yazar, okuyucu 137'den küçük şemada bu öğeleri reddeder. `FORMATS_VERSION` 100.

### 8. Performans

Bağdaştırıcı yalnız bir yazmanın oynattığı nesnelerin bağlı bileşenini çözer; yoğun Gauss yok etme bileşenin boyunda
(onlarca bilinmeyen). Bütçe: 100 bağlı, 400 duvarlı katta bir kaydırmanın bağ çözümü ve uygulanması ≤ 20 ms (release).

## Sınırlar

- Yay kenar, kolon, aks ve döşeme bağı yok; duvarın boyu ve ucu bağlanmaz; dönen (döndürülen) bağlı duvar çelişkidir,
  bağdaştırıcı döndürmez. Eğik (dik olmayan) pervaz–duvar bağı en az 30° ile sınırlı.
- Bağlar ADV-05'in modül örnekleri arasında taşınmaz; bağımsız bölüm (ADV-04) kapsamı yok.
- Oda bir nesne türü değildir: duvarlar ve mahal ayrı kalır; mahal tohumu kaydırmayla taşınmaz (tohum duvara düşerse mahal
  ADR 0342'deki gibi kopar ve etki önizlemesi bunu söyler).

## Uygulama

- Çekirdek: `ops::arch_bonds` (çizgilerin kenara uzaklığı `line_offset`, satırlar, en az değişiklikli çözüm ve çelişki
  nedeni), `ops::wall_adopt` (benimsenen aralıklar ve kalan yollar), `ops::wall_shift`'in köşe kuralı (§2).
- Sözleşme: `CadElement::Wall`, `CadElement::Jamb`, `WallLine`, `JambEnd`, `rule_is_architectural`; komutlar
  `WallsRectangle.adopt`, `WallsCreated.adopted`, `WallsOperation::Distance.keep`, `WallsEdited.effects` ve `.bonds`,
  `WallEffects`, `GapChange`, `SpaceAreaChange`.
- Uygulama katmanı: `arch_bonds.rs` (çizimden okuma, tutulanlar, çözüm, uygulama, `rule_problem`, `part_place`,
  `side_bonds`); `constraint_solve::solve_set` mimari kuralları ayırıp bağdaştırıcıya verir, sonuçları birleştirir;
  `walls_edit.rs`'in `effects` (okuma kopyasında adımın tamamı ve izleyicileri) ve `kept`; `walls_rectangle.rs`'in
  benimsemesi; `constraints_edit::add_bond`.
- Belge: kenar sayısı değişen ya da yaylaşan duvarın ve açıklık olmayan pervazın bağı ADR 0337'deki gibi gider.
- `.kcad`: `SCHEMA_WITH_ARCH_BONDS` 137, `FORMATS_VERSION` 100; `tools/kcad/kcad.py` aynı kural.
- Masaüstü: `Tool::text_draft` (yazı kutusu yazıldıkça araca), Duvarın mesafesi'nde etki kartı (`effect_lines`, yazının
  40 px altında, ret tehlike renginde), kilit kareleri (`Switch::Bond`, `toggle_bond`), bağlı yanın Enter'ı
  `cad.parameters.edit`; Dikdörtgen duvarda Ortak duvar (O: Benimse / Çiz) ve benimsenen duvarın vurgusu.

## Doğrulama

- Bağımsız başvurular (KentOS kodu olmadan): `wall_adopt_cases.py` → `cad.walls.rectangle.adopt.json` 26 durum;
  `wall_shift_cases.py` (köşe kuralı ve `effects`) → `cad.walls.edit.shift.json` 42, `.length.json` 29;
  `arch_bond_cases.py` → `cad.constraints.edit.arch.json` 31 durum (koridor bağı, izleme, çelişki, sabit, parametre,
  eşit aralık, pervaz–duvar ve pervaz–pervaz, zincir, gevşetme, kilitli katman, başvuru, paralel, geri alma, Bu ölçüyü koru
  dört durum, altı ret); `kcad_v2_reference.py` (`arch-bonds.kcad`, üç bozuk örnek, `schema-version-138.kcad`).
- Rust: `KENTOS_CASES_ONLY=cad.walls,cad.constraints,cad.parameters cargo test -p kentos-native-application --test all
  every_command_case`; çekirdeğin `ops::` testleri (230); `kentos-kcad` (234); `kentos-interaction` (818);
  masaüstünde `wall_scenes::bond_tests` (kilit karesi bağ yazar, dış duvar 40 cm kayınca bağlı iç duvar izler, tek geri
  alma, ikinci basış bağı kaldırır; yazarken kart, hiçbir şey yazılmaz); clippy `-D warnings` değişen sekiz pakette.
- Resimler (`.run/shots/arac-duvar-mesafe-etki-*`, `-bag-*`, `-bag-etki-*`, `-bag-izledi-*`; 1440×900 ve 1100×650, açık ve
  koyu): etki kartı “Bu yan mesafe: 4.000 m → 3.600 m / Karşı yan mesafe: 2.600 m → 3.000 m”; kilitlenen yanın dolu karesi
  ve `d1`; dış duvar 40 cm doğuya kayınca iç duvar izliyor, bağlı yan 2.600 m kaldı.
- Ölçülmedi: §8'in 20 ms bütçesi; gerçek pencerede elle deneme yapılmadı.
