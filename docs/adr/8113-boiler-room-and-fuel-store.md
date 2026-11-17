# ADR 8113: Kazan dairesi ve yakıt deposu (BYKHY 54/2, 54/4, 54/5, 56/1)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (B15'in yangın kuralları; konut
  binalarında da). Her madde için bağımsız durumlar ve gerçek çizimde Denetim (§4).
- **Bağlam:** ADR [8081](8081-fire-compartments.md) (mahaller arası ayrımlar, beyan edilen sınıflar; 24/2 kazan dairesinin
  kompartıman duvarı, EI 60), [8078](8078-regulation-rule-set-by-application-date.md) §8 (kapıların sınıfının okunması:
  dakika, S, C), plan K10.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi (`cad.arch.audit`'in açıklaması beş yeni
  kodu sayar). **Kural paketi:** mimari-mevzuat 0.26.0 (`kazan`). **Olgular:** bina olgularının mahallerine `net`.

## §1. Araştırma (18 Kasım 2026)

Binaların Yangından Korunması Hakkında Yönetmelik, ADR 8107 §1'deki aynı derlenmiş metin (SHA-256 `d82d06d2…a7d9`),
Altıncı Kısmın "Bina bölümleri ve tesisler"i: 53–56. Kural setine yazılan metin `pdftotext` çıktısında aynen geçer.

| Fıkra | Ne diyor | KentOS kararı |
|---|---|---|
| 54/2 (ilk metin) | Kazan dairesi binanın diğer kısımlarından en az 120 dakika dayanıklı bölmelerle ayrılmış, merkezi bir yerde ve bütün hâlinde; dilatasyon geçemez | Komşu her mahalle ayrımı: duvar ve pencere en az EI 120 (bütünlük ve yalıtım), duvarsız yer ve eşiksiz boşluk yok. Kapılar 54/5'in. Döşemeler, merkezi yer, dilatasyon çizimden okunmaz (satırın notu) |
| 54/4 (değişik 9.9.2009) | Kapısı kaçış merdivenine ya da genel kullanım merdivenlerine doğrudan açılmaz, mutlaka bir ortak hol ya da koridora açılır | Merdivene açılan ve hol ya da koridor olmayan mahale açılan kapı kaldı; dışarı açılan kapı uzman incelemesi; holün ortaklığı olgularda yok |
| 54/5 (ilk metin) | 50–350 kW'ta en az bir kapı; döşeme alanı 100 m²'nin ya da ısıl kapasitesi 350 kW'ın üzerinde en az 2 çıkış kapısı, olabildiğince ters yönde; kapılar en az 90 dakika, duman sızdırmaz, kendiliğinden kapanır | Döşeme alanı net (iç yüzlerden; olgulara `net`). Isıl kapasite çizimde yok: 100 m²'yi aşmayan tek kapılı kazan dairesi uzman incelemesi (tesisat projesi); kapıların yönü idarenin |
| 55 | Doğalgaz ve LPG'li kazan dairesi: sayaç dışarıda, vanalar, algılayıcı, LPG'li kazan dairesi bodrumda yapılamaz | Yakıt türü çizimde yok: sıradaki iş |
| 56/1 (ilk metin) | Yakıt deposu ile kazan dairesi 120 dakika dayanıklı bölmeyle ayrılır; havalandırma, havuzlama | Aralarındaki ayrım 54/2 gibi, kapısı da en az 120 dakika; havalandırma ve havuzlama çizimden okunmaz |
| 56/7 | Kömürlük kazan dairesine bitişik | Kömürlük yakıt deposu sayılır |

## §2. Karar

- **K211 — Kazan dairesi ve yakıt deposu:** adı Kazan, Kalorifer ya da Isı merkezi olan mahal kazan dairesi; adı Yakıt,
  Akaryakıt, Mazot ya da Kömür olan mahal yakıt deposu. Kazan dairesinin kendi yakıt deposuna açılan kapısı çıkış kapısı
  değildir (54/4 ve 54/5 okumaz), o ayrım 56/1'indir.
- **K212 — 54/2:** komşu her mahalle (yakıt deposu dışında) ayrımın duvar ve pencereleri en az EI 120 (`E` ve `I`, 120
  dakika); duvarsız yer (5 cm'den uzun) ve eşiksiz boşluk kaldı; beyansız ya da okunamayan eleman eksik veri
  (`bykhy_boiler_room_separation`). Bitişik mahali olmayan kazan dairesinde madde kapsam dışı.
- **K213 — 54/4 ve 54/5:** kapının öbür yanı merdiven mahaliyse ya da hol ve koridor olmayan bir mahalse kaldı, dışarısıysa
  uzman incelemesi (`bykhy_boiler_room_door`); kapısı çizilmemiş kazan dairesi eksik veri. Net alanı 100 m²'yi aşıp iki
  çıkış kapısı olmayan kaldı, aşmayıp tek kapılı olan uzman incelemesi (`bykhy_boiler_room_exits`); her çıkış kapısı en az
  90 dakika, `S` ve `C` (`bykhy_boiler_room_door_rating`).
- **K214 — 56/1:** yakıt deposu ile kazan dairesi arasındaki ayrımın duvarı, penceresi ve kapısı en az EI 120
  (`bykhy_fuel_store_separation`).
- **K215 — Olgu:** bina olgularının mahallerinde `net` (iç yüzler arası alan, m²; sınırı bulunamayanda yok).

## §3. Uygulama

- Olgular: `crates/shared/extensions/src/audit_facts/building.rs` (`SpaceFacts.net`), `crates/native/processing/src/
  audit_facts/building.rs` (`SpaceSearch::net_area`); SDK şeması; `fixtures/audit-facts/v1/mimari.json` (fark yalnız
  yeni alan).
- Kural paketi: `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kazan.py`, kural sırası, Denetim yüzü `kazan`,
  `manifest.json`, `data/surumler.json` (54/2, 54/4, 54/5, 56/1), 0.26.0, CHANGELOG.

## §4. Sınamalar

- **Bağımsız durumlar** (`fixtures/standards/v1/kazan.json`, 26 durum; `scripts/fixtures/kazan_cases.py`): iyi kazan
  dairesi (54/5 uzman incelemesi); kazansız; EI 60, beyansız, REI 120, yalnız E 120 duvar; duvarsız yer, E 30 iç pencere,
  eşiksiz boşluk; merdivene, otoparka ve dışarı açılan kapı; kapısız; 120 m² tek ve iki kapı, tam 100 m²; EI2 60, EI2 90
  (S ve C yok) ve beyansız kapı; Kalorifer dairesi, Isı merkezi; yakıt deposu EI 120, EI 90 duvar ve EI2 90 kapı; 1.6.2008
  başvurusu (54/4 uzman incelemesi); tarihsiz.
- **Gerçek çizimde Denetim** (`boiler_room_tests`): komutlarla zemin katta Kazan dairesi, Merdiven evi ve Koridor; kapı
  merdivene açılırken 54/4 kaldı, 54/2 ve 54/5 eksik veri; kapı koridora açılıp duvar EI 120 ve kapı EI2 90-S200C5
  beyan edilince 54/2 ve 54/4 geçti, 54/5 uzman incelemesi. `regulation_dates_tests`'in konut çiziminin bodrumundaki
  duvarsız kazan dairesi: 54/2 kaldı, 54/4 ve 54/5 eksik veri, 28 madde kapsam dışı.
- **Resim** (`.run/gui-evidence/kazan-dairesi-8113/`): merdivene açılan kapının bulgusu Denetim'de (açık ve koyu).

## §5. Arayüz

Değişmedi: Denetim'in Mevzuat grubunda yeni bulgular, kural setinde dört yeni madde, Eklenti denetiminde yeni yüz (Kazan
dairesi ve yakıt deposu).

## §6. Sıradaki adımlar

- Kazan dairesinin döşeme ve tavanının sınıfı (olgularda yok), yakıt türü (55: LPG'li kazan dairesi bodrumda olamaz).
- Isıl kapasitenin projede beyanı (54/5'in uzman incelemesi kalkar).
- 57 (mutfaklar), 58 (bacalar: mahyadan 0,80 m Mimari denetimde), 59 (sobalar), 60 (otoparkların açıklığı).
