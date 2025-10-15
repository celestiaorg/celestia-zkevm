# Local Testnet Deployment (Single Machine — CPU or GPU)

This setup lets you deploy and run a local testnet that includes:

- **ev block prover (ZK)**
- **hyperlane message prover (ZK)**
- **hyperlane relayer**
- **celestia validator**
- **celestia bridge**
- **ev reth**

---

## 1. Start the Testnet

To launch all components on your local machine:

```bash
make start
```

> **Note:**  
> Due to increased block times, deploying all Hyperlane contracts takes about **4200+ seconds (~70 minutes)**.  
> This delay is expected and unavoidable for now. Once everything is up, all services will run continuously.

---

## 2. Run the Prover Service

After the `hyperlane-init` container finishes deploying all contracts, start the prover service:

```bash
RUST_LOG="e2e=debug" cargo run -p e2e --bin service --release
```

- Use `debug` to see detailed output.  
- Use `info` for less verbose logs.

The prover will begin generating ZK proofs for blocks. If your CPU or GPU is strong enough, it should stay within **1–2 blocks behind the chain head**.

**Performance notes:**
- Tested on an M3 Max MacBook Pro CPU: maintained a fixed lag of 1–2 blocks (with minimal transactions).  
- GPU mode scales better and supports parallel (network) proving, though that mode is still untested.

---

## 3. Monitor the Network

Once the network is running, open separate terminal splits and monitor logs:

```bash
docker logs -f reth
docker logs -f ev-node-evm-single
```

**What to watch:**
- **reth** shows block execution details — block numbers and transaction counts.  
- **ev-node** shows Evolve block data submissions to Celestia.  
  This helps catch synchronization issues early.

If `reth` prints a **`WARNING` about invalid transaction nonces**, the network is corrupted.  
Currently, there’s no protection against duplicate transactions across different blocks.  
This will not occur if you use the provided scripts or wait for finalization when submitting manually.

---

## 4. Bridge Tokens from Celestia → Evolve

Run:

```bash
make transfer
```

This sends tokens from Celestia to Evolve.

**Timing:**
- Celestia block time: 5 minutes  
- EVM block time: 1 minute  
- Total expected time: ~6–7 minutes

You can watch logs or the `ev-prover` output to see when the transfer is included in a proven EVM block.

---

## 5. Bridge Tokens Back (Evolve → Celestia)

To bridge tokens back using the ZKISM in a permissionless way:

```bash
make transfer-back
```

This triggers a Hyperlane deposit in the Mailbox, which will be proven by the `ev-prover` once it has proven the Celestia block containing the EVM transaction.

When the block and message proof for that inclusion height are successfully submitted, the transaction executes and your **Tia balance on Celestia** is increased.

---

## 6. Retrieve and Check Balances

**Retrieve the `default` key:**

```bash
docker exec -it celestia-validator /bin/bash
celestia-appd keys list
```

**Check the balance of the `default` key:**

```bash
docker exec -it celestia-validator /bin/bash
celestia-appd query bank balance celestia... utia
```

Replace `celestia...` with the address of the `default` key from the keyring.
