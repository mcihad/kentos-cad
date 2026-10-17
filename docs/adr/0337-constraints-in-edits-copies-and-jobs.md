# ADR 0337: Düzenlemelerde kısıtlar, kayıp ve kopya, iş olarak çözüm

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §13 `CAD-PARAM-08`, `09`, `13` (sahibin sözü: “cad param
  ile devam et ve bitene kadar durma”; beş grubun dördüncüsü). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0334 (model ve çözücü), 0335 (kısıt komutları ve aracı), 0336 (parametreler, tek işlem, taşınma), 0253
  (komut protokolü, işler), 0297 (Esnet'in izleyenleri), 0300 (ilişkili dizi).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0334–0336 kısıtları kurdu, komutlarla ekletti ve parametrelerle sürdürdü. Ama kısıtlı bir çizim başka bir komutla
düzenlendiğinde (tutamaç, Taşı, Döndür, Esnet, köşe ekle, Buda, Kır, Kopyala) kısıtlar ya sessizce bozuluyor (yalnız düzenlenen
nesne yer değiştiriyor, ona bağlı olanlar kalıyordu) ya da yanlış parçayı anıyordu (köşe eklenince kenarın sırası kayar). AutoCAD'de
kısıtlı geometri sürüklendiğinde öbürleri izler; Shift ile “gevşetilen” düzenlemede tutmayan kısıtlar kalkar; kopyada kopyalanan
nesnelerin aralarındaki kısıtlar kopyalanır.

## Karar

### 1. Düzenlemede kısıtlar korunur (`CAD-PARAM-08`)

Her yazan komutun çağrısı (`call::recorded`, en dıştaki adım) kısıtların andığı nesneleri **değiştirdiyse** (eklemeden ve
silmeden; silme §2'de) ve kendisi kısıtları değiştirmediyse (kısıt ve parametre komutları kendi çözümünü yapar), yazmasının
ardından aynı adımda çizim çözülür:

- **Tutulanlar:** her değişen kısıtlı nesnenin yazmanın oynattığı parçaları (yeri değişen uç, köşe, merkez, düğüm; yarıçapı
  değişen daire ya da yayın çemberi). Bir sayının metrenin milyarda birinden az değişmesi oynatmak değildir: nesneyi bütünüyle
  yeniden yazan düzenleme öbür parçaları yuvarlanmış yazabilir (çekirdeğin `moved_parts`'ı). Taşınan bütün nesne bütünüyle tutulur; tutamaçla çekilen köşe yalnız kendisi. Yapısı
  değişen nesne (köşe sayısı, halka, parça, tür) tutulmaz; sıralı parçalarını anan kısıtlar §2'yle gider.
- **Sabit:** sabit kısıtı yerini yazmadan önceki hâlinden alır (çözücünün `fix_from`'u): sabitlenmiş noktayı taşımak çelişkidir.
- Kilitli katmandaki nesneler tutulur (ADR 0335).
- **Tutunca** çözümün oynattığı öbür nesneler aynı adımda yazılır; belgenin izleyicileri (bağlı ölçüler, ilişkili taramalar, bağlı
  yazılar) onları da izler. Çağrının özeti ve kaydı onları değişenler arasında sayar.
- **Tutmayınca** (çelişki, tekillik, yakınsamama) hiçbir şey yazılmaz: `constraint_conflict` (çelişen kısıtların kimlikleriyle),
  `constraint_singular`, `not_converged`; ileti gevşetmeyi söyler. Durdurulan çözüm `cancelled` (§4).

**Kesen komutlar** çözmez: Buda, Uzat, Kır, Birleştir, Böl, Patlat, Köşe yuvarla, Pah, Çizimi temizle, Yönü çevir ve Birimi
değiştir'in parçaları kısıtların yazıldığı parçalar değildir; yazmanın tutmadığı kısıtlar aynı adımda kaldırılır ve
`constraints_removed` sayar (`constraint_follow::CUTTING`).

**Gevşetme** (açık seçenek; AutoCAD'in Shift'i): çağrının `relaxConstraints: true`'su ile çözüm yapılmaz; yazmanın sonunda
tutmayan kısıtlar aynı adımda kaldırılır (ölçünün parametresi, başka bir parametre okumuyorsa onunla; ADR 0334'ün silme kuralı),
uyarı `constraints_relaxed` onları sayar ve nesneleri adlarıyla verir. Tutan kısıtlar kalır.

- Protokol: `CallEnvelope.relaxConstraints` (yok: koru); çağrının kaydı (`CallRecord.relaxConstraints`) onu taşır, tekrar
  oynatma aynı seçenekle oynatır; parmak izine girer.
- Belge: `Document::relax_constraints(Relax)` (`Off`, `Next`: bir sonraki yazan çağrı, `On`: kapatılana dek); oturumun durumu,
  geri alma adımı değil. Masaüstünün “Gevşeterek düzenle” komutu `Next`'i kurar; Python'un `with doc.relaxed():`'ı bloğun içinde
  `On`'u; MCP'nin yazan araçlarının `relaxConstraints` bağımsız değişkeni zarfınkini.

### 2. Kayıp (`CAD-PARAM-09`)

- **Silinen nesne:** kısıtları onunla gider (ADR 0334 §1).
- **Yapısı değişen nesne:** adımın değiştirdiği bir nesnenin türü, parçaları, halkaları, köşe sayısı ya da kapalılığı değiştiyse
  (Kır, Buda, Uzat ile tür değişimi, köşe ekle ve sil, Birleştir, Patlat'ın kalanı), sıralı parçalarını (köşe, kenar, düğüm) anan
  kısıtları ve artık sahip olmadığı bir parçayı (yaya dönen kenar, kalkan merkez) anan kısıtları aynı adımda gider. Başı, sonu,
  merkezi ve çemberi anan kısıtlar nesnenin o parçası durdukça kalır.
- Kaldırılan her kısıt açıkça söylenir: uyarı `constraints_removed` (sayısıyla). Protokolün planı yazmanın uyarılarını da
  verir (`call::plan` yazmayı kopyada yapar), böylece kayıp yazılmadan önce görünür.

### 3. Kopya (`CAD-PARAM-09`)

- `cad.entities.transform`'un `copy`'si ve `cad.entities.array` (ilişkisiz) kopyalanan nesnelerin **aralarındaki** kısıtları her
  kopyaya yeni kimlikle kopyalar: bütün nesneleri kopyalananlar arasında olan kısıt. Ölçünün kopyası yeni bir ad alır (`d…`,
  `ang…`, `rad…`, `dia…`; ADR 0335'in sırası) ve değeri kopyanın ölçtüğüdür (ölçekli kopyada ölçeklenmiş); kopyalar birbirinden
  bağımsızdır. Sabitin kopyası kopyayı yerinde tutar.
- Kopyanın tutmadığı kısıt kopyalanmaz (döndürülmüş kopyada yatay ve düşey, aynalanmış kopyada teğetin yanı): uyarı
  `constraints_not_copied`; aradaki kısıtı kopyalanmayan bir nesneyi anan kısıt da kopyalanmaz ve aynı uyarıyla sayılır.
- Pano (`cad.entities.copy`) ve ilişkili dizi kısıt taşımaz: seçilenlerin kısıtı varsa `constraints_not_copied`.

### 4. İş olarak çözüm (`CAD-PARAM-13`)

- Kısıt ve parametre komutları ve §1'in izleme çözümü işin durdurmasını dinler: çözücü adımlar arasında sorar
  (`ExecutionContext::stopped`), durdurulunca `cancelled` döner ve hiçbir şey yazılmaz.
- Her komut protokolün işi olarak başlatılabilir (ADR 0253 §9: `start`, `job`, `cancel`): çözüm belgenin okuma kopyasında, başka bir
  iş parçacığında yapılır, sonuç sahnelenir (`staged`) ve ev sahibinin kısa yazmasıyla kesinleşir; iş sürerken belge değişmişse ve
  çözümün oynattığı nesneler artık işin gördüğü gibi değilse `document_changed`, hiçbir şey yazılmaz. Durdurulan ya da süresi dolan
  iş (ev sahibinin `cancel`'ı) sahnelenen değişikliği bırakır; zaman aşımı ev sahibinin kararıdır, çözücünün adım sınırı
  `not_converged`'dir.

## Sonuçlar

- Kısıtlı çizimde her düzenleme komutu çözücüden geçer: kısıtsız çizimde maliyet yok (kısıt kümesi boşsa hiçbir şey okunmaz),
  kısıtlı çizimde yalnız kısıtların andığı nesneler ve onların grupları çözülür.
- Yeni uyarı kodları: `constraints_relaxed`, `constraints_removed`, `constraints_not_copied`; `codes.rs`'te. İzleme her yazan
  çağrıda olduğundan `constraint_conflict`, `constraint_singular`, `not_converged`, `constraints_removed` ve `constraints_relaxed`
  protokolün ortak kodlarındadır (`PROTOCOL_CODES`); `constraints_not_copied` Kopyala'nın, Dizi'nin ve panonun yeteneklerinde.
- Tutulan parçaların çelişkisinde hangi kısıtların “çelişen” sayıldığı çözücünün sırasıdır; düzenlemenin sayıları yuvarladığı
  (döndürülen) bir durumda ortak durumlar yalnız çelişkiyi karşılaştırır.
- Bilinen sınır: Yönü çevir (`cad.entities.reverse`) çizginin başını ve sonunu yer değiştirir; başı ve sonu anan kısıtlar yeni
  başı ve sonu anar (kısıt doğru kalır ama başka bir noktayı tutar). Kapalı halkanın başlangıç köşesini değiştiren düzenleme yapı
  değişikliği sayılmaz.

## Doğrulama

- `python3 scripts/fixtures/constraint_follow_cases.py --check`: `fixtures/commands/v1/cad.constraints.edit.follow.json`'un 22 durumu
  (taşıma, döndürme, tutamaç, sabit köşe, gevşetme, silme, köşe silme, kırma, kopya, döndürülmüş, ölçekli ve yarım kopya, dizi, pano),
  KentOS kodu olmadan (çözümler ADR 0334'ün bağımsız çözücü başvurusundan, tutmama denetimi çözücünün toleransıyla); masaüstünün ve
  Python'un oynatıcıları geçer (yeni adım anahtarı `relaxConstraints`, beklentiler `constraintKinds` ve `parameters`).
- İş: `cargo test -p kentos-native-application --lib constraint_jobs_tests`: durdurulan çözüm (parametre ve izleme) hiçbir şey
  yazmaz, işin sahnelenen çözümü belge değişince `document_changed`, durdurulan iş bırakılır.
- Python: `test_constraints.py`'nin `test_an_edit_keeps_the_constraints_or_relaxes_them`'i (izleme, çelişki, `with doc.relaxed():`).
- Resimler: `KENTOS_SHOTS_ONLY=kisit-izle-onizleme,kisit-izle,kisit-izle-celiski,kisit-gevset cargo test -p kentos-desktop tools_screens
  -- --ignored --nocapture` (`.run/shots/arac-kisit-izle-*`, `arac-kisit-gevset-*`).
