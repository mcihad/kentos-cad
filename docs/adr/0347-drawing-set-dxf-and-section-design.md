# ADR 0347: Pafta seti, mimari nesnelerin DXF'i ve otomatik kesitin tasarımı

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md `CAD-ARCH-43`, `CAD-ARCH-44`, `CAD-ARCH-45`; 7. grubun ikinci parçası (7b), CAD-ARCH'ın sonu.
- İlgili: ADR 0251 (pafta kitabı), ADR 0345 (görünüm işaretleri, Başvurular), ADR 0346 (mimari şablonu), ADR 0327 §6 (ölçü ve
  kılavuzun DXF'e görünüşüyle gitmesi), ADR 0339–0344 (mimari nesneler ve DXF'e düz parçalar).

## Bağlam

Paftalar projenin kitabında; görünüm işaretleri onlara bakıyor. Bir kat planı değişince hangi paftaların yeniden basılacağını
bulmanın yolu yoktu. Mimari nesneler DXF'e yalnız düz parçalarıyla gidiyordu: başka program onları görüyor ama KentOS geri
okuyunca duvar, açıklık, mahal yerine çizgiler buluyordu. Plan üçüncü olarak otomatik kesit ve görünüşün modelinin ayrı
tasarlanmasını, bu teslimde elle 2B üretimin kalmasını ister.

## Karar

### 1. Pafta seti ve etkilenen paftalar (`CAD-ARCH-43`)

- Pafta seti projenin pafta kitabıdır: paftalar kitabın sırasıyla, adlarının numarasıyla serilere ayrılır (`A-101`'in serisi
  `A-1`: planlar; şablonun düzeni A-1 planlar, A-2 görünüşler, A-3 kesitler, A-5 detaylar; numarasız pafta “Diğer”).
- Salt okunur `cad.sheets.set` (“Pafta seti”): her pafta (kimlik, ad, seri), her harita çerçevesi (kimlik, ad, ölçek, gösterdiği
  dünya kutusu: çerçevenin kâğıttaki boyu × ölçek, merkezinden, dönüşüyle) ve çerçeveye bakan görünüm işaretleri; `uids` verilince
  etkilenen paftalar: nesnelerden birinin kutusu çerçevenin kutusuyla kesişen (`shows`) ya da nesnelerden bir görünüm işaretinin
  hedefi olan (`marked`) çerçeveler. Sabit görünümlü (`fixed`) olmayan çerçeve kutusuyla değerlendirilmez (`unknown`).
- Masaüstünde Mimari › Görünüm'de Pafta seti penceresi: serilere göre paftalar, çerçeveler ve ölçekleri; seçili nesnelerin
  etkilediği paftalar işaretli; Paftaya git.

### 2. Mimari nesnelerin DXF'i (`CAD-ARCH-44`)

- Duvar, açıklık, mahal, aks, kolon, merdiven, rampa, etiket ve görünüm işareti DXF'e görünüşüyle gider: düz parçaları (çizgiler,
  çoklu çizgiler, taramalar, yazılar) adsız bir blokta (`*U`, tanımın nesneleri gibi: kendi rengi olmayan BYBLOCK), bloğun
  başlangıçta duran INSERT'i nesnenin kendisini ve çizimdeki kalıcı kimliğini (`uid`) KENTOS verisinde taşır (ölçünün ve
  kılavuzun ADR 0327 §6'daki yolu, `writer::plain_walls`'ın `Wholes`'u, `Writer::arch_whole`). KENTOS verisi kapalıysa
  parçalar düz yazılır (bugünkü gibi); AutoCAD'in 16 KB sınırını aşan nesnenin bloğu verisiz yazılır, KentOS blok olarak okur;
  ikisi de raporda söylenir.
- Okuma: INSERT'in KENTOS verisindeki nesne kendi görünüşü, öznitelikleri ve etiketiyle geri gelir (taşınmamış, dönmemiş,
  ölçeklenmemiş INSERT'te; başkası değiştirmişse bloğun parçaları alınır). Nesneler arası bağlar (açıklığın duvarı, kolonun
  aksları, etiketin nesnesi) dosyadaki eski kimliklerle yazılır; okuyucu bütün nesneler okununca onları dosyadaki nesnelerin
  yerlerinin yer tutucularına çevirir (`Emitter::tie_arch`, ADR 0333'ün `import_placeholder`'ı), içe aktarma aynı adımda yeni
  kimlikleri yazar (`tie_placeholders`), dosyada olmayan nesneye bağ hiçbir nesneyi adlandırmaz ve nesnenin kendi kopukluk
  kuralıyla kalır (raporda sayılır).
- Doğrulama ayrıdır: görünüş başka bir okuyucuyla (KentOS kodu olmadan, DXF grup kodlarından) — her mimari nesnenin INSERT'i,
  bloğunun çizilen parçaları ve kutusu; anlam ayrıca — KENTOS verisinin nesnesi kaynağın nesnesiyle aynı; ve KentOS'un geri
  okuması nesneleri aynı alanlarla verir.

### 3. Otomatik kesit ve görünüş: tasarım (`CAD-ARCH-45`)

Bu teslimde kesit ve görünüş çizimleri elle (2B) üretilir; görünüm işaretleri onlara bağlanır (ADR 0345). Otomatik üretimin
gerektirdiği model ve doğruluk sınırları:

- **Yükseklik modeli:** katın kotu ve yüksekliği (ADR 0339 §3) yeter değildir; duvarın tabanı ve tepesi kata göre (üst kata
  bağlı, sabit), açıklığın denizlik ve lentosu (var), döşeme ve tavan kalınlığı (yok: katın `slab`'ı), çatı ve eğimli tavan (yok).
- **Malzeme modeli:** duvar bileşenleri ve taramaları (var); döşeme, tavan, çatı katmanlarının bileşenleri (yok); kesitte kesilen
  ile görünen ayrımı için her bileşenin kesit taraması ve görünüş çizgisi (yok).
- **Katman ve görünürlük:** kesit düzlemi ve bakış derinliği (işaretin `depth`'i), arkada kalanın gizli çizgisi; kesilen nesne kalın,
  görünen ince (ADR 0312'nin kâğıt kalınlıkları).
- **Doğruluk sınırları:** kesit, düzlem ile nesnelerin prizmatik modellerinin kesişimidir; eğrisel duvar kirişlerle değil yaylarla
  kesilir; merdiven basamakları rıht sayısından; 1 mm'den küçük boşluklar birleşir. Prizmatik olmayan (eğik duvar, kubbe) nesne
  kesitte söylenir, çizilmez.
- **İş sırası:** önce katın döşeme ve tavanı, sonra duvar tepesinin kurala bağlanması, sonra kesit düzleminin çekirdekte kesişimi
  (`geom::section`) ve 2B çıktının görünüm işaretinin hedefine yazılması; her adım ayrı ADR'dir.

## Sonuçlar

- Paftaların etkilenmesi kutu kesişimiyle bulunur: çerçevenin kırpma çokgeni ya da katman geçersiz kılmaları (ADR 0314) sayılmaz;
  “belki etkilenmiş” bir pafta fazladan listelenebilir, eksik listelenmez.
- DXF'te mimari nesneleri başka programlar blok olarak görür; patlatırlarsa KentOS düz parçaları alır.

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/sheet_set_cases.py --check` (cad.sheets.set; KentOS kodu olmadan, 25 durum:
  seriler, dönük, dolgulu ve yazı bantlı çerçevelerin zemini, kapalı kutular, işaretler, bilinmeyen zemin, retler).
- DXF: `python3 scripts/fixtures/arch_dxf_cases.py --check` (görünüş ve KENTOS verisi ayrı, KentOS kodu olmadan; dosyaları
  `KENTOS_WRITE_DXF=1 cargo test -p kentos-headless --test headless arch_dxf_goes_round` yazar) ve aynı test (dokuz türün geri
  okunması, bağların yeni kimliklerle).
- Resimler: `KENTOS_SHOTS_ONLY=pafta-seti,pafta-seti-etkilenen,dxf-mimari cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
