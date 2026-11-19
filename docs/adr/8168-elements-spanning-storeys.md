# ADR 8168: Birden çok kata uzanan eleman — IFC'de kat başvurusu

- **Durum:** uygulandı (19 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-04 ("Çok kata uzanan elemanın
  öbür katlara başvurusu"), [bim-plan.md](../../plan/bim-plan.md) B21.
- **Bağlam:** ADR 8040 §2 (katın duvarları ve kolonları yapının üstünde, bitmiş döşemenin katmanları kadar altta durur),
  8047 (binanın IFC'si: her eleman katında, `IfcRelContainedInSpatialStructure`), 8073 (IFC modeli), 8123 ve 8143
  (bölgeler katlara başvuruyla bağlı), 8128 (merdivenin ve rampanın korkuluğu onların parçası), 8166 ve 8167 (katman
  ve profil takımları; görüntüleyicinin Özellikler'i).
- **Dayanak:** IFC 4.3 `IfcRelContainedInSpatialStructure` ve `IfcRelReferencedInSpatialStructure`
  (buildingSMART IFC4.3.x-development). Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-5 ve EK-6 elemanın katını
  `IfcBuildingStorey` olarak ister; birden çok kata uzanan eleman için ayrı hüküm koymaz.
- **Sözleşme:** değişmedi. **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Bir kattan üstteki kata uzanan eleman IFC'de yalnız durduğu katta yer alıyordu: iki kat boyunca tek parça çizilmiş dış
duvar ya da kalkan duvarı, iki katlı giydirme cephe, galeri kolonu, asansör, baca. Kat kat eleman listesi, kat planı ya
da kat metrajı çıkaran bir program onu üst katta bulamıyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcRelContainedInSpatialStructure` | Yer alma hiyerarşiktir: bir eleman tek bir mekânsal yapı öğesinde yer alır | Eleman yine yalnız kendi katında yer alır (K436) |
| IFC 4.3 `IfcRelReferencedInSpatialStructure` | Hiyerarşik değildir: eleman hiçbir kata, bir kata ya da birçok kata başvurabilir. Örneği zemin katta yer alan giydirme cephedir: "referenced by all additional stories it spans" | Gövdesi üstteki bir katın kotunu aşan eleman o kata başvurur; her kat için bir ilişki (K436) |
| Aynı varlığın AllowedRelatedElements kuralı | İlişki, mekânsal yapı öğesini başka bir öğeye katmak için kullanılmaz (hiyerarşi `IfcRelAggregates`'tedir) | Yalnız yapı elemanları başvurur; bölgelerin başvuruları (8123, 8143) olduğu gibi kalır |
| ADR 8040 §2 | Katın duvarları ve kolonları yapının üstünde, bitmiş döşemenin katmanları kadar altta durur | Uzanma yalnız yukarı ölçülür: üst katın 7 cm aşağıdan başlayan duvarı alttaki kata başvurmaz (K437) |
| [Sayısal politika](../specs/numeric-policy.md) §5 | Mekânsal sorgu ve topoloji toleransı 1 mm | Gövdenin üstü katın kotunu 1 mm'den çok aşarsa uzanır; döşemeye kadar örülen duvar ve son basamağı döşemeye inen merdiven uzanmaz (K437) |
| EK-5, EK-6 | Elemanın katı `IfcBuildingStorey`; çok kata uzanma üzerine hüküm yok | IFC 4.3'ün yolu (K436) |

## §3. Karar

- **K436 — Kat başvurusu:** duvar, kolon, giydirme cephe, dikme ve kayıt (`IfcMember`), merdiven, rampa, korkuluk,
  asansör (`IfcTransportElement`) ya da baca kendi katında yer alır (`IfcRelContainedInSpatialStructure`). Gövdesinin
  üstü daha yüksek bir katın kotunu aşıyorsa o kata da başvurur: o kata uzanan bütün elemanlar katın tek
  `IfcRelReferencedInSpatialStructure`'ındadır (GlobalId'si katınkinden türetilir, her yazışta aynı). Döşeme, kiriş,
  çatı, kaplama, kapı, pencere ve eşya başvurmaz: katlar arasında, en üst katın üstünde ya da bir duvarın içindedirler.
  Bir bütünün parçası (giydirme cephenin dikmesi ve kaydı, merdivenin korkuluğu) başvurmaz, bütününü izler; bütünün
  kendi gövdesi ölçülür (giydirme cephenin panelleri, merdivenin kolları).
- **K437 — Yalnız yukarı, 1 mm:** uzanma, gövdenin üstünün kendi katından yüksek bir katın kotunu projenin mekânsal
  toleransından (1 mm) çok aşmasıdır. Döşemeye kadar örülen duvar, döşemenin altında biten kolon ve son basamağı üst
  kata inen merdiven uzanmaz. Üst katın yapı üstünde duran (bitmiş döşemenin katmanları kadar aşağıdan başlayan) duvarı
  ve kolonu alttaki kata başvurmaz.
- **K438 — Görüntüleyici:** IFC modeli'nin Özellikler'inde Kat satırının altında, nesnenin başvurduğu her kat bir
  "Uzandığı kat" satırıdır (katın adı); dosyadan okunur.

## §4. Uygulama

- `crates/shared/ifc/src/building.rs` (`SPAN`, `spans_storeys`, `height_range`; yazıcıda katlara başvurular).
- `crates/shared/ifc/src/read/details.rs` (`spanned_storeys`; Uzandığı kat).
- `scripts/fixtures/storey_spans_check.py` (bağımsız okuyucu), `fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc`.
- [Sayısal politika](../specs/numeric-policy.md) §5 (kata uzanma toleransı).

## §5. Sınamalar

- **Birim** (`building::a_wall_two_storeys_tall_is_referenced_in_the_storey_over_its_own`): 6 m'lik duvar üst kata
  başvurur; üstü döşemede ya da 1 mm içinde biten başvurmaz, 2 mm aşan başvurur; üst katın −0,07'den başlayan duvarı
  alttaki kata başvurmaz; iki katlı giydirme cephe başvurur; alt kata sarkan üst kat döşemesi başvurmaz. Okunan
  ayrıntıda Kat ve Uzandığı kat.
- **Uygulama** (`arch_ifc::a_wall_two_storeys_tall_spans_the_storey_over_its_own`): kesit binasının doğu duvarı iki kat
  boyunca tek parça ve çatının üstünde korkuluk olarak bir kalkan duvarı gibi çizilir. Zemin kattaki duvar Değiştir'le
  6,6 m'ye yükseltilir, 1. kattaki duvar ve iki doğu kirişi silinir, 1. katın döşemesi ve teras duvarın iç yüzünde
  Kes'le kesilir. Duvar zemin katta yer alır, 1. kata başvurur; binanın öbür duvarları ve kolonları hiçbir kata
  başvurmaz. Altın dosya `kesit-binasi-kalkan-duvari.ifc`'dir (`KENTOS_WRITE_IFC_BUILDING=1`, her yazışta aynı).
- **Altın dosyalar:** belirlenimci yazıcılarla (kesit binası ve türevleri, eğri bina, yerel sistem) yeniden yazılanların
  hiçbiri değişmedi; bu binalarda kat aşan eleman yoktur.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0, `storey_spans_check`): elemanın gövdesini dünyada kurar, katların kotunu
  yerleşimlerinden alır; elemanın uzandığı katlar başvurduğu katlarla aynı olmalı, katlar bölgeler dışında hiçbir şeye
  başvurmamalı. Dokuz belirlenimci altın dosyada, kimlikleri her yazışta yenilenen beş dosyada (blok A ve B, çatı altı
  mahal, eğimli döşeme, katmanlı çatı), eğri binanın başvuru dosyasında ve dikme, giydirme cephe, kaplama sahnelerinin
  dosyalarında kat aşan eleman yok. Kalkan duvarı dosyasında yalnız doğu duvarı 1. kata başvurur: GlobalId'si kesit
  binasının tanımındaki kimlik, gövdesi çizildiği gibi 6,6 m. Doğrulayıcı (öznitelikler, EXPRESS kuralları) 0 bulgu
  verir.
- **Masaüstü:** `ifc_viewer_tests` ve ortak süzgeçler (156 sınama).
- **Resimler:** `.run/gui-evidence/iki-kat-duvar-8168/`: IFC modeli'nde kalkan duvarı seçili; Özellikler'de Kat: Zemin
  kat, Uzandığı kat: 1. kat; açık ve koyu.

## §6. Kalanlar

- Yapı ağacında elemanın başvurduğu katın altında da gösterilmesi; katı gizlemenin ve Yalnız bırak'ın başvuran
  elemanları da kapsaması.
- Aşağı uzanma: kendi katının yapısından daha aşağı inen eleman (katmanların altından alttaki katın içine) bugün yalnız
  kendi katındadır.
