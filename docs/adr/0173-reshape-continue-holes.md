# ADR 0173: Biçim değiştir, Sürdür ve delikler

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın on altıncı işi `HYB-16`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır.
- **Bağlam belgesi:** TODOS.md `HYB-16`, [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0065 (alan işlemleri:
  Alan çıkar, Alan böl, bindirme motoru), ADR 0047 (`cad.entities.edit`), ADR 0142 (köşe kotu: kotun taşınması), ADR 0143 (çok parçalı
  alan), ADR 0160 (topolojik düzenleme), ADR 0172 (köşe tablosu); ArcGIS Pro Reshape, Continue Feature, Cut a hole, Fill a hole; QGIS
  Reshape Features, Add Ring, Fill Ring, Delete Ring; Netcad Alan Düzeltme.

## Bağlam

Bir parselin sınırının bir bölümünü yeniden çizmek bugün köşe köşe yapılır: tutamaçla taşıma, köşe ekleme ve silme. Bitmiş bir çoklu
çizgiyi sürdürmenin yolu yoktur (yeni çizgi çizilip Birleştir'le bağlanır). Alana delik Alan çıkar'la açılır (önce deliğin alanı çizilir,
sonra çıkarılır, sonra silinir); delik silinemez (Patlat'la halkalara ayırıp yeniden kurmak gerekir), deliği dolduran alan elle çizilir.

ArcGIS'in Reshape'i ve QGIS'in Reshape Features'ı çizilen hatla sınırı değiştirir: “nesne, çizimin sınırı ilk ve son kestiği yerler
arasındaki biçimini alır”. Alanda hat alanın içinden geçerse alan kırpılır, dışından dolanırsa büyür (QGIS: “içerideki parçalar kırpar,
dışarıdakiler genişletir”); ArcGIS ikiye bölünen alanın büyüğünü tutar. Çizgide hat ilk ve son kesişimi arasındaki bölümün yerine geçer;
tek kesişimde (ArcGIS'in seçeneği) kısa taraf kırpılır. Continue Feature bitmiş çizgiyi ucundan sürdürür. QGIS'in Add Ring'i seçili alana
delik çizer, Delete Ring tıklanan deliği siler, Fill Ring deliği aynı biçimde yeni bir nesneyle doldurur (öznitelikleri üst nesneden).
ArcGIS'in Fill a hole'u deliği siler.

## Karar

### 1. Araçlar

Beş araç, ikisi değiştirme araçlarının (Değiştir, CBS'de Düzenle), üçü alan işlemlerinin yanında:

| Araç | Kimlik | Takma adlar | Adım |
|---|---|---|---|
| Biçim değiştir | `tool.reshape` | RESHAPE, BICIM | “Biçim değiştir” |
| Sürdür | `tool.continue` | CONTINUE, SURDUR | “Sürdür” |
| Delik ekle | `tool.holeAdd` | ADDRING, DELIKEKLE | “Delik ekle” |
| Deliği sil | `tool.holeRemove` | DELETERING, DELIKSIL | “Deliği sil” |
| Deliği doldur | `tool.holeFill` | FILLRING, DELIKDOLDUR | “Deliği doldur” |

Her araç `cad.entities.edit` ile yazar; işlemin adı geri alma adımının adıdır (sözleşmeye beş işlem eklenir: `reshape`, `continue`,
`holeAdd`, `holeRemove`, `holeFill`). Kilitli katmandaki nesne seçilemez ve yazılmaz (komutun kuralı). Hesaplar çekirdektedir; araçların
akışı iki platformda ortak izlerle sınanır.

### 2. Biçim değiştir: alan

- **Akış:** önce alan seçilir (seçim varsa o; birden çoksa “Biçimi değişecek tek alan ya da çizgi seçin.”). Sonra hat nokta nokta
  çizilir (kenet, değer kartı, Esc bir nokta geri); önizleme sonucu ve alanını gösterir; Enter ya da sağ tık uygular.
- **Hangi parça:** hat çok parçalı alanın tek bir parçasının dış halkasına değmelidir; birden çok parçaya değerse: “Hat alanın birden çok
  parçasına değiyor; bir parçayı düzenleyin.” Deliğe değen hat reddedilir: “Hat bir deliğe değiyor; deliği Delik araçlarıyla düzenleyin.”
- **Kırpma:** hat parçanın içinden geçip onu ikiye (ya da daha çoğuna) bölüyorsa en büyük alanlı parça kalır, ötekiler gider (ArcGIS'in
  kuralı; çevre yerine alan, kadastroda kırpılan şerit küçük olandır). Eşit alanlarda parçaların bindirmedeki sırası.
- **Genişletme:** hat parçanın dışından dolanıp sınırla bir ya da daha çok cep kapatıyorsa cepler parçaya eklenir. Cep: hatla sınırın
  çizgilerinin kapattığı, parçanın ve deliklerinin dışında kalan, hatta değen yüz.
- **İkisi birden** (hat hem keser hem cep kapatır) reddedilir: “Hat alanı hem kesiyor hem büyütüyor; ikisini ayrı hatlarla yapın.”
  Hiçbiri yoksa: “Hat alanın sınırını iki kez kesmeli ya da ona değmeli.”
- **Sonuç** tek alan olarak nesnenin yerine yazılır; öteki parçalar değişmez. Yaylar yay kalır (bindirme motoru yayları korur). Kotlar
  komutun kuralıyla taşınır (ADR 0142: eski sınırın üstündeki köşe kotunu, hattın sınır dışındaki köşeleri kotsuz).
- **İleti:** “Alan 1 234.56 m² oldu (−12.34 m²).”

### 3. Biçim değiştir: çizgi

- **Hedef:** çizgi ya da çoklu çizgi. Hat ile hedefin kesişimleri (kesme ya da değme; hattın uçları hedefin üstündeyse onlar da) hat
  boyunca sıralanır.
- **İki ya da daha çok kesişim:** hedefin ilk ve son kesişim arasındaki bölümü hattın o arasındaki bölümüyle değişir; hattın yönü
  hedefinkine çevrilir. Hattın ilk kesişimden önceki ve son kesişimden sonraki ucu atılır.
- **Tek kesişim:** hedefin kesişimden kısa tarafı atılır, yerine hattın kesişimden uzun tarafı gelir (çizginin ucu yeniden çizilir; ArcGIS'in
  “Reshape with single intersection”ı).
- **Kesişim yoksa:** “Hat çizgiyi kesmeli ya da ona değmeli.”
- Hedefin kalan bölümlerindeki yaylar korunur (kesilen yay çemberinde kalır). Çizgi çoklu çizgi olur.
- **İleti:** “Uzunluk 123.456 m oldu (+4.321 m).”

### 4. Sürdür

Çizgi ya da çoklu çizgi seçilir; tıklamaya yakın ucu başlangıçtır (seçimle gelinmişse imlece yakın ucu, ilk tıklamada belirlenir). Çoklu
çizgi aracı gibi noktalar verilir (kenet, değer kartı, Esc bir nokta geri); önizleme eklenen bölümü gösterir. Enter yazar: nesne aynı
kalır, köşeleri uca eklenir (baş uçtan sürdürülürse başa, sırası korunarak). Çizgi çoklu çizgi olur. Yeni köşeler kotsuzdur. Kapalı alan
sürdürülmez: “Sürdür çizgi ve çoklu çizgi içindir.”

### 5. Delikler

- **Delik ekle:** alan seçilir (yoksa ilk nokta hangi alanın içindeyse o; o da yoksa “Deliğin ekleneceği alanı seçin ya da alanın içine
  tıklayın.”). Halka Kapalı alan aracı gibi çizilir (en az üç köşe, Enter kapatır; yay kipi yok). Halka parçanın içinde kalmalıdır:
  sınırı aşan halka reddedilir, “Delik alanın içinde kalmalı; sınırı aşan bölümü çıkarmak için Alan çıkar'ı kullanın.”. Var olan bir
  deliğe değen ya da onu içine alan halka o delikle birleşir (çıkarmanın sonucu). Delik halkanın parçasına eklenir.
- **Deliği sil:** imleç bir deliğin içindeyken delik vurgulanır; tıklama siler (alan orada doluya döner). Araç açık kalır; birden çok delik
  sırayla silinir. Deliğin dışına tıklama: “Bir deliğin içine tıklayın.”
- **Deliği doldur:** imleç bir deliğin içindeyken delik vurgulanır; tıklama deliğin halkasıyla yeni bir alan yazar: üst nesnenin katmanı,
  rengi ve kalınlığı; öznitelikleri ve etiketi yok (parsel numarası kopyalanmaz). Delik yerinde kalır (QGIS'in Fill Ring'i): yeni alan
  deliği doldurur, üst nesnenin alanı değişmez. Yeni alanın köşe kotları deliğin halkasınınkilerdir.
- Hesaplar bindirme motorundandır: halkanın parçayla kesişimi halkanın kendisiyse içtedir; ekleme parçadan halkanın çıkarılmasıdır.

### 6. Kapsam dışı

- Topoloji açıkken komşuların biçim değiştirmeyle birlikte değişmesi (ADR 0160'ın değişiklikleri köşe, kenar bükümü, ekleme ve silmedir;
  bir sınır bölümünün yerine başka bir yol geçmesi onlardan biri değildir). Topoloji açıkken Biçim değiştir uyarır: “Topoloji açık:
  komşular Biçim değiştir'le değişmez.”
- Yaylı hat (hat düz parçalardır), birden çok nesnenin birlikte biçim değiştirmesi (ArcGIS'in paylaşılan sınırı).
- Deliği halka olarak çizmeden “içine tıkla” ile delik (Alan hesapla'nın İçine tıkla'sı dış sınırı verir).

## Adımlar

1. Sözleşme ve çekirdek: `EditOperation`'a beş işlem ve iki platformda adları; çekirdek `ops::holes` (ekleme denetimi ve sonucu, silme,
   doldurma) ve `ops::reshape_by` (alan: kırpma, cepler, retler; çizgi: kesişimler, değiştirilen bölüm); bağımsız başvuru: düz kenarlı
   durumlar kesin kesirlerle, yaylı durumlar elle türetilmiş (`scripts/fixtures/reshape_cases.py`). Bitti (5 Ekim): çekirdek bindirme
   motoruyla (kırpma `split_area`, cepler `FaceIndex` ve `union_areas`, birleşen delik `subtract_areas`), başvuru halka ve yol ekleyerek
   kesin kesirlerle; 36 durum iki platformda (`fixtures/reshape/v1/cases.json`).
2. Delik ekle, Deliği sil, Deliği doldur iki platformda; ortak izler.
3. Sürdür iki platformda; ortak iz.
4. Biçim değiştir iki platformda (alan ve çizgi); ortak izler; resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Çekirdek: başvurunun durumları iki platformda (Rust ve WASM aynı bitler); düz kenarlı sonuçlar köşe köşe 1e-9 m, alanlar 1e-6 m².
- Araçlar: ortak izler (`fixtures/interaction/v1`), iki platformda fareyle ve klavyeyle resimler.
