# ADR 0784: Bina ve kanat penceresi, ortak alanların bağımsız bölümlere tam payı

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-04'ün ikinci teslimi (ADV-04b) **[~]**.
- **Bağlam:** ADR 0773 §6 ve §7'nin açık bıraktıkları: pencerede bina ve kanat düzenleme, kat ile bölüm arasında hesaplı
  pay, resmî alan profili. CLAUDE.md §23.2 (yuvarlama ve dağıtım artığı açık ve sürümlü; artık son parsele verilmez,
  paylar toplamı korur).
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'ye yazılacak.
- **Şema:** `.kcad` 146 (opt-in, **geçici numara**: 144 ADR 0777'nin, 145 ADR 0779'un; 146'yı başka dilim de
  isteyebilir, birleşimde yeniden numaralanır), `FORMATS_VERSION` değişmez (dosya alışverişi değişmedi).

## Araştırma (üreticilerin resmî belgeleri; yerelde çalıştırılmadı)

- [Revit — Use Area Schemes](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-Model/files/GUID-828E8162-206D-4FB3-9866-30257CAE726F.htm),
  [Area Schemes](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-Model/files/GUID-194A8E37-06C5-426D-964C-D9148A1FEFE3.htm):
  alan şemaları (Gross Building, Rentable) ayrı alan planlarıdır; her şemanın sınırı ve çizelgesi kendinedir.
- [Revit — Area Type Rules](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-B4AC3088-DBA8-4EB0-BA8F-CAF8EC1EBF9D.htm):
  Rentable şemasında Building Common Area, Office, Store, Floor, Major Vertical Penetration türleri; sınır komşu türe
  göre duvar ekseninden ya da yüzünden alınır. Revit ortak alanı kiracılara **paylaştırmaz**; oran (BOMA'nın R/U'su)
  çizelgede kullanıcının formülüdür.
- [Archicad — Calculating Zone Area and Zone Volume](https://help.graphisoft.com/AC/21/INT/AC21Help/03_1_Elements_Virtual_Building/03_1_Elements_Virtual_Building-120.htm),
  [Zone Stamps](https://help.graphisoft.com/AC/28/INT/_AC28_Help/040_ElementsVB/040_ElementsVB-142.htm):
  bölgenin ölçülen alanı sınır öğelerinden, “Reduce Zone Area Only” ve kolon düşümleriyle; Alan Hesabı panelinde
  düşümler görünür. Ortak alan payı da yerleşik değildir.
- Sonuç: iki üründe de ortak alan payı kurala bağlı bir hesap değil, kullanıcının çizelge formülüdür; tam toplam ve artığın
  nereye gittiği söylenmez. KentOS'ta kural adlı ve sürümlü (`kentos.share/1`), tam kesirlerle hesaplanır, artığın
  kuralı yazılıdır ve paylar havuzun alanına tam eşittir. Kat ile bina bağı Revit'teki gibi paylaşılabilir (bir kat
  birden çok binada olabilir); bölümün katları yine mahallerinden okunur (ADR 0773 §1).

## Karar

### §1 Bina, kanat ve katları

- `ProgramBuilding.levels`, `ProgramWing.levels`: kapsadıkları katların kimlikleri, her biri bir kez; binası kat
  veriyorsa kanadınki onlardan seçilir (`invalid_program`). Boş liste: her katta olabilir.
- `cad.program.edit`: kaldırılan bina ya da kanat hâlâ bölümlerde yazılıysa **`building_in_use`** (programın kendi
  sorunundan önce; ileti bölümlerin numaralarını söyler, yol `program.buildings`); projede olmayan kat
  **`level_not_found`** (programın sorunundan sonra, `program.buildings[i](.wings[k]).levels[j]`).
- `cad.program.check`'in yeni bulgusu **`program_level_outside`**: bölümünün kanadı kat veriyorsa onun, yoksa binasının
  katlarında olmayan katta duran mahal (“… bölümünün binası “A Blok” bu katta değil”). Katsız mahal ve kat vermeyen bina
  bulgu vermez. Mimari denetim'in `program` grubunda.

### §2 Ortak alan payı (`kentos.share/1`)

- Programın `shareRule`'u: `equal` (eşit), `net` (bölümün kendi odalarının alanına göre), `weight` (bölümün
  `shareWeight`'i: ondalık metin, noktadan önce en çok 12, sonra en çok 6 basamak, tam okunur, hiç kayan sayıya
  çevrilmez). Kural yoksa pay yoktur; eski cevaplar değişmez.
- **Havuz:** her ortak alan (tür `common`) ayrı havuzdur. Alanı odalarıdır (şaft ve dış mahal dışı), çizim sırasıyla
  toplanıp 0,01 m² adıma yuvarlanır: kayan sayının tam ikili değeri × 100, yarıda çifte (`steps_of`; 12.345 → 1235,
  2.675 → 267, 0.125 → 12).
- **Kapsam:** kanadı varsa o kanadın, yoksa binası varsa o binanın, yoksa bütün programın bağımsız bölümleri.
- **Paylaştırma:** havuz `T` adım, ağırlıklar `wᵢ` (eşitte 1, nette bölümün odaları adımda), toplam `W`: her bölüm
  `⌊T·wᵢ/W⌋` alır; kalan adımlar (bölüm sayısından az) birer birer en büyük kalanlara, eşitlikte programdaki sıraya.
  Payların toplamı `T`'ye tam eşittir; hiçbir artık atılmaz ya da “son bölüme” verilmez. Tam pay `T·wᵢ/(W·100)` m²,
  en küçük terimli kesir olarak (`exact`: “1951/300”).
- **Paylaştırılmayan havuz:** kapsamında bağımsız bölüm yok, bir bölümün ağırlığı yok, ya da ağırlıklar toplamı sıfır
  (nette odasız bölümler): havuz hiç paylaştırılmaz (kısmen değil), nedeni `unshared`'da ve `share_unshared`
  uyarısında.
- **Bölümün payı:** kendi odaları adımda (`net`), havuzlardan aldığı (`share`), toplamı (`gross`); hepsi ondalık metin
  (“48.73”), JS sayısından geçmez. Toplamlar: `net`, `shared`, `unshared`, `gross = net + shared` tam.
- Pay **bütün programın** hesabıdır: `cad.program.check`'in `units` ya da `levels` süzgeci payı değiştirmez.
- Hesap `kentos_contracts::program_share` (saf, i128 tam sayılarla); belge `Document::program_rooms` ve
  `program_shares` ile çağırır, komut ve etiket/çizelge aynı sonucu okur.

### §3 Etiket, çizelge, Öznitelikler

- Etiket yer tutucuları `{share}` ve `{gross}` (hazır içerik “Bölüm payı”); bir mahal değişince (bölüm verme, sınır)
  bütün `{share}`/`{gross}` etiketleri aynı adımda yenilenir (`tag_follow`), program değişince ayar yenilemesiyle.
- Mahal çizelgesinin `share` (Pay) ve `gross` (Brüt) sütunları: mahalin bölümünün değerleri, her mahalinde aynı;
  toplanmaz, bu yüzden Bölüm ile gruplanan çizelge bölüm başına tek satır verir. Pay çizelge başına bir kez hesaplanır.
- Öznitelikler'de mahal için Bölümün payı ve Bölümün brütü.

### §4 Resmî alan profili

- Bölümün `recordedArea`'sı (kayıtlı/tapu alanı) yine yalnız okunur; pay hesabı onu okumaz ve yazmaz, Durum'da yanında
  gösterilir. Kayıtlı alanın hangi kuralla (net, brüt, ortak alan payı dahil) hesaplandığını söyleyen resmî profil
  (yönetmelik, kat mülkiyeti) **açık**: mevzuat profili ayrı iştir, kural adı ve sürümüyle gelir.

### §5 Arayüz

- Bina programı'nın **Binalar** sekmesi: bina ve kanatları, adları, kat kutucukları (projenin katları), Kanat ekle, Sil,
  Bina ekle, bölümleri; bölümü olan bina ya da kanat silinmez, pencere hangi bölümlerin bağlı olduğunu söyler.
- Bölümler'de her bölümün Bina ve Kanat seçimi (kanat yalnız binasınınkiler), ağırlık kuralında Pay ağırlığı.
- Başlıkta **Ortak alan payı** (Paylaştırma, Eşit, Net alana göre, Ağırlığa göre).
- Durum'da **Paylar** tablosu (Bölüm, Net, Pay, Brüt, Kayıtlı; Toplam) ve her ortak alanın havuzu ya da neden
  paylaştırılmadığı. Kaydet her şeyi `cad.program.edit` ile tek adımda yazar.

### §6 Şema 146 (geçici)

- Yazıcı 146'yı yalnız programda `shareRule`, bir bölümde `shareWeight`, bir bina ya da kanatta `levels`, ya da bir
  çizelgede `share`/`gross` sütunu varken yazar. Önceki şemada bunlar `unknown_field`. Örnek
  `fixtures/kcad/v2/program-shares.{json,kcad}`; bozuklar `program-shares-in-schema-145` (`unknown_field`),
  `-weight-comma`, `-wing-level-elsewhere`, `-level-twice`
  (`bad_value`); `schema-version-147`.

## Kapsam dışı (açık)

- Resmî alan profili (§4), arsa payı (tapu payı) ve kat mülkiyeti oranları; bunlar ayrı mevzuat profili ister.
- Bir bölümün birden çok ortak alan havuzuna farklı kuralla katılması (kural programın tek kuralıdır); havuz başına kural.
- Bağımsız bölüm çizelgesi türü (`of: units`): şimdilik mahal çizelgesi Bölüm ile gruplanır.
- Çok sayıda `{share}` etiketinde her etiket payı yeniden hesaplar (mahal sayısıyla doğrusal); önbellek ölçülmedi.
- Gerçek pencerede el ile kabul.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/program_share_cases.py` (KentOS kodu olmadan; paylar Python'un `fractions`'ıyla,
  denetimin öbür sayıları `program_cases.py`'den): `cad.program.check.shares.json` 20 durum (eşit, net, ağırlık; kanat,
  bina, program kapsamı; paylaştırılmayan üç neden; süzgeçler; katlar ve bulgu; etiket ve çizelge),
  `cad.program.edit.buildings.json` 22 durum (bina, kanat ve katları, kaldırma ve retleri, kural ve ağırlıklar, geri
  alma). `program_cases.py`'ye `program_level_outside` eklendi; eski üç dosya bayt bayt aynı.
- `kentos_contracts::program_share` birim testleri (yarıda çift, ağırlığın tam okunuşu, kalan adımların yeri).
- Masaüstü kabul testi `program_scenes::tests::the_shares_add_up_and_the_buildings_window_writes_through_the_command`;
  resimler `bina-pay-*`.
