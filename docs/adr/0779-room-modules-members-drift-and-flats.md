# ADR 0779: Oda modüllerinde üye ekleme ve çıkarma, elle değişen nesne, daire modülü ve kat tekrarı

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md `ADV-05`'in ikinci dilimi (ADV-05b); ilk dilim [ADR 0775](0775-room-modules-and-controlled-reuse.md).
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler aşağıda, “Belgelere eklenecek satırlar”da.
- **`.kcad`:** şema 144 (opt-in; geçici numara: 143 başka bir dalın, birleşimde numaralar yeniden verilir). `FORMATS_VERSION`
  değişmez (102): yeni alan proje ayarlarının JSON'undadır, tipli sütunlara girmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0775 modülü tanımla, yerleştir, kaynağı güncelle, yerel istisna, bağımsızlaştır ve kaynağı sil'le bitirdi; dört şeyi
açık bıraktı:

1. Tanımdan sonra üye eklemek ya da çıkarmak yoktu: yeni bir kapı ya da donatı için modül baştan tanımlanıyordu.
2. Komutun dışında elle değiştirilmiş örnek nesnesi işaretlenmiyordu; kaynak güncellenince sessizce kaynağın değeriyle
   yazılıyordu.
3. Bütün bir daire (birden çok mahal, duvar, açıklık, donatı, ölçü) modül olabiliyordu ama yeniden yerleşen daire kendi
   bağımsız bölümünü (ADR 0773'ün bina programı) alamıyordu: dört dairenin mahalleri aynı bölümde sayılıyordu.
4. Bir katı öbür katlara tekrarlamak örnek örnek yerleştirmekti.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Autodesk Revit, “Add or Remove Elements in a Group”.** Grup düzenleyicide (Edit Group) Add ve Remove ile grubun
  içeriği değişir; ekleme ve çıkarma grubun bütün örneklerini etkiler. Tek bir örnekte bir öğeyi kaldırmak ayrı bir
  eylemdir: Exclude (ADR 0775'in yerel istisnası).
  <https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-69AACF6E-4130-4499-8160-25DE412D2C97.htm>
- **Autodesk Revit, “About Modifying Groups”.** Grubun üyeleri grup düzenleyicide değişir; tek bir öğeyi düzenlemek için
  grup çözülür; bir öğe tek örnekten dışarıda bırakılabilir ya da örnekten projeye taşınabilir; grubun Reference Level'ı
  ve Origin Level Offset'i vardır.
  <https://help.autodesk.com/cloudhelp/2025/ENU/Revit-Model/files/GUID-B59163B4-E1DF-4C10-9DB9-F64B2861825E.htm>
- **Autodesk Revit, “Paste Aligned Elements” (Aligned to Selected Levels).** Bir katta kopyalanan öğeler seçilen katlara
  aynı yatay yerde yapıştırılır; çok katlı binalar için.
  <https://help.autodesk.com/cloudhelp/2026/ENU/Revit-Model/files/GUID-4E8562EC-38C4-485E-A86D-D459096187AF.htm>
- **Graphisoft Archicad, “Place Hotlink”.** Çok katlı modül tek katıyla ya da bütün katlarıyla (All Stories) yerleşir;
  Match Stories modülün katını ev sahibinin katına eşler; ev sahibinde yeterli kat yoksa modülün fazla katları
  görünmez. Yerleşimde Master Layer, Orientation ve Master ID verilir.
  <https://help.graphisoft.com/START/27/INT/_AC27_Help/080_Collaboration/080_Collaboration-58.htm>
- Revit'te grup dışında grubun bir öğesini değiştirmek uyarı verir; birden çok örnek varsa değişiklik engellenir
  (Autodesk forumu; resmî belge değil, yalnız bağlam olarak:
  <https://forums.autodesk.com/t5/revit-architecture-forum/a-group-has-been-changed-outside-edit-mode/m-p/7091851>).
  Archicad'de modülün öğeleri ev sahibinde kilitlidir; düzenlemek için kaynak açılır ya da bağ kopar (ADR 0775).

KentOS ikisinden ayrılır: örnek nesneleri sıradan nesnelerdir ve her araçla düzenlenebilir (kilitleme yok); bunun
karşılığı, elle değişen nesneyi görünür kılmak ve kaynak güncellemesinde açıkça söylemektir (§3). Üye ekleme Revit'in grup
düzenleyicisi gibi bütün bağlı örneklere gider, ama ayrı bir düzenleme kipi açılmaz: örnekte çizilen nesneler seçilip
“Örneğe ekle” denir. Kat tekrarı Revit'in Paste Aligned'ı gibi aynı yatay yerdedir, ama sonuç kopya değil bağlı
örneklerdir; Archicad'in Match Stories'i gibi hedef kat açıkça seçilir.

## Karar

### 1. Modüle ekle (`addMembers`)

`cad.modules.edit`'in `addMembers`'ı (`module`, `instance`, `uids`): örnekte çizilmiş, hiçbir modülde olmayan nesneler
tanıma katılır. Nesne örneğin ters matrisiyle tanımın çerçevesine alınır, başvuruları (kapının duvarı, etiketin mahali,
ölçünün noktaları) örneğin anahtarlarına çevrilir; katı, örneğin numara öneki ve soneki ve örneğin bölümü (§4) bırakılır;
anahtarı nesnenin kimliğidir. Tanımın sürümü bir artar; öbür bağlı örneklere yeni nesneler kendi matrisleriyle (aynalı
örnekte aynalı, kapı kendi duvarında, numara kendi önekiyle) yerleşir, duvarlar önce. Kilitli katmanda nesnesi olan örnek
atlanır (`module_instance_skipped`) ve eski sürümde kalır.

Retler ADR 0775'in tanımla kurallarıdır: tür (`module_kind`), başka modülün nesnesi (`module_member_taken`), örneğin ve
eklenenlerin dışına bağ (`module_outside`), örneğin katında olmayan (`module_levels`), kilitli katman (`layer_locked`).

**Geride kalan örnek tamamlanır.** Bir kaynak güncellemesinde (ADR 0775 §3) modülün sürümünden eski ya da tanımın bir
anahtarı olmayan hedef örnek bütünüyle eşitlenir: değişen anahtarlar yerine tanımın bütün anahtarları yazılır, eksik
anahtarlar yeni nesne olarak eklenir (`report.addedKeys`). Kaynak örnek tanımla aynı olsa da geride kalan varsa güncelleme
yapılır (sürüm artmaz); hiçbir şey yoksa `nothing_to_set`. Böylece kilit yüzünden atlanan örnek sonraki güncellemede yetişir.

### 2. Modülden çıkar (`removeMembers`)

`removeMembers`'ın (`module`, `uids`: bir örneğin nesneleri) anahtarları tanımdan çıkar. Verilen nesneler çizimde sıradan
nesne olarak kalır (`released`); öbür örneklerdeki karşılıkları silinir (`deleted`), ancak yerel istisna olan ya da
kilitli katmandaki karşılık silinmez, sıradan nesne olarak kalır ve söylenir (`conflicts`: `exception`, `locked`; uyarı
`module_member_released`). Kalan bir üye çıkarılana bağlıysa (kapı çıkarılan duvarda, etiket çıkarılan mahalde)
`module_member_needed` ile reddedilir: modül dışarıya bağ taşımaz (ADR 0775 §1). Bütün üyeleri çıkarmak `invalid_input`
(modül silmenin yolu Kaynağı sil'dir). Sürüm bir artar; tek geri alma adımı silinenleri de geri getirir.

Revit'in Remove'u gibi bütün örneklere gider; Revit'in tek örneklik Exclude'u ADR 0775'in yerel istisnasıdır.

### 3. Elle değişen nesne (drift)

Bir örnek nesnesi, tanımın üyesi örneğin matrisiyle (duvarları bugünkü hâliyle, açıklamanın görünüşteki yeri örneğin)
yerleştirilince ortaya çıkan nesneden farklıysa “elle değişmiş”tir (`modules_edit::drifted_keys`). Karşılaştırma
belgenin kendi yazdıklarını saymaz (açıklığın pervazları `place`, kaynaklı mahalin bulunan halkaları `parts`, etiketin
yazısı ve bağ noktası, kopukluk nedeni) ve sayıları 1e-6 çizim birimi içinde aynı sayar (matrisin yuvarlaması, elle
değişiklik değil; dönmüş örnekte de). Yerel istisna, silinmiş nesne, eski sürümdeki örnek ve kaynağı silinmiş modül
elle değişmiş sayılmaz. Değer saklanmaz, her okumada hesaplanır: şema değişmez ve belgenin bütün yazma yolları (araçlar,
tutamaçlar, Python, ajan) kendiliğinden kapsanır.

- `cad.modules.query` her örneğin `drifted`'ını (nesne kimlikleri), `uid` ile sorulan nesnenin `drifted`'ını verir.
- Kaynak güncellemesi elle değişmiş nesneyi kaynağın değeriyle yazar ve planında ve cevabında söyler (`conflicts`'te
  `drifted`, uyarı `module_drift_overwritten`). Korumanın yolu önce Yerel istisna yap'tır; geri almanın yolu Kaynağa
  döndür'dür (ADR 0775 §3, elle değişmiş nesnede de çalışır).
- Kaynağı güncellerken “değişen anahtar” da aynı toleransla karar verilir: başka yerdeki ya da dönmüş örnekten güncellemek
  matrisin yuvarlamasıyla her üyeyi değişmiş saymaz.

Öznitelikler'in Modül bölümünde “Kaynakla” satırı (Kaynakla aynı / Elle değişmiş (istisna değil) / Kendi
değerinde / Eski sürümde / Bağ kopuk), Oda modülleri penceresinde örneğin satırında “n nesne elle değişmiş”.

### 4. Daire modülü: örneğin bölümü

Tek kattaki birden çok mahal, duvar, açıklık, donatı, ölçü, etiket ADR 0775'in kurallarıyla modül olur (iki odalı daire
kabulde). Yeni olan bölüm: örneğin `unit`'i (ADR 0773'ün bina programındaki bağımsız bölüm). Örneğin mahalleri o
bölümdedir. Tanımlarken modülün mahallerinin hepsi aynı bölümdeyse bölüm tanımdan alınır ve ilk örneğin olur; değilse
mahaller bölümlerini tanımda tutar. `place`'in ve `repeat`'in `unit`'i (programda olmayan `unit_not_found`). Kaynağı
bölümlü örnekten güncellemek bölümü tanıma taşımaz (numara gibi örneğindir). Modül yerleştir'de Bölüm ayarı.

### 5. Katları tekrarla (`repeat`)

`repeat` (`from` kaynak kat, isteğe bağlı `modules` süzgeci, `to`: hedef katlar) kaynak kattaki bağlı her örneği her hedef
kata aynı nokta, dönüş ve aynayla yeni bağlı örnek olarak yerleştirir; kat göreli yerleşimdir (örneğin katı hedef kat,
düzlemdeki yer aynı). Numara ve bölüm sırasıyla: hedefteki örneğe özel satır (`instances`: `module`, `instance`,
`numbering`, `unit`), hedefin kendisi (`numbering`, `unit`), kaynak örneğinki. Tek geri alma adımı; cevabın `placed`'ı
yerleşen örnekler. Retler: hedef yok ya da kaynak kat hedeflerde ya da bir kat iki kez (`invalid_input`), olmayan kat
(`level_not_found`), kaynak katta bağlı örnek yok (`module_repeat_empty`; kaynağı silinmiş modülün kopuk örneği
tekrarlanmaz), hedefte aynı modülün aynı yerde örneği var (`module_repeat_taken`: aynı komut iki kez çalışınca kat iki
kez dolmaz), satırın örneği kaynak katta değil (`module_instance_not_found`), kilitli katman.

Oda modülleri penceresinde Katları tekrarla: kaynak kat, hedef katlar (her birinin numara öneki), planın sözü (kaç örnek,
kaç nesne) ve düğme.

### 6. `.kcad` şema 144

Örneğin `unit`'i şema 144'ten beri yazılır; bölümlü örneği olmayan proje eski şemasıyla aynı baytlardır. Okuyucu boş ya
da 64 karakterden uzun bölümü reddeder (`bad_value`), önceki şemada `unknown_field`. Örnek `module-units.kcad`, bozuk
örnekler `broken/module-units-in-schema-143.kcad` (143 bu dalda okunmadığı için şimdilik `schema_version`; birleşimde
şema numaralanınca `unknown_field` olur) ve `broken/module-units-empty.kcad`; aralık dışı örnek `schema-version-145`.

### 7. Masaüstü

- Oda modülleri: örnek satırında elle değişen sayısı ve bölüm; Seçimi örneğe ekle, Seçimi modülden çıkar; Katları
  tekrarla bölümü; güncellemenin sözünde “elle değişmiş n nesne kaynağa döner”, “n eksik nesne eklenir”.
- Öznitelikler'in Modül bölümü: Kaynakla satırı; işlemlerde Modülden çıkar.
- Komutlar `arch.moduleAddMembers` (Örneğe ekle: seçimin modülsüz nesneleri, seçimdeki örneğe) ve
  `arch.moduleRemoveMembers` (Modülden çıkar), `catalog/v1/additions.json`'da ikonlarıyla; düzeyleri `operations.json`'da
  `semantic`. Modül yerleştir'in Bölüm ayarı.

Her yazma `cad.modules.edit` ile, tek geri alma adımı “Oda modülü”; pencerelerin önizlemesi komutun planıdır.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/module_flats_cases.py` (KentOS kodu olmadan): `cad.modules.edit.flats.json` 52,
  `cad.modules.query.drift.json` 22 durum; ADR 0775'in 43 + 22 durumu değişmeden geçer.
- Kabul (`crates/native/headless/tests/headless/flat_modules.rs`): iki odalı daire tanımlanır, aynalı ikinci daire d2
  bölümünde yerleşir, zemin kat 1. kata tekrarlanır (3-, 4-; d3, d4); sekiz mahal kendi katında, bölümünde ve
  numarasıyla; tek geri alma tekrarı bütünüyle geri alır; eklenen yatak dört dairede (aynalıda aynalı yerde), çıkarılan
  ölçü üç dairede silinir, birinde kalır; elle taşınan yatak elle değişmiş görünür ve güncelleme onu kaynağa döndürür;
  KCAD 144'e yazılıp açılınca modüller ve bölümler aynı.
- Masaüstü: `room_modules_tests.rs` (seçimden Örneğe ekle ve Modülden çıkar, pencereden Katları tekrarla ve geri alma,
  elle taşınan yatağın sayılması ve Kaynağa döndür), resimler `module_flat_scenes.rs` (iki tema, 1440×900 ve 1100×650).
- KCAD: `kentos-kcad` testleri, `scripts/fixtures/kcad_v2_reference.py --check`, `tools/kcad/kcad.py`.
- Python SDK `kentos.cad.modules`, MCP aracı (katalogdan), manifest.

### Süreler

Release, Apple M5, `cargo test --release -p kentos-headless --test headless flat_modules_timing -- --ignored
--nocapture`: on daire zemin katta, dokuz kata tekrar (90 örnek, 1 170 nesne).

| İş | Süre | Bütçe |
|---|---|---|
| Katları tekrarla'nın planı (90 örnek) | 5,8 ms | 500 ms |
| Katları tekrarla'nın yazması (90 örnek, 1 170 nesne) | 17,2 ms | 1 000 ms |
| Elle değişenlerin sorgusu (100 örnek) | 7,7 ms | 500 ms |
| Modüle ekle (100 örneğe bir nesne) | 1,2 ms | 500 ms |

## Sınırlar

- Tasarım seçenekleri (alternatifler) kapsam dışı; evre seçenek kimliği olarak kullanılmaz (ADR 0775).
- Kat tekrarı yalnız modül örneklerini tekrarlar: kattaki modülsüz nesneler (merdiven, aks, döşeme) gelmez. Çok katlı
  tek modül (Archicad'in All Stories'i) yok; her kat ayrı örnektir.
- Elle değişen nesne saklanan bir işaret değil, hesaptır: bir nesnenin elle mi yoksa başka bir bağın (örneğin elle
  taşınan duvarın ölçüsü) izlemesiyle mi değiştiği ayrılmaz; ikisi de kaynaktan farklıdır ve öyle söylenir.
- Tanımdan çıkarılan nesneye modül dışından bağlı nesneler (kullanıcının kendi etiketi) silinen karşılıkla kopuk kalır
  (belgenin kendi izlemesiyle).
- Mimari denetimde (`cad.arch.audit`) ayrı bir bulgu yok: elle değişen yalnız Öznitelikler ve Oda modülleri'nde.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.

## Belgelere eklenecek satırlar

### CLAUDE.md §0

- **Oda modüllerinin ikinci dilimi: üye ekle ve çıkar, elle değişen, daire ve kat tekrarı (31 Ekim 2026; [ADR 0779](docs/adr/0779-room-modules-members-drift-and-flats.md), ADV-05 [~]).**
  `cad.modules.edit`'in `addMembers`'ı (örnekte çizilen nesneler tanıma katılır, öbür bağlı örneklere kendi matrisleriyle
  gelir; kilitli örnek atlanır, sonraki güncellemede eksiklerini alır: geride kalan örnek bütünüyle eşitlenir),
  `removeMembers`'ı (anahtar tanımdan çıkar, verilen nesne kalır, öbür karşılıklar silinir, istisna ya da kilitli olan
  kalır `module_member_released`; kalan üye ona bağlıysa `module_member_needed`), `repeat`'i (kaynak kattaki bağlı
  örnekler hedef katlara aynı yerde, numara ve bölüm hedefin ya da örneğe özel satırın; `module_repeat_empty`,
  `module_repeat_taken`) ve `place`'in ve örneğin `unit`'i (bina programının bölümü, daire modülü; `.kcad` şema 144
  opt-in). Elle değişen nesne hesaplanır (`modules_edit::drifted_keys`: tanım örneğin matrisiyle, belgenin yazdıkları ve
  açıklamanın yeri hariç, 1e-6 içinde); `cad.modules.query`'nin `drifted`'ı, güncellemede `module_drift_overwritten`.
  Oda modülleri'nde Seçimi örneğe ekle, Seçimi modülden çıkar, Katları tekrarla; Öznitelikler'de Kaynakla satırı ve Modülden
  çıkar; `arch.moduleAddMembers`, `arch.moduleRemoveMembers`; Modül yerleştir'de Bölüm. 52 + 22 ortak durum
  (`module_flats_cases.py`). Tasarım seçeneği, çok katlı tek modül açık; gerçek pencerede elle denenmedi.

### CLAUDE.md §2 (çalıştırma)

```bash
python3 scripts/fixtures/module_flats_cases.py --check; KENTOS_CASES_ONLY=cad.modules cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-headless --test headless flat_modules; cargo test -p kentos-desktop --lib room_modules   # daire modüllerinin 52 + 22 ortak durumu KentOS kodu olmadan (ADR 0775'in 43 + 22'siyle), iki odalı dairenin kabulü (bölüm, kat tekrarı, üye ekle ve çıkar, elle değişen, KCAD 144) ve pencere; resimler KENTOS_SHOTS_ONLY=daire-tanim,daire-tekrar,daire-katlar,daire-elle,daire-pencere,daire-yerlestir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (.run/shots/arac-daire-*); süreler cargo test --release -p kentos-headless --test headless flat_modules_timing -- --ignored --nocapture (ADR 0779)
```

### TODOS.md §0.2 (web'e ertelenenler)

- ADR 0779: `cad.modules.edit`'in `addMembers`, `removeMembers`, `repeat` işlemleri ve `place`'in `unit`'i,
  `cad.modules.query`'nin `drifted` ve `unit`'i web'in işleyicilerinde; Oda modülleri penceresinin Seçimi örneğe ekle,
  Seçimi modülden çıkar ve Katları tekrarla bölümü, Öznitelikler'in Kaynakla satırı, Modül yerleştir'in Bölüm ayarı;
  KCAD şema 144'ün okunması ve yazılması (`module-units.kcad`).

### plan/architecture-advanced.md ADV-05 (Durum)

_Durum (31 Ekim 2026, [ADR 0775](../docs/adr/0775-room-modules-and-controlled-reuse.md), [ADR 0779](../docs/adr/0779-room-modules-members-drift-and-flats.md)):_
tek kat, düz duvarlı oda ve bütün daire modülü; tanım ve örnek ayrı, yerleştirme (taban, açı, ayna, kat, evre, numara,
bina programının bölümü), planlı kaynak güncelleme (geride kalan örnek bütünüyle eşitlenir), yerel istisna,
bağımsızlaştırma, kaynağı silme; tanımdan sonra üye ekleme ve çıkarma; elle değişen örnek nesnesinin işaretlenmesi ve
güncellemede söylenmesi; katları tek adımda tekrarlama. Dört banyo ve iki katta dört daire kabulü geçti. Açık: tasarım
seçenekleri, çok katlı tek modül, kattaki modülsüz nesnelerin tekrarı, gerçek pencerede elle kabul.
