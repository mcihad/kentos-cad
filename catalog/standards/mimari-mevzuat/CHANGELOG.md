# Değişiklikler

## 0.2.0 — 17 Kasım 2026

- Kaçış yolu kapıları (`kapilar`, docs/adr/8078 §8): BYKHY 47/2 (kullanıcı yükü 50'yi aşan mekânın çıkış kapısı kaçış
  yönüne açılır; döner kanatlı olmayan kapı uzman incelemesi) ve 47/3 (kaçış merdiveni ve yangın güvenlik holü kapısı
  duman sızdırmaz, kendiliğinden kapanır; 4'ten az katta 60, bodrumda ya da 4'ten fazla katta 90 dakika; bodrumsuz tam 4
  kat uzman incelemesi; beyan yoksa eksik veri). Kapının sınıfı TS EN 13501-2 yazımıyla okunur. 20 bağımsız durum.
- Sınamalar paketin dışında (`catalog/standards/tests/mimari-mevzuat/`): doğrulayıcı paketin `sys.path` değiştirmesini
  reddeder.

## 0.1.0 — 17 Kasım 2026

- İlk sürüm (docs/adr/8078; plan/bim-plan.md BIM-13): başvuru gününe göre kural seti (`yururluk`, `data/surumler.json`)
  ve asansör kuralı (`asansor`): Planlı Alanlar İmar Yönetmeliği 34/1'in 28.7.2018, 11.7.2021 ve 1.7.2026 hâlleri,
  34/8'in 11.7.2021 ve 1.7.2026 hâlleri; Sivas İmar Yönetmeliği 34/1 ve 34/8. Bulgu durumu: kaldı, eksik veri (başvuru
  tarihi yok ve sonuç ona göre değişiyor; okunamayan tarih), uzman incelemesi (metni kural setinde olmayan gün, yalnız
  teknik hacimli bodrum). Maddelerin özet satırları geçti, kaldı, eksik veri, uzman incelemesi ya da kapsam dışı.
