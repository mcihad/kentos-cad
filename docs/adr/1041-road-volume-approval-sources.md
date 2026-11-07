# 1041 — Onaylı kübaj neyden hesaplandığını saklar

Tarih: 2026-11-07. Durum: kabul (Y106). Kapsam ROAD-VOL-01, ADR 0906 §5 (kübaj onayı), ADR 0950 (onay geçmişi), ADR
1040 (kübajın kaynakları).

## Sorun

ADR 1040 bugünkü hesabın kaynaklarını ve algoritmasını verdi. Saklanan tek hesap ise onaydır (`VolumeApprovalDef`):
rakamlar, satırlar, gün, not, tanımın sürümü. Onay neyden hesaplandığını saklamıyordu.

Onaydan sonra aks ya da arazi değişince sorgu rakamların farkını söylüyordu (`approval.changed`). Farkın nereden
geldiğini söylemiyordu: aks mı değişti, arazi mi, tip kesit mi? Yapım İşleri Genel Şartnamesi Md. 28/2'ye göre
imzalayan, rakamla birlikte "yapılan hesapların doğruluğunu" kabul eder (ADR 1040'ın araştırması). İmzalanan rakam
neyden hesaplandığını taşımıyorsa bu kabul sonradan sınanamaz.

## Araştırma (7 Kasım 2026)

- **ADR 1040'ın kaynakları** (Yapım İşleri Genel Şartnamesi Md. 28, 39, 40): metraj ve hesaplar birlikte tutulur ve
  imzalanır; kesin hesapta geçici hakedişin rakamlarına itibar edilmez.
- **ADR 0950:** onay hiç değiştirilmez ve silinmez; biten onay geçmişe olduğu gibi yazılır. Kaynaklar onayın parçası
  olursa geçmişteki her onay kendi kaynaklarıyla kalır.
- **Matematik:** yok. Karşılaştırma sürümlerin ve özetlerin eşitliğidir.

## Karar

- **`VolumeApprovalDef.sources` ve `algorithm`:** `road.volumes.approve` onaylarken hesabın o anki kaynaklarını (ADR
  1040'ın `VolumeSources`'u) ve yönteminin algoritmasını yazar. Şema 204'ten önceki onaylarda yoktur; okunur, değişmez.
- **`VolumeApprovalReport.moved`:** sorgu, onayın kaynaklarını bugünkülerle karşılaştırır. Her fark bir satırdır:
  - `source`: `corridor`, `alignment`, `profile`, `template`, `superelevation`, `surface`, `algorithm`;
  - `id`: kaynağın kimliği (algoritmada adı);
  - `then` ve `now`: onaydaki ve bugünkü sürüm (yüzeyde özet, algoritmada sürüm). Yoksa o an kaynak değildir.
- **Onayın kaynakları bilinmiyorsa** (eski onay) liste boştur. Pencere bunu ayrıca söyler.
- **KCAD şema 204:** onaydaki iki alan (Dalda 197 olarak yazıldı; main'le birleşirken 193–199 Harita işlerine verilmişti, 204 oldu.). Yazıcı onlardan biri varken 204 yazar; 203 ve öncesi onları
  `unknown_field` sayar.
- **Kübaj penceresi:** onay notu onaydan beri değişen kaynakları sayar: "Onaydan beri değişen: koridor c-1 (1 → 2)."
  Arazi özetinin ilk sekiz hanesiyle (`df9eb6e9… → 2751c2db…`), kalkan kaynak "yok" ile yazılır.

## Ölçüm

- **KCAD:**
  - Örnek `road-approval-sources.json/.kcad` üç onay taşıyor: geçerli onay kaynak ve algoritmayla; kaldırılan onay
    eski aks ve araziyle, `built` dahil; ilk onay şema 204'ten önceki gibi.
  - `broken/road-approval-sources-in-schema-203.kcad` `unknown_field` veriyor. En yeni + 1 dosyası
    `schema-version-205.kcad`.
  - Başvuru yazıcısı ve bağımsız okuyucu (`kcad.py`) 1654 dosyada tutarlı; Rust'ın 242 KCAD testi geçiyor (geçerli
    örnek 227 → 228).
- **Bağımsız başvuru** (`road_volume_command_cases.py`, KentOS kodu yok): `approved(signed=True)` onayı komutun
  yazdığı gibi kurar; `moved` farkları türe göre sayar.
- **`road.volumes.approve`:** 24 durum, komutun yazdığı 12 onay kaynak ve algoritmayla.
- **`road.volumes.query`:** 54 durum, 4 yeni:
  - koridor yeniden yazıldı, rakam aynı: `corridor c-1 (1 → 2)`, `changed` yanlış;
  - koridor sıklaştı ve yeniden yazıldı: aynı satır, `changed` doğru;
  - onayın arazisi ilk zemin noktasından önceki çizimdi: `surface t-1 (df9eb6e9… → 2751c2db…)`;
  - onaylayan algoritmanın eski sürümü: `algorithm kentos.end-area (0 → 1)`.
- **Bütün `road.*` dosyaları** Rust'ta geçiyor; eski onaylı kurulumların sonucu değişmedi.
- **Sözleşme testi:** `an_approvals_sources_and_algorithm_are_judged` (algoritma biçimi, boşluklu kimlik, büyük
  harfli özet, 257 tip kesit).
- **Masaüstü testleri:**
  - Pencereden onaylandı, sonra koridor sıklaştırıldı: onay `kentos.end-area/1` ve koridorun 1. sürümünü saklıyor; not
    "Onaydan beri değişen: koridor c-1 (1 → 2)." diyor.
  - Eski onay "kaynakları saklamıyor (eski onay)" diyor; kalkan tip kesit ve arazinin özeti yazılıyor.
- **Resimler** `.run/gui-evidence/yol-y106/`: açık 1440 px, koyu 1100 px.
