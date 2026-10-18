# ADR 0334: Kısıt modeli ve çözücü

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §13 `CAD-PARAM-01`, `05`, `06` (sahibin sözü: “cad param
  ile devam et ve bitene kadar durma”; beş grubun ilki). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0249 ve 0263 (ilişkili ölçünün alt nesne başvurusu, `SubPlace`), 0253 (komut protokolü), 0265
  (yeni şemanın altı adımı), 0149 ve [numeric-policy](../specs/numeric-policy.md) (sayısal doğruluk), 0214 (projenin
  değişkenleri: kısıt parametresi onlardan ayrıdır).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

AutoCAD'in geometrik kısıtları (GEOMCONSTRAINT) nesnelerin parçaları arasındaki ilişkileri, ölçüsel kısıtları (DIMCONSTRAINT)
ve parametre yöneticisi (PARAMETERS) bu ilişkilerin değerlerini tutar: dikdörtgenin genişliği değişince karşı kenarlar paralel,
köşeler dik kalır. KentOS'ta “constraint” sözü yalnız paftanın yerleşiminde ve ayarların sınırlarında geçiyordu; çizimin
geometrisi için bir kısıt modeli ve çözücü yoktu.

Planın kabulü iki şey ister: kısıtlar korunur, ve çelişkili değer bütün ev sahiplerinde aynı teşhisi verir, hiçbir geometri
yazılmaz. İkincisi çözücünün bir kütüphane ayrıntısı değil, sözleşme olmasını gerektirir: hangi kısıt fazla, hangisi çelişiyor,
kaç serbestlik kaldı, hangi nesne hâlâ oynar, her ev sahibinde aynı sırayla aynı cevap.

Bu ADR beş grubun ilkidir: model ve dosya (`PARAM-01`), çözücünün seçimi (`PARAM-05`) ve yapılandırılmış sonuçları (`PARAM-06`).
Komut, parametre ifadeleri, düzenlemelerde izleme, arayüz ve dış biçim sınırları sonraki gruplardadır (§9).

## Karar

### 1. Model (`CAD-PARAM-01`)

Kısıtlar çizimin kendi durumudur, nesnelerin değil: belgenin `constraints`'i (`kentos_contracts::ConstraintSet`). Böylece bir
kısıt iki nesneyi bağlar ama ikisinin de verisine yazılmaz; nesneyi değiştiren her yol (komutlar, bulut, içe aktarma) kısıtları
bilmeden çalışmaya devam eder, izleme bir sonraki grubun kuralıdır.

- **Kısıt** (`Constraint`): kalıcı kimliği (`ConstraintId`, UUIDv7), kuralı (`ConstraintRule`) ve yalnız ölçen ölçüsel kısıtta
  `reference`. Kısıtlar eklendikleri sırayla tutulur; sıra anlamlıdır (§4: fazla ve çelişen kısıt sonra gelendir).
- **Parça** (`ElementRef`): nesnenin kalıcı kimliği (`uid`) ve parçası (`CadElement`): nokta (ilişkili ölçünün `SubPlace`'i:
  baş, son, merkez, düğüm, köşe), düz kenar (`part`, `ring`, `index`; çizgi `0, 0, 0`) ya da daire (dairenin kendisi ya da
  yayın çemberi). İlişkili ölçünün başvurusu gibi kimlik ve yerle anılır; nesnenin belge içindeki yuvası dosyaya girmez.
- **On beş kural:** Çakışık, Yatay, Düşey, Paralel, Dik, Teğet, Eş merkezli, Doğrudaş, Eşit, Simetrik, Sabit (geometrik);
  Uzaklık, Açı, Yarıçap, Çap (ölçüsel). Yön seçenekleri (teğetin `left` ve `inside`'ı, uzaklığın `left`'i, açının `reverseA`,
  `reverseB`'si) kısıt yapılırken çizimden okunur ve çözüm o yanı korur: aynı denklemin öbür kökü başka bir şekildir.
- **Parametre** (`CadParameter`): adı, türü (uzunluk, açı, sayı), ifadesi, isteğe bağlı aralığı ve açıklaması. Ölçüsel kısıt
  parametresini adıyla anar ve onu sürer (ifade yazılır) ya da ölçer (başvuru kısıtı, ifade boş). İfade dili kendi küçük
  dilbilgisidir (`kentos_contracts::constraint_expr`): birimli sayılar, adlar, dört işlem ve üs, on üç işlev ve `pi`; uzunluk
  çizim biriminde, açı projenin açı biriminde okunur (`ParamUnits`). Projenin değişkenlerinin (ADR 0214, `@ad`) ve ifade dilinin
  (ADR 0100) yerine geçmez: parametre geometriyi süren sayıdır, metin ya da nesne değeri değildir. Parametreler birbirini döngüyle
  okumaz; döngü adlarıyla söylenir (`param_order`).
- **Kurallar** (`constraints_problem`): en çok 10 000 kısıt ve 10 000 parametre; kimlik boş değil ve bir kez; kuralın parçaları
  ona uyar (paralelin iki parçası kenar, yarıçapınki daire …); nesne çizimdedir; geometrik kural başvuru olmaz; parametre vardır,
  türü uyar ve tek kısıtındır; adlar geçerli ve bir kez; ifade okunur, okuduğu adlar vardır, döngü yoktur; sınırlar sonlu ve
  sıralıdır. Yer `constraints/<i>` ya da `parameters/<i>`'dir.
- **Belge:** kümenin değişmesi tek geri alma adımıdır (`Op::Constraints`, önce ve sonra); `set_constraints` kuralları nesnelerin
  kimlikleriyle denetler, reddi `Refusal`'dır. Bulutta ve protokolün damgalarında taşınması komutla birlikte gelir (§9, G3).
- **Silinen nesne:** bir adım kısıtlı bir nesneyi silerse kısıtları aynı adımda gider (AutoCAD'in kuralı; belgenin
  `follow_constraints`'i, izleme kurallarının sonuncusu). Ölçü kısıtının parametresi de onunla gider, ancak başka bir parametre
  onu okuyorsa kullanıcı parametresi olarak kalır; başvuru kısıtınınsa silinen nesneden ölçülen değeri ifadesi olur. Böylece
  çizim her adımdan sonra kaydedilebilir kalır (kısıt olmayan bir nesneyi anamaz). Geri alma kısıtları da geri getirir.
- **Eşleme:** sözleşmenin kuralı çözücünün kuralına belgenin `kentos_domain::constraints::core_rule`'unda bir kez çevrilir;
  komutlar ve belge aynı eşlemeyi kullanır.

### 2. Dosya: şema 73

`.kcad`'in belgesinin `constraints` anahtarı, sözleşmenin JSON'u opak değer olarak (§6.15). Yazıcı `73`'ü yalnız kısıt ya da
parametre varken yazar; başka her çizim eskisiyle bayt bayt aynıdır. Okuyucu değeri sözleşmenin türüyle çözer, yeniden JSON'a
çevirdiğinde aynı değeri bulmalıdır (yazıcının biçimi: varsayılan değerinde alan yazılmaz), boş küme reddedilir, kurallar önce
nesnesiz, nesneler okunduktan sonra kimliklerle denetlenir. Altı adımın hepsi (ADR 0265): `constraints.kcad` bağımsız Python
yazıcısının baytlarıyla aynıdır; `constraints-in-schema-72.kcad`, `schema-version-75.kcad` ve altı bozuk örnek
(`-empty`, `-unfit`, `-object-missing`, `-round`, `-reference-written`, `-unknown-field`); `tools/kcad/kcad.py` aynı kuralları
alan alan ve ifade dilbilgisinin kendi yazımıyla okur.

### 3. Denklemler

Her kısıt bir ile dört denklem verir; denklem sıfırken kısıt tutar (`ops::constraints::rules`). Uzunluk denklemi metredir;
doğrultu denklemi (paralel ve dikte açının sinüsü ya da kosinüsü, açıda kalan açı) çizimin boyu `s` ile çarpılır, böylece her
satır aşağı yukarı bir uzunluk kadar tartılır.

| Kural | Denklem |
|---|---|
| Çakışık | nokta–nokta iki fark; nokta–kenar kenarın doğrusuna işaretli uzaklık; nokta–daire merkeze uzaklık eksi yarıçap |
| Yatay, Düşey | kenarın iki ucunun y'si (x'i) farkı; iki noktanınki |
| Paralel, Dik | birim doğrultuların vektörel (skaler) çarpımı × `s` |
| Teğet | kenar–daire: merkezin seçilen yana işaretli uzaklığı eksi yarıçap; daire–daire: merkezler arası eksi yarıçapların toplamı (içten: farkının mutlak değeri) |
| Eş merkezli | merkezlerin iki farkı |
| Doğrudaş | ikinci kenarın iki ucunun birincinin doğrusuna uzaklığı |
| Eşit | boyların ya da yarıçapların farkı |
| Simetrik | iki noktanın ortası eksende, birleştiren doğru eksene dik |
| Sabit | parçanın sayıları eksi sabit yerinin sayıları (sabit yer düzenlemenin öncesinden, `fix_from`) |
| Uzaklık | hizalı boy, yatay ya da düşey işaretli fark, ya da kenara işaretli uzaklık; eksi değer |
| Açı | `a`'nın doğrultusundan değer kadar dönmüş doğrultuya `b`'nin açısı, (−π, π] × `s` |
| Yarıçap, Çap | yarıçap (iki katı) eksi değer |

Türevler ikili sayılarla kesindir (`dual`, 16 yuva): sonlu fark hatası yoktur. Bir kısıtın denklemi tek yerde yazılır ve hem
değeri hem Jakobiyen satırını verir.

Teğetle, değdiği noktadaki çakışık birlikte çift kök verir (yayın ucu doğruya teğet otururken): çözümde iki satır bağımlı olur ve
Newton yavaşlar. Bu, kısıt dilinin bilinen tuzağıdır (SolveSpace'te ayrı “uçtan teğet” kısıtı vardır). Kısıt ekleme komutu
(G2) uçları çakışık yay ve doğruya teğeti uçta teğet olarak yazar: merkez doğrunun ucuna diktir.

### 4. Çözücü (`CAD-PARAM-05`)

**Seçim: kendi Rust çözücümüz.** Değerlendirilenler:

| Seçenek | Lisans | Neden değil |
|---|---|---|
| planegcs (FreeCAD'in çizim çözücüsü) | LGPL-2.1+ | C++ ve Eigen, Boost; yoğun QR ile teşhis büyük çizimde ağır; WASM ve kendi `libm` kuralımız (ADR 0008) için ayrı yapı ve yama; teşhis FreeCAD'in kendi sırasıyla |
| SolveSpace (`libslvs`) | GPL-3.0 | lisansı KentOS'un dağıtımıyla bağdaşmaz |
| Genel en küçük kareler kütüphaneleri (Ceres, `argmin`, `levenberg-marquardt`) | BSD, MIT, Apache | yalnız çözer, teşhis etmez: fazla ve çelişen kısıtın sırası, serbestlikler ve serbest nesneler yine bizim işimiz |

Bu yüzden dışarıdan bir kütüphane ölçülmedi; ölçmek için önce bağımlılık eklemek gerekir, o da sahibin onayıyla olur (CLAUDE.md §3).
Kendi çözücümüzün bağımlılığı yok, `libm`'le her platformda aynı sayıları verir, teşhisi kısıtların sırasıyla sözleşmedir ve
çekirdekte olduğu için masaüstü, başsız sunucu, Python ve MCP aynı çözücüyü kullanır.

**Algoritma** (`ops::constraints::solve`):

- **Bilinmeyenler** nesnelerin kendi sayılarıdır (`model`): nokta x, y; çizgi iki ucu; daire merkezi ve yarıçapı; yay merkezi,
  yarıçapı ve iki açısı; yol bütün köşeleri. Yaylı kenar düz kenar değildir, ona kısıt bağlanmaz (kısıt kopar, §5). Tutulan
  bilinmeyenler (`Hold`: sürüklenen tutamak, değişmeyecek nesne) sabittir.
- **Yerel orijin:** koordinatlar ortalarına kaydırılarak çözülür; uzak koordinatlı çizim (512 000, 4 420 000) aynı doğrulukla
  çözülür (ortak durumlardan biri).
- **Gruplar:** serbest bilinmeyen paylaşmayan denklemler ayrı çözülür (birleşim–bulma); bin ayrı dikdörtgen bin küçük
  problemdir.
- **Adım:** en az değişikliğin Gauss–Newton adımı. Jakobiyen seyrektir (satırda en çok 16 değer); bilinmeyenler tartılır
  (`W`: uzunluk 1, yayın açısı yarıçapıyla), satırların Gram matrisi `J·W⁻²·Jᵀ` kısıtların sırasıyla seyrek LDLᵀ'ye ayrılır
  (`sparse`): bir satırın pivotu, önceki tutulan satırların uzayına uzaklığının karesidir; pivotu kendi boyunun 10⁻¹⁴'ünden küçük
  satır öncekileri tekrarlar ve tutulmaz. Adım tutulan satırların en küçük normlu çözümüdür,
  `dz = J_Iᵀ·(J_I·J_Iᵀ)⁻¹·(−r_I)`, `dx = W⁻¹·dz`; kareler toplamını düşürmüyorsa yarıya bölünür. En çok 64 adım; tolerans
  `10⁻¹¹·s` (`s` çizimin boyu, en az 1 m).
- **En az değişiklik** çizimin anlamını korur: küçük bir değişiklik şekli içinden çevirmez; gerektiğinde komut yönelimleri ayrıca
  karşılaştırır (G2, `PARAM-07`).
- **Durdurma:** her adımdan önce `stop` sorulur; durdurulan çözüm hiçbir şeyi değiştirmez (`Cancelled`).

### 5. Sonuçlar (`CAD-PARAM-06`)

`solve` bir `Outcome` döner; sözleşmenin karşılığı `SolveStatus` ve `ConstraintDiagnosis`'tir.

| Durum | Anlamı | Şekiller |
|---|---|---|
| `fully` | her kısıt tutar, hiçbir şey oynamaz | yazılabilir |
| `under` | her kısıt tutar, serbestlik kalır (`dof`, `free`) | yazılabilir |
| `over` | her kısıt tutar, bazıları öncekileri tekrarlar (`redundant`) | yazılabilir |
| `inconsistent` | tekrarlayan bir kısıt tutmuyor (`conflicting`: sonra gelen) | **değişmez** |
| `singular` | parçası anlamını yitirmiş kısıt (sıfır boylu kenara paralel; `singular`) | **değişmez** |
| `nonConvergent` | adımlar bitti, kısıtlar tutmuyor | **değişmez** |

- `redundant`, `conflicting`, `singular`, `broken` kısıtların, `free` ve `changed` nesnelerin sıralı numaralarıdır.
- **Kopuk** (`broken`): nesnesi olmayan, parçası nesnesinde olmayan (dairede kenar, yaylı kenar) kısıt; çözüme girmez, öbürleri
  çözülür. Yalnız kopuk kısıtlarda geçen nesnenin serbestliği sayılmaz.
- **Serbestlik:** serbest bilinmeyenler eksi tutulan satırlar. **Serbest nesne:** satırların izin verdiği bir hareketin hâlâ
  oynattığı bilinmeyeni olan nesne; genel bir vektörün satırların sıfır uzayındaki parçasıyla bulunur (iki vektör, ikişer kez
  izdüşüm).
- **Ölçen kısıt:** başvuru kısıtının çözümden sonraki değeri `measured`'dadır; `measure` tek bir kuralın bugünkü değerini verir.
- Bütün nesneleri tutulan kısıt yalnız denetlenir: tutuyorsa fazla, tutmuyorsa çelişen.
- `changed`: çözümün yuvarlama boyundan (`10⁻¹⁴·s`) fazla oynattığı nesneler; çağıran yalnız onları yazar.

### 6. Hız

Bayt bayt aynı çizim, `cargo test --release -p kentos-geometry-core --lib constraints::tests::timing -- --ignored`; dikdörtgenler
köşeleri çakışık, tabanı yatay, karşı kenarları paralel, köşesi dik, iki ölçülü; ızgarada her dikdörtgen komşusuna bağlı, tek
grup. Bu makinede (Linux, release):

| Çizim | Kısıt | Süre |
|---|---|---|
| 1 000 ayrı dikdörtgen | 11 000 | 22 ms |
| 10 × 10 bağlı dikdörtgen | 1 100 | 2,9 ms |
| 20 × 20 bağlı dikdörtgen | 4 400 | 10,7 ms |
| 50 × 50 bağlı dikdörtgen | 27 500 | 75 ms |
| 20 × 20 ölçüsüz (800 serbestlik) | 3 600 | 5,8 ms |
| 50 × 50 ölçüsüz (5 000 serbestlik) | 22 500 | 44 ms |

İlk sürüm yoğun Gram–Schmidt'le aynı işi 10 × 10'da 10,2 s'de, 20 × 20'de 663 s'de yapıyordu: satırların yoğun tabanı
satır × satır × bilinmeyen kadar iş istiyordu. Seyrek LDLᵀ yalnız bağlı satırların doldurduğu yerleri işler.

### 7. Doğrulama

- `python3 scripts/fixtures/constraint_cases.py --check`: 33 durum (`fixtures/constraints/v1/cases.json`), KentOS kodu olmadan:
  denklemler sympy'yle yeniden yazılır, rank 50 basamakta (mpmath) kısıtların sırasıyla, çözüm 10⁻⁴⁰'a kadar en küçük normlu
  Newton'la; durum, serbestlik, fazla, çelişen, tekil, kopuk, serbest nesneler, ölçülenler ve tam kısıtlı çözümlerin şekilleri.
- `cargo test -p kentos-geometry-core --test all constraints`: çözücü aynı durumları geçer; `KENTOS_CONSTRAINTS_OUT=FILE` ile
  yazdığı şekiller `constraint_cases.py --outputs FILE` ile başvurunun denklemleriyle denetlenir (serbestliği kalan çözüm tek
  değildir).
- `cargo test -p kentos-domain --test all constraints`: silinen nesnenin kısıtları ve parametreleri, okunan parametre, geri
  alma; kalan küme dosyanın kurallarını tutar.
- Çözücünün kendi testleri (`ops::constraints::tests`): genişleyen dikdörtgen, çelişki, fazlalık, serbestlikler, teğet, yay,
  ölçen kısıt, kopuk parça, tutulan tutamak, durdurma, bağlı ızgara; seyrek ayrışımın testi.
- `kentos-kcad`'in testleri ve `python3 scripts/fixtures/kcad_v2_reference.py --check`: şema 73'ün örnekleri.

### 8. Sonuçlar

- Kısıtlar çizimin dosyasında, geri alma adımıyla ve kurallarıyla vardır; çözücü teşhisiyle çekirdektedir.
- Henüz kullanıcının kısıt ekleyebileceği bir komut ya da araç yoktur; dosyadan gelen kısıtlar korunur, silinen nesneyle düşer,
  ama öbür düzenlemeler onları izlemez (G2–G4).

### 9. Sonraki gruplar

- **G2 (ADR 0335):** `PARAM-02`, `03`, `07`: `cad.constraints.edit` (ekle, sil; yönün çizimden okunması, sürücü ve başvuru),
  yönelimin korunması ve tersine dönme teşhisi, Kısıt ekle aracı.
- **G3 (ADR 0336):** `PARAM-04`, `10`, `12`: parametreler, ifadeler, birimler, aralıklar ve döngüler komutta; kısıt, parametre ve
  çözülen geometri tek adımda; protokolün damgaları, bulut ve veritabanı; Python ve MCP.
- **G4 (ADR 0337):** `PARAM-08`, `09`, `13`: tutamak, taşı ve esnet kısıtları korur (geçici gevşetme seçeneği); buda, birleştir,
  kır ve kopyala kısıt kaybını ya da kopyasını plan ve uyarıyla söyler; uzun çözüm işte ve iptalle.
- **G5 (ADR 0338):** `PARAM-11`, `14`, `15`: kısıt işaretleri, bağlı geometrinin vurgusu, Parametreler penceresi ve çelişkinin
  çözüm akışı; mimari nesnelerin ilişkileriyle otorite; DWG ve DXF'te kısıtın sınırı.
