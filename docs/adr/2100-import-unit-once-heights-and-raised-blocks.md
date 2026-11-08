# 2100 — İçe aktarmada birim bir kez: kotlar, iç içe bloklar ve yükseltilmiş yerleştirmeler

Tarih: 2026-11-08. plan/map-cad.md MAP-02, üçüncü tur. Durum: taslak. Plan satırları:

- “İçe aktarmadaki birim kararı bir kez uygulansın; ikinci kez blok, yazı veya kot ölçeklenmesin.”
- Kabul: örnekler bağımsız başvuruyla sınansın.

İlgili:

- [ADR 0165](0165-project-types-cad-gis.md) §2: `$INSUNITS`;
- [ADR 0257](0257-dxf-file-unit-and-import-command.md): blok birimleri;
- [ADR 0144](0144-blocks.md) §5: tanım olarak tutulan blok; tutulan bir yerleştirmenin söyleyemediği patlatılır;
- [ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md) §Sonra: bu bağımsız sınama oradan açık kalmıştı.

## Dayanak

- **Autodesk DXF Reference, INSERT**
  ([2024](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-28FA4CFB-9D5E-4880-9F11-36C97578252F.htm),
  8 Kasım 2026):
  - yerleştirme noktası 10, 20, 30'dur (X, Y, Z; nesne koordinat sisteminde);
  - X, Y, Z ölçek çarpanları 41, 42, 43'tür (isteğe bağlı, varsayılanı 1);
  - dönüklük 50'dir.

  Yani bir yerleştirmenin kendi yüksekliği ve Z ölçeği vardır; bloğun kotları onlarla yerine oturur.
- **ezdxf 1.4.5** (`ezdxf.disassemble.recursive_decompose`): iç içe blok yerleştirmelerinin nesnelerini model
  uzayında yerine koyar. KentOS kodu olmadan bağımsız yerleştirme budur.
- **KentOS bugün:**
  - okuyucu birimi tanımlara (kotlar dahil) ve yerleştirmelerin yerine bir kez uygular;
  - düz okuma (Blokları patlat) kotları yerleştirmenin Z'si ve Z ölçeğiyle birleştirir;
  - tutulan yerleştirmenin (`InsertEntity`) ise ne yüksekliği ne Z ölçeği vardır:
    - model uzayındaki yerleştirmenin Z'si bir notla düşüyordu;
    - bir tanımın içindeki yerleştirmenin Z'si sessizce düşüyordu;
    - Z ölçeği hiç uygulanmıyordu;
  - bu yüzden bir bloğun kotları dosyadaki yerinden yerleştirmenin Z'si ve ölçeği kadar kayabiliyordu.

## Karar

1. **Bağımsız sınama.** `scripts/fixtures/dxf_units_once_cases.py`, ezdxf'le milimetre, santimetre, inç, ayak ve
   metre biriminde birer çizim yazar. Her çizimde şunlar var:
   - 3B çizgi, kotlu nokta, yükseltili çoklu çizgi, sıfırın üstünde daire ve yay, yazı ve çok satırlı yazı;
   - INNER bloğu (3B çizgi ve yazı): OUTER'ın içinde 90° dönük, 3 ölçekli ve yükseltilmiş;
   - OUTER'ın kendi çizgisi; OUTER modelde 30° dönük, 0,5 ölçekli ve yükseltilmiş;
   - noktanın kotuna yükseltilmiş düz bir sembol bloğu (SEMBOL).

   Beklenen her yer ezdxf'in yerleştirmesi çarpı birimin metresidir. Metre projesine okununca sınanan:
   - **düz okumada** her nokta, kot (KentOS'un tuttuğu: çizgi, nokta, çoklu çizgi), yarıçap ve yazı boyu 1 nm içinde;
   - **bloklarıyla okumada** yerleştirmelerin düzlemdeki yeri aynı.

   Birim iki kez alınsa bir değer birimin çarpanı kadar kayardı.
2. **Kotlu bloğun yükseltilmiş ya da Z'de ölçeklenmiş yerleştirmesi patlatılır.** Bloğun kendi nesneleri ya da
   yerleştirdiği bir bloğunkiler kot taşıyabilir: çizgi ucu, nokta, çoklu çizginin yüksekliği ya da köşeleri, yüzün
   köşeleri, eğrinin noktaları. Böyle bir bloğun Z'si sıfır olmayan ya da Z ölçeği 1 olmayan yerleştirmesi tutulmaz,
   patlatılır. Rapor notu: “kotlu bloğun yükseltilmiş ya da Z'de ölçeklenmiş yerleştirmesi patlatılarak alındı; kotlar
   dosyadaki yerinde”.

   Kotlar dosyanın koyduğu yerdedir. Bu, ADR 0144 §5'in kuralının (dizi, eşit olmayan ölçek, eğik düzlem) bir
   uzantısıdır: tutulan yerleştirme bunu söyleyemez. Tanım çizimde kalır; yerleştirilmediği bilgi satırında söylenir.
3. **Düz bloğun yükseltilmiş yerleştirmesi blok kalır.** Ölçme çizimlerinde nokta sembolleri kotuna yükseltilir;
   bloğun nesnelerinde kot yoksa yerleştirme tutulur ve Z'si notla söylenir. Not artık tanımın içindekiler için de
   yazılır:
   - “yerleştirmenin yüksekliği (Z) alınmadı; bloğun nesnelerinde kot yok”;
   - “blok tanımının içindeki yerleştirmenin yüksekliği (Z) alınmadı; bloğun nesnelerinde kot yok”.
4. **Kapsam işareti: CAD + CBS.** Okuyucu ortaktır: Harita, Mimari, Yol ve CBS DXF'i aynı okur; DWG de DXF'e
   çevrilip aynı okuyucudan geçer. Web aynı Rust'ı WASM'la okur (TODOS.md §0.2).

## Kanıt

- **Formats** (`crates/shared/formats/tests/all/dxf_units_once.rs`): beş birim, düz okuma ve bloklarıyla okuma.
  Bloklarıyla okumada tek yerleştirme SEMBOL'dür; OUTER ile INNER'ın çizgileri kotlarıyla açılır, iki not yazılır.
- **Mevcut blok testi** (`dxf::blocks_are_kept_as_definitions_and_inserts_place_them`): KAPI'nin noktası 1,5 kotlu,
  yerleştirmesi 100'de. Artık açılır, nokta dosyanın koyduğu gibi 101,5'te; KAPI'nin tanımı yerleştirilmemiş diye
  söylenir. Bütün formats testleri, içe aktarma ortak durumları ve DWG testleri geçer.
- **Masaüstü** (`import_decision_scenes`): DXF içe aktar penceresi birimi bir kez, açılan blokları ve yükseltilmiş
  sembolün notunu listeler. Resim `.run/shots/arac-okuma-birim-bir-kez-*`.

## Sonra

- Daire, yay ve yazının kotu: KentOS bunları düzlemde tutar; düz okumada da kotları yoktur.
- Farklı blok birimiyle (BLOCK_RECORD) kotlu blok: yerleştirme ölçeği oranla düzeltilir; kotların oranı ayrıca
  sınanmadı.
- Kotlu bloğu blok olarak tutmak: yerleştirmenin yüksekliği ve Z ölçeği gerekir (sözleşme ve `.kcad`).
- Ölçme pencerelerinin çarpanı `cad.crs.factors`'tan okuması; sorgu, düzenleme ve dışa aktarmanın aynı zinciri
  taşıması; zamana bağlı Helmert (MAP-02'nin kalanı).
