# ADR 8063: Dijital teslim kapsamı — tarihli kapsam kararı Python kuralı, tarih ve eşikler veri, girdiler künyeden

- **Durum:** uygulandı (15 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-02; uygulama sırasında B2. İlk
  kabul sağlandı (§10). Eksik kalanlar §11'de.
- **Bağlam:** ADR [8062](8062-digital-project-rule-catalog.md) (kural kataloğu, §14: denetim Python'dadır),
  [8057](8057-phase-3-extension-packages.md) §36 (çizimin olguları, `facts.project`), [8037](8037-turkish-title-block-sheet-sets-pdfa.md)
  (Ruhsat takımı), [8030](8030-emsal-taks-zoning-badge-and-table.md) (yapı inşaat alanı).
- **Araştırma:** Hazırlama Yönetmeliğinin Md. 1, 16 ve Geçici 1–2'si bu işte tam metinden yeniden okundu (alomaliye.com
  kopyası; Resmî Gazete'nin sitesi bu bilgisayarda sertifika hatası veriyor, [araştırma notu](../research/2026-11-15-bim-ifc-yonetmelik.md)
  §0). e-PYS Yönetmeliğinin Md. 1, 4/4 ve 8'i notun §1.2'sinden. Takvim notun §2'sinde.
- **Web:** dokunulmadı (ADR 0228). Veri ve kural paylaşılabilir: web dönüşünde Ruhsat takımının bu bölümü TODOS.md
  §0.2'ye yazıldı.
- **`.kcad`:** biçim değişmedi. Künyenin yeni değerleri çizimin var olan değişkenlerine yazılır (yalnız verilenler).
- **Bağımlılık:** dış bağımlılık yok. `kentos-processing` → `kentos-digital-project` yeni bir iç bağ. Masaüstünde
  `kentos-digital-project` ve `kentos-ifc` test bağımlılığından olağan bağımlılığa taşındı (§8: B1'in penceresi yalnız
  test derlemesinde derleniyordu).

## §1. Karar

1. **Karar bir Python kuralıdır (bim-plan K10).** `catalog/standards/dijital-proje` paketi KentOS'un ilk standart kural
   paketidir. Kuralı `dijital_proje.kapsam.karar`'dır. Ruhsat takımı penceresi, KentOS'un denetimi (`@ext.audit("kapsam")`)
   ve Python (`kentos.standards.kapsam`) aynı işlevi çağırır.
2. **Tarih ve eşikler veridir (bim-plan K2).** `crates/shared/digital-project/data/kapsam-2026.json`: iki yönetmeliğin
   yürürlüğü, kapsamın istisnaları, her yükümlülüğün evreleri ve nüfus ile alan eşikleri, her biri okunduğu maddeyle.
   Girdilerin listesi de aynı dosyadadır. Paket bir kopyasını taşır; Rust testi ikisini bayt bayt bir tutar.
3. **Girdiler künyeden, ölçüler çizimden gelir.** Künyenin değerleri çizimin değişkenleridir. Toplam yapı inşaat alanı
   (PAİY 4/oooo) ve bodrum dışındaki kat adedi boş bırakılırsa çizimden ölçülür. Ölçüm Rust'tadır: olguların yeni
   `permit` bölümü.
4. **Tahmin yok (bim-plan K3).** Eksik ya da okunamayan bir girdi sonucu değiştirebiliyorsa sonuç "teyit gerekli"dir.
   Değiştiremiyorsa sonuç yazılır, eksik girdi notta söylenir.
5. **Sonuç dört durumdur:** uygulanır, uygulanmaz (maddesiyle), henüz başlamadı (tarihiyle), teyit gerekli (eksik
   bilgisiyle). Satırlar: yönetmeliğin kapsamı, 2B çizimler ve belgeler PDF/A (Md. 4/2–3), BIM modeli IFC (Md. 4/5),
   modelden üretilen çizimler PDF/A (Md. 9/1), e-PYS'de teslim (e-PYS Md. 4).

## §2. Metin

| Hüküm | Ne der | Kuraldaki yeri |
|---|---|---|
| Md. 1/1 | Yapı ruhsatı için "yapılacak müracaatlarda" teslim edilecek projeler | Yürürlükten önceki başvuru kapsam dışı (K25) |
| Md. 1/2 | Kapsam: uygulama imar planı bulunan alanlarda yeni yapılacak binalar. Hariç: bodrum dışında en çok iki katlı ve toplam yapı inşaat alanı 200 m²'yi geçmeyen müstakil yapılar; entegre tesis niteliğinde olmayan tarım ve hayvancılık yapıları | `scope` |
| Md. 16 | Yürürlük 1.9.2027 | `acts[hazirlama]` |
| Geçici 1/1-a | 4/2–3, il nüfusu 2.000.000'u geçen ve belediye nüfusu 600.000'den fazla olan yerde toplam yapı inşaat alanı 5.000 m²'yi geçen binalar dışındakilere 1.3.2029'a kadar uygulanmaz | `pdfa.stages` |
| Geçici 1/1-b (1)–(4) | 4/5: 1. grup (il > 2 milyon, belediye > 600 bin, alan > 10 bin m²) 1.9.2029; 2. grup (belediye > 400 bin) 1.9.2030; 3. grup (il > 1 milyon, belediye > 300 bin) 1.9.2031; kalanlar 1.9.2032 | `ifc.stages` |
| Geçici 1/1-b son cümle | Bent uygulamaya konulduktan sonra bende tabi yapılara 4/2–3 uygulanmaz | `pdfa.endsWith` |
| Geçici 1/2 | Konut ve ticaret+konut dışındaki binalara yönetmelik 1.9.2033'e kadar uygulanmaz | `uses` |
| Geçici 2 | Geçici 1'in tarihlerinden önce ruhsat başvurusu yapılmış ya da kamuda ihale kararı veya ihale tarihi alınmış yapılara o maddeyle başlayan hükümler uygulanmaz | `earlier` |
| e-PYS Md. 1/2 | Aynı kapsam; ayrıca 3194'ün 4, 26 ve 27. maddelerindeki yapılar hariç | `scope.lawArticles` |
| e-PYS Md. 4/4 | Bina türlerini, ölçütleri, yükleme biçimini ve tarihleri Bakanlık belirler | `epys.decision` |
| e-PYS Md. 8 | Yürürlük 1.9.2028 | `acts[epys]` |

## §3. Veri: `kapsam-2026.json`

- **Biçim:** `kentos.digitalScope` 1. `inputs` (15 girdi: ad, etiket, tür, seçenekler), `acts`, `scope`, `uses`,
  `earlier`, `obligations`. Her kaynak katalogdaki `Source` biçimindedir ve "Md. 1/2", "Geçici Md. 1/1-a", "e-PYS Md.
  4/4" diye atıf verir (geçici madde atfı bu işte `Source::cite`'a eklendi).
- **Evreler:** bir yükümlülüğün evreleri sıralıdır; yapının üç eşiğinin hepsini kesin geçtiği ilk evre başlangıcını
  verir. Son evrenin eşiği yoktur, "öbür bütün binalar"dır. Rust testi tarihlerin geçerliliğini, sırasını, yalnız son
  evrenin eşiksiz olduğunu ve her atfın varlığını denetler.
- **Girdiler:** başvuru tarihi, ihale tarihi, plan alanı, yapı türü, kullanım, müstakil, tarım/hayvancılık, işlemi
  yapan belediye, il ve belediye nüfusu, nüfusun kaynağı ve yılı, toplam yapı inşaat alanı, bodrum dışındaki kat adedi,
  İmar Kanunu istisnası, Bakanlığın e-PYS kararının günü.

## §4. Olgular: `permit` bölümü

- **Yer:** `kentos.auditFacts` 1'in isteğe bağlı bölümü (`kentos_extensions::audit_facts::permit`, şeması SDK'nın
  `factsSchema`'sında). Mimari bölümü gibi: künyenin girdilerinden hiçbiri verilmemişse bölüm yazılmaz, eski çizimin
  olguları bayt bayt aynı kalır.
- **İçerik:** `particulars` (verilen girdiler, adlarıyla, kırpılmış, en çok 200 karakter), `constructionArea`
  (`Document::construction_area`: bütün katlardaki odaların brüt alanı, 0,0001 m²'ye), `storeys` (oda taşıyan katlar,
  ±0.00 ve üstü ile altı; EK-2 Tablo 2.14'ün kat kodu okumasıyla).
- **Üretici:** `kentos_processing::audit_facts::permit`: `measure` (çizimin değişkenlerinden), `facts` (pencerenin
  henüz yazılmamış değerleriyle). Python tarafı `ext.PermitFacts`.

## §5. Kural

- **Keşif:** kural girdileri tembel okur. Bilinmeyen bir girdiyi okuduğunda karar o girdinin her aday değeriyle yeniden
  verilir: her eşiğin kendisi ve bir fazlası (K23), her seçenek, okunamayan bir tarih için takvimin her günü ve bir
  önceki günü, e-PYS kararı için yürürlük günü ve çok uzak bir gün. Bütün yollar aynı sonucu veriyorsa o sonuç
  yazılır; vermiyorsa "teyit gerekli". Okuma sırası keşfi kısa tutar: alan eşiğin altındaysa nüfus hiç okunmaz.
- **Aynı sonuç:** uygulanır ve uygulanmaz için yalnız durum karşılaştırılır; başladığı gün eksik girdiye bağlıysa gün
  yazılmaz, neden söylenir. Henüz başlamadı için durum ve gün birlikte karşılaştırılır.
- **PDF/A ile IFC:** 4/2–3, başvurusu PDF/A'nın başladığı gün ile IFC'nin başladığı gün arasında yapılan yapıya
  uygulanır. Başvurusu olmayan projede bu aralık "1.3.2029 – 1.9.2032" diye gösterilir.
- **Girdilerin okunması:** tarih `GG.AA.YYYY`, `G.A.YYYY` ya da `YYYY-AA-GG`. Nüfusun binlikleri nokta ya da boşlukla
  ayrılabilir. Alanın ondalığı nokta ya da virgüldür. "5.000" gibi tek noktalı ve üç basamaklı alan 5 m² de 5000 m² de
  olabilir: okunmaz, kural ne yazılacağını söyler, çizimin alanı da kullanılmaz. Seçim, katlanmış yazımla ya da tek bir
  seçeneğin başıyla okunur.
- **Paket:** `kentos.extension` 1 manifesti, MIT, kitaplıksız; `kentos kek verify` sıfır uyarıyla doğrular.

## §6. Okuma kararları (kural kataloğunda, `kararlar-2026.json`)

| Karar | Tür | Ne |
|---|---|---|
| K23 | Sayfadan | "Geçen" ve "fazla olan" kesin büyüktür; eşiğin tam eşiti geçmez |
| K24 | KentOS'un yorumu (idare) | "… tarihine kadar uygulanmaz" hükmü o gün başlar |
| K25 | KentOS'un yorumu | Başvuru ile ihale tarihinin erkeni; yürürlükten önceki başvuru kapsam dışı; boş başvuru tarihi gelecekteki başvurudur; e-PYS'ye Hazırlama Yönetmeliğinin geçici maddeleri uygulanmaz |
| K26 | KentOS'un yorumu | Konut dışı binada 4/2–3 kendi başına başlamaz; Md. 9/1 IFC ile başlar |
| K27 | İdareye soruldu | Hangi belediyenin nüfusu (büyükşehirde) ve nüfusun yılı ile kaynağı |
| K28 | KentOS'un yorumu (idare) | Tadilat kapsam dışı; ilavenin kapsamı teyit ister |

Dijital proje kuralları penceresinin Kararlar sekmesi bunları öbür kararlarla birlikte listeler.

## §7. Pencere

Ruhsat takımı (Mimari › Proje › Ruhsat takımı) yeni bir "Dijital teslim kapsamı" bölümü taşır: 15 girdi (seçimler açılır
kutuda, tarihler `GG.AA.YYYY` ipucuyla, alan ve kat çizimden ölçülen değeri ipucu olarak gösterir), altında kararın
tablosu (Yükümlülük, Durum, Başlangıç, Madde) ve seçili satırın nedeni. Teyit isteyen satır uyarı rengiyle seçili
açılır. Karar her yazımda yenilenir: her çağrı kısa bir süreçtir (`python -I`), eski bir yazıma gelen geç cevap kuşağıyla
ayıklanır. Pencere 860 piksele genişledi: maddeler kesilmeden okunur. Paftaları oluştur, verilen girdileri çizimin
değişkenlerine yazar; boş bırakılan bir girdi yeni değişken açmaz.

## §8. Crate ayrımı ve bağımlılık yönü

- Veri ve girdi listesi `kentos-digital-project`'te (saf, wasm32). Olguların biçimi `kentos-extensions`'ta, ölçümü
  `kentos-processing`'de. Kural Python paketinde. Masaüstünün çağırıcısı `apps/desktop/src/shell/standards.rs`: bir
  paket işlevini kısa bir süreçte çalıştırır; paket `KENTOS_STANDARDS_DIR`'da, yoksa programın üstündeki
  `catalog/standards`'ta, geliştirme derlemesinde derlendiği depoda aranır.
- Yön: `kentos-processing` → `kentos-digital-project`, `kentos-extensions`; masaüstü → hepsi. Yeni crate gerekmedi:
  kapsamın verisi kural kataloğunun bir parçasıdır.
- **Düzeltme:** B1'in Dijital proje kuralları penceresi `kentos-digital-project` ile `kentos-ifc`'i masaüstünün test
  bağımlılığı olarak kullanıyordu; program kendisi derlenmezdi. İkisi olağan bağımlılığa taşındı.

## §9. Sınamalar ve kanıt

- `kentos-digital-project`: girdilerin tekliği, takvimin okunması ve sırası, paketteki kopyanın eşitliği, geçici madde
  atfı; kararlar dosyasının denetimi K23–K28'i de okur.
- `kentos-extensions`: `permit` bölümünün kırpılması, sınırı, yuvarlaması; SDK şeması ve `kentos_ext.pyi` yeniden yazıldı.
- `kentos-native-application`: verilen kapsam girdisinin değişkene yazılması, boşun eklenmemesi, sonradan boşaltılanın
  temizlenmesi.
- Paket: `catalog/standards/dijital-proje/tests/test_kapsam.py` — 43 bağımsız durum (`fixtures/standards/v1/kapsam.json`,
  beklenenler metinden elle), her satırın maddesi ve nedeni, girdilerin okunması, boş künyenin hızı, aynı girdinin aynı
  sonucu.
- Masaüstü: `permit_scope_tests` — pencere açılınca sorar, satırları ve günleri doğru söyler, eski cevap ayıklanır,
  Paftaları oluştur değişkenleri yazar ve olgular onları taşır; eksik nüfus teyit ister, "5.000" okunmaz. Resimler
  `.run/gui-evidence/dijital-kapsam-8063/` (açık ve koyu: şerit, konut, büyük il, teyit gerekli).
- `python/tests/test_standards.py`: `kentos.standards.kapsam`; modül doğrudan çalıştırıldı, SDK'nın bütün takımı
  (yerel modülün derlenmesini ister) bu işte koşulmadı.

## §10. Kabul (plan: BIM-02 ilk kabul)

| Ölçüt | Durum |
|---|---|
| ≥ 20 bağımsız durum | 43 durum, hepsi geçiyor |
| Her eşiğin tam eşiti, altı ve üstü | il 1 ve 2 milyon, belediye 300, 400 ve 600 bin, alan 200, 5.000 ve 10.000 m² |
| Her tarihin önceki günü ve kendisi | 1.9.2027, 1.3.2029, 1.9.2029, 1.9.2032, 1.9.2033; başvuruyla ve kararın günüyle |
| Müstakil 200 m² ve iki kat; karma kullanım; kamu ihalesi | var; müstakil alan ve kat çizimden de |
| İlk ve sonraki IFC grupları, kalan grup, konut dışı erteleme | 1.–3. grup, kalan grup, konut dışı 1.9.2033 |
| e-PYS kararının farklı başlangıcı | 1.1.2029, yürürlükten erken karar, karardan önceki başvuru, karar yok |
| Nüfus ya da fiilî uygulama kararı eksikse teyit gerekli | eksik nüfus ve eksik e-PYS kararı teyit ister |

## §11. Eksik kalanlar

- **MCP aracı:** pencerenin çağırıcısı masaüstündedir. MCP'nin aynı kuralı çağırması için çağırıcı Python ortamı
  crate'ine (`kentos-python-env`) taşınmalı ve `scripts/arch/deps.mjs`'de MCP grubunun bağları genişlemelidir; ayrı bir
  adımdır.
- **Bulut işi:** aynı betiğin bulutta çalışması eklenti planının ADIM 20'sidir (`RunRequest::Audit`).
- **Kararın günü** bilgisayarın saatine göre UTC günüdür; Türkiye saatiyle gece yarısından sonraki ilk üç saatte bir
  önceki gün okunur.
- **Nüfus verisi:** KentOS il ve belediye nüfusunu kendisi getirmez; kullanıcı yazar, kaynağı kaydolur (K27).
- **Belediyenin kendi okuması** (K24, K27, K28) bir kurum profili katmanıdır; kurum katmanının projede saklanması
  ADR 8062 §5'te açık.
