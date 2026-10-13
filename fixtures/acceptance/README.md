# Kabul senaryoları

Planın §23.1'inin uçtan uca kabul çizimleri ([ADR 0252](../../docs/adr/0252-first-acceptance-drawing.md)). Bir senaryo veridir
(`kentos.acceptance` v1); arayüz, yazılı komut (masaüstünün ajan bağlantısı), Python ve MCP aynı dosyayı oynatır ve her adımın
beklentisini aynı hoşgörüyle tutar.

| Dosya | Ne | Yazan |
|---|---|---|
| `v1/room.json` | Oda (K01–K04), ilişkili tarama, duvarlar (K02), iki A3 pafta (K11, K12), DXF ve PDF, başka ortamda açma | `scripts/fixtures/acceptance_room.py` |

Adımlar:

- `command`, `input`, `as`: katalogun komutu; `$ad` önceki bir adımın adladığı kimlik, `$layer` etkin katman.
- `undo`: geri alınan adımın adı.
- `reopen`: kaydet ve aç.
- `export`: `dxf` ya da `pdf`.

Beklentiler:

- `objects`: nesnelerin alanlarının alt kümesi.
- `measures`: alan ve uzunluk.
- `count`: nesne sayısı.
- `sheets`: kitabın paftaları.
- `dxf`, `pdf`: dosyanın bağımsız okunan gerçekleri: `python/tests/acceptance_files.py` ve Rust ikizi
  `crates/native/plot/tests/support/files.rs`.

Beklentiler KentOS kodu olmadan hesaplanır:

```bash
python3 scripts/fixtures/acceptance_room.py --check
```
