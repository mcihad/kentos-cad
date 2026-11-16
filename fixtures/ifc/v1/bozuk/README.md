# Bilerek bozulmuş IFC dosyaları

`cad.ifc.inspect`'in ret ve bulgu durumları için (ADR 8073 §6). `scripts/fixtures/ifc_inspect_cases.py` yazar; elle
değiştirilmez.

| Dosya | Nasıl yapıldı | Beklenen |
|---|---|---|
| `step-degil.ifc` | Bir satırlık düz metin | `ifc_unreadable` |
| `bilinmeyen-sema.ifc` | Kaynak dosyanın şeması `IFC9` yapıldı | `ifc_unreadable` |
| `olmayan-basvuru.ifc` | Pencerenin gösterim başvurusu dosyada olmayan `#999999` yapıldı | okunur; pencerenin kaydında ve satırında `reference_missing` |
| `ayni-globalid.ifc` | Pencereye duvarın GlobalId'si verildi | okunur; pencerenin kaydında ve satırında `global_id_duplicate` |

**Kaynak ve lisans:** son üçü `fixtures/ifc/v1/disaridan/bsi-ifc4-wall-with-opening-and-window.ifc`'in değiştirilmiş
kopyalarıdır: (C) buildingSMART International Ltd., Creative Commons Attribution 4.0 International (CC BY 4.0,
<https://creativecommons.org/licenses/by/4.0/>); kaynağı ve özgün hâli [../disaridan/README.md](../disaridan/README.md)'de.
Yapılan değişiklik yukarıdaki tabloda yazar; başka bir şey değiştirilmedi.
