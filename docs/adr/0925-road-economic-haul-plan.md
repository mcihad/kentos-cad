# 0925 — KentOS Yol: serbest taşımalı ve fiyatlı (ekonomik) taşıma planı

Tarih: 2026-11-04. plan/road.md Y24 (ROAD-HAUL-09, ROAD-HAUL-10; ADR 0924'ün açığı). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** serbest taşıma uzaklığı içindeki taşıma serbest taşımadır. Ötesi fazla taşımadır (overhaul) ve
  uzaklığıyla ödenir. Serbest taşıma, eğim noktası ya da denge noktası yöntemiyle ölçülür
  ([About Free Haul and Overhaul](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4DF3D901-69A8-4A6D-AA8B-27F89D7DA66D.htm);
  4 Kasım 2026'da okundu). Ödünç ve döküm alanları hacimleriyle dengelemeye katılır (ADR 0924).
- **Ekonomik taşıma sınırı:** yol yapımı ders kitaplarında LEH = serbest taşıma + ödünç fiyatı / fazla taşıma
  birim fiyatı olarak geçer. Bu uzaklığın ötesinde kazıyı dökmek ve dolguyu ödünçten almak taşımaktan ucuzdur. Bu
  bilgi ders notlarından ve el kitabı özetlerinden okundu, kurum belgesi değildir; formülün birimi kaynaktan kaynağa
  değişir.
- **Trimble Business Center ve Softree Optimal:** kütle taşımasını en düşük maliyetle eniyiler; Softree ödünç,
  döküm ve yan dökümle eniyiler (ADR 0924'ün kaynakları).
- **Netcad:** NETPRO'nun Brükner'inde serbest taşıma ve fazla taşıma maliyeti vardır (ADR 0907). Çevrimiçi
  resmî eniyileme belgesi bulunamadı.
- Fiyatlar ve serbest taşıma kullanıcınındır (ROAD-DEC-01). KentOS sayı vermez. Brüknerin `free` ve `cost` alanları
  ADR 0907 §5'tedir.

## Karar

### §1 Hesap

`kentos_road::haul_plan::plan_priced` bir ulaştırma problemidir. Her yarma ile her dolgu arasında, her yarma ile
her döküm alanı arasında ve her ödünç alanı ile her dolgu arasında bir yay vardır. Fazla taşımanın fiyatı yol boyunca
toplanamaz (serbest taşımayı her taşıma bir kez düşer), bu yüzden yaylar çift çifttir.

Yayın maliyeti fazla taşıma birim fiyatı × max(0, uzaklık − serbest taşıma)'dır. Alanın birim fiyatı kaynaktan ya da
batağa giden yayındadır. Maliyet en düşüktür; aynı maliyetteki planlar içinde moment en küçüktür. Bu iki ölçü sırayla
karşılaştırılır; ardışık en kısa yollar ve Bellman–Ford tamdır. Kurulabilirlik ve kazanç kuralı ADR 0924 §1'deki
gibidir.

Çıktılar:

- fazla taşıma momenti: Σ hacim × (uzaklık − serbest taşıma), artı olanlar;
- fazla taşıma, döküm ve ödünç maliyetleri ve toplam;
- ekonomik taşıma sınırı: serbest taşıma + (döküm ve ödünç fiyatı, verilenler) / metre başına fazla taşıma fiyatı.

Parçaların akışı alanların kullanımından çıkar, net akıştır. Moment çiftlerin uzaklığıyla toplanır.

### §2 Komut

`road.hauls.plan` v1'in girdisinde isteğe bağlı `objective` vardır: `moment` (öntanımlı, ADR 0924) ya da `cost`.
Cevapta `objective`, `free` ve maliyet amacında `cost` yer alır. Maliyet planı kurulamazsa `cost` yoktur.

Ret: `invalid_haul`, şu durumlarda:

- Brüknerin fiyatı yok;
- fazla taşıma birim fiyatı yok ya da 0;
- döküm alanı var ama döküm fiyatı yok;
- ödünç alanı var ama ödünç fiyatı yok.

Ret, alanların km denetiminden sonra verilir. Serbest taşıma yoksa 0'dır (ADR 0907: her taşıma fazla taşımadır).

### §3 Masaüstü

Taşıma planı penceresinde Amaç seçilir: En küçük moment ya da En düşük maliyet. Maliyet amacında yanda şunlar
gösterilir: serbest taşıma, ekonomik taşıma sınırı, fazla taşıma momenti, kalemler ve toplam, para biriminde.

## Doğrulama

`fixtures/commands/v1/road.hauls.plan.json`'a 14 durum eklendi; dosyada toplam 40 durum var. Bağımsız başvuru
(`scripts/fixtures/road_haul_plan_cases.py`) ulaştırma programını HiGHS'la iki aşamada çözer:

1. En düşük maliyet bulunur.
2. Birinci aşamanın indirgenmiş maliyeti 0 olan yaylarında en küçük moment bulunur. Bu yüzün köşesidir; maliyet
   sınırı satırı eklenmez, çünkü o satır bir çevrim bırakırdı.

Alanların kullanımının tek olduğu denetlenir. Kullanılan yaylar kesirli Gauss yok etmesiyle yeniden çözülür. Maliyet,
moment ve fazla taşıma momenti kesirlerle bulunur. KentOS kodu kullanılmaz.

Durumlar:

- serbest taşımalı ve serbest taşımasız planlar;
- pahalı fazla taşıma;
- ödünç ile döküm birlikte;
- yalnız ödünç fiyatı (EUR);
- kurulamayan plan ve açık `moment`;
- dört ret.

Rust ve Python'da oynatılır. Masaüstü testi ve resimleri `.run/gui-evidence/yol-y23-tasima-ekonomik-*` ve
`-serbest-*`'tır.

## Açıklar

- Ödünç ve döküm alanı olmadan yol boyunca yan döküm ve yan ödünç (Softree'nin sidecast'i).
- Serbest taşımanın eğim noktası ve denge noktası yöntemleriyle Brüknerin okuması arasındaki fark (ADR 0907 katman
  okuması).
- Büyük kübajda başarım: çift yaylar O(n²); Dijkstra ve potansiyellerle ya da ağ simpleksiyle hızlandırma.
- Taşıma yolu, takvim, malzeme türleri ve karbon (ROAD-HAUL-05, 08, 09, 10'un kalanı).
