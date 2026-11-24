# ADR 9218: NCZ'nin sembol noktaları yönetmelik lejantının işaretiyle

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-18. Bulgu:
  [ADR 9216](9216-ncz-and-its-dxf-compared.md) §4.
- **Sahibin kararı** (24 Kasım 2026): “Yönetmelik lejantından”. Netcad'in sembol kodları KentOS'un kendi sembollerine
  eşlenir. Çizimler Mekânsal Planlar Yapım Yönetmeliği'nin lejantından alınır, Netcad'in kütüphanesinden değil.
- **Sözleşmeler:** değişmedi. Nokta, var olan `symbol` alanıyla bir kitaplık sembolüne bağlanıyor.
- **Stil çekirdeğinin bir kuralı değişti** (§2.1): masaüstü ve web aynı çekirdeği kullanıyor.

## §1 Bulgu

**NCZ'deki durum:** imar planı NCZ'sinde sembol noktaları yalnız Netcad kodunu taşıyor (`"Sembol": "S84"`). Çizimleri
dosyada değil, Netcad'in kütüphanesinde. KentOS bu noktaları düz nokta olarak çiziyordu.

**Kodların anlamı.** Sivas planındaki 52 kodun her biri neredeyse yalnız tek tür kullanım alanının içinde duruyor:

| Kod | İçinde durduğu alan |
|---|---|
| S75 | 63 noktanın 63'ü anaokulu alanında |
| S96 | 276 noktanın hepsi cami alanlarında (biri iki cami alanının içinde) |
| S152 | 543 noktanın 531'i trafo alanında |

Netcad'in aynı plandan aktardığı DXF'te bu kodlar adı kod olan bloklar. Çizimleri o kullanımın lejant işareti:

- üçgen içinde ANA, İLK, ORTA, LİSE;
- kare içinde TİCK, BHA, ASM;
- cami kubbesi, kitap.

**KentOS'un kütüphanesi.** Sistem kitaplığının MPYY sembolleri (uygulama imar planı) aynı işaretleri alan
sembollerinin ağırlık merkezi işareti (`centroidMarker`) olarak taşıyor. 190 alan sembolünde var, aynı gösterimle.
Ticaret alanının işareti `'T' || [Kademe]` ile T1 ve T2'yi de yazıyor.

## §2 Karar

### §2.1 Noktadaki alan sembolü ağırlık merkezi işaretini çizer

**Eski kural:** stil çekirdeğinde bir noktaya verilen alan sembolü hiçbir şey çizmiyordu.

**Yeni kural:** nokta, temsil ettiği alanın kendi noktasıdır. Alan sembolü noktada ağırlık merkezi işaretlerini o
noktaya çizer, dolgularını çizmez. Alanda aynı sembol eskisi gibi hem doldurur hem işaretler.

**Uygulama:**

- Değişiklik `kentos_style_core::style::compile::layer_on` içinde.
- Web derleyiciyi aynı çekirdekten (`styleCompile` işlemi) çağırıyor. `apps/web/src/style/compile.ts`'nin açıklaması da
  güncellendi.

**Dondurulmuş stil yanıtları** (`fixtures/style/v1/cases.json`, 48 katman ve 160 sembol vakası):

- Yalnız bir vaka değişti: 18. katman. Bir MPYY sembolü (koruma alanının kare, şemsiye ve “K/P” işareti) o katmanda
  bir noktaya da verilmiş.
- Eskiden noktada bir şey çizilmiyordu, şimdi üç işaret o noktada çiziliyor (topluluk 1 → 4, sayı 30 → 45).
- Dolgunun 30 sayısı aynı kaldı. Vaka bu yüzden yalnız o satırda yeniden yazıldı. Bütün vakaları baştan kaydetmek
  rastgele sembolleri de yeniden üretirdi.

### §2.2 Kod → lejant işareti tablosu

NCZ okuyucusu (`legend_mark`) 47 kodu MPYY uygulama imar planı alan sembollerine eşler. Okuyucu noktaya sembolün
kimliğini yazar. Ticaret alanının T1 ve T2'si için `Kademe` niteliğini de ekler. Açılış raporu kaç noktanın lejant
işaretiyle çizildiğini ve karşılığı olmayan kodları söyler.

**Bilerek dışarıda bırakılanlar:**

- **S101** (2 141 nokta, parklarda): Netcad'in bloğu boş, Netcad orada hiçbir şey çizmiyor. MPYY'nin park işareti “PARK”
  yazısı; 2 141 parka yazı koymak Netcad'in gösterdiğine eklemek olurdu.
- **S180, S181, S185** (özel sosyal, kültürel ve spor tesislerinin “Ö” işaretleri): lejantta satırları yok.
- **S15** (tek nokta): anlamı kesinleşmedi.

## §3 Sonuç (SIVAS_UIP_2025_06_HAZIRAN.NCZ)

- Lejant işaretiyle çizilen: 4 533 sembol noktası, 47 koddan.
- Yalnız kodunu taşımaya devam eden: 2 166 nokta, 5 kod.

## §4 Kanıt

**Sınamalar:**

- `kentos-style-core`: `a_fill_symbol_on_a_point_draws_its_centroid_marks_there`.
  - Noktada dolgu yok, işaret noktanın tam yerinde.
  - Alanda dolgu ve işaret var.
  - Çekirdeğin bütün sınamaları geçti: 76 birim, 9 bütünleşik.
- `kentos-ncz`: `the_legend_marks_are_the_system_librarys_area_symbols`.
  - Tablodaki 47 kimliğin her biri sistem kitaplığında.
  - Her biri ağırlık merkezi işareti olan bir alan sembolü.
  - T1/T2'nin işareti `Kademe`'yi okuyor.
  - Okuyucunun 26 sınaması geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm18-sembol/`, Xvfb):

- Plan açıldı, (584 405; 4 396 610) çevresine yakınlaşıldı.
- Önce ticaret alanında düz bir nokta, sonra kare içinde “T” (`once-sonra-ticaret.png`).
- Farklı pikseller yalnız işaretin kutusunda.

## Uygulama

- `crates/shared/style-core/src/style/compile.rs` (`layer_on`, `compile_symbol`'ün açıklaması);
  `crates/shared/style-core/src/style/tests.rs`.
- `fixtures/style/v1/cases.json` (18. katman vakası).
- `apps/web/src/style/compile.ts` (açıklama).
- `crates/shared/ncz/src/emit.rs` (`legend_mark`, sembol noktası, rapor notu, sınama).
