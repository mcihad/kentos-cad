# 3740 — Detay eşleştirmenin sürüm derlemesinde ölçümü

Tarih: 2026-11-09. plan/map-cad.md MAP-04, yedinci tur. Durum: taslak. ADR 2940'ın ve 3340'ın “Sonra”sındaki “sürüm
derlemesinde ölçüm”.

İlgili:
- [ADR 2940](2940-reconcile-grid-index.md): büyük aday kümesi için ızgara dizini ve debug ölçümü;
- [ADR 3340](3340-reconcile-arc-candidates.md): yaylı adaylar.

## Dayanak

- **CLAUDE.md §6.1 ve §20.4:** karşılaştırma donanım, derleme ve commit bilgisiyle yapılır; açılış ve modül süreleri
  üretim derlemesinde ölçülür.
- **KentOS bugün** (ADR 2940): `map.features.reconcile`'ın planı yalnız debug derlemesinde ölçülmüştü (40 000 küme
  1,02 s). Kullanıcının çalıştırdığı masaüstü ve sunucu sürüm derlemesidir; debug sayısı onun yerine geçmez.

## Karar

1. **Ölçüm** (aşağıda): aynı commit, aynı veri, aynı makinede debug ve sürüm derlemesi. Kod değişmez.
2. **Kayıt:** sürüm ölçümünün satırı kılavuz §2'dedir. `reconcile_large::timing` `#[ignore]` kalır; boyları
   `KENTOS_RECONCILE_SIZES` verir.
3. **Kapsam işareti:** ADR 2940'taki gibi **Harita**.

## Ölçüm

macOS, Apple M5, rustc 1.96.0; commit d28a9a657'nin üstü (çalışma ağacındaki değişiklikler eşleştirmeye dokunmuyor);
`reconcile_large::timing`, tek koşu, 9 Kasım 2026. Veri ADR 2940'ınki: ortak durumların çizimi ve doğusunda kümeler, her
kümede LiDAR ve ortofoto direği, beşte birinde üçüncü aday, onda birinde yanında var olan direk; kümeler 5 m arayla.
Plan süresi:

| Küme (aday) | Debug | Sürüm (`--release`) | Oran |
| --- | --- | --- | --- |
| 2 000 (4 400) | 0,050 s | 0,019 s | 2,6 |
| 10 000 (22 000) | 0,248 s | 0,050 s | 5,0 |
| 40 000 (88 000) | 1,042 s | 0,215 s | 4,8 |
| 100 000 (220 000) | 2,676 s | 0,567 s | 4,7 |

- İki derleme her boyda aynı grup sayısını verdi: küme sayısı artı ortak durumun 7 grubu.
- Debug sayıları ADR 2940'ınkilerle (0,054, 0,246, 1,02 s) aynı düzeyde: ADR 3340'ın yaylı aday yolu noktalı veriyi
  yavaşlatmadı.
- Sürümde süre aday sayısıyla doğrusala yakın büyüyor (10 kat veri, 11 kat süre).
- 2 000 kümede oran küçük: süre çizimin hazırlanması ve ızgaranın kurulması gibi sabit işlerle doluyor.

## Kanıt

- Ölçümün iki koşusu yukarıdaki tablodur; ikisinin de testi geçer (`test result: ok`).
- **Ortak durumlar, MCP, Python, masaüstü:** kod değişmedi.

## Sonra

- Gerçek bir çıkarımın adaylarıyla ölçüm: gerçek LiDAR ve ortofoto çıkarımı bekleniyor (yaylı ve alan adaylarıyla).
- ~~Ayrı inceleme kaydı (kararların öznitelik değil kayıt olarak tutulması)~~: [ADR 4140](4140-reconcile-decisions-as-review-notes.md).
  AI çıkarımı (MAP-04'ün kalanı).
- Web (TODOS §0.2): web'de eşleştirme WASM ise aynı ölçüm üretim paketinde.
