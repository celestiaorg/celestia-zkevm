## Overview

The `evm-prover` service is a simple gRPC service designed to serve ZK proofs to clients.
It encapsulates the SP1 programs maintained under `sp1`, and uses the `sp1_sdk::ProverClient` in order to interface with them.
