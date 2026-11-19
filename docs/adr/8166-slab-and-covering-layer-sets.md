# ADR 8166: Döşemenin ve kaplamanın katman takımı ve kullanımı — IFC

- **Durum:** uygulandı (19 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 ("Kalan: kaplamanın katman
  takımı") ve IFC-04 ("Katmanlı malzeme takımı ve kullanımı (duvar, döşeme, çatı, kaplama)"; ADR
  [8105](8105-wall-style-types-and-layer-sets.md) §6, [8106](8106-element-types-and-their-exceptions.md) §6).
- **Bağlam:** ADR 8040 §2 (döşemenin katmanları: üstündekiler ve altındakiler, üstten), 8070 §10 (kaplama: döşeme
  aracının plakası, `IfcCovering`), 8071 §8 (çatının katman takımı), 8105 (duvar stilinin katman takımı; kullanım
  yazılmadı), 8106 (eleman tipleri takımı üyeleri paylaşınca alır), 8127 (kaplamanın mahali).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-6 Tablo 6.20 (`IfcCovering`: Kaplama Malzemesi, Kaplama
  Kalınlığı) ve Tablo 6.56 (`IfcSlab`: Malzeme); IFC 4.3 `IfcMaterialLayerSetUsage`, `IfcMaterialLayer`, `IfcCovering`.
- **Sözleşme:** değişmedi (yeni komut yok). `kentos_ifc::building::Building.layer_usages`. **`.kcad`:** değişmedi.
  **Web:** dokunulmadı.

## §1. Sorun

Döşemenin katmanları (kaplama, şap, yalıtım, sıva) çizimde ve kesitte vardı, IFC'de yoktu: döşeme yalnız taşıyıcısının
malzemesiyle (`IfcMaterial` Betonarme) yazılıyor, gövdesi ise katman katman uzatılmış katılardı. Kaplama (asma tavan,
zemin kaplaması) da yalnız malzemesinin adıyla yazılıyordu; EK-6 Tablo 6.20'nin kaplama malzemesi ve kalınlığı satırları
kaplama nesnesinde karşılıksızdı.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcMaterialLayerSetUsage` (buildingSMART IFC4.3.x-development) | `IfcSlab` ve `IfcPlate` için LayerSetDirection AXIS3 (z ekseni); DirectionSense katmanların taban çizgisinden +z ya da −z yönünde dizilişi; OffsetFromReferenceLine taban çizgisinin başvuru düzleminden uzaklığı; takımın tabanı uzatılan profille tutarlı | Düz döşemede kullanım: AXIS3, NEGATIVE (üstten aşağı), taban çizgisi döşemenin üstü (K430) |
| Aynı belge | Dik olmayan uzatmada (eğik döşeme) kalınlık ve ofset dik ölçülür | Eğik döşeme takımı alır, kullanımı yazılmaz (K430; §6) |
| IFC 4.3 `IfcMaterialLayer` | Category anahtar sözcükleri: 'LoadBearing', 'Insulation', 'Inner finish', 'Outer finish' | Taşıyıcı LoadBearing, ısı yalıtımı Insulation; üst kaplama döşemede iç, balkon ve terasta dış; alt sıva odaya bakıyorsa iç, açık havaya bakıyorsa dış (K428) |
| IFC 4.3 `IfcCovering` | Yüzey kaplamaları (FLOORING, CEILING, CLADDING) katmanlarla tanımlanan malzeme alabilir | Her kaplama bir katman takımı (K429) |
| EK-6 Tablo 6.20 ve dipnotu | Kaplama Malzemesi ve Kalınlığı `IfcRelAssociatesMaterial > IfcMaterialLayerSetUsage > IfcMaterialLayerSet > IfcMaterialLayer` ile; ya `IfcSpace`'in tanımlarıyla ya `IfcCovering`'in ilişkileriyle, ikisinden biri zorunlu | Mahal yolu 8070 §6'dan beri var; kaplama nesnesi yolu da (K429) |
| EK-6 Tablo 6.56 | `IfcSlab`'ın Malzeme'si `IfcRelAssociatesMaterial > IfcMaterial`, "Malzeme İsmi_Dayanım" | Katmansız döşeme tek malzemesini korur; katmanlı döşemenin taşıyıcı katmanı aynı malzeme (K428) |
| ADR 8105 K181 | Duvarlar kendi eksen çerçevesinde yazılmadığı için kullanım yazılmadı (AXIS2 eğik duvara dik olmazdı) | Döşemenin z ekseni dünyanınki ve katınki: kullanım bugün doğru yazılabilir (K430) |

## §3. Karar

- **K428 — Döşemenin katman takımı:** katmanı olan döşeme (8040 §2) `IfcMaterialLayerSet` "Döşeme katmanları" alır,
  tek malzemesinin yerine: üstten, taşıyıcının üstündeki katmanlar, taşıyıcı ("Taşıyıcı", döşemenin malzemesi ve
  kalınlığı, LoadBearing), altındaki katmanlar; her katman türünün adı (Kaplama, Şap, Isı yalıtımı, Su yalıtımı, Sıva),
  malzemesi (yazılmadıysa söylenmez) ve kalınlığı; rolü: Isı yalıtımı Insulation, üst Kaplama döşemede 'Inner finish',
  balkon ve terasta 'Outer finish', alt Sıva altı açık havaysa (balkon, dış beyanlı döşeme, 8124) 'Outer finish', yoksa
  'Inner finish'; şap ve su yalıtımının rolü söylenmez. Katmansız döşeme tek `IfcMaterial`'ını korur (EK-6 Tablo 6.56).
- **K429 — Kaplamanın takımı:** her kaplama (8070 §10) "Kaplama katmanları": plakası (türünün sözü — Asma tavan, Zemin
  kaplaması —, malzemesi ve kalınlığı; rolü söylenmez) ve varsa katmanları, aynı kuralla.
- **K430 — Kullanım:** düz döşeme ve kaplama takımıyla değil `IfcMaterialLayerSetUsage(takım, .AXIS3., .NEGATIVE.,
  üstü, $)` ile ilişkilendirilir: taban çizgisi döşemenin üstü (bitmiş döşeme, katının kotundan ofseti), katmanlar
  oradan aşağı; takımın kalınlığı gövdenin kalınlığıdır (düşey modelin katman katman katıları). Aynı takımı ve üstü
  olanlar bir kullanımı paylaşır; aynı takım tek kez yazılır; eleman tipi (8106) üyeleri paylaşıyorsa takımı tipe de
  bağlar. Eğik döşeme takımla ilişkilendirilir, kullanımı yazılmaz.
- **K431 — Dosyadan okunan malzeme:** okuyucu (`read::material`) katman takımında takımın adını verir: katmanlı döşemenin
  malzemesi dosyadan "Döşeme katmanları" okunur (duvarda stilin adı gibi, 8105). EK-6'nın zorunlu satırı çizimin
  beyanından gelir, değişmedi; IDS'nin malzeme yüzü takımın malzemeleriyle tutar.
- **K432 — Görüntüleyicinin ayrıntısı:** IFC modeli'nin Özellikler panelinde katman takımı olan nesnenin "Malzeme
  katmanları" grubu: kullanımı varsa Diziliş ("z ekseni, eksi yön") ve Taban çizgisi, sonra her katman sırasıyla adı ve
  kalınlığı (projenin uzunluk birimiyle); malzemesi verilen katmanlar ayrı "Katmanların malzemesi" grubunda (ad ve
  malzeme yan yana panele sığmaz). Dosyadan okunan (`read::material::layers`), çevrilmez. Panelin kaydırma çubuğu
  birim sütununun yanında yer alır (uzun sayfa artık birimleri örtmez).

## §4. Uygulama

- `crates/shared/ifc/src/arch.rs` (döşemelerin ve kaplamaların takımları, `layer_usages`), `building.rs`
  (`Building.layer_usages`, `IFCMATERIALLAYERSETUSAGE`, kullanıma ilişkilendirme).
- `crates/shared/ifc/src/read/material.rs` (`LayerList`, `Model::layers`), `read/details.rs` (Malzeme katmanları,
  Katmanların malzemesi); `crates/model-view-ui/src/viewer_view.rs` (kaydırma çubuğunun yeri).
- `scripts/fixtures/slab_layers_check.py` (yeni bağımsız denetim), `scripts/fixtures/arch_ifc_check.py` (katmanlı
  döşemenin malzemesi tek LoadBearing katmanınınki).

## §5. Sınamalar

- **Birim** (`building::a_level_slab_s_set_is_laid_from_its_top`): üç döşeme tek takımı paylaşır; aynı üstlü ikisi tek
  kullanımı, öbürü (−0,30) ikinci kullanımı; hiçbiri takımın kendisine bağlanmaz.
- **Uygulama** (`arch_ifc::a_layered_slab_is_its_build_up_laid_from_its_top`): kesit binasının katmanlı her döşemesi
  takımını alır, katmanların toplamı bütün kalınlığı, taşıyıcı katman Betonarme 0,15 LoadBearing, kullanımın üstü
  döşemenin ofseti; zemin döşemesi Kaplama 0,02 (Inner finish), Şap 0,05, Isı yalıtımı 0,05 (Insulation), Su yalıtımı
  0,01, Taşıyıcı 0,15. Kaplama sınaması: asma tavan (Alçıpan 0,0125, üstü 2,60) ve zemin kaplaması (0,02) tek katmanlı
  takımlarını ve kullanımlarını alır.
- **Altın dosyalar:** `kesit-binasi*.ifc`, `yerel-sistem.ifc`, `egri-bina*.ifc` yeniden yazıldı (yalnız döşemelerin
  malzeme ilişkileri; GlobalId'ler aynı); ondan türeyen `kesit-binasi-kapi-eksik.ifc` (`ifc_missing_door_case.py`,
  doğrulayıcı bulgusu 0); `kesit-binasi-tipli.json` çizimdeki katmanlı döşemeleri de taşır; olgular (5) döşemenin okunan
  malzemesini "Döşeme katmanları" verir; okuma durumları (6) kayıt sayıları; `ids_check` beş dosyada KentOS'un
  kuralıyla aynı. Sahnesi her koşuda yeni
  kimlik üreten `blok-a/b`, `cati-alti-mahal`, `egimli-doseme`, `katmanli-cati` yeniden yazılmadı.
- **Okuyucu** (`read::details`): kesit binasının zemin döşemesi Diziliş "z ekseni, eksi yön", Taban çizgisi 0 m,
  Kaplama 0,02 · Şap 0,05 · Isı yalıtımı 0,05 · Su yalıtımı 0,01 · Taşıyıcı 0,15 m, malzemesi verilen yalnız Taşıyıcı
  (Betonarme); duvarın takımında Diziliş yok, stilin dört katmanı.
- **Resimler:** `.run/gui-evidence/doseme-katmanlari-8166/`: IFC modeli'nde teras döşemesi seçili, Özellikler'de
  Malzeme katmanları (Taban çizgisi 3,000 m; altı katman) ve Katmanların malzemesi; açık ve koyu.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0): `slab_layers_check.py` "uyuyor" (üç döşeme: kullanım AXIS3, NEGATIVE,
  taban çizgisi üstü; katmanlar çizimdeki sırasıyla ve kalınlığıyla; gövde üstten takımın kalınlığınca); `arch_ifc_check`
  beş dosyada 0 doğrulayıcı bulgusu; `ifc_types_check`, `curved_ifc_check` geçti; IfcTester (`ids_ifctester_check
  --check`) beş dosyada saklananla aynı, ayrışan 0.

## §6. Kalanlar

- Duvarın kullanımı (AXIS2) duvarlar kendi eksen çerçevelerinde yazılınca (8105 §6); çatının ve eğik döşemenin kullanımı
  (dik olmayan uzatma).
- Kaplamanın bağlı olduğu elemanla ilişkisi (`IfcRelCoversBldgElements`).
- Kolon ve kiriş tipinin profil takımı (`IfcMaterialProfileSet`), çok kata uzanan eleman (IFC-04).
