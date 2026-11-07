# Gerçek pencere: kullanıcı gibi kabul

Sahibin sözü (7 Kasım 2026): “mouse ile kullanmak ile kod testleri arasında fark oluşuyor”. Kod içi izler
(`kentos-cad kullan`, `snapshot`, `cargo test … traces`) Iced olaylarını uygulamaya doğrudan verir. Gerçek kullanımda
devreye giren şeyler orada yoktur: işletim sisteminin imleci, gerçek pencere, sahibin ayarları (açık tema, Kenet ve
Izgara açık), Başlangıç penceresi, şerit. Bu dizin uygulamayı gerçek pencerede, gerçek fare ve klavye olaylarıyla
sınar ([ADR 3000](../../../docs/adr/3000-real-window-acceptance.md)).

## Nasıl çalışır

- **Ekran:** `Xvfb :91` görünmez bir 1440×900 X ekranı açar; masaüstüne ve imlece dokunmaz.
- **Uygulama:** `kentos-cad` X11 penceresi olarak açılır. Xvfb'de DRI3 olmadığı için yazılım Vulkan'ı (lavapipe,
  `WGPU_BACKEND=vulkan`) kullanılır.
- **Ev dizini:** her koşu kendi geçici ev dizininde çalışır. İçine sahibin `ayarlar.json`, `yerlesim-mimari.json` ve
  `pencereler.json` dosyalarının kopyası konur. Sahibin `~/.config/kentos-cad`'i, kurtarma kopyaları ve son dosyaları
  hiç yazılmaz. Veritabanı bağlantıları kopyalanmaz.
- **Sürücü:** `driver.py` senaryoyu XTEST ile gerçek olay olarak gönderir: imleç aradaki noktalardan geçer, tuşlar
  basılıp bırakılır, Türkçe harfler için boş bir tuş ödünç alınır. Resimleri X ekranından alır.
- **Sayaç:** `count.py` senaryonun etkileşimlerini sayar ve Keystroke-Level Model süresini verir (kurallar dosyanın
  başında).

## Çalıştırma

```sh
scripts/dev/svc.sh heavy cargo build -p kentos-desktop --bin kentos-cad
scripts/usage/real/session.sh fixtures/interaction/real/daire-2p1.txt .run/gui-evidence/gercek-2p1 --modul mimari
python3 scripts/usage/real/count.py fixtures/interaction/real/daire-2p1.txt
```

**Gerekenler:** `xvfb` paketi, Mesa'nın lavapipe sürücüsü (`/usr/share/vulkan/icd.d/lvp_icd.json`) ve `uv`. `uv`,
`python-xlib` ile `pillow`'u sabit sürümleriyle getirir.

**Ortam değişkenleri:**
- `KENTOS_BIN`: başka bir ikili.
- `KENTOS_REAL_SETTINGS`: başka bir ayar klasörü.
- `KENTOS_REAL_DISPLAY`: başka bir ekran numarası.
- `KENTOS_REAL_START`: açılış beklemesi, saniye.

## Senaryo biçimi

Her satır bir el hareketidir; `#` ile başlayan satır ve ` #`'ten sonrası açıklamadır:

| Satır | Ne yapar |
|---|---|
| `fit KentOS Mimari` | O başlıklı pencereyi ekranı kaplayacak biçimde büyütür ve klavyeyi ona verir (pencere yöneticisi yok) |
| `wait MS` | Bekler |
| `move X Y` | İmleci aradaki noktalardan geçirerek götürür |
| `click X Y [right\|middle]`, `dclick X Y`, `drag X1 Y1 X2 Y2` | Tık, çift tık, sürükleme |
| `wheel X Y N` | Orada N tekerlek adımı: artı yaklaştırır, eksi uzaklaştırır |
| `key SPEC` | `Return`, `Escape`, `Home`, `F8`, `ctrl+a` … |
| `type METİN` | Metni tuş tuş yazar |
| `shot AD` | Ekranın resmi: `ÇIKTI/AD.png` |

**Yazım kuralları:**
- Noktalar ekran pikselidir. Çizimdeki bir noktanın pikseli, görünüm bilinen bir duruma getirilince (Home) ölçülür. Ölçek ve
  başlangıç senaryo üreticisinin başında yazılıdır (`scenario_2p1.py`). Şerit ya da panel düzeni değişirse yeniden ölçülür.
- Bir aracı başlatan tıkın açıklaması `araç:` ile başlar; sayaç düşünme süresini (M) oradan sayar.

## Sınırlar

- **Pencere sistemi:** sahibin oturumu KDE Plasma Wayland'dır, burada X11 kullanılır. Wayland'e özgü ölçekleme farkları
  görülmez.
- **Pencere yöneticisi:** yok. Pencere çerçevesi yoktur, odağı sürücü verir.
- **Çizim:** yazılımla yapılır; kare süresi ölçümü için kullanılmaz.
- **Çok oturum:** iki oturum aynı ekran numarasını kullanmamalıdır.
- **Neden başka yol değil (7 Kasım'da denendi):**
  - KWin'in görünmez iç içe oturumu (`kwin_wayland --virtual --xwayland`) gerçek pencereyi gösteriyor ve `spectacle` ile
    resmini alıyor, ama XTEST olayları XWayland'den uygulamaya geçmiyor.
  - `/dev/uinput` yalnız root'a açık ve xdotool/ydotool kurulu değil.
