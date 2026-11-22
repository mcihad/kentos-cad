# ADR 9144: İşlemin girdisi çizimin şimdiki hâlini izler; boş girdide Çalıştır kapalı

- **Durum:** kabul edildi (2026-11-22); adım VY-43. Plan: [plan/veri.md](../../plan/veri.md) VY-43.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  11. maddesi (P1):
  - yol etkin ve seçiliyken İşlemler › Tampon'un ilk girdisi boş Taslak katmanıydı;
  - uyarıya rağmen Çalıştır vurgulu görünüyordu;
  - sonuç katmanı “Park koruma 10 m” adıyla geldi; önceki ayarın mı sabit önerinin mi getirdiği ayrıştırılamadı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Araç penceresi son çalıştırmanın değerlerini geri getiriyordu (`memory.last_values`). Bunlara girdinin kapsamı ve
  katmanı ile sonuç katmanının adı da dahildi. Başka bir katmanda, başka bir iş için açılınca eski girdiyi ve eski adı
  önermesi bundandı.
- Girdinin satırı “uygun nesne yok” diye uyarırken Çalıştır açık kalıyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının işlem araç kutusu:
  - girdiyi etkin katmanla açar;
  - “yalnız seçili nesneler” seçeneği seçimle birlikte sunar;
  - çıktının adı aracın kendi önerisidir; kullanıcı değiştirirse o kalır.

  Kaynak: [işlem araç kutusu kılavuzu](https://documentation.qgis.org/3.44/en/docs/user_manual/processing/toolbox.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Girdi çizimden.** Pencere son değerleri getirirken nesne girdilerini getirmez:
   - seçim varsa ve girdi seçimi alıyorsa kapsam seçimdir;
   - yoksa aracın kendi varsayılanıdır: etkin katman, o katman aracın aldığı nesneyi tutmuyorsa tutan bir katman
     (`propose_layers`);
   - öbür ayarlar (uzaklık, ondalık, …) eskisi gibi geri gelir.
2. **Ad.** Son çalıştırmanın sonuç katmanı adı geri gelmez; aracın kendi önerisi gelir (“Tampon”).
   - Pencerede yazılan ad kişinindir ve kalır.
   - Adı girdiden türetmek (“Yol ekseni — tampon 10 m”) ortak işlem durumlarının sonuç adlarını, yani platformlar
     arası sözleşmeyi değiştirirdi. Bu yüzden yapılmadı (§4).
3. **Boş girdi.** Bir şey okuması gereken nesne girdisi şimdi hiçbir şey okumuyorsa Çalıştır kapalıdır; nedeni girdinin
   altındaki satırdadır.

## §3 Kanıt

- **Masaüstü** (`gis/processing/result_name_tests.rs`):
  - son çalıştırma başka bir katmanda “Park koruma 10 m” adıyla kayıtlıyken parsel seçili Tampon açılıyor: girdi seçim,
    ad “Tampon”;
  - yazılan ad uzaklık değişse de kalıyor;
  - yalnız bir yazı seçiliyken girdi boş, Çalıştır kapalı.
- “Hatırlanan tür daraltması geri gelmez” sınaması yeni kurala göre güncellendi: girdi geri gelmiyor, öbür değerler
  geliyor.
- İşlemler, ifade oluşturucu, izler, şerit ve modül sınamaları (183) geçiyor.

## §4 Kalan

- **Türetilmiş ad.** “Girdi katmanı — araç uzaklık” biçimindeki ad, ortak işlem durumları ve web ile birlikte
  değiştirilmeli.
- **Seçim satırı.** Seçim varken girdinin satırında “Yol ekseni · 1 seçili / 1 kayıt”.
- **İş kaydı.** Girdi, seçim, birim, algoritma, süre, hedef ve yazılan sayının hepsi.
- **Sınamalar.** Coğrafi ve metre kullanan sistem, iptal, bozuk geometri ve var olan çıktı adı ayrı ayrı.
- Web (TODOS.md §0.2).
