"""KentOS CAD from Python (docs/adr/0131).

``kentos.cad`` holds everything: a drawing opened without a window
(:class:`kentos.cad.Document`), every product command of the catalog under
its own name with its types (``kentos.cad.polygon.create``,
``kentos.cad.entities.set``, ``kentos.cad.project.share`` …), and the
KentOS server (:class:`kentos.cad.Connection`). ``kentos.services`` adds the
map services (docs/adr/0208): ready basemaps, map service layers and a data
service's objects taken into a layer. ``kentos.network`` asks the project's
networks (docs/adr/0209): routes, service areas, nearest facilities, cost
matrices, traces and Denetle. ``kentos.temporal`` reads the temporal layers
(docs/adr/0210): their objects' times, what a moment or a period shows, the
time slider's positions, and makes and applies scenarios. ``kentos.address``
searches the project's address data (docs/adr/0229): an address's parts, its
candidates and a place's address. A file at an address is read by HTTP ranges
with Python's own ``urllib`` (docs/adr/0645).
"""

from . import _remote, address, cad, network, services, temporal
from ._native import __version__, set_remote

# Addressed files (a cloud, a raster, a NetCDF, GRIB or Zarr data set) are read with Python's own HTTP (docs/adr/0645).
set_remote(_remote.Http())

__all__ = ["__version__", "address", "cad", "network", "services", "temporal"]
