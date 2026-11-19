# ADR 8170: Dikme ve kayıt tipinin kesiti — IFC profil takımı

- **Durum:** uygulandı (19 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-04 ("Kalan: dikme ve kaydın
  kesiti"; ADR [8167](8167-column-and-beam-type-profile-sets.md) §6), [bim-plan.md](../../plan/bim-plan.md) B21.
- **Bağlam:** ADR 8070 §8 (dikme kolonun, kayıt kirişin eleman rolüyle `IfcMember`; dikme çizimin bildiği bir
  taşıyıcılık söylemez), 8106 (eleman tipleri: dikmenin ve kaydın tipi `IfcMemberType`), 8130 (eğimli eleman), 8167
  (kolon ve kiriş tipinin kesiti profil takımında; örnekler `IfcMaterial`'larını korur; kullanım yazılmaz).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-6 Tablo 6.40 (`IfcMember`: Malzeme
  `IfcRelAssociatesMaterial > IfcMaterial` adıyla; "taşıyıcı olması zorunlu değildir"); IFC 4.3 `IfcMember`,
  `IfcMemberType`, `IfcMaterialProfileSet`, `IfcMaterialProfile`.
- **Sözleşme:** değişmedi. **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Kolon ve kiriş tipleri kesitlerini profil takımı olarak alıyordu (8167), dikme ve kayıt tipleri (`IfcMemberType`)
almıyordu: bir okuyucu dikme tipinin 10×10'unu ya da kayıt tipinin 5×10'unu dosyada göremiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcMember` (buildingSMART IFC4.3.x-development), Material Profile Set Usage | Süpürülmüş profilli eleman profil takımını tipinde ve örneğinde taşır, takım ve profil bütün örneklerce paylaşılır; kullanımı olan örnek kesitin yerinde bir 'Axis' ve süpürülmüş gövdeyle gösterilir | Takım tipte (K442). Kullanım yazılmaz: gövde elemanın kendi eksen çerçevesinde değil (8167 K433 gibi) |
| IFC 4.3 `IfcMember` | Eleman taşıyıcı olmak zorunda değildir | Profilin rolü (Category) söylenmez (K443) |
| EK-6 Tablo 6.40 | Malzeme `IfcRelAssociatesMaterial > IfcMaterial` adı; taşıyıcılık zorunlu değil | Örnekler kendi `IfcMaterial`'larını korur (K442) |
| ADR 8070 §8 | Dikme çizimin bildiği bir taşıyıcılık söylemez (`LoadBearing` yazılmaz) | Kesitin rolü de söylenmez (K443) |

## §3. Karar

- **K442 — Kesit tipte:** dikme (kolonun eleman rolü) ve kayıt (kirişin eleman rolü) tipinin bütün elemanları aynı
  kesiti paylaşıyorsa tipi (`IfcMemberType`) kolon ve kiriş tipi gibi bir `IfcMaterialProfileSet` alır (adı tipin
  adı): dikmenin kesiti kolonunki (dikdörtgen en × boy, daire yarıçap, L dış hat), kaydınki kirişinki (genişlik ×
  yükseklik), adı çizimlerin yazdığı gibi santimetreyle ("10×10", "5×10"), malzemesi elemanlarınki. Örnekler EK-6 Tablo
  6.40 için kendi `IfcMaterial`'larını korur; kullanım yazılmaz.
- **K443 — Rolsüz:** dikmenin ve kaydın profilinin rolü (`IfcMaterialProfile.Category`) yazılmaz: eleman taşıyıcı
  olmak zorunda değildir ve çizim bir taşıyıcılık söylemez (8070 §8).

Görüntüleyici değişmedi: Özellikler'in "Kesit (tipten)" grubu (8167 K435) dikmenin tipinin kesitini de okur.

## §4. Uygulama

- `crates/shared/ifc/src/arch.rs` (`section`: dikme ve kayıt da, rolsüz).
- `scripts/fixtures/ifc_types_check.py` (`--uyeler FILE`: dikme ve kayıt tiplerinin kesiti; kolon ve kiriş tipinin
  rolü LoadBearing, dikme ve kayıt tipininki yok).

## §5. Sınamalar

- **Uygulama** (`element_types::a_post_type_s_and_a_rail_type_s_section_is_their_profile_set`): kesit binasının
  kuzeyinde iki 10/10 Çelik dikme (birinden yapılan "Dikme 1" tipi, öbürü ona bağlı) ve 5/10 Çelik kaydın "Kayıt 1"
  tipi. İkisi de `IfcMemberType`; kesitleri "10×10" (0,1 × 0,1) ve "5×10" (0,05 × 0,1) dikdörtgen, adları tiplerinin,
  rolsüz; dosyada iki profil, ikisinin de Category'si yok; elemanlar Çelik'lerini korur. Eleman tipli altın dosya
  değişmedi (dikmesi yok); belirlenimci altın dosyalar yeniden yazımda aynı.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0, `ifc_types_check --uyeler`): iki `IfcMemberType`'ın malzemesi tek profilli
  profil takımı, adı tipin, profil `IfcRectangleProfileDef` ve XDim/YDim adındaki santimetreler, Çelik, rolsüz;
  elemanlar kendi `IfcMaterial`'larıyla; doğrulayıcı 0 bulgu. Eleman tipli altın dosyanın denetimi de uyar.
- **Resimler:** `.run/gui-evidence/dikme-kesiti-8170/`: kesit binasının kuzeyinde dört Çelik dikme ve üstünde kayıt;
  IFC modeli'nde bir dikme seçili, Tip "Dikme 1 (IfcMemberType)", Kesit (tipten) 10×10, Dikdörtgen, 0,100 × 0,100 m,
  Çelik; açık ve koyu.

## §6. Kalanlar

- Kullanım (`IfcMaterialProfileSetUsage`, CardinalPoint): dikme, kayıt, kolon ve kiriş kendi eksen çerçevelerinde
  yazılınca (8167 §6).
- Tipli örneğin `PredefinedType`'ı: IFC 4.3 tipi olan örnekte değeri tipe bırakmayı önerir; bugün ikisi de yazılır.
- Giydirme cephenin dikme ve kayıtlarının (MULLION) kesiti: tipleri yok.
