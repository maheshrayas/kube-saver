FROM lukemathwalker/cargo-chef:latest-rust-1.61 AS chef

WORKDIR kube

FROM chef AS planner

COPY src src
COPY Cargo.toml Cargo.lock ./

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 

RUN update-ca-certificates
COPY --from=planner /kube/recipe.json recipe.json

# Create appuser
ENV USER=kube
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


RUN cargo chef cook --release --recipe-path recipe.json 

COPY src src
COPY Cargo.toml Cargo.lock ./

RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM gcr.io/distroless/cc

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /kube

# Copy our build
COPY --from=builder /kube/target/release/kube-saver ./

# Use an unprivileged user.
USER kube:kube

ENTRYPOINT ["/kube/kube-saver"]
