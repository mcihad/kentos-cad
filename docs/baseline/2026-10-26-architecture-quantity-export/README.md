# Metraj seçili CSV ve doğru kayıt sayısı — ADR0681

26 Ekim 2026. `codex/architecture-2d-workflows`, başlangıç `2cefc14df`.
**Seçili CSV ve kayıt sayısı dilimi doğrulandı. Ana ARCH-DOC-02 açık.**

## Gerçek hata

`01-before-walls-selected.jpg`: Duvarlar sekmesinde 200 duvar,7200m.
CSV kaydet →`02-before-export-feedback.jpg`: 600 satır başarı bildirimi.
`before-walls.csv` bağımsız Python csv ile 400 mahal ve mahal başlıklarıdır;
duvar verisi yoktur. Paket ve kaynak SHA'ları `before-binary.json` içindedir.

## Çözüm ve kabul sınırı

CSV seçili tabloyu, Excel/ODS üç tabloyu yazar; mevcut kat filtresi korunur.
Boş sekme CSV'si yalnız başlık içerir. Kullanıcı bu farkı pencerede görür.
Ortak masaüstü yazıcı CSV için ilk sayfanın gerçekten yazılan satırlarını,
kitaplar için bütün sayfaları bildirir. Veri modeli/birim/hesap değişmez.

`quantities::` testleri **3 başarılı**: iki yeni dışa aktarım testi ve eski
kopuk mahal uyarı regresyonu. Tek test içinde üç sekme×üç biçimden dokuz
fiziksel dosya yazılıp geri okundu; bunlar dokuz ayrı test sayılmaz. CSV alıntı,
hücre içi satır sonu, eksik klasör/boş kitap hatası kontrol edildi. Bütün
masaüstü hedefleri `clippy -D warnings` ve paket derlemesi başarılıdır.
`test-runs.json` komut/süre/çıkış/log SHA'larını içerir. İlk test derlemesindeki
modül adı/optional isim hatası diagnostik logda korunur; başarı diye sayılmaz.
Önceki 300 Python/WASM bu UI diliminde tekrar çalıştırılmış test değildir.

## Son pakette gerçek kabul

- 03–04: açık 18px geniş/dar yardım metni ve düğmeler. 16–17: koyu 18/13px dar
  görünüm. Ham pencere 2880×1740 ve2200×1742 civarıdır; 1100×650/1440×900 tam
  hedef matrisi bu örneklemle kapanmaz.
- 05: **Duvar CSV200 satır/7200m**; 06: **Mahal CSV400 satır/14488m²**.
  Mahal CSV'si önceki kabulün 12 sütunlu ham baytlarıyla aynıdır.
- 07–08: boş Açıklık CSV'si kendi altı başlığını ve0 veri satırını korur.
- 09–10: boş aktif sekmeden XLSX/ODS bütün üç tabloyu taşır. 13–14 komut
  geçmişinde CSV200/400/0 ve kitap600 satır başarıları gerçek olarak görünür.
  Mevcut yazıcı15 anlamlı basamağı aşan1600 ham değeri metin tutup bildirir;
  son durum satırındaki uzun sayı notu başarı satırının yerine görünür.
  Bu dilim o hassasiyet/yuvarlama politikasını değiştirmez; her hücrenin sayı
  olduğu veya1600 notun yeni hata olduğu iddia edilmez.
- 11–12: native Save'de adı verilen `cancelled-export.csv` Cancel ile bırakıldı;
  dosya oluşmadı, geçmişte o adla başarı yok.
- 15: kaydedilen KCAD gerçek Dosya Aç ile yeniden yüklendi; taze Metraj yine
  400 mahal/14488m²,200 duvar/7200m. Bütün21000 nesne/UUID/ayar bağımsız KCAD
  okuyucusuyla ilk modelle aynıdır.

`verify.py` KentOS tablo okuyucusunu kullanmadan Python csv/zipfile/XML/Decimal
ile gerçek CSV/XLSX/ODS satırlarını, toplamlarını ve uzun metin değerlerini
okur; ODS tekrarlı boş hücreleri açar. KCAD için mevcut bağımsız dosya okuyucusu
kullanılır. `files-sha256.json` gerçek çıktı baytlarını, `binary.json` çalışan
paketin ve üç değişmiş Rust kaynağının SHA'larını korur.17 ham görüntü vardır.

`gui-events.json` ham JPEG SHA'larını tutar; resimler değiştirilmez. Testler
ayrı `.run/architecture-quantity-export` profili ve dosyalarını kullanır.
ARCH-DOC-02'nin gruplama/evre/tip/kaplama/donatı/PDF kapsamı açık kalır.
