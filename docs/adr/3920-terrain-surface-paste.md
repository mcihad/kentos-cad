# 3920 — Yüzey yapıştırma (`.kcad` şema 214)

Tarih: 2026-11-09. plan/map-cad.md MAP-14, yedinci tur. Durum: taslak. ADR 1560'ın “Sonra”sındaki “yüzey birleştir” ve
ADR 2320, 2720, 3120, 3520'nin “Yüzey birleştirme”si.

İlgili:
- [ADR 0900](0900-road-terrain-surface.md): yüzey tarifi, kısıtlı Delaunay, özet;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita projesinde yüzey;
- [ADR 1560](1560-terrain-surface-edits.md): tarifin sıralı düzenlemeleri (`.kcad` şema 196).

## Dayanak

- **Civil 3D 2023 kılavuzu, “To Paste Surfaces”**
  (<https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-38871FA0-E072-4F9B-8EEB-C102670C53D2.htm>,
  9 Kasım 2026'da okundu):
  - yapıştırılan yüzey, alan yüzeyinin “kapladığı kısmının yerine geçer”; üst üste gelen yer yapıştırılanın kotunu ve
    eğrilerini alır;
  - yapıştırma yüzey verisinin değişikliği olarak değil, Prospector'daki Edits listesine bir “Paste Surface” öğesi olarak
    eklenir; açıklaması yapıştırılan yüzeyin adıdır;
  - adımlar: yüzeyin Definition'ında Paste Surface, Select Surface To Paste'te yüzey seçilir.
  - Sayfa sıranın ya da yapıştırılan değişince ne olacağının sözünü etmez.
- **OpenRail Designer (OpenRoads ailesi) kılavuzu, “Create Complex Terrain Model”**
  (<https://docs.bentley.com/LiveContent/web/OpenRail%20Designer%20Help-v15/en/Createcomplexterrainmodel.html>,
  9 Kasım 2026'da okundu):
  - Merge'de ortak alanda birincil modelin verisi atılır, yalnız birleşen modelinki kullanılır; örnekte ortak kenarların
    çevresi yeniden üçgenlenir;
  - Append'de iki modelin bütün verisi birlikte üçgenlenir;
  - modeller listedeki sırayla işlenir; birleştirmede sıra kritiktir.
- **KentOS bugün** (ADR 1560): bir yüzey yalnız kendi kaynaklarından ve düzenlemelerinden kurulur. Bir dolgu, platform
  ya da tasarım yüzeyini zemine katmak için kaynak nesneleri elle bir araya getirmek, altta kalan zemin noktalarını elle
  ayıklamak gerekiyordu.

## Karar

1. **Yeni düzenleme: Yüzey yapıştır** (`{"op": "paste", "terrain": "<kimlik>"}`), tarifin sıralı düzenlemelerinden biri
   (Civil 3D'nin Edits › Paste Surface'ı, OpenRoads'un Merge'ü). Projenin başka bir yüzeyini kimliğiyle adlandırır.
2. **Kural** (kesin; çekirdeğin yön yüklemi, `crates/shared/road/src/paste.rs`):
   - yapıştırılan yüzey kendi başına kurulduğu gibi kurulur: kaynakları, düzenlemeleri, sınırı ve boşluklarıyla;
   - kapladığı yer, tutulan üçgenlerinin içi ve kenarlarıdır;
   - o düzenlemeye kadar toplanan nokta kayıtlarından kaplananlar gider;
   - bir kırık hattın parçası, iki ucu da kaplanmıyor ve yapıştırılanın dış çizgisine (yalnız bir tutulan üçgenin
     kenarları) değmiyorsa kalır; değmek de sayılır. Kalan parçalar ardışık koşulara bölünür. Parçası giden kapalı halka
     açılır, ilk gidenden sonraki parçadan başlar. Parçası olamayacak kadar kısa kırık hat, bir köşesi kaplanınca gider;
   - yapıştırılanın köşeleri (tutulan üçgenlerinin köşeleri) kotlarıyla nokta kaydı olarak gelir, yere göre sıralı
     (önce x, sonra y). Kenarlarının her biri iki köşeli bir kırık hat olur, küçük köşesinden başlar, köşelerine göre
     sıralanır. Böylece üçgenleri aynen kurulur, çevresi yeniden üçgenlenir (OpenRoads'un “ortak kenarların çevresi”);
   - **sıra:** önceki düzenlemelerin kaplanan kayıtları gider. Sonrakiler yapıştırılan köşelere de uygulanır: Kot değiştir
     köşenin kotunu değiştirir, Nokta ekle boş yere ekler;
   - yüzeyin kendi sınırı, boşlukları ve en uzun dış kenarı bütün yüzeye uygulanır. Yapıştırılan kenarlar kısıtlıdır, en
     uzun kenar onları atmaz;
   - yapıştırılanın sildiği üçgenin yeri kaplanmaz: orada yüzeyin kendi kayıtları ve yapıştırılan köşeler üçgenlenir;
   - dışarıda bırakılan noktalar (ADR 0916) yalnız kaynak kayıtlarına uygulanır.
3. **Yapılamayan yapıştırma** silinmez, söylenir (`left`, rol `edits`; ADR 1560 §3):
   - `no_surface`: projede o kimlikli yüzey yok;
   - `cycle`: yapıştırılanın yapıştırmaları, doğrudan ya da başka yüzeyler üzerinden, bu yüzeye döner. Karar yalnız
     tariflerden verilir: bir yüzey nereye yapıştırılırsa yapıştırılsın aynıdır. Birbirini yapıştıran iki yüzeyin ikisi
     de söyler, hiçbiri öbürünü yapıştırmaz;
   - `unbuildable`: yapıştırılan yüzey kaynaklarından kurulamıyor.
4. **Ret** (`invalid_terrain`, `surface.edits[k]`): yüzey kendini yapıştıramaz; yapıştırılacak yüzeyin kimliği boş olamaz.
5. **Özet:** yapıştırılan köşeler ve kenarlar yüzeyin girdisindedir. Yapıştırılan yüzeyin kaynakları değişince
   yapıştıran yüzeyin özeti de değişir, yüzey eskir (ADR 0900'ün “eskidi”si).
6. **`.kcad` şema 214:** yazıcı bir yüzey yapıştırma taşıyınca 214'ü seçer. 213 ve öncesi düzenlemeyi tanımaz
   (`unknown_field`); kendini yapıştıran ve kimliği boş yapıştırma `bad_value`.
7. **Masaüstü** (Arazi yüzeyleri › Düzenlemeler):
   - Yapıştırılacak yüzey kutusunda kaydedilmiş öbür yüzeyler adlarıyla listelenir; Yüzey yapıştır düğmesi düzenlemeyi
     sona ekler (ikon `surfacePaste`);
   - tabloda İşlem “Yapıştır”, Yer “Dolgu” yüzeyi; durumu ve yapılamazsa nedeni yazılır;
   - tablo artık pencereye sığar, yatay çubuk açmaz (sütunlar sözcüklerin genişliğine indi).
8. **Python** `kentos.road.paste(doc, into, surface)`; MCP ve SDK `road.terrains.edit` (`PasteSurfaceEdit`).
9. **Kapsam işareti:** ADR 1520'deki gibi **Harita + Yol**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/road_terrain_command_cases.py`; Triangle ve kesirler, KentOS kodu yok):
  - dolgunun topuğu ve üstü `dolgu` katmanında kapalı kırık hatlar; yalnız yapıştırma durumlarının çiziminde;
  - kural betikte yeniden yazıldı;
  - dolgunun üçgenlemesinin tek olduğu doğrulanır: iki üçgenin paylaştığı her kısıtsız kenar kesin Delaunay'dır, dördüncü
    köşe çemberin üstünde değildir. Kenarları özete girdiği için şarttır;
  - `road.terrains.edit`'e yedi durum (46): yapıştırma, sonra verilen kot, önce eklenen noktanın gitmesi, olmayan yüzey,
    kurulamayan yüzey, döngü, iki ret. `road.terrains.query`'ye bir durum (22): dolgunun üstünde, şevinde, dışında kot;
  - eski durumlar bayt bayt aynı; Rust ve Python geçer;
  - not: zeminin sınır köşeleri (2, 2), (38, 2) ile kırık hattın uçları (10, 5), (30, 5) bir çemberin üstünde. O şeritte
    iki üçgenleme vardır; betik sorduğu yerlerin onun dışında olduğunu doğrular.
- **Çekirdek** (`crates/shared/road`):
  - `paste.rs`'in birim testleri: kaplama, dış çizgi, koşular;
  - `tests/all/paste.rs`: dolgunun üçgenleri aynen girer, yanındaki kırık hat kalır, köşesine değen gider; döngü,
    olmayan ve kurulamayan yüzey söylenir; dolgu değişince eskir; sonraki kot köşeye uygulanır.
- **`.kcad`:**
  - bağımsız yazıcı ve okuyucu 1722 dosyada uyuşur;
  - yeni geçerli `surface-pastes`; bozuk `surface-pastes-in-schema-213`, `-itself`, `-no-id`;
  - çok yeni dosya artık `schema-version-215`;
  - Rust kodlayıcısı yeni dosyayı bayt bayt yazar.
- **Python** (`python/tests/test_surface_paste.py`): yapıştıran yüzey yapıştırılanın üçgenleri. İçlerinde kotlar aynı,
  alan 1200 m². Yapıştırılanın sildiği üçgenin yeri dolar (bir üçgen fazla). Geri yapıştırma `cycle`, kendini yapıştırma
  ret.
- **MCP** (`crates/native/mcp/tests/surface_paste.rs`): aynısı yapay zekâ sunucusu üzerinden.
- **Masaüstü:**
  - testler: `terrain_window_tests::a_saved_surface_is_pasted_into_another_and_kaydet_writes_the_paste`,
    `a_paste_back_is_a_cycle_and_a_surface_does_not_paste_itself`;
  - Arazi yüzeyleri, ribbon, ayarlar, katalog ve geçmiş metni denetimleri geçer;
  - resimler `.run/gui-evidence/yuzey-yapistir-3920/`, ikon seçenekleri `.run/gui-evidence/ikon-yuzey-yapistir/`.

## Sonra

- Bölgesel yeniden kurma ve bağlı eğri/profil/hacmin yüzey revizyonuna bağlanması (MAP-14'ün öbür maddesi). Yalnız
  değişenin yazılması [ADR 4320](4320-regional-redraw-of-surface-drawings.md)'de; pafta eğrilerinin bağı açık.
- Append'in karşılığı (iki yüzeyin bütün verisinin birlikte üçgenlenmesi); kaynak ve kullanım araştırılmalı.
- Yapıştırılanın yalnız bir alanının yapıştırılması; kaynak araştırılmalı.
- Gerçek veride süre: her yapıştırma yapıştırılanı yeniden kurar.
- Web (TODOS §0.2).
