# 1033 — Kot cevabı üçgenini, yöntemini ve güvenini söyler

Tarih: 2026-11-07. Durum: kabul (Y98). Kapsam ROAD-TIN-10, ADR 0900 §8 ve §11 (yüzeyin kotu ve sorgusu), ADR 0920
(tahmin edilen üçgenler).

## Sorun

`road.terrains.query` bir yerin kotunu, yerini (içeride, dışarıda, boşlukta), eğimini ve bakısını veriyordu. ROAD-TIN-10
cevabın şunları da taşımasını istiyor:

- kotun okunduğu üçgen;
- kaynağın sürümü;
- enterpolasyon yöntemi;
- verisizlik ve dışa uzatma durumu;
- kalite.

Üçgen telde yoktu; yöntem ve kalite hiç söylenmiyordu. Bir kotun 2 m'lik bir üçgenden mi, 60 m'lik bir üçgenden mi
geldiği cevaptan anlaşılmıyordu.

## Araştırma (7 Kasım 2026)

- **Civil 3D** ([TIN yüzeyi](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-D0FCED34-D68F-42D2-A6FB-14C454CA57FA.htm)):
  yüzeyin bir noktasındaki kot, noktanın içinde bulunduğu üçgenin köşelerinin kotlarından enterpole edilir; üçgenleme
  Delaunay'dır, kırık hat kenarı onu bozabilir.
- **ArcGIS** ([Create TIN](https://pro.arcgis.com/en/pro-app/3.6/tool-reference/3d-analyst/create-tin.htm)) ve
  **XMS** ([doğrusal enterpolasyon](https://xmswiki.com/wiki/QDlgInInterpLinear)): doğrusal enterpolasyon her üçgeni
  bir düzlem sayar. Nokta kümesinin dış bükey zarfının dışına uzatma yoktur.
- **Kalite:** taramada nokta başına bir güven ölçüsü standardı bulunamadı. KentOS'un kendi kuralı ADR 0920'dedir: kenarı
  kullanıcının sınırını aşan üçgen tahmindir. Bir kotun dayandığı noktaların aralığı (üçgenin en uzun kenarı) ve o
  noktalara uzaklık, kotun ne kadar ölçüye dayandığını söyler.
- **Netcad:** bu araştırmada kaynak bulunamadı.
- **Matematik:** yeni hesap yok.
  - Üçgen, eğimin kullandığı kesin işaretli içerde olma sınamasıdır (`orient2d`; kenarda ve köşede iki düzlem
    buluşur).
  - En uzun kenar ve en yakın köşe plan uzaklıklarıdır.

## Karar

- **Kot cevabının yeni alanları** (`TerrainHeight`), yer tek bir üçgenin içindeyse; kenarda ve köşede eğim gibi
  verilmez:
  - `corners`: üçgenin köşeleri (x, y, z), x'e sonra y'ye göre sıralı. Sıra iki üçgenlemenin köşe sırasından
    bağımsızdır.
  - `longest`: üçgenin en uzun plan kenarı.
  - `corner`: yerden en yakın köşeye plan uzaklığı.
- **`interpolation: linear`:** cevapta, kot ya da kesit sorulup yüzey kurulduğunda yazılır. Kot üçgenin düzleminde
  okunur; dışarıda ve boşlukta kot yoktur, dışa uzatılmaz (yer alanı zaten söylüyordu).
- **Kaynak sürümü** cevabın `surfaces[0].digest`'idir (kaynakların özeti, ADR 0900); ayrıca yazılmadı.
- **Çekirdek:** `Surface::strictly_in` eğimin içerde sınamasını ayrı verir; `slope_at` onu kullanır.
- **Masaüstü:** pencerelerde tek bir yerin kot sorgusu yok. Cevap komutun, Python'un ve MCP'nin cevabıdır.

## Ölçüm

- **Bağımsız başvuru** `road_terrain_command_cases.py` (KentOS kodu yok): Triangle'ın üçgenlemesinde kesirlerle
  içerde sınaması, köşeler, kenar ve uzaklık kendi hesabıyla.
- **`road.terrains.query`** 20 durumda yeni alanlar karşılaştırılıyor. "Kotlar: içeride, boşlukta, sınırın dışında"
  durumu:
  - (10, 10): en uzun kenar 13,342 m, en yakın köşe 5,000 m;
  - (20, 15): kenarda, üçgen verilmez.
- Bir köşenin kotu 1 mm değiştirilince durum düşüyor.
- Öbür arazi durumları değişmedi (`road.terrains.*`).
