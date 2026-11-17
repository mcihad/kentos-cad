//! Python package names that imitate well-known ones (plan/eklenti.md ADIM 22A, the dependency gate's "names that
//! imitate known ones"; docs/adr/8057 §81): `reqeusts` for `requests`, `shapley` for `shapely`. A package index takes
//! any free name, and a name one letter from a popular package's is how a mistyped `uv add`, or a name an AI model
//! made up, installs someone else's code. `kentos kek verify`, the install review and the catalog's package gate say
//! it (a warning: an honest small package may sit one letter from a famous one, `pyaml` beside `pyyaml`).
//!
//! The list is the most installed packages of the index and the geospatial and data packages KentOS's extensions
//! use, by their normalised names (PEP 503); it is data, kept short on purpose: a name not in it is never judged.

pub use crate::python::normalized;

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

/// Packages an extension of KentOS commonly needs and what each is for (docs/adr/8057 §105, D18): the blueprint offers
/// them, so an agent need not guess a name (models made up a package's name in at least 5.2% of their answers). Each is
/// one of [`KNOWN`]'s.
pub const SUGGESTED: &[(&str, &str)] = &[
    (
        "shapely",
        "geometri işlemleri: kesişim, tampon, birleşim, alan (GEOS)",
    ),
    ("pyproj", "koordinat sistemleri ve dönüşümler (PROJ)"),
    ("numpy", "sayı dizileri ve vektör hesabı"),
    ("pandas", "tablolar: süzme, gruplama, birleştirme"),
    (
        "scipy",
        "bilimsel hesap: interpolasyon, istatistik, en iyileme",
    ),
    ("networkx", "ağ ve çizge analizi: en kısa yol, bağlılık"),
    ("pyogrio", "coğrafi veri dosyalarını okuma ve yazma (GDAL)"),
    ("rasterio", "raster okuma ve yazma (GDAL)"),
    ("openpyxl", "Excel (.xlsx) okuma ve yazma"),
    ("python-docx", "Word (.docx) belgesi yazma"),
    ("reportlab", "PDF belgesi yazma"),
    ("matplotlib", "grafik ve şekil çizimi"),
    ("httpx", "HTTP istemcisi (eklenti ağ yeteneği ister)"),
];

/// The packages the blueprint offers and what each is for (docs/adr/8057 §105): the suggested ones; when the
/// organisation's policy lists the only packages allowed (`allowed`, names or requirements), those of them first, then
/// the others it allows.
pub fn suggested(allowed: Option<&[String]>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = SUGGESTED
        .iter()
        .map(|(name, purpose)| ((*name).to_owned(), (*purpose).to_owned()))
        .collect();
    let Some(allowed) = allowed else {
        return out;
    };
    // A requirement's name: what stands before its version or its markers (`numpy>=2`, `pyproj; python_version…`).
    let allowed: Vec<String> = allowed
        .iter()
        .map(|a| {
            normalized(
                a.split(['=', '<', '>', '!', '~', ';', '[', ' '])
                    .next()
                    .unwrap_or(a),
            )
        })
        .filter(|a| !a.is_empty())
        .collect();
    out.retain(|(name, _)| allowed.contains(name));
    for name in allowed {
        if !out.iter().any(|(n, _)| *n == name) {
            out.push((name, "kurumun izin verdiği paket".to_owned()));
        }
    }
    out
}

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

    /// The suggestions are known names, each once, each with what it is for; the policy's list keeps those it allows
    /// and adds the others it names (docs/adr/8057 §105).
    #[test]
    fn the_suggested_packages_are_known_and_kept_to_the_policy() {
        for (i, (name, purpose)) in SUGGESTED.iter().enumerate() {
            assert!(KNOWN.contains(name), "{name}");
            assert!(!SUGGESTED[..i].iter().any(|(n, _)| n == name), "{name}");
            assert!(!purpose.is_empty());
        }
        assert_eq!(suggested(None).len(), SUGGESTED.len());
        let allowed = [
            "Shapely>=2".to_owned(),
            "numpy".to_owned(),
            "kurum_paketi".to_owned(),
        ];
        let names: Vec<String> = suggested(Some(&allowed))
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, ["shapely", "numpy", "kurum-paketi"]);
    }
}
