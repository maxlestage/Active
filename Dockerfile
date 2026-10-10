# The whole site from Rust sources: the WebAssembly bundle, then the server.
# Used by Heroku (heroku.yml) and by any container host.

FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY examples examples
COPY site site
RUN cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm \
 && cargo build -p active-site --bin active-site --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/active-site /app/active-site
COPY --from=build /src/site/static /app/site/static
COPY --from=build /src/site/docs /app/site/docs
COPY --from=build /src/site/pkg /app/site/pkg
ENV SITE_DIR=/app/site
# Heroku sets PORT; 3000 elsewhere.
CMD ["/app/active-site"]
