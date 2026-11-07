# 1680 — Paftada koordinat bilgisi projeden: datum, elipsoit, projeksiyon, kot sistemi, epok; çelişen yazı bulgusu

Tarih: 2026-11-07. plan/map-cad.md MAP-18, ikinci dilim. Alt madde: “Harita grid'i, gerekiyorsa ikinci koordinat
sistemi, kuzey türü, ölçek çubuğu, datum/epok ve kalite notları aynı proje bilgisinden gelsin.” Durum: taslak.

İlgili ADR'ler:
- [ADR 1320](1320-sheet-sources-and-update-publication.md): MAP-18'in ilk dilimi;
- [ADR 0519](0519-vertical-crs-and-epoch.md): kot türü, geoit ve epok;
- [ADR 0167](0167-second-crs-and-converter.md): ikinci sistem;
- [ADR 0214](0214-expression-extras.md) §2.3: `@` değerleri.

## Dayanak

- **KentOS bugün:**
  - Pafta yazısı `@koordinat_sistemi` ile sistemin adını okuyordu.
  - Datum, elipsoit, projeksiyon, kot sistemi ve epok elle yazılıyordu. Proje başka sisteme geçince ya da şablon başka
    projeden gelince antet eski sistemi söylemeye devam ediyordu.
  - Baskı öncesi denetim bunu görmüyordu.
- **Değerlerin kaynağı:** EPSG kaydı (KentOS'un sistem kitaplığı). Örnek, EPSG:5256 TUREF / TM36: datum TUREF
  (ITRF96), elipsoit GRS 1980, Transverse Mercator, başlangıç meridyeni 36° D, ölçek katsayısı 1. Kot türü, düşey
  sistem, geoit modeli ve epok projenin kendi ayarlarıdır (ADR 0519).

## Karar

1. **`kentos_project::facts::coordinate_facts`:** projenin koordinat bilgisini tek yerden sözle verir.
   - Datum (`TUREF (ITRF96)`), elipsoit, projeksiyon (`Transverse Mercator, DOM 36°, ölçek katsayısı 1`).
   - Kot sistemi (`Ortometrik yükseklik (TUDKA-99; geoit TG-20)`), epok, ikinci sistemin adı, `EPSG:5256`.
   - Yalnız kayıtlı sistemler için verilir. Özel tanımlı sistemde yalnız adı vardır, gerisi boş kalır (tahmin edilmez).
     Sistemsiz projede hiçbiri yoktur.
2. **Pafta yazıları için yeni `@` değerleri:** `@datum`, `@elipsoit`, `@projeksiyon`, `@kot_sistemi`, `@epok`,
   `@ikinci_sistem`, `@epsg_kodu`.
   - `RenderInputs.project` bu alanları taşır. Baskı (`base_inputs`) ve masaüstünün pafta önizlemesi aynı işlevden
     doldurur.
   - **Ayrılmış ad değildir:** paftanın ya da projenin aynı adlı değişkeni önce gelir. Bu adla değişkeni olan eski
     dosyalar geçerli kalır, yazdıkları da değişmez.
   - Projenin ifade değişkenleri (`@epsg`, `@koordinat_sistemi` …) web ile ortak başvurudadır (ADR 0214); bu dilim
     onlara dokunmaz.
3. **Baskı öncesi bulgu `crs_text_mismatch`** (uyarı). Paftanın bir yazısı, `[% … %]` ifadeleri dışındaki sözlerinde:
   - projenin kendisi, ikinci sistemi ya da harita sistemi olmayan bir `EPSG:` kodu yazıyorsa;
   - ya da projenin datum ailesinden hiçbirini anmadan başka bir aileyi (TUREF/ITRF96, ED50, WGS 84) anıyorsa.

   “ED50'den TUREF'e dönüştürülmüştür” gibi bir not iki aileyi de andığı için bulgu değildir. Çözüm cümlesi
   `[% @koordinat_sistemi %]`, `[% @epsg_kodu %]` ve `[% @datum %]` kullanmayı söyler.
4. **Yazı tipi:** ölçek katsayısı “k₀” yerine sözle yazılır. Paftanın yazı tipinde alt simge sıfır yoktur, “?” basılırdı;
   test bunu yakaladı.
5. **Pafta tasarımcısı:** Bağla penceresinin hazır adlarına ve Değişkenler penceresinin açıklamasına yeni adlar
   eklendi.

## Kanıt

- **`kentos-project`** (`facts::tests`):
  - EPSG:5256 ve EPSG:23036 (ED50 / UTM 36N: 33° D, 0,9996, International 1924) kayıttaki değerleriyle;
  - kot, epok ve ikinci sistem;
  - yerel proje.
- **`kentos-plot`** (`tests/all/coordinate_facts.rs`):
  - TUREF / TM36 projesinin paftası ortometrik kotu (TG-20) ve 2005,0 epokunu basar;
  - projenin `datum` değişkeni önce gelir;
  - baskı denetimi `EPSG:23036` ve `EPSG 2320` yazılarını bulur, dönüşüm notunu ve `@` yazılarını bulmaz.
- **Takımlar:** `kentos-plot`, `kentos-sheet` ve `kentos-sheet-ui` geçer. Masaüstünde `sheets`, `module_tests`,
  `ribbon_tests`, `history::text` ve `catalog` geçer.
- **Resim:** paftanın PDF'i ve büyütülmüş görüntüsü `.run/gui-evidence/pafta-koordinat-1680/` altındadır
  (`pafta.pdf`, `pafta-yakin.png`, `bulgular.txt`).

## Sonra

- Kurum eksen adları: KentOS'ta Y sağa, X yukarıdır; kurumun teslim adlandırmasıyla açık eşleme.
- Kuzey türü (grid, coğrafi, manyetik) notunun aynı bilgiden yazılması.
- Ölçme kalite notu (MAP-02 koordinat zinciri bütçesi, [ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md)) için `@` değeri.
- Resmî profil kapısı.
- Web'in pafta girdileri (TODOS §0.2).
