# ADR 0733: Çizelge ve metraj iş akışının kalanı: donatı ve kaplama çizelgesi, tür süzgeci, ara toplamlar, Çizelge ayarları, hücrenin yazdığı nesneler, dosyaya yazma

- **Durum:** kabul edildi (29 Ekim 2026); `plan/architecture.md` ARCH-DOC-02'nin kalan alt maddeleri.
- **Bağlam:** ADR 0344 §3 (çizelge, `cad.schedules.*`), 0681 (Metraj CSV'si), 0722 §5 (süpürgelik, kaplama geçişleri),
  0345 (mahalin kaplamaları), 0702 (mobilya ve vitrifiye kitaplığı), 0727 (mutfak dizisi), 0713/0715/0716 (etkin kat, kat
  görünümü, paftada kat), 0346 (evreler), 0317 (blok öznitelikleri ve kartı), 0726 (bloğun gerçek boyu), 0223 (çalışma
  kitabı yazıcısı), 0717 (Kat ver).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0344'ün çizelgesi açıklık, mahal, kolon ve duvarı listeliyor; katı, evresi, biçimi ve katmanı süzüyor; sıralıyor, aynı
satırları birleştiriyor ve genel toplam yazıyordu. Eksikler:

1. Donatı (mobilya, vitrifiye, mutfak modülü) çizelgesi yoktu; blok yerleştirmesinin katı da yoktu: iki katlı bir çizimde
   üst kata konan koltuk alt katın planında da görünüyordu (kat görünümü yalnız mimari nesneleri ayırıyordu).
2. Mahal kaplama çizelgesi (döşeme, süpürgelik, duvar, tavan) için alan yoktu; süpürgelik kaplaması hiç yoktu.
3. Tür süzgeci yoktu; gruplar başlıksız ve ara toplamsızdı.
4. Yerleştirilen çizelgenin sütunları, süzgeci ve sıralaması arayüzden değiştirilemiyordu (yalnız Python/MCP).
5. Tabloyu düzenle bir hücrenin hangi nesnelere yazılacağını söylemiyordu; hesaplanan hücre yazılıp Kaydet'te reddediliyordu.
6. Çizelge dosyaya yazılamıyordu; Metraj'ın çalışma kitabı 15 anlamlı basamağı aşan hesap değerlerini metin yazıyordu
   (ADR 0681'in açık bıraktığı sayı politikası).

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Sort and Group Fields in a Schedule](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-3ED9C7F0-C340-4317-BCE6-327638ED4A39.htm):
  her sıralama anahtarı için Header (grubun değeri başlık satırı) ve Footer (“Title, count, and totals”, “Totals only” …),
  Blank Line; “Itemize every instance” kapalıyken aynı satırlar birleşir; genel toplam ayrı seçenek.
- [Revit — Export a Schedule](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-B2CCAC4F-1D38-4D5D-B4D1-95619D1B7EBE.htm)
  ve [ViewScheduleExportOptions.HeadersFootersBlanks](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-API-MainReference/files/html/432c134e-3333-a020-1ecd-a6411dd73369.htm):
  çizelge ayırıcılı metne görüldüğü gibi yazılır; grup başlıkları ve ara toplamlar seçenekle (öntanımlı açık).
- [Revit — Room Instance Properties](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-21326970-0037-41C6-A996-980C24EE019F.htm):
  odanın Base Finish, Floor Finish, Wall Finish, Ceiling Finish metin alanları; kaplama çizelgesi bunlardan.
- [Archicad 28 — Interactive Schedule](https://help.graphisoft.com/AC/28/INT/_AC28_Help/050_ViewsVB/050_ViewsVB-90.htm)
  ve [Archicad 29 — Edit the model using Interactive Schedule](https://help.graphisoft.com/AC/29/INT/_AC29_Help/055_InteractiveSchedule/055_InteractiveSchedule-26.htm):
  çizelgede yazılan değer modeli hemen değiştirir; [Archicad 30 — Scheduled Elements on Locked or Hidden Layer](https://help.graphisoft.com/AC/30/INT/Topics/055_InteractiveSchedule/055_InteractiveSchedule-29.html):
  gizli ya da kilitli katmandaki öğe çizelgeden düzenlenmez.

KentOS kararı: Revit'in başlık ve ara toplam satırları, kaplama alanları ve “görüldüğü gibi” dışa aktarımı alınır;
Archicad'in çizelgeden düzenlemesi zaten vardır (ADR 0344), kilitli katman bütün yazmayı reddeder (değişmedi). Eklenen:
hücreye yazmadan önce hangi nesnelerin değişeceği söylenir; hesaplanan sayıların dosyadaki biçimi açık bir seçimdir.

## Karar

### 1. Blok yerleştirmesinin katı ve evresi

- Yerleştirmenin `arch`'ı (`level`, `phase`; ADR 0339'un `ArchInfo`'su), KCAD şema 131 (opt-in). `Entity::own_level` ve
  `own_phase` yerleştirmeyi de okur: kat görünümü (ADR 0715), paftanın katı (0716) ve evre görünümü (0346) donatıyı ayırır.
- `cad.blocks.insert`'in `arch`'ı (bilinmeyen kat `level_not_found`). Blok ekle'nin ilk ayar satırı **Kat** (öntanımlı etkin
  kat; Katsız seçilebilir), Mimari araçların kuralıyla (ADR 0713).
- Mutfak dizisinin modülleri dizinin duvarının katıyla yazılır (`cad.casework.run` ve izleme).
- `cad.levels.assign` yerleştirmeye de kat verir (kimlikleri verilince). Denetim'in “katı yok” bulgusu bloklara bakmaz
  (antet, kuzey oku gibi çizim yardımcıları katsız kalabilir).
- Öznitelikler'de yerleştirmenin Kat ve Evre satırları.

### 2. Donatı çizelgesi

`ScheduleOf::Fixtures` (“Donatı çizelgesi”): çizimin blok yerleştirmeleri (dış başvuru ve PDF altlığı tanımlarınınkiler hariç).

| Alan | Ne | Düzenlenir |
|---|---|---|
| `type` (Tür) | bloğun adı | hayır (Bloğu değiştir'in işi) |
| `category` (Kategori) | bloğun kartındaki kategori yolu (ADR 0317) | hayır |
| `space` (Mahal) | bloğun çiziminin ortasını (yerleştirmenin dönüşümüyle; duvar yüzündeki taban noktası sınırda kalır) ya da, orası hiçbir mahalde değilse, taban noktasını içeren, yerleştirmeyle aynı kattaki mahalin “numara ad”ı; katsızda katsız mahaller | hayır |
| `level`, `phase`, `count`, `attr` | | yalnız `attr` |

`attr` önce bloğun öznitelik etiketini (tanımda varsa yerleştirmenin değeri), yoksa nesnenin özniteliğini okur ve yazar.
Öntanımlı sütunlar Tür, Kategori, Mahal, Kat, Adet; Grup açıkken Tür, Kategori, Adet.

### 3. Mahal kaplama çizelgesi

- Mahalin kaplamalarına `base` (süpürgelik kaplaması), KCAD şema 131 (opt-in); `cad.spaces.edit`'in `set`'i ve Öznitelikler.
- Mahal çizelgesinin alanları: `floorFinish`, `baseFinish`, `wallFinish`, `ceilingFinish` (düzenlenir) ve `baseboard`
  (Süpürgelik boyu, Metraj'ın kuralıyla: çevre eksi kapı ve döşemeye inen boşluk geçişleri; ADR 0722 §5). Geçiş ve süpürgelik
  hesabı belge katmanına taşındı (`kentos_domain::passages`): Metraj ve çizelge aynı hesabı okur.
- Hazır “Mahal kaplama çizelgesi”: Numara, Ad, Döşeme, Süpürgelik, Duvar, Tavan, Alan, Süpürgelik boyu.

### 4. Tür süzgeci ve ara toplamlar

- `filter.types`: açıklıkta tür, duvarda stil, kolonda tür, mahalde kullanım, donatıda bloğun adı.
- `sort[i].header`: grubun değeri, bütün satırı kaplayan kalın bir başlık satırında, grubun önünde.
  `sort[i].footer`: grubun ardında ara toplam: anahtarın sütununda “‹değer› toplamı” (boşsa “(boş) toplamı”), toplanan
  sütunlarda grubun toplamı, kalın. İç anahtarın ara toplamı dıştakininkinden önce gelir; genel toplam en sonda.
- Başlık ve ara toplam satırlarının `members`'ı boştur (`members` başlıktan sonraki her satır için, genel toplam hariç);
  onların hücresi düzenlenmez (`schedule_summary_row`). Başlık ve ara toplamı olmayan çizelge değişmez.

### 5. Çizelge ayarları ve Çizelge aracı

- **Çizelge ayarları** penceresi (Öznitelikler'de çizelge seçiliyken, tablonun sağ tık menüsünde ve Çizelge aracında):
  Tür; **Sütunlar** (ekle ▾, kaldır, yukarı, aşağı, başlık; düzenlenen ve hesaplanan sütun işaretli); **Süzgeç** (Kat, Evre,
  Tür, Biçim, Katman: her biri çoklu seçim menüsü); **Sıralama** (en çok üç anahtar: sütun, Azalan, Başlık, Ara toplam);
  **Aynı satırları birleştir**, **Genel toplam**; canlı önizleme (ilk satırlar ve satır sayısı). Tamam, yerleştirilmiş
  çizelgede tek `cad.schedules.edit` (`set`), tek geri alma adımıdır.
- Çizelge ▾: Kapı ve pencere, Mahal, **Mahal kaplama**, **Donatı**, Malzeme lejandı. Aracın görünür ayarları: Tür, Kat
  (Bütün katlar, Etkin kat), Grup, Toplam, Sırala ve Ayarlar….

### 6. Hücrenin yazdığı nesneler

- Salt okunur `cad.schedules.query`: sütunlar (başlık, alan, düzenlenir mi, toplanır mı, birim) ve satırlar (tablodaki
  sırası, rolü: `data`, `header`, `footer`, `total`; hücreler; nesneler). Python ve MCP hangi hücrenin hangi nesnelere
  yazılacağını yazmadan bilir; `cad.schedules.edit`'in planı yazılacak nesneleri `edited`'de verir (ADR 0344).
- Tabloyu düzenle'de çizelge hücresinin altında bilgi satırı: düzenlenen sütunda “Yazılır: n nesne — K1, K2 …”, satırın
  nesneleri çizimde seçilir; hesaplanan sütunda “Hesaplanır: … çizimden okunur” ve hücre salt okunur; başlık ve ara toplam
  satırı salt okunur.

### 7. Tabloyu dosyaya yazma ve sayıların biçimi

- `file.export.table`: bir tablonun (çizelge, blok çizelgesi, dosya ya da elle yazılmış tablo) hücreleri CSV, XLSX ya da ODS
  olarak (`format`, öntanımlı uzantıdan). `groups` (öntanımlı açık): başlık, ara toplam ve genel toplam satırları da yazılır.
  `numbers`:
  - `shown` (öntanımlı): hücreler görüldüğü gibi (projenin basamağıyla); sayı sütunu sayı olarak, basamağı biçimde.
  - `exact`: çizelgenin hesaplanan sayıları çizimden yeniden okunup 15 anlamlı basamağa yuvarlanarak (hesap tablosunun
    tuttuğu kadar) sayı olarak; görülen yuvarlamadan doğan toplam farkı olmaz.
  Kullanıcının yazdığı alanlar (numara, ad, tür, öznitelik) iki kipte de yazıldığı gibidir, hiçbir zaman yuvarlanmaz;
  15 basamağı aşan böyle bir değer yine metin yazılır ve söylenir. Yazma sonucu kipin sözünü taşır (sessiz yuvarlama yok).
- Masaüstünde Öznitelikler'in ve tablonun sağ tık menüsünün **Dosyaya yaz…** penceresi (biçim, Gruplar, Sayılar).
- Metraj'ın Dışa aktar'ı aynı **Sayılar** seçimini alır: hesaplanan sütunlar Görünen basamakla (öntanımlı) ya da Tam
  (15 basamak); metin sütunları olduğu gibi. ADR 0681'in seçili sekme kuralı değişmez. PDF, çizelgenin bulunduğu paftanın
  PDF'idir (değişmedi).

### 8. Kapsam dışı

Donatının en ve boy sütunları (bloğun kutusu uygulama katmanında ölçülüyor; belge katmanına taşınınca eklenir), hesaplanan
sütun formülleri (Revit'in Calculated Value'su), koşullu biçim, çizelgenin pafta üzerinde bölünmesi, boş satır
(Blank Line), web.

## Doğrulama

29 Ekim 2026, Apple M5, macOS, debug.

- **Ortak durumlar (KentOS kodu olmadan):** `scripts/fixtures/schedule_extras_cases.py --check` üç dosyayı yazar ve
  denetler: `cad.schedules.create.extras.json` 22, `cad.schedules.edit.extras.json` 21, `cad.schedules.query.json` 23
  durum (donatı, kaplama ve süpürgelik, tür süzgeci, grup başlığı ve ara toplam, ara toplam satırının reddi, kilitli
  satır, sorgunun satır rolleri ve nesneleri). `block_insert_cases.py` 37 (projede olmayan kat). Rust'ta bütün ortak
  durumlar `every_command_case` ile geçer (`cad.schedules` 138 durum); Python'un tipli katmanı aynı durumları oynatır
  (`pnpm py:test`: 375 test).
- **Tablo dosyası:** `file.export.table` başsız sunucuda `kentos_headless::file_commands` ile; CSV, XLSX, ODS; gruplar ve
  Tam sayılar (15 anlamlı basamak, `table_export::fifteen`). MCP'nin bütün takımları geçer.
- **KCAD şema 131:** `schedules-fixtures.kcad` bağımsız Python yazıcısıyla (katlı ve katsız donatı, süpürgelik
  kaplamalı banyo, tür süzgeçli, başlıklı ve ara toplamlı donatı çizelgesi); `broken/schedules-fixtures-in-schema-130`
  `unknown_field`, dört bozuk örnek `bad_value`; `broken/schema-version-132.kcad`. Kodek, tipli sütunlar (yerleştirmenin
  5. isteğe bağlı alanı) ve `tools/kcad/kcad.py` aynı sonucu verir (`kcad_v2_reference.py --check`: 1338 dosya).
- **Takımlar:** contracts, commands, kcad, domain, native-application, interaction, headless, mcp ve masaüstü testleri
  geçer; değişen crate'lerde `clippy --all-targets -D warnings` temiz; WASM paketleri wasm32'de derlenir. Komut
  geçmişinin başlık listesi kataloğa yetişti (önceki maddenin `cad.arch.exceptions`'ı da).
- **Gerçek arayüz (`schedule_extras_scenes.rs`, 1440×900 ve 1100×650, koyu ve açık):** 9 × 6 m dairenin on donatısı
  ve üç mahali; Donatı çizelgesi kategoriye göre başlık ve ara toplamla, her donatının mahali dolu (ilk denemede
  duvar yüzündeki taban noktası mahal dışında sayılıp mutfak ve banyo modüllerinin Mahal'i boş çıktı; kural bloğun
  çiziminin ortasına çevrildi, başvuru da); Çizelge ayarları (9 veri satırı önizlemesi); Dosyaya yaz; Kaplama çizelgesi
  (süpürgelik boyları 13,80 + 30,00 + 10,80 = 54,60 m elle hesapla aynı); Tabloyu düzenle'de “Hesaplanır: “Adet”
  çizimden okunur”; Metraj'da Tam sayılar; Blok ekle'nin Kat satırı. Ham görüntüler `.run/gui-evidence/2026-10-29-schedules/`.
- **Açık:** donatının en ve boy sütunu (§8), çok satırlı başlık, paftada ayrı çizelge görünümü; web'in karşılığı
  TODOS.md §0.2'de.
