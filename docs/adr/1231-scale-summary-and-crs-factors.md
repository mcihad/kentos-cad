# 1231 — Ölçeklerin özeti ve ölçek çarpanları (`cad.crs.factors`)

Tarih: 2026-11-06. plan/map-cad.md MAP-02 (ikinci dilim: “ölçek anlatımının tek özeti”). Durum: taslak. Önceki dilim
[ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md).

## Dayanak

- **Ayrı ölçekler, ayrı yerler.** Civil 3D'nin çizim ayarlarında Units and Zone sekmesi birimi, koordinat sistemini ve
  çizim ölçeğini; Transformation sekmesi deniz seviyesi (yükseklik) çarpanını, ızgara ölçek çarpanını (Unity, User
  Defined, Reference Point, Prismoidal) ve ikisinin birleşimini tutar; birleşik çarpan “yerelden deniz seviyesine ve deniz
  seviyesinden ızgaraya” iki çarpanın birleşimidir
  ([Transformation Tab](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-D54E5026-E7EA-481D-8FAD-C3BBDCF189A6.htm),
  [About Units and Zone Settings](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-UserGuide/files/GUID-94F6BAB1-43EA-4F5D-84FB-9D8CD9C77E4F.htm),
  6 Kasım 2026'da okundu). Trimble Access birleşik çarpanı yükseklik çarpanı × projeksiyon ölçeği, zemin ölçek çarpanını
  onun tersi olarak tanımlar ve hesaplanan değeri bir alanda gösterir
  ([To set up a ground coordinate system](https://help.fieldsystems.trimble.com/trimble-access/latest/en/coordinate-system-ground.htm)).
  TBC'de birleşik çarpanı görmek için kullanıcılar nokta tablosuna sütun ekliyor (topluluk forumu; resmî değil).
- **KentOS'ta bugün:** durum çubuğunda tek ölçek vardır: CAD projesinde çizim ölçeği, CBS'de görünümün ölçeği
  ([ADR 0626](0626-units-scale-and-true-size-texts.md) §3). Projeksiyon ölçeği ve yükseklik çarpanı yalnız Mesafe ölç ile
  Alan hesapla'nın satırlarında ve ölçme pencerelerinde görünür (`crs::ground`, [ADR 0171](0171-ground-ellipsoid-plane.md)).
  Kullanıcı “1:1000 çizim”, “ekranda 1:3780” ve “k = 1,0000308” sayılarının hangisinin neyi değiştirdiğini tek yerde
  göremiyordu; Python ve MCP'nin bu çarpanları soracağı bir komut yoktu.

## Karar

1. **Ölçeklerin özeti ▸** (yalnız koordinat sistemi olan projede): durum çubuğunun ölçek menüsünün sonunda bir alt menü
   (menü kısa kalır). Satırlar sırayla: Çizim ölçeği (yazıların kâğıttaki boyu; geometri gerçek boyda), Ekran (yakınlaştırma;
   geometriyi, ölçüleri ve kaydı değiştirmez), Projeksiyon ölçeği (ekranın ortasında, sistemin adıyla; yalnız ölçme
   hesaplarına girer), proje bir ortalama elipsoit yüksekliği taşıyorsa Yükseklik çarpanı R/(R + h) (doğu yönünde; R
   birinci düşey eğrilik yarıçapı N) ve Birleşik çarpan, sonra Ölçülen uzunluklar (projeksiyona indirilir / düzlemde
   kalır; Proje ayarları › Ölçme). Her çarpan sekiz ondalık ve milyonda birle (`1.00003076 (+30.8 ppm)`). Satıra basınca
   değer panoya kopyalanır, ileti bunu söyler; hiçbir şey değişmez. Sistemi olmayan (yerel) proje — Mimari'nin binası —
   özeti görmez; bir noktanın tek ölçeği olmayan sistemde (coğrafi, Pseudo-Mercator, afin yerel) “Tek değer yok” ve
   nedeni; ekranın ortası sisteme ulaşmıyorsa “Ulaşılmıyor”. Hesap `kentos_interaction::scale_summary` (geometry-core'un
   `point_scale` ve `grid_factor`'ı); çizim karesinde değil, menü açılınca.
2. **`cad.crs.factors` v1** (salt okunur, `effect: query`): verilen noktalarda (1–10 000, projenin koordinatlarında)
   `scale`, `heightFactor`, `combined`; `height` verilmezse projenin yüksekliği; sistemin adı, kullanılan yükseklik ve
   `reducesToGrid`. Retler `invalid_input` (nokta yok/çok, sonlu değil, yükseklik −500…9000 m dışında), `no_system`,
   `no_point_scale` (yeni kod); sistemin ulaşmadığı nokta çarpansız, uyarı `point_beyond_system` (yeni kod). Çizim ve
   kamera ölçeği bu çarpanları değiştirmez.
3. **CAD / CBS işareti: ikisi de.** Özet CAD'de (Harita, Yol) çizim ölçeği menüsünde, KentOS GIS'te görünümün ölçeği
   menüsünde aynı satırlarla; Mimari'de (yerel koordinat) yok. `cad.crs.factors` her projede çalışır; Python ve MCP'de
   (salt okunur, yıkıcı listede değil) aynı işleyiciden.

## Bütçe

Özet menü açılırken hesaplanır: bir ters izdüşüm, bir nokta ölçeği ve 1 m'lik bir doğu çizgisinin çarpanı (<0,1 ms).
`cad.crs.factors` nokta başına aynısını yapar; 10 000 nokta tek çağrıda (ölçülmedi; sınır sözleşmede).

## Kanıt

- `kentos_interaction::scale_summary` birim testleri bağımsız değere karşı: TM30'da 550 000 / 4 540 000'de PROJ
  (`invproj` + `proj -V`) k = 1.00003076; GRS80'de N/(N + 850) = 0.9998669420 elle; orta meridyende 1; yerel, coğrafi.
- `cad.crs.factors` 24 ortak durum, bağımsız `scripts/fixtures/crs_factors_cases.py`: projeksiyon ölçeği PROJ'un
  `get_factors`'ı (QGIS'in pyproj'u), yükseklik çarpanı sistemin elipsoidiyle elle; TM30, TM33, WGS 84/UTM 35N (k₀ =
  0,9996, 200 km'de >1), ED50/UTM 35N (Hayford), verilen ve sınırdaki yükseklikler, 50 000 km doğuda ulaşılmayan nokta,
  yerel, coğrafi ve Pseudo-Mercator retleri. Tolerans 10⁻⁹ (PROJ'un sayısal türevi orta meridyende 5·10⁻¹¹ sapar).
- Masaüstü `scale_summary_tests.rs`: harita projesinde altı satır sırasıyla, zoom yalnız Ekran satırını değiştirir, yerel
  projede özet yok. Resimler `.run/gui-evidence/scale-summary-1231/` (ölçek menüsü ve özet alt menüsü, açık ve koyu).

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("/is/harita/ana.kcad")
f = cad.crs.factors(doc, points=[[550000.0, 4540000.0]], height=850.0)
print(f.points[0].scale, f.points[0].height_factor, f.points[0].combined)   # 1.0000307…, 0.9998669…, 0.9998977…
```

MCP: `{"tool": "cad.crs.factors", "arguments": {"points": [[550000, 4540000]]}}`.

## Sonra

- İçe aktarmada birimin bir kez uygulandığının kot (Z) ve iç içe blok için ayrıca sınanması (DXF'in 64 ortak durumu metin
  boyu ve blok ölçeğini zaten sınıyor, `file.import.dxf.json`).
- Ölçme pencerelerinin “ölçek çarpanı” satırının bu komuttan okunması; kot farkıyla değişen yükseklik (nokta başına h).
