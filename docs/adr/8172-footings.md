# ADR 8172: Temel (tekil ve sürekli temel, radye)

- **Durum:** 1. ve 2. bölüm kabul edildi ve uygulandı (27 Kasım 2026).
  - 1. bölüm: tekil ve sürekli temel modelde, kesitte, 3B'de, IFC'de ve dosyada.
  - 2. bölüm: Tekil temel ve Sürekli temel araçları, şeritte Kiriş ▾'in altında (§5).
  - Kalan: radye (3. bölüm).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-10. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporu (§5):
  temel piyasadaki araçların hepsinde var, KentOS'ta yok.
- **Sözleşmeler:**
  - Kirişin türüne (`BeamRole`) iki değer eklendi: `padFooting` (tekil temel) ve `stripFooting` (sürekli temel).
  - `.kcad`'ın Mimari verisi 40 (`ARCH_WITH_FOOTINGS`). Belge şeması değişmedi.
  - `cad.arch.ifc`'nin sonucunda `footings` var; temel yoksa yazılmaz.
  - 3B görüntüleyiciye "Temel" sınıfı ve rengi eklendi.

## §1 Sorun

- **Temel yoktu.** Mimari'de temel çizilemiyordu.
  - Kesitte temel görünmüyordu (PAİY 57/2-a-6'nın kesitleri, sistem kesiti).
  - IFC'de `IfcFooting` yazılmıyordu. Oysa Dijital Hazırlama Yönetmeliği'nin zorunlu sınıflarındandır.
  - Temelin metrajı da yoktu.
- **Piyasa:** temel bütün BIM araçlarında ayrı bir araçtır (araştırma raporu §5). Revit'te tekil, sürekli ve döşeme
  temel; Archicad'de döşeme, kiriş ve duvar araçlarıyla ve sınıflandırmayla; Allplan'da "foundation slab".

## §2 Araştırma

### Türkiye mevzuatı

- **Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** (RG 5.8.2026/33331):
  - EK-2 Tablo 2.3 (Mimari disiplinin öğeleri): **Temel — TEML** (`mevzuat/bim/ek-2-dosya-isimlendirme.md`). Katman
    adı `MM-TEML-<gösterim>`.
  - EK-5 Tablo 5.1: `IfcFooting` "Temel", kategori kodu **TML**, sıra 34 (zorunlu sınıflar).
  - EK-6 Tablo 6.34 (`IfcFooting`, PDF s. 73):
    - ön tanımlı tipler: CAISSON_FOUNDATION, FOOTING_BEAM, PAD_FOOTING, PILE_CAP, STRIP_FOOTING, USERDEFINED, NOTDEFINED;
    - zorunlu alanlar: ön tanımlı tip, ad (EK-5'e göre), etiket ve malzeme ("Malzeme İsmi_Dayanım", ör. Beton_C25).
- **TBDY 2018 Bölüm 16** (temeller): resmî metin (webdosya.csb.gov.tr) bu makinede sertifika hatası veriyor; denetim
  kapatılmadı. İkincil kaynakların verdiği en küçük ölçüler doğrulanamadı. Bu yüzden ölçüler kural değil, aracın
  değiştirilebilir öntanımlılarıdır. Temel denetimi (Denetim › Mevzuat) bu bölümde yok.
- **PAİY 57/2-a-6:** mimari projede en az iki kesit. Temeli olan model kesitte temeli de gösterir.

### IFC

IFC 4.3'te tekil temel `IfcFooting` PAD_FOOTING, sürekli temel STRIP_FOOTING'dir. Miktarları
`Qto_FootingBaseQuantities`'tir: Length, Width, Height, CrossSectionArea, GrossVolume, NetVolume.

## §3 Karar

### §3.1 Model: kirişin rolü

Temel, lento ve hatıl gibi kirişin bir rolüdür (ADR 8041 §3).

- **Tekil temel** kısa bir eksen üstünde bant, yani dikdörtgen pabuçtur. Sürekli temel ekseni boyunca banttır.
- **Konumu:** kendi tabanına oturur. `baseOffset` katın tabanından negatif yüksekliktir; tepesi
  `kat + baseOffset + yükseklik`. Yüksekliği temelin kalınlığıdır. Döşemeden asılmaz: tabansız temel reddedilir.
- **Ret durumları:** temel eleman (kayıt) olamaz ve kot farkı almaz (kattan yükseklikle birlikte verilemez).
- **Neden ayrı nesne değil:** kirişin komutu, çizimi, katmanı, kesiti, 3B'si, IFC'si ve metrajı hazır. Ayrı bir nesne,
  nesne türünü bilen her yerde yeni dal isterdi.

### §3.2 Nerede ne olur

| Yer | Temel |
|---|---|
| Katman (EK-2) | `MM-TEML-TEKIL`, `MM-TEML-SUREKLI` |
| Kesit modeli | Gövdesi tabanından tepesine. Betonarmeyse kolon ve döşemeyle tek kesit (ADR 8041 §2): `MM-TASI-BETONARME`. Kolon, döşeme ve kiriş birleşimleri (ADR 8171 §2.1) temeli kesmez. Kesişen iki temelde önce çizilen baskındır. **Kolon temelin tepesine iner** (§5.5). |
| 3B | "Temel" sınıfı, beton kahvesi renk |
| IFC | `IfcFooting`, PAD_FOOTING ya da STRIP_FOOTING. Adı `MM-TML-Tekil_temel`, `MM-TML-Sürekli_temel` (EK-5 §5.5). `Qto_FootingBaseQuantities` kirişinkiyle aynı hesap, Width ve Height ile |
| Metraj | Kirişler çizelgesinde rolüyle; hacmi bütünü (döşemeden asılmaz) |
| Dosya | Mimari verisi 40. Bağımsız Python okuyucusu (`tools/kcad/kcad.py`) da tanır |

### §3.3 Öntanımlılar (araçta, §5)

Yönetmelik değeri değildir; değiştirilir.
- Tekil temel 150 × 150 cm, 50 cm yüksek.
- Sürekli temel 60 cm geniş, 50 cm yüksek.
- İkisinde de tepe katın tabanının 30 cm altında (`baseOffset` −0,80).

### §3.4 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Bina modeli Mimari'nin. |
| Harita, Yol, CBS | yok | Bina modeli yok; temel aracı ve rolü sunulmaz. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | Aynı komut: `cad.beams.create`'in `role`'ü (`padFooting`, `stripFooting`). Python türü `BeamRole` üretildi (`python/kentos/cad/types.py`); doğrulama, plan ve sonuç arayüzle aynı. |

## §4 Kanıt (1. bölüm)

- **Ortak durumlar:** `cad.beams.create` 72 durum (26'sı yeni). Bağımsız başvuru: `scripts/fixtures/arch_command_cases.py`.
  - Geçerli: tekil, döndürülmüş tekil, sürekli, L sürekli, C30, tadilatta ilave, −3 m derin, temel ile kiriş tek
    adımda, iki tekil tek adımda, renkli ve kalın; yaşam döngüsü (doğrula, plan, yaz ve geri al, gizli katman).
  - Ret: tabansız tekil, tabansız sürekli, kot farklı, kayıt temel, 3 m'den geniş, sıfır yükseklik, 12 m derin, sonsuz
    taban, tek noktalı, olmayan kat.
- **Bekçi** `frame_joins::footings_stand_under_the_floor_and_are_the_ifc_s_footings`:
  - temel −0,80 ile −0,30 arasında;
  - tekil temel 1,5 × 1,5 × 0,5;
  - sürekli temel tekilin kesişimi kadar eksik;
  - üstündeki kolon bütün;
  - IFC'de iki `IfcFooting` (PAD_FOOTING, STRIP_FOOTING), `MM-TML-` adlı, iki `Qto_FootingBaseQuantities`.
- **Dosya:** `arch_elements::a_footing_round_trips_under_arch_data_40`: Mimari verisi 40 ile yazılır ve aynen okunur;
  tabansız temel yazılırken reddedilir.
- **Sınamalar:** `kentos-native-application` kesit, IFC, metraj ve birleşim sınamaları; `kentos-kcad` (288);
  `kentos-commands` kataloğu; manifest tutarlı.

## §5 Araçlar (2. bölüm)

### §5.1 Ne yapar

- **Tekil temel** (`tool.footingPad`, kısa adı TT): tıklanan yere bir temel koyar, ortası tıklanan noktadır.
  - Bir kolona tıklanınca temel kolonun altına ortalanır ve kolonla döner. Ortayı ve dönüşü araç kolondan alır, komuta
    eksenin iki ucu gider.
  - İmleç bir kolonun üstündeyken kolon vurgulanır; önizlemenin yazısı "Kolonun altına ortalanır" der.
  - Seçenekler:
    - Tür (T)
    - Genişlik (E): eksene dik kenar
    - Boy (B): eksen boyunca kenar
    - Yükseklik (Y)
    - Kattan yükseklik (A)
    - Açı (D): kolonsuz temelin dönüşü
    - Numara (N)
- **Sürekli temel** (`tool.footingStrip`, kısa adı ST): eksen nokta nokta tıklanır, Enter ya da kısa sağ tık yazar.
  Kiriş aracı gibidir. Seçenekler:
  - Tür (T)
  - Genişlik (E)
  - Yükseklik (Y)
  - Hiza (H)
  - Kattan yükseklik (A)
  - Numara (N)
- **İkisi de aynı araçtır.** Tür (T) biri ile öteki arasında geçer ve geçerken o türün öntanımlı ölçülerini alır
  (§3.3).
  - Yazılan: `cad.beams.create`, rolü `padFooting` ya da `stripFooting`, `baseOffset` her zaman verilir.
  - Önizleme ve ret gerekçesi komutun kendi planından gelir (`beams_create::plan_in`).
  - Etkin kata, EK-2 katmanına yazar: `MM-TEML-TEKIL`, `MM-TEML-SUREKLI`. Katman yoksa açar (ADR 8008).
- **Numara** boş başlar, kiriş aracındaki gibi. Verilince her temelden sonra ilerler (T101 → T102).
- **İleti:**
  - "Tekil temel kondu: T101 150×150/50."
  - "Sürekli temel çizildi: 60/50."
  - Tekil temelin ölçüsü iki kenarıyla ve yüksekliğiyle yazılır.

### §5.2 Şerit

- **Yeri:** Mimari şeridinin Yerleştir panelinde, Kiriş ▾'in altında. Menü: Kiriş, Kayıt, Tekil temel, Sürekli temel.
- **Ayrı bir "Temel ▾" düğmesi yok:** sekmede 30 düğme var, sınır da 30 (DESIGN.md §7.3.1). Temel modelde de kirişin
  rolüdür.
- **Bulma:** komut aramasında TEMEL, TEKİLTEMEL, SÜREKLİTEMEL; kısa adları TT ve ST.
- **Simgeler:** plandaki gösterimleriyle çizildi.
  - Tekil temel: kolonun çevresinde kesikli kare.
  - Sürekli temel: duvarın altında kesikli bant.

### §5.3 Sınırlar

- **Plandaki yazı:** tekil temel 1:50'den bir kirişin yazısıyla yazılır: genişlik ve yükseklik, ör. "T101 150/50". Boyu
  yazıda yok. Kare temelde (öntanımlı) bilgi eksik değildir. Dikdörtgen temelde yazı eksiktir; temel planının yazısıyla
  birlikte ele alınacak.
- **Duvardan sürekli temel:** duvara tıklayınca altına sürekli temel çizilmiyor, eksen elle çizilir.

### §5.5 Kolon temelin tepesine iner

- **Kusur:** gerçek pencerede bulundu. Kolon katın döşemesinde başlıyordu, temelin tepesi ise öntanımlı olarak 30 cm
  aşağıda. Temelle kolon arasında boşluk kalıyordu: 3B'de temel binanın altında havada duruyordu, kesitte kolonun
  altı açıktı. Gerçekte kolon temelin tepesine kadar iner (kolon filizi).
- **Kural:** kolonun ortası aynı kattaki bir temelin üstündeyse kolonun tabanı o temelin tepesidir.
  - Birden çok temel varsa en yüksek tepeli olan geçerlidir. Temel, kolonun duracağı yerin altında olmalıdır.
  - Başka kattaki temel kolonu indirmez.
  - Kural kesit modelindedir (`sections/frame.rs`, `seats` ve `column`). 3B, IFC (gövde ve `Qto_ColumnBaseQuantities`)
    ve çizilen kesit aynı gövdeyi okur, kural bir yerde yapılır.
- **Ölçü:** 1. bölümün bekçisindeki 30/30 kolon 0,27 m³ yerine 0,297 m³. Kolon −0,30'dan katın tepesine (3,00) kadar
  uzanıyor; temelle hacim paylaşmıyor (`shares_none`).
- **Bekçi:** `frame_joins::a_column_goes_down_to_its_own_storey_s_footing_only`. Temelin üstündeki kolon −0,30'a
  iner. Temelin yanındaki kolon ±0,00'da kalır. Üst kattaki kolon kendi döşemesinde kalır.

### §5.4 Kanıt (2. bölüm)

- **Etkileşim sınamaları** (`crates/native/interaction/tests/all/footing_tools.rs`):
  - `the_pad_tool_puts_a_pad_under_the_column_it_clicks_turned_with_it`
    - 30° dönük kolonun ortasından 9 cm uzağa tıklanınca temelin ortası kolonunkidir, dönüşü 30°, boyu 1,50 m.
    - Kolonsuz yerde temel tıklanan noktaya ortalanır ve Açı kadar döner.
    - Numara ilerler; sıfır ölçü reddedilir.
  - `the_strip_tool_draws_a_strip_along_its_axis_and_enter_writes_it`
    - L eksen, Enter yazar, araç açık kalır.
    - Tür değişince ölçüler o türün öntanımlılarına döner.
  - `every_builtin_tool_can_leave_its_initial_stage_without_writing_a_draft`, kiriş ve kayıt sınamaları da geçti.
- **Katalog:** şerit `--check`, manifest (1099 satır) ve eklenti geliştirme kitinin simge listesi güncel.
- **MB-01'in artığı:** `element_types` sınamasının tipli dosyası (`fixtures/arch/v1/ifc/kesit-binasi-eleman-tipli.ifc`)
  19 Kasım'da, birleşim kuralından (ADR 8171 §2.1) önce yazılmıştı ve main'de kırmızıydı.
  - Dosya yeniden yazıldı. Bağımsız okuyucu (IfcOpenShell, `scripts/fixtures/ifc_types_check.py`) "uyuyor" diyor.
  - Tutulan BCF'te (`fixtures/ifc/v1/revizyon/tasinan-duvar.bcf`) kameranın konumu son basamakta kaymıştı
    (−0,8614680402387846 → …838). Dosya yeniden yazıldı; iki koşuda aynı.
- **Gerçek pencere:** önce ve sonra resimleri `.run/gui-evidence/mimari-bim/{once,sonra}/mb10-temel-araci.png`.

## Uygulama

- `crates/shared/contracts/src/arch/beam.rs`: `BeamRole::{PadFooting, StripFooting}`, `is_footing`, tabanın
  zorunluluğu.
- `crates/native/interaction/src/arch/arch_layer.rs`: TEML, TEKIL, SUREKLI, sözler. `beam_tools.rs`: yeni rollerin dalları.
- `crates/native/application/src/arch/sections/frame.rs`: TEML öğesi.
- `crates/shared/ifc/src/{building.rs, arch.rs, quantities.rs, read/view.rs}`: `Class::Footing`, roller,
  `Qto_FootingBaseQuantities`, okumada "Temel".
- `crates/shared/model-view/src/{input.rs, parts.rs}`: `Category::Footing`, `FOOTING` parçası.
- `crates/shared/commands/src/arch/cad_arch_ifc.rs`, `crates/native/application/src/arch/{arch_ifc.rs,
  underlay_check.rs}`, `apps/desktop-arch/src/arch_ifc.rs`: temel sayısı.
- `crates/shared/kcad/src/{lib.rs, decode.rs}`, `tools/kcad/kcad.py`: Mimari verisi 40.
- `scripts/fixtures/arch_command_cases.py`, `fixtures/commands/v1/cad.beams.create.json`.
- 2. bölüm: `crates/native/interaction/src/arch/footing_tools.rs` (`FootingTool`); kayıt `core/session.rs`;
  katalog `catalog/v1/{additions,operations,modules,names-mimari,ribbon}.json`, `scripts/ui/ribbon.py`;
  `crates/shared/extensions/sdk/icons.txt`; `docs/cad/manifest.{json,md}`.
- Kolon temelde (§5.5): `crates/native/application/src/arch/sections/frame.rs` (`Seat`, `seats`; `SlabAt` buraya
  taşındı). Boyut sınırı için konusuna göre ayrılanlar: emsal alanı `crates/shared/ifc/src/arch/emsal.rs`, düzlem
  geometrisi (`circle`, `nest`) `crates/shared/ifc/src/plane.rs`.
