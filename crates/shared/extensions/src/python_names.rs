//! Python package names that imitate well-known ones (plan/eklenti.md ADIM 22A, the dependency gate's "names that
//! imitate known ones"; docs/adr/8057 §81): `reqeusts` for `requests`, `shapley` for `shapely`. A package index takes
//! any free name, and a name one letter from a popular package's is how a mistyped `uv add`, or a name an AI model
//! made up, installs someone else's code. `kentos kek verify`, the install review and the catalog's package gate say
//! it (a warning: an honest small package may sit one letter from a famous one, `pyaml` beside `pyyaml`).
//!
//! The list is the most installed packages of the index and the geospatial and data packages KentOS's extensions
//! use, by their normalised names (PEP 503); it is data, kept short on purpose: a name not in it is never judged.

use crate::python::normalized;

/// Well-known names, normalised (lower case, `-` for runs of `-`, `_`, `.`).
const KNOWN: &[&str] = &[
    // The index's most installed.
    "aiohttp",
    "aiosignal",
    "annotated-types",
    "anyio",
    "appdirs",
    "arrow",
    "attrs",
    "babel",
    "backoff",
    "bcrypt",
    "beautifulsoup4",
    "black",
    "boto3",
    "botocore",
    "cachetools",
    "celery",
    "certifi",
    "cffi",
    "chardet",
    "charset-normalizer",
    "click",
    "colorama",
    "coverage",
    "cryptography",
    "cycler",
    "cython",
    "dask",
    "decorator",
    "defusedxml",
    "distlib",
    "django",
    "dnspython",
    "docutils",
    "duckdb",
    "faker",
    "fastapi",
    "filelock",
    "flake8",
    "flask",
    "fonttools",
    "frozenlist",
    "fsspec",
    "google-auth",
    "greenlet",
    "grpcio",
    "gunicorn",
    "h11",
    "h5py",
    "httpcore",
    "httpx",
    "huggingface-hub",
    "hypothesis",
    "idna",
    "imageio",
    "importlib-metadata",
    "iniconfig",
    "ipython",
    "isort",
    "itsdangerous",
    "jinja2",
    "jmespath",
    "joblib",
    "jsonschema",
    "keras",
    "keyring",
    "kiwisolver",
    "lxml",
    "markdown",
    "markupsafe",
    "matplotlib",
    "more-itertools",
    "msgpack",
    "multidict",
    "mypy",
    "networkx",
    "nltk",
    "numba",
    "numpy",
    "oauthlib",
    "openai",
    "openpyxl",
    "orjson",
    "packaging",
    "pandas",
    "paramiko",
    "pendulum",
    "pillow",
    "pip",
    "platformdirs",
    "plotly",
    "pluggy",
    "polars",
    "protobuf",
    "psutil",
    "psycopg",
    "psycopg2",
    "psycopg2-binary",
    "pyarrow",
    "pyasn1",
    "pycparser",
    "pydantic",
    "pydantic-core",
    "pygments",
    "pyjwt",
    "pylint",
    "pymongo",
    "pymysql",
    "pynacl",
    "pyparsing",
    "pytest",
    "python-dateutil",
    "python-docx",
    "python-dotenv",
    "pytz",
    "pyyaml",
    "pyzmq",
    "redis",
    "regex",
    "reportlab",
    "requests",
    "rich",
    "rsa",
    "ruff",
    "s3transfer",
    "scikit-image",
    "scikit-learn",
    "scipy",
    "seaborn",
    "setuptools",
    "simplejson",
    "six",
    "sniffio",
    "soupsieve",
    "sqlalchemy",
    "starlette",
    "statsmodels",
    "sympy",
    "tabulate",
    "tenacity",
    "tensorflow",
    "tomli",
    "toolz",
    "torch",
    "tornado",
    "tqdm",
    "typing-extensions",
    "tzdata",
    "ujson",
    "urllib3",
    "uvicorn",
    "virtualenv",
    "websockets",
    "werkzeug",
    "wheel",
    "wrapt",
    "xlrd",
    "xlsxwriter",
    "xmltodict",
    "yarl",
    "zipp",
    // Geospatial, survey and drawing.
    "affine",
    "branca",
    "cartopy",
    "ezdxf",
    "fiona",
    "folium",
    "gdal",
    "geojson",
    "geopandas",
    "geopy",
    "laspy",
    "mercantile",
    "netcdf4",
    "osmnx",
    "owslib",
    "pdal",
    "pygeos",
    "pyogrio",
    "pyproj",
    "pyshp",
    "pystac",
    "rasterio",
    "rioxarray",
    "rtree",
    "shapely",
    "tifffile",
    "trimesh",
    "xarray",
];

/// The shortest name compared: below it one letter is too much of the name.
const SHORTEST: usize = 5;

/// The well-known name `name` is one letter, or two swapped letters, from (after normalising), when `name` is not one
/// itself; none for a name under five letters.
pub fn resembles_known(name: &str) -> Option<&'static str> {
    let name = normalized(name);
    if KNOWN.contains(&name.as_str()) {
        return None;
    }
    KNOWN
        .iter()
        .copied()
        .find(|known| crate::resemblance::near(&name, known, SHORTEST))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A letter missing, added, changed or two swapped is said with the name it imitates; a known name, another word
    /// and a short name are not.
    #[test]
    fn a_name_a_letter_from_a_known_one_is_said() {
        for (typed, known) in [
            ("reqeusts", "requests"),
            ("request", "requests"),
            ("shapley", "shapely"),
            ("Num_Py", "numpy"),
            ("pandass", "pandas"),
            ("python-dateutils", "python-dateutil"),
            ("geopandsa", "geopandas"),
        ] {
            assert_eq!(resembles_known(typed), Some(known), "{typed}");
        }
        for honest in [
            "requests",
            "Shapely",
            "PyYAML",
            "kentos-vitrin",
            "rich",
            "attr",
            "numpyro",
        ] {
            assert_eq!(resembles_known(honest), None, "{honest}");
        }
        // The list is normalised and has no name twice.
        for (i, name) in KNOWN.iter().enumerate() {
            assert_eq!(normalized(name), *name);
            assert!(!KNOWN[..i].contains(name), "{name}");
        }
    }
}
