# ADR 0190: Orta hat

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-27`'nin ardından `CAD-28`; madde tek parçada biter
  (sahibin 5 Ekim kararı) ve sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler
  Netcad'in Orta Hat Çiz'i (Netcad 7, Yapınet) ve ArcGIS'in Collapse Dual Lines To Centerline'ıdır.
- **Bağlam belgesi:** TODOS.md `CAD-28`; ADR 0188 (güzergâh: `route_of`, yolun noktası), ADR 0161 (Zincir: tıklanan çizginin bağlı
  zinciri, `ops::join::chain`), ADR 0047 (Ötele).

## Bağlam

Yol kenarları, dere kıyıları ya da bir kanalın iki yanı çizilidir; ekseni elle, iki yandan ölçerek çizilir.

## Karar

### 1. Araç

**Orta hat** (Çizim ▾'nın yapı çizgilerinin yanında; iki proje türünde). Birinci kenara, sonra ikinci kenara tıklanır; orta hat soluk
görünür, Enter, Uygula ya da sağ tık tek adımda (“Orta hat”) etkin katmana çoklu çizgi olarak yazar ve araç sonraki çifti bekler. Esc ve
Ctrl+Z son seçilen kenarı bırakır; Esc kenar yokken çıkar.

- Kenar bir güzergâhtır (ADR 0188 §1; çizgi, çoklu çizgi, yay, eğri, elips): açık yol. Kapalı yol ve yolu olmayan nesne söylenir.
- **Zincir** (Z; açık): tıklanan çizginin uç uca bağlı zinciri bir kenardır (Birleştir'in Zinciri gibi; ADR 0161 §3); kapalıyken
  yalnız tıklanan nesne.
- İkinci kenar birincinin yönüne çevrilir: uçları ters eşleşmeye daha yakınsa ters yürünür. İki kenarın başları ve sonları karşılıklıdır.

### 2. Orta hat

- **Eşleşen kenarlar:** iki kenarın kenar sayısı aynıysa ve karşılıklı kenarlar düz ve paralel (doğrultuları 10⁻⁹ radyandan yakın) ya
  da aynı merkezli yaylarsa (merkezleri 1 mm'den yakın), orta hat kenar kenar kesindir: köşeleri karşılıklı köşelerin ortası, yayları iki
  yayın ortalama yarıçaplı, aynı açılı yayı (bükümü aynı). Ötele ve Paralel çizgi'nin kenarları böyledir.
- **Örnekleme:** öbür durumda iki kenar uzunluklarıyla orantılı eşlenir (birincinin s/L'si ikincinin aynı oranı); orta hat bu eşlenen
  noktaların ortalarından geçen çoklu çizgidir: köşeleri iki kenarın köşelerinin oranlarında ve uzun kenar boyunca **Adım** (B; 1 m,
  projenin uzunluk biriminde yazılır) aralıklarla; aynı orana 10⁻¹² içinde düşen köşeler bir kez. Eğri ve elipsin köşesi yoktur (kirişleri
  kendi köşesi değildir): onları Adım örnekler, eşleşmeye girmezler. Uzun kenar Adım'ın 100 000 katından uzunsa söylenir (“adımı
  büyütün”).

### 3. Çekirdek

`ops::centerline`: iki yolun orta hattı (yöntemi, köşeleri ve bükümleri); web'e işlem tablosundan. Ortak durumlar bağımsız Python
başvurusundan (`scripts/fixtures/centerline_cases.py`).

### 4. Komut

`cad.entities.create`'in `centerline` işlemi, adım “Orta hat”; orta hat hesaplandığı gibi çoklu çizgi olarak verilir.

## Kapsam dışı

Kapalı kenar çiftleri (çevre yolu halkaları), kenarları birbirini yalnız bir kesimde izleyen çiftlerin kırpılması, ikiden çok kenar,
kavşaklarda eksenlerin birleştirilmesi.

## Uygulama

- Çekirdek: `crates/shared/geometry-core/src/ops/centerline.rs` (`centerline`, `Method`, işlem `centerline`); güzergâhın yürüyüşüne
  (`tools::point_calc::Walk`) kenarları ve köşelerin uzunlukları eklendi (`edges`, `vertex_lengths`; eğride yok). Web'de
  `tools/constructions.ts`'in `centerline`'ı.
- Sözleşme: `CreateOperation::Centerline` (`centerline`), adı iki platformda “Orta hat”; katalog, TypeScript ve Python SDK'sı yeniden
  üretildi; ortak komut durumu `fixtures/commands/v1/cad.entities.create.json`'da (yaylı eksen).
- Araç: masaüstünde `kentos_interaction::centerline` (oturumun belleği `centerline_step`, `centerline_chain`), web'de
  `tools/centerlineTool.ts` (`centerlineOptions`); Zincir Birleştir'in çekirdek yoluyla (`joinChain`, `joinEntities`; uç boşluğu toleransı
  Birleştir'inki), görünen çizgiler, yaylar ve çoklu çizgiler arasında; kilitli katmandaki kenar okunur (yalnız eksen yazılır). Çizim ▾'da
  Paralel çizgi'nin ardında, iki proje türünde; ikon sahip uyurken seçenek sayfasının D'si (iki kıyı ve nokta-çizgi eksen).
- Resimler: masaüstü `apps/desktop/src/centerline_scenes.rs` (`orta-hat-*`), web `shots.mjs centerline`.

## Doğrulama

- Bağımsız başvuru: `python3 scripts/fixtures/centerline_cases.py --check` (mpmath ile 50 basamak, KentOS kodu olmadan; 11 durum: paralel
  çizgiler, ters çizilmiş ikinci kenar, farklı boylar, yolun düz ve yaylı kenarları, yaklaşan kıyı, çizgi ve yay, harita koordinatları,
  kapalı yol, yolu olmayan nesne, sıfır adım, çok nokta). Çekirdek doğal olarak (`tests/all/centerline.rs`) ve WASM'dan
  (`tools/centerline.wasm.test.ts`) aynı dosyayı geçer: köşeler 10⁻⁷ m, bükümler 10⁻¹² içinde.
- Ortak iz `fixtures/interaction/v1/centerline.json` (43 adım) iki platformda: yolu olmayan nesne ve boş yer, aynı kenar, yolun kenar kenar
  ekseni ve tek adım, Ctrl+Z'nin önce kenarı bırakması, derenin örneklenmesi ve Adım'ın reddi, Zincir kapalı ve açık, belleğin geri
  bırakılması.
- Komutun durumu: `python3 scripts/fixtures/create_command_cases.py --check`, iki platformun komut testleri.
