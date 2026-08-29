# Image de Sluis — distroless en production, conformément à convergence-iac.md.
#
# Deux étages. Le premier compile ; le second ne contient que les binaires et
# leurs certificats. Aucun shell, aucun gestionnaire de paquets, donc une
# surface d'attaque réduite à ce que le programme fait lui-même.
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

FROM gcr.io/distroless/cc-debian12:nonroot
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
