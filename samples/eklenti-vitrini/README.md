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
| Çizime yazan komut, ilerleme, Durdur | Yapıları numarala | Çalışırken Durdur: yazdıkları geri alınır; bitince tek geri alma adımı (`03-komutlar`) |
| Hata veren komut | Hata örneği | Bildirim, Ayrıntı (Python iz kaydı) ve eklentinin günlüğü; çizim değişmez (`03-komutlar`) |
| İşlemler aracı (shapely, NumPy) | `processing` › TAKS | Vitrin › TAKS: yeni katman, tablo ve sayı; Durdur (`04-taks`) |
| Formdan eklenti aracı | Parsel formunun "TAKS hesapla" düğmesi → `ext.tr.ornek.vitrin.taks`, eklentinin sürümü ve özetiyle sabitli | Bir parseli Formla düzenle: düğme aracı o kayıtla açar; ortam hazır değilse "Bekliyor", eklenti yoksa ya da kapalıysa "Kullanılamaz" der; Formlar'da başka sürüm uyarılır (`08-form-eylemi`) |
| Çalışma kökeni | TAKS'ın yazdığı katmanın türetim kaydı: eklenti, sürüm, paket özeti, ortam | TAKS'tan sonra TAKS katmanı › sağ tık › Veri › Veri kümesi bilgisi › Soy (`10-koken`); kaydedilen dosyada CBS verisi sürüm 1 |
| Modelde eklenti aracı | Model tasarımcısı'nda TAKS adımı, eklentinin sürümü ve özetiyle sabitli | Yeni model › Araç ara: TAKS; adımın ayarları "Eklenti Vitrini <sürüm> ile eklendi." der, eklentisiz bilgisayarda hangi sürümle yapıldığını söyler (`09-model`) |
| İfade işlevi | `expressions` › `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` | Etiket metni (`05-etiket`), Katman stili › Kurallar koşulu (`06-kural`), Katman süzgeci (`07-suzgec`), İşlemler'in ifadeleri; çizim beklemez, yanıt gelince yeniden çizilir |
| Denetim kuralı ve düzeltme | `audits` › Yapı numarası (`@ext.audit`) | Vitrin › Yapı numarası: numarasız yapılar alt paneldeki Eklenti denetimi'nde listelenir; Göster nesneye götürür, Düzelt sıradaki numarayı tek geri alma adımıyla yazar (`11-denetim`) |
| Yerleşik ikon | `icon: "space-number"`, `"layer-fields"` (kümenin `spaceNumber`, `layerFields` ikonları) | Vitrin sekmesinde Yapıları numarala, katmanın sağ tık menüsünde Katman özeti (`12-ikonlar`); kendi ikonları paketteki SVG'ler |
| Python'suz katkı | `dataSources` (XYZ altlık) | Kaynaklar: Örnek Belediyesi altlığı (sunucusu örnektir, yanıt vermez) |

Henüz çalıştırılamayan katkı türü (panel) eklenti sistemine geldiğinde buraya eklenecek.

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
