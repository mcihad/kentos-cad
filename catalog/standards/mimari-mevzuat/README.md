# Mimari mevzuat denetimi

KentOS'un standart kural paketi (`tr.kentos.mimari-mevzuat`; plan/bim-plan.md K10, BIM-13; docs/adr/8078). Türkiye imar
mevzuatının mimari çizimdeki kuralları, projenin başvuru gününde yürürlükte olan hâlleriyle.

- **Kural seti başvuru gününe göre** (`yururluk`): maddelerin hâlleri `src/mimari_mevzuat/data/surumler.json`'da, metni,
  Resmî Gazete'si, yürürlük günleri ve geçiş hükmüyle. Gün künyenin `basvuru_tarihi`'dir. Gün yoksa kural her hâlle
  denenir: sonuç değişiyorsa bulgu "eksik veri"dir.
- **Asansör** (`asansor`): PAİY 34/1 ve 34/8; yerel yönetmelik seçiliyse Sivas 34/1 ve 34/8. Planın kat adedi parselin
  plan notundan, yoksa künyeden (docs/adr/8098).
- **Kaçış yolu kapıları** (`kapilar`): BYKHY 47/2 (kalabalık mekânın kapısı kaçış yönüne açılır) ve 47/3 (kaçış merdiveni
  ve yangın güvenlik holü kapısının sınıfı).
- **Emsal sınırları** (`emsal`, docs/adr/8079): KentOS Emsal ve TAKS tablosunda 22/1'in sınırlarını sayar (balkon %20,
  toprak altında depo eklentisi %50, kapıcı daireleri 70 m², kütüphane 50 m²); kural koşullarına bakar: emsal harici
  kapıcı dairesi 44/1'e göre zorunlu mu (22/1-ç), kütüphane alışveriş merkezinde ya da 40'tan fazla bağımsız bölümlü
  binada, normal katta ve ortak alan mı (22/1-f), geleneksel kesme taş yığma binanın emsal harici taşlık, dış sofa ve
  eyvanı zemin dahil 3 kata kadar binada ve zemin katta mı (22/1-m, docs/adr/8094); kalın taş duvarların payı (5/8,
  20/8-j, docs/adr/8095): beyansız kalın duvar, katı ve başvuru günü; Sivas'ta duvarlar %30'un içinde (22/k,
  docs/adr/8096).
- **Merdiven altı** (`merdiven_alti`, docs/adr/8080): 1,80 m'den alçak merdiven altı kapatılır (31/6); alçak boşluk bir
  mahale açıksa kaldı.
- **Yangın kompartımanları** (`kompartiman`, docs/adr/8081, 8086): mahaller arasındaki ayrımlardan ve katlar arası
  açıklıklardan kompartımanlar (en az EI 60, açık yer yok; galeri, atrium, merdiven katları birleştirir); tehlikeli
  alanların ayrımı (BYKHY 24/2), yüksek binada en çok üç kat (24/4), atrium (24/5), Ek-4'ün en çok alanı (24/6),
  kompartıman duvarındaki açıklıklar (24/7). `mimari.kompartimanlar` kompartımanları IFC dosyasının zon grupları için
  verir (docs/adr/8088).
- **Yangına dayanım süreleri** (`dayanim`, docs/adr/8082, 8087): beyan edilen sınıf Ek-3/B ve Ek-3/C'nin istediğiyle:
  kolon ve kiriş R, döşeme REI, dış ve kompartıman duvarı EI, korunumlu yangın merdiveni REI 120, teras döşemesi REI,
  çatı EI, asma tavan EI 30 (docs/adr/8089); beyan edilen taşıyıcı duvar R, kaçış yolu olan çatı REI 30
  (docs/adr/8090).
- **Kaplamaların yanıcılığı** (`kaplamalar`, docs/adr/8091): döşeme kaplaması en az normal alevlenici (yüksek binada
  zor alevlenici, 26/2), tavan kaplaması ve asma tavan en az zor alevlenici (26/4); malzemenin TS EN 13501-1 sınıfı
  Ek-2/Ç ile.
- **Şaftlar ve asansörler** (`kuyular`, docs/adr/8087): yüksek binada tesisat şaftı ve baca duvarı en az 120, kapağı en
  az 90 dakika ve duman sızdırmaz (BYKHY 25/3); kaçış merdivenine ya da yangın güvenlik holüne açılan şaft kapağı (41/9);
  asansör kuyusu en az 60 dakika (62/2); asansör kapısının koridor, hol ve benzerine açılması, bodrumda korunmuş
  koridor ya da yangın güvenlik holü (62/4, docs/adr/8092), yangın merdiveni
  yuvasına açılmaması (62/6); kat kapısının dayanımı, yapı yüksekliği 51,50 m'den yüksekse E 60, alçaksa E 30 (62/7 ve
  Ek-3/B 12, 9.7.2015'ten).
- **Kat yüksekliği** (`kat_yuksekligi`, docs/adr/8083, 8084): konut projesinde zemin ve normal katlar 3,60 m;
  12.8.2023'ten bodrum hariç 3 kata kadar 4,00 m (PAİY 28/1-c); Sivas 3,60 m. Parselin bölgesine göre ticaret (28/1-a),
  karma (28/1-b), zemin katı ticaretli konut alanı (28/1-ç) ve asma katlı zemin kat (docs/adr/8085); konut bölgesinde
  zemin dahil 3 kata kadar geleneksel kesme taş yığma binada 9.10.2021'den 4,50 m (28/1-d, projenin taşıyıcı sistem
  beyanıyla, docs/adr/8094). Projenin plan notu (zemin kat, kat yüksekliği) hepsinin önüne geçer (69/3).
- **Bina yüksekliği ve kat adedi** (`bina_yuksekligi`, docs/adr/8098): planın plan notundaki bina yüksekliği (±0,00'dan
  son katın tavan döşemesinin üstüne) ve kat adedi (zemin ve normal katlar) aşılamaz (PAİY 5/11).
- **Kapı ölçüleri** (`kapi_olculeri`, docs/adr/8083): yükseklik 2,10 m; net genişlik bina girişinde 1,50, bölüm girişinde
  1,00, öbür kapılarda 0,90 m; 13.8.2025'e kadar ve Sivas'ta balkon ve tuvalet kapısı 0,80 m (PAİY 39/1). İki katmanın
  yargısı `katman`'da; gün yoksa her kat ve kapı ayrı denenir (`yururluk.parcalarla`).
- **Olgular** KentOS'un ölçtüğüdür (`facts.building`, `facts.permit`); kural çizimi okumaz.
- **Bağımsız durumlar** `fixtures/standards/v1/asansor.json`, `kapilar.json`, `emsal.json`, `merdiven-alti.json`, `kompartiman.json`, `dayanim.json`, `kat-yuksekligi.json` ve `kapi-olculeri.json`: beklenen sonuçlar mevzuat
  metninden elle çıkarıldı.

Sınama: `python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py`, `test_kapilar.py`, `test_emsal.py`, `test_merdiven_alti.py`, `test_kompartiman.py`, `test_dayanim.py`, `test_kat_yuksekligi.py` ve `test_kapi_olculeri.py`.
