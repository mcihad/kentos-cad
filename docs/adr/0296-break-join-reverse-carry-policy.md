# ADR 0296: Kır, Birleştir ve Yönü çevir'de nesnenin taşıdıkları ve ilişkileri; `cad.entities.reverse`

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-10`: “Break at point / iki nokta, join toleransı,
  polyline birleştirme ve direction reverse işlemleri metadata ve ilişki politikasını korusun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0142 (köşe kotu), 0175 (nesneye bağlı yazı), 0186 §6 (ilişkili tarama), 0224 (köşede ölçü, M), 0249 ve 0263
  (ilişkili ölçü, `along`), 0254 §3–§4 (anlamsal Kır ve Birleştir), 0258 (genişlikli çoklu çizgi), 0260 (nesnenin görünüşü), 0278
  (kilitli katman).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.break` ve `.join` (ADR 0254) geometriyi çekirdekten alıyordu, ama taşınanlar komuttan komuta değişiyordu: Kır'ın ikinci
parçası öznitelik, etiket ve eki almıyordu; iki komut da nesnenin kendi çizgi tipini, ölçeğini, saydamlığını, sırasını ve sembolünü
yeni nesnede bırakıyordu (`inherited`'in yalnız `keep_data`'sı vardı). İlişkili ölçü yalnız aynı kimliği izliyordu: Kır'da ucu yeni
parçada kalan ölçü “köşe yok” diye kopuyor, Birleştir'de silinen nesneye bağlı uç `object_gone` oluyordu. Birleştir aynı doğrudaki iki
çizgiyi iki köşeli çoklu çizgi, aynı dairenin yaylarını bükümlü çoklu çizgi yapıyordu; birleşmeyen nesneyi ve ayrışan özniteliği
söylemiyordu. Yönü çevir'in komutu yoktu: araç geometriyi kendisi yazıyordu (`geometry` düzeyi).

AutoCAD'le karşılaştırma:

| AutoCAD | Önce | Şimdi |
|---|---|---|
| BREAK, iki nokta (çizgi, yay, daire, elips, çoklu çizgi, spline) | var; spline yok | aynı; NURBS de kırılır, Catmull–Rom eğrisi kırılmaz (`not_breakable`) |
| BREAKATPOINT (tek nokta) | var; daire tek noktada kırılmaz | aynı (`break_failed`, AutoCAD de dairede reddeder) |
| BREAK'te parçalar kaynağın özelliklerini alır | ilk parça; ikinci yalnız görünüşünü | her parça her şeyi (aşağıda tablo) |
| JOIN: aynı doğrudaki çizgiler tek çizgi | iki köşeli çoklu çizgi | tek çizgi (uç uca ya da toleranstaki aralıkla) |
| JOIN: aynı çemberin yayları tek yay, kapanınca daire | bükümlü çoklu çizgi | yay ya da daire |
| JOIN: çizgi, yay, çoklu çizgi karışık | çoklu çizgi | aynı |
| JOIN: kaynak nesnenin özellikleri kalır | ilkinin, görünüşsüz | ilkinin, bütün taşıdıklarıyla; ayrışan öznitelik ve etiket `data_conflict` |
| JOIN: “n nesne birleşmedi” | yok | `not_joined`, nesne başına nedeniyle |
| JOIN: kapanan zincir kapalı çoklu çizgi | kapalı alan | aynı (aşağıda “Kalanlar”) |
| REVERSE (çizgi, çoklu çizgi, spline, helis) | araçta, geometri düzeyinde | `cad.entities.reverse`; araç komutla yazar |
| REVERSE'te genişlikler ve köşe verisi | araçta | kotlar (`zs`), ölçüler (`ms`) ve genişlikler (`widths`) tersine |
| REVERSE: yay, daire, elips | atlanır | `not_reversible` uyarısıyla atlanır (hep saat tersi) |

## Karar

### 1. Taşınanlar tablosu

| Taşınan | Kır | Birleştir | Yönü çevir |
|---|---|---|---|
| Kimlik (`uid`) | ilk parça kaynağınkini tutar; öbür parçalar yeni kimlik | ilk birleşen nesneninki; öbürleri silinir | aynı |
| Katman, renk, kalınlık | her parça | ilkinin | aynı |
| Çizgi tipi ve ölçeği, saydamlık, çizim sırası (`look`) | her parça | ilkinin | aynı |
| Sembol | her parça | ilkinin | aynı |
| Öznitelikler, etiket, ekler (`attachments`) | her parça | ilkinin; ayrışan anahtar ve etiket `data_conflict` uyarısında “ilk / öbürü” | aynı |
| Köşe kotu (`za`, `zb`, `zs`) | yerinden: köşedeki nokta köşeninkini, kenardaki nokta uzunlukla doğrusal (ADR 0142) | her köşe kendi kaynağından | ters sırayla |
| Ölçü M (`ma`, `mb`, `ms`) | kotun kuralıyla (ADR 0224) | kotun kuralıyla; varılmayan köşe boş (`null`) | ters sırayla |
| Genişlik (`widths`) | kenar başına, kesilen kenarda uzunlukla doğrusal (ADR 0258) | kenarlar kaynaklarından; ters giden kenarın başı ve sonu yer değiştirir | kenarlar ters sırayla, her kenarın başı ve sonu yer değiştirir |
| Yay kenarının bükümü | kalan yay parçaları bükümleriyle | yaylar kenar olarak | işaret değişir |
| İlişkili ölçünün noktası (ADR 0249, 0263) | noktasının üstünde durduğu parçaya yeniden bağlanır; silinen kısımdaysa `vertex_gone` ile kopar, noktası yerinde | birleşen nesneye, silinen nesnedeki uç da; noktası yerinde | aynı nesnede yeni yerine (başlangıç ve son yer değiştirir) |
| Bağlı yazı (`labelOf`, ADR 0175) | ilk parçayı izler (kimlik onun) | ilkinin yazısı kalır ve izler; silinen nesnenin yazısı onunla gider (o etiket artık yoktur, ayrışması `data_conflict`'te) | izler |
| İlişkili tarama (ADR 0186 §6) | kapalı nesnesi kırılınca açılır: tarama yerinde kalır, hiçbir şeyi izlemez | silinen ada listeden çıkar; silinen kapalı nesne taramayı bırakır | kapalı nesne kapalı kalır: tarama izler, bölgesi aynı |

“Her parça her şeyi” AutoCAD'in BREAK'idir: parçalar aynı nesnenin parçalarıdır, verisini ve görünüşünü taşırlar. Birleştir'de yalnız ilk
nesnenin verisi kalabilir (tek nesne olur); kaybolan değerler sessizce gitmez, uyarıda adlarıyla söylenir.

### 2. Komutlar

- `cad.entities.break`: girdisi aynı (`uid`, `first`, `second`). Her parça `EntityEdit::add`'in `keepData`'sıyla; kırılabilen türlere NURBS
  eklendi. Çoklu çizgi ve alanın iki köşeli parçası çoklu çizgi kalır (genişliği ve köşe verisi kaybolmasın); yalnız çizginin parçası çizgidir.
- `cad.entities.join`: girdisi aynı (`uids`, `tolerance`, `chain`). Çekirdeğin `join_entities`'inden çıkan şekil `ops::join::simplest` ile
  sadeleşir: bütün üyeler çizgi, açık ve aynı doğruda tek yöne gidiyorsa çizgi; bütün üyeler aynı çemberin yayıysa ve bükümler tek
  işaretliyse yay ya da (kapanınca) daire; öbür durumda değişmez. `join_entities` web'in kayıtlı çağrıları
  (`fixtures/geometry/v1/calls-p7-editing.json`) için olduğu gibi kalır. Yeni uyarılar:
  - `not_joined`: zincirde olmayan nesne (kilitli değil, Zincir kipi değil): “Ucu başka bir nesnenin ucuna toleransta değmiyor;
    birleşmedi.” ya da türü birleşmiyorsa “Yalnız çizgi, yay ve açık çoklu çizgi birleşir; bu nesne birleşmedi.”, yolu `uids[i]`;
  - `data_conflict`: “Birleşen nesnelerin öznitelikleri ayrışıyor: anahtar (ilk / öbürü), …; ilk nesnenin değerleri kaldı.”, yolu `uids`.
- `cad.entities.reverse` (yeni, sürüm 1): girdi `uids` ve `expectedRevision`; çıktı ve plan `cad.entities.edit`'inki. Denetim sırası:
  kimlikler, sürüm, adlar, kilitli katman (`skip_locked`: hepsi kilitliyse `layer_locked`, bir kısmıysa sayılarak atlanır). Her nesne
  çekirdeğin `ops::reshape::reverse`'iyle döner; yönsüz nesne (yay, daire, elips, nokta, yazı, ölçü …) `not_reversible` uyarısıyla kalır,
  dönen hiçbir şey yoksa komut `not_reversible` ile reddedilir. Hedef türler: çizgi, çoklu çizgi, alan, eğri, NURBS. Kapalı halka bütün
  noktalarıyla ters sırayla yazılır. Adım “Yönü çevir”, tek geri
  alma adımı.
- Yönü çevir aracı (`kentos_interaction::reshape`) önizlemeyi aynı çekirdekle, yazmayı `entities_reverse::execute` ile yapar; kayıtta komutun
  adı görünür. Manifestte `reverse` `semantic`, `via` `cad.entities.reverse`.

### 3. İlişkilerin izlenmesi

Belgenin adım sonu izlemesi (`follow_into`: bağlı yazılar, ilişkili taramalar, ilişkili ölçüler) kimlik üzerinden çalışır; Kır ve Birleştir
bir kimliğin yerine başka kimlikler koyar. Komut bunu belgeye söyler: `Document::relocate_ties(of, onto)` adımın sonuna kadar “`of`'a bağlı
olanlar artık `onto`'dakilerdendir” kaydı tutar (adım geri sarılırsa silinir). Ölçünün izlemesi (`dimension_ties::follow_dimensions`) böyle bir
nesneye bağlı noktayı, `onto`'nun ilk uyan nesnesinde `ops::subref::place_near` ile 1e-6 içinde yeniden bulur: bulursa kimliği ve yeri
değişir (noktanın koordinatı değişmez), bulamazsa `vertex_gone` ile kopar. Kır'da `onto` kaynağın kendisi ve yeni parçalar; Birleştir'de
tutulan da silinenler de birleşen nesneye; Yönü çevir'de nesnenin kendisi (yer köşe sırasıyla değiştiği için yeniden bulunur).

Bağlı yazı ve ilişkili tarama mevcut kurallarıyla izler (ADR 0175 §4, 0186 §6): onlar nesneye bağlıdır, yere değil; tablo yukarıda.
`crates/native/application/tests/all/break_join_ties.rs` bunları kalıcı kimlikli bir çizimde sınar (ortak durumların çizimi kimlik taşımaz).

### 4. Ortak durumlar ve bağımsız başvuru

- `scripts/fixtures/break_join_policy_cases.py`, KentOS kodu olmadan: zengin verili bir çizim (katman, renk, kalınlık, çizgi tipi ve
  ölçeği, saydamlık, sembol, öznitelik, etiket, ek, kot, M, genişlik, yay kenarları, kapalı çoklu çizgi, kilitli katman, ilişkili ölçüler);
  parçaların geometrisi kesin, kotlar, M ve genişlikler yerden, ölçülerin yeni yerleri kendi `place_on`'uyla.
  `fixtures/commands/v1/cad.entities.break.policy.json` (21), `cad.entities.join.policy.json` (20), `cad.entities.reverse.json` (21).
- Ölçüler çizimde bağsızdır; durum önce `cad.dimensions.reassociate` ile bağlar. Bunun için durum adımına `command` anahtarı eklendi
  (adım başka bir komutu çalıştırır; Rust ve Python çalıştırıcıları): v1 çiziminde kimlik içerikten türediği için kurulumda kimlik
  yazılamaz.
- `reshape_command_cases.py` ve `edit_command_cases.py` yeni kuralla: Kır'ın bütün parçaları veriyle, `whole` işlemlerde sembol ve
  görünüş; Birleştir'in aynı doğrudaki çizgileri çizgi, ayrışan öznitelik uyarısı. `cad.entities.break.json` (20), `.join.json` (23),
  `.divide.json` yeniden yazıldı.

## Kalanlar

- Aralıklı aynı doğrudaki çizgiler (toleranstan uzak) birleşmez; AutoCAD birleştirir. KentOS'ta tolerans açıkça verilir.
- Kapanan zincir kapalı alan olur (ADR 0254 §4), kapalı çoklu çizgi değil; kapalı çoklu çizgi ayrı bir seçenek ister.
- Catmull–Rom eğrisi kırılmaz (NURBS kırılır); NURBS'in Kır'ı ortak durumlarda yok, çekirdeğin `ops::nurbs_cuts` testlerinde.
- Buda (`cad.entities.trim`) parça üretir ama `relocate_ties` kullanmaz: budanan nesnenin ikinci parçasındaki ölçü noktası kopar.
- Birleştir silinen nesnenin bağlı yazısını siler; o yazıyı tutulan nesneye taşımak iki etiket demektir, sahibin kararı gerekirse ayrı iş.
