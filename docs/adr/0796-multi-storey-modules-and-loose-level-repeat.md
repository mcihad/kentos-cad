# ADR 0796: Çok katlı oda modülü ve kattaki modülsüz nesnelerin tekrarı

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md `ADV-05`'in dördüncü dilimi (ADV-05d); önceki dilimler [ADR 0775](0775-room-modules-and-controlled-reuse.md),
  [ADR 0779](0779-room-modules-members-drift-and-flats.md), tasarım seçenekleri ADR 0788.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler “Belgelere eklenecek satırlar”da.
- **`.kcad`:** şema **152**, opt-in. `FORMATS_VERSION` değişmez: yeni alan proje
  ayarlarının JSON'undadır, tipli sütunlara girmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0779 iki şeyi açık bıraktı:

1. Modül tek kattaydı: bir dubleks ya da bir merdiven çekirdeği gibi birden çok kata yayılan nesneler tanımlanamıyordu
   (`module_levels`); dubleks iki ayrı modül olarak yerleştiriliyor, iki kat birbirini bilmiyordu.
2. Katları tekrarla yalnız modül örneklerini götürüyordu: kattaki modülsüz duvar, kapı, mahal, donatı, ölçü ve etiketler
   (hol, koridor, ortak alan) elle kopyalanıyor, kopyaların bağları (kapının duvarı, mahalin kaynakları, etiketin mahali,
   ölçünün noktaları) ayrıca kuruluyordu.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Graphisoft Archicad, “Place Hotlink”.** Çok katlı modül tek katıyla (Single Story) ya da bütün katlarıyla (All Stories)
  yerleşir; Match Stories modülün hangi katının ev sahibinin hangi katına geleceğini seçtirir. Ev sahibinde yeterli kat yoksa
  Archicad kat açmaz: “the module stories which do not ‘fit’ in the host project will not be visible”; çözüm ev sahibine kat
  eklemektir. <https://help.graphisoft.com/START/27/INT/_AC27_Help/080_Collaboration/080_Collaboration-58.htm>
- **Autodesk Revit, “About Modifying Groups”.** Grubun Reference Level'ı ve Origin Level Offset'i vardır; grup başka bir
  kata yerleşince öğeleri kendi seviyeleriyle taşınır.
  <https://help.autodesk.com/cloudhelp/2025/ENU/Revit-Model/files/GUID-B59163B4-E1DF-4C10-9DB9-F64B2861825E.htm>
- **Autodesk Revit, “Paste Aligned Elements” (Aligned to Selected Levels).** Kopyalanan model öğeleri adlarıyla seçilen
  katlara aynı yatay yerde yapıştırılır; görünüşe özgü öğeler (ölçüler gibi) bu yolla değil Aligned to Selected Views ile
  gider. <https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-Model/files/GUID-4E8562EC-38C4-485E-A86D-D459096187AF.htm>
- Revit'te grupla kat çoğaltmanın önerilen yolu bir katı grup yapıp Paste Aligned to Selected Levels ile öbür katlara
  yapıştırmaktır; birden çok katı tek gruba toplamak “tek bir nesne” verir (Autodesk forumu; resmî belge değil, yalnız
  bağlam: <https://forums.autodesk.com/t5/revit-architecture-forum/how-to-collect-all-levels-in-a-group/td-p/7427969>).

KentOS'un kararları ikisinden alınır ve ayrılır: modülün katları Archicad'in Match Stories'i gibi kat sırasıyla eşlenir, ama
sığmayan kat görünmez bırakılmaz, yerleştirme açık bir kodla reddedilir (§1). Modülsüz nesnelerin tekrarı Revit'in Paste
Aligned'ı gibi aynı yatay yerdedir; ama KentOS'ta ölçü ve etiket görünüşe özgü değil, kattaki model nesnelerine bağlı
nesnelerdir: kopyalarıyla birlikte giderler ve bağları kopyalara çevrilir (§2).

## Karar

### 1. Çok katlı modül

**Kat sırası.** Projenin katları kotlarına göre sıralanır, kotu eşit olanlar listedeki sıralarıyla
(`modules_edit::storey_order`). Listenin sırası değil kotlar belirler: bodrum, zemin, 1. kat, 2. kat.

**Tanım.** Tanımın üyesinin `storey`'i (`ModuleMember.storey`, 0–15): örneğin taban katının kaç kat üstünde durduğu. Yalnız
duvar, mahal ve donatı (kendi katı olan türler) kat taşır; açıklık duvarını, etiket nesnesini, ölçü noktalarının nesnelerini
izler ve 0'dır. Modül tanımla birden çok kattaki nesneleri reddetmez: taban katı en alçak kattır, her üyenin katı onun bu
taban üstündeki yeridir (`ArchModule::storeys`: en yüksek kat + 1). Katlı ve katsız duvar, mahal ya da donatı birlikte, ya da
projede olmayan bir kattaki nesne `module_levels` ile reddedilir; en çok 16 kat (`MAX_MODULE_STOREYS`).

**Yerleştirme ve tekrar.** `place`'in `level`'i ve `repeat`'in hedefi örneğin taban katıdır; n. kattaki üye taban katının n
kat üstündeki kata yazılır (`storey_level`). Üstünde yeterli kat olmayan taban ya da katsız yerleşen çok katlı modül
`module_storey_missing` ile reddedilir: ““Dubleks” modülü 2 katlı; “2. kat” katının üstünde 1 kat gerekir, projede 0 var. Daha
alçak bir taban katı seçin ya da projeye kat ekleyin.” Archicad'in sığmayan katı gizlemesi yerine ret seçildi: yarım bir
dubleksi sessizce yerleştirmek metrajı, bina programını ve çizelgeyi yanıltır; reddin cümlesi eksik kat sayısını söyler.

**Öbür işlemler.**

- Kaynağı güncelle: üyenin katı kaynağın nesnesinin bugün durduğu kattır (taban altına inmiş ya da katsız kalmış nesne
  tanımdaki katını korur); kat değişikliği de değişikliktir (`changedKeys`), öbür örneklerde nesne kendi karşılık katına
  gider. Kaynakla karşılaştırma belgenin kendi yazdıklarını (etiketin numaralı yazısı, açıklığın pervazları, kaynaklı mahalin
  halkaları) saymaz: ADR 0779'da numaralı bir örnekten güncelleme etiketlerini değişmiş sayıyor, yeni yerleşmiş örnekten bile
  “1 kaynak nesnesi değişir” diyordu (dubleksin pencere resminde bulundu).
- Modüle ekle: örneğin taban katındaki ya da üstündeki katlardaki nesne o katın üyesi olur; taban altındaki `module_levels`.
  Öbür bir örneğin üstünde o kat yoksa `module_storey_missing` (yol `operation.uids`).
- Elle değişen (ADR 0779 §3): beklenen nesne üyenin katındadır; elle başka kata alınan nesne elle değişmiş sayılır, Kaynağa
  döndür onu kendi katına geri yazar.
- Yerel istisna, bağımsızlaştırma, kaynağı silme, üye çıkarma değişmez; çıkarılan üyelerle tanım tek kata inebilir.
- `cad.modules.query`: modülün `storeys`'i (birden çoksa), örneğin `levels`'ı (durduğu katlar, taban önce).

Her yazma yine tek geri alma adımıdır: “Oda modülü”.

### 2. Kattaki modülsüz nesnelerin tekrarı (`loose`)

`repeat`'in `loose`'u açıkken kaynak katın modülsüz nesneleri de her hedefe düz kopya olarak gider. Katın nesnesi kat
görünümünün okuduğu kattır (`Entity::level`: duvarın, mahalin, donatının kendisi; açıklığın duvarınınki; etiketin nesnesininki;
noktalarının nesneleri tek kattaysa ölçününki). Kopyalanan türler duvar, açıklık, mahal, donatı, ölçü ve etikettir.

- **Bağlar.** Kopyanın kopyalanan nesnelere ve aynı adımda tekrarlanan örneklerin nesnelerine bağı (kapının duvarı, mahalin
  kaynakları, etiketin nesnesi, ölçünün noktaları) kopyalara çevrilir: holün duvarından dubleksin duvarına çekilmiş ölçünün
  kopyası tekrarlanan dubleksin duvarına bağlanır.
- **Kat ve numara.** Duvar, mahal, donatı hedef kata yazılır. Mahal numarasından kaynak katın proje öneki (ADR 0690
  `spaceNumberPrefixes`) çıkarılır, hedefin `numbering`'i konur; numbering yoksa hedef katın öneki kaynağınkinin yerine
  geçer (`Z-101` → `1-101`); açıklık numarası yalnız hedefin numbering'ini alır (`K5` → `A1-K5`). Bölüm (`unit`) ve evre
  değişmez: modülsüz mahal çoğunlukla ortak alandır.
- **Kalanlar** (`report.left`, çizimin sırasıyla; uyarı `module_repeat_left`): tekrarlanmayan bir örneğin nesnesi
  (`module`: kopuk, süzgeç dışı ya da tabanı başka kattaki örneğin, örneğin bodrumdan başlayan dubleksin zemin katı),
  bir tasarım seçeneğinin nesnesi (`option`, ADR 0788: düz kopyası ana modele katılırdı), kilitli katmandaki (`locked`),
  kopyalanmayan türden (`kind`: kolon, merdiven, rampa, döşeme, kaplama, ayırıcı) ve kopyaların
  ya da tekrarlanan örneklerin dışına bağlı olan (`outside`; zincirle: dışarıya bağlı mahalin etiketi de kalır).
- **Sıra ve adım.** Her hedefte önce örnekler, sonra kopyalar çizimin sırasıyla; cevabın `copied`'ı (kaynak, kopya, kat).
  Bütün tekrar tek geri alma adımıdır. Kaynak katta ne örnek ne kopyalanacak nesne varsa `module_repeat_empty`.
- **Yinelenen kopya.** Modülsüz kopyalar düz nesnelerdir; aynı tekrar ikinci kez çalışınca kat iki kez dolmasın diye modül
  örneklerinin `module_repeat_taken`'ı korur, ama yalnız modülsüz nesnelerden oluşan kat ikinci kez kopyalanabilir (Revit'in
  Paste Aligned'ı gibi). Pencere yazmadan önce planı söyler.

### 3. `.kcad` şema 152

`arch.modules[].members[].storey` şema 152'den beri yazılır; katlı üyesi olmayan proje eski şemasıyla aynı baytlardır.
Okuyucu 1–15 dışını ve duvar, mahal, donatı dışındaki türde katı reddeder (`bad_value`), önceki şemalarda `unknown_field`.
Örnek `module-storeys.kcad` (iki katlı dubleks), bozuk örnekler `broken/module-storeys-in-schema-151.kcad` (`unknown_field`) ve `broken/module-storeys-on-a-door.kcad`;
aralık dışı örnek `broken/schema-version-153.kcad`. Bağımsız okuyucu `tools/kcad/kcad.py`, yazıcı
`scripts/fixtures/kcad_v2_reference.py`, spesifikasyon `docs/specs/kcad-v2.md`.

### 4. Masaüstü

Oda modülleri penceresi: modül satırında “n katlı”, örnek satırında durduğu katlar (“Bodrum–Zemin kat”); Seçimden yeni
modülün sözü birden çok katı anar; Katları tekrarla'da **Modülsüz nesneler de (duvar, açıklık, mahal, donatı, ölçü, etiket)**
seçimi ve planın sözü (“8 modülsüz nesne düz kopya olur; 10 nesne kalır …”, kalan varsa uyarı rengi). Modül yerleştir'in
Kat ayarı taban katıdır; sığmayan kat komutun reddiyle araçta söylenir. Yeni komut yok: her şey `cad.modules.edit` ile.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/module_storeys_cases.py` (KentOS kodu olmadan; katlar listede kotlarının sırasından
  farklı): `cad.modules.edit.storeys.json` 38 (biri ADR 0788'in seçeneğindeki donatının kalması), `cad.modules.query.storeys.json` 23 durum. ADR 0775'in “iki kattaki duvarlar”
  reddi ve ADR 0779'un “başka kattaki duvar” reddi artık başarılı durumlardır (başvuruları güncellendi); öbür 43 + 22 ve
  52 + 22 durum değişmeden geçer. `KENTOS_CASES_ONLY=cad.modules` ile 200 durum Rust'ta, Python'da da geçer.
- Kabul (`crates/native/headless/tests/headless/storey_modules.rs`): dubleks iki katlı tanımlanır, bodrumda yerleşince üst katı
  zemine gelir, en üst katta reddedilir (yazmaz); zemin modülsüz nesneleriyle 1. kata tekrarlanır (1 örnek, 8 kopya; holün
  kapısı, mahali, etiketi kopyalara bağlı, mahal `1-101`; dubleksin duvarına çekilmiş ölçünün kopyası yeni dubleksin
  duvarında; kolon `kind`, üst kat duvarına bağlı mahal ve etiketi `outside`, bodrum dubleksinin zemin katı `module`); tek
  geri alma hepsini geri alır, yineleme getirir; KCAD 152'a yazılıp açılınca modüller aynı.
- Masaüstü: `room_modules_tests.rs` (dubleks pencereden tanımlanır, “2 katlı” ve örneğin katları okunur; Modülsüz nesneler de
  açılınca plan 8 kopya ve 3 kalan söyler, yazmaz; tekrar 19 nesne yazar, geri alma siler), resimler `module_storey_scenes.rs`
  (iki tema, 1440×900 ve 1100×650): `.run/shots/arac-dubleks-{pencere,tekrar,katlar,yerlestir}-*`.
- KCAD: `kentos-kcad` testleri (234), `scripts/fixtures/kcad_v2_reference.py --check` (1447 dosya), Python `test_carriage`.
- Python SDK `kentos.cad.modules` (yeniden üretildi), MCP aracı katalogdan, manifest; clippy `-D warnings` (contracts,
  commands, kcad, native-application, headless, desktop; headless testlerindeki iki eski uyarı da düzeltildi).

### Süreler

Release, Apple M5, `cargo test --release -p kentos-headless --test headless storey_modules_timing -- --ignored --nocapture`:
on dubleks ve holleri zemin katta, dört kata modülsüz nesnelerle tekrar (40 örnek, 440 modül nesnesi, 320 kopya).

| İş | Süre | Bütçe |
|---|---|---|
| Katları tekrarla'nın planı | 9,1 ms | 500 ms |
| Katları tekrarla'nın yazması | 11,5 ms | 1 000 ms |

## Sınırlar

- Sığmayan katı olan çok katlı modül yerleşmez (Archicad gibi gizlenmez); tek katın yerleşmesi (Single Story) yok: o kat
  ayrı bir modül olarak tanımlanır.
- Kat eşlemesi kat sırasıyladır; kotların farkı (kat yüksekliği) modül nesnesini değiştirmez, duvar yüksekliği tanımdakidir.
- Modülsüz tekrar kolon, merdiven, rampa, döşeme, kaplama ve ayırıcıyı kopyalamaz (`kind`); katsız nesneler (yazı, aks,
  katları karışık ölçü) katın nesnesi sayılmaz, kopyalanmaz ve söylenmez. Aksa bağlı ölçü `outside` kalır.
- Modülsüz kopyaların ikinci kez yerleşmesi engellenmez; yinelenen kopya denetimi yok.
- Tasarım seçeneğinin nesneleri (birincil olanınki de) modülsüz tekrarda kopyalanmaz; seçenek kümesini başka kata
  tekrarlamak açık iştir. Seçenekteki bir modül örneğinin tekrarı ayrıca denenmedi.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.

## Belgelere eklenecek satırlar

### CLAUDE.md §0

- **Çok katlı oda modülü ve kattaki modülsüz nesnelerin tekrarı (31 Ekim 2026; [ADR 0796](docs/adr/0796-multi-storey-modules-and-loose-level-repeat.md), ADV-05 [~]).**
  Birden çok kattaki nesneler tek modül olur: taban en alçak kat, duvar, mahal ve donatının `storey`'i taban üstündeki yeri
  (katlar kotlarına göre, `modules_edit::storey_order`; `.kcad` şema 152, opt-in); `place` ve `repeat` üst katları
  hedefin üstündeki katlara yazar, yetmeyen kat `module_storey_missing`; katlı ve katsız nesne birlikte `module_levels`;
  güncelleme kat değişikliğini taşır, Modüle ekle üst kattaki nesneyi o katın üyesi yapar; `cad.modules.query`'nin `storeys`'i
  ve örneğin `levels`'ı. `repeat`'in `loose`'u: kaynak katın modülsüz duvar, açıklık, mahal, donatı, ölçü ve etiketleri düz
  kopya olarak her hedefe, bağları kopyalara ve tekrarlanan örneklerin nesnelerine çevrilir, mahal numarası hedefin
  numbering'iyle ya da katın önekiyle; kalanlar `report.left` (`module`, `option`, `locked`, `kind`, `outside`) ve `module_repeat_left`,
  cevabın `copied`'ı; tek geri alma adımı. Oda modülleri'nde “n katlı”, örneğin katları ve Modülsüz nesneler de. 38 + 23 ortak
  durum (`module_storeys_cases.py`). Gerçek pencerede elle denenmedi.

### CLAUDE.md §2 (çalıştırma)

```bash
python3 scripts/fixtures/module_storeys_cases.py --check; KENTOS_CASES_ONLY=cad.modules cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-headless --test headless storey_modules; cargo test -p kentos-desktop --lib room_modules   # çok katlı modülün ve modülsüz kat tekrarının 38 + 23 ortak durumu KentOS kodu olmadan (ADR 0775 ve 0779'un durumlarıyla 200), dubleksin kabulü (bodrumda yerleşme, en üst katta ret, holün kopyaları ve bağları, kalanlar, tek geri alma, KCAD 152) ve pencere; resimler KENTOS_SHOTS_ONLY=dubleks-pencere,dubleks-tekrar,dubleks-katlar,dubleks-yerlestir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (.run/shots/arac-dubleks-*); süreler cargo test --release -p kentos-headless --test headless storey_modules_timing -- --ignored --nocapture (ADR 0796)
```

### TODOS.md §0.2 (web'e ertelenenler)

- ADR 0796: `cad.modules.edit`'in çok katlı tanımı (`ModuleMember.storey`), yerleştirme ve tekrarda kat eşlemesi
  (`module_storey_missing`), `repeat`'in `loose`'u (`copied`, `report.left`, `module_repeat_left`), `cad.modules.query`'nin
  `storeys` ve `levels`'ı web'in işleyicilerinde; Oda modülleri penceresinin kat sözleri ve Modülsüz nesneler de seçimi; KCAD
  şema 152'ın okunması ve yazılması (`module-storeys.kcad`).

### plan/architecture-advanced.md ADV-05 (Durum)

_Durum (31 Ekim 2026, [ADR 0775](../docs/adr/0775-room-modules-and-controlled-reuse.md), [ADR 0779](../docs/adr/0779-room-modules-members-drift-and-flats.md), ADR 0788, [ADR 0796](../docs/adr/0796-multi-storey-modules-and-loose-level-repeat.md)):_
tek kat, düz duvarlı oda, bütün daire ve çok katlı modül (dubleks, çekirdek; katlar kotlarına göre eşlenir, sığmayan kat
reddedilir); tanım ve örnek ayrı, yerleştirme (taban, açı, ayna, kat, evre, numara, bina programının bölümü), planlı kaynak
güncelleme, yerel istisna, bağımsızlaştırma, kaynağı silme; tanımdan sonra üye ekleme ve çıkarma; elle değişen örnek
nesnesinin işaretlenmesi; katları tek adımda tekrarlama, kattaki modülsüz nesnelerle birlikte (bağları kopyalarına çevrilir,
kalanlar nedeniyle söylenir); tasarım seçenekleri. Dört banyo, iki katta dört daire ve dubleks kabulü geçti. Açık: tek katın
yerleşmesi, modülsüz kopyaların yinelenme denetimi, gerçek pencerede elle kabul.
