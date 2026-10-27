# ADR 0716: Paftada kat

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-UX-04'ün üçüncü dilimi.
- **Bağlam:** ADR 0715 (kat görünümü: nesnenin katı, soluk alt kat), ADR 0713 (etkin kat), ADR 0164 (pafta çekirdeği),
  ADR 0314 (haritanın kendi katmanları), ADR 0252 (başsız kitap PDF'i).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Kat görünümü (ADR 0715) ekranda bir katı gösterdi, ama pafta haritaları ve PDF bütün katları üst üste çiziyordu: iki katlı
dairenin “1. kat planı” paftasında zemin katın bölmeleri, kapıları ve mahal yazıları da vardı. Kat görünümü oturumundur;
paftayı ona bağlamak her paftayı aynı kata zorlar, yazdırılan pafta ekranda o an seçili kata göre değişirdi.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — Create a Plan View](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-3FE4DAED-82D8-4C56-A017-737BDA740E50.htm):
  kat planı bir katın görünümüdür; New Plan bir ya da birkaç kat için plan açar.
- [Revit 2023 — Create an Underlay](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-77184183-E245-4F3B-8486-617E9A9FB296.htm):
  altlık görünümün özelliğidir (Range: Base Level), yarım tonla çizilir.
- [Revit 2023 — Add Views to a Sheet](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-D3E7FDC3-57AB-4497-AB87-D195476254C2.htm):
  paftaya görünüm konur; belge “Each view can be placed on one sheet only” der, aynı plan için kopya görünüm açılır.
- [Archicad 28 — Place Drawing on Layout](https://help.graphisoft.com/AC/28/INT/_AC28_Help/070_Documentation/070_Documentation-88.htm):
  paftaya (layout) bir görünüm, örneğin bir katın kat planı, çizim olarak yerleştirilir.

KentOS kararı: Revit'in kat planı görünümü ve altlığı, **pafta haritasının kendi ayarı** olarak. KentOS'ta adlı model
görünümü (Revit'in görünümü) zorunlu değildir; harita çizimi doğrudan gösterir. Kat haritanın özelliğidir: aynı çizim
birkaç haritada ayrı katlarla durur, görünüm kopyalamak gerekmez. Ekranın kat görünümü paftayı değiştirmez.

## Karar

### 1. Haritanın katı (`MapItem.level`, pafta çekirdeği)

- `level: { id, below }` isteğe bağlı: yoksa harita **Bütün katlar**'ı çizer (eski paftalar aynı kalır). Varsa katın ve
  katsız nesneleri; `below` ile hemen alttaki kat (ADR 0715 §2'nin kuralı) soluk. Öbür katların nesneleri haritada yoktur:
  çizilmez, yazıları yerleşmez.
- Nesnenin katı ADR 0715 §1'in kuralıyla (`Entity::level`): açıklık duvarının, etiket nesnesinin, kaplama mahalin, ölçü
  bağlarının katında.
- `setViewport`'un `level`'i: `{ "type": "all" }` ya da `{ "type": "level", "id", "below" }`. Yama nesnenin bütününü yazar
  (JSON birleştirme yaması nesneleri anahtar anahtar birleştirir; eski `below` kalmasın). Boş ya da 128 harften uzun kimlik
  çekirdeğin `bad_level` reddi.
- `cad.sheets.edit` projede olmayan katı `level_not_found` ile reddeder, çağrının hiçbir işlemi yazılmaz. Cevabın haritaları
  `level` ve `below`'u söyler.

### 2. Kâğıt (PDF ve masaüstü)

- Başsız kitap PDF'i ve masaüstünün pafta kipi aynı kuralla: haritanın dışarıda bıraktıkları çizilmez; soluk katın renkleri
  kâğıtta beyazla `PHASE_FADED` oranında karışır (yarım ton), yazıları da.
- Ön denetim: haritanın katı projede yoksa `map_level_missing` uyarısı (harita yalnız katsız nesneleri çizer) ve “Bütün
  katları çiz” düzeltmesi.
- Masaüstünde haritanın denetçisinde, proje katlıyken **Kat** (Bütün katlar, katlar; projede olmayan kat “(projede yok)”)
  ve kat seçiliyken **Alt kat soluk** anahtarı; her biri tek geri alma adımı. Kat seçilmemişken Alt kat soluk adım yazmaz.
- “Modeli bu görünümde düzenle”: model haritanın gösterdiği gibi seçilir ve kenetlenir (haritanın katı ve katsızlar; soluk
  kat kenetlenir, seçilmez); oturumun kat görünümü orada sayılmaz.

### 3. Görünüm penceresinin kapsamı

Haritanın model kapsamı (`apply_boundary`) artık yalnız geometridir: sınıra değen her nesne, o an gizli ya da soluk olsa da.
Görünümün evre, kat, katman, süzgeç ve zaman kuralları okunurken eklenir (`Store::view_shown`). Önceden kapsam, hesaplandığı
anın seçilebilenleriyle donuyordu: kat değişince gösterilen kat kapsam dışında kalıyor, soluk kat hiç kenetlenmiyordu
(testte görüldü).

## Sınırlar

- Pafta seti (her kat için ayrı pafta) kendiliğinden kurulmaz; kat haritada elle seçilir (ARCH-DOC-04).
- Bloklar, çizgiler ve yazılar katsızdır: her kat haritasında görünürler (katman ayrımı sürer).
- Katın yükseklik aralığı (Revit'in View Range'i, kesit düzlemi) yoktur: kat, nesnenin kendi katıdır.

## Doğrulama

- `cargo test -p kentos-sheet --test all map_levels`: kat bütün yazılır, kalkar, geri alınır; boş ve uzun kimlik reddi.
- `KENTOS_CASES_ONLY=cad.sheets.edit`: 56 ortak durum (10'u kat: kat, alt kat soluk, Bütün katlar, başka kat, projede olmayan
  kat ve atomik ret, katsız proje), bağımsız başvuru `scripts/fixtures/sheets_edit_command_cases.py`, Rust ve Python.
- `cargo test -p kentos-plot --test all map_levels`: haritanın yazıları katına göre, alt kat soluk renkte; projeden kalkan kat
  ön denetimin bulgusu ve düzeltmesi.
- `cargo test -p kentos-sheet-ui --test designer a_map_draws_a_level`: denetçinin mesajları, tek adımlar, boş yama adım değil
  (düzeltme olmadan test düşüyordu).
- `cargo test -p kentos-interaction --test all level_view`: paftanın modelini yerinde düzenlerken seçim ve kenet haritanın
  katıyla; kapsam düzeltmesi olmadan soluk kat kenetlenmiyordu. Etkileşim (771), pafta çekirdeği (403+77), çıktı (16+47),
  pafta tasarımcısı (45) takımları, masaüstünün pafta ve yazı testleri, değişen crate'lerde clippy.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=pafta-kat-1,pafta-kat-1-alt,pafta-kat-denetci
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`: iki katlı dairenin
  paftasında yalnız 1. kat, altında soluk zemin kat, haritanın denetçisinde Kat ve Alt kat soluk.
