# ADR 1122: CAD çizgi tipinin tiresi kaynak eğri üzerinde ölçülür

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` açığı “eğride analitik faz”:
  işaret yerleştiricisi (ADR 0930 §18, §21) gerçek yay uzunluğunu kullanıyordu, taşıyıcının kendi tiresi ise
  çizilen kirişlerin toplamıyla ölçülüyordu (GPU köşe mesafesi, CPU tarayıcı, PDF `d` işleci).
- **Bağlam belgesi:** ADR 0930, ADR 1121, ADR 1063'ün örnekleme kuralı (desenli/çizgi tipli parçalar örneklenmez).

## 1. Kaynaklar

- Autodesk, [About Simple Custom Linetypes](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-EF1DF0A9-2088-487C-8085-16FEE6425405.htm):
  desen eğri boyunca tekrar eder; yaylarda da uçlarda tire olacak biçimde ayarlanır (LIN'in `A` hizalaması).
- Autodesk [PLINEGEN](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-352844E4-FBEA-44E0-B8F3-41C4187AD9C7.htm):
  kesintisiz kipte desen tepe noktalarından fazı sürdürerek geçer.
- ISO 32000-1 §8.4.3.6 (PDF çizgi deseni): tire uzunlukları yol boyunca kullanıcı biriminde ölçülür; eğri
  düzleştirilmiş haliyle ölçüldüğünden kirişli yolda faz kiriş toplamıdır. Bu, PDF'e kirişli eğri verildiğinde
  ölçünün gerçek yay uzunluğu olmayacağının nedenidir.
- KentOS'un mevcut kararı (ADR 0930): sabit periyot ve kesintisiz faz; AutoCAD'in uçları tireye uyduran `A`
  hizalaması uygulanmaz. Bu ADR o kararı değiştirmez, yalnız ölçünün kaynağını değiştirir.

## 2. Karar

- Bir çizimin kendi CAD görünüşleri (`MODE_SET` kümeleri) için `View::cad_phase` açıkken (masaüstü model ve pafta
  ekranı, PDF/vektör/CPU çıktısı, ölçüm), tireli çizgi tipi yay, daire, elips, yaylı çoklu çizgi ve NURBS üzerinde
  `style-core/style/cad_dash.rs` ile kesilir:
  1. Çizilen taşıyıcının her tepe noktasına kaynak eğri üzerindeki gerçek uzaklığı verilir (yay/daire açıdan,
     elips birim uzay parametresinden ve mevcut Gauss–Legendre birikiminden, yaylı yol kenar kenar, NURBS en
     yakın parametre ve `length_between` ile). Uzaklık azalırsa ya da nokta eğriden uzaksa (başka taşıyıcı) analitik
     yol kullanılmaz; eski çizim aynen kalır.
  2. Desen bu uzaklıklarda kesilir (`dash_offset` kuralı PDF fazı). Kiriş içine düşen kesim noktası **eğrinin
     kendi noktasıdır** (`Route::point_at`: yay/düz kenar tam, elips braketli Newton, NURBS uzunluk tersinmesi);
     tepe noktasına düşen kesim tepe noktasıdır. Böylece faz parçalar, tepe noktaları ve kapalı dikiş boyunca
     birikmez; dikişten geçen mürekkep tek parçadır.
  3. Her mürekkep parçası kesintisiz (dash'sız) bir stroke olarak taşıyıcının kendi noktalarıyla çizilir; GPU,
     CPU ve PDF aynı parçaları alır. Nokta (sıfır tire), renderer'ların mevcut nokta kuralını korumak için tek
     öğesi o nokta olan iki noktalı tireli stroke'tur.
- Yol başına 20 000 parça/nokta sınırı aşılırsa analitik kesim yapılmaz, önceki kiriş ölçülü tire kalır (eksik
  çizim değil; ölçü farkı çizim toleransı kadardır). Kırpılmış taşıyıcı parçaları da kendi global uzaklıklarıyla
  kesilir.
- Varsayılan `View::cad_phase = false`: web'in WASM yapısı, `fixtures/style/v1/cases.json` ve web kaydı
  `batches.json` değişmez (iki dondurulmuş test ve web eşdeğerlik testi geçer). Tek sembol önizlemesi girdide
  `"cadPhase": true` ile ister. Masaüstünde açılan yerler: `style/mod.rs` (model), `sheets.rs` (pafta ekranı),
  `style/perf.rs`, `plot/maps.rs` (vektör/PDF/raster). Web'in ihtiyacı TODOS.md §0.2'de.
- Yeni NURBS rotası işaret yerleştiricisine de girer: NURBS üzerindeki yazı/SHX artık kirişlerden değil gerçek
  yay uzunluğundan yerleşir.
- Örnekleme (ADR 1063, başka dalda): bu ADR sembolleri değiştirmez; parçalar nesne derlemesinde üretilir.
  Sembolün `dash`'ı ve çizgi tipi bağı yerinde kaldığından “desenli parça örneklenmez” kuralı geçerli kalır;
  birleşmede ayrıca denetlenmeli.

## 3. Kanıt

- `style-core` `cad_dash::tests` 7/7: daire köşelerinin `r·θ` uzaklığı (kirişle 0,3 m fark), her kirişin yarısı ve
  eğrinin kendi yarı noktası, dikişten geçen tek parça, düz+yay+düz yolda fazın sürmesi, elips çeyreği = toplamın
  dörtte biri, rasyonel NURBS çeyrek çemberinde `θ`, noktalar ve geri düşüş, `View`'ı açmayan derlemenin eski
  tek tireli stroke'u korunması. Bütün `kentos-style-core` (64 + 9) geçti; iki dondurulmuş test dahil.
- `kentos-plot` `curve_dashes`: 1:1/1:20/1:100 × yay/daire/saat yönü yaylı yol ve elips/NURBS: her tire 4 mm'lik
  periyodun katında başlar, 3 mm sürer; ölçü bağımsız `atan2·r`, `ellipse_length` ve `Nurbs::length_between`
  ile, 1e-6 mm içinde; vektörde kiriş ölçülü tire kalmaz; PDF üretilir; belge değişmez. Bütün `kentos-plot`
  (18 + 74, 4 ignored) ve `kentos-native-style` (42 + 44 + 2 + 2; web `batches` ve lejant eşdeğerliği) geçti.
- Gerçek çıktı resimleri (repoya alınmaz): `.run/gui-evidence/adr1122-egri-1-20.pdf`, `-1-100.pdf`, Poppler
  resimleri `adr1122-egri-1-{20,100}-pdf-1.png`, CPU önizlemeleri `adr1122-egri-1-{20,100}-cpu.png`; GPU ekranları
  `.run/shots/arac-egri-desen-{cizim,yakin}-1440x900.png`. 600 dpi PDF resminde 15 mm'lik dairenin 23 tire başı
  4 mm ızgarasından sabit −0,10 mm (ölçüm penceresi ±2 px = ±0,085 mm) ve 0,05 mm (≈1 px) yayılımla; GPU
  ekranında (18,9 px/m) 11 tire başı sabit −0,12 m, yayılım 0,077 m (≈1,5 px): faz birikmiyor.

## 4. Sınırlar

- AutoCAD'in `A` (uçlara uydurma) hizalaması ve PLINEGEN=0 uygulanmaz (ADR 0930 kararı).
- PDF'de sıfır uzunluklu tire (nokta) `butt` uçla çizilmez; bu düz çizgilerde de böyledir ve bu ADR'den önce de
  vardı (CPU önizlemesi ve GPU bir piksel çizer). Noktalı CAD deseninin PDF'de görünmemesi ayrı açık bulgudur
  (TODOS'a yazıldı), bu ADR'de kapatılmadı.
- Örnekleme dalıyla birleşmede `cad_phase` ile örneklenen bloklar ayrıca sınanmalı.
- Gerçek AutoCAD çıktısıyla karşılaştırma yapılmadı.
