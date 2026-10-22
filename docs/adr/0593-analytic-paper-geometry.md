# ADR 0593: Analitik pafta geometrisi ve açık kaynak bilgisi

Tarih: 22 Ekim 2026 · CAD-SHEET-08

Mevcut pafta kitabı; yazı, tablo, antet, çizgi, şekil, resim ve bunların çerçeve/döndürme/grup/yerleşim/geri alma davranışları için otorite olarak kalır. Model uzayının nesneleri pafta katmanına kopyalanmaz. Paftadaki şekil için ikinci bir CAD belge deposu kurmak yerine `ShapeKind::Cad(CadShape)` eklenir. Başka bir öğenin sahipliğine bağlanan canlı geometri değildir.

- `reference`: özgün seçim çerçevesi; geometri yerel kâğıt mikrometresinde `f64` ile saklanır. Öğe çerçevesinin genişliği/boyu ve dönüşü bu kaynağa afin dönüşüm uygular. 1 mm seçim çerçevesi, örneğin 0,2 mm çaplı daireyi büyütmez.
- Kaynak türler: daire, açık yay, tam/kısmi elips, ikinci/üçüncü derece Bézier, rasyonel/genel NURBS, yaylı ve değişken genişlikli çoklu çizgi. Çoklu yollar, delikler ve parçalar aynı kaynakta tutulur. NURBS derece, denetim noktaları, düğüm/ağırlıklar, kapalı/periyodik bayrakları, geçiş noktaları/teğetleri/toleransı taşır.
- `Item.cadOrigin`: kaynak dosya adı ve varsa SHA-256, kaynak pafta adı, sıfır tabanlı kayıt sırası, tür ve kaynak katman kimliği. Okuyucunun vermediği handle uydurulmaz. Kaynak bir kopyanın kökenidir; model nesnesiyle canlı düzenleme bağı anlamına gelmez. Öznitelikler paneli kökeni gösterir.
- Ekran/PDF/SVG aynı geçici yol üreticisini kullanır. Bézier PDF/SVG'de kübik yol olarak çıkar. Daire/yay/elipsin kübik çizim yaklaşımı 0,025 µm radyal hata bütçesinde; genel NURBS'in örnekleme hedefi 0,25 µm kâğıt kiriş toleransıdır. Kaynak sayılar değişmez; son ekran/PDF yol noktaları mevcut 1 µm kâğıt ızgarasına yuvarlanır. Değişken genişlikli çizginin görsel bandı mevcut ortak CAD bandı kuralını kullanır; bu banda NURBS toleransı atfedilmez.
- İşlem başına 65536 yol adımı sınırı vardır. Aşımda kısmi eğri kabul edilmez. Geniş çizginin bandı kurulmadan önce iş miktarı kontrol edilir. Uç büyüklükler ve geçersiz düğüm/ağırlık/çerçeve sayıları reddedilir.
- DXF/DWG kâğıt köprüsü daire, yay, elips ve NURBS'i analitik geri yazar; Bézier clamped NURBS olarak gider. Afin dönüşen konik mevcut ortak CAD warp hesabından geçer. Orantısız büyütülen yaylı çizginin kenarları analitik konik/çizgilere ayrılır. Orantısız büyütülen değişken genişlikli çizgi DXF'te güvenle temsil edilemiyorsa atlanır ve kayıp raporuna girer; KCAD kaynağı aynen kalır.
- Mevcut yazı/tablo/antet/ok öğeleri pafta açıklamaları için korunur. Kaynaktaki gelişmiş CAD açıklama semantiğinin tamamı yerel düz metin değildir: özel hizalama, genişletme/eğiklik, MTEXT karakter biçimleri ve maske dönüşümleri mevcut açık raporunda kalır. Bu karar tam foreign DIMENSION/MLEADER/proxy düzenleme desteği iddiası taşımaz.
- Küçük yazı alt sınırı 1 µm olur; font boyu seçim kutusuna göre büyütülmez. Döndürülmüş yazının baseline hesabı, minimum seçim kutusu uygulandıktan sonra yapılır.
- Arayüzün Şekil menüsünde Yay ve Bézier bulunur; çerçeve, dönüş, çizgi/dolgu, grup ve undo mevcut ortak işlemlerdir. Python/MCP aynı ayrıntılı `cad.sheets.edit` şemasıyla bütün kaynak tanımını düzenler; tam kitap sorgusu ve KCAD kaydı bu tanımı taşır.

## Kanıt

- `scripts/fixtures/paper_cad_cases.py`: KentOS çağırmadan 24 bağımsız küçük/büyük daire, yay, döndürülmüş tam/kısmi elips, rasyonel çeyrek daire ve değişken genişlikli yay tanımı.
- Native köprü testleri: analitik tür/ölçü, NURBS kaynak bilgileri, affine daire→elips, orantısız geniş çizgide açık aktarım sınırı.
- Shared sayısal testler: 24 yarıçap/büyütme birleşiminde kübik yaklaşımın bağımsız daire denklemiyle hata ölçümü; rasyonel NURBS'te daire denklemi; sınırlı örneklemede bütün sonuç ya da hata.
- Python: 24 örneği gerçek komut host'unda oluşturma, kaynak kimliği, grup undo/redo, KCAD baytlarından yeniden açma.
- `sheet08_analytic_paper_screens`: açık/koyu 1440×900 ve 18 px yazıyla 1100×650 gerçek GPU ekranları; paftanın kendi bölgesindeki eğri pikselleri denetlenir. Aynı modelden PDF üretildi ve Poppler PNG'si açılıp incelendi.
- Web arayüzü ADR 0228 gereği erteli; ortak sözleşme/SDK üretilir, WASM derlenebilirliği korunur.

Doğrulama sonucu: 714 komut sözleşmesi testi; 130 native entegrasyon + native analitik köprü testleri; 375 sheet birim + 52 entegrasyon, 44 UI birim + 31 tasarımcı + 7 kitaplık; 216 Python testi geçti. NURBS seçili sayısal testleri, clippy, sheet WASM ve 47 crate / 61 hedef bağımlılık denetimi geçti. Downloads içindeki üç DWG, sırasıyla 282/282/9730 model nesnesi ve 2/2/4 paftayla okundu; kaynak dosyalar değiştirilmedi.
