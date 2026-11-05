# ADR 1007: Çizgi tipi kaybının viewport'un görünen şekline göre denetlenmesi

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` açığı “Çokgen veya döndürülmüş
  viewportun tam kırpma denetimi de açık; ilk koruma batch sınır kutusunu kullanır.”
- **Bağlam belgesi:** ADR 0930 §22–23 (desteklenmeyen desen teslimi engeller), ADR 1001 (uzun desen),
  `crates/shared/sheet/src/clip.rs` (çerçeve, dikdörtgen, çokgen kırpma).

## 1. Kaynak

Autodesk'in [VPCLIP yardımı](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-D5FD4D1A-5785-4A8E-B0D1-D12079C0A4FF.htm)
viewport'un sınırını bir nesneyle ya da noktalarla tanımlanan çokgenle yeniden tanımladığını, nesnenin değişmediğini
söyler. Kırpılan viewport dikdörtgen görünümün yalnız bir parçasını gösterir. KentOS'ta aynı karar ADR 0789 ve
`Clip` ile vardır: kâğıt ve PDF yalnız kırpma şeklinin içini basar. Belgeler basımda kırpma dışında kalan desen
kaybının nasıl raporlandığını söylemez; aşağıdaki kural KentOS kararıdır.

## 2. Karar

- Haritanın **görünen zemini** `maps::ground_clip`'tir: kırpma şeklinin (çerçeve, dikdörtgen, çokgen) kâğıt
  köşeleri, içeriğin dönüşüyle birlikte (`Transform::paper(...).inverse`) zemine çevrilir. Model kapsamının
  `model_boundary`'siyle aynı dönüşümdür.
- `linetype_unsupported` kaybı, eskisi gibi önce sınır kutusundan (`in_view`) geçer; sonra yığının erişimle
  büyütülmüş kutusu (`StyledBatch::reach_bounds`) görünen zeminle kesişmiyorsa kayıp sayılmaz. Kesişim kesin
  sınamayla değil, kutu–çokgen kesişimiyle (köşe içerme ve kenar kesişimi) yapılır; bu tutucudur: yığının kutusu
  şekle değiyorsa çizgi şekle değmese bile engel kalır, şeklin içindeki kayıp hiçbir zaman atlanmaz.
- Aynı zemin dört yoldan geçer: ön denetim (`maps::line_type_losses`), vektör teslimi ve PDF kitabı
  (`vector_content_clipped`), raster yedek (`raster::map`). Eski `collect_vector_profile`/`vector_content_profile`
  zemin vermeden çağrılır ve eski davranışı korur.
- Belgede ve basılan geometride değişiklik yoktur; yalnız raporlama ve engelleme kararı değişir.

## 3. Kanıt

`crates/native/plot/tests/all/line_types.rs::a_loss_blocks_only_inside_the_polygon_or_turned_viewport_shape`:
eksik SHX şekilli desen (teslim edilemez) altı durumda. Sol üst üçgen kırpmada kutuda ama üçgen dışında kalan çizgi
engellemez, PDF üretilir; sağ üst yarı üçgende çizgi engeller. 45° ve 315° döndürülmüş çerçevede kutunun köşesindeki
(merkezden bir eksen boyunca 14 m, yarı boylar 9,75/5,75 m) çizgi engellemez, merkezdeki engeller; 30° döndürülmüş
sağ çeyrek dikdörtgen kırpmada merkezdeki çizgi engellemez. Beklenen değerler testte elle hesaplanan geometriden
gelir. Çokgen sınaması kapatılınca (mutasyon) test `triangle_outside`'ta kırmızı verdi. Bütün `kentos-plot`
testleri (18 + 71) geçti. Resim: `.run/gui-evidence/adr1007-ucgen-kirpma.png` (`clip_picture`, `--ignored`):
üçgenin içindeki kesikli çizgi basılı, üçgen dışındaki desteklenmeyen desenli çizgi çıktıyı engellemedi.

## 4. Sınırlar

- Yığın kutusu sınaması tutucudur: uzun eğik bir çizginin kutusu şekle değip kendisi değmiyorsa engel sürer. Parça
  düzeyinde kesin kırpma ayrı iştir.
- Kâğıt sistemi projeninkinden farklı (yeniden izdüşümlü) haritada zemin, sınır kutusuyla aynı koordinatta alınır;
  bu durum ayrıca test edilmedi.
- Unicode/Big Font SHX, noktalı bant, eğride analitik faz ve 2M nokta sınırının ötesi CAD-ADV-11'de açık kalır.
