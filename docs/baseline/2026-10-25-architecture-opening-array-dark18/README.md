# Açıklık dağıt — 18 px koyu tema ve dar panel takibi

25 Ekim 2026 · Dal `codex/architecture-2d-workflows` · Taban `8191f0cd7`.
Önceki [işlevsel kabulün](../2026-10-25-architecture-opening-arrays/README.md)
gerçek pencere takibidir. **ARCH-OPEN-02 [~] kalır**: 1440×900 / 1100×650
boyutlarının açık/koyu × 13/18 px tam matrisi tamamlanmadı.
Bu oturumdaki gerçek iç alan 1440×828, fiziksel görüntü 2880×1720 (2×).

## Gerçek etkileşim ve sonuç

Ayrılmış Ribbon QA uygulamasında, önceki 12 nesneli KCAD üzerinden; yalnız
bu test uygulamasının ayarları Görünüm ekranından koyu tema / 18 px yapıldı.
Kullanıcının çalışma çizimi ve asıl uygulama tercihleri kullanılmadı.
Görüntüler gerçek CUA fare/klavye işlemlerinden alınan ham JPEG dosyalarıdır;
aşağıdaki sonuçlar görüntülere bakılarak ve dosyalar ayrıca okunarak doğrulandı.

- **32–35:** mimari ve açıklık bağlamı; kaynak #3 Kapı; geometrik çakışma ve
  K1 numara çakışması okunabilir kırmızı önizleme. Sağ tuş Onayla çakışmayı
  reddeder. 1 kopya / 14 m net aralık ve ilk numara 10 ile geçerli önizleme.
- **36–38:** bir sağ tuş Onayla, kaynak merkez 1 + en 1 + net aralık 14 =
  **16 m** konumunda K10 ekler; toplam 13 nesne ve canlı çizelgede K10 satırı.
  Bir Undo 12, bir Redo 13 nesne. İki durumda da gerçek Kaydet.
- **39–42:** K10 genişliği Öznitelikler'den `1.1m` girilince 1.100 m olur;
  canlı çizelge güncellenir. Escape ve tek Undo ile 1 m'ye döner. Tekerle
  yakınlaştır/uzaklaştır, Shift+H Kaydır ile gerçek 180×90 fiziksel piksel
  sürükleme; Home, kaydet, uygulamayı kapat/aç. Tek fotoğraftan akıcılık veya
  kare süresi kabulü çıkarılmadı.
- **43–44:** yeniden açılışta saklanan 312 mantıksal piksel sağ panel, büyük
  yazıda eski uzun aralık/yön değerlerini kısaltıyordu. Menü bütün ifadeyi
  gösteriyordu; panelde de okunurluk için takip düzeltmesi yapıldı.
- **45–48 (son etiket derlemesi):** `Net aralık`, `Merkez arası`, `Sona doğru`,
  `Başa doğru`, `Uç paylarıyla` yalnız yeni aracın görünür etiketleridir.
  Kaynak/dağıtım/aralık/yön hücreleri 18 px ve 312 px panelde okunur. Dağıtım
  menüsünden Uç paylarıyla, 1 adet, başlangıç 17 m / bitiş 1 m:
  kullanılabilir [17,19] aralığının **18 m** ortasında bir yeni kapı önizlemesi.
  Bir Uygula 14 nesne ve yeni çizelge satırı; tek Undo önceki 13 nesneye döner.

43–44 önceki etiketlerin tespit kanıtıdır; yeni etiket kabulü 45–48'dir.
32–44 GUI ikilisi SHA-256:
`4230233c24a9d58f76e9ab0104596065eff075f070d3fd2e48e5461bbbcfadf7`.
45–48 GUI ikilisi SHA-256:
`3f1eb2397d96f1881aa0a6f3d60aaafa76e5d1be806487f676af8f43dad7b852`.

Bu düzeltme komut girdisini, aralık hesabını, yeni kopya sayısını veya
serileştirilmiş `clear/centers/start/end/fit` değerlerini değiştirmez.
Mimari çizim birimi m, çizim ölçeği 1:100 ve önceki geometri aynıdır.
Yeni TS, Python SDK veya KCAD şema çıktısı yoktur.

## Bağımsız denetim ve çalıştırılan kontroller

`python3 docs/baseline/2026-10-25-architecture-opening-array-dark18/verify.py`
KentOS modülünü çağırmadan `tools/kcad/kcad.py` ile gerçek GUI kayıtlarını okur:

- Önceki `final-one-copy.kcad` ile tek Undo'nun ayar/nesne/UUID dizisi birebir.
- Yeni K10 merkez 16 m, en 1 m; önceki 12 nesnenin tablo dışındaki bütün
  alanları ve UUID'leri korunur; proje ayarları aynı. Tablo K10/1.000 satırını ekler.
- Gerçek özellik düzenlemesinde yalnız yeni kapı ve tablo eni 1.1 / 1.100;
  Undo + gezinme + kapat/aç sonrası 13 nesne/UUID/ayar dizisi Redo kaydıyla birebir.
- Son etiket derlemesinde Fit kopyası merkez 18 m, en 1 m, numarasız. Tek Undo
  bütün önceki 13 nesneyi/UUID'yi/ayarı birebir geri getirir.

Son kaynakta hedef interaction kümesi **4 geçti**; mevcut kabul testi yeni
Türkçe görünen seçenekleri de kullanarak native komuta ulaşır. Son kaynakta
interaction/desktop `--all-targets` clippy ve gerçek masaüstü derlemesi geçti.
Ham günlükler `logs/`, sonuç ve dosya SHA-256 kayıtları `test-results.json`.
Önceki 275 Python/799 sözleşme/1280 desktop ve diğer geniş küme sonuçları
bu etiket farkında yeniden çalıştırılmış gibi sunulmaz; önceki kabul kaydındadır.

Tam pencere matrisi, kalan tema/font birleşimleri ve kare gecikmesi ölçümü
bu kayıtla kapanmaz. Genel panel boyutu veya mevcut diğer araçların etiketleri
bu dilimde değiştirilmedi.
