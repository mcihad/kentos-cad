# ADR 0191: Paralel kaydır

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-28`'in ardından `CAD-29`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnek Netcad'in Düzenle › Paralel
  Kaydır'ı ve Alan Düzeltme (Paralel)'idir; adı Netcad'inkidir (AutoCAD'de karşılığı yok).
- **Bağlam belgesi:** TODOS.md `CAD-29`; hukuki alan düzeltmesi `GIS-06` [M] bu aracın işi değildir. ADR 0047 (Ötele, kilitli katman),
  ADR 0149 (alan ve gösterim), ADR 0160 (topolojik düzenleme).

## Bağlam

Bir parselin ya da çoklu çizginin bir kenarını kendine paralel kaydırıp komşu kenarları uzatmak ya da kısaltmak, ya da bir alanı hedef
değere getirmek için kenarı kaydırmak elle yapılıyor (Ötele kenarı kopyalar, köşeleri düzeltmez).

## Karar

### 1. Araç

**Paralel kaydır** (Değiştir'de Ötele'nin yanında; iki proje türünde). Bir alanın ya da çoklu çizginin düz kenarına tıklanır (kilitli
katmandaki ve çok parçalı nesne söylenir). Kenar imleçle canlı kayar: kenar imlecin geçtiği paralel doğruya gelir, imlecin yanında
kaydırma uzaklığı, alanda yeni alan ve farkı. Bir tık o yerde, yazılan değer o uzaklıkta tek adımda (“Paralel kaydır”) yazar; araç sonraki
kenarı bekler. Esc ve Ctrl+Z kenarı bırakır; Esc kenar yokken çıkar.

- Uzaklık işaretlidir: alanda dışarı artı, çoklu çizgide yürüme yönünün sağı artı; projenin biriminde yazılır.
- **Alan** (A): hedef alan (projenin alan biriminde); kaydırma uzaklığı ona göre bulunur (§3).

### 2. Kural

- Kenarın doğrusu uzaklık kadar kendine paralel kayar; kenarın iki köşesi, kaymış doğrunun komşu kenarların doğrularıyla kesişimine
  taşınır (komşular uzar ya da kısalır). Köşelerin sayısı ve sırası değişmez; öbür köşeler yerinde kalır, her köşe kotunu korur
  (taşınan iki köşe de; ADR 0142'nin “yerindeki köşenin kotu” kuralı).
- Açık çoklu çizginin uç kenarında serbest uç kenara dik kayar.
- Komşu kenar yaysa ya da kaymış kenara paralelse (doğrultular arasındaki açının sinüsü 1e-12'den küçük) köşe bulunamaz; söylenir.
  Komşu kenarın ya da kayan kenarın uzunluğu sıfıra ya da yönü tersine düşerse (kenar komşusunu aşarsa) söylenir, yazılmaz.
- Kayan kenar düz olmalıdır (yay kenar söylenir). Halkanın başka yerlerindeki yaylar kalır.
- Çok parçalı nesne söylenir (önce Parçalara ayır); yalnız alan (delikleriyle) ve çoklu çizgi kaydırılır.

### 3. Hedef alan

Alan kaydırma uzaklığının ikinci dereceden işlevidir: kenar L boyunda, iki köşesi kenarın doğrultusunda d başına λₐ ve λ_b kayarsa
A(d) = A₀ + L·d + ½(λ_b − λₐ)·d² (kenarın süpürdüğü yamuk; A₀ alanın bugünkü değeri, Öznitelikler'inki). Hedefe ulaşan kökler
bulunur, |d|'si küçük olan alınır (küçük katsayıda basamak kaybetmeyen biçimiyle); kök yoksa ya da bulunan uzaklıkta §2'nin kuralı
ret verirse söylenir. Hedef projenin alan biriminde yazılır (m², dönüm ya da ha; yerel projede mm² ya da cm²).

### 4. Çekirdek ve komut

`ops::edge_shift`: kenarın kaydırılması (§2) ve hedef alan için uzaklık (§3); web'e işlem tablosundan. Ortak durumlar bağımsız Python
başvurusundan (`scripts/fixtures/edge_shift_cases.py`, kesirlerle ve kök için mpmath). `cad.entities.edit`'in `edgeShift` işlemi, adım
“Paralel kaydır”; nesne yerinde yeni geometrisiyle; köşeler kotlarını yerlerindeki köşeden alır (tutamaç ve Esnet'in kuralı, ADR 0142).

## Kapsam dışı

Hukuki alan düzeltmesi ve hisse dağıtımı (`GIS-06`), birden çok kenarın birlikte kaydırılması, yay kenarın kaydırılması, Topoloji açıkken
komşu nesnelerin ortak kenarlarının birlikte kayması (ADR 0160'ın kuralıyla ileride).

## Uygulama

- Çekirdek: `crates/shared/geometry-core/src/ops/edge_shift.rs` (`shifted`, `for_area`, `pick`, `area_of`; işlemler `edgeShift`,
  `edgeShiftForArea`, `edgeShiftPick`). Köşe, komşusunun doğrultusunda `d / (ê·n)` kayar (`ê` komşunun birim doğrultusu, `n` kenarın
  normali); kaymanın kenar boyundaki payı `λ`'dır ve §3'ün katsayısı ondandır. Komşunun ya da kenarın yönü tersine dönerse (nokta
  çarpımı 0 ya da eksi) ret.
- Sözleşme: `EditOperation::EdgeShift` (`edgeShift`), adı iki platformda “Paralel kaydır”; yerinde kayan köşeler kotlarını yerlerinden
  alır (`by_place`, `byPlace`); katalog, TypeScript ve Python SDK'sı yeniden üretildi; ortak komut durumu
  `fixtures/commands/v1/cad.entities.edit.json`'da.
- Araç: masaüstünde `kentos_interaction::edge_shift`, web'de `tools/edgeShiftTool.ts`; Değiştir ▾'da Ötele'nin yanında, iki proje türünde
  (CBS'nin Düzenle sekmesinde Kenar panelinin ▾'inde: seyrek araç, `rare`; panel geniş pencerede tasarımındaki gibi sığar). Kenar
  seçilince kenet açılır (kenar bir noktadan geçirilebilir); imlecin yanında uzaklık ve alanın yeni değeri farkıyla. Hedef alan projenin
  alan biriminde (`Format::area_to_square_metres`, `Formatter.areaToSquareMetres`).
- Resimler: masaüstü `apps/desktop/src/edge_shift_scenes.rs` (`paralel-kaydir-*`), web `shots.mjs edgeshift`.

## Doğrulama

- Bağımsız başvuru: `python3 scripts/fixtures/edge_shift_cases.py --check` (kesirlerle kesin; irrasyonel boy ve kök mpmath ile 50
  basamak; KentOS kodu olmadan): kare, saat yönündeki kare, yamuğun içeri kayan kenarı, deliğin kenarı, çoklu çizginin orta ve ilk kenarı
  (serbest uç), harita koordinatları, irrasyonel boylu eğik kenar, halkadaki başka yay, retler (yay kenar, yay komşu, paralel komşu, komşuyu
  aşan, olmayan kenar ve halka, çok parçalı, çizgi); hedef alanlar (doğrusal ve ikinci dereceden, ulaşılamayan, yalnız komşuyu aşarak
  ulaşılan, eğik kenar, yaylı halka, çoklu çizgi). Çekirdek doğal olarak (`tests/all/edge_shift.rs`) ve WASM'dan
  (`tools/edgeShift.wasm.test.ts`) aynı dosyayı geçer: köşeler ve alan 10⁻⁹ içinde.
- Ortak iz `fixtures/interaction/v1/edge-shift.json` iki platformda: çizgi, kilitli parsel ve yay kenar söylenir; yazılan uzaklık, Ctrl+Z'nin
  önce kenarı bırakması ve geri alması, Alan'ın yanlış ve ulaşılamayan değerleri ve 180 m², çoklu çizgide imlecin yeri, komşuyu aşan
  uzaklık.
