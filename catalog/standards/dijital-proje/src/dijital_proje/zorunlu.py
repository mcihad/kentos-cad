"""Modelin zorunlu alanları (docs/adr/8068; plan/bim-plan.md BIM-07, plan/ifc-uyum.md IFC-02): EK-6 ve EK-7'nin zorunlu
satırlarından modelin bazı nesnelerinde taşımadığı her alan, nesneleriyle. Kural modeli okumaz: KentOS'un ölçtüğü
olgulara (``facts.cad.modelGaps``: ``cad.arch.ifc``'nin yazacağı modelde, sınıfının her nesnesinde değeri olmayan
zorunlu satır) bakar.

- ``model_required`` — Md. 12/1, 13; EK-6, EK-7; EK-9 Tablo 9.1 satır 9: "zorunlu öznitelikler ve özellikler tanımlandı
  mı?" Eksik her satır bir hatadır; hiçbiri varsayılanla doldurulmaz, boş metin değer sayılmaz. Onarımı alanına göre
  söylenir: yangın dayanım sınıfı ve acil çıkış Öznitelikler'de (duvar stilinin, kapı ve pencere tipinin sınıfı
  nesnelerine geçer), malzemenin dayanımı Malzemeler'de, binanın yangın koruma sınıfı Mimari ayarları'nda.
"""

from __future__ import annotations

from typing import Any

RG = "RG 5.8.2026/33331"

#: The repairs by the path's end, the most specific first.
_ONARIM = (
    (".FireRating", "Öznitelikler › Yangın dayanımı'nı yazın (TS EN 13501-2: REI 120, EI 60); duvar stilinin, kapı ve "
                    "pencere tipinin sınıfını stil ve tip pencerelerinde verirseniz kendi sınıfı olmayan nesneleri taşır."),
    (".FireExit", "Öznitelikler › Acil çıkış'ı seçin: evet ya da hayır; kaçış yollarının geçtiği kapılar ve merdivenler "
                  "adaydır."),
    ("IfcMaterial.Name", "Nesnenin malzemesini verin; dayanımını Malzemeler penceresinde yazın (Beton_C25, Çelik_S355)."),
    ("Pset_BuildingCommon.FireProtectionClass", "Binanın yangın koruma sınıfını Mimari ayarları › Yangın ve performans'ta "
                                                "yazın."),
    ("Pset_BuildingCommon.ConstructionMethod", "Ruhsat takımı › Dijital teslim kapsamı'nda yapı türünü seçin (yeni, "
                                               "ilave, tadilat)."),
    ("PredefinedType", "Nesnenin ön tanımlı tipi yazılmıyor; tipi olan bir nesneyle (merdivende kol biçimi, çatıda biçim) "
                       "yeniden çizin ya da kayıp listesine bakın."),
)


def denetle(facts: dict[str, Any], data: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """``facts``: ``{"cad": {"modelGaps": [{class, classTr, path, tr, table, page, missing, of}]}}``."""
    cad = facts.get("cad") or {}
    out: list[dict[str, Any]] = []
    for g in cad.get("modelGaps") or []:
        uids = [u for u in g.get("missing") or [] if u]
        ek = "EK-7" if str(g["table"]).startswith("7.") else "EK-6"
        n = len(g.get("missing") or [])
        if uids:
            ne = f"{n}/{g['of']} {g['classTr'].lower()} nesnesinde {g['tr']} ({g['path']}) yok"
        else:
            ne = f"{g['classTr']}: {g['tr']} ({g['path']}) yok"
        out.append({
            "rule": "model_required",
            "code": "digital_required_field",
            "severity": "error",
            "message": f"{ne} — Dijital Proje Yön. {ek} Tablo {g['table']}",
            "repair": _onarim(g),
            "uids": uids,
            "at": [0.0, 0.0],
            "cite": f"{ek} Tablo {g['table']} · PDF s. {g['page']} · EK-9 Tablo 9.1 satır 9 · {RG}",
            "fix": None,
        })
    return out


def _onarim(g: dict[str, Any]) -> str:
    path = g["path"]
    for son, words in _ONARIM:
        if path.endswith(son) or path == son:
            return words
    return f"“{g['tr']}” ({path}) modelde yazılmıyor ya da çizimde verilmemiş; EK-6'nın {g['table']} tablosuna bakın."
