"""Dijital proje yönetmeliğinin denetimleri (catalog/standards/dijital-proje; plan/bim-plan.md K10, docs/adr/8063).

Kural Python'dadır, ölçü KentOS'tadır: çizimin olguları (``facts.project``) Rust'ta bir kez ölçülür, bu paket yalnız
karar verir. Masaüstünde ve bulut işinde aynı betik çalışır.

- ``kapsam``: dijital teslim kapsamı; bu projede PDF/A, IFC, modelden PDF/A ve e-PYS ne zaman, hangi maddeyle.
- ``denetim``: KentOS'un denetimine katılan kurallar (``@ext.audit``).
"""

__all__ = ["kapsam"]
