"""KentOS'un standart kural paketleri Python'dan (catalog/standards; plan/bim-plan.md K10, docs/adr/8063).

Ruhsat takımı penceresinin ve denetimin çalıştırdığı kural burada da aynıdır: paket Python betiğidir, KentOS ona veri
verir. Paketler ``KENTOS_STANDARDS_DIR`` altında, yoksa KentOS deposunun ``catalog/standards`` klasöründe aranır.

    >>> from kentos import standards
    >>> sonuc = standards.kapsam({"kullanim": "konut", "il_nufusu": "650000", "belediye_nufusu": "400000"},
    ...                          construction_area=1850.0, storeys_above=4)
    >>> [(s["title"], s["words"], s["from"]) for s in sonuc["rows"]]
"""

from __future__ import annotations

import importlib
import os
import sys
from pathlib import Path
from types import ModuleType
from typing import Any


def package(name: str) -> Path:
    """Standart paketin klasörü (``dijital-proje``); bulunamazsa nedeni ve çözümüyle hata."""
    places = []
    if os.environ.get("KENTOS_STANDARDS_DIR"):
        places.append(Path(os.environ["KENTOS_STANDARDS_DIR"]) / name)
    places.append(Path(__file__).resolve().parents[2] / "catalog" / "standards" / name)
    for place in places:
        if (place / "manifest.json").is_file():
            return place
    raise FileNotFoundError(
        f"Standart kural paketi “{name}” bulunamadı: KentOS'un catalog/standards klasörünü KENTOS_STANDARDS_DIR ile "
        "gösterin."
    )


def load(name: str, module: str) -> ModuleType:
    """Paketin bir modülü, paketin ``src``'si yola eklenerek içe aktarılmış."""
    src = str(package(name) / "src")
    if src not in sys.path:
        sys.path.insert(0, src)
    return importlib.import_module(module)


def kapsam(particulars: dict[str, str], today: str | None = None, *, construction_area: float | None = None,
           storeys_above: int | None = None, storeys_below: int = 0) -> dict[str, Any]:
    """Dijital teslim kapsamı (``dijital_proje.kapsam.karar``): künyenin değerleri (``kullanim``, ``il_nufusu`` …;
    adları ``kentos_digital_project::scope``'un), kararın günü (``YYYY-AA-GG``; verilmezse bugün) ve çizimin
    ölçtüğü toplam yapı inşaat alanı ile katlar. Satırlar, okunamayan girdiler ve kararın okuduğu değerler döner."""
    permit: dict[str, Any] = {"particulars": dict(particulars), "constructionArea": construction_area}
    permit["storeys"] = None if storeys_above is None else {"above": storeys_above, "below": storeys_below}
    return load("dijital-proje", "dijital_proje.kapsam").karar(permit, today)
