FROM golang:1.24.3-alpine AS go-builder

WORKDIR /home/hyperlane

COPY hyperlane/go.* /home/hyperlane/
COPY hyperlane/cmd /home/hyperlane/cmd

# Build your Go CLI for cosmosnative deployment
RUN go build -o hyp ./cmd/hyp

FROM node:24-slim

# Install the hyperlane CLI (used for evm deployment)
RUN npm install -g @hyperlane-xyz/cli

COPY --from=go-builder /home/hyperlane/hyp /usr/local/bin/hyp

WORKDIR /home/hyperlane

COPY hyperlane/configs /home/hyperlane/configs
COPY hyperlane/registry /home/hyperlane/registry
COPY hyperlane/scripts /home/hyperlane/scripts

ENTRYPOINT [ "scripts/docker-entrypoint.sh" ]