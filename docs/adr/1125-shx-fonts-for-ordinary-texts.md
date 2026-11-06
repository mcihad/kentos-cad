# ADR 1125: Yazı nesnesinde gömülü SHX fontu; CAD aktarımının ve paketin SHX yazı fontları

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`'in 6 Kasım açıkları “çizgi tipi
  dışındaki SHX yazıları”, “DXF/DWG aktarımında SHX yazı fontlarının dosya yanına yazılması”, “Unicode → Big Font kod
  sayfası”; ADR 1120 §6'nın ilk üç sınırı.
- **Bağlam belgesi:** ADR 0930 (gömülü SHX kaynağı, `cad_shape_files`), ADR 1120 (unifont/Big Font okuyucusu, çizgi
  tipi yazısı), ADR 0145 (yazının noktası ve kutusu), ADR 0182 (çok satırlı yazı), ADR 0183 (stil ve yüz), ADR 0324 §5
  (`cad.fonts.report`), ADR 1111 (paket raporu).

## 1. Araştırma

- Autodesk, [About Text Font Descriptions](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-Customization/files/GUID-9BBE5B28-DF02-4EC5-863A-BA04AB6F5EF1.htm):
  yazı fontu bir şekil dosyasıdır, şekil numarası karakter kodudur, şekil 0 fontun tanımıdır (`above,below,modes`);
  karakter, programının sonundaki kalem-kaldırık hareketle bir sonrakinin başlangıcına ilerler. Yazı yüksekliği
  büyük harf yüksekliğidir (`above` birimi). Yazı, fontun **açık çizgileridir**: doldurulmaz.
- Autodesk [Plot Style Table Editor / Assign Plotted Line End and Join Styles](https://help.autodesk.com/cloudhelp/2023/ENU/DWGTrueView/files/GUID-1778B6B5-3356-4CA8-912A-794EC0C56D71.htm):
  SHX yazısı en iyi yuvarlak uç ve yuvarlak birleşimle basılır; çizgileri nesnenin kalınlığıyla basılır.
- Autodesk [About Defining a Big Font](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-Customization/files/GUID-DE0CCC57-AC55-4CDC-887E-730FC90364E4.htm):
  Big Font dosyası kaçış (ilk bayt) aralıklarını ve iki baytlık kodları tanımlar; **kod sayfası** alanı yoktur. DXF'in
  `\M+cXXXX` yazımı kod sayfasını (c) yazının kendisinde taşır (ADR 1120 §1). KentOS projesi DXF'in `$DWGCODEPAGE`'ini
  saklamaz (`formats/dxf/mod.rs` yalnız okuma kodlamasını seçer), içe alma `\M+` dizgisini olduğu gibi bırakır.
- Autodesk [Special Unicode Characters](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-7B3CC5EF-FFAC-403D-9F89-9D0921DDA8A4.htm)
  ve [Control Codes and Special Characters](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-968CBC1D-BA99-4519-ABDD-88419EB2BF92.htm):
  `%%d/%%p/%%c` Unicode U+00B0/U+00B1/U+2205'tir; klasik (Unicode olmayan) fontta bunların şekil numarası için
  kaynaklar çelişiyor (127/128 Bentley'in aktarımı, 256–258 Autodesk'in sembolik adları); doğrulanabilir tek eşleme yok.
- AutoCAD eTransmit (ADR 1111 §Araştırma): destek dosyaları, SHX yazı fontları dahil, çizimin yanında taşınır.

## 2. Karar: ne SHX ile çizilir

- Bir **yazı nesnesi** (TEXT/MTEXT karşılığı, `Entity::Text`) stilinin `fontFile`'ı `.shx` ise
  (`fonts::resolve → Shx`) ve yazı kendi yüzünü seçmemişse (`overrides` içinde `font` yok), yazı projeye **gömülü** SHX
  kaynağından çizilir (`kentos_native_application::shx_text`). Disk ya da ağ araması yok; dosya adı yalnız taban adıyla
  eşlenir (ADR 0930).
- Yerleşim ADR 0145/0182'nin kuralıdır, harf genişliği SHX ilerlemesidir: yükseklik büyük harf yüksekliği;
  genişlik çarpanı x'e, eğiklik (tan) yazının kendi çerçevesinde `x + y·k` olarak (TrueType yazının kutusu gibi);
  satır sonları, kutu genişliğinde sözcük kaydırma (eski yerleşimin `wrap` kuralı: ilk satır paragrafın başından,
  sonraki satırlar sözcüğünden; satır sonundaki boşluk yer tutmaz); satır aralığı `5/3 · h · aralık`; her satır kutuda
  hizanın payı kadar; `p` hizanın noktası (`paragraph::rise`). Kutu (`outline`) TrueType kutusunun oranlarıdır
  (taban altında 0,23 h, üstte 1,15 h); maske bu kutudur.
- Çizim: ekranda (`labels.rs`, `Ink::stroke_line`), PDF'de (`MapLabel.shx` → yuvarlak uçlu/birleşimli `MapPath`,
  nesnenin ya da katmanın çizgi kalınlığında; baskı profili kalınlığı/rengi aynı yoldan uygular) ve CPU baskı
  resminde (`raster.rs`, tiny-skia) aynı çizgiler. PDF'de yazı yüzle yazılmaz (`text_of` atlanır); SHX yazısı
  aranabilir metin değildir (AutoCAD'in SHX PDF'i gibi).
- Önbellek: `place_cached` yazının yerini, sözcüklerini, stilin fontlarını ve fontların SHA-256'sını anahtar yapar
  (iş parçacığı başına en çok 4096); aynı görünüm yeniden çizilirken font yeniden okunmaz. Cevap `place` ile aynı.

## 3. Karar: ne çizilmez ve nasıl söylenir

- Gömülü font **çizemiyorsa** (fontta olmayan karakter, Big Font'ta olmayan `\M+` kodu, Big Font'suz `\M+`, sekme,
  harf biçimli koşu, eğri boyunca yazı, girinti/sütun/Unicode yerleşim profili) yazı stilin yüzüyle çizilir ve
  `drawing.inspect`/pafta ön denetimi **`text_shx_unsupported` hatası** verir (nedeni ve karakteri adıyla); PDF yayını
  bu kodla reddedilir (`book.rs`'nin engelleyen kodları). Hiçbir karakter boşluk olarak geçilmez, yerine geçen yüz
  özgünmüş gibi teslim edilmez.
- Font ya da Big Font **gömülü değilse** eski davranış sürer: stilin yüzü, `text_font_substituted` uyarısı; ileti artık
  eksik dosyayı ve “SHX ekle…”/`cad.library.edit` çözümünü söyler. `cad.fonts.report` aynı satırı `font_substituted`
  ile söyler; ileti “KentOS okumaz” yerine “projeye eklenmemiş ya da okunamıyor”.
- **Unicode → Big Font:** fontta olmayan Unicode karakteri Big Font'ta **aranmaz**; hata, Big Font dosyasının kod
  sayfası bildirmediğini ve karakterin `\M+` koduyla yazılmasını ya da onu içeren bir unifont seçilmesini söyler.
  Gerekçe: dosya biçimi kod sayfası taşımaz, proje çizimin kod sayfasını saklamaz; bir tablo (yeni bağımlılık) ile
  yapılacak çeviri hangi sayfanın doğru olduğunu bilemez. Klasik fontta 0x80 üstü karakter ADR 1120 §3 gibi reddedilir.
- **Blok içindeki yazı** (`PieceText`) bu dilimde SHX ile çizilmez: stilin yüzüyle çizilir ve `text_font_substituted`
  uyarısı “Blok içindeki yazı SHX fontuyla çizilmiyor” der (sessiz değil). Boyut değerleri, kılavuz notları, tablo
  hücreleri ve etiketler de yüzle çizilir (stilleri ayrıdır).

## 4. Komut ve sözleşme

- `cad.fonts.report` v1 çıktısı `StyleFont.embedded` (yoksa `false`): stilin SHX fontu ve adlandırdığı Big Font
  projeye gömülü ve okunuyor; böyle stil `font_substituted` değildir. Komut sürümü değişmez (yalnız eklenen, varsayılanı
  eski davranış olan alan). Katalog, Python SDK'sı, manifest yeniden üretildi; MCP aynı katalogdan okur.
- Paylaşılan durumlar: `fixtures/commands/v1/cad.fonts.report.json` 28 → 49 durum; 21 yeni durum
  `scripts/fixtures/font_report_cases.py`'nin bağımsız kuralıyla (KentOS kodu yok): gömülü unifont, unifont + Big Font,
  eksik Big Font, eksik yazı fontu, büyük/küçük harf, yollu dosya adı, şekil dosyası (yazı fontu değil), ters adlanmış
  Big Font/unifont, TrueType yanında Big Font, iki stil, yazı sayımı, ad sorusu, plan = execute, kullanılmayan kaynak,
  başka adlı kaynak, şekil dosyası Big Font yerine.
- `fixtures/shx/v1/expected.json`'a 23 yerleşim durumu (`texts`): betiğin kendi okuyucusu ve ADR 0145/0182 kuralıyla
  yerdeki çizgiler (hizalar, dönüş, çarpan, eğiklik, üç satır, aralık, kutuya kaydırma, Big Font, sondaki/baştaki
  boşluk, büyük koordinat, boş satır, CR LF).
- `.kcad` şeması değişmez (yeni alan yok; ADR 1120'nin `bigFontFile`/kaynakları yeter).

## 5. CAD aktarımı ve paket

- `cad_shape_files::plan`: çizgi tipi şekillerinin yanına, her yazı stilinin **projeye gömülü** SHX yazı fontu ve Big
  Font'u eklenir; DXF/DWG'nin STYLE tablosu onları adlandırdığı için yanına bayt bayt yazılır (aynı içerik yeniden
  kullanılır, farklı içerik/symlink reddedilir, ADR 0930 §17'nin işlemi). Gömülü olmayan yazı fontu (ör. `romans.shx`)
  alıcının fontudur: rapora girmez, kayıp uyarısı vermez, diskte aranmaz. Python/MCP planı `report.shapeResources`'ta
  yolu, boyu, özeti söyler, yazmaz.
- Paket (ADR 1111): gömülü SHX fontlu stilin `font` satırı `embedded` (“projeye gömülü SHX yazı fontuyla çizilir”),
  `left`'e `substituted` olarak girmez; her gömülü yazı fontu/Big Font bir `shapeFile` satırıdır (`embedded`,
  kullanan `stil:Ad`). Kaynak zaten proje dosyasının içindedir.

## 6. Kanıt

- `kentos-native-application` `shx_text`: 3 test — 23 bağımsız yerleşim durumu 1e-9 içinde aynı; ret iletileri
  (eksik karakter, kod sayfası, Big Font'ta olmayan kod, sekme, biçimli koşu, eğri, Big Font'suz `\M+`, gömülü olmayan
  font/Big Font, Big Font'u yazı fontu, unifont'u Big Font sayma); stil kuralı ve önbelleğin `lay_out` ile aynılığı.
- `kentos-plot` `shx_texts`: 5 referans durumu 1:100/1:250'de PDF'in vektör yollarında (yuvarlak uç, 0,35 mm, nokta
  nokta referansla 1e-9), yüzle yazı yok, bitmap yok, belge değişmez; CPU resminde her referans noktası boyalı, U'nun
  açık ağzı boş; eksik karakter `text_shx_unsupported` hatası ve PDF reddi; kod sayfası iletisi; gömülü olmayan Big Font
  uyarısı ve yüzle yazılan PDF.
- `kentos-headless` `cad_shape_files`: gömülü unifont ve Big Font planda (yazmadan), işlemde dosya yanına aynı bayt;
  gömülü olmayan `romans.shx` yazılmaz.
- Python (`python/tests/test_shx_texts.py`): `cad.fonts.report` `embedded`, ön denetimde bulgu yok, DXF planı/aktarımı
  iki fontu yanına yazar; eksik karakter hatası; eksik Big Font uyarısı.
- Ekranlar `.run/shots/arac-shx-metin-*.png` (repoya alınmaz).

## 7. Sınırlar

- Blok yazısı, ölçü değeri, kılavuz notu, tablo hücresi, etiketin SHX ile çizimi; eğri boyunca SHX yazısı; harf biçimli
  MTEXT koşuları; dikey (`modes 2`) yazı; genişletilmiş Big Font ilkeli (ADR 1120) açık.
- Ekranda SHX çizgileri 1,2 piksel; ekranın çizgi kalınlığı gösterimi yazıya uygulanmaz (PDF/CPU kalınlığı uygular).
- Seçim kutusu ve yakalama hâlâ TrueType ölçüsüyle (`TextPlace`); SHX genişliği farklıysa kutu yazıyı tam sarmayabilir.
- Gerçek AutoCAD çıktısıyla karşılaştırma yapılmadı; test fontları KentOS'un kendisinindir.
