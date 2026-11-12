# 5080 — Dışarıdaki numaranın çakışma denetimi; değer esaslı dağıtım (kullanıcının seçtiği karşılaştırma)

Tarih: 2026-11-11. plan/map-cad.md MAP-11, onuncu tur. Durum: taslak. ADR 4680'in “Sonra”sındaki “dışarıdaki notun
çakışma denetimi” ve ADR 1460'tan beri açık “değer esaslı dağıtım”.

İlgili:
- [ADR 4680](4680-separation-number-on-a-leader.md): sığmayan numara kılavuzla dışarıda (yalnız doğuya, bir boy öte);
- [ADR 1460](1460-readjustment-distribution-draft.md): düzenleme dağıtım taslağı (`cad.readjustment.distribute`);
- ADR 0145 (yazının kutusu), ADR 0146 (kılavuzun düzeni), ADR 0004 (sayısal politika).

## Dayanak

- **Mevzuat** (araştırma notu `.run/research/map-cad/mevzuat/MAP-11.10.md`, 10 Kasım 2026):
  - Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik (RG 22.02.2020/31047; değişiklikler RG 21.10.2020/31281,
    11.09.2025/33014, 13.12.2025/33106) ve BÖHHBÜY Ek-3 (2612 ada, 2613 parsel numarası yazı boyları): numaranın
    parselin dışına yazılması, kılavuz ve yazıların çakışması için **hüküm yok**.
  - Dağıtım **alan ve konum esaslıdır** (m. 17(1)). Değere dayanan hükümler yalnız imar hakkı aktarımında
    (m. 38/B, 38/C: değerler lisanslı değerleme ve kıymet takdir komisyonundan) ve tarımsal toplulaştırmada (parsel
    endeks değeri, değer sayısı; Genelge 2025/4 m. 3) vardır. Kentsel düzenlemede değer esaslı genel dağıtım kuralı
    **yoktur**.
- **ArcGIS Pro**, [Prevent labels from overlapping certain features](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/text/prevent-labels-from-overlapping-certain-features.html)
  (11 Kasım 2026'da okundu; resimler `.run/research/map-cad/uygulama/MAP-11.10-resim/`):
  - çokgende iç ve sınır ağırlığı ayrıdır: etiket çokgenin içine binebilir, sınırına binmeyebilir;
  - motor önce boş yere koyar; boş yer yoksa en az ağırlığa (önce/sonra resminde: yolların hepsini değil yalnız birini
    kestiği yere).
- **ArcGIS Pro**, dışarıya yerleştirme (ADR 4680'in sayfası): önce sınırdan bir aralık ötede yatay, olmazsa dışa doğru en
  büyük aralığa dek. **QGIS** etiket ayarları: sekiz yerleşim yönü, engel olarak çokgenin sınırı.
- **Netcad wiki** (wiki.netcad.com.tr): giriş istiyor; okunamadı. NETTOP'un değer esaslı dağıtımı (endeks, değer sayısı)
  tarımsal toplulaştırma içindir (ürün sayfası, araştırma notu).
- **Baugesetzbuch (Almanya, resmî federal metin)**, [§ 56](https://www.gesetze-im-internet.de/bbaug/__56.html) ve
  [§ 57](https://www.gesetze-im-internet.de/bbaug/__57.html) (11 Kasım 2026'da okundu):
  - § 56(1): hak sahiplerinin dağıtım kütlesindeki payı eski parsellerin ya alanları ya da düzenlemeden önceki
    değerleri oranından hesaplanır; ölçüt bir kez seçilir;
  - § 57: değer esasında dağıtım kütlesi bu oranla dağıtılır; ayrıca her malike en az eski değeri kadar parsel,
    düzenlemenin değer değişimi ve farkların parayla denkleştirilmesi.
- **KentOS bugün:** dışarıdaki numara yalnız doğuya konuyordu, çizimdeki hiçbir şey denetlenmiyordu (ADR 4680'in
  sınırı); dağıtım yalnız alan esaslıydı.

## Karar

### 1. Dışarıdaki numaranın çakışma denetimi (`cad.readjustment.sheets`, sürüm aynı)

1. `numberLeader` ile sığmayan her numara için yerler **en yakından** denenir:
   - uzaklık: kılavuzun çizgisi parçanın sınırını son kestiği yerden 1, 2, sonra 3 yazı boyu öteye uzanır;
   - yön: her uzaklıkta doğu (ADR 4680'inki), batı, kuzey, güney, sonra kuzeydoğu, kuzeybatı, güneydoğu, güneybatı;
   - kılavuzun ucu parçanın etiket yeridir; iniş ve not ADR 0146'nın düzeniyle (çizgi sağa ya da dik giderse sağa,
     değilse sola).
   - Bu sıra ve üç boy KentOS'un seçimidir, resmî bir değer değildir.
2. **Engeller** (paftanın haritasının gösterdikleri):
   - çizimin gösterilen katmanlarındaki nesneler: çizgiler ve alanların **sınırları** (içleri değil: kadastro paftasında
     boş yer yoktur, her yer bir parseldir; ArcGIS'in sınır ağırlığı), yazıların kutusu, kılavuzların çizgisi ve notu,
     noktalar;
   - aynı paftanın numaraları: parçasına sığan numaraların kutusu ve daha önce yerleştirilen kılavuzlar;
   - ayırma çapının kendi katmanları (“Ayırma çapı noktaları” grubu) sayılmaz: her harita öbür parsellerinkini dondurur.
3. **Değme:** kılavuzun parçanın dışındaki çizgisi (parçadan çıktığı yer sayılmaz; komşunun ortak sınırı da orada), inişi
   ya da notun kutusu bir engelin kenarını keser ya da ona değer, bir yazının kutusuna girer ya da bir noktayı içine
   alır. Değmeyen ilk yer seçilir.
4. **Hiçbir yer temiz değilse:** en az engele değen yer, eşitlerde ilki (ArcGIS'in “en az ağırlık”ı). Numara
   kaybolmaz; etikette `clear: false` ve değdikleri (`clashes`: çizimin nesneleri kalıcı kimlikleriyle, paftanın öbür
   numaraları parça adlarıyla, sıralı) ve `number_crowded` uyarısı: “… dışarıda hiçbir şeye değmeyen bir yere
   konamadı; en az şeye değdiği yere kondu (n nesne ya da numara). Numaranın yazı boyunu küçültün ya da kılavuzu
   çizimde taşıyın.”
5. **Cevap:** etiketin `at`'ı inişin başladığı yer, `leader` ucu (ADR 4680), `clear` her kılavuzlu numarada.
6. **Masaüstü:** Sığmayan numara anahtarının yazısı “Küçültme; parçanın dışına, hiçbir şeye değmeyen yere kılavuzla
   yaz”; uyarı durum çubuğunda.

### 2. Değer esaslı dağıtım (`cad.readjustment.distribute`, sürüm aynı; yalnız kullanıcının seçimiyle)

1. **Kural paketine `basis`:** yokken ya da `area` iken bugünkü (yönetmeliğin) alan esaslı hesap; `value` kullanıcının
   seçtiği karşılaştırmadır.
2. **Değer esasında:**
   - kesinti bugünkü gibi bir kez yuvarlanır (Σ alan × oran);
   - **dağıtım kütlesi** (tapu toplamı − kesinti) parsellere düzenlemeden önceki **değerleri oranında** dağıtılır
     (BauGB § 56(1), § 57'nin ilk cümlesi); paketin basamağı, yuvarlaması ve artık kuralıyla; artığın gittiği parseller
     `massRemainderTo`'da;
   - parselin kesintisi alanı − payıdır; değeri yüksek parselde eksi olabilir (alan kazanır);
   - hak sahiplerine paylarıyla dağıtım ve kesin korunum denetimi bugünkü gibi.
3. **Her değer kullanıcınındır:** kayıtlarda `value` (düz ondalık, eksi değil, parselin her kaydında aynı) ve paketin
   `valueSource`'u (kim, hangi kararla belirledi). KentOS değer üretmez, tahmin etmez.
4. **Resmî sunulmaz:** her değer esaslı cevap `basis: "value"`, `valueSource` ve `value_basis_not_a_rule` uyarısını
   taşır: “Değer esaslı dağıtım arazi ve arsa düzenlemesinin yönetmeliğinde bir kural değildir (dağıtım alan
   esaslıdır); bu, kullanıcının seçtiği bir karşılaştırma taslağıdır …”. Sonuç ADR 1460'taki gibi her zaman taslaktır.
5. **Retler** (`invalid_input`): bilinmeyen esas (`rules.basis`), değerde kaynak yok (`rules.valueSource`), alan
   esasında değer verilmiş, değer esasında değeri yok, düz ondalık değil, eksi ya da parselin kayıtlarında farklı
   (`entries[i].value`), değerler toplamı sıfır (`entries`).
6. **Masaüstü** (Düzenleme dağıtım taslağı): **Dağıtım esası** satırı (Alan · yönetmelik / Değer · karşılaştırma) ve
   değerde **Değerlerin kaynağı**; CSV'de beşinci sütun `değer`; Parseller görünümünde Değer ve Değer payı sütunları;
   uyarı sonucun ilk satırında.

### 3. Yapılmayanlar

- § 57'nin öbür cümleleri (yeni parselin piyasa değeri, en az eski değer koşulu, parayla denkleştirme): yeni parseller ve
  onların değerleri yoktur; uydurulmaz.
- İmar hakkı aktarımı (m. 38/B) ve tarımsal toplulaştırmanın değer sayısı ayrı işlerdir.

## Sınırlar

- Noktaların etiketleri (numaraları) haritada ekran ölçüsüyle çizilir; kutuları engel sayılmaz, noktanın kendisi sayılır.
- Ölçülerin yazıları, tarama ve resimler engel değildir; yalnız kenarları olan nesneler (`entity_edges`) ve yazılar,
  kılavuzlar, noktalar.
- Kılavuzun parçanın içindeki kısmı denetlenmez (parçanın kendi içi).
- Kenara tam değme kayan noktalı sayıyla kararlaştırılır; ortak durumlar kararı bir milimetreden uzak tutar.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/readjustment_sheets_cases.py` (52 durum, 6 yeni): doğuda yazıya değen numara batıya; gizli
    katmandaki yazı engel değil; başka kılavuzun çizgisi; her yer bir çizgiye değince en aza değen yer ve uyarı (uygula
    ve plan); aynı paftanın öbür numarasına değen not kuzeye. ADR 4680'in 6 mm'lik durumunda 101/10'un kuzeydeki
    numarası artık batıdadır (doğuda 101/8'in sınırını kesiyordu); öbür 45 durum aynı.
  - `scripts/fixtures/readjustment_cases.py` (44 durum, 14 yeni): değer oranında kütle (4/7, 3/7), artık ilk en çok
    eksik kalana, alanla aynı oranda değerde alan esaslıyla aynı net, değeri sıfır parsel, `reject`, bütün retler.
    Python `fractions`'la.
- **Rust koşucusu** (`every_command_case capabilities`, iki komut, 96 durum) geçer; bozma denemesi: iki beklenen değer
  (strip durumunda kuzeydeki numaranın yeri, değer esasında 101/1'in payı) değiştirilince koşucu tam o ikisini yakalar.
- **Python** (`maturin build` ile): `test_separation_number_clear.py` (testin kendi okuması: fontTools ilerlemeleri,
  ADR 0145/0146 kutusu, kendi kesişme sınaması; notlar ve kılavuzlar hiçbir alan kenarına değmez, yol halkasında ikisi
  de yalnız ona değer ve söylenir), `test_readjustment_value_basis.py` (kesirlerle kütle, paylar, korunum, uyarı,
  retler), güncellenen `test_separation_number_leader.py` ve ortak durumlar geçer.
- **MCP** (`crates/native/mcp/tests/separation_number_clear.rs`): kalabalık numaralar durumunki ve iki `number_crowded`;
  değer esaslı dağıtım durumunki, uyarı ve kaynaksız ret.
- **Masaüstü** (`readjustment_window_tests`): 4680'in testi güncel (güneydeki doğuda, kuzeydeki batıda, uyarı yok); yol
  halkasında iki uyarı; Dağıtım esası Değer ile 4/7, 3/7 ve CSV, kaynaksız ve alan esasında değerli ret. Çapraz
  denetimler (module_tests, ribbon_tests, history::text, catalog, settings) geçer. Resimler
  `.run/gui-evidence/map-11-10/` (değer esaslı pencere, Ayırma çapı penceresi, çakışmasız model; açık geniş ve koyu dar).

## Sonra

- Yaylı parçada yay boyunca yazı; noktaların etiket kutularının engel sayılması.
- Değer esaslı dağıtımda yeni parsellerin değeri ve parayla denkleştirme (kaynak ve veri beklenir).
- Tescil bildirimi (TKGM'nin biçimi), m. 8(5), ikinci kabul. Web (TODOS §0.2).
