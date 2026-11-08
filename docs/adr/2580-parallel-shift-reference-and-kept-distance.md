# 2580 — Paralel kaydır'da referans ve Bu uzaklığı koru: kenarın kalıcı uzaklığı (harita bağı)

Tarih: 2026-11-08. plan/map-cad.md MAP-06, dördüncü tur. Durum: taslak. Plan satırı: “‘Şimdi bu değere getir’ ile ‘bu
ilişkiyi koru’ farklı olsun. Kalıcı ilişki kullanıcı kararıyla kurulsun; serbest CAD nesnesi fark etmeden mühendislik
modelinin bağlı öğesine dönüşmesin.” ve “Çelişkide hangi ilişkinin neden çözülemediği sahnede bulunsun; otomatik tolerans
büyütme veya bağ koparma yapılmasın.”

İlgili:
- [ADR 0191](0191-parallel-shift.md), [1024](1024-parallel-shift-on-arcs.md), [1960](1960-parallel-shift-shared-boundary.md):
  Paralel kaydır, yaylı kenar, ortak sınır;
- [ADR 0334](0334-constraint-model-and-solver.md)–[0337](0337-constraints-in-edits-copies-and-jobs.md): kısıt modeli,
  düzenlemede izleme, gevşetme;
- [ADR 0770](0770-linked-rooms-and-architectural-bonds.md): mimari bağlar (“Bu ölçüyü koru”, kaymaların doğrusal
  çözümü), [0780](0780-bonds-in-the-way.md)/[0790](0790-bonds-while-dragging-and-together.md): yoldaki bağ ve onu kaldırma yolu.

## Dayanak

- **Civil 3D 2026, “About Creating Feature Lines From Alignments”**
  ([belge](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-UserGuide/files/GUID-5CB0D8C6-1F2B-43C2-B06C-7088E5F5D10B.htm),
  8 Kasım 2026'da okundu): güzergâha bağlı oluşturulan özellik çizgisi “updates automatically when you make edits to the
  alignment”; bağlıyken “you cannot edit it directly”; bağsız çizgi “does not maintain its relationship with the
  alignment”. Bağ kullanıcının seçimidir (oluşturma penceresindeki dinamik bağlantı kutusu).
- **OpenRoads Designer** (Bentley yardımı, 8 Kasım 2026'da okundu): “Single Offset Entire Element” bir öğeyi “at a
  specified distance from the original” çoğaltır ve öteleme kuralı uygular, kural “Remove Offset Rule” ile kaldırılır;
  “Remove Rules” sonrası öğeler “will not update”. Kalıcı ilişki bir kural kaydıdır, kaldırılınca öğe serbest kalır.
- **Civil 3D'nin özellik çizgisinden ötelemesi** ise (aynı yardımın “To Offset a Feature Line by Distance and
  Elevation” sayfası) bir bağ anlatmaz: öteleme tek seferliktir. Üreticiler “şimdi bu değere getir” ile “bu ilişkiyi
  koru”yu ayrı tutar.
- **KentOS bugün:**
  - Paralel kaydır kenarı yalnız kendi normali boyunca bir uzaklıkla ya da hedef alanla kaydırıyordu; başka bir kenara
    uzaklık verilemiyor, ilişki kaydedilemiyordu;
  - genel kısıt çözücüsü (ADR 0334) köşeleri serbest oynatır: kenar bir yola bağlansa yan kenarlar döner, parselin yan
    sınırları doğrultusunu yitirirdi. Paralel kaydır'ın kuralı (komşular kendi doğrularında uzar ya da kısalır) mimari
    bağların kaydırma kuralının (ADR 0770 §4) aynısıdır: bilinmeyen, kenarın kendine dik kaymasıdır.

## Karar

1. **Sınır kenarı** (`CadElement::Boundary { part, ring, index }`): bir alanın ya da tek parçalı çoklu çizginin yalnız
   kendine paralel kayan düz kenarı. Yalnız **harita bağı** alır: `distance` (hizalı), `a` bir düz kenar (`segment`:
   çizgi, yolun ya da alanın düz kenarı) ya da başka bir sınır kenarı, `b` sınır kenarı; `left` çizimden okunur, değer
   projenin parametresidir (`d1`, `d2` …), `reference: true` yalnız ölçer. Başka kural, ters sıra, duvar çizgisi ve
   pervazla karışım sözleşmede reddedilir.
2. **Çözüm** (ADR 0770 §4'ün bağdaştırıcısı, tek kayıt, tek çözüm): her sınır kenarı bir bilinmeyendir (sol normali
   boyunca kayma); `segment` referans tutulur, ancak aynı kenar başka bir bağın sınır kenarıysa o kenarın kaymasıdır
   (zincir). Satırlar ADR 0770'inkiler: `ε·Δ_b − Δ_a = σd − g₀`. Tutulanlar: yazmanın döndürdüğü ya da dik yönde
   oynattığı kenar (kendi doğrusunda kayan oynamış sayılmaz), kilitli katmandaki nesne, referans. Kaymalar Paralel
   kaydır'ın kuralıyla (`ops::edge_shift::shifted`) yazılır; komşular kendi doğrularında uzar ya da kısalır. Sonra bütün
   bağlar yeniden ölçülür (1e-6 m).
   - Bağlı kenar doğrudan oynatılamaz (Civil 3D'nin bağlı çizgisi gibi): yazma reddedilir, gevşeterek düzenleme bağı
     kaldırır.
   - Referans dönerse kenar döndürülmez: “Bağlı kenar artık referansına paralel değil …” (`constraint_conflict`).
   - Kilitli katmandaki bağlı kenar, iki bağın çelişmesi: “Bağlar birlikte tutmuyor”. Ret yoldaki bağları ve kaldırılınca
     yazmayı geçiren bağı söyler (ADR 0780, 0790); hiçbir şey yazılmaz, bağ koparılmaz.
3. **Komut** (`cad.shapes.edit` › `edgeShift`, yeni `reference: {id, at, keep}`):
   - `distance` referansın doğrusuna uzaklıktır, kenar bulunduğu yanda kalır; `area` referansla verilmez;
   - kenar düz olmalı (yayın doğruya uzaklığı tek sayı değil), referans düz bir kenar olmalı ve kenarın kendisi olmamalı,
     ikisi 0,001° içinde paralel olmalı (Kaydır'ın eşiği); referansın doğrusu üstündeki kenar bir yana çekilemez;
   - `keep` (**Bu uzaklığı koru**): kaydırmadan sonra aynı adımda önce var olan bağlar izler (zincirin sonraki kenarı
     kayar; başka yere bağlı kenar reddeder), sonra bağ ve parametresi yazılır; parametre verilen değerdir (`5`), uzaklık
     verilmezse şimdiki uzaklık (yalnız bağ). Cevapta `bonds`. Kilitli katmandaki kenara yalnız bağ kurulmaz.
4. **`.kcad` şema 208:** sınır kenarı anan kısıt; 207 ve öncesi `bad_value`.
5. **Masaüstü** (Paralel kaydır aracı):
   - kenar seçilince **Referans (R)**: tıklanan nesnenin en yakın düz kenarı; paralel değilse açıyla söylenir;
   - referans uzun kesikli çizilir, imleçten referansa dik bir kılavuz çıkar; etiket “referanstan 5.000 m”, alanın yeni
     boyu; referansın öbür yanında tehlike renginde “referansın öbür yanı”, tıklama yazmaz;
   - **Koru (K)** açılınca etiket “uzaklık korunacak” der; yazılan uzaklık bağ olur; **Enter** şimdiki uzaklığı korur;
   - yazınca “uzaklık d1 olarak korunuyor, referans kayınca kenar izler”; bağın işareti (Kısıtları göster) kenarın
     ortasında `d1 = 5 m`; Kısıtlar paneli onu Uzaklık olarak listeler, oradan kaldırılır;
   - referans sonraki kenara paralel kaldıkça durur; kısayol listesine “A / R / K”.
6. **Kapsam işareti:** ADR 1960'taki gibi **CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi**; komut Python ve
   MCP'de.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_bond_cases.py` → `fixtures/commands/v1/cad.shapes.edit.bond.json`, 22 durum,
  KentOS kodu yok. Paralel kaydır ADR 0191'in kuralıyla (komşu doğruların kesişimi), bağın izlemesi uzaklık farkıyla,
  kesirlerle; harita koordinatında (E ≈ 487 200, N ≈ 4 420 000), mikrometre içinde.
  - Bu uzaklığı koru (5 m, d1 = 5), tek adımda geri alma; tek seferlik referans (8 m, bağ yok, yol kayınca parsel kalır);
    şimdiki uzaklıkla yalnız bağ (d1 = 6);
  - yol 2 m kayınca kenar izler, geri alma ikisini birlikte alır; yol kendi doğrultusunda kayınca bir şey olmaz;
  - yol dönünce, kilitli katmandaki bağlı parsel, bağlı kenarın doğrudan kaydırılması ve iki referansın çelişmesi
    `constraint_conflict` (yoldaki bağlar ve ölçtükleri); gevşetmeyle bağ ve parametresi kalkar;
  - parametre 7 olunca kenar 7 m'ye kayar; bağ kaldırılınca yol yalnız kayar;
  - zincir: çekme alanının kenarı parselin kenarına bağlı, ikisi birlikte izler; çoklu çizginin (bordür) düz kenarı,
    komşu kenarlar uzar;
  - dokuz ret: paralel olmayan referans (açısıyla), daire, yaylı kenar, kenarın kendisi, hedef alan, ne uzaklık ne Koru,
    eksi uzaklık, referansın doğrusu üstündeki kenar, kilitli katmanda yalnız bağ.
  - Rust ve Python geçer. Mimari bağ, kısıt, duvar, parametre ve dönüşüm durumları değişmedi.
- **`.kcad`:** `map-bond.json/.kcad` (şema 208), üç bozuk dosya (şema 207'de, paralel kuralında, ters sırada), “daha
  yeni” dosya `schema-version-209`; bağımsız okuyucu ve başvuru yazıcısıyla 1697 dosya tutarlı.
- **MCP** (`crates/native/mcp/tests/map_bond.rs`): örnek açılır, yol kayınca kenar izler, kaydedilen dosyada bağ kalır;
  dönen yol reddedilir; arka kenar ön kenara 25 m'de bağlanır (d2 = 25), yol yeniden kayınca zincir izler.
- **Masaüstü** (`edge_shift_bond_scenes.rs`): Referans ve Koru bağı yazar (d1 = 5), yol kayınca kenar izler, tek geri alma
  ikisini alır, işaret çizilir; paralel olmayan referans açısıyla söylenir, Enter şimdiki 6 m'yi korur, referansın öbür
  yanına tıklama yazmaz. Çapraz denetimler geçer. Resimler `.run/shots/arac-paralel-kaydir-{referans,koru,koru-yazildi,
  bag-izler}-*`.

## Sonra

- Dönen referansı izleyen bağ (kenarın açısı da), yaylı kenar ve yaylı referans bağı (eş merkezli öteleme).
- Ortak sınırlı izleme: bağın kaydırdığı kenarın komşu parseli de götürmesi (bugün komşu kendi bağıyla izler).
- Bağın sahnede kaldırılması (işaretin üstünde), bağlı kenarın ayrı rengi.
- Çok parçalı nesne; seçilen köşelerin sahnede işareti (MAP-06'nın öbür maddeleri).
- Web (TODOS §0.2).
