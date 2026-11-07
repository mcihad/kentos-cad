# 1034 — Enkesit arazisiz aralığını söyler

Tarih: 2026-11-07. Durum: kabul (Y99). Kapsam ROAD-SEC-08, ADR 0903 (kesit hattı ve enkesit), ADR 0900 §8 (yüzeyin
dışında ve boşluğunda kot yok).

## Sorun

Kesit hattının yüzeyin dışında ya da boşluğunda kalan yerinde arazi kotu yoktur. Arazi noktaları o aralığı `gap` ile
işaretliyordu; yarma ve dolgu alanı o aralığı atlıyordu. Bu doğru ama sessizdi:

- tasarımın bir kısmı arazisiz aralığın üstüne düştüğünde kesitin alanı eksik çıkıyordu;
- sorgu bunu söylemiyordu;
- pencerede yalnız arazi çizgisinin bittiği görülüyordu.

ROAD-SEC-08 eksik aralığın görünmesini, alanın onaysız alınmamasını istiyor.

## Araştırma (7 Kasım 2026)

- **ADR 1033'ün kaynakları:** Civil 3D ve ArcGIS kotu üçgenin düzleminden okur; yüzeyin dış bükey zarfının dışına
  uzatma yoktur (XMS'in doğrusal enterpolasyonu).
- **Civil 3D'nin kesit görünüşü:** yüzeyin bittiği yerde arazi çizgisi de biter. Kesit alanı (sample line group'un
  malzeme listesi) yalnız iki yüzeyin birlikte bulunduğu yerde hesaplanır. Bunun bir uyarıyla söylendiğine dair bir
  kaynak bulunamadı.
- **KGM:** enkesitte arazinin bilinmediği yer için bir kural bulunamadı. Uygulamada arazi ölçüsü kesit şeridini
  kaplayacak genişlikte alınır.
- **Matematik:** yeni hesap yok. Arazisiz aralık, `gap` işaretli her noktanın bir öncekinden kendisine kadarki
  ötelemedir. Tasarımın açıklığı sol yanın en dış noktasından sağ yanınkine kadardır; ikisi kesişiyorsa tasarım
  arazisiz yerin üstündedir.

## Karar

- **`SectionRow.missing`:** şeridin yüzeyin dışında ya da boşluğunda kalan öteleme aralıkları, soldan sağa. Yoksa
  yazılmaz.
- **`section_ground_missing` uyarısı:** tasarımın açıklığı bir arazisiz aralıkla kesişen kesitlerin km'lerini tek
  uyarıda söyler: "oradaki kazı ve dolgu alana katılmadı; yüzeyi genişletin ya da boşluğu doldurun."
  - Alan hesabı değişmedi: arazisiz yerde alan sayılmaz, kot uydurulmaz.
  - Uyarı `road.sections.query`, `road.sections.draw` ve `road.sheets.sections`'ta, `section_open`'dan sonra gelir.
- **Enkesit penceresi:** arazisiz aralık kesitte soluk bir uyarı renginde "arazi yok" şeridi olarak görünür; uyarı
  öbür uyarıların yanında yazılır.

## Ölçüm

- **Bağımsız başvuru** `road_section_command_cases.py` (KentOS kodu yok): aralıklar başvurunun 40 basamaklı kesit
  arazisinden, uyarı aynı kuralla. Çizim ve pafta başvuruları uyarıyı aynı yerden alır.
- **`road.sections.query`:** 152 kesitten 8'inde arazisiz aralık var; 6 durum uyarıyı veriyor (yan aksın km 0+050'si,
  ana aksın km 1+000'ı).
- **Öbür komutlar:** `road.sections.draw`'da 2, `road.sheets.sections`'ta 1 durum uyarıyı da veriyor. Öbür bütün
  kesit, pafta ve üstyapı durumları değişmedi.
- **Masaüstü testi:** yan takımda uyarı ve kesit aralıkları; ana takımda uyarı yok.
- **Resimler** `.run/gui-evidence/yol-y99/` (1440 açık, 1100 koyu): km 0+050'de −20'den +7,5'e "arazi yok" şeridi ve
  tasarımın sol yarısı onun üstünde.
