# ADR 1024: Paralel kaydır yaylı kenarda ve yay komşusunda; yay kendi çemberinde kalır

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/map-cad.md `MAP-06` maddesi “Düz kenarla sınırlı mevcut alan/kenar
  işlemini olduğu gibi koru. Eğrisel ve çok parçalı kapsam yeni matematik/kabul işi olarak açılsın; yay doğruluğunu
  düşürüp sessiz kirişlere dönüştürerek çözülmesin” ve kabulü “gerçek koordinatta yaylı bordür/ada kenarı ... üzerinde
  mesafeli düzenleme yap; bağımsız geometri başvurusuyla karşılaştır”. ADR 0191'in sınırı (“yay kenar ya da komşusu
  reddedilir”) kalktı.
- **Bağlam belgesi:** ADR 0191 (Paralel kaydır), [K16] `ops/edge_shift.rs`.

## 1. Kaynak

- QGIS gelişmiş sayısallaştırma (`QgsAdvancedDigitizingDockWidget`, `QgsCadUtils::alignMapPoint`, plan [S21]): kesin
  giriş ve kısıtlar UI ile komutun aynı hizalama çağrısından geçer; KentOS'ta da araç, `cad.shapes.edit` ve web aynı
  `edgeShift` işlemini çağırır.
- Bir yayın paralel ötelenmesi aynı merkezli yaydır (offset eğrisinin tanımı: yarıçap ±d). KentOS kararı: komşu yay
  da kendi çemberini (merkez, yarıçap) ve uzak ucunu korur; değişen yalnız tarama açısıdır (yeni bulge = tan(θ′/4)).

## 2. Karar

- Kenar ya da komşularından biri yaysa (`edge_shift_arcs`): taşınan kenarın eğrisi düz kenarda normal boyunca `d`
  kaydırılmış doğrusu, yayda yarıçapı `d` kadar büyüyen (normal merkezden uzağı gösteriyorsa) ya da küçülen aynı
  merkezli çemberidir. Her köşe bu eğrinin komşunun eğrisiyle (doğrusu ya da çemberi) kesiştiği, eski köşeye yakın
  noktaya gider; açık çoklu çizginin serbest ucu kenara dik (yayda yarıçap boyunca) gider. Hiçbir yay kiriş olmaz.
- Retler ADR 0191'inkiler: komşu doğru kenara paralel, eğriler kesişmiyor ya da komşu/kenar aşılıyor (yarıçap sıfıra
  iniyor, tarama yok oluyor ya da yarım turdan çok sıçrıyor) → “Kenar bu uzaklıkta komşusunu aşıyor”.
- Hedef alan: yaylı durumda alan `d`'nin ikinci dereceden fonksiyonu değildir; 0'dan dışa doğru `|A₀ − hedef| /
  kenar boyu` ile başlayıp ikiye katlanarak hedefi aşan aralık bulunur, sonra son bitine dek ikiye bölünür (her
  platformda aynı adımlar). Tümü düz kenarlarda eski kapalı biçim aynen kalır (eski vakalar bayt bayt aynı).
- Alanın boyu kenar kaydırmada halkanın ilk köşesine göre (yerel kökende) hesaplanır: farklar yakın koordinatlarda
  kesindir.
- Masaüstü aracı, `cad.shapes.edit`'in `edgeShift` işlemi ve WASM işlem tablosu aynı kuralı alır (komut sürümü
  değişmez; reddedilen bir durum artık yapılır).

## 3. Kanıt

- `scripts/fixtures/edge_shift_cases.py` (KentOS kodu yok; mpmath 50 basamak; çember bulge'dan kendi formülüyle,
  kesişim ve tarama kendi kodu, hedef alan mpmath'in kök bulucusuyla) dokuz yeni kaydırma ve üç yeni hedef: yaylı
  kenar dışarı/içeri, yay komşulu düz kenar (iki yönde), iki çemberin kesişmesi, **harita koordinatında yaylı bordür
  0,25 m** ve 640 m² hedefi, açık çoklu çizginin yaylı ilk kenarı, deliğin yaylı kenarı, merkezini geçen yay reddi.
  `kentos-geometry-core` `edge_shift` testleri: noktalar 1e-9, bulge'lar 1e-12, alan 1e-9, uzaklık 1e-9.
- İlk koşuda harita koordinatındaki alan 9·10⁻⁹ m² farklı çıktı: başvuru alanı 50 basamaklı köşelerden alıyordu,
  KentOS'unki f64'e yuvarlanmış köşelerden. Kuzeyde 4,4·10⁶ m'de bir köşenin son biti 30 m kirişte alanı ~10⁻⁸ m²
  oynatır; başvuru artık saklanan (yuvarlanmış) köşelerin alanını kesin olarak hesaplar. Tolerans büyütülmedi.
- Etkileşim izi `edge-shift` (masaüstü oynatıcısı): yay kenara tıklama artık kenarı seçer, Esc bırakır.
- Resimler `paralel-kaydir-yay` (yay seçili, imleç 2 m dışarıda) ve `paralel-kaydir-yay-yazildi`. İlk resim iki kusur
  gösterdi: seçili yay düz kiriş olarak vurgulanıyordu ve imlecin uzaklığı kenarın başından ölçüldüğü için 2 m yerine
  5 m yazıyordu. `Picked` artık kenarın ortasını (`mid`, yayda yayın üstünde) ve bulge'ını taşır: uzaklık ortadan
  normal boyunca (yayda yarıçap boyunca) ölçülür, komut kenarı bu noktayla adlandırır, vurgu yayın kendisidir.
  Yeniden çekilen resim 2,000 m ve +42,09 m² gösterir (≈ yay boyu 21,2 m × 2 m).

## 4. Sınırlar

Kalıcı ilişki (“bu ilişkiyi koru”), komşu nesnenin ortak sınırının birlikte kayması, kot/eğim hedefi, çok parçalı
nesne ve önizlemede komşu etkisinin gösterimi bu dilimde yok; web aracı aynı WASM işlemini kullanır ama web ekranı
yenilenmedi (ADR 0228; TODOS §0.2).
