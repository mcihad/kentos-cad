# KentOS Launcher

KentOS'un programlarını (KentOS Mimari, KentOS Harita, KentOS GIS) ve **hangi dille yazılmış olursa olsun** başka
programları ya da web adreslerini tek pencereden başlatır; KentOS programlarında son açılan çizimleri gösterir.
Karar ve gerekçeleri: [ADR 0800](../adr/0800-kentos-launcher.md). Kaynak: `apps/launcher`, KentOS programlarıyla ortak
sözleşme: `crates/native/launch`.

```sh
kentos-launcher                                   # pencere
kentos-launcher snapshot ekran.png --sayfa son    # pencere açmadan resim (aşağıda)
```

Her uygulama bir **manifesttir**: kartta ne görüneceğini ve uygulamanın nasıl başlayacağını söyleyen küçük bir JSON
dosyası. KentOS'un kendi modülleri de birer manifesttir (launcher'ın içinde, `apps/launcher/assets/apps`).

## Uygulamalar nereden okunur

Üç yerden, bu sırayla:

1. **Yerleşikler:** launcher'ın içinde. KentOS Mimari, KentOS Harita, KentOS GIS; KentOS YOL, KentOS 3D ve KentOS Afet
   “yakında”.
2. **Kurulumla gelenler:** launcher'ın yanındaki `uygulamalar/` klasörü (ör. `/opt/kentos/bin/uygulamalar/`). Bir kurum
   kendi programlarını ya da bir KentOS modülünün başka bir kurulumunu buraya koyar.
3. **Kullanıcının uygulamaları:** `$XDG_DATA_HOME/kentos/uygulamalar/` (değişken yoksa `~/.local/share/kentos/uygulamalar/`).
   Uygulama ekle buraya yazar; Düzenle ve Kaldır yalnız buradakiler içindir.

Klasördeki `*.json` dosyaları ada göre sırayla okunur; `.` ile başlayanlar atlanır. Aynı `id` sonraki yerde yeniden
geçerse **sonraki öncekinin yerine geçer**: kurum ya da kullanıcı bir yerleşik uygulamayı başka bir programa
yöneltebilir (kendi manifesti silinince yerleşik geri gelir). Aynı klasörde aynı `id` ikinci kez geçerse ikincisi yok
sayılır ve söylenir.

**Okunamayan manifest launcher'ı durdurmaz:** kart çıkmaz, zilin bildirimlerinde dosyası ve nedeni yazar. Yalnız
görünüşü ilgilendiren bir alan (simge, renk, resim, sürüm, sıra, dosyalar) okunamazsa kart varsayılanla çıkar, alan
yine bildirilir.

## Biçim: `kentos.launcher-app/1`

```json
{
  "format": "kentos.launcher-app/1",
  "id": "parsel-raporu",
  "name": "Parsel raporu",
  "note": "Python betiği: ada ve parsel raporu",
  "icon": "terminal",
  "color": "orange",
  "picture": "resimler/rapor.png",
  "version": "1.2",
  "state": "ready",
  "launch": { "kind": "program", "program": "python3", "args": ["rapor.py", "{dosya}"] },
  "order": 60,
  "files": ["csv", "xlsx"]
}
```

| Alan | Tür | | Anlamı |
|---|---|---|---|
| `format` | metin | zorunlu | Her zaman `kentos.launcher-app/1`. Başka bir sürüm okunmaz. |
| `id` | metin | zorunlu | Küçük harf, rakam ve tire (`a-z0-9-`), en çok 64; tireyle başlamaz ve bitmez. Bir uygulama, bir kimlik. |
| `name` | metin | zorunlu¹ | Kartın adı, en çok 60 harf. |
| `note` | metin | | Kartın altındaki tek satır. |
| `icon` | metin | | Yerleşik simgenin adı (aşağıda) ya da manifestin klasörüne göre bir `.svg`/`.png` yolu. Yoksa `uygulama`. |
| `color` | metin | | Karonun rengi (aşağıda). Yoksa `gray`. |
| `picture` | metin | | Kartın resmi: yerleşik resmin adı ya da bir `.png` yolu. Yoksa karo rengiyle simge. |
| `version` | metin | | Kartta `v1.2` diye yazılır, en çok 32 harf. KentOS modüllerinde launcher'ın sürümü. |
| `state` | metin | | `ready` (öntanımlı) ya da `soon`: henüz çıkmamış; kart “Yakında” der, bir şey başlatmaz. |
| `launch` | nesne | zorunlu² | Nasıl başlar: aşağıdaki üç türden biri. |
| `order` | tam sayı | | Kartların sırası (küçük önce), eşitse ad. Yoksa `100`; KentOS'unkiler 10–60. |
| `files` | metin dizisi | | Açtığı dosyaların uzantıları, noktasız (`["csv", "xlsx"]`). Kartın ⋯ menüsündeki **Dosya aç…** bunlarla açılır. |

¹ `kind: kentos` ise yazılmayabilir: modülün adı ve notu kullanılır. ² `state: soon` ise yazılmayabilir.

Bilinmeyen alanlar yok sayılır (ileride eklenecekler eski launcher'ı bozmaz). Dosya en çok 256 KB.

### `launch`: üç tür

**Program** (`"kind": "program"`): herhangi bir dilde herhangi bir program.

| Alan | Anlamı |
|---|---|
| `program` | Tam yol, manifestin klasörüne göre bir yol (`bin/rapor`) ya da yalnız ad (`python3`, `java`): ad önce launcher'ın yanında, sonra `PATH`'te aranır. |
| `args` | Bağımsız değişkenler, dizi olarak (tırnak sorunu yok). `{dosya}` açılacak dosyanın yeridir (`--girdi={dosya}` de olur). Dosya verilmeden başlatılınca `{dosya}` taşıyan değişken atlanır; `{dosya}` yoksa dosya en sona eklenir. |
| `env` | Ortam değişkenleri, `{"AD": "değer"}`. |
| `cwd` | Çalışma klasörü, manifestin klasörüne göre. Yoksa manifestin klasörü. |

**Web adresi** (`"kind": "url"`): `url` `http://` ya da `https://` ile başlar; sistemin tarayıcısında açılır
(Linux'ta `xdg-open`, macOS'ta `open`, Windows'ta `explorer`). Web adresinin `files`'ı olmaz.

**KentOS modülü** (`"kind": "kentos"`): `module` `mimari`, `harita` ya da `cbs`. Launcher önce modülün kendi adını
(`kentos-mimari`, `kentos-harita`, `kentos-gis`: tek program dosyasının bağlantıları, ADR 0802 §1), yoksa
`kentos-cad --modul <modül>`'ü çalıştırır; ikisi de launcher'ın
yanında, sonra `PATH`'te aranır. Bir dosya `kentos-cad`'e `--modul <modül> <dosya>`, modülün kendi programına yalnız
`<dosya>` olarak verilir (`kentos_launch::args`).

### Simgeler (`icon`)

`mimari` (bina), `harita` (ölçme aleti), `cbs` (katlanmış harita), `yol` (yol), `3d` (3B şehir), `afet` (sismograf),
`rapor` (belge), `uygulama` (dört karo), `terminal`, `web` (küre), `tablo`, `grafik`, `klasor`. Hepsi KentOS'un 20 × 20
çizgi simgeleridir; karonun rengindeki mürekkeple çizilir. Bir SVG ya da PNG dosyası verilirse karonun yerine olduğu gibi
çizilir.

### Renkler (`color`)

`blue` (mavi), `teal` (petrol), `green` (yeşil), `yellow` (sarı), `orange` (turuncu), `red` (kırmızı), `purple` (mor),
`gray` (gri). Karo yukarıdan aşağı açılan bir geçişle boyanır, simge beyazla (sarıda koyu mürekkeple); her renkte simge
ile karo arasında en az 3:1 karşıtlık vardır.

### Resimler (`picture`)

Yerleşikler: `mimari`, `harita`, `cbs`, `yol`, `3d`, `afet` (sahibin resimleri; `scripts/launcher/pictures.py --fit` ile). Kartın resim alanı 2:1'dir;
kendi resminiz için kartın iki katı boyda, 2:1 bir PNG verin (ör. 720 × 360). Oranı başka olan resim alanı doldurur,
taşan kenarları kesilir. En çok 4096 × 4096 piksel. Okunamayan resim bildirilir, kart resim yerine karonun renginde
simgeyle çıkar.

## Kartın durumu

| Söz | Ne zaman | Düğme |
|---|---|---|
| Kurulu | Programı bulundu | ▶ Başlat |
| Çalışıyor | Buradan başlatıldı, hâlâ açık (birden çoksa sayısıyla) | ▶ Başlat (bir tane daha) |
| Web adresi | `kind: url` | Aç |
| Bulunamadı | Programı bulunamadı: neden ipucunda | Konumu seç… |
| Yakında | `state: soon` | Yakında (kapalı, nedeni ipucunda) |

Üzerine gelinen ya da seçili kartın düğmesi dolu, öbürleri çizgilidir. **Konumu seç…** seçilen programı yalnız o
uygulama için saklar (`konumlar.json`); kartın ⋯ menüsünde **Konumu sıfırla** onu bırakır.

Başlayan her program kendi sürecidir; launcher onu beklemez, saniyede bir bakar. Program 0'dan başka bir kodla ya da
sinyalle kapanırsa bildirimde nedeni, çıktısının son satırı ve çıktı dosyası yazar. Launcher kapanınca başlattığı
programlar açık kalır.

Güncelleme denetimi henüz yoktur: launcher sürümleri yalnız gösterir; **Tümünü güncelle** kapalıdır ve nedenini söyler.

## Dosyalar

| Dosya | Ne |
|---|---|
| `$XDG_CONFIG_HOME/kentos-launcher/ayarlar.json` | Tema (Sistem, Açık, Koyu), yazı boyutu (11–18), “Uygulama başlayınca küçült”. Okunamazsa öntanımlılar. |
| `$XDG_CONFIG_HOME/kentos-launcher/konumlar.json` | Konumu seç… ile gösterilen programlar, uygulamanın kimliğiyle. |
| `$XDG_STATE_HOME/kentos-launcher/gunluk/<id>.log` | Başlatılan programın çıktısı (standart çıktı ve hata); her başlatmada baştan yazılır. Aynı uygulamanın önceki kopyası hâlâ çalışıyorsa `<id>-2.log`, `<id>-3.log` … |
| `$XDG_STATE_HOME/kentos-cad/son-dosyalar.json` | Son dosyalar: KentOS programları yazar, launcher yalnız okur. |

Hepsi bütün yazılır, sonra yerine taşınır (yarım dosya kalmaz). Değişkenler yoksa `~/.config`, `~/.local/state`
kullanılır.

## Klavye

Ctrl+F (macOS'ta ⌘F) aramaya gider; oklar kartlar arasında gezer; Enter seçili kartı, seçili kart yoksa görünen ilk
başlatılabilir kartı başlatır; Esc sırasıyla açık pencereyi, bildirimleri, aramayı kapatır.

## Örnekler

Hepsi [`ornekler/`](ornekler) klasöründe; bir test hepsinin uyarısız okunduğunu denetler.

- [`parsel-raporu.json`](ornekler/parsel-raporu.json): Python betiği, yorumlayıcısıyla; dosyayı `{dosya}` ile alır.
- [`kot-hesabi.json`](ornekler/kot-hesabi.json): Java programı (`java -jar …`).
- [`kurum-portali.json`](ornekler/kurum-portali.json): Electron programı, kendi resmiyle.
- [`e-plan.json`](ornekler/e-plan.json): web adresi.
- [`kentos-gis-kurum.json`](ornekler/kentos-gis-kurum.json): kurulumun `uygulamalar/` klasöründe, yerleşik KentOS GIS'in
  yerine geçen manifest (aynı `id`).

## Resimler

```sh
kentos-launcher snapshot çıktı.png [--sayfa uygulamalar|son|guncellemeler|indirmeler|ayarlar|ekle|ikonlar]
                [--tema koyu|acik] [--boyut 1440x900] [--yazi 13|18] [--secili kentos-harita] [--bildirimler]
python3 scripts/launcher/pictures.py --fit 3d=sehir.png afet=afet.png   # sahibin resimlerini kartlara uydurur (altı ad)
python3 scripts/launcher/pictures.py --kentos /yol/kentos-cad   # dört kartın resmini KentOS'un kendisiyle yeniden çeker
```

`snapshot` sabit veriyle çizer (yerleşik uygulamalar, `/opt/kentos/bin`'de bulunan `kentos-cad`, sabit bir son dosyalar
listesi, çalışan program yok): resimler her makinede aynıdır; kullanıcının hiçbir dosyası okunmaz ya da yazılmaz.
Simge sayfasının (`--sayfa ikonlar`) on sekiz satırı 900 piksele sığmaz; `--boyut 1440x1280` ile çizin.
