# Sluis — cibles de fabrication.
#
# Toute cible passe par CARGO, surchargeable. Sur la machine de Gilles, la
# racine ne fait que 48 Go et un target/ Rust la sature : CARGO=kcargo place la
# cible et le registre sur des volumes Docker adossés au disque de 500 Go.
#
#     make ci CARGO=kcargo
#
# En CI le cargo natif convient, le disque y est jetable.

CARGO ?= cargo

.DEFAULT_GOAL := aide

.PHONY: aide setup fmt fmt-check lint test doc ci secret-scan sbom purete

aide: ## Affiche cette aide
	@grep -hE '^[a-z][a-zA-Z0-9_-]*:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

setup: ## Prépare l'environnement de développement
	rustup component add rustfmt clippy 2>/dev/null || true
	@echo "Astuce : sur une machine à petite racine, utilisez CARGO=kcargo."

fmt: ## Formate le code
	$(CARGO) fmt

fmt-check: ## Vérifie le formatage (plancher)
	$(CARGO) fmt --check

lint: ## Clippy en mode bloquant (plancher)
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test: ## Joue tous les tests, dont la gate de pureté du domaine
	$(CARGO) test --all-features

purete: ## Rejoue seule la gate de pureté du domaine
	$(CARGO) test --test purete_domaine

doc: ## Construit la documentation
	$(CARGO) doc --no-deps

secret-scan: ## Cherche des secrets dans le diff et l'historique (plancher)
	@if command -v gitleaks >/dev/null 2>&1; then \
		gitleaks detect --source . --redact --verbose; \
	else \
		echo "gitleaks absent : gate non jouée en local, la CI la joue."; \
		echo "Installation : https://github.com/gitleaks/gitleaks"; \
	fi

sbom: ## Produit la nomenclature CycloneDX (plancher)
	@if $(CARGO) cyclonedx --version >/dev/null 2>&1; then \
		$(CARGO) cyclonedx --format json; \
	else \
		echo "cargo-cyclonedx absent : gate non jouée en local, la CI la joue."; \
		echo "Installation : cargo install cargo-cyclonedx"; \
	fi

ci: fmt-check lint test secret-scan sbom ## Chaîne complète, celle que la CI rejoue
	@echo ""
	@echo "  make ci : vert."
	@echo "  Rappel : les gates absentes en local sont jouées côté serveur."
