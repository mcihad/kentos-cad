# Eklenti Vitrini

KentOS eklenti sisteminin yaşayan örneği: küçük bir `.kek` paketi, onunla çalışan yapay bir çizim ve her durumu
gerçek pencerede gösteren senaryolar. Eklenti sistemine eklenen her yetenek buraya da eklenir; gösterimler ve
ekran görüntüleri bununla yapılır (plan/eklenti.md, docs/adr/8057).

| Dosya | Ne |
|---|---|
| `eklenti/` | Paketin kaynağı (`tr.ornek.vitrin`): manifest, Python işlevleri, İşlemler aracı tanımı, kilit, ikonlar |
| `vitrin_cizim.py` → `vitrin.kcad` | Örnek çizim: dört imar parseli (biri yay kenarlı), beş yapı, TAKS düğmeli Parsel formu; TM36, yapay veri |
| `hazirla.sh` | Paketi bir katalog klasörüne koyar ve kataloğun dizinini yazar |
| `senaryolar/*.txt` | Gerçek pencere senaryoları (`scripts/usage/real/session.sh`; 1440 × 900, açık tema) |

## Durumlar

| Durum | Katkı | Nasıl denenir |
|---|---|---|
| Katalogdan kurma, yetenek onayı | manifest, `requestedCapabilities` | Eklentiler › Bulunabilir › Gözden geçir ve kur (`00-kurulum`) |
| Eklentinin kendi Python ortamı | `pyproject.toml`, `uv.lock` | Eklentiler › Python ortamı › Ortamı hazırla: önce paketler gösterilir (`01-python-ortami`) |
| Haritada şekil isteyen araç | `mapTools` (yönlü dikdörtgen, 10 × 12 m şablon) | Vitrin › Yapı yerleştir: ortası, sonra uzun kenarın yönü; Esc'e kadar sürer (`02-harita-araci`) |
| Çizimi okuyan komut, rapor | `commands` › Katman özeti | Vitrin › Katman özeti; katmanın sağ tık menüsünde de (`03-komutlar`) |
| Çizime yazan komut, ilerleme, Durdur | Yapıları numarala (`ctx.progress`) | Çalışma bildiriminde ilerleme çubuğu ("%40 · 3. yapı numaralanıyor") ve Durdur: yazdıkları geri alınır; bitince tek geri alma adımı (`03-komutlar`) |
| Kullanıcıya soran komut | Yapıları numarala (`ctx.dialog`, alanları `kentos.ui_panel` ile: `ui.text_field("onek", "Önek", value="Y-", row="numara")`; açıklaması hesaplı: `calculate`; Önek ve İlk numara bir satırda: `row`) | Şeritten çalışınca KentOS'un penceresi önek ve ilk numarayı sorar; açıklaması yazdıkça "… B-10 … B-14 yazılır." olur, Python'suz (`35-hesap`): B- ve 10 yazılıp Numarala'ya basılınca yapılar B-10 … B-14 olur; İptal'de bir şey yazılmaz (`29-pencere`); bitince Yapı ara paneli açılır, numaralananlar tabloda (`ctx.open_panel`, `32-panel-ac`); İlk numara zorunlu (`required`): boş bırakılınca alanın altında söylenir (`30-zorunlu`); Asistan onu `extension.workshop.trial` ile sentetik çizimde dener: pencereyi `dialogs` ile cevaplar (B-, 5 → B-05, B-06), aynı çağrıda Yapı yerleştir'i dener, `auditAfter` ile Yapı numarası denetimi komuttan sonra yeniden koşar |
| Katman menüsünden komut | `contextMenus` (`target: "layer"`) › Değerlerini say: tıklanan katman `ctx.params["layer"]`'da | Bir katmanın sağ tık menüsü › Değerlerini say: Değer say o katman seçili açılır, alan seçilince sayar (`33-katman-menusu`) |
| Hata veren komut | Hata örneği | Bildirim, Ayrıntı (Python iz kaydı) ve eklentinin günlüğü; çizim değişmez (`03-komutlar`) |
| İşlemler aracı (shapely, NumPy) | `processing` › TAKS | Vitrin › TAKS: yeni katman, tablo ve sayı; Durdur (`04-taks`) |
| Formdan eklenti aracı | Parsel formunun "TAKS hesapla" düğmesi → `ext.tr.ornek.vitrin.taks`, eklentinin sürümü ve özetiyle sabitli | Bir parseli Formla düzenle: düğme aracı o kayıtla açar; ortam hazır değilse "Bekliyor", eklenti yoksa ya da kapalıysa "Kullanılamaz" der; Formlar'da başka sürüm uyarılır (`08-form-eylemi`) |
| Çalışma kökeni | TAKS'ın yazdığı katmanın türetim kaydı: eklenti, sürüm, paket özeti, ortam | TAKS'tan sonra TAKS katmanı › sağ tık › Veri › Veri kümesi bilgisi › Soy (`10-koken`); kaydedilen dosyada CBS verisi sürüm 1 |
| Modelde eklenti aracı | Model tasarımcısı'nda TAKS adımı, eklentinin sürümü ve özetiyle sabitli | Yeni model › Araç ara: TAKS; adımın ayarları "Eklenti Vitrini <sürüm> ile eklendi." der, eklentisiz bilgisayarda hangi sürümle yapıldığını söyler (`09-model`) |
| İfade işlevi | `expressions` › `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` | Etiket metni (`05-etiket`), Katman stili › Kurallar koşulu (`06-kural`), Katman süzgeci (`07-suzgec`), İşlemler'in ifadeleri; çizim beklemez, yanıt gelince yeniden çizilir |
| Denetim kuralı ve düzeltme | `audits` › Yapı numarası (`@ext.audit`) | Vitrin › Yapı numarası: numarasız yapılar alt paneldeki Eklenti denetimi'nde listelenir; Göster nesneye götürür, Düzelt sıradaki numarayı tek geri alma adımıyla yazar (`11-denetim`) |
| Ölçülmüş olgularla denetim | `audits` › Parsel çakışması (`facts.project`) | Vitrin › Denetim › Parsel çakışması: çakışmayı ve boşluğu KentOS ölçer, kural parselin adıyla söyler; örnek çizimin parselleri temiz (`37-olgular`); çakışan sentetik parselleri Asistan `extension.workshop.trial` ile dener |
| Dosya seçici | Katmanın sağ tık menüsü › CSV'den nokta ekle (`ctx.dialog` içinde `filePicker`, `files.userSelected`) | Katmana sağ tıklayıp CSV'den nokta ekle: pencerede dosyanın yolu yazılır ya da Seç… ile seçilir (yalnız CSV); `ornek-noktalar.csv`'nin dört noktası katmana eklenir, öznitelikleriyle (`39-dosya`) |
| Girdiyi yerinde değiştirme | `tools/cekme.tool.json`'da yapıların girdisi `"writes": true`, kodda `ctx.update(yapi, cekme=durum)` | Çekme mesafesi çalışınca her yapının Öznitelikler'inde `cekme` = uygun / taşıyor; geri al tamponlarla birlikte alır (`46-yerinde`) |
| Aracın örnekleri | `tools/taks.tool.json`, `tools/cekme.tool.json`: `examples` (sentetik nesneler, parametreler, beklenen çıktılar ve özet) | Eklentiler › Eklenti Vitrini › Örnekleri çalıştır: iki örnek KentOS'un kendi çiziminde koşar, "2 örneğin hepsi geçti" (`45-ornekler`) |
| Araçtan yerleşik araç | `tools/cekme.tool.json`, Vitrin › Analiz › Çekme mesafesi (aracın içinden `ctx.run_tool("geometry.buffer", input=nesneler, …)`, `ToolAnswer`) | Çekme mesafesi (3 m): KentOS'un Tampon aracı yapıların tamponunu çizer, eklenti parselden taşanı bulur; taşan yapılar seçili, tamponlar "Çekme mesafesi" katmanında (`44-cekme`) |
| Yerleşik araç çağrısı | Vitrin › Analiz › Yapı tamponu (`ctx.run_tool("geometry.buffer", …)`, `ext.layer_input`, `ext.new_layer`) | Yapı tamponu: uzaklık sorulur (3 m), KentOS'un Tampon aracı yapıların tamponlarını yeni katmana çizer; tek geri alma adımı (`43-arac`) |
| Koordinat sistemi seçici | CSV'den nokta ekle'nin Seçenekler sekmesi (`crsPicker`; `cad.crs.transform`) | Seçenekler'de dosyanın sistemi çizimin sistemiyle (TUREF / TM36) başlar; `ornek-noktalar-turef.csv` için TUREF (EPSG:5252) seçilince dört nokta coğrafi koordinatlardan çizimin sistemine dönüşüp TM36 dosyasındakilerin yerine düşer (`42-crs`) |
| Kaydetme yeri, sihirbaz | Katmanın sağ tık menüsü › Katmanı CSV'ye yaz (`filePicker` `save`, `defaultName`; `files.userSelectedWrite`; `ctx.dialog(…, steps=True)`) | Katmana sağ tıklayıp Katmanı CSV'ye yaz: iki adımlı sihirbaz; Sütunlar adımında yazılacak öznitelikler ve ayırıcı, İleri, Dosya adımında kaydedilecek dosyanın yolu yazılır ya da Seç… ile seçilir; uzantısız ada `.csv` eklenir, Yaz nesneleri merkezleri ve seçilen öznitelikleriyle yazar (`41-kaydet`) |
| Panel içi sekme | CSV'den nokta ekle'nin penceresi: bileşenlerin `tab` anahtarı (Dosya, Seçenekler), sekmesiz açıklama her sayfada | Seçenekler sekmesine geçip dosya vermeden Ekle: Dosya sekmesi açılır, alanın altında "dosya verilmedi"; ayırıcı ve öznitelik seçenekleriyle eklenir (`40-sekme`) |
| Katman şeması | `schema.json` (`schemaFile`): Vitrin grubu, Örnek tesis (nokta, formuyla: `forms/tesis.kform`) ve Bakım kaydı katmanları, Tesis türü, Kademe ve İşletme listeleri, ilişki; Örnek tesis'in alt türleri (`subtypes`: türe göre kademe) ve değer grubu (`contingents`: kademe ve işletme) | Vitrin › Katmanları kur: katman ağacında Vitrin grubu belirir, rapor ne eklendiğini söyler; ikinci basış bir şey değiştirmez (`13-sema`); Örnek tesis'e konan noktada Tür = Okul: Kapasite zorunlu, Kademe yalnız İlkokul/Ortaokul/Lise, Lise seçilince İşletme yalnız Bakanlık/Özel (`23-alt-tur`) |
| Panelden harita seçimi | `ui/yakin-yapilar.panel.json` (`entrypoints.panels`): `pointPicker` ("Nokta", `onChange`) ve `numberField` bir satırda (`row`; Uzaklık'ın − ve + düğmeleri `step`, km ile de yazılır: `units`), açıklamanın altında çizgi (`divider`), `yakin-yapilar` komutu | Vitrin › Yakın yapılar: Haritadan seç'e basıp haritada bir nokta gösterin (kenetlenir; Esc vazgeçer) ya da x, y yazın; nokta değişince Uzaklık içindeki yapılar yakından uzağa tabloda, satıra tıklamak yapıyı gösterir (`47-nokta`); Uzaklık'a 0,03 km yazılınca 30 m içinde arar, 2 km'de sınırı metreyle söyler (`49-birim`) |
| Panelden alan çizimi | `ui/alandaki-yapilar.panel.json`: `shapePicker` ("Alan", `"shape": "polygon"`, `onChange`), onay kutusu, `alandaki-yapilar` komutu (alan GeoJSON olarak gelir, `shape()` ile okunur) | Vitrin › Alandaki yapılar: Haritada çiz'e basıp yapıların üstünden bir alan çizin (kenetlenir; Enter ya da çift tık bitirir, Esc vazgeçer); alanın köşe sayısı ve büyüklüğü alanın yanında, alana giren yapılar içinde kalan taban alanı ve payıyla tabloda; "Yalnızca tamamı içinde kalanlar" kesişenleri çıkarır, ✕ alanı bırakır (`48-alan`) |
| İlişki seçicisi ve listesi | `ui/bakim-kayitlari.panel.json`: `relationPicker` (ilişki `bakim-tesis`, `display: "ad"`) ve `relationList` (`of: "tesis"`, `columns`), `bakim-ekle` komutu (`cad.records.create`) | Vitrin › Katmanları kur, Örnek tesis'e iki nokta (Ad: Merkez Okulu, Park Kreşi), Vitrin › Bakım kayıtları: Tesis'te iki tesis adlarıyla; Merkez Okulu seçilip Açıklama yazılıp Ekle: kayıt yazılır, altta liste Python'suz yenilenir; öbür tesis seçilince liste boş (`50-iliski`) |
| Ek listesi | `ui/bakim-kayitlari.panel.json`: `attachments` (`of: "tesis"`) | Vitrin › Bakım kayıtları'nda tesis seçilince altta tesisin ekleri; Adres ekle… ile okulun sayfası eklenir, listede adıyla ve adresiyle görünür, Aç tarayıcıda açar (`51-ek`) |
| Bölünmüş panel | `ui/bakim-kayitlari.panel.json`: `"split": true` | Vitrin › Bakım kayıtları: form üstte, kayıtlar ve ekler altta kendi kaydırmalarıyla; aradaki çizgi sürüklenince yüksekliği paylaşırlar, çift tık ilk hâline döndürür (`52-bolucu`) |
| Panel | `ui/yapi-ara.panel.json` (`entrypoints.panels`, `ui.panel`): metin, arama kutusu, katman seçici, onay kutusu, düğme, tablo | Vitrin › Yapı ara: sağdaki Eklenti sekmesinde açılır; Ara `yapi-ara` komutunu panelin değerleriyle çalıştırır, bulunanlar panelin tablosunda; satıra tıklamak yapıyı gösterir (`14-panel`); Katman seçilirse o katmanda arar (`15-katman-secici`); Yapıları numarala sürerken panel ilerlemeyi ve Durdur'u gösterir, iletisi tabloyu boşaltmaz (`17-ilerleme`); Özetle gösterge kartlarını (`stats`) doldurur, en büyük yapının kartı onu gösterir (`26-ozet`); Değer say ile birlikte açık: Eklenti sekmesinin üstünde birer sekme, geçişte değerler ve satırlar durur, arka planda biten komutun sekmesinde nokta (`21-paneller`); sonundaki İpuçları katlanır bölüm (`section`): kapalı açılır, başlığına basınca açılır (`36-bolum`); Özetle taban alanlarını NumPy ile çizilmiş çubuk grafik olarak da gösterir (`image`, kodun PNG'si: `38-gorsel`) |
| Panel seçicileri | `ui/deger-say.panel.json`: katman seçici, ona bağlı alan seçici (`fieldPicker`: seçilen katmanın alanları) ve nesne seçici (`featurePicker`: çizimdeki seçimden) | Vitrin › Değer say: Katman › Parsel sınırı, Alan › ada, Say: 101 → 3, 102 → 1; iki parsel seçip Seçimi al, Say: yalnız onlar; başka katman alanı ve nesneleri temizler (`16-secici`); haritanın solunda açılır (`side: "left"`), Katmanlar görünür kalır (`18-kenar`); alan seçilince ya da nesneler alınınca sayım kendiliğinden yenilenir (`onChange`, `19-degisince`); sonuç bir ağaçta: her değer bir grup, yanında sayısı, altında nesneleri (`tree`, `20-agac`); aynı satırlar ağacın altında çubuk grafik (`chart`, `of: dagilim`): 101 boyu 3, 102 boyu 1 (`28-grafik`) |
| Panel oturumu | `ctx.panel` (`deger_say`, `deger_sec`): çalışan kod panelini değiştirir: seçimin seçenekleri, metnin yazısı | Vitrin › Değer say: Katman › Parsel sınırı, Alan › ada: Seç kutusu "101 (3)", "102 (1)" seçeneklerini alır, altındaki satır "En sık değer: “101” (3 nesne)." der; Seç › 101: üç parsel çizimde seçilir (`31-oturum`) |
| Kart | `ui/parsel-karti.panel.json`: nesne seçici (`onChange`) ve kart (`card`: `title`, `tag`) | Vitrin › Parsel kartı: iki parsel seçip Seçimi al: her parsel bir kart (101/1 "TAKS aşıyor", 101/2 "uygun"; alan, yapı sayısı, taban alanı, TAKS); karta tıklamak parseli gösterir (`22-kart`); Kart tarihi (`dateField`): 20.11.2026 yazıp Enter ya da takvimden seçilen gün kartları yeniler, her kart tarihi taşır (`24-tarih`); Bilgiler (`multiSelect`): bırakılan çipin satırı kartlardan çıkar (`25-coklu`); Saat (`timeField`) ve Geçerlilik sonu (`dateTimeField`): 14:30 ve 20.11.2026 17:00 yazıp Enter, kartın tarihi saatini, "geçerli" satırı geçerlilik sonunu taşır (`27-saat`); Kart tarihi ve Saat yalnız Tarih seçiliyken görünür (bileşen kuralı `visibleWhen`, Python'suz): Tarih çipi bırakılınca iki alan kalkar (`34-kural`); Kart tarihi bugünle açılır (`defaultExpr: "bugün()"`, `35-hesap`) |
| Yerleşik ikon | `icon: "space-number"`, `"layer-fields"` (kümenin `spaceNumber`, `layerFields` ikonları) | Vitrin sekmesinde Yapıları numarala, katmanın sağ tık menüsünde Katman özeti (`12-ikonlar`); kendi ikonları paketteki SVG'ler |
| Python'suz katkı | `dataSources` (XYZ altlık) | Kaynaklar: Örnek Belediyesi altlığı (sunucusu örnektir, yanıt vermez) |

Vitrin'in iki paneli `kentos.ui.panel` 1'in bileşenlerinden metni, metin alanını, onay kutusunu, katman, alan ve nesne
seçicilerini, düğmeyi, tabloyu ve ağacı gösterir; sayı alanı ile seçim kutusu biçimin kendi testlerinde sınanır. Yeni bileşen
geldiğinde buraya eklenir.

## Hazırlama

```bash
samples/eklenti-vitrini/hazirla.sh /tmp/vitrin       # paket ve katalog
.run/py/bin/python samples/eklenti-vitrini/vitrin_cizim.py   # örnek çizimi yeniden yazmak gerekirse
```

`hazirla.sh` KentOS GIS'i bu kataloğu ve ayrı bir eklenti klasörünü görecek biçimde açan komutu da yazar. Paket
değişince sürümü (`manifest.json`, `pyproject.toml`) ve `CHANGELOG.md`'yi yükseltin; bağımlılık değişince
`eklenti/` içinde `uv lock` çalıştırın. Paketin bir dosyası değişince `vitrin_cizim.py`'yi de çalıştırın: Parsel
formunun düğmesi paketin içerik özetine sabitlidir (masaüstü sınaması eskiyen özeti söyler).

## Senaryolar

Senaryolar ayrı bir oturumda, bilgisayarın kendi ayarlarına ve eklentilerine dokunmadan çalışır:

```bash
H=/tmp/vitrin
KENTOS_EXTENSIONS_DIR=$H/eklentiler KENTOS_EXTENSIONS_CATALOG=$H/katalog \
KENTOS_EXTENSIONS_POLICY=$H/ilke-yok.json KENTOS_PYTHON_DIR=$H/python KENTOS_REAL_DISPLAY=:93 \
  scripts/usage/real/session.sh samples/eklenti-vitrini/senaryolar/02-harita-araci.txt $H/cikti \
  --modul cbs samples/eklenti-vitrini/vitrin.kcad
```

`00-kurulum` ve `01-python-ortami` bir kez çalışır (kurulum ve ortam `KENTOS_EXTENSIONS_DIR`'da ve Python klasöründe
kalır); ötekiler onların üstüne. `ilke-yok.json` bilerek var olmayan bir dosyadır: kurum ilkesi yok.
`KENTOS_PYTHON_DIR` KentOS'un Python ve uv kurulumunun klasörüdür (Eklentiler › Python). Python
kitaplığı değiştiyse önce `python3 scripts/desktop/bundle_sdk.py target/debug --debug`: eklentinin ortamı bir
sonraki çalıştırmada onu alır.

## Yapay zekâ ve otomasyon için

- Eklentinin işlevlerinin kimlikleri manifestin yerel kimlikleridir: `ext.tr.ornek.vitrin.taks` (İşlemler aracı,
  `cad.processing.run` ile de çalışır), `ext.tr.ornek.vitrin.ada-parsel` (ifade işlevi).
- İki İşlemler aracı da `headless-python` bildirir (`tools/*.tool.json`, `capabilities.runtimes`): MCP'de
  (`kentos-mcp`) ve KentOS'un Python kitaplığında masaüstündekiyle aynı kimlik, şema ve sonuçla, eklentinin
  ortamında çalışır. Bildirmeyen bir araç başsız listede görünmez; adıyla istenirse süreç başlamadan reddedilir.
- `ada-parsel` ifade işlevi de başsız çalışır (manifestte `runtimes`: `headless`): MCP'de ve Python kitaplığında bir
  etiketin, stil kuralının, formun ya da İşlemler ifadesinin (`attributes.calculate`) çağrısı eklentinin ortamında
  cevaplanır. `headless` demeyen eklentinin işlevi orada derlenirken nedeniyle reddedilir.
- `cad.processing.tools` eklenti aracını tanımındaki `modelDescription` ile listeler (ne zaman ve nasıl
  kullanılacağı, yapay zekâ için) ve aracın eklentisini (`extension`: kimlik, sürüm, paket özeti) verir; kod
  yüklenmez. Vitrin'in TAKS tanımında örneği var (`eklenti/tools/taks.tool.json`).
- Masaüstüne bağlı bir ajan görünümün ölçeğini `drawing.view` ile okur; Python konsolunda `doc.view()`.
- Bir eklentinin hatası eklentinin günlüğüne (`<eklenti klasörü>/logs/tr.ornek.vitrin.jsonl`) iz kaydıyla yazılır.
- Paketlemeden önce `kentos kek verify samples/eklenti-vitrini/eklenti` (ajanlarda `extension.verify`): paket, Python
  işlevlerinin `@ext` işaretleri ve tehlikeli çağrılar; her bulgu dosyası, satırı ve ne yazılacağıyla.
- Araçların ve kuralların örnekleri pencere açmadan: `kentos kek test samples/eklenti-vitrini/eklenti` doğrular, geçici
  bir kayıt defterine kurar, Python ortamını kilidinden hazırlar ve iki aracın, iki kuralın ve ifade işlevinin
  örneklerini çalıştırır ("9 örneğin hepsi geçti.", çıkış 0; kalan örnek nedeni ve kodun iz kaydıyla, çıkış 1).
  Kuralın örnekleri kendi dosyasındadır (`eklenti/audits/yapi-no.examples.json`): nesneler ya da mimari olgular ve
  beklenen bulgular; ifadeninkiler manifestte (`entrypoints.expressions[].examples`: argümanlar ve değer). Sürekli
  tümleştirmede `--install-python` KentOS'un Python'unu uv ile kurar.
- Yeni bir eklentiye Vitrin'i kopyalayarak değil iskeletle başlayın: `kentos kek new KLASÖR --id … --name … --with
  command,processing,audit,expression,mapTool` (ajanlarda `extension.blueprint`): doğrulamadan sorunsuz geçer.
- SDK'nın rehberi ve `kentos.ext` imzaları: `kentos kek sdk` (`--json` paketin tamamı; ajanlarda `extension.sdk`, MCP'de
  `kentos://sdk/ai-kek-pack`). Bir ajan eklenti yazmadan önce bunu okur.
- Yerleşik ikonlar tire biçiminde yazılır (`space-number`, `layer-fields`; bütün adlar `kentos kek sdk --part icons`).
  Vitrin'in "Yapıları numarala" düğmesi ve katmanın sağ tık "Katman özeti" öğesi örnektir (senaryo 12).
