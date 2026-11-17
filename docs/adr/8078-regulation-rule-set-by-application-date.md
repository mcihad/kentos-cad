# ADR 8078: Mevzuat denetimi başvuru tarihine göre kural setiyle — asansör (PAİY 34/1, 34/8), bulgu durumları

- **Durum:** ilk dilim yapıldı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (B15), dayanak Z9;
  [architecture-advanced.md](../../plan/architecture-advanced.md) ADV-23'ün kalanı "başvuru tarihi".
- **Bağlam:** ADR [8033](8033-regulation-rules-rooms-storeys-doors.md) ve [8034](8034-regulation-stairs-shafts-elevators-local-rules.md)
  (Mevzuat grubu Rust'ta; `RULE_SET` PAİY'nin yalnız son değişikliğini okuyordu), [8035](8035-fire-escape-routes-distances-load-exits.md)
  (yangın kaçışı, yağmurlama ayarı), [8063](8063-dated-digital-delivery-scope.md) (künyenin `basvuru_tarihi`'si ve
  tarihli kapsam kararı), bim-plan K10 (kural Python betiği, ölçü Rust'ta; kural dokunuldukça Rust'tan taşınır).
- **Sorun:** Planlı Alanlar İmar Yönetmeliği 1.7.2026'da (RG 33297) 34/1'i ve 34/8'i değiştirdi; Geçici 3/11 o günden önce
  başvurusu yapılmış yapılara eski hâli uygular. KentOS her çizimi yeni hâlle okuyordu: 30.6.2026'da başvurmuş bir
  projeye kendi başvurusunda aranmayan asansörü soruyordu. Bulgu da yalnız "kaldı" olabiliyordu: gün bilinmeyince ya da
  metin yoruma kalınca KentOS ya susuyor ya da tahmin ediyordu.

## §1. Araştırma (17 Kasım 2026)

mevzuat.gov.tr ve Resmî Gazete bu bilgisayarın tarayıcısında sertifika hatası veriyor; metinler WebFetch ile Resmî
Gazete'nin eski sayılarından, Lexpera'nın Resmî Gazete metninden ve meslek odasının karşılaştırma tablosundan okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| RG 1.7.2026/33297, 4 üncü madde (Lexpera) | 34/1: tek bağımsız bölümlü konut hariç, bağımsız bölüm, ortak alan (ısı merkezi, kazan dairesi, elektrik odası, su deposu, iklimlendirme merkezi, jeneratör odası ve benzeri teknik hacimler dışında) ya da eklenti bulunan bodrum dahil kat adedi 3 olan binada asansör yeri, 4 ve fazlasında asansör tesisi. 34/8: asansör zorunlu binada asansör bodrumlar dahil bütün katlara hizmet verir; bağımsız bölüm, ortak alan ya da eklenti bulunmayan bodruma ulaşması gerekmez | 34/1 ve 34/8'in 1.7.2026 hâli |
| Aynı yönetmelik, 7 nci madde: Geçici 3/11 | Fıkranın yürürlüğünden önce 55/2'deki belgeler için idareye başvurusu yapılmış yapılara (6306 sayılı Kanun kapsamında noter onaylı sözleşmeli ve kamu ihaleli yapılara da) 5/27 ile 34/1 ve 34/8'in değişik hâli uygulanmaz, önceki hâli uygulanır | Hâli seçen gün başvurunun günüdür (künyenin `basvuru_tarihi`); 1.7.2026'daki başvuru "önce" değildir (K60) |
| RG 11.7.2021/31538, 3 üncü madde (resmigazete.gov.tr) | 34/1'in ikinci cümlesinde "ve ortak alan" ibaresi "veya bağımsız bölüm" oldu; 34/8 yeniden yazıldı: bağımsız bölüm ya da iskân edilen alan bulunmayan bodruma ulaşmak gerekmez | 34/1 ve 34/8'in 11.7.2021–30.6.2026 hâli |
| FTSO karşılaştırma tablosu (15.7.2021) | 34/1'in derlenmiş hâli: "(Değişik:RG-28/7/2018-30492) … uygulama imar planına göre kat adedi 3 olan binalarda asansör yeri …, 4 ve daha fazla olan binalarda ise asansör tesisi zorunludur. İskân edilen (Değişik ibare: RG-11/7/2021-31538) veya bağımsız bölüm bulunan bodrum katlar dâhil kat adedi 4 …" | 28.7.2018 hâlinin ikinci cümlesi 2021'in ibare değişikliğinden geri okundu: "İskân edilen ve ortak alan bulunan bodrum katlar dâhil" |
| Yades'in 14 Ocak + 1 Temmuz 2026 listesi; alomaliye 1.7.2026 değişikliği | 14.1.2026 (RG 33137) değişikliği 4/1-d, 5/8, 23, 57/4 ve 59'a dokundu; 34'e dokunmadı | 11.7.2021 hâli 30.6.2026'ya kadar sürer |
| Sivas İmar Yön. (RG 10.11.2022/32009) 34/1, 34/8 | En çok üç katlı tek bağımsız bölümlü konut hariç, uygulama imar planına göre kat adedi 3 ve fazla binada asansör; iskân edilen ve ortak alan bulunan bodrum sayılır; 34/8 PAİY'nin 2021 hâli gibi, her asansörde. Geçici 3: mevcut başvurularda PAİY'nin geçici 3 üncü maddesi | Yerel katman 10.11.2022'den; sıkı olan bağlar, gevşeğin tuttuğu ölçü çelişki (ADR 8034) |
| bim-plan BIM-13, Z9 | Her bulgu geçti, kaldı, eksik veri, uzman incelemesi ya da kapsam dışı; yorum isteyen konu kendiliğinden "uygun" sayılmaz | Bulgu kaldı, eksik veri ya da uzman incelemesi; geçti ve kapsam dışı maddenin özet satırındadır (K62) |

Okumalar (metin açıkça söylemiyor, kural böyle okuyor; bağımsız durumlar da böyle yazıldı):

- 2018 ve Sivas metnindeki "iskân edilen ve ortak alan bulunan bodrum" ikisinden biri olan bodrumdur (2021 değişikliğinin
  gerekçesi: "önceden ortak alan niteliğindeki alan kullanımı olan katlarla birlikte" sayılıyordu); o metinlerde teknik
  hacim de ortak alandır (2026 metni teknik hacmi açıkça ayırmadan önce ayırmıyordu).
- Grupsuz mahal hiçbir bağımsız bölümün değildir: ortak alan gibi okunur.
- "Uygulama imar planına göre kat adedi" (2018, 2021, Sivas) künyenin `kat_adedi`'dir; yazılmamışsa çiziminki ve iletide
  hangisinin olduğu yazılır. 2026 metni plana atıf yapmaz: çiziminki.
- Kat sayısı ±0,00 ve üstündeki her kattır (ADR 8034'teki gibi projenin katları); bodrum içindekine göre sayılır.
  Asansörün ulaşması mahali olan katlarda aranır.
- 34/8'in 2026 hâli ortak alanı olan bodruma ulaşmayı ister, 34/1 teknik hacmi ortak alandan ayırır: yalnız teknik hacmi
  olan bodrum uzman incelemesidir.

## §2. Karar

- **K60 — Kural seti başvuru gününe göre.** Maddelerin hâlleri veridir (`catalog/standards/mimari-mevzuat/src/mimari_mevzuat/data/surumler.json`):
  metni, Resmî Gazete'si, yürürlüğe girdiği ve yerini bıraktığı gün, geçiş hükmü, kaynağı ve kuralın o hâli okurken
  kullandığı anahtarlar (bodrumu neyin saydırdığı, katın plandan mı sayıldığı, asansörün hangi bodruma ulaşacağı). Gün
  künyenin `basvuru_tarihi`'dir (`GG.AA.YYYY`, `G.A.YYYY`, `GG/AA/YYYY`, `YYYY-AA-GG`).
- **K61 — Gün yoksa tahmin yok.** Gün yoksa ya da okunamıyorsa kural, ulusal maddelerin hepsinin bilindiği ilk günden
  (11.7.2021) başlayarak her hâlin ilk günüyle denenir. Bulguları (kodu, nesneleri, durumu ve ne istediği) her denemede
  aynıysa en yeni hâliyle söylenir; değilse tek bir bulgu verir: `regulation_date_needed`, eksik veri, hangi gün
  aralığında ne çıktığını yazarak. Gün maddenin bilinen ilk hâlinden önceyse `regulation_text_unknown`, uzman
  incelemesi: o günün metni KentOS'ta yok.
- **K62 — Bulgunun durumu ve maddenin sonucu.** `ArchAuditFinding.state`: yoksa kaldı, `missingData` eksik veri,
  `expertReview` uzman incelemesi. `ArchAudited.ruleSet` (`ArchAuditRuleSet`): başvuru günü, nereden geldiği ve her
  maddenin hâli ve sonucu (`passed`, `failed`, `missingData`, `expertReview`, `outOfScope`). Geçti ve kapsam dışı bulgu
  değildir: listeyi doldurmaz, özet satırında söylenir.
- **K63 — Paket.** Kurallar yeni standart pakettedir: `catalog/standards/mimari-mevzuat` (`tr.kentos.mimari-mevzuat`,
  Python `mimari_mevzuat`), kitaplıksız, MIT, `kentos kek verify` 0 uyarı. main'de başka bir çalışma ulusal kuralları
  `tr.kentos.mevzuat` paketinde yazıyor (bu dalda yok; sahip 17 Kasım'da "main ile birleştirme şimdilik dal ile devam
  edelim" dedi). Birleştirmede bu paketin modülleri oraya taşınır; masaüstü ve başsız kanca paketi adıyla çağırır, tek
  satır değişir.

## §3. Uygulama

- **Olgular** (Rust, `kentos.auditFacts` 1'in isteğe bağlı `building` bölümü): her kat (kotu, adı), üstündeki mahaller
  programdaki grubuyla (bağımsız bölüm, ortak alan, eklenti, grupsuz) ve ayrıca teknik ve iskân edilen mahal sayısı,
  asansörleri (Asansör adlı mahal; kuyusunda durmayan Asansör bloğu); programın bağımsız bölümleri ve konutları; yerel
  yönetmelik. `kentos_extensions::audit_facts::building` biçimi, `kentos_processing::audit_facts::building` ölçüyü
  tutar; teknik mahal okuması Rust denetiminden `kentos_domain::room_names::technical`'a taşındı (ikisi aynı işlevi
  okur). Katı olmayan projede bölüm yoktur, eski olgular bayt bayt aynıdır. SDK'da `ProjectAuditFacts.building` (sözlük;
  şekli `factsSchema`'da); ajan paketi bu dalda 48.000 baytın altında.
- **Kural** (`mimari_mevzuat`): `yururluk` (hâller, gün, deneme), `asansor` (34/1 ve 34/8'in hâlleri, Sivas katmanı),
  `mimari.denetle` (masaüstü ve başsız tek süreçte çağırır: bulgular, özet satırları, günün notu), `denetim`
  (`@ext.audit("asansor")`: bulut işi ve Eklenti denetimi; eksik veri uyarı, uzman incelemesi bilgi, ikisi de iletinin
  başında).
- **Rust'tan taşınan:** 34/1'in bulgusu (`paiy_elevator_missing`) Mevzuat grubunun Rust kodundan çıktı (K10: dokunulan
  kural taşınır); 34/2'nin kuyu ve kabin ölçüsü ve 34/5'in sedye asansörü değişmedi, Rust'ta kaldı. 34/8 yeni:
  `paiy_elevator_not_serving`. Kodlar `codes.rs`'te; grup sırasında tarih ve metin bulguları başta.
- **Masaüstü** (Denetim): iki paket aynı arka plan iş parçacığında sırayla sorulur; ulusal paketin bulguları Mevzuat
  grubunda, satırın başında durumuyla ("Eksik veri ·" turuncu, "Uzman incelemesi ·"); tablonun üstünde kural setinin
  satırı ("Kural seti — Başvuru tarihi künyeden: 2.7.2026 · PAİY 34/1 (1.7.2026) geçti · PAİY 34/8 (1.7.2026) kaldı").
  Paket cevap vermezse nedeni tablonun üstünde yazılır.
- **Başsız** (MCP, Python): `cad.arch.audit` regulation grubu bütün çizim için istenince paketin bulguları grubun
  kendi bulgularından sonra, `ruleSet` çıktıda; kural çalışmazsa `regulation_rule_unavailable` uyarısı. Etkin kat ya da
  seçili nesneler için sorulmaz (katlar ve bodrumlar çizimindir).
- **Ölçüm** (bu Mac, Python 3.15.1): kuralın süreci ortanca 22 ms (en çok 24 ms), kuralın kendisi 0,03 ms (gün yokken iki
  hâl denenerek); bina olguları oda sayımıdır.

## §4. Kabul (bim-plan BIM-13'ün ilk kabulü)

- **Her yeni kural için geçen ve kalan durum, KentOS kodu olmadan yazılmış başvuruyla:** `fixtures/standards/v1/asansor.json`,
  25 durum; beklenen her sonuç §1'deki metinlerden elle çıkarıldı (`neden`). 34/1 her hâlde geçer ve kalır, 34/8 geçer,
  kalır ve uzman incelemesine kalır, Sivas katmanı bağlar ve çelişir, gün yok, okunamıyor ve bilinen metinden önce.
  `python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py` (8 sınama, eklenti yüzü dahil; sınamalar paketin dışında, §7).
- **Başvuru tarihi 30.6.2026 ve 2.7.2026 olan aynı çizim farklı kural setiyle:** üç katlı, bodrumunda iki daire
  eklentisi (depo) ve kazan dairesi olan, asansör kuyusu katlarda olup bodrumda olmayan bina. 30.6.2026: 34/1 (11.7.2021)
  plana göre 3 kat, kuyu yer olarak yeter, geçti; asansör zorunlu değil, 34/8 kapsam dışı. 2.7.2026: eklentili bodrum
  sayılır, bodrum dahil 4 kat, asansör zorunlu; 34/8 (1.7.2026) bodruma ulaşmayı ister, kaldı. Gün yok: eksik veri.
  Masaüstünde `regulation_dates_tests`, başsızda `regulation_dates` (MCP ve Python'un yolu), resimler
  `.run/gui-evidence/mevzuat-tarih-8078/` (açık ve koyu).
- **Yağmurlama beyanı kaldırılınca kaçış uzaklığı bulgusu değişir:** ADR 8035'in kuralı (Rust'ta, bu dilim dokunmadı)
  `fire_escape_tests::a_tall_dwelling_building_s_walks_and_stairs_are_said` sınamasında:
  yağmurlamasız dairede 22,00 m > 20 m (48/2) ve kapıdan kat çıkışına 26,15 m > 15 m bulgudur; `settings.arch.sprinklers`
  açılınca sınır 30 m'ye çıkar, iki bulgu düşer.

## §5. Sınamalar

- Rust: `kentos-extensions` ve `kentos-processing` olgu sınamaları (bölümün sırası ve yuvarlaması, kabul çiziminde
  katların grupları ve asansörleri, katı olmayan projede bölüm yok; `fixtures/audit-facts/v1/mimari.json` yeni bölümle
  yeniden yazıldı), SDK paketi ve stub; `kentos-commands` ve `kentos-contracts` (katalog, TS yeniden üretildi);
  uygulamanın bütün ortak komut durumları; başsız `regulation` birim sınaması ve `regulation_dates`, `bim_package`.
- Masaüstü: `regulation_audit_tests` (dört katlı binanın asansör bulgusu artık paketin; 3 katta Sivas ile gün yokken
  eksik veri, 2.7.2026'da Sivas bağlar), `regulation_dates_tests`, `fire_escape_tests`, `arch_audit`, `scene3d_building_tests`,
  `bim_permit_tests`, `permit_package_tests`, `module_tests`, `ribbon_tests`, `history::text`, `catalog`, `settings`:
  166 geçti.
- Python: paketin 8 sınaması; `kentos kek verify` iki paket için 0 uyarı.

## §6. Uzman onayı (K64)

- **Sorun:** uzman incelemesi bulgusu bir kişinin kararını bekler; plan "uzman onayı kayıtlıdır; etkilediği nesne
  değişince eskir" der ve ikinci bir denetim sistemi kurmayı yasaklar (ADV-23: mevcut istisna mekanizması). Ama
  `cad.arch.exceptions` yalnız kendi denetiminin (Rust) bulgusunu tanıyordu: paketin bulgusu işaretlenemiyordu.
- **K64 — Onay istisnadır, bulgu verilerek.** `cad.arch.exceptions`'ın `findings` girdisi: paketin Python'unu çalıştıran
  ev sahibi (masaüstü, MCP, Python) paketin bulgusunu `cad.arch.audit`'in listelediği gibi verir; `mark` onu da anahtarıyla
  işaretler, özeti öbürleriyle aynıdır (kodu, nesneleri, sözü, yeri). Uzman incelemesi bulgusunun istisnası uzmanın
  onayıdır, notu uzmanın sözüdür; Denetim'de satırı "Uzman onayı ·" diye başlar ("Uzman onayı düştü ·" eskiyince).
- **Eskime:** bulgu değişince (sözü, yeri) özet tutmaz, onay düşer. Paketin katla ilgili bulgusu artık katın
  mahallerini adlandırır (olguların `building.storeys[].rooms`'u): anahtarı mahallerindir, tabloda tıklanınca seçilir;
  bodruma bir mahal eklenince anahtar değişir, eski onay hiçbir bulguyu örtmez, yeni bulgu açıktır. Nesnesiz bulgunun
  istisnası olmaz (`auditExceptions`'ın nesne kuralı ve `.kcad` biçimi değişmedi).
- **Kendi denetimi paketin istisnasını silmez:** Mevzuat grubunun kod listesi yalnız Rust'un ürettiği kodlardır; paketin
  koduna bağlı istisna Rust denetimine göre ne eski (`stale`) ne de temizlenir (`prune`). (İlk dilimde paketin kodları
  sıra için listeye eklenmişti: onları "bulgusu kalmamış" sayardı; çıkarıldı.)
- **Ev sahibi istisnayı paketin bulgusuna bağlar:** `arch_audit::lay_exceptions` komutun kendi bulgularına uyguladığı
  kuralı (istisna, düşmüş, not; İstisnalar: Gizle/Göster/Yalnız) paketin bulgularına uygular; masaüstü ve başsız aynı
  işlevi çağırır, sayılar bir kez eklenir.
- **Sınamalar:** masaüstü `an_expert_s_approval_of_a_package_finding_is_kept_until_the_finding_changes` (onaylanır, gizlenir,
  bir kez sayılır, Göster'de notuyla; bodruma depo eklenince yeni bulgu açık), başsız `regulation_dates` (protokolle
  işaretlenir, gizlenir, Göster'de istisna ve not), ortak durumlar `cad.arch.exceptions` 28 (paket bulgusu verilerek
  onay, kodsuz verilen bulgunun reddi, verilmeyen paket bulgusunun reddi). Resim `4-uzman-onayi-{acik,koyu}.png`.

## §7. main ile birleştirme (17 Kasım 2026)

- Sahip "github üzerindeki main ile birleştir ve main üzerine push yap" dedi; dal main'e birleştirildi. Üç çakışma
  eklemeydi (çalışma alanının üyeleri, masaüstünün bağımlılıkları, `deps.mjs`): ikisi de alındı; main'in
  `extension-tools` ve `cli` grupları başsız üzerinden `kentos-standards`'a ulaştığı için bağımlılık kuralına eklendi.
- **Ajan paketinin sınırı:** main'in SDK paketi 48.000 baytın hemen altındaydı (Asistan'ın bir aracın sonucuna verdiği
  en çok bayt); dalın olgu bölümleri (`permit`, `cad`, `building`) stub'a eklenince 48.691 oldu. Olguların Python
  sınıfları (`ProjectAuditFacts` … `DoorEscape`, 7.225 bayt) paketin bir parçası oldu: `extension.sdk part=factsApi`
  (`kentos kek sdk --part factsApi`); paketin stub'ı onları bir satırla adlandırır, şeması zaten `factsSchema`'daydı.
  Simgeler, şemalar ve panel yapıcıları da böyle ayrıdır. Sınama: iki yarının toplamında stub'ın her sınıfı bir kez.
- Birleşen simge listesi yeniden yazıldı; olguların altın dosyası ve şeması yeni bölümlerle yeniden üretildi.
- MCP sınamalarından dokuzu birleştirmeden önce de main'de ve dalda kırıktı, birleştirme değiştirmedi: yedisi
  (`drape_over_mcp`, `office_standard`, `raise_by_reference`, `road_alignments`, `road_profiles`, `mcp.rs`'in iki araç
  listesi sınaması) `destructiveHint` bekliyor ama ajanların yıkıcı listesi ADR 8056 §2'de (1e4551bb3) 199 girdiden
  31'e indi; `the_read_only_hint_follows_the_catalogs_effect` `cad.rasters.gpkgwrite`'ı salt okunur buluyor;
  `the_catalog_and_the_open_drawings_are_resources` main'in eklediği `kentos://sdk/llms.txt` kaynağını (8f00ba19f)
  bilmiyor. Listenin kararı o çizginin; TODOS.md'de ayrı iş.
- main'in doğrulayıcısı paketin her Python dosyasında `sys.path` değiştirmeyi reddediyor (ADR 8057 §72); iki BIM
  paketinin sınamaları paketin kendi kaynağını bulmak için bunu yapıyordu. Sınamalar paketin dışına taşındı
  (`catalog/standards/tests/dijital-proje/`, `catalog/standards/tests/mimari-mevzuat/`): `.kek`'e girmezler, paket
  `kentos kek verify` ile yine 0 uyarı.
- Ulusal kuralların iki paketi (`tr.kentos.mevzuat`, `tr.kentos.mimari-mevzuat`) yan yana kaldı; birleşmeleri K63'teki
  gibi ayrı iştir.

## Sıradaki adımlar

- BIM-13'ün kalan kuralları: kapı açılış yönü, emsalin 22. madde sınırları (%20, %50, 70 m²) ve kat ara toplamları,
  merdiven altı 1,80 m, yangın kompartımanı ve sınırındaki açıklık, beyan edilen dayanımın (BIM-07) yönetmeliğin
  istediğiyle karşılaştırılması, erişilebilir yol (TS 9111'in değerleri TSE metninden doğrulanmadan kural olmaz). İç
  yükseklik ADR 8040 §3'te Rust'ta yapılmıştı.
- Maddenin özet satırı uzman onayını bilmiyor (onay ev sahibinde uygulanıyor): onaylanan madde "uzman incelemesi" kalır,
  onay satırda görünür.
- Öbür tarihli maddeler: 28/1-c (12.8.2023) ve 39/1 (13.8.2025) Rust'ta tek hâlle okunuyor; geçiş hükümleri okunmadan
  hâlleri ayrılmaz. 5/27'nin 1.7.2026 değişikliği (Geçici 3/11 onu da sayar) okunmadı.
- Sivas 34/1'in "bina girişi bodrum kattan yapılan yapılarda asansör yapılması zorunlu olan kata göre işlem" cümlesi
  okunmuyor; plan notu katmanı (ulusal → belediye → parsel) ve bölge türü.
- Birleştirmede: bu paket ile main'in `tr.kentos.mevzuat`'ı birleşir (K63); olgu bölümleri ve ajan paketinin sınırı
  yeniden denetlenir.
