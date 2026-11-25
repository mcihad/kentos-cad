# ADR 9233: NURBS ve spline, derin görünümde ve ekranda toleransla çizilir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-09, 3. adım.
- **İstek:**
  - planın RM-09 maddesi: “Spline ve NURBS kutuda ve ekranda toleransla”;
  - sahibin derin yakınlaşma isteği ([ADR 9220](9220-deep-view-scene.md));
  - sahibin “kaliteden ödün verilmez” sözü ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut, ayar ve stil topluluklarının biçimi aynı.

## §1 Bulgu

Derin görünüm ve ekranın eğrileri ([ADR 9223](9223-screen-curves-round-at-every-zoom.md)) daireyi, yayı, elipsi ve
bombeli kenarı görünümün toleransıyla çiziyordu. Spline ve NURBS kendi kirişleriyle kalıyordu:

- **NURBS:** sabit 0,1 mm tolerans (`geom::nurbs::TOLERANCE`). 500 px/m'de 0,05 piksel, ama 10⁴ px/m'de 1, 10⁶ px/m'de
  100 piksel.
- **Spline** (KentOS'un merkezcil Catmull-Rom'u): açıklık başına 16 adım, eğrilikten bağımsız.
  - Sözleşme örneğinin 10 m'lik açıklıklarında kirişin sapması 6 mm; 500 px/m'de 3 piksel.

## §2 Karar

### §2.1 Kutuda toleransla, uzağında çizimin kendi kirişleri

**NURBS** (`geom::nurbs::Nurbs::tessellate_in_box`):

- Bölme kuralı mevcut yarılamanın aynısı.
- Parçanın beş yoklamasının kutusu, kirişten saptıkları kadar büyütülüp derin görünümün kutusuna değiyorsa
  `min(tolerans, 0,1 mm)` ile bölünür, değmiyorsa 0,1 mm ile.

**Spline** (`geom::spline::catmull_rom_in_box`):

- Önce bugünkü 16 adım.
- Kutunun yakınındaki her adım, ortası kirişinden toleransın ötesindeyse ikiye bölünür (en çok 2¹⁸).

**İkisi de hiçbir yerde bugünkünden kaba değil:**

- Uzak görünümün “her yer” kutusunda NURBS bugünkü noktaların aynısı.
- Spline bugünkü noktaları içerir.

### §2.2 Stil çekirdeği

`style::build::deep::curve`, NURBS'ü ve spline'ı da derin görünümün (ve ekranın “her yer” toleransının) eğrileri
arasına alır:

- Eğrinin kendisi başa döndüğü için açık çizilir, çizimin çizdiği gibi.
- Geçersiz bir NURBS (`nurbs::of` → yok) eskisi gibi denetim çokgeniyle çizilir.

## §3 Kanıt

**Sınamalar:**

- `kentos-geometry-core` `geom::nurbs::tests::a_curve_in_a_box_is_round_to_the_box_s_tolerance`. Rasyonel çeyrek
  çember, 2 mm'lik bir kutu ve 10⁻⁹ tolerans:
  - her nokta çemberin üstünde;
  - kutudaki her kirişin sapması toleransın içinde;
  - 200'den az nokta;
  - her yer kutusunda `tessellate(TOLERANCE)` ile birebir aynı.
- `kentos-geometry-core` `geom::spline::tests::a_spline_in_a_box_is_round_to_the_box_s_tolerance`:
  - gevşek toleransla çıktının aynısı;
  - sıkı toleransla kutuda daha çok nokta, bütün çıktı noktaları korunuyor;
  - kirişlerin ortası 4 096 adımlık eğriye 10⁻⁶ içinde.
- Geometri ve stil çekirdeğinin derin görünüm sınamaları, çağrı fikstürleri, CAD uygunluk sınaması geçti. Derin
  görünümün eski sınamalarındaki yasaklı `f64` yöntemleri `jsmath`'e çevrildi.

**Altın görüntüler:**

- `cizim` izni aştı: 478 piksel farklı, 45'inin eşi yok.
- Neden: sözleşme örneğinin kesikli spline'ı artık 16 yerine yaklaşık 64 adım. Yolun uzunluğu gerçeğine yaklaştığı
  için kesikler yol boyunca biraz kaydı; eğrinin biçimi aynı.
- Görüntü yeniden yazıldı; öbür 14 görüntü bayt bayt aynı.

**Gerçek pencere** (`.run/gui-evidence/render/rm09/once-sonra-nurbs.png`):

- Kurulum: 1 m genişliğinde, derece 3 bir NURBS (`.run/rm09/blok/egri-nurbs.dxf`); tepesine tekerlekle yaklaşık
  50 000 px/m'ye yakınlaşıldı.
- **Önce:** tepede köşe yapan düz kirişler.
- **Sonra:** düzgün bir yay.

## Uygulama

- `crates/shared/geometry-core/src/geom/nurbs.rs`: `tessellate_in_box`, `refine_in_box`; sınama.
- `crates/shared/geometry-core/src/geom/spline.rs`: `catmull_rom_in_box`, `segment_distance`; sınama.
- `crates/shared/style-core/src/style/build/deep.rs`: `curve` NURBS ve spline'ı alır; sınamaların `f64` yöntemleri.
- `crates/shared/geometry-core/src/geom/deep.rs`: sınamaların `f64` yöntemleri.
- `fixtures/render/golden/cizim.png`: yeniden yazıldı.
