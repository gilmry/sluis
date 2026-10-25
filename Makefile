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

.PHONY: aide setup fmt fmt-check lint test doc ci secret-scan sbom purete contrat \
	shellcheck test-tireur

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

contrat: ## Régénère contrat/depot-cible.v1.json depuis le code (ADR-034)
	@mkdir -p contrat
	$(CARGO) run --quiet --bin sluis -- contract > contrat/depot-cible.v1.json

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

# Les scripts shell livrés. shellcheck local s'il existe, sinon son image :
# on ne saute pas la vérification faute d'outil.
SCRIPTS_SH := tools/tireur-etiquette-signee.sh.example tests/tireur/tests.sh
SHELLCHECK_IMAGE ?= koalaman/shellcheck:stable

shellcheck: ## shellcheck des scripts du tireur (plancher)
	@if command -v shellcheck >/dev/null 2>&1; then \
		shellcheck --shell=bash $(SCRIPTS_SH); \
	else \
		docker run --rm -v "$(CURDIR):/mnt:ro" -w /mnt $(SHELLCHECK_IMAGE) \
			--shell=bash $(SCRIPTS_SH); \
	fi

# Le harnais exige root et deux comptes (root possède allowed_signers, nobody
# déploie) : il tourne donc dans un conteneur jetable, en local comme en CI,
# et jamais en root sur la machine de travail.
TIREUR_IMAGE ?= debian:stable-slim

test-tireur: ## Tests du tireur d'étiquette signée (ADR-013), en conteneur
	docker run --rm -v "$(CURDIR):/src:ro" -w /src $(TIREUR_IMAGE) bash -c '\
		apt-get update -qq >/dev/null && \
		DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
			git openssh-client >/dev/null && \
		bash tests/tireur/tests.sh'

ci: fmt-check lint test secret-scan sbom ## Chaîne complète, celle que la CI rejoue
	@echo ""
	@echo "  make ci : vert."
	@echo "  Rappel : les gates absentes en local sont jouées côté serveur."
