# Mimari mevzuat denetimi

KentOS'un standart kural paketi (`tr.kentos.mimari-mevzuat`; plan/bim-plan.md K10, BIM-13; docs/adr/8078). Türkiye imar
mevzuatının mimari çizimdeki kuralları, projenin başvuru gününde yürürlükte olan hâlleriyle.

- **Kural seti başvuru gününe göre** (`yururluk`): maddelerin hâlleri `src/mimari_mevzuat/data/surumler.json`'da, metni,
  Resmî Gazete'si, yürürlük günleri ve geçiş hükmüyle. Gün künyenin `basvuru_tarihi`'dir. Gün yoksa kural her hâlle
  denenir: sonuç değişiyorsa bulgu "eksik veri"dir.
- **Asansör** (`asansor`): PAİY 34/1 ve 34/8; yerel yönetmelik seçiliyse Sivas 34/1 ve 34/8.
- **Kaçış yolu kapıları** (`kapilar`): BYKHY 47/2 (kalabalık mekânın kapısı kaçış yönüne açılır) ve 47/3 (kaçış merdiveni
  ve yangın güvenlik holü kapısının sınıfı).
- **Olgular** KentOS'un ölçtüğüdür (`facts.building`, `facts.permit`); kural çizimi okumaz.
- **Bağımsız durumlar** `fixtures/standards/v1/asansor.json` ve `kapilar.json`: beklenen sonuçlar mevzuat metninden elle
  çıkarıldı.

Sınama: `python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py` ve `test_kapilar.py`.
