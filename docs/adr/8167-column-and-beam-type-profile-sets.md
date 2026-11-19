# ADR 8167: Kolon ve kiriş tipinin kesiti — IFC profil takımı

- **Durum:** uygulandı (19 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-04 ("Kalan: kolon ve kiriş
  tipinin profil takımı (`IfcMaterialProfileSet` ve kullanımı)"; ADR [8106](8106-element-types-and-their-exceptions.md)
  §6), [bim-plan.md](../../plan/bim-plan.md) B21 ("profil takımı").
- **Bağlam:** ADR 0725 (kolon: dikdörtgen, daire, L; bacakları), 8041 (kiriş: genişlik, yükseklik, rol), 8106 (eleman
  tipleri: proje tipi IFC tipi, elemanları ona bağlı; takım üyeleri paylaşınca tipin), 8124 (kolonun taşıyıcılık beyanı),
  8166 (döşemenin katman takımı; görüntüleyicinin ayrıntısı).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-6 Tablo 6.3 (`IfcBeam`) ve 6.16 (`IfcColumn`): Malzeme
  `IfcRelAssociatesMaterial > IfcMaterial` adıyla ("Malzeme İsmi_Dayanım"); IFC 4.3 `IfcMaterialProfileSet`,
  `IfcMaterialProfile`, `IfcMaterialProfileSetUsage`, `IfcRectangleProfileDef`, `IfcCircleProfileDef`,
  `IfcArbitraryClosedProfileDef`.
- **Sözleşme:** değişmedi. `kentos_ifc::building::{ProfileSet, Section}`, `TypeObject.profile_set`;
  `read::material::{ProfileLine, Model::profiles}`. **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Kolon ve kiriş tipleri (8106) IFC'de tipti ama kesitsizdi: tipin kesiti (40×40, Ø50, 25×60) ve malzemesi dosyada
yalnız örneklerin gövdesinden okunabiliyordu; bir program tipin kesitini göremiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcMaterialProfileSetUsage` (buildingSMART IFC4.3.x-development) | CardinalPoint kesitin üyenin boyuna eksenine göre yerini söyler; profiller eleman geometrisine sığmalı, kullanımın yeri gövdeyle uyumlu olmalı | KentOS kolonu ve kirişi kendi eksen çerçevesinde değil kat kökünde yazar (8105 K181 gibi): kullanım yazılmaz (K433; §6) |
| EK-6 Tablo 6.3, 6.16 | Kolonun ve kirişin Malzeme'si `IfcRelAssociatesMaterial > IfcMaterial` adı | Örnekler `IfcMaterial`'larını korur; kesit tipte (K433) |
| IFC 4.3 `IfcMaterialProfile` | Material, Profile, Category | Tipin tek profili: malzemesi örneklerinki, rolü LoadBearing (taşımaz beyanlı kolonda söylenmez) (K434) |
| IFC 4.3 profil tanımları | `IfcRectangleProfileDef` (XDim, YDim), `IfcCircleProfileDef` (Radius), `IfcArbitraryClosedProfileDef` (kapalı eğri) | Dikdörtgen kolon en × boy, daire kolon yarıçap, L kolon dış hattı (kutunun ortasına göre), kiriş genişlik × yükseklik (K434) |
| ADR 8106 | Tipin katman takımı, üyeleri paylaşıyorsa | Kesit de üyeleri aynı kesiti paylaşıyorsa tipin (K433) |

## §3. Karar

- **K433 — Kesit tipte:** proje tipinin IFC tipi `IfcColumnType` ya da `IfcBeamType` ise ve bütün elemanları aynı kesiti
  (biçim, ölçüler, malzeme, rol) paylaşıyorsa tip bir `IfcMaterialProfileSet` alır (adı tipin adı) ve onunla
  ilişkilendirilir. Örnekler kendi `IfcMaterial`'larıyla kalır (EK-6 Tablo 6.3, 6.16'nın yolu); kullanım
  (`IfcMaterialProfileSetUsage`) yazılmaz: gövdeler kendi eksen çerçevesinde değildir. Dikme ve kayıt (`IfcMember`)
  kesit almaz.
- **K434 — Kesit:** dikdörtgen kolon `IfcRectangleProfileDef` (XDim en, YDim boy), daire kolon `IfcCircleProfileDef`
  (yarıçap), L kolon `IfcArbitraryClosedProfileDef` (bacakları kutunun sol alt köşesinden, 0725 §3; kutunun ortasına
  göre); kiriş dikdörtgen (XDim genişlik, YDim yükseklik). Kesitin adı çizimlerin yazdığı gibi santimetreyle: "30×30",
  "Ø50", "L 50×50", "25×50". `IfcMaterialProfile`'ın malzemesi örneklerinki, rolü 'LoadBearing' (taşımaz beyanlı kolonda
  yok).
- **K435 — Görüntüleyici:** IFC modeli'nin Özellikler'inde nesnenin kendi profil takımının kesiti "Kesit", yoksa tipininki
  "Kesit (tipten)": Ad, Biçim (Dikdörtgen, Daire, Serbest biçim), XDim, YDim ya da Radius (projenin uzunluk birimiyle),
  Malzeme; dosyadan okunur (`Model::profiles`).

## §4. Uygulama

- `crates/shared/ifc/src/building.rs` (`ProfileSet`, `Section`, `TypeObject.profile_set`; profil, malzeme profili,
  takım ve tipe ilişkilendirme), `arch.rs` (`section`; eleman tiplerinde üyelerin ortak kesiti).
- `crates/shared/ifc/src/read/material.rs` (`ProfileLine`, `Model::profiles`), `read/details.rs` (Kesit, Kesit (tipten)).
- `scripts/fixtures/ifc_types_check.py` (tiplerin kesiti).

## §5. Sınamalar

- **Birim** (`building::a_column_type_s_section_is_its_profile_set`): 40×30 dikdörtgen, Ø50 daire ve L 50×50 serbest
  profil; malzeme bir kez; üç takım, her biri yalnız tipine bağlı; kolonun kendi malzemesi `IfcMaterial`.
- **Uygulama** (`element_types`): tipli kesit binasında kolon ve kiriş tiplerinin her biri kesitini alır (adı tipin,
  Betonarme, LoadBearing); takım sayısı bu tiplerin sayısı. Altın dosya (`kesit-binasi-eleman-tipli.ifc`) yeniden yazıldı:
  yalnız "Kolon 1" (30×30) ve "Kiriş 1" (25×50) takımları ve ilişkileri, GlobalId'ler aynı; not çizimdeki kesitleri taşır.
- **Okuyucu** (`read::details::a_typed_column_shows_its_type_s_section`): tipli kolonun ayrıntısında Kesit (tipten): 30×30,
  Dikdörtgen, XDim 0,3 m, YDim 0,3 m, Betonarme; duvarda kesit yok.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0): `ifc_types_check` "uyuyor" (Kolon 1: IfcRectangleProfileDef 0,3 × 0,3;
  Kiriş 1: 0,25 × 0,5; ikisi de Betonarme, LoadBearing; elemanların kendi malzemesi IfcMaterial); `arch_ifc_check`
  tipli dosyada 0 doğrulayıcı bulgusu.
- **Resimler:** `.run/gui-evidence/kolon-kesiti-8167/`: IFC modeli'nde tipli kolon seçili, Özellikler'de Kesit (tipten);
  açık ve koyu.

## §6. Kalanlar

- Kullanım (`IfcMaterialProfileSetUsage`, CardinalPoint) kolon ve kiriş kendi eksen çerçevelerinde (eksen gösterimiyle)
  yazılınca.
- Dikme ve kaydın (`IfcMember`) kesiti; yay kirişin kesiti aynı dikdörtgen.
- Örneğin kendi kesiti (tipinden ayrılan istisna): bugün tipin kesiti yalnız üyeler paylaşınca yazılır.
