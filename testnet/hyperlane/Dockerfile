FROM node:24.22.1-slim

# Install the hyperlane CLI (used for evm deployment)
RUN npm install -g @hyperlane-xyz/cli

# Install system packages and foundry 
RUN apt-get update && \
    apt-get install -y curl git build-essential pkg-config libssl-dev

RUN curl -L https://foundry.paradigm.xyz | bash && \
    ~/.foundry/bin/foundryup

ENV PATH="/root/.foundry/bin:$PATH"

WORKDIR /home/hyperlane

COPY testnet/hyperlane/configs /home/hyperlane/configs
COPY testnet/hyperlane/registry /home/hyperlane/registry
COPY testnet/hyperlane/scripts /home/hyperlane/scripts

ENTRYPOINT [ "scripts/docker-entrypoint.sh" ]
