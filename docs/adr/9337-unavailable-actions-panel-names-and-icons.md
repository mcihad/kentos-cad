# ADR 9337: Karşılaştır üç görünümde kapanır, panel düğmeleri grubu söyler, Asistan'ın kutusu dar panelde kesilmez, simgeler satırda

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  9, P3, ve iki simge tablosu). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0523](0523-linked-views-themes-and-swipe.md) ve [ADR 0637](0637-up-to-four-views.md) (bölünmüş görünüm,
    kaydırmalı karşılaştırma);
  - [ADR 0117](0117-desktop-ribbon-bar-and-menus.md) (şerit ve menüler);
  - [ADR 8056](8056-phase-2-mcp-v2-and-in-app-agent.md) (Asistan);
  - [ADR 9328](9328-quick-measure-box.md) (Hızlı ölçüm).

## Bağlam

- **Madde 9:**
  - Üç ya da dört görünümde Karşılaştır basılabiliyordu, bastıktan sonra iletiyle reddediliyordu.
  - Görünüm › Paneller'deki Katman paneli ve Komut geçmişi düğmeleri bütün sağ ve alt panel gruplarını açıp
    kapıyordu; adları tek bir sekmeyi anıyordu.
  - Asistan normal pencerede açıldı; yazma kutusunun yer tutucusu satır ortasından kesildi. Kutunun yüksekliği yazılı
    metnin mantıksal satırlarından hesaplanıyor, dar panelde sarılan satırları saymıyordu.
- **Simge tabloları:**
  - Taşma menülerinde aç/kapa komutlarının satırı yalnız onay işaretini gösteriyordu, komutun simgesini göstermiyordu:
    Tam ekran, Çizgi kalınlığını göster, Mevcut durum.
  - Görünüş grubunun başlatıcısı genel bir çapraz oktu; ipucu “Uygulama ayarları: görünüm” idi.
  - Boru hattı denetimi altyapı ağı yokken yalnız Komut geçmişine yazdı; Alt ağlar penceresi Ağlar… eylemini sunuyordu.
  - Ölçme › Hızlı ölçüm: grup ADR 9328'de Harita'nın simgeli komutlarıyla kuruldu (Mesafe ölç, Alan hesapla, Açı ölç,
    Koordinat oku).

## Karar

### 1. Karşılaştır

- **Uygunluk:** Kaydırmalı karşılaştır bir ya da iki görünümde açık, üç ya da dört görünümde kapalıdır.
- **İpucu:** kapalıyken “Kaydırmalı karşılaştırma için iki görünüm kullanın: fazla görünümleri üstlerindeki × ile
  kapatın.” Komut satırından çalıştırılınca da aynı söz söylenir.
- **Açıklama:** “Tek görünümdeyken basınca çizim alanı ikiye bölünür ve karşılaştırma başlar; üç ya da dört görünümde
  kapalıdır.”

### 2. Panel düğmeleri

- **Adlar:** `view.rightPanel` “Sağ panel”, `view.bottomPanel` “Alt panel” (şerit ve ipucu).
- **Açıklamalar:** sağdaki ve alttaki panel grubunu sekmeleriyle birlikte gösterip gizlediklerini söyler; alt panelin
  sağdaki Öznitelikler'i etkilemediği yazılır. Bir sekmeyi açmak için onun komutu kullanılır.
- **Kısa adlar:** masaüstü web envanterinden gelen kısa adı geçersiz kılabilir (`DESKTOP_SHORTS`).

### 3. Asistan'ın kutusu

- Kutu, metninin (boşken yer tutucusunun) panelin genişliğinde sarılan satırları kadar yüksektir: en az iki, en çok
  altı satır; sonrası kutunun içinde kayar.
- Sohbet kutunun üstünde kayar. Kutu ve Gönder altta erişilir kalır.

### 4. Simgeler

- **Aç/kapa satırı:** taşma ve açılır listelerde onay işaretinin yanında komutun simgesi durur (Tam ekran, Çizgi
  kalınlığını göster, Mevcut durum). Tam ekran açıkken satır “Tam ekrandan çık” der.
- **Başlatıcı:** komut açan başlatıcı o komutun simgesini gösterir (Uygulama ayarları, Katman stili, Proje ayarları…).
  Sekmeye götüren başlatıcı çapraz okta kalır. Görünüş'ün başlatıcısının ipucu “Görünüm ayarları”dır.
- **Boru hattı denetimi:** altyapı ağı yokken iletiye ek olarak bir bildirim çıkar. Bildirim nedeni ve **Ağlar…**
  eylemini, Alt ağlar'daki gibi tek basışta sunar.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Karşılaştır'ın kapanması ve ipucu | var | var | var | var |
| Sağ panel, Alt panel adları | var | var | var | var |
| Asistan'ın kutusu | var | var | var | var |
| Aç/kapa satırının simgesi, başlatıcının simgesi | var | var | var | var |
| Boru hattı denetiminin Ağlar… bildirimi | yok: Mimari'de altyapı ağı yoktur | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Hepsi arayüzün sunumudur; komutlar aynıdır. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Klavye odağında ipucu:** başlatıcının ve araçların açıklaması fare üstündeyken görünür; klavye odağında gösterilmez
  (kitin ipucu yalnız fareyle açılır).
- **Form tasarımcısı ve alt tablo:** araç ipuçları ve daralınca taşma menüsü bu dilimde yapılmadı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- catalog ribbon split_many assistant view:: module_tests
  history::text settings`: 259 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (`shell/approved_layer.rs`,
  `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden değil).
  - `review_panels_tests`:
    - Sağ panel ve Alt panel adları ve açıklamaları; Karşılaştır'ın tek görünüm açıklaması;
    - bir ve iki görünümde Karşılaştır açık, üçte kapalı ve nedeni;
    - aç/kapa satırlarında simge ve onay, Tam ekran açıkken “Tam ekrandan çık”.
  - `split_many_tests`: üç görünümde komut aynı sözle reddediliyor, şeritte kapalı.
  - `assistant`: 24 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9337/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-gorunum-paneller`: üç görünümde sönük Karşılaştır; Sağ panel, Alt panel; Görünüş başlatıcısında Uygulama
    ayarları'nın simgesi;
  - `03-asistan-dar`: kısa ve dar panelde başlık yok, öneriler kayıyor, yer tutucu iki satıra tam, model etiketi sarılıyor,
    Gönder tam.
    - İlk çekimde Gönder kesik ve sıkışıktı; satır düzeni ve kısa paneldeki başlık bu yüzden değişti.
  - `04-boru-denetimi-aglar`: altyapı ağı yokken bildirim, nedeni ve Ağlar….
- **Şerit:** `-- ribbon review_panels`: 66 geçti (özel panel gruplarının başlatıcıları da komutun simgesini alır).
- **Kit:** `cargo test -p kentos-ui --lib --features snapshot -- ai`: 37 geçti.
- **Harita paketi:** `pipes_prerequisite_tests::without_a_utility_network_the_check_offers_aglar`: bildirimin eylemi Ağlar…
  (`network.manage`), iletide neden.
- **Takımlar:** `cargo test -p kentos-cad` (izler) 37, `cargo test -p kentos-desktop-cbs` 63 geçti.
