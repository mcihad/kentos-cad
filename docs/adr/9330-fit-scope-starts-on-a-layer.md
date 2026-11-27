# ADR 9330: CBS'de Oturt seçimsiz açılınca etkin katmanla başlar; Uygula'nın yanında sayılar, Dönüştür'de sistem ve işlem ayrı

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  13, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0156](0156-vector-fit.md) (Vektör oturtma), [ADR 1095](1095-vector-fit-scope-keeps-targets.md)
  (GIS-038: seçimsiz açılışta kaynak noktaların katmanı, hedef katman yerinde),
  [ADR 0047](0047-desktop-edit-tools.md) (kilitli katmanın nesnesi taşınmaz, düzenleme bütün ya da hiç).

## Bağlam

- **Gözlem:**
  - Üç nokta, üç yol ve bir analiz sonucu olan projede seçim yokken Ölçme › Oturt açıldı. Parametrelerle kipinde
    **Tümü** kendiliğinden seçildi, Kopya kapalıydı.
  - Katman › qa_noktalar ve Kopya olarak seçilince üç kopya 10 m doğuya gitti, asıllar yerinde kaldı; tek geri alma
    kopyaları kaldırdı.
  - Tümü öntanımlı kalsaydı tek ölçüm katmanını oturtmak isteyen kişi analiz çıktılarını ve öbür katmanları da
    taşıyabilirdi.
- **Dönüştür:** `500000,4320000` EPSG:5256 → EPSG:4326 bağımsız denetimle doğruydu (39°00′48.5126″ K, 36°00′00.0000″
  D). Sonuç satırında işlemin kodu (EPSG:5261) hedef sistemin adının yanında okunuyordu.
- **İncelemenin kabulü:**
  - CBS'de kapsam açık seçimle ya da etkin uyumlu katmanla başlasın.
  - Toplu dönüşümde katman ve nesne sayıları uygulama düğmesinin yanında görünsün.
  - Kaynak bağlı ve korunan katmanlar ön denetimde söylensin.
  - Dönüştür'de Hedef CRS ile Dönüşüm işlemi ayrı etiketlensin.
- **Kod:** seçim yokken Kontrol noktaları kipi kaynak noktaların katmanıyla başlıyordu (ADR 1095); Parametrelerle kipi
  Tümü'yle başlıyordu.

## Karar

### 1. CBS'de seçimsiz Oturt

- **Başlangıç:** CBS projesinde, seçim yokken ve Kontrol noktaları'nın kaynak katmanı yokken Oturt bir katmanla açılır:
  - nesnesi olan ve kilitli olmayan etkin katman;
  - o yoksa ağaç sırasıyla nesnesi olan ilk kilitsiz katman;
  - hiçbiri yoksa Tümü (taşınacak bir şey zaten yoktur).
- **Tümü:** yalnız kullanıcı seçince olur.
- **CAD modülleri:** öntanımları ADR 1095'teki gibi kalır.
- **Kontrol noktaları:** kaynak katmanı varsa başlangıç yine odur.

### 2. Uygula'nın yanında sayılar

- Pencerenin alt çubuğunda, Uygula'nın solunda neyin taşınacağı söylenir: “Uygula: 1 katman, 3 nesne”. Hata varken
  yerinde hata durur.
- Katman katman ayrıntı, Uygulanacak nesneler'in altındaki “Taşınacak: …” satırındadır (ADR 1095).

### 3. Ön denetim

- **Kilitli katman:** taşınacaklar arasında kilitli katman varsa söylenir: ““Parsel” katmanı kilitli; kilitli katmanın
  nesnesi taşınmaz. Kilidini açın ya da kapsamdan çıkarın.”
  - Uygula kilit açılana ya da kapsam değişene kadar kapalıdır. Önceden düğme açıktı, basınca iş bütünüyle
    reddediliyordu (ADR 0047).
- **Veri katmanı:** Tümü seçiliyken veri katmanlarının satırlarının kaynakta olduğu ve taşınmadığı söylenir. Düzenleme
  oturumunda alınan satırlar çizimdedir ve taşınır.
- **Yazma:** Uygula yine kilitli katmanı denetler ve pencerede söyler; hiçbir şey yazılmaz.

### 4. Dönüştür'ün sonucu

- Tek noktada iki satır yazılır:
  - “Hedef sistem: WGS 84 (EPSG:4326).”
  - “Dönüşüm işlemi: ±1 m, EPSG:5261.” (doğruluk ve işlemin kodu; resmî olmayan dönüşüm söylenir).
- Listede de aynı iki ad kullanılır: “N nokta dönüştürüldü. Hedef sistem: …. Dönüşüm işlemi: ….”

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Seçimsiz Oturt'un etkin katmanla başlaması | yok: Mimari'de Oturt yoktur | yok: CAD'de ADR 1095'in öntanımı kalır | yok: aynı gerekçe | var |
| Uygula'nın yanında sayılar, kilitli ve veri katmanının ön denetimi | yok: Oturt yok | var | var | var |
| Dönüştür'de hedef sistem ve işlem ayrı | yok: Mimari'de Dönüştür yoktur | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Kapsam pencerenin sunumudur. Betik nesneleri kimlikleriyle `cad.entities.transform`'a verir; kilitli katmanın reddi aynıdır (ADR 0047). |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Bağlı katmanlar:** CBS'de bağlı katman yoktur (ADR 9196). Harita'daki bağlı katmanın (ADR 0543) ön denetimde ayrıca
  söylenmesi açık.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- calc::`: 87 geçti.
  - `a_cbs_projects_oturt_starts_on_the_active_layer_and_moves_it_alone`:
    - CBS projesinde seçimsiz Oturt'un kapsamı Katman: Parsel (Tümü değil);
    - yanındaki söz “Uygula: 1 katman, 3 nesne”;
    - Parametrelerle ΔY 10 m ve Kopya ile yalnız Parsel'e üç kopya, 10 m doğuya; Yol ve Çizim değişmiyor;
    - tek geri alma kopyaları kaldırıyor.
  - `a_locked_layer_in_reach_is_said_before_uygula` ve `a_locked_layer_is_said_in_the_window`: kilitli katman
    taşınacaklar arasında söyleniyor, Uygula hiçbir şey yazmıyor.
- **Resim** (`.run/gui-evidence/kullanim-9330/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-oturt-etkin-katman`: Katman: Parsel, “Taşınacak: “Parsel” 3 nesne”, alt çubukta “Uygula: 1 katman, 3 nesne”;
  - `02-oturt-kilitli`: kilit uyarısı, Uygula kapalı;
  - `03-donustur`: “Hedef sistem: WGS 84 (EPSG:4326).” ve “Dönüşüm işlemi: ±1 m, EPSG:5261.”.
