# ADR 1128: DXF'te tablo stili (TABLESTYLE)

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12`'nin ADR 1126 ve 1127'de açık kalan
  “DXF TABLESTYLE” satırı.
- **Bağlam belgesi:** ADR 0184 §7 (tablonun DXF'e çizgi ve yazılarıyla, KENTOS verisi taşıyan INSERT olarak yazılması),
  0183 §7 (yazı ve ölçü stilinin STYLE/DIMSTYLE kaydı), 1010 (okuyucunun kararları), 1023 (DIMTXSTY), 1126, 1127.

## 1. Araştırma

- Autodesk, [DXF Reference: TABLESTYLE](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-0DBCA057-9F6C-4DEB-A66F-8A9B3C62FB1A.htm):
  0 TABLESTYLE, 5 tutamaç, `{ACAD_REACTORS` içinde ve dışında 330 sahibi (named object dictionary'nin ACAD_TABLESTYLE
  girdisi), 100 AcDbTableStyle, 280 sürüm (0 = 2010), 3 açıklama (en çok 255), 70 akış yönü (0 aşağı, 1 yukarı),
  71 bayraklar, 40/41 yatay ve dikey hücre kenar boşluğu (varsayılan 0,06), 280 başlık ve 281 sütun başı gizli
  bayrakları; sonra her hücre stili için 7 yazı stili adı, 140 yazı yüksekliği, 170 hücre hizası, 62 yazı rengi
  (ByBlock), 63 dolgu rengi (7), 283 dolgu açık mı, 90 veri türü, 91 birim türü, 274–279 kenar kalınlıkları (ByBlock),
  284–289 kenarların görünürlüğü (1), 64–69 kenar renkleri (ByBlock).
- Autodesk sayfası tekrar eden hücre stillerinin sırasını ve adlarını söylemez. LibreDWG'nin
  [`src/dwg2.spec`](https://github.com/LibreDWG/libredwg/blob/master/src/dwg2.spec) `DWG_OBJECT (TABLESTYLE)`'ı (Open
  Design belirtiminin 20.4.101'i, tersine mühendislik): R2007'ye kadar üç satır stili, sırası “0: data, 1: title,
  2: header”; her birinde altı kenar, sırası “top, horizontal inside, bottom, left, vertical inside, right”
  (274+i, 284+i, 64+i). Kenar boşlukları (40/41) hücre stilinin değil tablo stilinindir.
- ACAD_TABLE'ın 342 grubu tablo stili nesnesinin tutamacıdır (ezdxf 1.4 `acad_table.py`, LibreDWG
  `FIELD_HANDLE (tablestyle, 5, 342)`); 343 tabloyu çizen anonim bloğun kaydıdır.
- Autodesk, [New and Modify Table Style Dialog Boxes](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT/files/GUID-4DCAAF7A-27BF-40A8-B363-41C2ED4E4ABE.htm):
  hücre hizası dikey (üst/orta/alt) × yatay (sol/orta/sağ); yükseklik varsayılanı veri ve sütun başında 0,18, başlıkta
  0,25 inç; metrik kenar boşluğu 1,5.
- ezdxf 1.4.5 TABLESTYLE yazamaz (`TableStyle` sınıfı “todo”, kayıtlı değil); okurken nesneyi etiketleriyle
  (`DXFTagStorage`) korur. Bağımsız taraf bu yüzden ezdxf'in yazdığı dosyaya TABLESTYLE ve ACAD_TABLE gruplarını
  Autodesk başvurusundan elle ekler ve KentOS'un çıktısını ezdxf ile etiket etiket okur.

## 2. Karar: sayılar

- Hücre stillerinin kâğıt mm'si DXF'te çizim birimidir: mm / 1000 × çizim ölçeği × birimin metredeki karşılığı
  (yazı ve ölçü stilleriyle aynı, ADR 0183 §7); okurken tersi, projenin çizim ölçeğinde.
- Hiza (170): KentOS hücreleri dikeyde ortalar; sol/orta/sağ 4/5/6. Hizası olmayan hücre stili veri için 4, başlık
  ve sütun başı için 5 yazılır. Okurken 1–9'un yatay kısmı alınır; üst ya da alt hiza kayıptır.
- Renk ACI'dir (başvuru 420 vermez): yazarken en yakın ACI (`aci::from_app`), okurken 1–255'in RGB'si; yazı ve kenar
  renginde 0, 256 ve 7 (ByBlock, ByLayer, mürekkep) “tablonun kendi rengi”dir. Dolgu 283 = 1 iken 63'tür.
- Kenar kalınlığı AutoCAD'in en yakın çizgi kalınlığıdır (1/100 mm); verilmeyen ByBlock (-2). Okurken negatif
  değerler yoktur; 0 en ince çizgidir.

## 3. Karar: yazma

- Projenin her tablo stili bir TABLESTYLE nesnesidir; ACAD_TABLESTYLE sözlüğünde adıyla (DXF adının kuralıyla)
  durur. Gruplar §1'in sırasıyla; tek kenar boşluğu çifti veri hücresininki, yoksa başlığın, yoksa sütun başınınki,
  hiçbiri yoksa AutoCAD'in metrik 1,5 mm'si. Altı kenar aynı bakışla, hepsi görünür.
- Nesnenin KENTOS verisi (`tablestyle` öğesi) stilin kendisidir (kimlik dahil); KentOS onu tam okur. Gruplar
  yuvarlayınca (farklı kenar boşlukları, RGB, ara kalınlık, projede olmayan yazı stili) dışa aktarma raporu söyler.
- Tablo DXF'e ACAD_TABLE olarak değil, ADR 0184 §7'deki gibi çizgi ve yazılarının anonim bloğu ve KENTOS verili
  INSERT olarak yazılır (ACAD_TABLE'ın hücre verisini başka programlar farklı okur). Tablonun stili iki yerde: KENTOS
  verisindeki tablo JSON'unda `tableStyle` kimliği ve `tablestyleref` öğesinde TABLESTYLE nesnesinin tutamacı (1005;
  başka programlar tutamacı nesneyle taşır).
- Yazarın JSON girdisi tablonun `tableStyle`'ını ve parçalara bölünmesini (`breaks`) artık düşürmez (önceden ikisi
  de okunmuyordu).

## 4. Karar: okuma

- OBJECTS taranır: kök sözlüğün ACAD_TABLESTYLE girdisinin adları sırasıyla, sözlüğün anmadığı TABLESTYLE'lar 3
  açıklamasıyla. KENTOS verisi olan nesne o stildir, olduğu gibi (kimliği korunur). Başka programınki yalnız bir
  ACAD_TABLE'ın 342'si onu anıyorsa alınır; §2'nin kurallarıyla, okuyucunun verdiği `dxf-table-N` kimliğiyle.
  Kenar boşlukları üç hücre stiline de yazılır. Altı kenarın bakışı farklıysa alt kenarınki (KentOS'ta satırın altındaki
  çizgi hücrenin bakışını taşır) alınır.
- Kararlar (ADR 1010, yeni konu `style`): `table_style_from_acad_table` (ACAD_TABLE bloğunun çizgi ve yazılarıyla
  alındı, KentOS tablosu olmadı; stili projenin tablo stili oldu), `table_style_lossy` (yukarı akış, gizli başlık ya
  da sütun başı, üst/alt hiza, farklı ya da gizli kenarlar), `table_style_unused_left_out` (hiçbir tablonun izlemediği
  stiller, adlarıyla).
- Hücre stillerinin andığı yazı stili, metin izlemese de STYLE kaydıyla alınır (Standard dışında; ADR 1023'ün
  DIMTXSTY kuralı gibi).
- Uygulama (`import_apply`): projede aynı adlı tablo stili varsa o kullanılır; yoksa dosyanın kimliği boşsa korunur
  (KentOS'un kendi stilleri geri geldiği gibi gelir), değilse ya da okuyucunun numarasıysa yeni UUIDv7. Tablo
  kuralı (`table_style_defs_problem`) reddederse stil alınmaz. KentOS tablosunun `tableStyle`'ı yeni kimliğe döner;
  karşılığı olmayan stil bırakılır.

## 5. Kanıt

- `crates/shared/formats/tests/all/table_style_dxf.rs`:
  - `another_programs_table_style_comes_with_its_acad_table_and_what_is_lost_is_said`: ezdxf'in yazdığı ve betiğin
    TABLESTYLE/ACAD_TABLE eklediği `fixtures/exchange/v1/table-styles.dxf`; beklenen stil, yazı stili, kararlar ve
    kayıplar `table-styles.json`'da betiğin gruplardan hesabı (KentOS kodu yok).
  - `the_project_table_styles_go_out_as_tablestyle_objects_and_come_back_exactly`: betiğin girdisinden
    (`table-styles-export.json`) yazılan `table-styles-export.dxf` diskteki ile aynı; KentOS geri okuyunca stiller
    birebir, tablo `tableStyle`'ıyla.
- `python -I scripts/fixtures/table_style_dxf.py --check`: ezdxf dışa aktarımı okur; ACAD_TABLESTYLE adları,
  her nesnenin 40/41, üç hücre stilinin 7/140/170/62/283/274/279/64'ü §2–§3'ün kuralından hesaplanan değerler;
  KENTOS verisi girdinin stili; tablonun INSERT'inin `tablestyleref`'i CETVEL nesnesinin tutamacı.

## 6. Sınırlar

- ACAD_TABLE KentOS tablosuna çevrilmez (hücre verisi okunmaz); bloğuyla çizilir, stili projeye gelir.
- R2010+ hücre stili haritası (CELLSTYLEMAP, hücre başına kenar boşluğu, özel hücre stilleri) okunmaz ve yazılmaz.
- Kenar başına ayrı bakış, çift çizgi, kenar çizgi tipi, dikey hiza, yukarı akış taşınmaz (ADR 1127 §6).
- Yazarın JSON girdisi tablonun kendi hücre stillerini (`styles`, `formulas`, `fields`) hâlâ okumaz (ADR 0327'nin
  DXF'i masaüstü yolundan yazılır); bu ADR'nin kapsamı değil.
