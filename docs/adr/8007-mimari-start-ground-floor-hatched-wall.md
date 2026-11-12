# 8007 — Mimari başlangıç: Zemin kat, poşeli ilk duvar, Mimari sekmesi; şablon m ve 1:100

Tarih: 2026-11-07. Durum: kabul (M8, ADV-13; plan S4, S12, G5, G9).

## Sorun

Ölçümler M7'nin programıyla, gerçek pencerede alındı (`.run/gui-evidence/mimari-m8/once/`).

- **Yeni proje Giriş sekmesinde açılıyordu (G9).** Duvar Mimari sekmesinde. Boş projeden ilk duvara 4 eylem gerekiyordu:
  Mimari sekmesi, Dikdörtgen duvar ve iki köşe.
- **Projede kat yoktu.** İlk duvarın Kat'ı "Yok"tu.
- **Duvar araçlarının stili "Yok"tu (G5).** İlk duvar 1:100'de içi boş iki ince çizgiydi; Kalınlık 0,200.
- **Şablon cm ve 1:50'ydi (S12).** `catalog/v1/templates/mimari.json` böyleydi, oysa yeni Mimari projesi m ve 1:100
  (ADR 0626).
  - "Ofis paketinin ayarlarını uygula" yeni projeyi cm ve 1:50'ye çeviriyordu.
  - Penceredeki "cm / 1:50" ve "1:50 / 1:20 oranlarıyla" sözleri elle yazılmıştı.

## Araştırma (7 Kasım 2026)

Resmî belgelerden alınan görseller `.run/gui-evidence/mimari-m8/referans/` altında.

- **AutoCAD Architecture 2026:**
  - *A Quick Start for Transitioning From AutoCAD to AutoCAD Architecture 2026 Toolset*: duvar, açılıştaki sekmededir
    ("Home tab › Build panel › Wall drop-down › Wall").
  - *About Wall Styles*: "Wall styles control the appearance of wall objects … dimensions, components, endcaps,
    materials". Bileşenli stiller planda taralı çizilir (`aca-wall-styles-components.png`).
- **Revit:**
  - *Add Levels* (2026): katlar L1 = 0 ve L2 = 3000 (`revit-level-name-height.png`). "each level you create is a story
    level and has an associated floor plan view".
  - *Getting Started, Part 2: Create Walls* (2024): dış duvarlar Architecture › Build › Wall ile, "Basic Exterior -
    Render on Brick on Block" tipiyle çizilir (`revit-getstarted-exterior-walls.png`).
- **Archicad:**
  - *Wall Tool Settings* (27): "Home Story is the current story", "Wall top is linked to one story up".
  - Varsayılan ayar "0. Ground Floor (Current)", 3100 mm ve "Generic Wall/Shell" (`archicad-WallGeometry.png`); kompozit
    seçici (`archicad-ChooseComposite.png`).
  - *Walls* (28): "The bottom of every Wall (like other elements) is linked to its Home Story."
- **Ortak yol:**
  - yeni projede bir zemin kat ve kat yüksekliği hazırdır;
  - ilk duvar bir kata bağlıdır ve bileşenli, taralı bir tiple başlar;
  - duvar aracı açılış sekmesindedir.
- **Mevzuat:**
  - **PAİY 28(1):** konutta kat yüksekliği 3,60 m. **28(4):** iskân edilen katta iç yükseklik en az 2,60 m. 3,00 m'lik
    Zemin kat ikisinin de içinde kalır; döşeme ve kaplamadan sonra iç yükseklik yaklaşık 2,75 m'dir.
  - **PAİY 57(27):** belediye projeleri "1/100 ölçekli, ancak 1/50 ölçekli proje tekniğinde çizilmiş olarak"
    isteyebilir. Ruhsat kat planları ve kesitleri 1/100'dür; uygulama projesi 1/50.
  - **BEP Yönetmeliği md. 9 ve TS 825 (Ekim 2024):** TS 825, RG 20.02.2025/32819'daki tebliğle zorunludur. Dış havaya
    temas eden bileşenler TS 825'in asgari şartlarını sağlar. Varsayılan dış duvar bu yüzden ısı yalıtımı taşır (5 cm).
    Kalınlığın bölgenin TS 825 hesabına uyduğunu proje gösterir, KentOS denetlemez.
  - **2026 Dijital Hazırlama Yönetmeliği EK-4 4.31.1:** aplikasyon projesinde ölçü ve koordinatlar TUREF 3°
    sistemindedir, yani metredir.

## Karar

- **Yeni Mimari projesi Zemin kat'la açılır** (`kentos_contracts::ground_floor`, `kentos_project::new_project`).
  - Zemin kat ±0,00'dadır, kat yüksekliği 3,00 m, kimliği `zemin`. Şablonun ilk katıyla aynıdır; bunu bir test kilitler.
  - Etkin kat kuralı (ADR 0720) onu etkin kat yapar.
  - Kural işi Mimari olan her yeni CAD projesinde geçerlidir: KentOS Mimari'nin Yeni proje'si, kentos-cad sihirbazında
    Mimari işi ve Python'daki `Document.new`.
  - Harita ve yol işleri kat almaz. Çizgi gibi nesnelerin katı olmaz; kat yalnız duvar, kolon, merdiven ve rampaya
    yazılır (ADR 0713).
- **Duvar araçları Dış duvar 25 ile başlar** (yalnız KentOS Mimari'de; `arch_start.rs`).
  - Stil: iç sıva 2 cm, tuğla, ısı yalıtımı 5 cm, dış sıva 3 cm; toplam 0,25 m. 1:100'de koyu gri poşe çizilir.
  - Duvar, Dikdörtgen duvar, Ölçülü bölme ve Konturdan duvar aynı stil belleğini kullanır.
  - Stil menüsünde "Yok" ve öbür stiller durur. Seçilen stil program açık kaldıkça sürer.
  - Stil projeye ilk duvarla girer (ADR 0339 §3).
  - Hangi stilin varsayılan olacağını sahip seçti: "Dış duvar 25 (Önerilen)".
- **Yeni proje Mimari sekmesinde açılır.**
  - Plan bunu M15'e koymuştu. İlk duvara en çok 3 eylem kabulü buna bağlı olduğu için M8'e alındı.
  - Dosya açmak sekmeyi değiştirmez; öbür programlar da değişmez.
- **Şablon m ve 1:100'dür.**
  - Paket metre ve 1:100'dür. A-101 ile A-301 paftaları 1:100, A-501 detayları 1:20'dir.
  - "Ofis paketinin ayarlarını uygula" yeni projede birimi ve ölçeği değiştirmez.
  - Koordinat sistemli projede birim verilmeden uygulanabilir, çünkü paket zaten metredir. Açıkça cm verilirse yine
    `unit_needs_local` ile reddedilir.
- **Eski otomasyon çağrısı değişmez.**
  - `settingsMode`'suz `cad.templates.apply` ADR 0346'nın v1 çağrısıdır ve ADR 0659 onu korur. Bu çağrı boş çizime cm,
    her çizime 1:50 yazmayı sürdürür (`LEGACY_TEMPLATE_UNIT`, `LEGACY_TEMPLATE_PLOT_SCALE`).
  - Eski 23 sözleşme durumu bayt bayt aynıdır.
- **Pencere metinleri paketten okunur** (`architecture_template.rs`, `Package::read`):
  - "Ofis paketinin ayarlarını uygula — m / 1:100";
  - "Ofis paketi m / 1:100, 2 kat ve 3 A3 pafta içerir";
  - "Paftalar kendi ölçekleriyle eklenir: 1:100 ve 1:20."
- **Sözcük:** "kendi 1:50 / 1:20 oranlarıyla" → "kendi ölçekleriyle". Ölçeğe oran denmez.

## Ölçüm

- **Testler:**
  - `arch_start_tests.rs` (3 durum):
    - KentOS Mimari'nin duvar araçları Dış duvar 25 ve 0,25 m ile başlar; kentos-cad, Harita ve GIS "Yok" ve 0,20 m ile;
    - Mimari'de Yeni proje, Giriş'te kalmış bir yerleşimde bile Mimari sekmesinde ve Zemin kat'ta açılır; Dikdörtgen
      duvar ve iki köşe duvarı Dış duvar 25, 0,25 m ve `zemin` katıyla yazar, stil projeye poşesiyle girer;
    - kentos-cad'in sekmesi değişmez.
  - `new_project` (kentos-project): Mimari işi Zemin kat'la, öbür işler katsız açılır.
  - `cad_templates` (kentos-commands): paket m, 1:100, paftaları 100/100/20, ilk katı `ground_floor`; eski çağrının
    değerleri cm ve 1:50.
  - `templates_safe` (kentos-native-application): template kipi cm / 1:200'lük çizimi m / 1:100'e çevirir; ayarsız eski
    çağrı boş çizime cm ve 1:50 yazar.
  - Şablon penceresi: metinler paketten okunur ("m / 1:100", 2 kat, 3 pafta, "1:100 ve 1:20").
  - Ortak sözleşme dosyaları:
    - eski `cad.templates.apply` 23 durum, bayt bayt aynı;
    - güvenli şablon 34 → 35 durum; koordinat sistemine paketin metresi birim verilmeden uygulanır, açıkça cm verilirse
      ret.
  - Python SDK şablon testleri 4/4, yerli modül yeniden derlendi.
- **Test takımları:**
  - kentos-commands 1725;
  - kentos-contracts 1354 + 34 + 10;
  - kentos-project 34;
  - kentos-interaction 75 + 896;
  - kentos-native-application 146 + 526.
  - Masaüstünde 1977 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels` (KentOS Harita'nın
    yol paneli), `main`'den gelen eski hata.
- **Gerçek pencere** (ADR 8000; `fixtures/interaction/real/mimari-baslangic{,-once}.txt`, 1440 × 900):
  - **Önce** (M7'nin ikilisi): Yeni proje Giriş sekmesinde açıldı. Mimari sekmesi, Dikdörtgen duvar ve iki köşe olmak üzere
    4 eylem gerekti, KLM 9,2 s. Öznitelikler: Kalınlık 0.200, Stil Yok, Kat Yok. Duvarın iki yüzü arası boştu (piksel 248).
  - **Sonra:** Yeni proje Mimari sekmesinde açıldı. Dikdörtgen duvar ve iki köşe olmak üzere 3 eylem yetti, KLM 7,9 s.
    Aracın paneli Kat Zemin kat, Stil Dış duvar 25, Kalınlık 0.250 m dedi. Seçili duvar: Kalınlık 0.250, Stil Dış duvar
    25, Kat Zemin kat. İki yüzün arası gri poşeyle dolu (piksel 177).

## Kalanlar

- İlk duvarın kendi katmanında yazılması (ADV-14, M9).
- ADV-13'ün öbür maddeleri:
  - stillerin bileşenlerinin ADR 0791 malzeme kitaplığına bağlanması;
  - 13,5 cm'lik iç bölme stili;
  - mahal adı listesi;
  - örnek 2+1 projesi;
  - ilk beş dakikanın ipuçları.
- Seçilen stil program kapanınca unutulur, çünkü araç belleği kalıcı değil.
