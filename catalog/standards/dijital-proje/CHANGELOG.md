# Değişiklikler

## 0.10.0 — 17 Kasım 2026

- Gelen paketin incelemesi (docs/adr/8104): `inceleme.denetle(olgular)` idarenin inceleyeni için paketi müellifin çizimi
  olmadan, KentOS'un dosyalardan ölçtüğü olgularla katman katman denetler: paket (liste, dosyaların baytı ve özeti,
  listede olmayan dosya, model, pafta ve form), pafta (PDF/A-2, taranmış sayfa, paftanın kaynak revizyonu), model
  (okunuyor, şema, formun dosyadan cevapladığı sorular; bulgular nesneleriyle), konum (modelin sistemi idarenin
  dilimi mi), form (müellifin cevabı ölçülenle aynı mı, form bu modelin mi). Formun 21 satırı yan yana.

## 0.9.0 — 17 Kasım 2026

- Cevabın eskimesi (docs/adr/8077 §7): müellifin cevabı dayandığı kanıtı (`basis`, o zamanki KentOS notu) söylüyorsa ve
  sorunun notu artık başkaysa `form.cevapli` cevabı okumaz (`eski`), soru yeniden incelemeye kalır; `bim.denetle`
  `digital_form_answer_stale` ile yayını durdurur. `bim.denetle`'nin satırları KentOS'un kendi notunu da taşır
  (`gerekce`): cevabın dayanağı odur.

## 0.8.0 — 17 Kasım 2026

- Müellifin cevapları çizimde (docs/adr/8077): `form.cevapli(satirlar, cevaplar)` incelemeye kalan soruların resmî
  cevabını ve notunu müellifin cevabından verir (`muellif`); `form.tablo(ifc, cevaplar)` basılan formda onları yazar;
  `bim.denetle` aynı birleştirmeyi kullanır. KentOS'un dosyadan cevapladığı soruya verilen cevap okunmaz.

## 0.7.1 — 17 Kasım 2026

- 9. soru: modelin gerçek kuzeyi (EK-7 Tablo 7.8 `TrueNorth`) yoksa onarım, dışa aktarımın nedenini söyler
  (`export.northWhy`: projenin koordinat sistemi ya da kuzey oku yok, oklar farklı yönde; docs/adr/8076 K58). KentOS
  gerçek kuzeyi artık yazıyor; "henüz yazmıyor" sözü bağlamın öbür alanlarında kaldı.

## 0.7.0 — 16 Kasım 2026

- Yeni kural: model kalite kontrol formu yazılmış dosyadan (`form`, docs/adr/8074; Md. 15, EK-9 Tablo 9.1). 21 sorunun
  cevabı KentOS'un yazdığı IFC'nin olgularından; resmî cevap ile kanıt (geçti, kaldı, inceleme) ayrı. Eski Rust
  denetiminin bulguları (`digital_project_info_missing`, `digital_project_code_missing`, `digital_space_unnamed`,
  `digital_system_wrong`, `digital_no_spaces`, `digital_model_loss`) aynı kodlarla buradadır; yenileri her sorunun kendi
  kodudur. `form.tablo` çizimdeki formun satırlarını verir. `mimari.denetle` ikinci girdi olarak dosyanın olgularını
  alır; verilince EK-9 dosyadan cevaplanır, `kalite` ve `zorunlu` yinelenmez. 9. soru modelin bağlamının kayıttaki
  zorunlu alanlarını da sayar (EK-7 Tablo 7.8). Formun cevapları başka bir okuyucunun (IfcOpenShell) olgularında ve
  profilin IDS'inin bağımsız denetleyicisinde (IfcTester) aynı çıkar.
- Yeni kural: BIM ruhsat paketinin kararı (`bim`, docs/adr/8075; Md. 9/1, 15; e-PYS Md. 4/2, 5/1–2): paftaların ve
  paketin kuralı (`ruhsat`), formun kalan satırları ve müellifin cevabı birlikte; cevapsız ya da Hayır cevaplı inceleme
  sorusu da yayını durdurur. Beş sonuç (okunuyor, şema, Türkiye profili, model ile pafta uyumu, idare kabulü) ayrı.
- `paket`: EK-2'nin ad denetimi uzantıyı alan saymaz; modelin `.ifc` adı da aynı şablonla denetlenir.

## 0.6.0 — 16 Kasım 2026

- Yeni kural: modelin zorunlu alanları (`zorunlu`, docs/adr/8068; Md. 12/1, 13; EK-6, EK-7; EK-9 satır 9). KentOS'un
  ölçtüğü `cad.modelGaps` (yazılacak modelde sınıfının her nesnesinde değeri olmayan zorunlu satır) her satır için bir
  hata, nesneleriyle; onarım alanına göre (yangın dayanım sınıfı ve acil çıkış Öznitelikler'de, malzemenin dayanımı
  Malzemeler'de, binanın yangın koruma sınıfı Mimari ayarları'nda). `mimari.denetle` dört kuralı birlikte verir.

## 0.5.0 — 16 Kasım 2026

- Yeni kural: EK-9'un modelde ölçülen soruları (`kalite`, docs/adr/8067). Satır 13: aynı kattaki iki mahal aynı zemini
  kaplıyorsa hata, ikisi de nesnesiyle; ortak alan KentOS'un ölçtüğü (`cad.roomOverlaps`), 0,01 m²'nin altı çizimin
  yuvarlaması sayılır. `mimari.denetle` üç kuralı birlikte verir; masaüstünün 3B görünümü bulguları kırmızıya boyar.

## 0.4.0 — 16 Kasım 2026

- Yeni kural: 2B ruhsat paketi (`paket`, docs/adr/8066; Md. 4/2, 6, 8). PDF/A seçilmemiş, bir dosyada birden çok pafta,
  A4 ve katları dışında kâğıt (K39), EK-2 Tablo 2.1'e uymayan dosya adı; hepsi hata. `ruhsat.denetle` ruhsat yayınının
  kapısı için `turetim` ile birlikte verir.

## 0.3.0 — 16 Kasım 2026

- Yeni kural: paftalar modelden (`turetim`, docs/adr/8065; Md. 9/1, EK-9 satır 21). Eski kesit ve görünüş (Yeniden çiz tek
  adım), işareti silinmiş kesit, model katmanında serbest çizim, kopmuş etiket ve ölçü; hepsi hata: ruhsat yayını onlar
  kapanmadan yapılmaz. `mimari.denetle` iki kuralı birlikte verir.

## 0.2.0 — 16 Kasım 2026

- Yeni kural: EK-3 ve EK-4 (`ek34`, docs/adr/8064). Mahal numarasının kalıbı, kat kodu, sırası ve saat yönü; mahal adının
  Tablo 3.2'deki yazımı; katman adı (4.11, Tablo 4.1–4.3), "0" katmanı, tarama katmanı ("-T"), emsal katmanı, aks çizgisi
  (noktalı kesikli), ölçü yazısının yazı tipi (4.21.2) ve blok adı (Tablo 4.4). Olgular KentOS'un `cad` bölümünden; kural
  verisi `data/ek34-2026.json` (KentOS'un kural kataloğundan `scripts/mevzuat/ek34_veri.py` yazar). Düzeltmeler tek adım:
  mahalleri saat yönünde yeniden numaralamak, mahalin adını ya da numarasını düzeltmek, katmanı yeniden adlandırmak, aks
  katmanının çizgi tipini noktalı kesikli yapmak.

## 0.1.0 — 15 Kasım 2026

- İlk kural: dijital teslim kapsamı (`kapsam`, docs/adr/8063). Bu projede yönetmeliğin kapsamı, 2B çizim ve belgelerin
  PDF/A'sı (Md. 4/2–3), IFC modeli (Md. 4/5), modelden üretilen çizimlerin PDF/A'sı (Md. 9/1) ve e-PYS'de teslim
  (e-PYS Md. 4) ne zaman, hangi maddeyle başlar. Tarih ve eşikler `data/kapsam-2026.json`'da (KentOS'un kural
  kataloğunun kopyası); eksik girdi sonucu değiştirebiliyorsa "teyit gerekli".
