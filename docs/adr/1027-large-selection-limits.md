# ADR 1027: Büyük seçimde Öznitelikler ve vurgu sınırı

- **Durum:** kabul edildi (2026-11-06; sahibin kararı: “Evet, sınır koy”). Kapsam: plan/cad-advanced.md `CAD-ADV-19`,
  ADR 1066 §Kararlar (10⁶ nesnede Tümünü seç 1,15 s: yarısı vurgu kurulumu, kalanı Özniteliklerin değerleri).
- **Bağlam belgesi:** ADR 1066, ADR 0029 (seçim vurgusu), `kentos_interaction::select::GRIP_LIMIT` (150, var olan).

## 1. Kaynak

AutoCAD büyük seçimde işi sınırlar: [PROPOBJLIMIT](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-LT/files/GUID-6170C0EC-0910-46A6-81DF-DE0A888573A0.htm)
(varsayılan 25 000) aşılınca Özellikler paleti değer göstermez,
[SELECTIONPREVIEWLIMIT](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-9CDDA4F9-5B88-428F-B0C8-345E9A9DC4F1.htm)
(varsayılan 20 000) seçim önizlemesini kapatır, [GRIPOBJLIMIT](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Core/files/GUID-705F3A42-4A2F-4B5C-A2A6-0CF8949B8ED5.htm)
(varsayılan 100) tutamaçları gizler. KentOS'ta tutamaç sınırı zaten 150'dir (web'in değeri).

## 2. Karar

- **Öznitelikler:** seçim 25 000 nesneyi aşınca pano yalnız sayıyı, türleri ve “Büyük seçim” notunu gösterir; değer
  ve düzenleyici yoktur. Kilitli nesne sayısı katman başına bir kez bakılarak sayılır. Tür seçimiyle 25 000'e inen
  seçimde değerler geri gelir. Komutlar (Katmana taşı, Özellik kopyala, Taşı…) etkilenmez.
- **Vurgu:** seçim 20 000 nesneyi aşınca vurgu nesne nesne değil, seçimin geometri deposundaki sınırlarının kutusu
  olarak (dolgusuz, vurgu renginde) çizilir. Üzerine gelinen nesnenin vurgusu değişmez.
- Sınırlar sabittir (ayar yok): AutoCAD'in varsayılanları; görünür etkileri pano notu ve kutudur.

## 3. Kanıt

- Masaüstü `properties::tests::a_selection_past_the_limit_shows_its_size_and_no_editors`: 25 001 nesnede tek “Büyük
  seçim” bölümü, düzenleyici yok, hiçbiri kilitli sayılmaz; 25 000'de değerler ve düzenleyiciler önceki gibi.
- `viewport::highlight::tests::a_selection_past_the_limit_is_highlighted_as_its_box`: 30 001 çizginin kutusu deponun
  sınırlarıdır (0…299,5 × 0…100,25 m).
- Resim `secim-buyuk` (30 001 çizgi, kutu vurgusu ve pano notu), bakıldı; ilk çekimde “30001 kilitli” yanlış sayımı ve
  kesilen not düzeltildi.
- 10⁶ Tümünü seç ölçümü bu değişiklikten sonra yeniden alınmadı; CAD-ADV-19'un yakınlaştırma işinin ölçüm koşusunda
  alınacak.

## 4. Sınırlar

Pencereyle seçim sırasındaki önizleme ayrıca sınırlanmadı (KentOS'ta pencere bırakılınca seçilir, ara önizleme yok).
