# 构建阶段
FROM rust:1.92-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig libxml2-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY static ./static

# 静态链接构建
ENV OPENSSL_STATIC=1
RUN cargo build --release

# 运行阶段
FROM alpine:3.20

RUN apk add --no-cache ca-certificates libxml2

WORKDIR /app
COPY --from=builder /app/target/release/anime-search-api .
COPY rules ./rules

ENV PORT=3000
EXPOSE 3000

CMD ["./anime-search-api"]

