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
| Kullanıcıya soran komut | Yapıları numarala (`ctx.dialog`) | Şeritten çalışınca KentOS'un penceresi önek ve ilk numarayı sorar: B- ve 10 yazılıp Numarala'ya basılınca yapılar B-10 … B-14 olur; İptal'de bir şey yazılmaz (`29-pencere`); bitince Yapı ara paneli açılır, numaralananlar tabloda (`ctx.open_panel`, `32-panel-ac`); İlk numara zorunlu (`required`): boş bırakılınca alanın altında söylenir (`30-zorunlu`) |
| Hata veren komut | Hata örneği | Bildirim, Ayrıntı (Python iz kaydı) ve eklentinin günlüğü; çizim değişmez (`03-komutlar`) |
| İşlemler aracı (shapely, NumPy) | `processing` › TAKS | Vitrin › TAKS: yeni katman, tablo ve sayı; Durdur (`04-taks`) |
| Formdan eklenti aracı | Parsel formunun "TAKS hesapla" düğmesi → `ext.tr.ornek.vitrin.taks`, eklentinin sürümü ve özetiyle sabitli | Bir parseli Formla düzenle: düğme aracı o kayıtla açar; ortam hazır değilse "Bekliyor", eklenti yoksa ya da kapalıysa "Kullanılamaz" der; Formlar'da başka sürüm uyarılır (`08-form-eylemi`) |
| Çalışma kökeni | TAKS'ın yazdığı katmanın türetim kaydı: eklenti, sürüm, paket özeti, ortam | TAKS'tan sonra TAKS katmanı › sağ tık › Veri › Veri kümesi bilgisi › Soy (`10-koken`); kaydedilen dosyada CBS verisi sürüm 1 |
| Modelde eklenti aracı | Model tasarımcısı'nda TAKS adımı, eklentinin sürümü ve özetiyle sabitli | Yeni model › Araç ara: TAKS; adımın ayarları "Eklenti Vitrini <sürüm> ile eklendi." der, eklentisiz bilgisayarda hangi sürümle yapıldığını söyler (`09-model`) |
| İfade işlevi | `expressions` › `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` | Etiket metni (`05-etiket`), Katman stili › Kurallar koşulu (`06-kural`), Katman süzgeci (`07-suzgec`), İşlemler'in ifadeleri; çizim beklemez, yanıt gelince yeniden çizilir |
| Denetim kuralı ve düzeltme | `audits` › Yapı numarası (`@ext.audit`) | Vitrin › Yapı numarası: numarasız yapılar alt paneldeki Eklenti denetimi'nde listelenir; Göster nesneye götürür, Düzelt sıradaki numarayı tek geri alma adımıyla yazar (`11-denetim`) |
| Katman şeması | `schema.json` (`schemaFile`): Vitrin grubu, Örnek tesis (nokta, formuyla: `forms/tesis.kform`) ve Bakım kaydı katmanları, Tesis türü, Kademe ve İşletme listeleri, ilişki; Örnek tesis'in alt türleri (`subtypes`: türe göre kademe) ve değer grubu (`contingents`: kademe ve işletme) | Vitrin › Katmanları kur: katman ağacında Vitrin grubu belirir, rapor ne eklendiğini söyler; ikinci basış bir şey değiştirmez (`13-sema`); Örnek tesis'e konan noktada Tür = Okul: Kapasite zorunlu, Kademe yalnız İlkokul/Ortaokul/Lise, Lise seçilince İşletme yalnız Bakanlık/Özel (`23-alt-tur`) |
| Panel | `ui/yapi-ara.panel.json` (`entrypoints.panels`, `ui.panel`): metin, arama kutusu, katman seçici, onay kutusu, düğme, tablo | Vitrin › Yapı ara: sağdaki Eklenti sekmesinde açılır; Ara `yapi-ara` komutunu panelin değerleriyle çalıştırır, bulunanlar panelin tablosunda; satıra tıklamak yapıyı gösterir (`14-panel`); Katman seçilirse o katmanda arar (`15-katman-secici`); Yapıları numarala sürerken panel ilerlemeyi ve Durdur'u gösterir, iletisi tabloyu boşaltmaz (`17-ilerleme`); Özetle gösterge kartlarını (`stats`) doldurur, en büyük yapının kartı onu gösterir (`26-ozet`); Değer say ile birlikte açık: Eklenti sekmesinin üstünde birer sekme, geçişte değerler ve satırlar durur, arka planda biten komutun sekmesinde nokta (`21-paneller`) |
| Panel seçicileri | `ui/deger-say.panel.json`: katman seçici, ona bağlı alan seçici (`fieldPicker`: seçilen katmanın alanları) ve nesne seçici (`featurePicker`: çizimdeki seçimden) | Vitrin › Değer say: Katman › Parsel sınırı, Alan › ada, Say: 101 → 3, 102 → 1; iki parsel seçip Seçimi al, Say: yalnız onlar; başka katman alanı ve nesneleri temizler (`16-secici`); haritanın solunda açılır (`side: "left"`), Katmanlar görünür kalır (`18-kenar`); alan seçilince ya da nesneler alınınca sayım kendiliğinden yenilenir (`onChange`, `19-degisince`); sonuç bir ağaçta: her değer bir grup, yanında sayısı, altında nesneleri (`tree`, `20-agac`); aynı satırlar ağacın altında çubuk grafik (`chart`, `of: dagilim`): 101 boyu 3, 102 boyu 1 (`28-grafik`) |
| Panel oturumu | `ctx.panel` (`deger_say`, `deger_sec`): çalışan kod panelini değiştirir: seçimin seçenekleri, metnin yazısı | Vitrin › Değer say: Katman › Parsel sınırı, Alan › ada: Seç kutusu "101 (3)", "102 (1)" seçeneklerini alır, altındaki satır "En sık değer: “101” (3 nesne)." der; Seç › 101: üç parsel çizimde seçilir (`31-oturum`) |
| Kart | `ui/parsel-karti.panel.json`: nesne seçici (`onChange`) ve kart (`card`: `title`, `tag`) | Vitrin › Parsel kartı: iki parsel seçip Seçimi al: her parsel bir kart (101/1 "TAKS aşıyor", 101/2 "uygun"; alan, yapı sayısı, taban alanı, TAKS); karta tıklamak parseli gösterir (`22-kart`); Kart tarihi (`dateField`): 20.11.2026 yazıp Enter ya da takvimden seçilen gün kartları yeniler, her kart tarihi taşır (`24-tarih`); Bilgiler (`multiSelect`): bırakılan çipin satırı kartlardan çıkar (`25-coklu`); Saat (`timeField`) ve Geçerlilik sonu (`dateTimeField`): 14:30 ve 20.11.2026 17:00 yazıp Enter, kartın tarihi saatini, "geçerli" satırı geçerlilik sonunu taşır (`27-saat`) |
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
- `cad.processing.tools` eklenti aracını tanımındaki `modelDescription` ile listeler (ne zaman ve nasıl
  kullanılacağı, yapay zekâ için) ve aracın eklentisini (`extension`: kimlik, sürüm, paket özeti) verir; kod
  yüklenmez. Vitrin'in TAKS tanımında örneği var (`eklenti/tools/taks.tool.json`).
- Masaüstüne bağlı bir ajan görünümün ölçeğini `drawing.view` ile okur; Python konsolunda `doc.view()`.
- Bir eklentinin hatası eklentinin günlüğüne (`<eklenti klasörü>/logs/tr.ornek.vitrin.jsonl`) iz kaydıyla yazılır.
- Paketlemeden önce `kentos kek verify samples/eklenti-vitrini/eklenti` (ajanlarda `extension.verify`): paket, Python
  işlevlerinin `@ext` işaretleri ve tehlikeli çağrılar; her bulgu dosyası, satırı ve ne yazılacağıyla.
- Yeni bir eklentiye Vitrin'i kopyalayarak değil iskeletle başlayın: `kentos kek new KLASÖR --id … --name … --with
  command,processing,audit,expression,mapTool` (ajanlarda `extension.blueprint`): doğrulamadan sorunsuz geçer.
- SDK'nın rehberi ve `kentos.ext` imzaları: `kentos kek sdk` (`--json` paketin tamamı; ajanlarda `extension.sdk`, MCP'de
  `kentos://sdk/ai-kek-pack`). Bir ajan eklenti yazmadan önce bunu okur.
- Yerleşik ikonlar tire biçiminde yazılır (`space-number`, `layer-fields`; bütün adlar `kentos kek sdk --part icons`).
  Vitrin'in "Yapıları numarala" düğmesi ve katmanın sağ tık "Katman özeti" öğesi örnektir (senaryo 12).
