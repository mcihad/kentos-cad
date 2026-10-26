"""Addressed files read by Python's own HTTP (docs/adr/0645 §5).

The native extension carries no HTTP client; a point cloud, a raster or a
NetCDF, GRIB or Zarr data set at an address is read through this reader,
registered once when :mod:`kentos` is imported: HTTP ranges with the
address's version kept (``ETag``, else ``Last-Modified``), a store's objects
whole, a passing failure tried three times. An S3 bucket is read as a public
address here (the desktop and the MCP server sign it with the user's AWS
credentials).
"""

from __future__ import annotations

import time
import urllib.error
import urllib.request

_AGENT = "KentOS-CAD-Python"
_TIMEOUT = 30


def _get(url: str, headers: dict[str, str]):
    request = urllib.request.Request(url, headers={"User-Agent": _AGENT, **headers})
    return urllib.request.urlopen(request, timeout=_TIMEOUT)


def _tried(url: str, work):
    wait = 0.25
    last: Exception | None = None
    for _ in range(3):
        try:
            return work()
        except urllib.error.HTTPError as e:
            e.close()
            if e.code < 500:
                raise OSError(f"“{url}” okunamadı: sunucu {e.code} dedi.") from e
            last = e
        except urllib.error.URLError as e:
            # A name not found or a refused connection does not pass; a dropped one or a timeout may.
            if not isinstance(e.reason, (TimeoutError, ConnectionResetError, ConnectionAbortedError)):
                raise OSError(f"“{url}” okunamadı: {e.reason}.") from e
            last = e
        except (TimeoutError, ConnectionResetError, ConnectionAbortedError) as e:
            last = e
        time.sleep(wait)
        wait *= 4
    raise OSError(f"“{url}” okunamadı: {last}.")


class Http:
    """The reader the native extension calls back."""

    def size(self, url: str) -> tuple[int, str | None]:
        def work():
            with _get(url, {"Range": "bytes=0-0"}) as r:
                if r.status != 206:
                    raise OSError(
                        f"“{url}”: sunucu parça parça okumaya (HTTP Range) izin vermiyor; "
                        "dosyayı indirip bağlı dosya olarak ekleyin."
                    )
                whole = (r.headers.get("Content-Range") or "").rsplit("/", 1)[-1].strip()
                if not whole.isdigit():
                    raise OSError(f"“{url}”: sunucu dosyanın boyunu (Content-Range) söylemiyor.")
                return int(whole), r.headers.get("ETag") or r.headers.get("Last-Modified")

        return _tried(url, work)

    def read(self, url: str, offset: int, length: int, version: str | None) -> bytes:
        def work():
            with _get(url, {"Range": f"bytes={offset}-{offset + length - 1}"}) as r:
                now = r.headers.get("ETag") or r.headers.get("Last-Modified")
                if version is not None and now is not None and now != version:
                    raise OSError(f"“{url}” açıldıktan sonra değişmiş; yeni sürümü yeniden açılarak okunur.")
                data = r.read()
                if r.status != 206 or len(data) != length:
                    raise OSError(f"“{url}”: sunucu {length} bayt yerine {len(data)} bayt verdi.")
                return data

        return _tried(url, work)

    def object(self, url: str) -> bytes | None:
        def work():
            try:
                with _get(url, {}) as r:
                    return r.read()
            except urllib.error.HTTPError as e:
                e.close()
                # An object never written (an S3 bucket that does not list its keys answers 403).
                if e.code in (403, 404):
                    return None
                raise

        return _tried(url, work)
