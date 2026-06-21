# StealthPay 

[![Rust](https://img.shields.io/badge/Rust-000000?style=flat\&logo=rust\&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-Async_Runtime-green)](https://tokio.rs/)
[![Axum](https://img.shields.io/badge/Axum-Web_Framework-blue)](https://github.com/tokio-rs/axum)
[![Solana](https://img.shields.io/badge/Solana-9945FF?style=flat\&logo=solana\&logoColor=white)](https://solana.com/)
[![Next.js](https://img.shields.io/badge/Next.js-000000?style=flat\&logo=nextdotjs\&logoColor=white)](https://nextjs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat\&logo=typescript\&logoColor=white)](https://www.typescriptlang.org/)

A privacy-focused payment protocol for Solana that combines stealth addresses, transaction relaying, and gas abstraction to enable private transfers without exposing a recipient's primary wallet address on-chain.

StealthPay allows recipients to publish a reusable meta-address while senders generate unique one-time destination addresses for every payment. A relayer infrastructure handles transaction broadcasting and fee sponsorship, creating a smoother and more private payment experience.

---


<img src="https://github.com/user-attachments/assets/bea2a7cf-ddcb-48c5-a83c-bc83cb080a1d" alt="Generate Privacy Handle" width="900" />


Example:

<img src="https://github.com/user-attachments/assets/13338775-accf-4684-8a5e-6794bcf0b0b5" alt="Deposit" width="900" />


---

## What It Does

Traditional Solana payments expose a recipient's public wallet address.

StealthPay introduces a privacy layer where:

* Recipients generate a Meta Address
* Senders derive a unique one-time Stealth Address
* Every payment goes to a different destination address
* Recipients can recover ownership of the funds
* Relayers can sponsor transaction fees

```text
Recipient  ->  Meta Address  ->  Sender   ->  Stealth Address Generation   ->  Relayer   ->  Solana Network
```

---

## Motivation

Most blockchain transactions are fully transparent.

Anyone can:

* Track wallet balances
* Link multiple transactions together
* Monitor payment activity
* Build transaction histories

StealthPay reduces wallet traceability by generating a unique destination address for every payment while preserving the recipient's ability to recover and spend the funds.

---

## Features

### Stealth Address Generation

Recipients create a Meta Address consisting of:

```text
Public View Key
Public Spend Key
```

Senders use the Meta Address and an ephemeral keypair to derive a unique destination address.

```text
Meta Address
      +
Ephemeral Key
      ↓
Stealth Address
```

Every transfer results in a different address.

---

### ECDH-Based Shared Secret Derivation

The protocol uses Elliptic Curve Diffie-Hellman (ECDH) over Curve25519.

```text
Sender:
r × A

Recipient:
a × R
```

Both parties derive the same shared secret independently.

---

### One-Time Payment Addresses

Each payment generates:

```text
R = Ephemeral Public Key
P = Stealth Public Key
```

Result:

```text
Payment 1 → Address A
Payment 2 → Address B
Payment 3 → Address C
```

Even when sent to the same recipient.

---

### Memo-Based Key Discovery

The sender publishes the ephemeral public key inside a Solana Memo instruction.

Recipients scan transaction memos to discover stealth payments intended for them.

```text
Transaction
    ├── Transfer
    └── Memo (Ephemeral Public Key)
```

---

### Relayer Infrastructure

A relayer service validates transactions before broadcasting them.

Responsibilities include:

* Transaction validation
* Fee payment verification
* Fee sponsorship
* Transaction broadcasting

```text
User
  │
  ▼
Relayer
  │
  ▼
Solana
```

---

### Payment Detection

Recipients can scan transactions and determine whether a payment belongs to them.

The detection flow:

```text
Read Memo   ->  Recover Ephemeral Key   ->  Derive Shared Secret   ->   Reconstruct Expected Address   ->  Match Found
```

---

## Architecture

```text
Frontend (Next.js)   ->  Axum API   ->  Stealth Engine   ->  Relayer    ->  Solana
```

---

## Backend Endpoints

### Health Check

```http
GET /health
```

Returns:

```json
{
  "status": "ok"
}
```

---

### Create Meta Address

```http
POST /meta-address/create
```

Generates a new recipient meta-address.

---

### Send Stealth Payment

```http
POST /send
```

Builds a stealth transaction from a recipient's meta-address.

---

### Relay Transaction

```http
POST /relay
```

Validates and broadcasts transactions through the relayer.

---

### Scan Payments

```http
GET /payments
```

Scans transactions for stealth payments intended for a recipient.

---

## Tech Stack

### Backend

* Rust
* Tokio
* Axum
* Curve25519-Dalek
* Solana SDK
* Serde

### Frontend

* Next.js
* React
* TypeScript
* Tailwind CSS

### Cryptography

* Curve25519
* ECDH
* BLAKE3
* SHA-256

---

## Project Structure

```text
stealthpay/
│
├── backend/
│   ├── src/
│   │   ├── routes/
│   │   ├── stealth/
│   │   ├── relayer/
│   │   └── main.rs
│   │
│   └── Cargo.toml
│
├── frontend/
│   ├── src/
│   ├── app/
│   ├── components/
│   └── package.json
│
└── README.md
```

---

## Running the Project

### Clone Repository

```bash
git clone https://github.com/your-username/stealthpay.git

cd stealthpay
```

---

## Start Backend

```bash
cd backend

cargo build

cargo run
```

Backend starts on:

```text
http://localhost:8080
```

---

## Start Frontend

```bash
cd frontend

pnpm install

pnpm run dev
```

Frontend starts on:

```text
http://localhost:3000
```

---

## Local Development

Run both services:

```text
Frontend → http://localhost:3000

Backend  → http://localhost:8080
```

The frontend communicates with the backend through HTTP API endpoints.

---

## Current Status

### Completed

* Stealth address generation
* Meta address generation
* ECDH shared secret derivation
* Memo-based ephemeral key publication
* Payment detection logic
* Relayer validation architecture
* Axum backend
* Next.js frontend

### In Progress

* Transaction fee sponsorship
* Full relayer signing flow
* Wallet integration
* Automated stealth payment scanning

---

## Future Improvements

* MPC-based relayer signing
* Secure enclave integration
* Sponsored gas payments
* Multi-relayer architecture
* Wallet adapter integration
* Mobile support
* Encrypted payment metadata
* Payment notifications

---

## License

MIT License

---

Built with Rust, Solana, and an unreasonable amount of time spent convincing cryptography and frontend tooling to coexist peacefully.
