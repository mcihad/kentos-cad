# 1261 — Topolojik düzenleme: kilitli komşuda durdur

Tarih: 2026-11-06. plan/map-cad.md MAP-07 (“kilitli/yetkisiz komşuda kısmi tutarsız veri yerine açıklanabilir ret”).
Durum: taslak. İlgili: [ADR 0160](0160-topological-editing.md) §2 (kilitli komşu ayrılır), [ADR 1260](1260-topology-quality-review-and-repair-recipe.md).

## Dayanak

- **QGIS:** topolojik düzenleme açıkken ortak köşe ve kenarlar komşu nesnelerde de taşınır; bu yalnız görünür ve
  düzenleme kipindeki katmanlar arasında olur
  ([QGIS 3.10 Editing](https://docs.qgis.org/3.10/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html),
  [topo_editing eğitim belgesi](https://github.com/qgis/QGIS-Documentation/blob/master/docs/training_manual/create_vector_data/topo_editing.rst),
  6 Kasım 2026'da okundu). Düzenlenemeyen komşu geride kalır: sınır ayrılır.
- **ArcGIS Pro:** harita topolojisinde paylaşılan kenar ve düğüm düzenlenebilir katmanlar arasında birlikte taşınır;
  kullanıcılar paylaşılan düğümün kenarlarıyla taşınamadığı durumları bildiriyor (Esri Community,
  [How to modify the shared edge of adjacent polygons](https://community.esri.com/t5/arcgis-pro-questions/how-to-modify-the-shared-edge-of-adjacent-polygons/td-p/29076);
  resmî değil).
- **KentOS bugün (ADR 0160 §2):** kilitli komşu değişmez, düzenleme yine yapılır, ayrılan komşu sayısı söylenir
  (QGIS'inki gibi). MAP-07 ortak sınırın kısmen ayrılması yerine açıklanabilir reddi istiyor; sahibin bekleyen
  “ADR 0160 kilitli komşu” kararı da bu seçimdi. Rakipler bugünkü davranışı yapıyor; kadastroda ise ayrılan sınır
  sonradan bulunması zor bir boşluk/bindirmedir. Bu yüzden ikisi de sunulur, bugünkü davranış ilk değerdir.
- **Netcad:** topolojik düzenlemede kilitli tabakanın nasıl davrandığını anlatan resmî belge bulunamadı.

## Karar

1. **Ayar `drafting.topologyStopOnLocked`** (oturum, ilk değer kapalı): **Kilitli komşuda durdur**. Komutu
   `draft.topologyStopOnLocked` (ikon `topologyStopLocked`: kilitli kutuya dayanan kırık sınır), Topoloji hücresinin
   sağ tık menüsünde Noktalar da'nın altında; Komut ara'da da. Komut hiçbir şeridin olmadığı için her programda
   (**Harita ve GIS**; topolojik düzenleme ikisinde de vardır).
2. **Açıkken** ortak köşe ya da kenarı kilitli katmandaki bir nesneyle paylaşan düzenleme hiç yazılmaz: tutamaçla
   taşıma (tutamaç yerinde kalır), Köşe tablosu, Esnet ve `cad.entities.vertex`. İleti hepsinde aynı: “Ortak sınır
   kilitli katmandaki N komşu nesneyle paylaşılıyor; hiçbir şey yazılmadı. Kilidi açın ya da Topolojik düzenleme ›
   Kilitli komşuda durdur seçeneğini kapatın.” Komutların girdisinde `topology.stopOnLocked`; ret kodu
   `shared_boundary_locked` (yeni), yol `topology`. Kapalıyken ADR 0160'taki gibi: kilitli komşu kalır, `neighbour_locked`.
3. Geçersiz kalacak komşu (ADR 0160 §3) bu ayardan bağımsızdır; uyarıyla değişmez.

## Kanıt

- Ortak durumlar: `cad.entities.stretch` (`reshape_command_cases.py`) ve `cad.entities.vertex`
  (`vertex_command_cases.py`) bağımsız başvurularına “kilitli komşuda durdur” durumları: ret, hiçbir şey yazılmaz.
- Masaüstü `vertices::tests::with_stop_on_locked_a_locked_neighbour_stops_the_table`: komşu kendi kilitli katmanında,
  tabloya yazılan ortak köşe hiçbir şey yazmaz ve nedeni söylenir; ayar kapanınca köşe taşınır, kilitli komşu kalır.
  Resim `.run/gui-evidence/kilitli-komsu-1261/kilitli-komsuda-durdu.png`.
- Python/MCP: `cad.entities.vertex(doc, …, topology={"stopOnLocked": True})` ve `cad.entities.stretch` aynı işleyiciden.

## Sonra

- Yetkisiz komşu (bulut projesinde katman/kayıt yetkisi) için aynı ret; yay, delik, üçlü birleşim ve çoklu köşede
  önizlemede kilitli komşunun vurgulanması (MAP-07'nin kalanı).
