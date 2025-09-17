# Use the official Rust image as a builder - force linux/amd64 for SP1 compatibility
FROM --platform=linux/amd64 rust:1.88-slim as builder

# Set the working directory
WORKDIR /app

# Install build dependencies including Go
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    protobuf-compiler \
    curl \
    bash \
    git \
    clang \
    libclang-dev \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install Go (required for SP1 gnark FFI)
RUN wget https://go.dev/dl/go1.21.6.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.21.6.linux-amd64.tar.gz \
    && rm go1.21.6.linux-amd64.tar.gz

# Set Go environment
ENV PATH="/usr/local/go/bin:$PATH"
ENV GOROOT="/usr/local/go"

# Install SP1 toolchain 
RUN curl -L https://sp1up.succinct.xyz | bash
RUN /root/.sp1/bin/sp1up --version 5.2.1
RUN /root/.sp1/bin/sp1up --c-toolchain --arch amd64

# Set SP1 environment
ENV PATH="/root/.sp1/bin:$PATH"
ENV SP1_TOOLCHAIN_PATH="/root/.sp1"

# Copy only dependency manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Fetch dependencies (this layer gets cached)
RUN cargo fetch

# Copy the source code
COPY crates/ ./crates/
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

# Change ownership to the non-root user
RUN chown -R evm-prover:evm-prover /app

# Switch to the non-root user
USER evm-prover

# Expose the gRPC port
EXPOSE 50051

# Set the default command
CMD ["/app/evm-prover"]
