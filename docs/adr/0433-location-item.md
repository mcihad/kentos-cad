# ADR 0433: Formda konum

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AP`; `plan/form.md` FORM-DESIGN-06'nın harita konumu.
- **Bağlam belgesi:** ADR 0401 (form öğeleri), 0407 (ekler).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Formla kayıt düzenlenirken kaydın nerede olduğu ve büyüklüğü çoğu zaman sorulur (parselin alanı, yolun uzunluğu). QGIS'in
formunda kaydın şekli görünmez; ölçüler yalnız ifade alanıyla ve elle yazılarak gösterilir.

## Karar

- Form öğesi **Konum** (`location`: kimlik, başlık, görünme koşulu): kaydın şekli küçük bir haritada, kuzey yukarı (alanı dolu,
  çizgisi vurgu renginde, noktası daire); yanında merkezin koordinatları (kapalı şeklin alan ağırlık merkezi, açık şeklin
  kutusunun ortası; projenin eksen adlarıyla, Y önce), kapalı şekilde alan ve çevre, açık şekilde uzunluk; projenin
  biçimlendiricisiyle. Hiçbir şey yazmaz; yeni kayıtta ve tasarımcının önizlemesinde “kaydedildikten sonra görünür” der.
- Öğenin kuralı sözleşmede (`body_problem`: başlık, koşul), ön denetim koşulu derler, alanların başvurusu (`field_refs`) koşulu
  izler. Tasarımcının paletinde **Konum** (iğne ikonu) ve özellikleri.
- Bağımsız başvuru `forms_cases.py` ve `.kcad` okuyucusu `kcad.py` öğeyi tanır.

## Denetimler

- `forms_cases.py --check`: ekli ağaç formuna koşullu bir konum öğesi (ön denetim ve öğelerin durumları).
- Masaüstü: `location::tests::a_rings_centre_is_its_area_centroid`; resim `form-konum` (L biçimli parsel: 664 m², 116 m).

## Sonuçlar

- Kaydın yeri ve ölçüsü formun içinde, hesap çekirdeğin ölçüleriyle.
- Açık: konumu formdan seçmek ya da değiştirmek (haritada tıklama), grafik bileşeni.
