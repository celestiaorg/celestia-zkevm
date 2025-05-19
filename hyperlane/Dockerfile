FROM golang:1.24.3-alpine AS go-builder

WORKDIR /hyperlane
COPY ./cmd ./cmd

# Build your Go CLI for cosmosnative deployment
RUN go build -o hyp ./cmd/hyp

FROM node:24-slim

# Install the hyperlane CLI (used for evm deployment)
RUN npm install -g @hyperlane-xyz/cli

COPY --from=go-builder /hyperlane/hyp /usr/local/bin/hyp

WORKDIR /home/node
COPY scripts/ /home/node/scripts/

ENTRYPOINT [ "scripts/docker-entrypoint.sh" ]
