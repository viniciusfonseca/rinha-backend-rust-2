FROM rust:1.76.0 as base

RUN apt-get update -yqq && apt-get install -yqq cmake g++
RUN mkdir /tmp/sockets
WORKDIR /app

FROM base as build

RUN mkdir src; touch src/main.rs

COPY Cargo.toml Cargo.lock ./

RUN cargo fetch

COPY src ./src/

RUN cargo build --release

FROM base

COPY --from=build /app /app

EXPOSE 80
EXPOSE 3001/UDP
EXPOSE 3002/UDP

RUN chown -R www-data:www-data /tmp/sockets

USER www-data:www-data

CMD ./target/release/rinha-backend-rust-2
