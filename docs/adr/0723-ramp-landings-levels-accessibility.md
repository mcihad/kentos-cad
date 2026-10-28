# ADR 0723: Rampada kendiliğinden ara sahanlık, kata bağlı başlangıç kotu, metraj ve erişilebilirlik profili

- **Durum:** kabul edildi (28 Ekim 2026); plan/architecture.md ARCH-STAIR-02'nin kalan alt maddeleri ve kabulü.
- **Bağlam:** ADR 0343 (rampa), ADR 0705 (eğimden rampa, korkuluk), ADR 0721 (kata bağlı kot), ADR 0713/0720
  (etkin kat), ADR 0722 (metrajın döşeme ve geçiş satırları), ADR 0703 (Mimari denetim), ADR 0718 (kat planı
  paftaları), ADR 0265 (yeni şemanın kuralı), ADR 0622 §9 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0705 rampayı eğimden tasarladı ve korkuluk ekledi. Açık kalanlar şunlardı:

- Uzun düz kolda kendiliğinden ara sahanlık yoktu.
- Başlangıç kotu katı izlemiyordu. Kat yükselince rampanın kot yazıları eski kalıyor, araç da etkin kattaki rampayı
  0 kotundan başlatıyordu.
- Metraj ve pafta kabulü yapılmamıştı.
- Yerel erişilebilirlik kuralları ayrı, sürümlü bir profil olarak denetlenmiyordu.

Araştırma (yerelde doğrulanmadı; kaynaklar birbirini tam tutmuyor):

- Planlı Alanlar İmar Yönetmeliği md. 30: binalarda ve girişlerinde engellilerin erişimi için TS 9111'e uyulur;
  rampaların genişlikleri ve sahanlıkları TS 9111'e uygun yapılır
  ([özet](https://hukukcularevi.com/yapilarda-erisilebilirlik-imar-ek-madde-1-denetim/)).
- TS 9111 özetleri ([MEB](https://efeler.meb.gov.tr/meb_iys_dosyalar/2018_10/24145413_ERYYEBYLYRLYK_WORD.docx),
  [Ankara Üni. açık ders](https://acikders.ankara.edu.tr/mod/resource/view.php?id=6217),
  [Tamer Heper, Posta](https://www.posta.com.tr/yazarlar/tamer-heper/engelli-rampasi-kurallari-2917690)):
  - Yükseklik farkına göre en çok eğim: 15 cm'ye kadar %8 (1:12), 16–50 cm %7, 51–100 cm %6, 100 cm üstü %5.
  - 10 m'den uzun rampada eğim %6'yı aşmaz.
  - Yön değiştiren sahanlık en az 150 × 150 cm.
  - Genişlik için kaynaklar 90 cm ile 120 cm arasında ayrılıyor; bazıları kamu girişlerinde %5 der.
- Revit ve Archicad rampayı “Max Slope” / “Max Run Length” gibi tür ayarlarıyla sınar, aşan rampayı uyarıyla çizer;
  onay vermez.

## Karar

1. **Ara sahanlık.** `cad.ramps.create`'in isteğe bağlı `maxRun`'ı en uzun eğimli koldur (metre, 0 < m ≤ 100).
   - Eğimli kol bu uzunluğa varınca yürüme çizgisine düz bir köşe eklenir; köşe, kolun başından `maxRun` artı yarım
     sahanlık ötededir.
   - O köşede `landing` uzunluğunda sahanlık olur (sahanlık her iç köşededir, ADR 0343).
   - `fit` ile her sahanlıktan sonra son nokta yeniden taşınır; son kol kalan kot farkıyla biter.
   - Çekirdekte `geom::arch::ramp_with_landings` yapar.
   - Rampa aracında “En uzun kol (L)” ayarı var; boş bırakılırsa eski davranış sürer.
2. **Kata bağlı başlangıç.** Rampanın isteğe bağlı `baseOffset`'i başlangıç kotunun katının kotuna göre farkıdır.
   - Başlangıç kotu `base` = katın kotu + fark. Belge, kat kotu ya da fark değişince `base`'i aynı adımda yazar. Bunu
     ADR 0721'in kot izleyicisi (`level_marks.rs`) yapar. Ayar değişikliği, geri alma ve yinelemeden sonra da
     yazılır; kendisi bir geri alma adımı değildir. Etiketlerden önce koşar.
   - Fark sonlu ve en çok ±1000 m olmalı. Katı olmayan rampada fark `invalid_ramp`'tir. Projede olmayan kat
     `level_not_found` olur.
   - `base` ile `baseOffset` birlikte verilmez (`invalid_ramp`).
   - `cad.arch.edit`'te `baseOffset` bağlar. `null` bağı kaldırır; kot yerinde kalır. `base` yazmak bağı kaldırır.
     Bağlı rampanın katı kaldırılamaz. Kat değişince başlangıç yeni katı izler.
   - KCAD opt-in şema 124: yalnız bağlı rampası olan dosya yeni şemayı yazar.
   - Rampa aracının Başlangıç kotu (Z) ayarı:
     - boş ya da “kat”: etkin katın kotu (öntanımlı);
     - “kat+0.15” ya da “kat-0.15”: kattan fark;
     - sayı: rampanın kendi kotu.

     Ayarlarda “2.910 m · katın −0.150 m” gibi yazılır. Öznitelikler'de “Kata göre” satırı var; boş bırakılırsa bağ
     kalkar.
3. **Metraj.** `cad.spaces.quantities` rampa satırları (`ramps`) verir. Satırda şunlar var:
   - kat ve genişlik;
   - kot farkı ve çizildiği eğim (kot farkının büyüklüğü ÷ eğimli uzunluk; eğimli kolu yoksa yok);
   - eğimli uzunluk (orta çizgi eksi sahanlıklar), orta çizginin uzunluğu ve sahanlık sayısı;
   - plan alanı (dönüşlerde gönyeli şeridin halkası);
   - başlangıç ve bitiş kotu, kata bağlıysa fark;
   - korkuluk çizgilerinin uzunluğu ve kural.

   Toplamlara `rampArea` ve `rampRailingLength` eklenir. Kat, evre ve katman süzgeçleri mahallerinkidir; kullanım
   süzgeci rampaları almaz. Metraj penceresinde Rampalar sekmesi var. CSV açık sekmeyi, XLSX/ODS altı sayfayı yazar.
4. **Erişilebilirlik profili.** Sürümlü veri `catalog/v1/accessibility/erisilebilirlik-rampa-1.json`
   (`erisilebilirlik-rampa/1`): TS 9111 özetinin değerleri, kaynakları ve “mevzuat onayı değildir” notu.
   - `cad.arch.audit`'in `profile`'ı verilince yeni `accessibility` grubu rampaları sınar. Profil yoksa grup bir şey
     bulmaz, eski cevaplar aynı kalır. Katalogda olmayan profil `invalid_arch_audit`'tir.
   - `ramp_slope_over_profile`: eğim, kot farkının bandına göre fazla. Eşit eğim geçer.
   - `ramp_long_run_slope`: 10 m'den uzun bir eğimli kol %6'dan dik.
   - `ramp_landing_short`: sahanlık 1.50 m'den kısa. Düzelt `cad.arch.edit` ile sahanlığı 1.50 m yapar.
   - `ramp_turn_narrow`: dönen rampa 1.50 m'den dar. Düzelt genişliği 1.50 m yapar. Düz köşe dönüş sayılmaz.
   - Kilitli katmandaki rampanın düzeltmesi yoktur. Bulgular uyarıdır, yazmayı engellemez.
   - Denetim sekmesinde “Erişilebilirlik: Yok / TS 9111 özeti” seçimi var; masaüstünde öntanımlı TS 9111 özeti. API'de
     öntanımlı yok.
   - Genişliğin alt sınırı kaynaklarda ayrıldığı için profil 1'de yok.
5. **Planda eğim ve kabul.** Rampa planda okunun solunda, en uzun eğimli kolun ortasında çizildiği eğimi yazar
   (“%5.00”). Kot farkı ya da eğimli kolu olmayan rampada bu yazı yoktur. Bu yazı ekranda, paftada ve DXF'te aynı
   parçadandır.

## Doğrulama

- **Ortak durumlar** KentOS kodu olmadan bağımsız başvurulardan üretildi:
  - `cad.ramps.create.design.json`: 31 (9'u `maxRun`).
  - `cad.ramps.create.levels.json`: 27. Kat bağı, kat yükselince izleme, geri al/yinele, Değiştir ve bağ kaldırma,
    retler.
  - `cad.spaces.quantities.ramps.json`: 22, `ramp_quantity_cases.py` ve `ramp_geometry.py`'den.
  - `cad.arch.audit.accessibility.json`: 24, `ramp_access_cases.py`'den.
  - Çekirdek başvurusu `arch_cases.py`: 48 durum, eğim yazısıyla.
  - Eski 41 denetim, 48 düzenleme ve 25 metraj durumu değişmeden geçer.
- **KCAD:** `ramp-levels.kcad` ve iki kural bozuk örnek, önceki şema örneği; bilinmeyen şema örneği
  `schema-version-125`'e taşındı. Bağımsız Python okuyucusu 1289 dosyanın hepsini okudu. Kodek 228 test.
- **Takımlar:**
  - uygulama 242 + 67, alan 121, etkileşim 783 + 66;
  - komutlar 918, sözleşmeler 1061 + 33 + 10;
  - geometri çekirdeği 534 + 284, başsız 64, pafta çıktısı 16 + 48;
  - MCP, Python 374, masaüstü 1356;
  - clippy `-D warnings`.
- **DXF:** `arch_dxf_cases.py` 11 mimari nesne, eğim yazısıyla yeniden yazılan `mimari.dxf`.
- **Gerçek PDF** (`ramp_levels.rs`): `cad.ramps.create` (En uzun kol, kata bağlı, iki yan korkuluk) ile
  `cad.sheets.levels`'in 1. kat paftası yazıldı. poppler'ın `pdftotext`'iyle iki ölçüm:
  - ilk hâl: %5.00, +2.91 ve +4.11;
  - kat 3.06'dan 3.40 m'ye çıkınca: +3.25 ve +4.45; eski kot kalmaz.
- **Gerçek arayüz resimleri** (`.run/shots/arac-rampa-sahanlik-*`, iki tema, 1440×900 ve 1100×650):
  - Rampa aracında iki ara sahanlık ve Başlangıç kotu;
  - yazılan rampa;
  - kat yükselince izleyen kotlar ve Öznitelikler'in Kata göre satırı;
  - Metraj'ın Rampalar sekmesi;
  - Denetim'in Erişilebilirlik bulguları, Düzelt ve sonrası.

  Bunları yöneten masaüstü testi `ramp_scenes::the_ramp_tool_binds_lands_and_is_measured_and_audited`.

## Açık kalanlar

- Profilde genişlik ve korkuluk/küpeşte yüksekliği (kaynaklar ayrılıyor), kurumun kendi profil dosyası.
- Döner (yaylı) rampa, rampanın kesitte ve 3B'de gösterimi, rampa ile döşeme birleşimi.
- Merdivenin başlangıç kotunun da kata bağlanması (merdiven bugün yalnız katın yüksekliğini okur).
