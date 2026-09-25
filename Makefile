# KentOS CAD geliştirme komutları. `make` (ya da `make help`) hepsini gruplarıyla listeler.
#
# Servisler (web, API, önizleme, masaüstü, arayüz vitrini) arka planda, kendi
# süreç gruplarında çalışır: PID'leri ve günlükleri .run/ altındadır; `make stop`
# hepsini (bütün alt süreçleriyle) kapatır, `make status` neyin çalıştığını gösterir.
#
# Ağır işler (cargo, Vitest, e2e, ölçüm, derleme) /tmp/kentos-heavy.lock kilidiyle
# sırayla çalışır: iki terminalden aynı anda başlatılırsa ikincisi bekler, makine
# kilitlenmez (CLAUDE.md §2). Kilitsiz çalıştırmak için: make test HEAVY=
#
# Değiştirilebilir değerler (örnek: make dev WEB_PORT=5174):
#   WEB_PORT      web geliştirme sunucusu (5173)
#   PREVIEW_PORT  üretim önizlemesi (4173)
#   API_PORT      kentosd (8787)
#   PG_CONTAINER  yerel PostGIS Docker kabı ve compose servisi (postgis)
#   DB_COMPOSE    PostGIS'i tanımlayan compose dosyası (~/Projects/database/compose.yml; yoksa docker start/stop)
#   TEST_DB       Rust veritabanı testleri: .env.local varsa "required" (atlanamaz), yoksa boş
#   LABEL         ölçüm raporunun etiketi (latest)
#   ARGS          kentosd yönetim komutuna geçen argümanlar
#   YES           1 ise db-stop onay sormaz
#   FILE          desktop-snapshot'ın çizeceği .kcad dosyası (boşsa açık çizim yok)

SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help
MAKEFLAGS += --no-print-directory

WEB_PORT ?= 5173
PREVIEW_PORT ?= 4173
API_PORT ?= 8787
PG_CONTAINER ?= postgis
DB_COMPOSE ?= $(HOME)/Projects/database/compose.yml
TEST_DB ?= $(if $(wildcard .env.local),required,)
LABEL ?= latest
ARGS ?=
YES ?=
FILE ?=

SVC := $(CURDIR)/scripts/dev/svc.sh
HEAVY ?= $(SVC) heavy
API_URL := http://127.0.0.1:$(API_PORT)
WEB_URL := http://localhost:$(WEB_PORT)
PREVIEW_URL := http://localhost:$(PREVIEW_PORT)

.PHONY: help install setup doctor \
		up down dev dev-web dev-api run desktop showcase stop stop-web stop-api stop-preview stop-desktop stop-showcase \
		restart status logs logs-web logs-api logs-desktop open \
		db-status db-start db-stop db-setup db-migrate kentosd \
		build wasm build-api build-rust build-desktop catalog inventory \
		check verify typecheck test test-rust test-wasm test-desktop fmt fmt-check arch e2e e2e-visual e2e-interaction e2e-cloud inventory-check ui-snapshots desktop-snapshot \
		perf-startup perf-interaction clean clean-wasm clean-rust

##@ Yardım

help: ## Bu listeyi gösterir (varsayılan hedef)
	@awk 'BEGIN { FS = ":.*## " } \
	  /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } \
	  /^[a-zA-Z0-9_.-]+:.*## / { printf "  \033[36m%-17s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\nDeğişkenler: Makefile'ın başı. Servis günlükleri: .run/<servis>.log\n"

##@ Kurulum

install: ## pnpm bağımlılıklarını kilit dosyasına göre kurar
	pnpm install --frozen-lockfile

setup: install ## İlk kurulum: bağımlılıklar, araç zinciri denetimi, WASM paketleri
	@rustup target list --installed | grep -qx wasm32-unknown-unknown || { echo "• wasm32 hedefi yok: rustup target add wasm32-unknown-unknown"; exit 1; }
	@want=$$(grep -m1 -oE 'wasm-bindgen = "=[0-9.]+"' Cargo.toml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+'); \
	  have=$$(wasm-bindgen --version 2>/dev/null | awk '{print $$2}'); \
	  [ "$$want" = "$$have" ] || { echo "• wasm-bindgen-cli $$want gerekli (bulunan: $${have:-yok}): cargo install wasm-bindgen-cli --version $$want --locked"; exit 1; }
	$(HEAVY) pnpm wasm

doctor: ## Araç sürümlerini, servisleri ve veritabanını gösterir
	@echo "Araçlar:"
	@for c in "rustc --version" "cargo --version" "wasm-bindgen --version" "node --version" "pnpm --version" "google-chrome --version" "docker --version"; do \
	  printf "  %-24s %s\n" "$${c%% *}" "$$($$c 2>/dev/null | head -1 || echo yok)"; done
	@echo "Servisler:"; $(SVC) status
	@$(MAKE) db-status

##@ Servisler (arka planda; durdurmak: make stop, günlükler: make logs)

up: dev desktop ## Her şey: API + web geliştirme sunucusu + masaüstü uygulaması

down: stop ## Her şeyi durdurur (make stop ile aynı)

dev: ## Geliştirme: API + web (Vite, anlık yenileme; http://localhost:5173)
	@$(SVC) stop preview
	@$(MAKE) dev-api || echo "• API başlatılamadı; web API'siz çalışır (günlük: make logs-api)"
	@$(MAKE) dev-web
	@$(MAKE) status

dev-web: ## Yalnız web geliştirme sunucusu; API'siz de çizer
	@$(HEAVY) pnpm wasm
	@KENTOS_API_PORT=$(API_PORT) $(SVC) start web pnpm -C apps/web exec vite --port $(WEB_PORT) --strictPort
	@$(SVC) wait web $(WEB_URL) 90

dev-api: ## Yalnız API (kentosd serve; .env.local ve PostGIS gerekir)
	@[ -f .env.local ] || { echo "• .env.local yok: önce yerel veritabanını kurun (make db-setup)"; exit 1; }
	@$(SVC) stop api
	@$(HEAVY) cargo build -q -p kentos-api
	@KENTOS_API_PORT=$(API_PORT) KENTOS_PUBLIC_URL=$(WEB_URL) $(SVC) start api target/debug/kentosd serve
	@$(SVC) wait api $(API_URL)/v1/health 30

run: ## Üretim derlemesiyle çalıştırır: build + önizleme (http://localhost:4173) + API
	@$(SVC) stop web api preview
	@$(HEAVY) pnpm build
	@if [ -f .env.local ]; then \
	  $(HEAVY) cargo build -q -p kentos-api && \
	  KENTOS_API_PORT=$(API_PORT) KENTOS_PUBLIC_URL=$(PREVIEW_URL) $(SVC) start api target/debug/kentosd serve && \
	  $(SVC) wait api $(API_URL)/v1/health 30 || echo "• API başlatılamadı; önizleme API'siz (make logs-api)"; \
	else echo "• .env.local yok: önizleme API'siz çalışır"; fi
	@KENTOS_API_PORT=$(API_PORT) $(SVC) start preview pnpm -C apps/web exec vite preview --port $(PREVIEW_PORT) --strictPort
	@$(SVC) wait preview $(PREVIEW_URL) 30
	@$(MAKE) status

desktop: ## Masaüstü uygulaması (apps/desktop, native Iced + wgpu)
	@$(HEAVY) cargo build -q -p kentos-desktop
	@$(SVC) start desktop target/debug/kentos-cad

showcase: ## Arayüz bileşenleri vitrini (apps/ui-showcase)
	@$(HEAVY) cargo build -q -p kentos-ui-showcase
	@$(SVC) start showcase target/debug/kentos-ui-showcase

stop: ## Bütün servisleri durdurur (web, önizleme, API, masaüstü, vitrin; veritabanına dokunmaz)
	@$(SVC) stop

stop-web: ## Web geliştirme sunucusunu durdurur
	@$(SVC) stop web

stop-api: ## API'yi durdurur
	@$(SVC) stop api

stop-preview: ## Üretim önizlemesini durdurur
	@$(SVC) stop preview

stop-desktop: ## Masaüstü uygulamasını kapatır
	@$(SVC) stop desktop

stop-showcase: ## Arayüz vitrinini kapatır
	@$(SVC) stop showcase

restart: ## Durdurup geliştirme modunda yeniden başlatır
	@$(MAKE) stop
	@$(MAKE) dev

status: ## Çalışan servisler, adresleri ve veritabanı
	@echo "Servisler:"; $(SVC) status
	@echo "Adresler:"
	@$(SVC) running web && echo "  web       $(WEB_URL)" || true
	@$(SVC) running preview && echo "  önizleme  $(PREVIEW_URL)" || true
	@$(SVC) running api && echo "  API       $(API_URL)/v1/health" || true
	@echo "Durdurmak: make stop · Günlükler: make logs"

logs: ## Bütün servis günlüklerini izler (Ctrl+C çıkar)
	@ls .run/*.log >/dev/null 2>&1 || { echo "• henüz günlük yok"; exit 0; }
	@tail -n 40 -F .run/*.log

logs-web: ## Web geliştirme sunucusunun günlüğü
	@tail -n 60 -F .run/web.log

logs-api: ## API günlüğü
	@tail -n 60 -F .run/api.log

logs-desktop: ## Masaüstü uygulamasının günlüğü
	@tail -n 60 -F .run/desktop.log

open: ## Web uygulamasını tarayıcıda açar
	@xdg-open $(WEB_URL) >/dev/null 2>&1 || echo "$(WEB_URL)"

##@ Veritabanı (yerel PostGIS; yalnız geliştirme makinesi)

db-status: ## PostGIS kabının ve bağlantının durumu
	@echo "Veritabanı:"
	@docker ps -a --filter "name=^$(PG_CONTAINER)$$" --format '  {{.Names}}: {{.Status}} ({{.Image}})' 2>/dev/null | grep . || echo "  $(PG_CONTAINER) kabı bulunamadı (PG_CONTAINER=...)"
	@docker exec $(PG_CONTAINER) pg_isready -q 2>/dev/null && echo "  bağlantı hazır" || echo "  bağlantı yok"
	@[ -f "$(DB_COMPOSE)" ] && echo "  compose: $(DB_COMPOSE)" || echo "  compose dosyası yok ($(DB_COMPOSE)); kap docker ile yönetiliyor"

# The compose file also holds other services (redis, mongodb, keycloak) and
# the PostGIS server hosts other applications' databases: only the one
# service is started, and a running container is never recreated.
db-start: ## PostGIS'i başlatır: compose varsa up --no-recreate (çalışan kabı yeniden kurmaz), yoksa docker start
	@if [ -f "$(DB_COMPOSE)" ]; then docker compose -f "$(DB_COMPOSE)" up -d --no-recreate $(PG_CONTAINER); else docker start $(PG_CONTAINER); fi

db-stop: ## PostGIS kabını durdurur; DİKKAT: kaptaki başka uygulamaların veritabanları da kapanır
	@[ "$(YES)" = 1 ] || { read -r -p "$(PG_CONTAINER) kabı durdurulsun mu? İçindeki başka veritabanları da kapanır [e/H] " a; [ "$$a" = e ] || { echo "vazgeçildi"; exit 0; }; }
	@if [ -f "$(DB_COMPOSE)" ]; then docker compose -f "$(DB_COMPOSE)" stop $(PG_CONTAINER); else docker stop $(PG_CONTAINER); fi

db-setup: ## Roller, kentos_cad veritabanı, migration ve geliştirme verisi (yalnız yerel!)
	$(HEAVY) pnpm db:setup

db-migrate: ## Bekleyen migration'ları uygular
	$(HEAVY) pnpm kentosd -- migrate

kentosd: ## Yönetim CLI'si: make kentosd ARGS="tenant list"
	$(HEAVY) pnpm kentosd -- $(ARGS)

##@ Derleme

build: ## Web üretim derlemesi: WASM + tsc + Vite (apps/web/dist)
	$(HEAVY) pnpm build

wasm: ## Kaynağı değişen Rust WASM paketlerini derler
	$(HEAVY) pnpm wasm

build-api: ## kentosd'yi derler
	$(HEAVY) cargo build -p kentos-api

build-rust: ## Web ve sunucu Rust crate'lerini derler (masaüstü hariç)
	$(HEAVY) cargo build

build-desktop: ## Masaüstü uygulamasını, arayüz kütüphanesini ve vitrini derler
	$(HEAVY) cargo build -p kentos-ui -p kentos-ui-showcase -p kentos-desktop

catalog: ## Ürün komutu katalogunu yeniden yazar (ADR 0013; farkı okuyup commit edin)
	$(HEAVY) env KENTOS_WRITE_CATALOG=1 cargo test -q -p kentos-contracts catalog

inventory: ## Web özellik envanterini yeniden yazar (docs/inventory)
	$(HEAVY) pnpm inventory

##@ Test ve denetim (sırayla, ağır iş kilidiyle)

check: ## Hızlı takım: typecheck + test + fmt-check + test-rust + test-desktop
	@$(MAKE) typecheck test fmt-check test-rust test-desktop

verify: ## Commit öncesi tam takım: check + build + e2e + e2e-visual + inventory-check
	@$(MAKE) check build e2e e2e-visual inventory-check

typecheck: ## TypeScript tip denetimi
	$(HEAVY) pnpm typecheck

test: ## Vitest birim testleri
	$(HEAVY) pnpm test

test-rust: ## cargo test + clippy + bağımlılık yönü (TEST_DB=required: DB testleri atlanamaz)
	$(HEAVY) env KENTOS_TEST_DB=$(TEST_DB) pnpm rust:test

test-wasm: ## Rust testleri + WASM paketleri + WASM ve biçim entegrasyon testleri
	$(HEAVY) env KENTOS_TEST_DB=$(TEST_DB) pnpm test:rust

test-desktop: ## Arayüz kütüphanesi, vitrin ve masaüstü testleri + clippy
	$(HEAVY) pnpm rust:test:desktop

fmt: ## Rust kaynaklarını biçimler
	cargo fmt --all

fmt-check: ## Rust biçim denetimi
	cargo fmt --all -- --check

arch: ## Crate bağımlılık yönü denetimi (ADR 0010)
	pnpm arch:deps

e2e: ## Tarayıcı duman testi (başsız Chrome)
	$(HEAVY) pnpm e2e

e2e-visual: ## Arayüzün görsel karşılaştırması
	$(HEAVY) pnpm e2e:visual

e2e-interaction: ## Etkileşim izleri: poligon kabul izi ve tuş anlamları (fixtures/interaction, ADR 0018)
	$(HEAVY) pnpm e2e:interaction

e2e-cloud: ## Gerçek kentosd + PostGIS bulut akışı (geliştirme veritabanına “E2E …” projeleri yazar)
	$(HEAVY) pnpm e2e:cloud

inventory-check: ## Web özellik envanteri güncel mi
	$(HEAVY) pnpm inventory:check

ui-snapshots: ## Arayüz vitrinini ekransız çizer, başvuru görüntüleriyle karşılaştırır
	$(HEAVY) cargo build -q -p kentos-ui-showcase
	$(HEAVY) node scripts/ui/snapshots.mjs

desktop-snapshot: ## Masaüstü penceresini görüntüye çizer: .run/desktop.png (FILE=çizim.kcad; çizim alanı wgpu ister)
	$(HEAVY) cargo build -q -p kentos-desktop
	mkdir -p .run && KENTOS_SNAPSHOT_BACKEND=wgpu target/debug/kentos-cad snapshot .run/desktop.png $(FILE)
	@echo "• .run/desktop.png"

##@ Ölçüm (başka servis ve ağır iş yokken; önce make stop)

perf-startup: ## Üretim derlemesi envanteri ve açılış süresi (LABEL=...)
	cd apps/web && $(HEAVY) node scripts/perf/bundle.mjs --label $(LABEL)
	cd apps/web && $(HEAVY) node scripts/perf/startup.mjs --label $(LABEL)

perf-interaction: ## Etkileşim ölçümü, tabanla karşılaştırmalı (LABEL=...)
	$(HEAVY) pnpm perf:interaction --label $(LABEL)

##@ Temizlik

clean: ## Servisleri durdurur; web derleme çıktısı ve .run/ silinir
	@$(SVC) stop
	rm -rf apps/web/dist dist .run

clean-wasm: ## WASM paketlerini siler (sonraki komut yeniden derler)
	rm -rf apps/web/src/wasm/pkg apps/web/src/io/pkg apps/web/src/style/svg/pkg

clean-rust: ## cargo clean: bütün Rust derleme çıktısı (sonraki derleme uzun sürer)
	cargo clean
