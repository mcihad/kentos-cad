# 1037 — Kübaj ne ölçtüğünü söyler

Tarih: 2026-11-07. Durum: kabul (Y102). Kapsam ROAD-VOL-19, ADR 0975 §2 (Netpro'nun kübaj kuralları), ADR 0906 (kübaj),
ADR 0957 (hacim halleri).

## Sorun

Kübaj hesabı kuralını (`rule`) Netpro'nun adlarıyla tutuyordu. ROAD-VOL-19 ayrıca şunları istiyor:

- kuralın sürümlü olması;
- her kuralın hangi iki çizgiyi, hangi yatay ve düşey sınırları ve hangi malzeme halini kullandığını söylemesi.

Rapor yalnız kuralın adını veriyordu. Kuralsız bir hesabın ne ölçtüğü (yol üstü ile zemin, sıyırmada sıyrılmış zemin)
koddan okunuyordu.

## Araştırma (7 Kasım 2026)

- **Netpro'nun kuralları** (ADR 0975 §2'nin kaynağı): Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak,
  Formasyon–Bitkisel Toprak. Sıyırma Kazısı kendi kalemidir.
- **KGM birim fiyat tanımları** (ADR 0975 ve 0984'ün kaynakları): kazı yerinde (sıkışık) hacimle ölçülür; dolgu
  sıkıştırılmış gövde hacmidir; kabarma ve sıkışma katsayıları bunları birbirine çevirir (ADR 0906 §4,
  `road.volumes.states`: "kazı yerinde, dolgu sıkışmış, ölçüldüğü gibi").
- **Civil 3D'nin malzeme listesi:** bir kalemi iki yüzey (üst ve alt) ve sınırlar (örnek çizgisinin genişliği)
  tanımlar. KentOS'ta kesit yöntemlerinde sınır şev uçları, yüzey ve ızgara yönteminde koridorun üst yüzeyidir.
- **Matematik:** hesap yok. Raporun kuralından, yönteminden ve satırlarından okunur.

## Karar

- **`VolumeReport.basis`**, yalnız `road.volumes.query`'nin cevabında; öbür komutların cevabı değişmedi. Alanları:
  - `name`: Netpro'nun adı.
  - `given`: hesabın kendi kuralı mı. Kuralsız hesapta varsayılan yol üstü ile zemin, sıyırmada yol üstü ile
    sıyrılmış zemindir.
  - `version`: `kentos.volume-rule/1`. Bir kuralın çizgisi, yanı ya da hali değişirse sürüm artar.
  - `top` ve `bottom`: `road`, `formation`; `ground`, `topsoil`.
  - `sides`: `daylights` (ortalama alan ve prizmoidal yöntem) ya da `corridor` (yüzeyler arası ve ızgara).
  - `from` ve `to`: satırların km aralığı.
  - `cut: inPlace`, `fill: compacted`.
- **Çekirdek:** `road_volumes::basis_of` yalnız raporu okur; sorgu ve masaüstü aynı işlevi kullanır.
- **Kübaj penceresi:** diyagramın altındaki satır "Kural: Yol–Zemin (kuralsız): yol üstü–zemin, koridor yüzeyi"
  der; rakamlar "Kazı (yerinde)", "Dolgu (sıkışmış)" olur.

## Ölçüm

- **Bağımsız başvuru** `road_volume_command_cases.py` (`with_basis`, KentOS kodu yok) hesabın tanımından kurar.
- **`road.volumes.query`:** 48 durumda 43 hesap karşılaştırılıyor. Netpro'nun dört kuralı da var:
  - Yol–Zemin kuralsız, şev uçları: 22;
  - Yol–Zemin kuralsız, koridor yüzeyi: 8;
  - Yol–Bitkisel Toprak kuralsız sıyırmayla: 5;
  - Formasyon–Zemin: 4;
  - Yol–Bitkisel Toprak kuralla: 2;
  - Formasyon–Bitkisel Toprak: 1;
  - Yol–Zemin kuralla: 1.
- Bir hesabın yanı `corridor`'dan `daylights`'a çevrilince durum düşüyor.
- **Masaüstü testi:** ana kübaj (ortalama alan) şev uçları, yan kübaj (yüzeyler arası) koridor yüzeyi.
- **Resimler** `.run/gui-evidence/yol-y102/`.
