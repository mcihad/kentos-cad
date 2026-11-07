# 1900 — Paftada kuzey bilgisi: yakınsama, manyetik sapma ve grid–manyetik açı yazıda (`@` değerleri)

Tarih: 2026-11-07. plan/map-cad.md MAP-18, ikinci tur. Durum: taslak. Plan satırı: “Harita grid'i, gerekiyorsa ikinci
koordinat sistemi, kuzey türü, ölçek çubuğu, datum/epok ve kalite notları aynı proje bilgisinden gelsin.” İlgili:
- [ADR 1680](1680-sheet-coordinate-facts.md): paftada koordinat bilgisi (`@datum` …); bu ADR oradaki “Sonra: kuzey türü
  notu” maddesini açar;
- ADR 0110 ve pafta tasarımı §8a: kuzey oku ve Türk topografik paftalarının kuzey diyagramı (GK, CK, MK).

## Dayanak

- **KentOS bugün:** paftanın kuzey oku haritanın merkezinde grid, coğrafi ve manyetik kuzeyi gösteriyordu:
  - meridyen yakınsaması TM parametrelerinden çekirdekte hesaplanır (`geodesy::tm_inverse`);
  - manyetik sapma Dünya Manyetik Modeli WMM2025'ten, paftanın tarihinde (`@tarih`) hesaplanır ya da elle yazılır;
  - kuzey diyagramı üç satırı yazar: “GK–CK yakınsama”, “CK–MK manyetik sapma”, “GK–MK açısı”.

  Antetin yazıları bu değerleri okuyamıyordu; elle yazılan bir yakınsama paftanın yeri ya da tarihi değişince eski
  kalıyordu (ADR 1680'in datum için bulduğu sorunun aynısı).
- **Bağımsız değerler** (testin beklentisi, KentOS'un değil):
  - PROJ (pyproj 3.7): TUREF / TM36'da (450 000, 4 420 000) 39,912705° K, 35,415224° D; meridyen yakınsaması
    −0,375212° (`get_factors`);
  - pygeomag 1.1.0'ın kendi WMM2025'i (`WMM_2025.COF`): 2026-11-07'de (2026,849315) sapma 6,299937° D.

## Karar

1. **Yeni `@` değerleri** (pafta çekirdeği, `display::marks::north_texts`):
   - `@yakinsama`: grid kuzeyinden coğrafi kuzeye, işaretli derece-dakika (“−0°23'”);
   - `@manyetik_sapma`: coğrafi kuzeyden manyetik kuzeye, WMM'nin haritanın merkezinde paftanın tarihindeki değeri
     (“6°18' D (WMM2025, 2026-11)”);
   - `@grid_manyetik`: grid kuzeyinden manyetik kuzeye (“6°41' D”), diyagramın kendi toplamıyla (−yakınsama + sapma).
2. **Hangi harita:** yazı bir haritaya bağlıysa o haritanın merkezi, değilse paftanın ilk (genel bakış olmayan)
   haritasının merkezi. `@olcek` ile aynı kural.
3. **Bilinmeyen değer boştur:** haritanın yeri hesaplanamıyor (TM ya da UTM olmayan sistem, merkezi olmayan harita),
   tarih yok ya da modelin yıllarının dışında. Yazı “değer boş” bulgusunu alır; tahmin edilmez. Yakınsama için çekirdek
   hesaplayamıyorsa ev sahibinin verdiği değer kullanılır (kuzey okunun kuralı).
4. **Ayrılmış ad değildir:** paftanın ya da projenin aynı adlı değişkeni önce gelir (ADR 1680 gibi).
5. **Pafta tasarımcısı:** Bağla penceresinin hazır adlarına ve Değişkenler penceresinin açıklamasına eklendi.

## Kanıt

- **`kentos-plot`** (`tests/all/coordinate_facts.rs::a_sheet_reads_the_north_at_its_map_s_centre`):
  - TUREF / TM36 projesinde merkezi (450 000, 4 420 000) olan harita: “GK–CK yakınsama −0°23'”, “CK–MK manyetik sapma
    6°18' D (WMM2025, 2026-11)”, “GK–MK açısı 6°41' D”; değerler PROJ'un ve pygeomag'ın;
  - orta meridyendeki ikinci haritaya bağlı yazı “+0°00'” okur.
- **Takımlar:** `kentos-sheet` (429), `kentos-plot`, `kentos-sheet-ui` geçer; masaüstünde `sheets`, `module_tests`,
  `ribbon_tests`, `history::text`, `catalog`, `settings` geçer.
- **Resim:** `.run/gui-evidence/pafta-kuzey-1900/kuzey.pdf` (ve PNG'leri).

## Sonra

- Kurum eksen adları (Y sağa, X yukarı ile kurumun adlandırması).
- Ölçme kalite notu: MAP-02'nin koordinat zinciri bütçesi projede tutulursa `@` değeri.
- Resmî profil kapısı.
