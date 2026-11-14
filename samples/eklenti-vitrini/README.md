# Eklenti Vitrini

KentOS eklenti sisteminin yaşayan örneği: küçük bir `.kek` paketi, onunla çalışan yapay bir çizim ve her durumu
gerçek pencerede gösteren senaryolar. Eklenti sistemine eklenen her yetenek buraya da eklenir; gösterimler ve
ekran görüntüleri bununla yapılır (plan/eklenti.md, docs/adr/8057).

| Dosya | Ne |
|---|---|
| `eklenti/` | Paketin kaynağı (`tr.ornek.vitrin`): manifest, Python işlevleri, İşlemler aracı tanımı, kilit, ikonlar |
| `vitrin_cizim.py` → `vitrin.kcad` | Örnek çizim: dört imar parseli (biri yay kenarlı), beş yapı; TM36, yapay veri |
| `hazirla.sh` | Paketi bir katalog klasörüne koyar ve kataloğun dizinini yazar |
| `senaryolar/*.txt` | Gerçek pencere senaryoları (`scripts/usage/real/session.sh`; 1440 × 900, açık tema) |

## Durumlar

| Durum | Katkı | Nasıl denenir |
|---|---|---|
| Katalogdan kurma, yetenek onayı | manifest, `requestedCapabilities` | Eklentiler › Bulunabilir › Gözden geçir ve kur (`00-kurulum`) |
| Eklentinin kendi Python ortamı | `pyproject.toml`, `uv.lock` | Eklentiler › Python ortamı › Ortamı hazırla: önce paketler gösterilir (`01-python-ortami`) |
| Haritada şekil isteyen araç | `mapTools` (yönlü dikdörtgen, 10 × 12 m şablon) | Vitrin › Yapı yerleştir: ortası, sonra uzun kenarın yönü; Esc'e kadar sürer (`02-harita-araci`) |
| Çizimi okuyan komut, rapor | `commands` › Katman özeti | Vitrin › Katman özeti; katmanın sağ tık menüsünde de |
| Çizime yazan komut, ilerleme, Durdur | Yapıları numarala | Çalışırken Durdur: yazdıkları geri alınır; bitince tek geri alma adımı |
| Hata veren komut | Hata örneği | Bildirim, Ayrıntı (Python iz kaydı) ve eklentinin günlüğü; çizim değişmez |
| İşlemler aracı (shapely, NumPy) | `processing` › TAKS | Vitrin › TAKS: yeni katman, tablo ve sayı; Durdur |
| İfade işlevi | `expressions` › `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` | Etiketler (Parsel sınırı › İfade), Katman stili › Kurallar koşulu, Katman süzgeci, İşlemler'in ifadeleri |
| Python'suz katkı | `dataSources` (XYZ altlık) | Kaynaklar: Örnek Belediyesi altlığı (sunucusu örnektir, yanıt vermez) |

Henüz çalıştırılamayan katkı türleri (denetim, panel) eklenti sistemine geldiğinde buraya eklenecek.

## Hazırlama

```bash
samples/eklenti-vitrini/hazirla.sh /tmp/vitrin       # paket ve katalog
.run/py/bin/python samples/eklenti-vitrini/vitrin_cizim.py   # örnek çizimi yeniden yazmak gerekirse
```

`hazirla.sh` KentOS GIS'i bu kataloğu ve ayrı bir eklenti klasörünü görecek biçimde açan komutu da yazar. Paket
değişince sürümü (`manifest.json`, `pyproject.toml`) ve `CHANGELOG.md`'yi yükseltin; bağımlılık değişince
`eklenti/` içinde `uv lock` çalıştırın.

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
- Masaüstüne bağlı bir ajan görünümün ölçeğini `drawing.view` ile okur; Python konsolunda `doc.view()`.
- Bir eklentinin hatası eklentinin günlüğüne (`<eklenti klasörü>/logs/tr.ornek.vitrin.jsonl`) iz kaydıyla yazılır.
