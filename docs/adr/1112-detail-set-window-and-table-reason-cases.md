# ADR 1112: Detay paftaları penceresi ve tablo nedeninin bağımsız ortak durumları

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-16`'nın açık bıraktığı “masaüstünde
  detay kurma penceresi” ve “`table` nedeni için bağımsız vakalar” (ADR 1090 §1 ve Doğrulama). İlke “Performance
  First”; iki alt madde tek parçada.
- **Bağlam belgesi:** ADR 1090 §2–§3 (`cad.sheets.details`, tablo parçası), §6 (`cad.sheets.set`'in `table` nedeni),
  1030 §7 (tablo parçaları), 0594 (oran, kilit), 0314 §1 (pencere katman ezmesi), 0605 (açıklama temsili), 0975 §4 ve
  DESIGN.md §7.19 (pencere iskeleti), 0937 (görüntüleyici), 0938 (veri tablosu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.sheets.details` Python, MCP ve komut satırından çalışıyordu; masaüstünde detay seti kuran pencere yoktu (ADR 1090
§1'in kapsam dışı). `cad.sheets.set`'in `table` nedeni (değişen nesneyi okuyan model tablosunu gösteren pencere) yalnız
başsız kabul testinde sınanıyordu; ortak durumu yoktu.

### Araştırma

- AutoCAD Sheet Set Manager › Model Views: çizimin adlı model görünümü paftaya **Place on Sheet** ile konur; yerleştirmeden
  önce sağ tıkla görünümün **ölçeği** seçilir, sonra yeri; görünüm kendi ölçeğini ve katman ayarını taşır, görünüm etiketi
  tanımlıysa kendiliğinden konur; sonradan ölçek Özellikler'den değişir
  (help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-216C475D-7CC7-4B13-BBCA-AAC91BC34B22.htm; “To Add a View to a
  Sheet”, AutoCAD LT 2018, GUID-D1FC29EA-CEFD-4248-A750-73C0B104F24D).
- AutoCAD “To Lock the Scale of a Layout Viewport” (ADR 1090'da anılan): kilitli pencerede yakınlaştırma baskı ölçeğini
  değiştirmez.
- **KentOS kararı:** adlı görünüm şart değil; kapsam seçimden ya da tablo parçasından alınır, ölçek yerleştirmeden önce
  kesin orandan seçilir, yer pafta çekirdeğinin yerleşimidir (ADR 1090 §2). Bütün set bir onayla yazılır.

## Karar

### 1. Pencere (`sheet.details`, “Detay paftaları…”, Çıktı › Pafta)

`road_dialog::RoadDialog` iskeleti (DESIGN.md §7.19):

- **Solda form.** *Detaylar:* kitin düzenlenen veri tablosu (`edit_table`): Ad (yazılır), Kaynak (kapsamın boyu ya da
  “Tablo, parça i/n”), Oran (kesin oran seçimi: pafta çekirdeğinin standart oranları 10:1 … 1:1 … 1:1000), satırı silen
  ×; satıra basmak detayı seçer. Altında **Seçimden ekle** ve **Tablo parçası ekle**. *Seçili detay:* tablo parçası
  seçimi (tablonun parçaları), Katmanlar (çizimin katmanları ya da projenin katman durumları), Dondur (yalnız bu
  pencerede dondurulacak katman; her biri × ile çıkar), Açıklama (pencerenin ölçeği ya da sabit temsil), Kilit. *Set:*
  İlk numara (D-501), Başlık (Detaylar).
- **Sağda sonuç.** Kâğıt önizlemesi (`Picture` görüntüleyicisinde: her pafta yan yana, her detayın çerçevesi adıyla),
  altında tek açıklama satırı; özet kartları (Pafta, Detay, Oran); uyarılar; tablo (Detay · Pafta · Oran · Kâğıtta mm).
  Sonuç formu izler: her değişiklikte `cad.sheets.details`'in **planı** yeniden okunur, hiçbir şey yazılmaz; ret
  (`detail_does_not_fit`, `detail_too_small` …) sonuç yerinde kırmızı nottur.
- **Alt çubuk.** Kapat ve tek birincil eylem **Paftaları kur** (plan yoksa kapalı). Paftaları kur aynı girdiyle komutu
  çalıştırır: bütün set pafta kitabının tek değişikliği, tek geri alma adımıdır (pafta kitabının yazımı tek başına çizimin
  geri alma adımı değildir, ADR 0251 §3; pencere komutu çizimin bir grubunda çalıştırır ve grup onu adım yapar, ADR
  0590); pencere kapanır, ileti paftaları ve
  “tek adımda geri alınır”ı söyler. Esc ve Kapat hiçbir şey yazmaz.

### 2. Kapsamın alınması

- **Seçimden ekle:** seçili nesnelerin model kutusu (`sheets_fit::object_extent`, blokların parçalarıyla), uzun kenarının
  %5'i kadar her yandan büyütülür (Seçime sığdır'ın kenar payı): tek düz çizginin de yüksekliği olur ve 1 mm'den küçük
  pencere reddi (`detail_too_small`) yanlışlıkla gelmez. Varsayılan oran 1:10.
- **Tablo parçası ekle:** seçimdeki ilk model tablosunun her parçası ayrı detay (ADR 1030 §7; parça sayısı
  `sheets_details::table_parts`), varsayılan 1:20; kutusu komutun kendi kuralıdır (ADR 1090 §3).
- Pencere açılırken seçim varsa ilk detay ondan kurulur.

### 3. Detayın tarifi

Her satır `DetailRequest`'tir: `bounds` ya da `table {table, part}`, `ratio {paper, model}` (seçilen kesin oran),
`layers {type: theme, name}` (katman durumu seçildiyse; başka kullanıcının katman aç/kapası baskıyı değiştirmez, ADR
1090 §5), `layerOverrides` (dondurulanlar `{layer, frozen: true}`), `annotation {type: fixed, scale}` (sabit temsil
seçildiyse), `locked` (kilit kaldırıldıysa `false`). Pencere başka bir şey yazmaz; ret ve sıra komutundur.

### 4. Tablo nedeninin ortak durumları (`cad.sheets.set` › `table`)

`fixtures/commands/v1/cad.sheets.set.table.json` (24 durum), bağımsız başvuru `scripts/fixtures/sheet_set_table_cases.py`
ADR 1090 §6'nın ve ADR 0347'nin kuralından, KentOS kodu olmadan yazar: kalıcı kimlikli (v2) çizim; koordinat, alan,
adsız ve adlı blok çizelgesi, satır üyeli çizelge, hücre alanlı tablolar; pencerenin dondurduğu ve çizimin gizlediği
katmandaki tablo; pencere kenarına değen (kapalı kutu) ve 1 cm ötesindeki tablo; tablonun kendisinin verilmesi
(okuduklarını saymaz, kendisi `shows`); neden sırası shows, table; girdinin sırası; tekrar eden kimlik; plan ve doğrula;
silinen tablo ve çizelge; sonradan verilen dondurma (`setViewport`); olmayan kimlik retleri; kitapsız proje. Native koşucu
24/24 ilk çalıştırmada geçti (başvurunun düzeltilmesi gerekmedi).

### 5. Komutlar ve otomasyon

Yeni komut yok: pencere `cad.sheets.details` v1'i kullanır (Python `kentos.cad.sheets`, MCP aynı komut). Katalogda yeni
arayüz komutu `sheet.details` (`catalog/v1/additions.json`, `operations.json`: semantic, `cad.sheets.details` üzerinden;
şerit `scripts/ui/ribbon.py`'den yeniden üretildi). İkon mevcut `viewDetail`; yeni ikon gerekmedi.

### 6. Performans

Önizleme plan çağrısıdır (pafta çekirdeğinin yerleşimi, şablon bir kez); form değiştikçe çalışır, çizime ve sıcak yola
dokunmaz.

### 7. Kapsam dışı

- Detayı paftada sürükleyerek yerleştirme (yer pafta çekirdeğinin satır yerleşimidir); adlı görünümden detay; dönmüş
  kapsam (ADR 1090 §9).
- Şablon seçimi pencerede yok (varsayılan `sys:cad-teknik`; komut `source` alır).

## Uygulama

- Masaüstü: `apps/desktop/src/sheet_details_window.rs` (pencere, önizleme, kur), `sheet_details_window_tests.rs`,
  `sheet_detail_scenes.rs` (resimler); `app.rs`, `view.rs`, `traces/answers.rs` bağları.
- Uygulama: `crates/native/application/src/sheets_details.rs` (`table_parts`).
- Ortak durumlar: `scripts/fixtures/sheet_set_table_cases.py`, `fixtures/commands/v1/cad.sheets.set.table.json`.

## Doğrulama

Masaüstü testleri: pencere 1:1 ve 1:5 detayı ve tablonun parçalarını önizler (plan yazmaz; 1:5 detayın kâğıt eni model
uzunluğundan hesaplanır), Paftaları kur tek adımda yazar, dondurma ve kilit pencereye gider, tek Undo seti geri alır;
Kapat yazmaz, boş seçim söylenir. Ekran resimleri `.run/shots/arac-adv16-detay-penceresi-*`. Sonuçlar ve çalıştırılmayanlar:
plan/cad-advanced.md CAD-ADV-16'nın 6 Kasım 2026 paragrafı.

## Web'e ertelenenler

Web'in detay seti penceresi (`sheet.details`), TODOS.md §0.2.
