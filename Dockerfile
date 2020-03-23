FROM rustlang/rust:nightly as build

COPY ./ ./

RUN cargo build --release

RUN mkdir -p /build-out

RUN cp target/release/exusss-covid19 /build-out/


FROM ubuntu:18.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=build /build-out/exusss-covid19 /app/

WORKDIR /app

CMD ["./exusss-covid19"]