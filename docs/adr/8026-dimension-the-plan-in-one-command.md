# 3026 — Planı tek komutla ölçülendirmek

Tarih: 2026-11-08. Durum: kabul (M26, ADV-16; plan S19).

## Sorun

- **Dört cephe için dört ayar.** Cephe ve dört yan ölçü dizisi (ADR 0729) kodda vardı, ama Duvar ölçüleri'nde her seferinde
  elle ayarlanıyordu:
  - Kaynak türü Cephe;
  - dış duvarların hepsi tek tek seçilir;
  - Yanlar Dört yan;
  - sıralar (Açıklık, Duvarlar, Akslar, Toplam) açılıp kapanır;
  - yer seçilir.
- **İç ölçüler zincir zincir.** İç ölçü zinciri (ADR 0711) her zincir için iki tıklamaydı. Kabul akışında (2+1, ADR 0776)
  yalnız güneyde bir dış zincir vardı; iç ölçü ve öbür cepheler yoktu (M0, W2'nin 6. fikri).
- **ADV-16:** "Tek komut planın dört cephesini ölçülendirir … MO kuralı: ön projede 3, uygulamada 4 zincir … İç ölçü:
  mahal başına 2+2 zincir."

## Araştırma (8 Kasım 2026)

- **Mimarlar Odası İstanbul Büyükkent Şubesi, Mimari Proje Çizim ve Sunuş Standartları** (2015; W4 §2.19):
  - "Ön projede 3 dış ölçü zinciri: blok, cephe hareketleri ve taşıyıcı akslar."
  - "Uygulama projesinde 4. zincir eklenir: doluluk ve boşluklar."
  - "Her mahalde enine ve boyuna 2'şer iç ölçü. İç ölçü çizgileri birden çok mahalden kesintisiz geçer."
  - Uygulama projesinin genel ölçeği 1/50.
- **Archicad 29, Automatic Exterior Dimensioning** (resmî yardım; resimler `referans/`):
  - Dört zincir: Overall, External Geometry, Structures, Openings.
  - "Place Dimensions on Four Sides"; gerekmeyen zincir atlanır.
- **KentOS'ta parçalar hazırdı:**
  - `cad.dimensionsets.create`'in cephe tarifi: `walls`, `around`, en çok dört sıra, boş sıranın atlanması, saplanan
    duvarlar.
  - `cad.dimensions.interior`'ın kesen çizgisi.
  - Ölçü katmanı anahtarı (MM-XXXX-OLCU, ADR 8008).

## Karar

- **Planı ölçülendir** (`tool.planDimensions`, `plan_dimensions.rs`). Mimari › Belgele › Ölçü ▾'nün ilk satırıdır; takma
  adları PLANOLCU, PLANÖLÇÜ.
- **Dış zincirler:**
  - Dış duvarlar kendiliğinden bulunur: Etkin kattaki bütün duvar gövdeleri birleştirilir; gövdesinin bir köşesi
    birleşimin dış çizgisinde olan duvar dıştır. Saplanan iç duvarlar cephe tarifinin kuralıyla gelir.
  - Doğrultu, dış gövdelerin en uzun düz kenarıdır. İlk sıra en dış gövdeden yazı yüksekliğinin üç katı uzakta (Duvar
    ölçüleri'nin), sıra aralığı 7,5 mm.
  - Aşama (A):
    - Ön proje: Duvarlar (cephe hareketleri), Akslar (taşıyıcı akslar), Toplam (blok).
    - Uygulama: önlerine Açıklıklar (doluluk ve boşluklar).
    - Çizim 1:50 ya da daha büyükse araç Uygulama ile başlar.
  - Boş sıra yapılmaz: aksı olmayan planda cephe başına iki sıra kalır.
- **İç zincirler (İ):**
  - Her mahal enine ve boyuna en az ikişer kez kesilir; her kesen çizgi binanın bir ucundan öbürüne gider (`REACH` 0,5 m
    dışarıdan dışarıya). Zincir duvar yüzlerini ölçer; dış duvar kalınlığı da içindedir.
  - Kesenler az ve paylaşılır (`cuts_for`):
    - mahaller uzak kenarına göre sıralanır;
    - her biri derinliğinin %70'inden ve %30'undan kesilir; orada bir kesen varsa o paylaşılır;
    - iki kesen arası en az 6 mm (kâğıtta);
    - iki kesene yetmeyen dar mahal ortasından bir kez kesilir.
  - İlk sürüm kesenleri mahalin uzak duvarına yığıyordu ve zincirler kapıların üstüne düşüyordu.
- **Önizleme:**
  - Komutların planı yazılmadan çizilir.
  - Kart dış zincirlerin üstünde: "Ön proje: dört cephe, 3 zincir · 32 ölçü · 1 dış duvar", "İç: 10 zincir · 54 ölçü ·
    5 mahal, her biri 2+2".
  - Enter ya da tık yazar; hepsi tek geri alma adımıdır ("Planı ölçülendir"), MM-XXXX-OLCU'da (yoksa aynı adımda açılır).
- **İkon** `planDimensions`: çevre duvarı ve bir bölme, altında ve sağında ölçü çizgileri. Duvar ölçüleri'nden (tek kenar)
  ve İç ölçü zinciri'nden (kesen) ayrılır; ADR 8016'nın bekçileri geçer.

## Ölçüm

- **Test** (`building_context_tests::plan_dimensions_writes_four_sides_and_two_chains_each_way_in_one_step`), 5 × 4 m Dış
  duvar 25 oda:
  - Kart "Ön proje: dört cephe, 3 zincir", "İç: 4 zincir … 1 mahal".
  - Enter: 4 ölçü dizisi ve iç zincirler; tek geri alma hepsini kaldırır.
- **`plan_dimensions::tests`:** yan yana üç mahal (0–4, 0–3, 3–6) her biri iki kez, en çok 4 kesenle, aralar ≥ 0,6;
  0,9 m'lik koridor ortasından bir kez.
- **Gerçek pencere** (`fixtures/interaction/real/plani-olculendir.txt`, kabul testinin sakladığı 2+1 daire, 1440×900):
  - Senaryonun güney zinciri silindi; orta tuşa çift tıkla bütün plan ekrana alındı.
  - Ölçü ▾ › Planı ölçülendir: önizleme "1 dış duvar" (dairenin kapalı dış duvarı), dört cephe, 10 iç zincir, 5 mahal.
  - Enter: "Plan ölçülendi: 4 dış dizi (ön proje, dört cephe), 10 iç zincir"; MM-XXXX-OLCU 86 ölçü.
  - Resim: `.run/gui-evidence/mimari-m26/m26-once-sonra.png`, ikon `plan-olcu-ikonu.png`.
- **Test takımları:** masaüstü 2007 (önceden kırık `each_program_docks_its_own_panels` dışında), etkileşim 81 + 897.

## Sonraki adımlar

- Yeniden çalışınca var olan cephe dizilerini güncellemek (elle taşınan ölçüler korunarak; W2'nin 6. fikri). Bugün yeni
  diziler eskilerin yanına eklenir.
- Kesenlerin kapı ve pencere boşluklarından geçmemesi: bugün boşluktan geçen kesen o duvarı ölçmez.
- Mahal etiketleri ve ölçü yazıları çakışmasın (M27).
- Akslar sırası için aks sistemi (ADV-18); kaba açıklık ölçüsü ve 90/210 (M28).
