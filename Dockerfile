# Image de Sluis — distroless en production, conformément à convergence-iac.md.
#
# Deux étages. Le premier compile ; le second ne contient que le binaire et ses
# certificats. Aucun shell, aucun gestionnaire de paquets, donc une surface
# d'attaque réduite à ce que le programme fait lui-même.

FROM rust:1.98-bookworm AS constructeur
WORKDIR /src

# Les dépendances d'abord, pour que le cache de couche survive à une
# modification du code, qui est de loin le cas le plus fréquent.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
 && echo 'fn main() {}' > src/bin/sluis_mcp.rs \
 && echo 'fn main() {}' > src/bin/sluis.rs \
 && echo '' > src/lib.rs \
 && cargo build --release --locked 2>/dev/null || true \
 && rm -rf src

COPY . .
RUN cargo build --release --locked --bin sluis-server --bin sluis-mcp --bin sluis 2>/dev/null \
 || cargo build --release --locked --bin sluis-mcp --bin sluis

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=constructeur /src/target/release/sluis-mcp /usr/local/bin/sluis-mcp
COPY --from=constructeur /src/target/release/sluis /usr/local/bin/sluis
USER nonroot:nonroot

# Le journal d'audit doit être inscriptible : Sluis refuse d'exécuter un outil
# s'il ne peut pas tracer, donc un volume non monté se voit tout de suite.
VOLUME ["/app/donnees"]
ENV SLUIS_CONFIG=/app/sluis.toml
ENTRYPOINT ["/usr/local/bin/sluis-mcp"]
