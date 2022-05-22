FROM rust:latest AS builder

RUN update-ca-certificates

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


WORKDIR /kube

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

CMD ["/kube/kube-saver"]
