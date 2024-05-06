FROM docker.io/library/rust:1.78.0-bullseye AS build

ARG DATABASE_URL

WORKDIR /app

COPY Cargo.toml Cargo.lock .
COPY models/Cargo.toml models/Cargo.toml
COPY crystal_snapshots/Cargo.toml crystal_snapshots/Cargo.toml

RUN mkdir models/src \
    && touch models/src/lib.rs \
    && mkdir crystal_snapshots/src \
    && echo "fn main() {}" > crystal_snapshots/src/main.rs \
    && cargo build --release

COPY . /app

RUN touch models/src/lib.rs \
    && touch crystal_snapshots/src/main.rs \
    && cargo build --release

FROM gcr.io/distroless/cc AS deploy

COPY --from=build /app/target/release/crystal_snapshots /crystal_snapshots

ENTRYPOINT ["/crystal_snapshots"]
