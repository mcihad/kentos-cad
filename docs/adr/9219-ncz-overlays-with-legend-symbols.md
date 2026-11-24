# ADR 9219: NCZ'nin bindirme katmanları yönetmelik lejantının sembolüyle; nesnenin saydamlığı kendi sembolünde

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-18. Bulgu:
  [ADR 9216](9216-ncz-and-its-dxf-compared.md) §4, §4½.
- **Sahibin kararları** (24 Kasım 2026):
  - “Evet, çöz”: NCZ'nin çizgi tipi ve desen kayıtları çözülür.
  - “Bindirmeler MPYY, alanlar Netcad”: koruma kuşakları, sitler, tescilli yapılar ve plan hatları Mekânsal Planlar
    Yapım Yönetmeliği lejantının sembolüyle çizilir. Plan alanları (PL_*) Netcad'in renginde kalır.
  - “Yarı saydam”: bindirme alanları %60 saydam çizilir, altındaki çizgiler görünür. Bu, nesnenin kendi saydamlık
    alanıyla yapılır; sembole dokunulmaz.
- **Sözleşmeler:** değişmedi. Nesne var olan `symbol` ve `transparency` alanlarını taşıyor.
- **Stil çekirdeği:** bir işlev eklendi (`style::seen_through`), masaüstünün katman programı onu çağırıyor.
- **Web:** web duruyor ([ADR 0228](0228-web-pause-desktop-and-cloud-first.md)), tarayıcının işlem tablosuna bir şey
  eklenmedi. Web'de gerekenler TODOS.md §0.2'de.

## §1 Bulgu: katmanın stil kaydı

NCZ'nin `LEX.ST2` bloğu her katman için 256 baytlık bir kayıt tutuyor (ADR 9216 §4½):

- 5. bayt: çizgi tipinin numarası;
- 17. bayt (19. bayt 1 iken): katmanın sembol numarası.

Okuyucu bunları artık okuyor (`format::layer_style`; katman kodu kayıtları renkler gibi sıralıyor). İki sayı da
Netcad'in kütüphanesine numara: çizimi NCZ'de yok. Anlamları Sivas planında onları kullanan katmanlardan okundu, katman
adı ve rengi ikisini de doğruluyor.

**Sembol numarası → alan sembolü (12):**

| Numara | Katman | MPYY uygulama imar planı sembolü |
|---|---|---|
| 35, 36, 37 | KA_1DER_ARK_SIT, KA_2DER…, KA_3DER… | 1., 2., 3. derece arkeolojik sit alanı |
| 42 | KA_TARIHI_SIT | tarihi sit alanı |
| 52 | TESCILLI_ANIT_YAPI | tescilli anıt yapı |
| 53 | TESCILLI_BINA | tescilli bina |
| 55 | SNR_SEG | sit etkileşim geçiş alanı sınırı |
| 66 | KST_KARAYOLU_KORUMA | karayolları yol kenarı koruma kuşağı |
| 67 | KST_BORU_HATTI_KORU | boru hattı koruma kuşağı |
| 68 | KST_SU_KANAL_KORUMA | su kanalları koruma kuşağı |
| 71 | KST_DEMIRYOLU_KORU | demiryolları koruma kuşağı |
| 74 | KST_ENERJI_NAKIL_HAT | enerji nakil hattı koruma kuşağı |

**Çizgi tipi → çizgi sembolü (11):**

| Numara | Katman | MPYY sembolü |
|---|---|---|
| 11 | SNR_YAPIYAK | yapı yaklaşma sınırı |
| 15 | SNR_PLANONAMA_TUM, SNR_PLANONAMA_ETAP | plan onama sınırı (Netcad ikisini aynı çizgi tipiyle çiziyor) |
| 19 | HAT_DOGALGAZ | doğalgaz boru hattı |
| 21 | HAT_ATIKSU | atık su ana kollektörü |
| 23 | HAT_BISIKLET | bisiklet yolu |
| 29 | SNR_OZEL_PROJE_ALANI | özel proje alanı sınırı |
| 32 | SNR_GOB | gecekondu önleme bölgesi sınırı |
| 40 | HAT_IFRAZ | ifraz hattı |
| 41 | HAT_KADEME, HAT_KITLE | kademe hattı |
| 42 | HAT_ANA_ICME_SUYU | içme suyu ana iletim hattı |
| 45 | SNR_RISKLI_ALAN | riskli alan sınırı (uygulama planı lejantında satırı yok; nazım planınki) |

**Çizgi tipi → alan sembolü (1):** 13, TESCILLI_PARSEL → tescilli parsel. Katmanın sembol numarası yok. Netcad
tescilli parseli tarıyor: DXF'inde 188 parsel, 188 tarama.

Plan alanlarının sembol numarası alanın lejant kodudur (PL_CAMI 96). O kod burada değil, ADR 9218'deki sembol
noktalarında kullanılır.

## §2 Karar

### §2.1 Bindirme katmanının sembolü

Okuyucu (`overlay_symbol`) nesneye katmanının kaydından bir lejant sembolü verir:

- **Alan** (çokgen, üçgen): katmanın sembol numarası alan tablosundaysa o alan sembolü. Değilse çizgi tipinin alan
  sembolü (tescilli parsel). O da yoksa ve çizgi tipi çizgi tablosundaysa o çizgi sembolü; alanın kenarları çizilir
  (SNR_OZEL_PROJE_ALANI, plan onama sınırı).
- **Çizgi:** çizgi tipinin çizgi sembolü.
- **Rapor:** açılış raporunun “Katman sembolü” notu kaç nesnenin böyle çizildiğini söyler.

Çizimler KentOS'un sistem kitaplığındaki MPYY sembolleri. Netcad'in kütüphanesinden bir şey alınmadı.

### §2.2 Bindirme alanı %60 saydam

**Sorun:** MPYY'nin kuşak sembolü dolgulu (demiryolu kuşağı #F57A7A). Opak çizilince demiryolunun ve kuşağın altında
kalan plan hatlarının üstünü örttü. Netcad kuşağı dolgusuz çiziyor.

**Karar:** okuyucu, alan sembolü verdiği nesnenin saydamlığını 0,6 yapar (`OVERLAY_TRANSPARENCY`). Nesne saydamlığı
0–0,9 arasındadır ([ADR 0260](0260-object-look.md) §3).

### §2.3 Nesnenin saydamlığı kendi sembolünde

**Eski kural:** kendi sembolü olan nesnenin (`MODE_OWN`) saydamlığı yok sayılıyordu. Saydamlık yalnız basit görünümde,
yani renk ve kalınlıkta işliyordu.

**Yeni kural:**

- Saydam nesnenin sembolü programa kitaplıktaki sembolün saydam bir kopyası olarak girer, kendi kimliğiyle
  (`<kimlik>#saydam<t>`). Kitaplıktaki sembol öbür nesneler için olduğu gibi kalır.
- Kopyada her katmanın opaklığı (yoksa 1) `1 − t` ile çarpılır.
- İşaret taşıyan katman (çizgi boyunca, ağırlık merkezinde, desende) bunu işaretin kendi katmanlarına geçirir.
  Derleyici bir işareti taşıyıcısının opaklığıyla değil, kendi katmanlarınınkiyle çiziyor.
- Sonuç: bütün sembol aynı ölçüde saydam, nesne bir bütün olarak saydam. Dolgu, kesik sınır, çarpılar, yazılar ve
  ortadaki işaret buna dahil.
- `t` 0 ile 0,9 arasına çekilir; sayı olmayan değer 0 sayılır.

**Uygulama:**

- Hesap stil çekirdeğinde (`kentos_style_core::style::seen_through`). Masaüstünün katman programı
  (`program.rs`, `own_seen_through`) onu çağırır.
- Çekirdekte durmasının sebebi sonra gelecek istemciler (ADR 0228 §3, §4). Tarayıcıya işlem olarak açılması web'e
  dönüşte yapılır.

**Görünümün Saydamlık anahtarı.** `graphics.transparency` ([ADR 0195](0195-view-modes.md)) kapalıyken her dolgu gibi
kuşak da opak çizilir; bu kural değişmedi. Sahibin ayarında anahtar kapalı. Kuşağın altını görmek için Görünüm sekmesi
→ Görünüm kipleri → Saydamlık açılır.

## §3 Sonuç (SIVAS_UIP_2025_06_HAZIRAN.NCZ)

Lejant sembolüyle çizilen nesne: **69 142.**

**Yarı saydam alanlar (471):**

| Ne | Sayı |
|---|---|
| tescilli parsel | 200 |
| tescilli bina | 148 |
| tescilli anıt yapı | 71 |
| koruma kuşakları: enerji nakil hattı 22, demiryolu 9, karayolu 4, boru hattı 2, su kanalı 1 | 38 |
| arkeolojik ve tarihi sitler: 1. derece 8, 3. derece 3, 2. derece 1, tarihi 1 | 13 |
| sit etkileşim geçiş alanı | 1 |

**Çizgiler (68 671):**

| Ne | Sayı |
|---|---|
| doğalgaz boru hattı | 41 609 |
| yapı yaklaşma sınırı | 20 251 |
| plan onama sınırı (3 082 çizgi, 45 alanın kenarı) | 3 127 |
| kademe hattı (HAT_KADEME 1 974, HAT_KITLE 232: Netcad ikisini aynı çizgi tipiyle çiziyor) | 2 206 |
| ifraz hattı | 1 366 |
| atık su ana kollektörü | 78 |
| bisiklet yolu | 20 |
| özel proje alanı sınırı (alanların kenarı) | 9 |
| içme suyu ana iletim hattı | 2 |
| riskli alan sınırı | 2 |
| gecekondu önleme bölgesi sınırı | 1 |

**Bilerek eşlenmeyenler:**

- **31, KA_KORUNMA_ALANI:** Netcad'in “daire deseni” desen değil, sınır boyunca iri noktalar. DXF'te 255 noktanın
  her biri 16 eş merkezli daire (yarıçap 1,125–1,5 m). 4 080 dairenin hepsinin merkezi sınırın üstünde.
  - Lejantta böyle noktalı sınırı olan birkaç satır var: sit alanları, kesin korunacak hassas alan… Katman adı hangisi
    olduğunu söylemiyor.
  - Yanlış lejant anlamı, düz çizgiden kötü. Sahibin kararı (24 Kasım): “Düz çizgi kalsın”; anlamı belli olunca
    eşlenir.
- **4:** demiryolu kuşağı ile SNR_REZERV_YAPI aynı çizgi tipini kullanıyor. Kuşak alan sembolünü alıyor; rezerv yapı
  düz kaldı. Bu numarayı rezerv yapıya vermek, başka planlarda bu çizgi tipini kullanan her çizgiye o anlamı verirdi.
- **22** (JEO_SNR_FORMASYON): lejantta karşılığı yok.
- **6** (PL_TGB): plan alanı; Netcad'in renginde.

## §4 Kanıt

**Sınamalar:**

- `kentos-ncz` (26 sınama geçti): `the_legend_marks_are_the_system_librarys_area_symbols`. Bindirme tablolarının 13
  alan ve 11 çizgi kimliğinin hepsi sistem kitaplığında; alan kimlikleri alan, çizgi kimlikleri çizgi sembolü. Bir
  çizgi tipi ya alanın ya çizginin.
- `kentos-style-core` (79 birim, 9 bütünleşik sınama geçti), `seen_through` için üç sınama:
  - her katman ve her işaretin her katmanı saydamlaşıyor, işaretin taşıyıcısına dokunulmuyor;
  - aralık dışındaki saydamlık en yakın uca çekiliyor;
  - sembol olmayan girdi olduğu gibi dönüyor.
- `kentos-native-style` (55 sınama): `an_objects_transparency_sees_its_own_symbol_through`.
  - Demiryolu kuşağı sembolünün 8 opaklığının (dolgu, kesik sınır, sınırın çarpısı ve kodu, ortadaki işaretin iki
    karesi, çarpısı ve kodu) her biri × 0,4.
  - Kimlik kendine ait. Saydamlık yokken kitaplıktaki sembol olduğu gibi.

**Gerçek pencere** (`.run/gui-evidence/render/rm18-stil/`, Xvfb, sahibin ayarlarının Saydamlık'ı açık bir kopyasıyla):

- Plan açıldı, demiryolu kuşağına yakınlaşıldı.
- Önce kuşaklarda yalnız sınır çizgileri vardı. Sonra MPYY sembolleriyle çiziliyorlar: yarı saydam dolgu, kesik
  kırmızı sınır, çarpılar, DKK ve YKK yazıları, ortada DKK işareti (`once-sonra-demiryolu.png`).
  - Dolgu ekranda 247/198/199: #F57A7A'nın %40'ı, kâğıt renginin üstünde.
- Kuşağın altındaki mor ve yeşil hatlar ve parsel çizgileri görünüyor. Mor hattın piksel sayısı 886 → 856.
- Değişen piksel: 1 296 000'in 127 102'si.
- Sahibin kendi ayarıyla (Saydamlık kapalı) aynı görünümde kuşak opak #F57A7A. Bu, ADR 0195'in kuralı.
- İkinci tur (`.run/gui-evidence/render/rm18-stil2/once-sonra-tescil.png`), (586 611; 4 401 186) çevresi:
  - Görünümün çoğunu kaplayan tescilli parsel yarı saydam taramayla çiziliyor. Netcad'in DXF'inde bu parsel 377 378
    m² ve Netcad onu da tarıyor.
  - Plan onama sınırı, mor düz çizgi yerine lejantın mavi halkalarıyla çiziliyor.

## Uygulama

- `crates/shared/ncz/src/format.rs`: `Header::layer_styles`, `layer_style`.
- `crates/shared/ncz/src/emit.rs`: `overlay_symbol`, `overlay_area`, `overlay_line`, `OVERLAY_TRANSPARENCY`, rapor
  notu, sınama.
- `crates/shared/style-core/src/style/seen_through.rs` (yeni).
- `crates/native/style/src/program.rs`: `own_seen_through`, `seen_through`.
- `crates/native/style/tests/all/own_symbol_seen_through.rs` (yeni).
- TODOS.md §0.2: web'e ertelenenler.
