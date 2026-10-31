# ADR 0773: Bina programı: bağımsız bölümler, tasarım hedefi ve geometriyi izleyen denetimler

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-04'ün ilk teslimi **[~]**.
- **Bağlam:** ADV-04 ("Mahalleri bina programı ve bağımsız bölüm bağlamında yönet"); dayanak A6 (bina organizasyonu
  sınırlı: kat, evre, durum, disiplin var; bölüm, bina, kanat yok). Mahal motoru ADR 0342, 0670–0692; denetim ve
  istisnaları ADR 0703, 0732; durdurulabilir uzun iş ADR 0253 §9, 0756.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'ye yazılacak.
- **Şema:** `.kcad` 138 (opt-in), `FORMATS_VERSION` 100. Birleştirmede numaraları koordinatör verir (ADV-01 ve
  ADV-02 de 137'yi aldı).

## Araştırma (üreticilerin resmî belgeleri; yerelde çalıştırılmadı)

- [Revit — Area Schemes](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-Model/files/GUID-194A8E37-06C5-426D-964C-D9148A1FEFE3.htm),
  [Area Type Rules](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-B4AC3088-DBA8-4EB0-BA8F-CAF8EC1EBF9D.htm),
  [Use Area Schemes](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-828E8162-206D-4FB3-9866-30257CAE726F.htm):
  oda (Room) ile alan planı (Area) ayrı; brüt bina alanı dış yüzden, kiralanabilir alan kurallı; her şemanın kendi
  çizelgesi. KentOS'ta alanın yöntemi mahalin kendisinde (net iç yüz, eksen, brüt dış yüz; ADR 0342) kalır, program
  bunu değiştirmez; bölümün toplamı yöntemleri söyler, karışığı gizlemez.
- [Archicad — Creating Zones](https://help.graphisoft.com/AC/22/INT/_AC22_Help/040_ElementsVB/040_ElementsVB-127.htm),
  [Zone Stamps](https://help.graphisoft.com/AC/28/INT/_AC28_Help/040_ElementsVB/040_ElementsVB-142.htm),
  [API_ZoneType](https://archicadapi.graphisoft.com/documentation/api_zonetype): bölge kategorisi etiketini seçer,
  alan iç kenardan ya da referans çizgisinden, kolon düşümü seçimli. KentOS'ta kolon, ayırıcı, şaft (boşluk), niş ve
  bitmiş/çekirdek yüz mahal motorunun mevcut kurallarıdır; program yeni bir alan motoru kurmaz.
- Sonuç: bölüm (daire), ortak alan ve dış alan, katmandan ya da kattan bağımsız bir **üyelik** olarak tutulur;
  tasarım hedefi kullanıma bağlı bir **brief**tir, yönetmelik değildir; resmî/kayıtlı alan yalnız okunur.

## Karar

### §1 Veri modeli

- Mimari ayarların `program`'ı (`BuildingProgram`): `buildings` (kimlik, ad, `wings`), `units` (kimlik, numara, ad,
  `kind`: `unit` bağımsız bölüm / `common` ortak alan / `outdoor` dış alan, `building`, `wing`, `recordedArea`),
  `briefs`, `clearances`, `version`. Kimlikler, numaralar, kullanımlar ve bloklar birer kez; kanat binasının; aralık
  doğru sırada (`BuildingProgram::problem`).
- Mahalin `unit`'i grubun kimliğidir. Kat ile bölüm arasında zorunlu ağaç yoktur: bölümün katları mahallerinden
  okunur (dubleks iki kat), bir katta birçok bölüm olur. Katman adı hiçbir anlam taşımaz.
- Programda olmayan gruba bağlı mahal okunurken reddedilmez; denetimde `program_unit_missing` olur, düzeltmesi
  `cad.spaces.edit` ile bölümden çıkarır.
- `cad.program.edit`: program yerine (verilmezse olduğu gibi) ve `assign` ile mahallerin grubu; tek geri alma adımı
  “Bina programı” (`Op::BuildingProgram` ve mahal güncellemeleri birlikte). Mahali kalan grup kaldırılmaz
  (`unit_in_use`, kimlikleriyle); aynı çağrıda başka gruba verilirse kaldırılır. `cad.spaces.edit` `set`'in `unit`'i
  ve `cad.spaces.create`'in `unit`'i de grubu yazar (`unit_not_found`).

### §2 Tasarım hedefi

- `SpaceBrief`: kullanım (mahalin `usage`'ıyla tam eşleşir), `areaMin`, `areaMax`, `minWidth` (net en),
  `minPassage` (kapılar arası net geçiş), `adjacent` (komşu olması istenen kullanımlar), `reach` (kapı ya da açık
  geçişle ulaşılması istenen kullanımlar), `outdoor` (balkon, teras: bölümün dış toplamına). `version` hedeflerin
  adı ve sürümüdür; her bulgu `kentos.program/1 · <sürüm>` ile söyler. Hedefler yasal sınır değildir, sözcükler de
  bunu söyler.
- Gerçek/hedef/fark: etiket yer tutucuları `{unit}`, `{target}`, `{targetDiff}` (hazır içerik “Bölüm ve hedef”);
  çizelge alanları `unit`, `target`, `targetDiff` (Bölüm, Tasarım hedefi, Hedef farkı); Öznitelikler'de Bölüm
  (seçilir), Tasarım hedefi, Hedef farkı; Bina programı penceresinin Durum'u. Programı okuyan etiketler program
  değişince aynı adımda yenilenir (`tag_follow`).

### §3 Alan yöntemleri

- Mahalin alanı kendi halkalarının, kendi yöntemiyle (ADR 0342 §1); açık mutfak ayırıcıyla (ADR 0679), şaft
  boşlukla (ADR 0708, toplamdan ayrı), kolon düşümü (ADR 0672), niş, bitmiş yüz ve çekirdek mahal motorundan gelir.
- `cad.program.check`'in bölüm satırı `rooms`, `outdoor`, `voids`, `methods` (ilk kullanım sırasıyla; iki yöntem:
  karışık), `levels`, `recordedArea` ve `recordedDifference`'ı verir; kayıtlı alan hiçbir ölçümle yazılmaz.
- Toplamlar her mahali bir kez sayar: `units`, `unitOutdoor`, `common`, `outdoor`, `voids`, `unassigned`, `all`
  (boşluklar dışı hepsi = öbürlerinin toplamı). Ortak alan hiçbir bölümün toplamına girmez; pay (arsa payı) yoktur.

### §4 Bölme ve birleştirmede kimlik kararı

- Birleştir (`merge`): `keep` verilen mahalin, yoksa ilkinin kimliği sürer; numarası onun (`number` başka numara,
  null numarasız), kaplaması ve grubu onun. Öbürlerinin numarası, kaplaması ve etiketleri cevabın `decisions`'ında
  “gidecek” olarak söylenir; grupları farklıysa `space_units_merged` uyarısı. Masaüstünde Mahalleri birleştir önce
  planı çalıştırır, kararı soruda söyler, Birleştir'e basılınca yazar.
- Böl (`split`): `keep` noktasını içeren, yoksa en büyük parça kimliği korur (`by`: given/largest); yeni parçalar
  büyüklük sırasıyla `numbers`'ı alır, verilmezse **numarasız** kalır (önceki davranış numarayı kopyalıyordu: aynı
  numara iki kez yazılırdı), grubu alır, `finishes` false değilse kaplamaları. Karar her parçanın alanını,
  numarasını, kaplamasını ve içinde duran donatıları (ADR 0733 §2'nin kuralıyla) söyler; Mahali böl aracı kararı
  önizleme etiketinde yazmadan önce gösterir. Tohum yakınlığıyla sessiz yer değiştirme yoktur.

### §5 Geometriyi izleyen denetimler

- Kapılar: mahalin kenarının üstünde koştuğu kapı ya da eşiksiz boşluk (`kentos_domain::passages`) ve kenarı başka
  mahalin kenarına 1 mm içinde, en az 0,3 m oturan komşu (ayırıcı). İki mahal ortak kapıyla ya da bu açık sınırla
  birbirine açılır; yüzleri en çok 0,6 m aralıklı ve 0,3 m örtüşenler komşudur.
- Net en: mahalin içindeki en büyük dairenin çapı (ulaşılmazlık kutbu, 0,1 mm). Net geçiş (hedefinde `minPassage`
  olan, iki kapısı ya da daha fazlası olan mahal): halkalar genişliğin yarısı kadar içe aşındırıldığında bütün
  kapılar aynı parçaya en çok yarım genişlik (ve bir mikron) uzaktaysa o genişlik geçer; en genişi ikiye bölerek 0,1
  mm'de bulunur, net ene 0,1 mm yakınsa net endir. En dar yeri: geçişe en yakın uzaklıktaki iki karşılıklı, değmeyen
  kenarın en yakın noktalarının ortası. Kolonun iki yanında geniş yan, niş geçişi değiştirmez.
- Donatı kullanım alanı: programın `clearances`'ı blok adına önündeki dikdörtgeni (bloğun kendi kutusunun +y
  kenarının ortasından, en × derinlik) verir; mahalden taşan alan ya da başka donatının ayak izine binmesi bulgu.
- Bulgular Mimari denetim'in `program` grubu (`cad.arch.audit`'te de): `program_unit_missing`, `program_area_below`,
  `program_area_above`, `program_width_narrow`, `program_passage_narrow`, `program_adjacent_missing`,
  `program_reach_missing`, `program_no_door` (bölüm ya da ortak alanın kapısız odası), `program_fixture_clearance`.
  İleti ölçülen değeri, hedefi, farkı ve kuralı yazar; yeri santimetreye yuvarlanır. İstisna (ADR 0732) bulgunun
  kodu, nesneleri, sözcükleri ve yerine bağlı olduğundan, 1 mm'lik kayma onu bozmaz, 10 cm'lik kayma (değişen
  sözcük) bulguyu yeniden açar (`exception_lapsed`).
- `cad.program.check` uzun iş olarak çalışır (ADR 0253 §9): mahal başına ilerleme söyler, durdurulunca `cancelled`.
  Masaüstünde Denetle okuma kopyasında kendi iş parçacığında çalışır, yüzdesini ve Durdur'u gösterir.

### §6 Arayüz

- Mimari › Proje › **Bina programı** (`arch.program`, ikon `archProgram`): Bölümler (no, ad, tür, kayıtlı alan;
  Seçilenleri ver; Denetle sonrası katlar, mahal sayısı, oda/dış/şaft toplamı, yöntemler, kayıtlı fark; toplamlar
  satırı), Hedefler (kullanım, aralık, net en, net geçiş, komşu ve geçilen kullanımlar, dış mahal), Durum (mahaller,
  hedef, fark, net en, geçiş, kapı; bulgular; kural). Kaydet `cad.program.edit`; Bina ve kanat düzenleme şimdilik
  komutla (Python/MCP), pencerede yok.

### §7 Kapsam dışı

- Kat ile bölüm arasında hesaplı pay, arsa payı ve resmî alan hesabı (mevzuat profili ayrı iştir).
- Brief'in kurum/ofis profillerinden alınması, brief'te kat ya da bölüm koşulu, komşuluk için duvar türü ayrımı.
- Kapılar arası en kısa yol uzunluğu, kaçış mesafesi ve erişilebilir dönüş dairesi.
- Pencerede bina/kanat düzenleme, birleştirmede korunacak mahali soruda seçme (komutta `keep` var).
- Gerçek pencerede el ile kabul; performans bütçesi ölçülmedi (yüzlerce mahalde geçiş taraması yalnız hedefi olan
  koridorlarda koşar).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/program_cases.py` (KentOS kodu olmadan; geçiş shapely/GEOS ile aşındırılıp
  ikiye bölünerek): `cad.program.check` 26, `cad.program.edit` 30, `cad.spaces.edit.decisions` 21 ortak durum;
  `space_command_cases.py` yeni parçanın numarasızlığına göre yenilendi.
- KCAD 138: `fixtures/kcad/v2/building-program.{json,kcad}` ve altı bozuk örnek; Rust kodeği ve
  `tools/kcad/kcad.py` aynı sonuç.
- Masaüstü kabul testi `program_scenes::tests::the_program_follows_the_partition_and_counts_each_space_once`:
  bölme 10 cm kayınca koridor geçişi 1,20 → 1,10 m ve 2. dairenin yatak odası hedef farkı aynı adımda; toplamlar her
  mahali bir kez sayar; dubleksin iki katı; birleşim kararı yazmadan önce sorulur.
