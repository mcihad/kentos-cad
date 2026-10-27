# ADR 0705: Eğimden rampa ve merdiven/rampa korkuluğu

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-STAIR-02'nin ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-STAIR-02; ADR 0343 (rampa ve merdivenin modeli), ADR 0704 (yükseklikten
  merdiven, son kolun rıhtlara göre bitmesi, kesmenin üstü kesikli), ADR 0265 (yeni şema kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0343'ün rampası bir orta çizgi, kot farkı ve isteğe bağlı eğimle çizilir; çizgi eğimin istediğinden kısa ya da
uzunsa yalnız `ramp_slope_mismatch` uyarısı gelir, rampa kullanıcının tıkladığı yerde biter. Kolları bir eninden uzun
sahanlıklı U rampada iki dönüşün sahanlıkları üst üste binince eğimli uzunluktan iki kez düşülüyordu (eğim yanlış).
Merdivenin ve rampanın korkuluğu hiç çizilmiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Ramp Type Properties](https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-D0A28BE6-BDD0-43D3-A2A5-3F0C867C55E5.htm)
  ve [Revit LT 2024](https://help.autodesk.com/view/RVTLT/2024/ENU/?guid=GUID-D0A28BE6-BDD0-43D3-A2A5-3F0C867C55E5):
  rampanın en büyük eğimi (1/x) ve sahanlık gerekmeden çıkılabilecek en uzun eğimli kol tip özelliğidir.
- [Revit — Place a Railing During Stair or Ramp Creation](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-ArchDesign/files/GUID-B93D0EDC-C61B-4F5F-AACE-7AC559B79539.htm):
  korkuluk merdiven ya da rampa çizilirken seçilir; sonra eklenebilir ve değiştirilebilir.
- [Vectorworks 2025 — Inserting ramps](https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Stairs/Inserting_ramps.htm):
  rampa rise/run ile tanımlanır; sol ve sağ korkuluk ayrı seçilir; 2B'de ok yukarı, aşağı ya da yok.
- [Archicad 29 — Place Associative Railing](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-207.htm)
  ve [Railing Settings: Geometry and Symbol Display](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-212.htm):
  korkuluk merdivene bağlıdır, sınırını izler; merdivenin kesme işareti ayarını kullanır, gizli kısım kesikli çizilir.

KentOS kararı: Revit'in “eğim tip kuralı, korkuluk çizerken seçilir” ve Vectorworks'ün sol/sağ korkuluğu alınır;
Archicad'deki gibi korkuluk merdivenin kendi alanıdır (ayrı nesne değil), merdivenin yolunu izler ve kesmeyi kullanır.
Alınmayanlar: en uzun eğimli koldan kendiliğinden ara sahanlık, korkuluk yüksekliği/profili ve 3B, erişilebilirlik
yönetmeliği denetimi. Öntanımlı eğim %6 KentOS'un değeridir, yönetmelik değeri diye sunulmaz.

## Karar

### 1. Rampa eğimden: `cad.ramps.create`'in `fit`'i

`NewRamp`'in yeni isteğe bağlı `fit` alanı: `true` iken son nokta son kolun doğrultusunda taşınır, rampanın eğimli
uzunluğu |kot farkı| ÷ (eğim/100) olur (kısa çizgi uzar, uzun çizgi kısalır; inen rampa da). Retler (`invalid_ramp`,
`fit` yolunda): eğim verilmemiş, kot farkı sıfır, eğimli uzunluk son kolun sahanlığı bitmeden doluyor. Çekirdek
`geom::arch::ramp_fit_end`: son kolu bütün sahanlıklara yetecek kadar uzatıp sahanlıkların toplamını bulur, uç
başlangıçtan eğimli uzunluk + sahanlıklar kadardır.

Rampanın sahanlıkları üst üste binerse bir kez sayılır (`geom::arch::ramp_landings`); eğimli uzunluk, eğim
denetimi (`ramp_slope_mismatch`, `ramp_no_run`) ve sahanlık çizgileri bunu kullanır. Üst üste binmeyen rampalarda
sonuç ve eski ortak durumlar aynıdır.

### 2. Korkuluk: merdivenin ve rampanın `railings`'i, KCAD şema 115

Merdivenin (`StairFields`) ve rampanın (`RampFields`) yeni isteğe bağlı alanı `railings`: `left`, `right` ya da `both`,
ilk noktadan yukarı yürürken sol ve sağ; yoksa korkuluk yoktur. `cad.stairs.create`, `cad.ramps.create` ve
`cad.arch.edit`'in `set`'i (`null` kaldırır; aks ve kolonda `field_not_for_kind`) yazar. KCAD şema 115 opt-in'dir:
yazıcı 115'i yalnız korkuluk varken seçer, öteki dosyaların baytları aynıdır; 114'te korkuluk `bad_value` olur
(`railings.kcad`, `broken/railings-in-schema-114.kcad`, `broken/schema-version-116.kcad`).

Planda her seçili yanın korkuluğu bandın kenarından 5 cm içeride, dönüşlerde gönyeli bir çizgidir. Merdivende kesme
çizgisi gibi kesilir: sol korkuluk kesme rıhtında, sağ korkuluk bir basamak ileride; kesmenin üstü kesiklidir. U
merdivende iç korkuluk iki kolun arasındaki boşluğu dolanır. Rampada korkuluk bütünüyle süreklidir. DXF ve Patlat
parçaların çizgi tipini taşır.

### 3. Araçlar ve Öznitelikler

Rampa aracı: Yükseklik (Y), Eğim (E, %6; boş ya da Hesaplanır eğimi çizgiden hesaplatır), Genişlik (G), Sahanlık (H),
Korkuluk (O: Yok/Sol/Sağ/İki yan) ve Son kol (U: Eğime göre biter / Tıklanan yerde biter). Merdiven aracına Korkuluk
(O) eklendi. Değerler Öznitelikler'in “Etkin aracın ayarları”ndadır; rampanın tasarımı (eğim, eğimli uzunluk, kotlar)
etikette ve ayarların altında görünür. Seçili merdiven ve rampanın Öznitelikler'inde Korkuluk satırı vardır.

### 4. Python ve MCP

Aynı komutlar ve alanlar; `kentos.cad` katalogdan yeniden üretildi. Yeni komut yoktur.

## Sınırlar

- En uzun eğimli koldan kendiliğinden ara sahanlık, korkuluğun kol uçlarından taşması, yüksekliği ve 3B gösterimi
  açıktır.
- Rampanın başlangıç kotu katın kotuna bağlanmaz (kaynak kot değişikliği izlenmez); metraj ve pafta gösterimi ayrı
  kabul ister.
- Erişilebilirlik profili (eğim ve kol uzunluğu sınırları) ayrı, sürümlü bir profil olarak sonraki dilimdir.

## Doğrulama

- `scripts/fixtures/arch_cases.py --check`: çekirdeğin 38 durumu; yeni merdiven korkulukları (U'da kesmeyle, L, son
  rıhtta kesme, eğik) ve rampalar (üst üste sahanlık bir kez, eğime göre uç, inen, sığmayan, kot farkı sıfır, eğik,
  korkuluklar) kesirlerle. `cargo test -p kentos-geometry-core --test all arch`.
- `scripts/fixtures/arch_command_cases.py --check`: yeni `cad.ramps.create.design.json` (22 durum), merdiven tasarımına
  3 korkuluk durumu ve `cad.arch.edit.json`'a 3 korkuluk durumu; uydurulmuş uçlar kesirli başvurudan, saklanan noktalar
  çekirdeğin kayan nokta adımlarıyla bit bit; eski dosyalar aynı. Rust ve Python koşucuları geçer.
- KCAD: `railings` testi, `schemas` kuralı ve bağımsız `tools/kcad/kcad.py` (1247 dosya); Python taşıma testi.
- `cargo test -p kentos-interaction --test all stair_design`: eğime göre biten rampa ve korkuluk, merdivende korkuluk.
- Gerçek GUI resimleri (`KENTOS_SHOTS_ONLY=rampa-egim,rampa-l-korkuluk,merdiven-korkuluk,rampa-sigmaz cargo test -p
  kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`); `.run/shots/arac-rampa-*`,
  `arac-merdiven-korkuluk-*`.
