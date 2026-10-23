FROM rust:1-slim-trixie AS construction
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY serveur serveur
RUN cargo build --release --locked -p eko-rip-serveur

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ghostscript \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home eko \
    && mkdir /donnees && chown eko /donnees
COPY --from=construction /src/target/release/eko-rip-serveur /usr/local/bin/eko-rip-serveur
USER eko
ENV EKO_RIP_ADDR=0.0.0.0:8640 EKO_RIP_DATA=/donnees
VOLUME /donnees
EXPOSE 8640
CMD ["eko-rip-serveur"]
