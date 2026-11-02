# Builds the Hyperlane CLI from the celestiaorg fork of hyperlane-monorepo so the
# docker testnet always deploys with the fork's latest main. EVM contract addresses
# depend on the exact deploy sequence of the CLI, so nothing in this repository
# hardcodes them; hyperlane-init generates them on every run, see
# scripts/docker-entrypoint.sh.
ARG NODE_VERSION=26

FROM node:${NODE_VERSION}-slim AS builder

ARG HYPERLANE_MONOREPO_REPO=celestiaorg/hyperlane-monorepo
ARG HYPERLANE_MONOREPO_REF=main

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash ca-certificates curl g++ git jq make python3 unzip \
    && rm -rf /var/lib/apt/lists/*

# The ref file changes whenever the fork's branch moves, which invalidates the
# layer cache from here on, so a rebuild always picks up the latest commit.
ADD https://api.github.com/repos/${HYPERLANE_MONOREPO_REPO}/git/refs/heads/${HYPERLANE_MONOREPO_REF} /tmp/hyperlane-monorepo-ref.json
RUN git clone --depth 1 --branch "${HYPERLANE_MONOREPO_REF}" "https://github.com/${HYPERLANE_MONOREPO_REPO}.git" /hyperlane-monorepo

WORKDIR /hyperlane-monorepo

# Foundry at the version the fork pins; forge is needed to build the solidity package.
RUN FOUNDRY_VERSION=$(cat solidity/.foundryrc) \
    && curl --fail -L "https://github.com/foundry-rs/foundry/releases/download/${FOUNDRY_VERSION}/foundry_${FOUNDRY_VERSION}_linux_$(dpkg --print-architecture).tar.gz" \
    | tar -xzC /usr/local/bin forge cast

# pnpm at the version the fork pins in package.json.
RUN npm install -g "pnpm@$(node -p "require('./package.json').packageManager.split('@')[1].split('+')[0]")"

# ccip-server runs prisma generate on install and needs some DATABASE_URL.
ENV DATABASE_URL="postgresql://placeholder:placeholder@localhost:5432/placeholder"
RUN pnpm install --frozen-lockfile --filter "@hyperlane-xyz/cli..." \
    && pnpm turbo run build --filter="@hyperlane-xyz/cli..." \
    && pnpm store prune \
    && rm -rf .git

FROM node:${NODE_VERSION}-slim

RUN apt-get update && apt-get install -y --no-install-recommends bash ca-certificates jq \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /hyperlane-monorepo /hyperlane-monorepo
RUN printf '#!/bin/sh\nexec node /hyperlane-monorepo/typescript/cli/dist/cli.js "$@"\n' > /usr/local/bin/hyperlane \
    && chmod +x /usr/local/bin/hyperlane \
    && hyperlane --version

WORKDIR /home/hyperlane

COPY testnet/hyperlane/configs /home/hyperlane/configs
COPY testnet/hyperlane/registry /home/hyperlane/registry
COPY testnet/hyperlane/relayer-config.template.json /home/hyperlane/relayer-config.template.json
COPY testnet/hyperlane/scripts /home/hyperlane/scripts

ENTRYPOINT [ "scripts/docker-entrypoint.sh" ]
