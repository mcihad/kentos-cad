# 3340 — Detay eşleştirmede yaylı aday

Tarih: 2026-11-09. plan/map-cad.md MAP-04, altıncı tur. Durum: taslak. ADR 1760'ın “Sonra”sındaki “yaylı adayların yay
uzaklığı” ve ADR 2940'ın “yaylı aday”ı.

İlgili:
- [ADR 1760](1760-asset-candidate-reconciliation.md): adayların uzlaştırılması (Hausdorff; yaylar kirişleriyle);
- [ADR 2940](2940-reconcile-grid-index.md): büyük aday kümesinde ızgara dizini (kutular).

## Dayanak

- **ArcGIS Pro, Densify** ([Densify (Editing)](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/editing/densify.html),
  9 Kasım 2026'da okundu):
  - gerçek eğriler (yaylar) karşılaştırma ve yayından önce ara noktalarla çoklu çizgiye çevrilir;
  - Offset yöntemi eğride en büyük sapmaya göre, Angle yöntemi en büyük sapma açısına göre köşe koyar.

  Yay kirişi değildir; ölçülmeden önce yayın kendisi temsil edilmelidir.
- **DXF bulge** ([ezdxf, LWPOLYLINE](https://ezdxf.readthedocs.io/en/stable/dxfentities/lwpolyline.html); KentOS'un
  `geom::bulge`'ı): kenarın yayı bulge = tan(θ/4) ile verilir.
- **KentOS bugün** (ADR 1760 §2): çoklu çizginin Hausdorff'u köşelerden öbürünün doğru parçalarınadır; yaylar
  kirişleriyle ölçülür. Kirişinden 3 m uzak bir yay ile kiriş boyunca çizilmiş düz bir çizgi 5 cm'de “aynı” bordür
  sayılıyordu. Yazılan varlık da temsilcinin yaylarını taşıyordu ama ölçüm onları görmüyordu.

## Karar

1. **Çoklu çizgi bir eğridir:** kenarları doğru parça ya da yaydır (bulge).
2. **Ayrık Hausdorff'un noktaları:**
   - köşeler;
   - her yayın taradığı açının yedi iç sekizde birindeki noktalar (açının k/8'i, k = 1…7), eğrinin sırasıyla.

   Yarım daire için sekizde birler arasındaki en büyük sapma yarıçapın %1,9'udur. Densify'ın açı yöntemine benzer,
   ama sabit bölmelidir: iki uygulama aynı noktaları üretir.
3. **Noktanın eğriye uzaklığı:** kenarlarına uzaklıklarının en küçüğü. Yaya uzaklık kesindir:
   - noktanın merkezden doğrultusu yayın taradığı açının içindeyse çembere uzaklık, |d − r|;
   - değilse yakın ucuna uzaklık.
4. **Kutu** (ADR 2940'ın ızgarası, var olan varlık araması): köşelerin ve yayların merkezden doğu, kuzey, batı ve güneye
   düşen ve açının içinde kalan noktalarının kutusu. Yayın şişkinliği kutuda olur, çift kaçmaz.
5. **Yazılan varlık** temsilcinin geometrisidir, yaylarıyla (eskisi gibi; şimdi ölçümle tutarlı).
6. Komut, sözleşme ve sürüm değişmez. Yaysız adayların sonuçları aynıdır: noktaları yalnız köşeleridir.
7. **Kapsam işareti:** ADR 1760'taki gibi **Harita** (pencere ve yazma); komut Python ve MCP'de.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_features_reconcile_cases.py`, shapely 2.0.6; yay hesabı betikte, KentOS
  kodu yok):
  - iki durum eklendi (37 durum):
    - 0,5 m yarıçapta kirişinden 3 m uzaktaki yay ile kiriş boyunca (5 cm ötede) düz çizgi iki ayrı tek kaynak kalır.
      Kirişle ölçülseydi bir grup olurdu;
    - kirişinden 1 m uzaktaki yay ile aynı bordürün yayın sekizde birlerinde 2 cm dışarıdan çizilmiş köşeleri uyuşur
      (yayılım 0,020 m). Eşleştir yayın kendisini, bulge'ıyla yazar;
  - eski durumlar aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/reconcile_arcs.rs`): planın grupları ve yayılımı durumunki; yazılan bordür kaydedilen
  dosyada yayıyla durur.
- **Masaüstü** (`features_reconcile_window_tests::an_arc_is_measured_as_an_arc`):
  - 0,5 m'de iki tek ve bir uyuşan grup;
  - Eşleştir tek çoklu çizgiyi yayıyla yazar;
  - resimler `.run/gui-evidence/yayli-aday-3340/`.

  Eşleştirme testleri ve çapraz denetimler geçer.

## Sonra

- Ayrı inceleme kaydı (kararların öznitelik değil kayıt olarak tutulması); AI çıkarımı (MAP-04'ün kalanı).
- Sürüm derlemesinde ve gerçek bir çıkarımın adaylarıyla ölçüm (ADR 2940 Sonra).
- Web (TODOS §0.2).
