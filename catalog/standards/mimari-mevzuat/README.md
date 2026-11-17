# Mimari mevzuat denetimi

KentOS'un standart kural paketi (`tr.kentos.mimari-mevzuat`; plan/bim-plan.md K10, BIM-13; docs/adr/8078). Türkiye imar
mevzuatının mimari çizimdeki kuralları, projenin başvuru gününde yürürlükte olan hâlleriyle.

- **Kural seti başvuru gününe göre** (`yururluk`): maddelerin hâlleri `src/mimari_mevzuat/data/surumler.json`'da, metni,
  Resmî Gazete'si, yürürlük günleri ve geçiş hükmüyle. Gün künyenin `basvuru_tarihi`'dir. Gün yoksa kural her hâlle
  denenir: sonuç değişiyorsa bulgu "eksik veri"dir.
- **Asansör** (`asansor`): PAİY 34/1 ve 34/8; yerel yönetmelik seçiliyse Sivas 34/1 ve 34/8.
- **Kaçış yolu kapıları** (`kapilar`): BYKHY 47/2 (kalabalık mekânın kapısı kaçış yönüne açılır) ve 47/3 (kaçış merdiveni
  ve yangın güvenlik holü kapısının sınıfı).
- **Emsal sınırları** (`emsal`, docs/adr/8079): KentOS Emsal ve TAKS tablosunda 22/1'in sınırlarını sayar (balkon %20,
  toprak altında depo eklentisi %50, kapıcı daireleri 70 m², kütüphane 50 m²); kural koşullarına bakar: emsal harici
  kapıcı dairesi 44/1'e göre zorunlu mu (22/1-ç), kütüphane alışveriş merkezinde ya da 40'tan fazla bağımsız bölümlü
  binada, normal katta ve ortak alan mı (22/1-f).
- **Merdiven altı** (`merdiven_alti`, docs/adr/8080): 1,80 m'den alçak merdiven altı kapatılır (31/6); alçak boşluk bir
  mahale açıksa kaldı.
- **Yangın kompartımanları** (`kompartiman`, docs/adr/8081): mahaller arasındaki ayrımlardan kompartımanlar (en az EI 60,
  açık yer yok); tehlikeli alanların ayrımı (BYKHY 24/2), Ek-4'ün en çok alanı (24/6), kompartıman duvarındaki
  açıklıklar (24/7).
- **Yangına dayanım süreleri** (`dayanim`, docs/adr/8082): beyan edilen sınıf Ek-3/B ve Ek-3/C'nin istediğiyle: kolon ve
  kiriş R, döşeme REI, dış ve kompartıman duvarı EI, korunumlu yangın merdiveni REI 120.
- **Kat yüksekliği** (`kat_yuksekligi`, docs/adr/8083): konut projesinde zemin ve normal katlar 3,60 m; 12.8.2023'ten
  bodrum hariç 3 kata kadar 4,00 m (PAİY 28/1-c); Sivas 3,60 m.
- **Kapı ölçüleri** (`kapi_olculeri`, docs/adr/8083): yükseklik 2,10 m; net genişlik bina girişinde 1,50, bölüm girişinde
  1,00, öbür kapılarda 0,90 m; 13.8.2025'e kadar ve Sivas'ta balkon ve tuvalet kapısı 0,80 m (PAİY 39/1). İki katmanın
  yargısı `katman`'da; gün yoksa her kat ve kapı ayrı denenir (`yururluk.parcalarla`).
- **Olgular** KentOS'un ölçtüğüdür (`facts.building`, `facts.permit`); kural çizimi okumaz.
- **Bağımsız durumlar** `fixtures/standards/v1/asansor.json`, `kapilar.json`, `emsal.json`, `merdiven-alti.json`, `kompartiman.json`, `dayanim.json`, `kat-yuksekligi.json` ve `kapi-olculeri.json`: beklenen sonuçlar mevzuat
  metninden elle çıkarıldı.

Sınama: `python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py`, `test_kapilar.py`, `test_emsal.py`, `test_merdiven_alti.py`, `test_kompartiman.py`, `test_dayanim.py`, `test_kat_yuksekligi.py` ve `test_kapi_olculeri.py`.
