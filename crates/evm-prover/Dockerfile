# Use the official Rust image as a builder
FROM rust:1.88-slim as builder

# Set the working directory
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    protobuf-compiler \
    curl \
    bash \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install SP1 toolchain
RUN curl -L https://sp1up.succinct.xyz | bash \
    && export PATH="$HOME/.sp1/bin:$PATH" \
    && sp1up --version 5.0.8 \
    && sp1up --c-toolchain --arch amd64

# Copy the workspace Cargo files
COPY Cargo.toml Cargo.lock ./
COPY crates/evm-prover/Cargo.toml ./crates/evm-prover/
COPY crates/sp1/evm-exec-types/Cargo.toml ./crates/sp1/evm-exec-types/
COPY crates/evm-state-queries/Cargo.toml ./crates/evm-state-queries/
COPY crates/evm-state-types/Cargo.toml ./crates/evm-state-types/
COPY crates/evm-storage-proofs/Cargo.toml ./crates/evm-storage-proofs/
COPY crates/storage/Cargo.toml ./crates/storage/
COPY crates/sp1/evm-exec/program/Cargo.toml ./crates/sp1/evm-exec/program/
COPY crates/sp1/evm-range-exec/program/Cargo.toml ./crates/sp1/evm-range-exec/program/
COPY crates/sp1/evm-exec/script/Cargo.toml ./crates/sp1/evm-exec/script/
COPY crates/sp1/evm-range-exec/script/Cargo.toml ./crates/sp1/evm-range-exec/script/

# Create dummy source files to build dependencies
RUN mkdir -p crates/evm-prover/src crates/sp1/evm-exec-types/src crates/evm-state-queries/src crates/evm-state-types/src crates/evm-storage-proofs/src crates/storage/src crates/sp1/evm-exec/program/src crates/sp1/evm-range-exec/program/src crates/sp1/evm-exec/script/src crates/sp1/evm-range-exec/script/src
RUN echo "fn main() {}" > crates/evm-prover/src/main.rs
RUN echo "fn main() {}" > crates/sp1/evm-exec-types/src/lib.rs
RUN echo "fn main() {}" > crates/evm-state-queries/src/lib.rs
RUN echo "fn main() {}" > crates/evm-state-types/src/lib.rs
RUN echo "fn main() {}" > crates/evm-storage-proofs/src/lib.rs
RUN echo "fn main() {}" > crates/storage/src/lib.rs
RUN echo "fn main() {}" > crates/sp1/evm-exec/program/src/main.rs
RUN echo "fn main() {}" > crates/sp1/evm-range-exec/program/src/main.rs
RUN echo "fn main() {}" > crates/sp1/evm-exec/script/src/main.rs
RUN echo "fn main() {}" > crates/sp1/evm-range-exec/script/src/main.rs

# Build dependencies
RUN cargo build --release --bin evm-prover

# Remove dummy files and copy real source
RUN rm -rf crates/evm-prover/src crates/sp1/evm-exec-types/src crates/evm-state-queries/src crates/evm-state-types/src crates/evm-storage-proofs/src crates/storage/src crates/sp1/evm-exec/program/src crates/sp1/evm-range-exec/program/src crates/sp1/evm-exec/script/src crates/sp1/evm-range-exec/script/src

# Copy the actual source code
COPY crates/ ./crates/

# Copy the proto directory
COPY proto/ ./proto/

# Build the application
RUN cargo build --release --bin evm-prover

# Create a new stage with a minimal runtime image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false evm-prover

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/evm-prover /app/evm-prover

# Copy the resources directory
COPY --from=builder /app/crates/evm-prover/resources /app/resources

# Copy the default configuration
COPY crates/evm-prover/config.yaml /app/config.yaml

# Change ownership to the non-root user
RUN chown -R evm-prover:evm-prover /app

# Switch to the non-root user
USER evm-prover

# Expose the gRPC port
EXPOSE 50051

# Set the default command
CMD ["/app/evm-prover"]
