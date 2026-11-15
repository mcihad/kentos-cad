"""Dijital proje yönetmeliğinin denetimleri (catalog/standards/dijital-proje; plan/bim-plan.md K10, docs/adr/8063).

Kural Python'dadır, ölçü KentOS'tadır: çizimin olguları (``facts.project``) Rust'ta bir kez ölçülür, bu paket yalnız
karar verir. Masaüstünde ve bulut işinde aynı betik çalışır.

- ``kapsam``: dijital teslim kapsamı; bu projede PDF/A, IFC, modelden PDF/A ve e-PYS ne zaman, hangi maddeyle.
- ``ek34``: EK-3 ve EK-4 denetimi; mahal numarası ve adı, katman, tarama, aks çizgisi, ölçü yazısı, blok adı.
- ``turetim``: paftaların modelden üretildiği; eski kesit ve görünüş, model katmanında serbest çizim, kopmuş bağ.
- ``paket``: 2B ruhsat paketinin dosyaları; PDF/A, bir dosya bir pafta, A4 ve katları, EK-2 adı.
- ``mimari``: Mimari denetimin standart kuralları bir arada; ``ruhsat``: ruhsat yayınının kapısı.
- ``denetim``: KentOS'un denetimine katılan kurallar (``@ext.audit``).
"""

__all__ = ["ek34", "kapsam", "mimari", "paket", "ruhsat", "turetim"]
