# 0823 — Yazılan pah ve yuvarlama değerini onaydan önce göster

Tarih: 2026-11-01. Durum: masaüstü dilimi uygulandı; CAD-ADV-04 bütünü açık.

## Sorun ve karar

Gerçek metin alanı seçme/yapıştırmayı sağlıyordu, ancak altındaki geometri
fare mesafesini göstermeyi sürdürüyordu. `10cm;20cm` yazan kişi Enter'dan
önce iki farklı kesme mesafesini göremiyordu. Onaylanan ve gösterilen
geometri aynı anlamsal plan olmalı.

`Tool::preview_input` salt okunur `Context` alır. Varsayılanı kapalıdır;
bu dilimde yalnız `CornerTool` boyut adımında açılır. Bu yol belgeye,
Undo'ya veya hatırlanan araç değerlerine yazmaz. Metin, mevcut boyut
okuyucusundan geçip aynı `cad.entities.chamfer`/`fillet` planını çalıştırır.
Yeşil kenar yardımcıları da o planın iki mesafesini/yarıçapını kullanır.
Tanınmayan ya da geometrik olarak olanaksız değer, fareye veya son değere
düşmez. Hatalı taslakta geçerliymiş gibi bağlayıcı gösterilmez.

Masaüstü alanı geçici kimliğiyle Ready/Invalid/Off durumunu taşır. Tek
Enter, sahne tıklaması, menüde yazılı değeri gösteren Uygula ve isteğe bağlı
hızlı sağ tuş onayı ortak submit yolundadır. Geçersiz girişte alan korunur.
Menüyü kapatan Esc ayrıca genel klavye aboneliğinden alttaki alanı
kapatmamalıdır: açık menünün aboneliği yakalanmış Esc'yi ikinci kez
iletmez. Sonraki Esc yalnız değer taslağını kaldırır; belge değişmez.

Yeni geometri komutu ya da Python/MCP sözleşmesi yoktur. Önizleme geçici
etkileşimdir; yazma hâlâ mevcut ortak komutlarla gerçekleşir. Web araç
yüzeyi ADR 0228 ve TODOS §0.2 gereği ertelidir.

## Dayanak

[AutoCAD CHAMFER](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-B1DCF991-90A7-4DB0-96FC-BDA3FB76337C.htm)
iki kenarın ayrı mesafelerini;
[FILLET](https://help.autodesk.com/cloudhelp/ENU/AutoCAD-Web-Help/files/List-Commands/AutoCAD_Web_Help_List_Commands_FILLET_html.html)
yarıçap ve kırpma davranışını ayırır. KentOS'un kararı bu değerlerin aynı
komutun yazmadan planında gösterilmesidir. Dairesel yuvarlamaya ikinci
yarıçap eklenmez; farklı kenar mesafesi pah içindir.

## Doğrulama

- Interaction: 71 kitaplık + 850 bütünleşme testi geçti (2 ignored).
  Yeni üç durum; iki ayrı yardımcı mesafe, fareyle bozulmayan taslak,
  eksik girişin hatırlanan değere düşmemesi, olanaksız yarıçap/düzeltme ve
  tek Undo'yu denetler.
- Masaüstü: tüm 1.570 test geçti (256 ignored); menü/Esc düzeltmesinden
  sonra beş odaklı gerçek bileşen/etkileşim testi tekrar geçti. Menü ve
  hızlı sağ tuş yolları ayrı doğrulanır. Clippy ve uygulama derlemesi geçti.
- Gerçek macOS uygulamasında 10/20 cm pah, 10/20 m ret ve aynı kutuda
  düzeltme, sağ tuşta değeri görme, tek menü tıklaması, menü Esc'sinde
  alanın korunması; 25 cm yuvarlama yazarken önizleme ve tek sahne tıklaması
  sınandı. Kaydedilen KCAD bağımsız okuyucuyla okundu: pah uçları
  `(0.1,0)` ve `(0,0.2)`; yay merkezi `(0.25,0.25)`, yarıçap `0.25` metre.
  Kaynak kimlikleri/öznitelikleri ve tek Undo doğrulandı.
- Ham görseller ve günlükler repo dışındadır:
  `/tmp/kentos-cad-adv04-2026-11-01/22-...30-...png`,
  `live-pah-gui-audit.txt`; makineye özgü kanıtlar Git'e girmez.

Bu dilim fit/kontrol düzenleme, süreklilik arayüzü veya birleşik
alan/ölçü/alt nesne kabulünün tamamlandığı anlamına gelmez.
