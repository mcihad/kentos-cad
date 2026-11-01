#!/usr/bin/env bash
# KentOS CAD geliştirme makinesini temiz bir Ubuntu'da (26.04 / 26.10) baştan kurar.
#
# Kurulanlar:
#   - derleme araçları ve masaüstünün (Iced + wgpu) sistem kitaplıkları
#   - GDAL, PROJ, OpenCASCADE (geliştirme paketleriyle), QGIS ve GRASS, LibreOffice, poppler, sqlite3,
#     PostgreSQL istemcisi (bağımsız başvuruların ve çapraz denetimlerin araçları)
#   - Rust (rust-toolchain.toml'daki sürüm, wasm32 hedefi, clippy, rustfmt) ve wasm-bindgen-cli 0.2.128
#   - Node.js 24 ve pnpm 11.24.0, Google Chrome (e2e ve resimler), GitHub CLI, Docker (PostGIS için)
#   - uv, Python ortamları: .run/py (SDK ve maturin), .run/geo-py (sistemin GDAL'ıyla pyarrow),
#     .run/pyref (laspy + LASzip), R ortamı .run/r (micromamba: fpc, dbscan, spatstat)
#   - OpenCASCADE başvuru aracı .run/occt_ref, deponun pnpm paketleri ve WASM paketleri
#   - ~/Projects/database/compose.yml ile PostGIS (postgis/postgis:18-3.6), kentos_cad veritabanı
#
# Kullanım (normal kullanıcıyla; gerektiğinde sudo sorar):
#   curl -fsSLO https://raw.githubusercontent.com/mcihad/kentos-cad/main/scripts/setup/ubuntu.sh   # depo özelse gh ile indirin
#   bash ubuntu.sh                 # hepsi
#   bash ubuntu.sh --skip gui,r    # bölüm atla: apt, gui, docker, chrome, node, rust, repo, python, r, occt, web, db, nvidia
#   bash ubuntu.sh --only rust     # yalnız bu bölümler
#
# Betik tekrar çalıştırılabilir: kurulu olanı atlar. Paket adı Ubuntu sürümünde yoksa uyarır ve sürer;
# sonunda kurulamayanları listeler.
set -euo pipefail

# --- Sürümler (depodaki dosyalarla aynı tutun) -----------------------------------------------------
NODE_MAJOR=24
PNPM_VERSION=11.24.0
WASM_BINDGEN_VERSION=0.2.128            # Cargo.toml: wasm-bindgen = "=0.2.128"
REPO_URL="https://github.com/mcihad/kentos-cad.git"
PROJECTS="${KENTOS_PROJECTS:-$HOME/Projects}"
REPO="$PROJECTS/kentos-cad"
DB_DIR="$PROJECTS/database"

# --- Bölümler --------------------------------------------------------------------------------------
ALL_SECTIONS="apt gui docker chrome node rust repo python r occt web db nvidia"
SKIP=""
ONLY=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip) SKIP="${2//,/ }"; shift 2 ;;
    --only) ONLY="${2//,/ }"; shift 2 ;;
    -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
    *) echo "Bilinmeyen seçenek: $1 (--help)"; exit 2 ;;
  esac
done
want() {
  local s="$1"
  if [[ -n "$ONLY" ]]; then [[ " $ONLY " == *" $s "* ]]; return; fi
  [[ " $SKIP " != *" $s "* ]]
}

FAILED=()
say()  { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
warn() { printf '\033[1;33m!! %s\033[0m\n' "$*"; }
have() { command -v "$1" >/dev/null 2>&1; }

if [[ $EUID -eq 0 ]]; then
  echo "Bu betiği root olarak değil, kendi kullanıcınızla çalıştırın (sudo kendisi sorar)."; exit 1
fi
. /etc/os-release
if [[ "${ID:-}" != "ubuntu" ]]; then warn "Ubuntu değil ($PRETTY_NAME); paket adları tutmayabilir."; fi
CODENAME="${VERSION_CODENAME:-$UBUNTU_CODENAME}"
sudo -v

# Paketleri toplu kurar; toplu kurulum düşerse tek tek dener, bulunamayanları not eder.
apt_install() {
  local missing=()
  for p in "$@"; do dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p"); done
  [[ ${#missing[@]} -eq 0 ]] && return 0
  if ! sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${missing[@]}"; then
    for p in "${missing[@]}"; do
      sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$p" \
        || { warn "kurulamadı: $p"; FAILED+=("apt:$p"); }
    done
  fi
}

# --- 1. Temel paketler ve sistem kitaplıkları -------------------------------------------------------
if want apt; then
  say "apt: temel araçlar, derleyiciler, masaüstü kitaplıkları"
  sudo apt-get update
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y software-properties-common
  sudo add-apt-repository -y universe >/dev/null 2>&1 || true
  sudo apt-get update
  apt_install \
    build-essential clang lld cmake ninja-build pkg-config git git-lfs curl wget ca-certificates gnupg \
    unzip zip xz-utils bzip2 jq ripgrep fd-find tree htop bc file util-linux zsh \
    libssl-dev libsqlite3-dev zlib1g-dev \
    `# Iced + wgpu + winit (X11 ve Wayland), yazı tipleri, Vulkan` \
    libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev libx11-xcb-dev libxcursor-dev \
    libxrandr-dev libxi-dev libxinerama-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
    libxcb-xfixes0-dev libgl1-mesa-dev libegl-dev libvulkan-dev vulkan-tools mesa-vulkan-drivers \
    libfontconfig-dev libfreetype-dev libharfbuzz-dev libdbus-1-dev libudev-dev \
    fonts-noto-core fonts-dejavu-core fonts-liberation \
    `# Python (pyo3, maturin, bağımsız başvurular)` \
    python3 python3-dev python3-venv python3-pip pipx \
    python3-numpy python3-mpmath python3-sympy python3-matplotlib python3-pil python3-jsonschema \
    python3-owslib python3-psycopg2 python3-openpyxl python3-pandas python3-shapely python3-geopandas \
    python3-pyogrio python3-httpx python3-zstandard python3-lz4 python3-scipy \
    `# GDAL, PROJ, OpenCASCADE (geliştirme paketleriyle)` \
    gdal-bin libgdal-dev python3-gdal proj-bin proj-data libproj-dev python3-pyproj \
    libocct-foundation-dev libocct-modeling-data-dev libocct-modeling-algorithms-dev \
    libocct-data-exchange-dev libocct-ocaf-dev \
    `# çapraz denetimlerin araçları` \
    sqlite3 poppler-utils postgresql-client imagemagick inkscape \
    gh
fi

# --- 2. QGIS, GRASS, LibreOffice -------------------------------------------------------------------
if want gui; then
  say "gui: QGIS (Python'uyla), GRASS, LibreOffice Calc"
  # QGIS 4.x qgis.org'un deposundan (Ubuntu'nunki geride kalır); depo bu sürüm için yoksa Ubuntu'nunki kurulur.
  if [[ ! -f /etc/apt/sources.list.d/qgis.sources ]] \
     && curl -fsSL "https://qgis.org/ubuntu/dists/$CODENAME/Release" >/dev/null 2>&1; then
    sudo install -m 0755 -d /etc/apt/keyrings
    sudo curl -fsSL -o /etc/apt/keyrings/qgis-archive-keyring.gpg https://download.qgis.org/downloads/qgis-archive-keyring.gpg
    sudo tee /etc/apt/sources.list.d/qgis.sources >/dev/null <<EOF
Types: deb deb-src
URIs: https://qgis.org/ubuntu
Suites: $CODENAME
Architectures: amd64
Components: main
Signed-By: /etc/apt/keyrings/qgis-archive-keyring.gpg
EOF
    sudo apt-get update
  elif [[ ! -f /etc/apt/sources.list.d/qgis.sources ]]; then
    warn "qgis.org'un deposu '$CODENAME' için henüz yok; Ubuntu'nun QGIS'i kurulacak (sürümü eski olabilir)."
  fi
  apt_install qgis python3-qgis qgis-plugin-grass grass libreoffice-calc
fi

# --- 3. Docker (PostGIS ve geçici test veritabanları) ----------------------------------------------
if want docker; then
  say "docker"
  if ! have docker; then
    sudo install -m 0755 -d /etc/apt/keyrings
    if curl -fsSL "https://download.docker.com/linux/ubuntu/dists/$CODENAME/Release" >/dev/null 2>&1; then
      curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor --yes -o /etc/apt/keyrings/docker.gpg
      echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $CODENAME stable" \
        | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
      sudo apt-get update
      apt_install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
    else
      warn "Docker'ın deposu '$CODENAME' için henüz yok; Ubuntu'nun docker.io paketi kuruluyor."
      apt_install docker.io docker-compose-v2 docker-buildx
    fi
  fi
  sudo systemctl enable --now docker || true
  if ! id -nG "$USER" | grep -qw docker; then
    sudo usermod -aG docker "$USER"
    warn "docker grubuna eklendiniz; oturumu kapatıp açana dek 'sudo docker' ya da 'sg docker' kullanılır."
  fi
fi
docker_cmd() { if docker info >/dev/null 2>&1; then docker "$@"; else sudo docker "$@"; fi; }

# --- 4. Google Chrome (web e2e, resimler, WebGPU denetimi) ------------------------------------------
if want chrome; then
  say "chrome"
  if ! have google-chrome; then
    tmp="$(mktemp -d)"
    curl -fsSL -o "$tmp/chrome.deb" https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "$tmp/chrome.deb" || FAILED+=("chrome")
    rm -rf "$tmp"
  fi
fi

# --- 5. Node.js ve pnpm ----------------------------------------------------------------------------
if want node; then
  say "node $NODE_MAJOR ve pnpm $PNPM_VERSION"
  if ! have node || [[ "$(node -p 'process.versions.node.split(".")[0]')" -lt $NODE_MAJOR ]]; then
    if curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" -o /tmp/nodesource_setup.sh; then
      sudo -E bash /tmp/nodesource_setup.sh && apt_install nodejs || FAILED+=("nodejs")
      rm -f /tmp/nodesource_setup.sh
    else
      apt_install nodejs npm
    fi
  fi
  if ! have pnpm || [[ "$(pnpm -v)" != "$PNPM_VERSION" ]]; then
    sudo npm install -g "pnpm@$PNPM_VERSION" || FAILED+=("pnpm")
  fi
fi

# --- 6. Rust -------------------------------------------------------------------------------------
if want rust; then
  say "rust (rustup), wasm32, clippy, rustfmt, wasm-bindgen-cli $WASM_BINDGEN_VERSION"
  if ! have rustup && [[ ! -x "$HOME/.cargo/bin/rustup" ]]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  fi
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
  # Depodaki rust-toolchain.toml'un sürümü; depo henüz yoksa bilinen sürüm.
  TOOLCHAIN="1.96.0"
  [[ -f "$REPO/rust-toolchain.toml" ]] && TOOLCHAIN="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$REPO/rust-toolchain.toml")"
  rustup toolchain install "$TOOLCHAIN" --profile minimal -c clippy -c rustfmt -c rust-analyzer -t wasm32-unknown-unknown
  rustup default "$TOOLCHAIN"
  if ! wasm-bindgen --version 2>/dev/null | grep -q "$WASM_BINDGEN_VERSION"; then
    cargo +"$TOOLCHAIN" install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --locked
  fi
fi
[[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"

# --- 7. Depo -------------------------------------------------------------------------------------
if want repo; then
  say "depo: $REPO"
  mkdir -p "$PROJECTS"
  if [[ ! -d "$REPO/.git" ]]; then
    if have gh; then
      gh auth status >/dev/null 2>&1 || gh auth login --hostname github.com --git-protocol https --web
      gh repo clone mcihad/kentos-cad "$REPO"
    else
      git clone "$REPO_URL" "$REPO"
    fi
  fi
  git -C "$REPO" lfs install --local >/dev/null 2>&1 || true
fi

# --- 8. Python ortamları ---------------------------------------------------------------------------
if want python; then
  say "uv ve Python ortamları (.run/py, .run/geo-py, .run/pyref)"
  if ! have uv; then curl -LsSf https://astral.sh/uv/install.sh | sh; fi
  export PATH="$HOME/.local/bin:$PATH"
  # Ubuntu'da paketi olmayan ya da eski olan kullanıcı paketleri (CLAUDE.md'deki betikler sistem Python'unu kullanır).
  for m in shapely geopandas pandas pyogrio openpyxl httpx; do
    python3 -c "import $m" 2>/dev/null || python3 -m pip install --user --break-system-packages "$m" || FAILED+=("pip:$m")
  done
  if [[ -d "$REPO" ]]; then
    mkdir -p "$REPO/.run"
    # SDK'nın ortamı: maturin, ezdxf (bağımsız DXF okuyucusu), mypy; kentos paketi pnpm py:test ile derlenir.
    [[ -x "$REPO/.run/py/bin/python" ]] || uv venv --python /usr/bin/python3 "$REPO/.run/py"
    uv pip install --python "$REPO/.run/py/bin/python" "maturin>=1.9.4,<2" "ezdxf==1.4.5" numpy mypy fonttools
    # Sistemin GDAL'ını gören ortam: GeoParquet ve çok boyutlu denetimler için pyarrow.
    [[ -x "$REPO/.run/geo-py/bin/python" ]] || uv venv --python /usr/bin/python3 --system-site-packages "$REPO/.run/geo-py"
    uv pip install --python "$REPO/.run/geo-py/bin/python" "pyarrow==26.0.0"
    # Nokta bulutu başvuruları: laspy + LASzip.
    [[ -x "$REPO/.run/pyref/bin/python" ]] || uv venv --python /usr/bin/python3 "$REPO/.run/pyref"
    uv pip install --python "$REPO/.run/pyref/bin/python" "laspy==2.7.0" "laszip==0.3.0" numpy
    # `uv run --with …` ile çağrılanlar (h5py, netCDF4, zarr, eccodes, esda, scikit-learn …) ilk çalıştırmada
    # uv'nin önbelleğine iner; burada önceden ısıtılır, ağ yoksa sonra iner.
    uv run --no-project --with h5py --with netCDF4 --with zarr --with numcodecs --with eccodes \
      --with esda --with libpysal --with pointpats --with scikit-learn --with kneed --with botocore \
      --with pyarrow --with zstandard --with lz4 --with mpmath --with pyproj --with shapely \
      python3 -c 'print("uv önbelleği hazır")' || warn "uv önbelleği ısıtılamadı (sorun değil)"
  fi
fi

# --- 9. R ortamı (micromamba, conda-forge) -------------------------------------------------------
if want r && [[ -d "$REPO" ]]; then
  say "R ortamı .run/r (fpc, dbscan, spatstat, jsonlite)"
  if [[ ! -x "$REPO/.run/r/bin/Rscript" ]]; then
    MM="$REPO/.run/mamba/bin/micromamba"
    if [[ ! -x "$MM" ]]; then
      mkdir -p "$REPO/.run/mamba"
      curl -fsSL https://micro.mamba.pm/api/micromamba/linux-64/latest | tar -xj -C "$REPO/.run/mamba" bin/micromamba
    fi
    MAMBA_ROOT_PREFIX="$REPO/.run/mamba" "$MM" create -y -p "$REPO/.run/r" -c conda-forge \
      r-base r-fpc r-dbscan r-spatstat.explore r-jsonlite || FAILED+=("r")
  fi
fi

# --- 10. OpenCASCADE başvuru aracı (tools/occt) ----------------------------------------------------
if want occt && [[ -f "$REPO/tools/occt/occt_ref.cpp" ]]; then
  say "OpenCASCADE başvuru aracı .run/occt_ref"
  inc=/usr/include/opencascade
  [[ -d "$inc" ]] || inc="$(dirname "$(find /usr/include -name Standard.hxx -print -quit 2>/dev/null)")"
  mkdir -p "$REPO/.run"
  g++ -std=c++17 -O2 "$REPO/tools/occt/occt_ref.cpp" -o "$REPO/.run/occt_ref" -I"$inc" \
    -lTKernel -lTKMath -lTKG2d -lTKG3d -lTKGeomBase -lTKGeomAlgo || FAILED+=("occt_ref")
fi

# --- 11. Web paketleri ve WASM ---------------------------------------------------------------------
if want web && [[ -d "$REPO" ]] && have pnpm; then
  say "pnpm install ve WASM paketleri"
  (cd "$REPO" && pnpm install --frozen-lockfile) || FAILED+=("pnpm install")
  (cd "$REPO" && pnpm wasm) || FAILED+=("pnpm wasm")
fi

# --- 12. Yerel PostGIS ve kentos_cad veritabanı ----------------------------------------------------
if want db; then
  say "PostGIS ($DB_DIR/compose.yml)"
  mkdir -p "$DB_DIR"
  if [[ ! -f "$DB_DIR/compose.yml" ]]; then
    cat >"$DB_DIR/compose.yml" <<'YAML'
services:
  postgis:
    image: postgis/postgis:18-3.6
    container_name: postgis
    restart: always
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - postgis_data:/var/lib/postgresql

volumes:
  postgis_data:
YAML
  fi
  if have docker; then
    docker_cmd compose -f "$DB_DIR/compose.yml" up -d postgis || FAILED+=("postgis")
    for _ in $(seq 1 30); do
      docker_cmd exec postgis pg_isready -U postgres >/dev/null 2>&1 && break; sleep 2
    done
    if [[ -f "$REPO/.env.local" ]]; then
      (cd "$REPO" && pnpm db:setup) || FAILED+=("pnpm db:setup")
    else
      warn "$REPO/.env.local yok: yedeğinizi geri koyun, sonra 'pnpm db:setup' (ya da yedek dökümü pg_restore ile)."
    fi
  fi
fi

# --- 13. NVIDIA sürücüsü (isteğe bağlı) ------------------------------------------------------------
if want nvidia && lspci 2>/dev/null | grep -qi nvidia; then
  say "NVIDIA sürücüsü (ubuntu-drivers)"
  apt_install ubuntu-drivers-common
  sudo ubuntu-drivers install || FAILED+=("nvidia")
fi

# --- Özet ----------------------------------------------------------------------------------------
say "Bitti"
for t in rustc cargo wasm-bindgen node pnpm uv gdalinfo projinfo qgis grass soffice docker google-chrome gh; do
  if have "$t"; then printf '  %-14s %s\n' "$t" "$("$t" --version 2>/dev/null | head -1)"; else printf '  %-14s —\n' "$t"; fi
done
if [[ ${#FAILED[@]} -gt 0 ]]; then
  warn "Kurulamayanlar: ${FAILED[*]}"
  echo "Bunları elle kurun ya da betiği yeniden çalıştırın (--only <bölüm>)."
fi
cat <<'EOT'

Sonraki adımlar:
  1. Oturumu kapatıp açın (docker grubu, PATH: ~/.cargo/bin, ~/.local/bin).
  2. cd ~/Projects/kentos-cad && make            # komut listesi
  3. cargo build -p kentos-desktop && python3 scripts/desktop/module_links.py target/debug
  4. pnpm py:test                                # Python SDK'sını derler ve sınar
  5. Yedekten geri koyun: .env.local, ~/.config/kentos-cad, ~/.local/share/kentos-cad,
     ~/.claude (bellek), veritabanı dökümü (pg_restore).
EOT
