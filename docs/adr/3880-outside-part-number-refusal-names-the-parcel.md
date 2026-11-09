# 3880 — Dış parça numarasının retinde parselin adı ve nedeni

Tarih: 2026-11-09. plan/map-cad.md MAP-11, yedinci tur. Durum: taslak. ADR 3080'in ve 3480'in “Sonra”sındaki “Kapsam
dışı kimlikte retin parseli adıyla söylemesi (bugün kimlikle; masaüstü adıyla söylüyor)”.

İlgili:
- [ADR 3080](3080-separation-sheet-outside-part-numbers.md): dış parçaların parsel numaraları (m. 13/3);
- [ADR 3480](3480-separation-sheet-piece-numbers-drawn.md): numaraların çizimde yazılması.

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik** m. 13(3) (ADR 3080'in okuması): düzenleme sınırı dışında kalan
  parçalar parsel numarası verilerek tescil edilir. Numara yalnız düzenlemeye kısmen giren parselin dışarıda kalan
  parçasına verilir.
- **KentOS bugün:**
  - `cad.readjustment.sheets` kısmen girmeyen bir parsele numara verilince parseli kalıcı kimliğiyle söylüyordu
    (“0192f800-… kapsamda kısmen giren bir parsel değil”). Python ya da MCP kullanan hangi parsel olduğunu ve neden
    numara alamadığını kimlikten çıkaramıyordu;
  - masaüstünün Dış parça no kutusu adıyla söylüyor ama nedenini söylemiyordu.
- Kapsamın kendisi her parselin türünü bilir (`whole`, `partial`, `outside`; ADR 2060).

## Karar

1. **Ret (`invalid_input`, `numbers.<kimlik>`):** parsel Ada/Parsel adıyla söylenir, nedeniyle:
   - kapsamda tamamen içerideyse: ““101/1” parseli düzenleme sınırının tamamen içinde, dışarıda parçası yok; …”;
   - kapsamda tamamen dışarıdaysa: ““101/8” parseli düzenleme sınırının tamamen dışında, düzenlemeye girmiyor; …”;
   - kapsamın parselleri arasında değilse: ““101/1” parseli kapsamda kısmen giren bir parsel değil; …”;
   - adı olmayan nesnede (Ada ve Parsel yok) ya da çizimde olmayan kimlikte eskisi gibi kimlikle.

   Ad kapsamın kuralıyla yazılır: Ada/Parsel, yoksa olanı.
2. **Masaüstü** (Dış parça no): aynı nedenler aynı sözlerle: “Dış parça no: “101/1” parseli düzenleme sınırının tamamen
   içinde, dışarıda parçası yok.”
3. Komutun sürümü ve sözleşmesi değişmez; yalnız iletinin sözü.
4. **Kapsam işareti:** ADR 2680'deki gibi **Harita ve CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/readjustment_sheets_cases.py`, KentOS kodu yok): kısmen girmeyen parselin
  durumu adıyla; üç yeni durum (38): kapsamda tamamen içerideki 101/1, kapsamda tamamen dışarıdaki 101/8, adı olmayan
  nesne (düzenleme sınırı) kimliğiyle. Rust ve Python geçer.
- **Masaüstü** (`readjustment_window_tests::the_parts_outside_take_the_typed_parcel_numbers`): 101/1, 101/8 ve
  kapsamda olmayan 999/9 için sözler. Düzenleme penceresinin öbür testleri ve çapraz denetimler geçer. Resim
  `.run/gui-evidence/dis-parca-ret-3880/`.

## Sonra

- Numaranın yazısının dar parçaya sığması (ADR 3480 Sonra): nokta etiketi dönmediği için yazı nesnesine geçmeyi ister.
- Tescil bildirimi (m. 22(1)(ğ)): TKGM'nin biçimi beklenir.
- Web (TODOS §0.2).
