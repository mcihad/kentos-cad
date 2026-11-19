# ADR 8169: Eğrisel aks ağı IFC'de RADIAL ızgara; aksların eğrisi

- **Durum:** uygulandı (19 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 ("Kalan: RADIAL aks"),
  [bim-plan.md](../../plan/bim-plan.md) B9.
- **Bağlam:** ADR 0725 (eğrisel aks ağı: merkez, ilk açı, dönüşler, ilk yarıçap, halkalar, uzatma; ışınsal aks
  merkezi geçmez), 8070 §4 (aks ızgarası: ilk düz aksa paralel olanlar U, öbürleri V; RECTANGULAR ya da IRREGULAR;
  tek yönlü akslar dışarıda), 8071 §1 (yaylar gerçek yay), 8072 §2 (bina dosyasında akslar binanın), 8102 (altlık
  denetimi dosyanın akslarını okur), 8168 (1 mm: projenin mekânsal toleransı).
- **Dayanak:** IFC 4.3 `IfcGridTypeEnum`, `IfcGrid` (Footprint Geometry kavramı ve gayriresmî önermeleri),
  `IfcGridAxis`, `IfcTrimmedCurve`, `IfcCircle` (buildingSMART IFC4.3.x-development); Dijital Proje Yönetmeliği (RG
  5.8.2026/33331) EK-6 Tablo 6.36 (`IfcGridAxis` `AxisTag`).
- **Sözleşme:** değişmedi. **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

- Eğrisel aks ağı IFC'de düzensiz (IRREGULAR) bir ızgaraydı: ilk ışınsal aks U, öbür ışınsal akslar ve yaylar V. Okuyan
  bir program merkezi, dönüş sırasını ve halkaları bulamıyordu; binanın dik aksları da aynı ızgaraya karışıyordu.
- Akslar `IfcIndexedPolyCurve`'dü. IFC 4.3'ün ızgara ayak izi kavramı aks eğrisi olarak `IfcPolyline`, `IfcCircle`,
  `IfcTrimmedCurve` ve `IfcOffsetCurve2D` sayar. KentOS'un altlık denetimi (8102) de yalnız `IfcPolyline` aksları
  okuyordu: KentOS'un yazdığı dosyanın akslarını göremiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcGridTypeEnum` | RADIAL: U aksları aynı merkezde kesişir ve saat yönünün tersine döner, V aksları aynı merkezli dairesel yaylardır. RECTANGULAR dik düz U ve V; IRREGULAR bu kalıplara uymayan | Işınsal sistem RADIAL bir ızgara: U ışınsal akslar saat yönünün tersine, V yaylar (K439) |
| IFC 4.3 `IfcGrid`, Footprint Geometry | Ayak izi tek bir `IfcGeometricCurveSet`; uygulanan eğriler `IfcPolyline`, `IfcCircle`, `IfcTrimmedCurve` (`IfcLine` ya da `IfcCircle` üstünde), `IfcOffsetCurve2D` (öneri) | Düz aks `IfcPolyline`, yay `IfcCircle` üstünde `IfcTrimmedCurve`; bütün ızgaralarda (K440) |
| IFC 4.3 `IfcGrid` gayriresmî önermeleri | Ayrı listelerdeki akslar paralel değildir; iki aks iki kez kesişmez | Işınsal aks merkezin bir yanında durur, her yayı bir kez keser (bağımsız okuyucu denetler) |
| IFC 4.3 `IfcTrimmedCurve` | Uçlar nokta ya da parametreyle; ikisi verilirse tutarlı; MasterRepresentation esas olanı, SenseAgreement taban eğrisine göre yönü söyler | Uçların noktası ve merkez çevresindeki açısı (radyan, dosyanın açı birimi) birlikte, esas nokta; saat yönündeki yay `.F.` (K440) |
| Autodesk revit-ifc `GridExporter.cs` (açık kaynak) | Yayları merkezlerine göre gruplar; yayları U, merkezden geçen düz aksları V yazar | Standardın metni esas (U ışınsal, V yay). Okuyucu U ve V'nin türüne bakmaz, ters yazılmış dosya da okunur (K441) |
| EK-6 Tablo 6.36 | `IfcGridAxis` için `AxisTag` | Her aks etiketiyle (değişmedi) |
| [Sayısal politika](../specs/numeric-policy.md) §5 | Mekânsal tolerans 1 mm | Aynı merkez ve merkezden geçen doğru 1 mm içinde (K439) |

## §3. Karar

- **K439 — Işınsal sistem:** bir katın (yoksa binanın) aksları arasında aynı merkezli (1 mm içinde) yaylar ile doğrusu o
  merkezden geçen ve merkezi ucunda ya da dışında bırakan düz akslar bir ışınsal sistemdir. Kendi `IfcGrid`'idir:
  PredefinedType RADIAL, adı `MM-AKS-Eğrisel_aks`. U ışınsal akslardır, merkez çevresinde saat yönünün tersine,
  aralarındaki en geniş boşluktan sonraki ilk akstan başlayarak; V yaylardır, merkezden dışa. Düz aksı olmayan
  yaylar ışınsal değildir. Kalan akslar eskisi gibi (8070 §4) ayrı bir ızgaradır (`MM-AKS-Aks`; RECTANGULAR ya da
  IRREGULAR); merkezden geçip onu ortasında bırakan aks (kesit binasının 2 ve D aksları) orada kalır. Her ızgaranın
  GlobalId'si kendi akslarının en küçük kalıcı kimliğinden türer; ışınsal sistemi olmayan çizimin ızgarası değişmez.
- **K440 — Aks eğrisi:** düz aks iki noktalı `IfcPolyline`'dır. Yay, merkezine konmuş (`IfcAxis2Placement2D`)
  yarıçaplı bir `IfcCircle` üstünde `IfcTrimmedCurve`'dür: Trim1 başlangıç, Trim2 bitiş noktası ve merkez çevresindeki
  açısı (radyan, 0–2π), MasterRepresentation CARTESIAN, saat yönünün tersine giden yayda SenseAgreement `.T.`, saat
  yönünde `.F.`. Merkez yayın başlangıcından hesaplanır, büyük koordinatta küçük farklar basamaklarını korur.
- **K441 — Okuyucu ve görüntüleyici:** altlık denetiminin aks okuyucusu `IfcPolyline`'ın ve `IfcIndexedPolyCurve`'ün
  ilk ve son noktasını, `IfcTrimmedCurve`'ün kırpma noktalarını okur; yalnız parametreyle kırpılmış eğri tahmin
  edilmez, dışarıda kalır. IFC modeli'nin Özellikler'inde ızgaranın Akslar grubu vardır: U aksları, V aksları (ve W
  aksları), etiketleri dosyadaki sırayla.

## §4. Uygulama

- `crates/shared/ifc/src/grids.rs` (yeni; `arch.rs`'teki aks eşlemesi buraya taşındı, ışınsal sistemle),
  `building.rs` (aks eğrileri, `circle`), `math.rs` (`atan2`), `read/coordination.rs` (`ends`), `read/details.rs`
  (Akslar).
- `scripts/fixtures/radial_grid_check.py` (bağımsız okuyucu), `scripts/fixtures/arch_ifc_check.py` (binanın aksları
  `IfcPolyline`).

## §5. Sınamalar

- **Birim** (`grids`): büyük koordinatta saat yönünde çizilmiş beş ışınsal aks ve iki yay tek RADIAL ızgara (U 8, 7, 6,
  5, 4; V E, F); merkezden geçen iki dik aks kendi RECTANGULAR ızgarasında; GlobalId'ler kendi akslarından. Doğuyu kesen
  yelpazede sıra en geniş boşluktan sonra başlar. Merkezin doğrusundan 2 mm kayık aks ışınsal sayılmaz; tek yönlü akslar
  dışarıda kalır.
- **Yazıcı** (`building::a_radial_grid_s_arcs_are_trimmed_circles`): düz akslar `IfcPolyline`, yaylar `IfcCircle`
  üstünde `IfcTrimmedCurve` (4 ve 8 m; kırpma açıları; saat yönündeki `.F.`). Geri okunuşta Akslar grubu ve altlık
  okuyucusunun aks uçları.
- **Uygulama** (`arch_ifc::a_curved_grid_system_is_a_radial_grid`): kesit binasına Eğrisel aks ağı çizilir (merkez
  doğu cephesinin ortası, −45°'den 30° dönüşlerle 4–7, yaylar E 6 m ve F 10 m, uzatma 1 m). İki ızgara çıkar:
  binanınki RECTANGULAR (A–D, 1–3), eğrisel RADIAL (4–7, E ve F); 13 aks; her aksın eğrisi `IfcPolyline` ya da
  `IfcTrimmedCurve`.
- **Altın dosyalar:** kesit binası ve türevleri, kalkan duvarı ve yerel sistem yeniden yazıldı; değişen yalnız aks
  eğrileridir (her aksa bir örnek fazla), GlobalId'ler aynı. Olgular, okuma dosyaları (örnek numaraları, kayıt sayısı)
  ve kapısı eksik türev dosya yenilendi.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0): `radial_grid_check` eğrisel aks dosyasında doğrulayıcıda 0 bulgu verir. U
  doğruları (12; 4,5)'te buluşur, −45°, −15°, 15°, 45° sırasıyla; V yayları aynı merkezde 6 ve 10 m. Kırpma noktaları
  çemberin üstünde, açıları parametreleriyle aynı; yay her U yönünü kapsar, her U her yayı bir kez keser; binanın
  ızgarası tanımdaki gibidir. `arch_ifc_check` (üç dosya; akslar `IfcPolyline`), `ifc_types_check`,
  `slab_layers_check`, `storey_spans_check`, `curved_ifc_check`, `ifc_read_cases --check`,
  `ifc_missing_door_case --check`, `ids_ifctester_check --check` ve `ids_check` uyar; dijital proje paketinin
  sınamaları geçer.
- **Masaüstü:** `ifc_viewer_tests` ve ortak süzgeçler.
- **Resimler:** `.run/gui-evidence/egrisel-aks-8169/`: planda kesit binasının yanında eğrisel aks ağı; IFC modeli'nde
  `MM-AKS-Eğrisel_aks` seçili, Özellikler'de Ön tanımlı tür RADIAL, Akslar: U 4, 5, 6, 7; V E, F; açık ve koyu.

## §6. Kalanlar

- Görüntüleyicide aksların planda ve 3B'de çizilmesi (bugün ızgara "Gösterilemeyen"dedir).
- Üçgen (TRIANGULAR) ızgara; bir katın kalan aksları arasında birden çok dik sistem (bugün kalanlar tek ızgara).
- Yalnız parametreyle kırpılmış aksların okunması (başka programların dosyaları).
- Kimlikleri her yazışta yenilenen eski altın dosyalar (blok A ve B, çatı altı mahal, eğimli döşeme, katmanlı çatı)
  eski aks eğrileriyle kalır; okuma örneği olarak geçerlidirler.
