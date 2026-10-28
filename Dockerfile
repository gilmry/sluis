# Image de Sluis — distroless en production, conformément à convergence-iac.md.
#
# Le premier étage compile ; les étages finaux ne contiennent que les binaires
# et leurs certificats. Aucun shell, aucun gestionnaire de paquets, donc une
# surface d'attaque réduite à ce que le programme fait lui-même.
#
# **Deux cibles, un seul binaire Rust**, conformément au choix d'architecture
# du Sprint 5 :
#
#   --target executant  le Sluis du réseau interne. Porte tofu et wrk, et c'est
#                       le seul à qui l'on confie des clés OVH.
#   --target public     le Sluis exposé sur Internet. Ne porte aucun moteur, et
#                       ne peut donc rien muter même si on lui donnait des clés.
#
# `public` est déclarée EN DERNIER, donc c'est elle que produit un `docker
# build` sans `--target`. L'image armée ne s'obtient qu'en la demandant.
#
# L'entrée est `sluis-server`, le transport Streamable HTTP : c'est le seul
# mode qui a un sens derrière Traefik. Le mode stdio se lance en local depuis
# un binaire, pas depuis un conteneur exposé sur un réseau.

FROM rust:1.98-bookworm AS constructeur
WORKDIR /src

# Les dépendances d'abord, pour que le cache de couche survive à une
# modification du code, qui est de loin le cas le plus fréquent. Les trois
# binaires déclarés dans Cargo.toml doivent avoir une ébauche, sinon cargo
# échoue et la couche de cache ne sert à rien.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
 && echo 'fn main() {}' > src/bin/sluis_server.rs \
 && echo 'fn main() {}' > src/bin/sluis_mcp.rs \
 && echo 'fn main() {}' > src/bin/sluis.rs \
 && touch src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src

COPY . .
# Les sources copiées portent la date du dépôt, donc antérieure à la
# compilation des ébauches ci-dessus : sans invalider leur empreinte, cargo
# les croit à jour et l'image embarquerait trois binaires vides.
RUN rm -rf target/release/.fingerprint/sluis-* \
 && touch src/lib.rs src/bin/*.rs
# Aucun `|| true` ici : un binaire qui ne compile pas doit faire échouer
# l'image, pas produire silencieusement une image amputée.
RUN cargo build --release --locked --bin sluis-server --bin sluis-mcp --bin sluis

# Garde-fou : une ébauche vide rend 0 sans rien écrire, ce qui ressemble à un
# succès jusqu'au déploiement. Cette vérification transforme ce silence en
# échec de build.
RUN target/release/sluis --help | grep -q "l'écluse" \
 && test -x target/release/sluis-server \
 && test -x target/release/sluis-mcp

# Le répertoire de données est créé ici, avec l'uid de `nonroot`. C'est le seul
# moyen d'en fixer le propriétaire : l'étage final n'a pas de shell, et un
# volume nommé hérite des droits du point de montage présent dans l'image.
RUN install -d -o 65532 -g 65532 /donnees

# ── Les moteurs, pour le seul Sluis exécutant ──────────────────────────────
#
# **OpenTofu, pas Terraform.** Terraform est sous BSL 1.1 depuis la 1.6, et ce
# dépôt est MIT avec une image publique sur GHCR : redistribuer du BSL y serait
# contradictoire. OpenTofu est le fork MPL-2.0, lit les mêmes fichiers `.tf` et
# rend les mêmes résumés, que `analyser_resume_plan` sait déjà lire. Le binaire
# s'appelle `tofu`, et c'est sous ce nom que l'allowlist et le diagnostic le
# connaissent : le renommer `terraform` aurait masqué le choix.
#
# À noter, mesuré le 2026-09-02 : ce choix ne se paie PAS en vulnérabilités.
# Terraform v1.16.1 porte 9 CVE CRITICAL/HIGH dans son propre binaire, tofu
# 1.12.6 en porte 12. Voir ADR-011.
#
# Un seul étage produit les deux moteurs, puis on ne garde que les binaires.
# wrk lie l'OpenSSL de cette Debian, que la distroless n'a pas ; les
# bibliothèques nécessaires sont donc emportées avec lui, depuis la même
# Debian 12, donc avec le même ABI.
FROM debian:bookworm-slim AS moteurs
ARG WRK_VERSION=4.2.0
ARG TOFU_VERSION=1.12.6
# `unzip` n'est pas décoratif : le Makefile de wrk déballe un LuaJIT vendorisé
# et échoue sinon sur un « make: unzip: No such file or directory » qui ne dit
# pas qu'il s'agit d'une dépendance de build manquante.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential libssl-dev ca-certificates curl unzip \
 && rm -rf /var/lib/apt/lists/*

# OpenTofu vient de son archive de release, PAS de son image.
#
# Depuis la 1.10, l'image officielle refuse délibérément de servir de base : un
# `ONBUILD RUN exit 1` fait échouer tout Dockerfile qui la référence, même pour
# n'en extraire qu'un fichier (opentofu/opentofu#1931). L'archive est le chemin
# que le projet documente.
#
# L'empreinte est vérifiée contre le SHA256SUMS publié à côté. Sans cela, on
# ferait confiance à un binaire tiré du réseau pour piloter une infrastructure,
# ce qui est exactement le genre de raccourci qu'un tel outil ne peut pas se
# permettre. `--ignore-missing` parce que le fichier de sommes couvre toutes les
# plateformes et qu'on n'en télécharge qu'une.
#
# **Version épinglée, et à remonter régulièrement.** Le binaire embarque ses
# dépendances Go, donc leurs CVE : la 1.8.8 en portait 53 en CRITICAL/HIGH, la
# 1.12.6 en porte 12. C'est la charge de sécurité que l'image exécutante
# transporte, et le scan de la CI la mesure (ADR-011).
RUN cd /tmp \
 && base="https://github.com/opentofu/opentofu/releases/download/v${TOFU_VERSION}" \
 && curl -fsSL -O "$base/tofu_${TOFU_VERSION}_linux_amd64.zip" \
 && curl -fsSL -O "$base/tofu_${TOFU_VERSION}_SHA256SUMS" \
 && sha256sum -c --ignore-missing "tofu_${TOFU_VERSION}_SHA256SUMS" \
 && unzip -q "tofu_${TOFU_VERSION}_linux_amd64.zip" tofu \
 && install -m 0755 tofu /usr/local/bin/tofu \
 && /usr/local/bin/tofu version | grep -q "OpenTofu v${TOFU_VERSION}"

# `WITH_OPENSSL=/usr` n'est pas une optimisation, c'est un correctif de sécurité.
# Sans lui, le Makefile de wrk compile et lie STATIQUEMENT l'OpenSSL 1.1.1i qu'il
# embarque dans `deps/`, une version de décembre 2020, en fin de vie depuis
# septembre 2023. On embarquerait sciemment de la cryptographie non maintenue
# dans l'image qui détient les clés OVH, et le scan de la CI n'y verrait qu'un
# binaire statique. Avec cette option, wrk lie l'OpenSSL 3 de Debian, que le
# scan connaît et que les mises à jour de base suivent.
#
# Effet de bord bienvenu : le build passe de plusieurs minutes à quelques
# dizaines de secondes, l'étage n'ayant plus tout OpenSSL à compiler.
RUN curl -fsSL "https://github.com/wg/wrk/archive/refs/tags/${WRK_VERSION}.tar.gz" -o /tmp/wrk.tar.gz \
 && tar -xzf /tmp/wrk.tar.gz -C /tmp \
 && make -C "/tmp/wrk-${WRK_VERSION}" WITH_OPENSSL=/usr -j"$(nproc)" \
 && install -m 0755 "/tmp/wrk-${WRK_VERSION}/wrk" /usr/local/bin/wrk

# Même garde-fou que pour les binaires Rust : un wrk qui ne répond pas doit
# faire échouer le build, pas se découvrir au premier palier de charge.
# `wrk --version` écrit sa bannière puis rend 1 ; c'est la bannière qui compte.
RUN /usr/local/bin/wrk --version 2>&1 | grep -q '^wrk '

# Ce que wrk réclame hors de la base distroless est emporté avec lui.
#
# Rien n'est codé en dur ici, et c'est délibéré : selon la version et l'OpenSSL
# de l'hôte, wrk lie sa cryptographie statiquement ou dynamiquement. Une liste
# devinée d'avance aurait été fausse dans un cas sur deux, et fausse tard, au
# premier appel, avec un « no such file or directory » qui ne nomme pas le
# manquant. On lit donc ce dont il a réellement besoin.
RUN mkdir -p /sortie/lib && touch /sortie/lib/.garder \
 && ldd /usr/local/bin/wrk | awk '/=> \//{print $3}' \
    | grep -vE '/(libc|libm|libdl|libpthread|librt|libgcc_s|libstdc\+\+)\.so' \
    | xargs -r -I{} cp -L {} /sortie/lib/

# Et on vérifie que la moisson est complète : toute dépendance hors base doit
# désormais exister dans /sortie/lib. Ce contrôle transforme une panne différée
# en échec de build.
RUN ldd /usr/local/bin/wrk | awk '/=> \//{print $3}' \
    | grep -vE '/(libc|libm|libdl|libpthread|librt|libgcc_s|libstdc\+\+)\.so' \
    | while read -r so; do \
        test -f "/sortie/lib/$(basename "$so")" \
          || { echo "dépendance non emportée : $so" >&2; exit 1; }; \
      done \
 && echo "wrk : $(ls /sortie/lib | grep -c '\.so' || true) bibliothèque(s) hors base emportée(s)"

# ── OpenSSL du socle, quand la distroless retarde sur Debian ───────────────
#
# La distroless n'est reconstruite qu'avec retard sur les correctifs de
# sécurité de Debian. Le 2026-10-28, elle portait encore libssl3
# 3.0.20-1~deb12u2 (CVE-2026-84782, HIGH) alors que bookworm-security
# distribuait 3.0.22-1~deb12u2, et le scan bloquait toutes les images.
# ADR-011 écarte un `.trivyignore` : on remplace donc le paquet, pas le
# verdict.
#
# Le remplacement suit le paquet Debian à la lettre : les fichiers que la
# distroless embarque, tirés du `.deb` officiel, et son entrée `status.d`
# réécrite depuis le contrôle de ce même `.deb`. Le scan lit cette entrée ;
# on vérifie donc que la bibliothèque copiée est bien la version qu'elle
# déclare, sinon le scan serait vert sur un mensonge.
#
# L'étage se retire seul : si la distroless porte une version égale ou plus
# récente, rien n'est copié, et on ne régresse jamais sa bibliothèque.
FROM debian:bookworm-slim AS openssl
COPY --from=gcr.io/distroless/cc-debian12:nonroot /var/lib/dpkg/status.d/libssl3 /socle/libssl3
RUN set -eu; mkdir -p /sortie; cd /tmp \
 && apt-get update && apt-get download libssl3 && rm -rf /var/lib/apt/lists/* \
 && deb="$(ls libssl3_*.deb)" \
 && neuf="$(dpkg-deb -f "$deb" Version)" \
 && socle="$(sed -n 's/^Version: //p' /socle/libssl3)" \
 && if dpkg --compare-versions "$neuf" le "$socle"; then \
      echo "libssl3 : socle $socle à jour (Debian $neuf), rien à remplacer"; exit 0; \
    fi \
 && dpkg-deb -x "$deb" x && dpkg-deb -e "$deb" ctl \
 && lib="$(dirname "$(find x -name libssl.so.3)")" && lib="${lib#x/}" \
 && for f in libssl.so.3 libcrypto.so.3 ossl-modules/legacy.so; do \
      install -D -m 0644 "x/$lib/$f" "/sortie/$lib/$f"; \
    done \
 && mkdir -p /sortie/usr/share/doc && cp -r x/usr/share/doc/libssl3 /sortie/usr/share/doc/ \
 && mkdir -p /sortie/var/lib/dpkg/status.d \
 && dpkg-deb -f "$deb" > /sortie/var/lib/dpkg/status.d/libssl3 \
 && cp ctl/md5sums /sortie/var/lib/dpkg/status.d/libssl3.md5sums \
 && grep -aq "OpenSSL ${neuf%%-*} " "/sortie/$lib/libcrypto.so.3" \
 && echo "libssl3 : socle $socle remplacé par $neuf"

# ── Socle commun aux deux Sluis ────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS socle
# Vide quand la distroless est à jour ; voir l'étage `openssl`.
COPY --from=openssl /sortie/ /
WORKDIR /app
COPY --from=constructeur /src/target/release/sluis-server /usr/local/bin/sluis-server
COPY --from=constructeur /src/target/release/sluis-mcp /usr/local/bin/sluis-mcp
COPY --from=constructeur /src/target/release/sluis /usr/local/bin/sluis
COPY --from=constructeur --chown=65532:65532 /donnees /app/donnees
USER nonroot:nonroot

EXPOSE 8080

# Le journal d'audit et le dépôt OAuth doivent être inscriptibles : Sluis
# refuse de démarrer s'il ne peut pas tracer. Les deux vivent dans le volume,
# jamais dans /app, qui appartient à root et reste en lecture seule pour le
# processus.
VOLUME ["/app/donnees"]
ENV SLUIS_CONFIG=/app/sluis.toml \
    SLUIS_DEPOT_OAUTH=/app/donnees/sluis-oauth.json
ENTRYPOINT ["/usr/local/bin/sluis-server"]

# ── Sluis exécutant ────────────────────────────────────────────────────────
#
# Le socle plus les deux moteurs, et rien d'autre : ni shell, ni gestionnaire
# de paquets, ni les outils de compilation de l'étage qui a produit wrk.
FROM socle AS executant
COPY --from=moteurs /usr/local/bin/tofu /usr/local/bin/tofu
COPY --from=moteurs /usr/local/bin/wrk /usr/local/bin/wrk
# Les bibliothèques que wrk réclame hors de la base, récoltées par l'étage
# précédent. Le répertoire peut être vide, si wrk s'est lié statiquement ; c'est
# un cas normal, pas une erreur.
COPY --from=moteurs /sortie/lib/ /usr/local/lib/sluis/
ENV LD_LIBRARY_PATH=/usr/local/lib/sluis

# ── Sluis public ───────────────────────────────────────────────────────────
#
# Volontairement identique au socle, et déclarée en dernier pour être la cible
# par défaut. Une image sans moteur ne peut pas provisionner, quoi qu'on lui
# demande et quelles que soient les clés qu'on lui donne : c'est une propriété
# de l'image, pas une règle qu'un appelant pourrait contourner.
FROM socle AS public
