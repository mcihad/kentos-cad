# ADR 0259: NURBS ve eğriler

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-15`, `CAD-DATA-16`, `CAD-DATA-17`. Sahibin
  sözleri: “Nurbs ve eğrileri”, “Başla o zaman”; doğrulama OpenCASCADE'le (“opencascade … kullanarak doğrula”); NVIDIA'nın usd-brep'i
  (SMLib, Apache 2.0) algoritma başvurusu olarak okundu, kodu alınmadı.
- **Bağlam belgesi:** ADR 0057 (Eğri: Catmull–Rom), 0149 (ölçü doğruluğu, `curve_outline`), 0250 ve 0254 (anlamsal Buda, Kır),
  0253 (komut protokolü), 0025 ve `docs/specs/kcad-v2.md` (KCAD v2), 0257 (DXF içe almanın komutu).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- **CAD-DATA-15:** daire, yay ve elips kaynakta analitikti (merkez, yarıçap, açılar; eksenler, oran, parametreler); denetlenmesi gereken,
  ölçmenin, kenetin ve budamanın çizilen kırık çizgiye düşmediğiydi.
- **CAD-DATA-16:** genel NURBS yoktu. DXF'in denetim noktalı SPLINE'ı 1 mm içinde çoklu çizgiye çevriliyordu (ADR 0057); derece,
  düğümler ve ağırlıklar kayboluyordu, tam bir çember yayı bile kırık çizgi oluyordu.
- **CAD-DATA-17:** KentOS'un kendi eğrisi (`spline`, geçiş noktalarından merkezcil Catmull–Rom) eski çizimlerde vardır; biçimi değişmemeli.

## Karar

### 1. Daire, yay ve elips analitik kalır (CAD-DATA-15)

Denetim sonucu, bugünkü çekirdek zaten eğrinin kendisiyle çalışıyor; değişiklik gerekmedi, bağımsız doğrulama eklendi:

- **Ölçü:** daire ve yayın uzunluğu kapalı biçimde, elipsinki uyarlamalı Gauss–Legendre karelemesiyle (`ellipse_length`, ADR 0149 §3).
- **Kenet:** elipste yakın, dik ve teğet eğrinin kendisinde (`closest_param`, `ellipse_tangent_points`); kesişimler kirişlerde bulunup
  eğriye taşınır (`store::snap`).
- **Buda, Kır, Uzat:** elips parametre uzayında kesilir, parçalar elips yayı kalır (`ops::curve_cuts`); daire ve yay kenar olarak kesin.
- **Doğrulama:** `scripts/fixtures/curves_occt.py --check`, ADR 0149'un 50 basamaklı uzunluklarını (daire, yay, elips, elips yayı ve
  Catmull–Rom'un 60 durumu) OpenCASCADE'in kendi eğrileri ve `GCPnts_AbscissaPoint`'iyle yeniden ölçer; hepsi durumların sınırı içinde.

### 2. Genel NURBS nesnesi (CAD-DATA-16)

Yeni nesne türü `nurbs` (`NurbsEntity`, çekirdekte `Shape::Nurbs`):

- **Kaynak:** `degree` (1–11), `ctrl` (en az derece + 1), `knots` (denetim noktası + derece + 1 tane, azalmayan, aralığı boş olmayan,
  iç düğüm en çok derece kez), `weights` (isteğe bağlı, her biri sıfırdan büyük). Sayılar verildiği gibi saklanır, yuvarlanmaz.
- **Yanında tutulanlar:** `closed` ve `periodic` (AutoCAD'in 70'inin 1 ve 2 bitleri, yalnız true yazılır), `fit` (çizilirken geçtiği
  noktalar), `tangents` (geçiş noktalarının uç teğetleri, olmayan yerde null), `fitTolerance`. Kaynak denetim noktalarıdır; ötekiler
  bilgidir.
- **Kural:** `nurbs_problem` tek yerdedir; KCAD okuyucusu, komutlar, sunucu ve DXF yazıcısı onu kullanır.

Çekirdek (`geom::nurbs`, Piegl ve Tiller'ın The NURBS Book'u): türdeş koordinatlarda de Boor, A2.3 ve A4.2 ile türevler, A5.1 ile düğüm
ekleme; uzunluk aralık aralık uyarlamalı Gauss–Legendre, en yakın nokta örneklenmiş başlangıçtan Newton, alan Green'le. Nokta ve
türevler aralığın ilk denetim noktasına göre yerel koordinatta hesaplanır: harita koordinatlarında (milyonlarca metre) türevler
basamaklarını korur, uzunluğun karelemesi 0,1 m civarındaki çizimdekiyle aynı sürede biter.

İşlemler eğrinin kendisiyle: kenet (uçlar, uzunluğa göre orta, yakın, dik; kesişimler kirişte bulunup eğri ile sınır arasında
yaklaştırılarak), Buda ve Kır (düğüm eklemeyle bölünür, her parça aynı eğridir; kapalı eğri dikişinden birleştirilerek), Yönü çevir,
taşıma, döndürme, ölçekleme, aynalama (denetim noktaları dönüşür, düğümler ve ağırlıklar kalır; afin dönüşümde eğri tam olarak
dönüşür), tutamaçlar (denetim noktaları; tutamaçla çekilen eğrinin geçiş noktaları düşer). Çizim, seçme ve kapalı eğrinin alanı her
kirişi 0,1 mm içinde kalan noktalardan; Patlat 0,1 mm içinde çoklu çizgi verir, kapalıysa kapalı çizgi.

### 3. Komut: `cad.nurbs.create`

Anlamsal komut; girdisi kullanıcının verdiğidir, eğriyi komut kurar (`geom::nurbs_build`):

- **`by.kind: control`** (Denetim noktalı): derece, denetim noktaları, isteğe bağlı ağırlıklar ve düğümler. Düğüm verilmezse uçlara
  sıkıştırılmış eşit aralıklı düğümler (0 derece + 1 kez, 1, …, n − p, n − p derece + 1 kez). `closed`: ilk derece kadar denetim noktası
  (ve ağırlık) sona sarılır, düğümler 0, 1, …, n + 2p; eğri kapalı ve periyodik saklanır. Kapalı eğriye düğüm verilmez; en az üç ve
  dereceden az olmayan denetim noktası.
- **`by.kind: fit`** (Geçiş noktalı): noktalar ve derece (yoksa 3; nokta sayısının bir eksiğine iner). Kiriş uzunluğuyla parametreler;
  açık eğride ortalanmış düğümler (Piegl ve Tiller 9.8) ve bant içinde pivotsuz çözülen sistem (B-spline toplama matrisi tamamen pozitiftir);
  kapalı eğride periyodik düğümler (tek derecede parametrelerde, çiftte ortalarında, dönem 1) ve kısmi pivotlu çözüm, en çok 1000 nokta.
  Kapalı eğrinin ilkiyle aynı yerdeki son noktası ilk noktadır. Noktalar eğrinin `fit`'i olarak saklanır.
- **Denetim sırası:** sonlu sayılar (denetim noktaları, ağırlıklar, düğümler; geçiş noktaları) `not_finite`; ardışık aynı yerde iki geçiş
  noktası `coincident_points`; derece, sayılar ve düğüm kuralı `invalid_nurbs`; kalınlık; sürüm; katman.
- **Belirlenimcilik:** kurucunun her adımı düz IEEE işlemleriyle sabit sırada (hypot yok, kaynaşık çarpma yok); bağımsız başvuru aynı
  sırayı Python'da yineler ve sonucu bit bit bulur.
- `cad.entities.create` ve `.edit` NURBS geometrisini alır, kuralı dışındakini `invalid_nurbs` ile reddeder.

### 4. Dosyalar

- **KCAD v2 şema 47** (yalnız NURBS'lü çizimde, blok tanımınınki de): `nurbs` türü, alanları §2'deki gibi; tipli sütunlarda tür 19;
  depo paketinde tür 23. `FORMATS_VERSION` 60. Bağımsız Python okuyucu ve yazıcısı, örnek `nurbs.kcad` ve on bozuk dosya.
- **DXF:** denetim noktalı SPLINE tam olarak NURBS gelir (bloğun dönüşümüyle denetim noktaları, geçiş noktaları ve teğetler dönüşür;
  B-spline afin dönüşümle değişmez); derecesi 11'i aşan ya da kuralı dışı olan eskisi gibi notla çoklu çizgi. NURBS SPLINE olarak gider:
  70 (düzlemsel, kapalı, periyodik, rasyonel), 71–74, 40, 10 ve 41, 11, 12/13, 44.
- **GeoJSON, KML, GPX:** 0,1 mm içinde örneklenir, rapor söyler. **Sunucu:** kaynağı tanımdır (`cad_definition`); izdüşümü 1 mm
  içinde çizgi, kapalıysa alan halkası.

### 5. Catmull–Rom korunur (CAD-DATA-17)

KentOS'un `spline`'ı ayrı tür olarak kalır: verisi, çizimi, ölçüsü ve DXF'e gidişi değişmez (eski çizimin baytları aynı;
`a_drawing_without_elevations_is_written_byte_for_byte_as_before` NURBS'süz nesnelerle aynı izi korur). Göç yapılmaz: Catmull–Rom
kübik Bézier'lerden oluşur, NURBS'e kayıpsız çevrilebilir ama çevirmek kullanıcının istediği bir düzenlemedir, sessiz göç değil.

### 6. Masaüstü

- **NURBS eğrisi** aracı (`tool.nurbs`, CAD › Giriş › Çizim): Yöntem (Y) geçiş ya da denetim noktaları, Derece (D) 1–11, Kapat (K),
  Geri (G), Bitir (Enter); yöntem ve derece oturumda kalır. Önizleme komutun planıdır: eğri düz; geçiş yönteminde verilen noktalar,
  denetim yönteminde denetim çokgeni kesikli. Yazma komutla, adı “Ekle”.
- Öznitelikler'de Derece, Denetim noktası, Geçiş noktası, Ağırlıklı, Kapalı, Uzunluk ve kapalıysa Alan. Seçim süzgecinin Eğri'si NURBS'ü
  de tutar. Nesne türünün adı “NURBS eğrisi”.

## Sonuçlar

- AutoCAD'den gelen eğriler artık tam: çember yayı NURBS'ü çember yayıdır, kapalı periyodik eğri dikişsizdir.
- Python (`kentos.cad.nurbs.create`) ve ajan (MCP) aracın kurduğu eğrinin aynısını kurar.
- Sonraya kalanlar: NURBS'te Ötele, Uzat, Köşe yuvarla ve Böl'ün eğri sonucu (bugün kirişler üstünden ya da reddedilir; CAD-EDIT),
  eğriyi geçiş noktalarından yeniden kurma ve uç teğetleriyle interpolasyon, eğrinin düzenlenmesi (denetim noktası ekleme, derece
  yükseltme). Web'in türü ve aracı TODOS.md §0.2'de.

## Doğrulama

- Çekirdek: `geom::nurbs`, `geom::nurbs_build` (uzak koordinatta uzunluk dahil) ve `ops::nurbs_cuts` testleri.
- Komut: `uv run --with mpmath python3 scripts/fixtures/nurbs_cases.py --check`: 27 durum; beklenen eğriler IEEE ikizinden bit bit,
  50 basamaklı kesin çözümle ve OpenCASCADE'le (`tools/occt/occt_ref.cpp`; eğri geçiş noktalarından geçer, sıkıştırılmış eğri uçlarında
  başlar ve biter, kapalı eğri dikişte derecesinin sürekliliğiyle) denetlenmiş. Masaüstünde `fixtures::every_command_case_matches_the_desktop_handlers`,
  Python'da `test_command_cases.py` ve `test_nurbs.py`.
- Eğriler: `python3 scripts/fixtures/curves_occt.py --check` (60 uzunluk).
- KCAD: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `kentos-kcad`'in `nurbs.rs`'i; bağımsız okuyucu `tools/kcad/kcad.py`.
- DXF: `dxf_write`'ın gidiş-dönüşü (rasyonel çeyrek daire, teğetler ve tolerans, kapalı periyodik kübik), `dxf.rs`'in SPLINE'ı.
- Araç: `kentos-interaction`'ın `nurbs.rs`'i (araç ile komut aynı eğriyi yazar, önizleme plandır). Resimler
  `KENTOS_SHOTS_ONLY=nurbs-cizim,nurbs-oznitelikler,nurbs-gecis,nurbs-denetim,nurbs-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-nurbs-*`).
