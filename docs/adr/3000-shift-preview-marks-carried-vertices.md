# 3000 — Paralel kaydır önizlemesinde birlikte kayan komşu köşelerin işareti

Tarih: 2026-11-09. plan/map-cad.md MAP-07, beşinci tur. Durum: taslak. ADR 2600'ün “Sonra”sındaki “taşınmaların sahnede
işaretlenmesi”.

İlgili:
- [ADR 1960](1960-parallel-shift-shared-boundary.md): Paralel kaydırmada ortak sınır; önizleme komşuları ve komşu
  alanın yeni boyunu gösterir;
- [ADR 2600](2600-shared-edge-junctions-and-inner-vertices.md): ortak kenarın ortasındaki köşeler (üçlü birleşim, ara
  köşe, çizgi ucu) kenarla kayar.

## Dayanak

- **ArcGIS Pro, topolojik düzenleme** (ADR 2600'ün okuması): topolojik olarak bağlı kenarlar uzar ve bağlı kalır.
  Düzenleyen, neyin birlikte değişeceğini düzenlemeden önce görmelidir.
- **KentOS bugün** (ADR 1960, 2600):
  - önizleme komşuların yeni sınırını kesik çizgiyle ve komşu alanların yeni boyunu etikette gösteriyordu;
  - kenarın ortasındaki üçlü birleşim ya da çizgi ucu gibi birlikte kayacak köşeler ayrıca işaretlenmiyordu. Yalnız
    yazımdan sonraki ileti sayısını söylüyordu (“kenarın ortasındaki 2 ortak köşe de kenarla kaydı”);
  - kesik sınırlar üst üste binince hangi köşenin gittiği görülmüyordu.

## Karar

1. **İşaret** (araç önizlemesi; komut ve sözleşme değişmez):
   - Topolojik düzenleme açıkken imlecin kaydırdığı kenarla birlikte yer değiştirecek her komşu köşe, gideceği yerde
     yakalama renginde bir halkayla işaretlenir. Köşe başına bir halka; aynı yere giden iki komşunun köşesi tek halkadır;
   - köşeler komutun kendi kuralının (`neighbours`, ADR 2600) yazacağı şekillerden çıkar: komşunun şimdiki ve
     yazılacak köşeleri sırasıyla karşılaştırılır;
   - etiket sayıyı söyler: “komşunun N köşesi birlikte kayar”.
2. Komşu yoksa ya da kural reddediyorsa (kısalan kenarın dışına düşen köşe, yaylı kenarın ortasındaki köşe) halka
   çizilmez. Etiket eskisi gibi nedeni söyler.
3. **Kapsam işareti:** ADR 2600'deki gibi **CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi**.

## Kanıt

- **Masaüstü** (`edge_shift_junction_scenes.rs`, `a_shared_edge_carries_the_t_junction_and_the_branch_s_end`):
  - A parselinin doğu kenarı 3 m doğuda: önizleme B1'in ve B2'nin kenar üstündeki köşelerini, ortak köşelerini ve çizginin
    ucunu dört halkayla işaretler (210 → 213);
  - etiket “komşunun 4 köşesi birlikte kayar” der;
  - yazım ve geri alma eskisi gibi.

  Test pencereyi bir kez yazılımla çizer (çizim alanı ekrandaki yerini alır), sonra tıklar. Eskiden boyutsuz pencerede
  imleç kenarı kaydırmıyordu, önizleme “0.000 m” idi; testin denetimi bu yüzden zayıftı.
  Paralel kaydır'ın öbür sahneleri ve çapraz denetimler geçer. Resim
  `.run/shots/arac-paralel-kaydir-ucuncu-birlesim-*.png`.
- **MCP ve Python:** komut değişmedi.

## Sonra

- Tutamaç, Köşe tablosu ve Esnet'te kenar içindeki komşu köşe (ADR 2600 Sonra).
- Köşenin şimdiki yerinden gideceği yere ince bir ok.
- Topoloji denetiminin veri formu yolu (MAP-07'nin öbür maddesi).
- Web (TODOS §0.2).
