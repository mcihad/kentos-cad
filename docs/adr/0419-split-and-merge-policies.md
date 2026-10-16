# ADR 0419: Alanın bölme ve birleştirme kuralı

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-AA`; `plan/form.md` FORM-DATA-21.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0065 ve 0069 (alan işlemleri), 0140 (Parçala), 0143 (çok parçalı alan), 0296
  (Kır ve Birleştir'de taşınanlar); CLAUDE.md §23.2 (toplam korunur, artık rastgele son parçaya verilmez).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir parsel bölününce her parça eski parselin bütün özniteliklerini kopyalıyordu: hisse, tapu alanı, değer iki katına çıkıyordu;
birleşince ilkinin değerleri kalıyordu. QGIS'te alanın “bölme kuralı” (varsayılan değer, kopyala, boşalt, oranla) ve “birleştirme
kuralı” (varsayılan, toplam, ortalama, …) vardır; ArcGIS'te de alan adının bölme ve birleştirme politikası.

## Karar

### 1. Kurallar

Katman alanının `split`'i (yoksa Kopyala) ve `merge`'i (yoksa İlkinin değeri):

| Bölününce | Anlamı |
|---|---|
| `copy` Kopyala | her parça değeri korur |
| `clear` Boşalt | parçalarda boş |
| `default` Varsayılan | parçalar alanın varsayılanını alır (yoksa boş) |
| `ratio` Alan oranında paylaştır | değer parçaların alanlarıyla (alan değilse uzunluklarıyla) paylaşılır |

| Birleşince | Anlamı |
|---|---|
| `first` İlkinin değeri | kalan nesnenin değeri |
| `clear` Boşalt | boş |
| `sum` Topla | dolu değerlerin kesin toplamı (hiç yoksa boş) |
| `min`, `max` | dolu değerlerin en küçüğü, en büyüğü (sayıları) |

Paylaştırma ve toplam tam sayı ve ondalık basamağı verilmiş ondalık sayı alanında olur, en küçük ve en büyük sayı alanlarında
(alanın kuralı). Değer alanın son basamağının tam birimleriyle hesaplanır: her parçaya toplam × ağırlık / ağırlıklar'ın tabanı,
kalan birimler birer birer en büyük kalanlara, eşitlikte önceki parçaya; ağırlıkların toplamı sıfırsa eşit. Eksi değer mutlak
değeriyle paylaştırılır. Paylaştırılan değer alanın bütün basamaklarıyla yazılır (`100.00`).

### 2. Nerede

`cad.entities.edit`'te: **bölme** işleri (Alan böl, Parçala, Kır, Parçalara ayır) bir nesnenin verisini taşıyan parçaları
(yerinde güncellenen, `keepData`'lı yerine geçen ve yeni parçası) iki ya da daha çoksa kuralları uygular; ağırlıklar parçaların
yazılış sırasıyla. **Birleştirme** işleri (Alan birleştir, Birleştir, Parçaları birleştir) verisini taşıyan tek parça kalınca onun
nesnesinin ve silinenlerin değerlerinden (o önce) birleştirir. Böylece bu araçların hepsi, Python ve MCP aynı kurala uyar.

### 3. Dosya ve pencere

`split` ve `merge` `.kcad` şema 60'ta (`broken/field-split-in-schema-59.kcad`); Alanlar penceresinde seçili alan için **Nesne
bölününce** ve **Nesneler birleşince** (yalnız alanın türüne uyan seçenekler).

## Denetimler

- `scripts/fixtures/field_policy_cases.py --check`: `cad.entities.edit`'in 21 durumu (`cad.entities.edit.policies.json`): bölme
  oranları, kalanlar, eksi değer, varsayılan, boşaltma, verisini taşımayan parça, tek parça, iki nesne, çizgi uzunlukları, birleştirme
  toplamı, en büyük ve en küçük, boş değerler, iki parça kalan birleştirme, başka iş; KentOS kodu olmadan.
- `scripts/fixtures/layer_field_cases.py --check`: alan kuralının dört durumu; `kentos-kcad`, `kcad_v2_reference.py --check`.
- Masaüstü: `the_fields_window_writes_the_split_and_merge_policies`; resim `form-alan-politika`.

## Sonuçlar

- Bölünen parselin hissesi ve tapu alanı parçalara toplamı korunarak dağılır; birleşenler toplanır.
- Açık: ortalama (yuvarlama kuralı gerekir), metinleri birleştirme, ağırlık olarak başka bir alan.
