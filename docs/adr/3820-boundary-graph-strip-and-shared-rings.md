# 3820 — Sınır grafiği çizimde: kirişli yayın şeridi, aynı yerdeki sorunlar tek sözde

Tarih: 2026-11-09. plan/map-cad.md MAP-08, yedinci tur. Durum: taslak. ADR 3420'nin “Sonra”sındaki “Çizimde zincirin ve
yayın vurgulanması” ile ADR 3020'nin ve 3420'nin “aynı yerdeki sorun sözlerinin tek sözde birleşmesi”.

İlgili:
- [ADR 2220](2220-boundary-graph-on-the-drawing-and-arc-corners.md): sınır grafiği çizimde (halkalar, sözler, numaralar);
- [ADR 3020](3020-boundary-graph-row-on-the-drawing.md): seçilen satırın çizimde öne çıkması;
- [ADR 3420](3420-boundary-arc-drawn-with-chords.md): kirişlerle çizilmiş yay (`approxArc`).

## Dayanak

- **QGIS, Point Cluster Renderer**
  ([Vector Properties](https://docs.qgis.org/3.40/en/docs/user_manual/working_with_vector/vector_properties.html),
  9 Kasım 2026'da okundu):
  - belli bir uzaklıktan yakın noktalar tek bir işarette birleşir;
  - işaret öntanımlı olarak birleşen nesnelerin sayısını yazar;
  - noktaların yerleri değişmez, yalnız görünüşleri.
- **ADR 3420:** kirişli yayın sorunu yayı, zincirin sınırlarını, şeridin alanını ve en geniş yerini verir. Çizimde
  yalnız en geniş yer halkalanıyordu; şeridin kendisi görünmüyordu.
- **ADR 3020'nin resmi:** birbirine yakın sorunların sözleri üst üste biniyordu. Özellikle uzaktan bakınca okunmuyordu.

## Karar

1. **Şerit** (Çizimde göster, `approxArc`): yay ile kirişlerin arası kapalı bir çizgiyle çizilir:
   - yay ilk köşesinden son köşesine, sonra zincir sondan başa (her sınır yürüdüğü yönde, yayı varsa bulge'ı ters);
   - tehlike renginde soluk doldurulur (saydamlık 0,22) ve çevresi çizilir; halkalardan ve sözlerden önce, altta;
   - yay şeridin en geniş yerinin 1/200'ü sehimle noktalanır: şerit dar olduğundan, kirişin 1/2000'i (ADR 3020'nin
     payı) alanı %7 eksik veriyordu. Böylece alanı sorunun alanıyla %1 içinde aynıdır;
   - zincir yayın iki ucunu birleştirmiyorsa çizilmez.
2. **Halkası buluşan sorunlar** (ekranda iki halka yarıçapından, 18 px'ten yakın) tek halkada toplanır, ilkinin
   yerinde:
   - sözleri tek söz olur: her tür bir kez, ilk görüldüğü sırada; birden çoksa sayısı:
     “Komşuda eksik köşe (2) · Çakışan kenar”;
   - toplama ekran uzaklığıyladır, yakınlaştırınca halkalar yeniden ayrılır (QGIS'in kümesi gibi);
   - sorunların kendisi, tablo ve komutun cevabı değişmez; yalnız görünüş değişir.
3. **Hiçbir şey yazılmaz;** sözleşme değişmez. Python ve MCP şeridi sorunun `boundaries`'inden, sözleri `kind`'dan okur.
4. **Kapsam işareti:** ADR 2000'deki gibi **Harita + CBS**.

## Kanıt

- **Masaüstü** (`parcel_boundaries_marks_tests`):
  - iki kirişli yay durumunda (köşeler yayda; eşit olmayan kirişler ve T birleşimi) şerit bir tane, yayın ilk köşesinde
    başlar ve orada kapanır, zincirin her köşesi üstündedir;
  - köşeleri yayda olan durumda şeridin alanı sorunun alanıdır (%1 içinde). Bağımsız Python hesabı (yay segmenti eksi
    zincirin çokgeni) durumun alanını 1.72978 m² verdi; kaba noktalama 1.6066 m² buluyordu;
  - üç yakın sorun bir halkada tek söz, uzaktaki ayrı; 18,5 px uzaktaki ayrı kalır;
  - Çizimde göster'den sonra işaretlerde şerit vardır.

  Sınır grafiği'nin öbür testleri geçer. Resimler `.run/gui-evidence/kirisli-yay-serit-3820/` (şeridin en geniş yerine
  yakın; sorunlu ada uzaktan, birleşmiş sözler; açık geniş ve koyu dar).
- **MCP ve Python:** cevap değişmedi (yalnız masaüstü kodu değişti).

## Sonra

- ~~Kalıcı parsel kaydı ve sınır kimliği (MAP-08'in ana açığı): saklanan bir veri modeli ister (belge, geçerlilik,
  önceki ve sonraki parsel, mantıksal sınır); ayrı bir tasarım dilimi.~~ ADR 4220.
- Erişim denetimli hak/pay; resmî kapsam.
- Web (TODOS §0.2).
