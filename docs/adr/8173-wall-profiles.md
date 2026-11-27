# ADR 8173: Duvar profili (cephe silmesi, harpuşta)

- **Durum:** 1. bölüm kabul edildi ve uygulandı (27 Kasım 2026): profil modelde, kesitte, 3B'de, IFC'de ve dosyada.
  Kalan: Duvar profili aracı ve şerit (2. bölüm).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-11. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporu (§5): duvar
  profili (silme, oyuk) piyasadaki BIM araçlarında var, KentOS'ta yok.
- **Sözleşmeler:**
  - Kirişe `covering` alanı eklendi (`Covering`: tip ve ad). Kaplama olan bant profildir.
  - `.kcad`'ın Mimari verisi 42 (`ARCH_WITH_PROFILES`). Belge şeması değişmedi.
  - `cad.beams.create` `covering`'i alır.

## §1 Sorun

- **Cephe silmesi ve harpuşta çizilemiyordu.**
  - Cephe silmesi: kat hizasında ya da subasmanda cepheyi saran bant.
  - Harpuşta: parapetin ve bahçe duvarının tepesini örten başlık.
- **Sonuçları:**
  - Görünüşte ve kesitte bu bantlar elle çizgiyle çiziliyordu. Modelde yoklardı.
  - 3B'de ve IFC'de görünmüyorlardı.
  - Mahal listesinin (PAİY 57(29)) harpuştası modelden okunamıyordu.
- **Piyasa** (araştırma raporu §5):
  - Revit: duvar silmesi (Wall Sweep) ve oyuğu (Wall Reveal).
  - Archicad: karmaşık profil.
  - İkisinde de bant duvar boyunca bir profilin süpürülmesidir.

## §2 Araştırma

### Türkiye mevzuatı

- **Dijital Hazırlama Yönetmeliği** (RG 5.8.2026/33331):
  - EK-2 Tablo 2.3 (Mimari disiplinin öğeleri), `mevzuat/bim/ek-2-dosya-isimlendirme.md`:
    - Harpuşta **HARP**.
    - Cephe **CEPH**.
    - Kartonpiyer KARP, Süpürgelik SUPR, Denizlik DENZ.
    - Cephe silmesinin kendi öğesi yok. Cephenin parçasıdır: `MM-CEPH-<malzeme>`.
  - EK-5 Tablo 5.1: kaplama `IfcCovering`, kategori kodu **KPL**.
  - EK-6 Tablo 6.20 (`IfcCovering`): ön tanımlı tipleri CEILING, CLADDING, **COPING**, FLOORING, INSULATION, MEMBRANE,
    **MOLDING**, ROOFING, SKIRTING BOARD, SLEEVING, TOPPING, WRAPPING, USERDEFINED, NOTDEFINED.
- **Planlı Alanlar İmar Yönetmeliği:**
  - 40/6: düz çatı parapeti en çok 1,10 m; kullanılan terasta 1,10 m parapet ya da korkuluk zorunlu
    (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`). Parapet bir duvardır; harpuştası profildir.
  - 57(29): mahal listesinde süpürgelik, denizlik ve harpuşta yazılır.
- **IFC 4.3:** `IfcCoveringTypeEnum` COPING bir duvarın tepesini örten başlık, MOLDING bir yüzeye eklenen süs
  bandıdır.

### Mevcut KentOS

- Kaplama (ADR 8070 §10) döşemenin bir rolüdür: `SlabFields.covering`, bir plaka.
- Kayıt (ADR 8070 §8) kirişin bir rolüdür: `BeamFields.member`, `IfcMember`.
- Profil çizgisel bir banttır: ekseni, genişliği (çıkıntısı), yüksekliği ve kattan yüksekliği olan bir kiriş bandı.
  Kirişin gövdesi, kesiti, 3B'si ve IFC'si hazırdır.

## §3 Karar

### §3.1 Model: kaplama olan kiriş bandı

- **Profil** `BeamFields.covering: Option<Covering>`'dir. Kaplama döşemeyi plaka yapar, kirişi profil yapar; aynı
  `Covering` türü, aynı tipler.
- **Kurallar** (`BeamFields::problem`):
  - Kaplamanın kendi kuralları: tipi EK-6'nın listesinden; USERDEFINED'ın adı var.
  - Eleman (kayıt) olamaz.
  - Türü Kiriş olmalı: lento, hatıl ve temel profil olamaz.
  - Kattan yüksekliği (`baseOffset`) zorunlu: profil katın döşemesinden o kadar yukarıda durur, hiçbir döşemeden
    asılmaz.
  - Eğim almaz (eğimi yalnız eleman alır).
- **Cephe silmesi** `MOLDING` ve adı "Silme"dir (`Covering::facade_moulding`). Adıyla Cephe'nin öğesine çizilir
  (`Covering::element_of`); başka adlı MOLDING (kartonpiyer) KARP'tadır.
- **Harpuşta** `COPING`'dir (`Covering::coping`).

### §3.2 Nerede ne olur

| Yer | Profil |
|---|---|
| Yapı | Yapı değildir (ADR 8070 §10'daki kaplama gibi). Duvarın tepesini kesmez: duvarın üstündeki harpuşta duvarı kısaltmaz. Kolon ve kiriş birleşimlerine (ADR 8171 §2.1) girmez, bütün kalır. |
| Kesit | Kaplamasının EK-2 öğesinde, malzemesiyle (yoksa GENEL): `MM-HARP-TAS`, `MM-CEPH-PREKASTBETON` (EK-4: en çok 15 büyük harf ya da rakam). Kütle olarak kalın kesilir, taranmaz. Betonarmenin birleşik kesitine girmez. |
| 3B | `IfcCovering`'in rengi (Döşeme sınıfı) |
| IFC | `IfcCovering`, tipi (COPING, MOLDING …), adı EK-5'e göre `MM-KPL-<sözcük>` (MM-KPL-Silme, MM-KPL-Harpuşta), `Qto_CoveringBaseQuantities`. Dışa aktarmanın sayılarında `coverings`; `beams` profili saymaz. |
| Metraj | Kirişler çizelgesine girmez (kayıt gibi). |
| Dosya | Mimari verisi 42. Bağımsız Python okuyucusu da tanır. |

### §3.3 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Bina modeli Mimari'nin. |
| Harita, Yol, CBS | yok | Bina modeli yok. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | Aynı komut: `cad.beams.create`'in `covering`'i; doğrulama, plan ve sonuç arayüzle aynı. |

## §4 Kanıt (1. bölüm)

- **Ortak durumlar:** `cad.beams.create` 82 durum, 10'u yeni. Bağımsız başvuru: `scripts/fixtures/arch_command_cases.py`.
  - Geçerli: cephe silmesi; taş harpuşta, katıyla; L eksenli kullanıcı tanımlı subasman silmesi.
  - Ret: tabansız profil, lento profil, temel profil, kayıt profil, listede olmayan tip, adsız kullanıcı tanımlı, eğimli
    profil.
- **Bekçi** `frame_joins::a_profile_covers_a_wall_unjoined_and_is_the_ifc_s_covering`:
  - Harpuşta `MM-HARP-…`, silme `MM-CEPH-…` katmanında.
  - Harpuşta 1,00 ile 1,05 arasında, 1,00 m'lik parapetin tepesinde. Duvar 1,00 m kalır.
  - Kolonla kesişen silmenin hacmi bütün: 6 × 0,05 × 0,2.
  - IFC'de iki `IfcCovering` (COPING, MOLDING), `MM-KPL-` adlı; kiriş sayısı 0.
- **Dosya:** `arch_elements::a_profile_round_trips_under_arch_data_42`. Mimari verisi 42 ile aynen okunur; tabansız
  profil yazılırken reddedilir.
- **Sınamalar:** sözleşme, alan, etkileşim, IFC (görünüm özelliğiyle 112), `kentos-kcad` (290), uygulamanın birleşim,
  kesit ve IFC sınamaları.

## Kalanlar

- **2. bölüm:** Duvar profili aracı.
  - Silme: binanın içine tıklanınca dış yüzler boyunca.
  - Harpuşta: duvara tıklanınca tepesinde.
  - Şerit, simge, gerçek pencere.
- **Profilin kesiti:** yalnız dikdörtgen. Yuvarlak ve pahlı silme ileride.
- **Söve** (açıklığın çevresindeki bant) ve oyuk yok. Duvarda boyuna oyuk bugün Niş ile açılabilir.
- **Kaplama çizelgesi** (ADR 8070 §11) yalnız döşeme kaplamalarını listeler, profilleri listelemez.

## Uygulama

- `crates/shared/contracts/src/arch/{beam.rs, covering.rs}`: `covering`, kurallar; `Covering::{of, element_of,
  facade_moulding, coping}`.
- `crates/shared/commands/src/arch/cad_arch.rs`: `NewBeam.covering`. `crates/native/application/src/arch/arch_elements.rs`.
- `crates/native/application/src/arch/sections/frame.rs`: birleşimin dışında, `profile_look`.
- `crates/native/domain/src/wall_height.rs`: profil duvarı kesmez. `crates/native/application/src/arch/spaces_quantities.rs`.
- `crates/shared/ifc/src/arch.rs`: `Covering::of` ile `IfcCovering` (satır sayısı aynı).
- `crates/shared/kcad/src/{lib.rs, decode.rs}`, `tools/kcad/kcad.py`: Mimari verisi 42.
- `scripts/fixtures/arch_command_cases.py`, `fixtures/commands/v1/cad.beams.create.json`.
