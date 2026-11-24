# ADR 9217: NCZ'de numarası yalnız etiketinde olan parsellere numara

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-18. Bulgu:
  [ADR 9216](9216-ncz-and-its-dxf-compared.md) §4.
- **Sahibin kararı** (24 Kasım 2026): “Yalnız eksiklere”. Numarası ayrı bir yazıyla zaten görünen parsele dokunulmaz,
  Netcad'deki gibi çift numara olmaz.
- **Sözleşmeler:** değişmedi. Okuyucu, nesneleri bugünkü türlerle (yazı) üretiyor.

## §1 Netcad'in kuralı (ölçüldü)

Netcad parsel poligonunun “Etiket”inden (`ada/parsel`) parsel kısmını parselin içine yazıyor. Netcad'in aynı plandan
aktardığı DXF'te bu yazılar 3,0 boyunda ve hizalama grubu yok. Parselin alan ağırlık merkezine göre konumları
43 714 etikette ölçüldü:

| Karakter | Yerleştirme − ağırlık merkezi (ortanca) |
|---|---|
| 1 | −1,188, −1,501 m (20 285 etiket) |
| 2 | −2,375, −1,496 m (20 160) |
| 3 | −3,562, −1,500 m (3 012) |

Yani yazı ağırlık merkezine yatayda ve dikeyde ortalanıyor. Yüksekliği 3,0 m, karakter başına 2,375 m genişlik
hesaplanmış. Kutunun ortası (ortanca 0,65 m sapma), köşelerin ortalaması (2,31 m) ve en içteki nokta (4,05 m) daha
uzak.

**Numara çoğu parselde iki kez çiziliyor.** Planın 46 826 etiketli parselinden:

- 44 430'unda numara NCZ'nin ayrı bir yazısıyla da görünüyor. Netcad orada numarayı iki kez çiziyor.
- **2 396'sında** numarayı yalnız etiket taşıyor.

ADR 9216 bu sayıyı ilk ölçümde 4 149 vermişti. O ölçüm bir yazıyı yalnız ilk bulduğu poligona saymıştı. Planda
iç içe ve üst üste parseller var (208 parsel halkası iki kez), bu yüzden sayı şişmişti.

## §2 Karar

Okuyucu (`kentos_ncz::read`, `parcel_numbers`) okumanın sonunda şu kuralı uygular.

1. **Kadastro katmanı:** katmanın etiketli alanlarının çoğu, ve en az 10'u, `ada/parsel` biçiminde (ada sayı, `/`,
   parsel). Planın alanları da etiket taşıyor (`70`, `İRT_A952`, `OA_2_1`) ama Netcad onları yazmıyor. İlk sürüm bu
   koşul olmadan 41 katmanı kadastro saydı.
2. **Numara:** `ada/parsel`'in parsel kısmı; adasız etikette (`6587`) etiketin tamamı. Rakam içermeyen ya da boşluklu
   etiket numara değildir (`TESCİL HARİCİ`).
3. **Zaten görünüyor mu:** aynı katmandaki bir yazı parselin içinde duruyor ve metni numara ya da etiketin tamamıysa,
   parsele dokunulmaz.
4. **Yalnız tek parselde:** ağırlık merkezi bu parselin içinde ve katmanın başka hiçbir alanının içinde değilse yazı
   eklenir. Yazı ağırlık merkezine, ortası ortada (`MiddleCenter`), 3 m boyunda, katmanın renginde konur.
5. **Rapor:** açılış raporu kaç parsele yazıldığını, kaçına dokunulmadığını ve kaçının neden atlandığını söyler.

**4. maddenin gerekçesi.** Üst üste parsellerde (eski bir parsel ve bölünmüşleri) Netcad'in yazdığı numara etiketten
değil, bu okuyucunun parsele bağlayamadığı bir alandan geliyor (öznitelik tabloları, ADR 0138). Etiketi `6481/1`
olan küçük poligonlara Netcad “14”, “9”, “7” yazıyor. Netcad'in DXF'iyle karşılaştırma:

| Kural | Netcad'le aynı | Netcad başka numara | Netcad'de yok |
|---|---|---|---|
| **Ağırlık merkezi yalnız bu parselde (seçilen)** | 1 128 | **0** | 2 |
| Ağırlık merkezi başka parsellerde de | 939 | 183 | 77 |
| … içerenlerin hepsi aynı numarayı söylüyorsa | 16 | 2 | 16 |
| … en küçük içeren parsel buysa | 806 | 129 | 36 |

Hiçbir inceltme yanlış numarayı sıfırlamadı. Yanlış bir parsel numarası, hiç numara olmamasından kötüdür; bu yüzden
bu parseller atlandı.

## §3 Sonuç (SIVAS_UIP_2025_06_HAZIRAN.NCZ)

- **Yazılan:** 1 117 numara, yalnız KADASTRO'da.
  - 1 106'sı Netcad'in DXF'teki etiketiyle aynı metinde ve 0,5 m içinde. Ortanca uzaklık 4 nanometre, yani konum
    aynı.
  - **0'ı farklı numara.**
  - 11'inde Netcad'in etiketi yakında yok.
- **Dokunulmayan:** numarası bir yazıyla zaten görünen 44 424 parsel.
- **Atlanan:** ağırlık merkezi başka bir parselin içinde de kalan 1 199 parsel. Rapor bunu söylüyor.
- **Okuma süresi:** 0,49 s (release). Kural ayrıca ölçülebilir bir süre eklemiyor.

## §4 Kanıt

**Sınamalar** (`kentos-ncz`, 25 sınama geçti):

- `a_parcels_number_is_the_part_after_its_block`;
- `only_the_parcels_no_text_numbers_get_their_label_written`: on iki parselden yazısı olan sekizine dokunulmaz, dördüne
  ağırlık merkezinde numara yazılır. Şunlara yazılmaz:
  - başka parselin numarası içinde duran parsel;
  - başka katmanın yazısı;
  - beş parsellik katman;
  - sözcük etiketli katman;
  - adasız numaralı plan alanları;
  - üst üste iki parsel.

**Gerçek pencere** (`.run/gui-evidence/render/rm18/`, Xvfb):

- Plan açıldı, rapor kapandı, parsel `103/TH4`'e tekerlekle yakınlaşıldı.
- Önce parselin içi boş, sonra ağırlık merkezinde “TH4” var (`once-sonra-th4.png`).
- Farklı pikseller yalnız yazının çevresinde.

## Uygulama

`crates/shared/ncz/src/emit.rs`:

- `parcel_numbers`, `parcel_number`, `block_and_parcel`, `area_centroid`, `in_ring`;
- `PARCEL_NUMBER_HEIGHT`, `LEAST_PARCELS`;
- `read`'in rapor notu.
