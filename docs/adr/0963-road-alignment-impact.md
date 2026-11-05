# 0963 — Aks değişikliğinin etkisi: km eşlemesi ve bağlı km'ler

Tarih: 2026-11-05. Durum: kabul (Y57). Kapsam ROAD-ALIGN-10.

## Sorun

Aksın bir PI'sını oynatmak, aksın üstündeki her km'nin anlamını değiştirir. Aksa km ile bağlanan şunlar vardır:

- boykesitin PVI'ları;
- kesit takımının aralığı;
- koridor bölgelerinin uçları;
- dever çizelgesinin satırları.

Bunların her biri bir yer adlandırır, ama o yer artık başka bir km'de ya da aksın üstünde değil.

ADR 0901 §9'un eşlemesi elemanları kimlikle eşler: bir eleman ya aynı kalır, ya kayar, ya değişir. Bir kurbun yarıçapı
büyüyünce önündeki aliyman da "değişir", çünkü boyu kısalmıştır. Oysa aliymanın büyük kısmı yerinde durur. Yalnız
sonu kısalmıştır, üstündeki PVI'lar da aynı yerdedir. Eleman eşlemesi bir km'nin hâlâ aksın üstünde olup olmadığını
söyleyemez.

ROAD-ALIGN-10: "Alignment edit ve reverse için segment/station mapping üret; ölçü, profil, corridor region ve stakeout
refs'e impact raporu ver."

## Araştırma

- **Civil 3D:**
  - Aks düzenlenince profiller, kesit hatları ve koridor "out of date" olur. PVI'lar km'leriyle kalır, yerleri kayar.
  - Hangi PVI'nın artık eğrinin ortasına düştüğünü söylemez; profil görünüşü yeniden çizilir ve kullanıcı bakar.
  - Station equation eklemek iç km'yi değiştirmez (ADR 0901 §9'un kuralı da budur).
  - Kaynaklar: Autodesk Civil 3D yardımı "About alignment and profile dynamic updates", forum "profile PVIs after
    alignment edit".
- **OpenRoads:** "Civil rules" ile bağlı elemanlar kendiliğinden güncellenir. Kopan kurala "rule broken" denir ama km
  düzeyinde bir rapor yoktur.

## Karar

### Matematik: geometrik eşleme

Eski aksın km'si s'nin yeri P(s)'dir. Bu yerin yeni aks üstündeki ayağı (en yakın dik izdüşümü) yeni km s′'yü ve
ofseti verir:

- **|ofset| ≤ tolerans** (öntanımlı 1 mm): s, s′'ye taşınır; kayma s′ − s'dir.
- **Aksı:** Yer aksın dışında kalmıştır (ya da ayağı yoktur, aksın ucunun ötesindedir). Bu km'ye bağlı olan
  **çözülmez**.

**Eski çizginin aralıkları** (`removed`):

- her eleman 64 noktada örneklenir;
- toleransı kestiği yerler ikiye bölmeyle bulunur;
- en büyük ofset altın oran aramasıyla inceltilir. Değeri iyi koşulludur, yeri değildir (ADR 0962'deki gibi); yeri
  verilmez.

**Yeni çizgi:** Aynı yol tersinden yürür; eski çizginin dışında kalan yeni aralıklar `added` olur.

**Aradaki aralıklar** (`runs`): yerinde kalan eski km'lerdir. Uçlarındaki kayma verilir; yarıçap büyüyünce
sonrakilerin hepsi aynı kadar kayar.

**Ortak uç:** İki aksın ortak ucu yerindedir. `locate` uçtaki bir yere "ucun ötesinde" der; 1e-7 m içinde çakışan uç
yine ayak sayılır. Böyle sayılmasaydı kurbu kaldırılan aksın ucunda boyu sıfır olan bir "yeni aralık" çıkıyordu.

### `road.alignments.impact` (salt okur)

- **Girdi:** aks ve olacak PI'lar; doğrulanma ve kurulma `road.alignments.edit` gibidir.
- **Paralel aks:** Reddedilir; kaynağını izler.
- **Çıktı:**
  - eleman eşlemesi (ADR 0901 §9);
  - `removed`, `added` ve `runs`;
  - taşınan ya da çözülmeyen her km adı (`refs`): boykesit PVI'sı, kesit takımının `from`/`to`'su ya da yazılı km'si,
    koridor bölgesinin uçları ve yazılı km'leri, dever satırı;
  - izleyen paralel akslar;
  - dokunulan koridorlardaki kübaj ve taşıma planları.
- **Kendiliğinden izleyenler:**
  - Dever çizelgesinin kurp kuralı PI'sını izler. Yalnız PI'nın kurbu kalktıysa çözülmez sayılır. Hiç kurbu olmayan
    bir PI'daki kural düzenlemeden önce de kurulmuyordu; etki sayılmaz.
  - Kesit takımının "çizgilerin kestiği yer" yöntemi yeni aksla yeniden kesişir; adlandırdığı km yoktur.
- **Uyarılar:**
  - `alignment_impact_unresolved`: yeri kalmayan km'ler, ilk 8'i adlarıyla;
  - `alignment_impact_approved`: dokunulan onaylı hesaplar.

### Pencere

Güzergâh penceresi, seçili aksın yazılan PI'ları kaydedilenden farklıysa etkiyi okur. Durum satırının altına bir cümle
yazar: yerinden kayan aralıklar ve en büyük ofset, km kayması, taşınan ve yerini yitiren km sayısı, eskiyecek hesaplar,
izleyen paralel akslar. Uyarılar ayrı satırlarda verilir.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_alignment_impact_command_cases.py` KentOS kodu kullanmaz:

- akslar `road_alignment_cases.py`'nindir (mpmath, klotoid serisi);
- ayak 0,5 m'lik ızgaranın **her** yerel en küçüğünden altın oranla inceltilir ve en yakını alınır;
- eleman başına 32 örnek (KentOS 64) ve ikiye bölme kullanılır.

20 durumun hepsi Rust'ta ve Python'da 1e-6 m içinde geçer.

**Referansın kendi hatası:** İlk sürüm ızgaranın tek en küçüğünü inceltiyordu. Kurbu kaldırılan aksta eski kurbun
noktası iki aliymana neredeyse eşit uzaklıktaydı ve ızgara yanlış olanı seçti. Referans en büyük ofseti 12,906176 m,
Rust 12,906147 m buldu.

Hangisinin doğru olduğu kapalı bir yapıyla bulundu: eski kurbun açıortayı kestiği noktanın iki aliyman doğrusuna
uzaklığı 30 basamakta 12,906146846 m'dir. Rust doğruydu.

| Değişiklik | Eski çizgide kayan | Yeni çizgide yeni | Sonrası |
|---|---|---|---|
| Yarıçap 150 → 180 | km 185,19 – 367,38 | km 185,19 – 365,96 | km'ler −1,42 m kayar, 8 km adı |
| Klotoidler 40 → 60 | km 188,28 – 364,30 | km 188,28 – 363,87 | |
| Bitiş aliyman boyunca %10 uzar | yok | km 669,39 – 709,09 | hiçbir km adı oynamaz |
| Başlangıç aliyman boyunca 20 m ileri | km 0 – 20,01 | yok | 13 km adı −20,01 m kayar |
| Kurp kaldırılır (kırık) | km 198,01 – 354,56, en çok 12,906 m | km 198,01 – 362,34 | km'ler +7,78 m kayar |
| Yarıçap 150 → 150,001, 1 cm tolerans | yok | yok | kayma 1e-6'yı aştığı için 8 km adı taşınır |

## Açık

- **Ters çevirme:** Eşlemesi `road.alignments.edit reverse`'te var (ADR 0901 §9). Ters çevirmede km adlarının
  dönüştürülmesi (s → L − s) bu raporda yok.
- **Diğer km adları:** Aplikasyon, istinat duvarı ve bariyer km'leri (koridor ya da aks üstünden) sonraki işe kaldı.
- **Yazma:** Taşınan km'lerin kaydedilirken kendiliğinden taşınması (PVI'ların s′'ye yazılması) ayrı bir karar. Şimdilik
  rapor söyler, kullanıcı karar verir.
