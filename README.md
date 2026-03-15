# MPC Solana Wallet

A secure multi-party computation (MPC) based Solana wallet system that eliminates single points of failure through distributed key management and threshold signatures.

## Overview

This project implements a non-custodial Solana wallet using MuSig2 (Multi-Signature Scheme 2) protocol. Private keys are never held by a single party - instead, cryptographic key shares are distributed across multiple parties who collaboratively sign transactions without ever reconstructing the full private key.

## Architecture

The system consists of four main components:

### 1. Backend (`/backend`)
REST API server handling user authentication and wallet operations.

**Key Features:**
- User registration and authentication with JWT tokens
- Jupiter aggregator integration for token swaps
- SOL and SPL token balance queries
- Transaction management

**Endpoints:**
- `POST /api/v1/signup` - Create new user account
- `POST /api/v1/signin` - Authenticate user
- `GET /api/v1/user` - Get user details
- `POST /api/v1/quote` - Get swap quote from Jupiter
- `POST /api/v1/swap` - Execute token swap
- `GET /api/v1/sol-balance` - Check SOL balance
- `GET /api/v1/token-balance/{pubkey}/{mint}` - Check SPL token balance

### 2. MPC Service (`/mpc`)
Multi-party computation server implementing MuSig2 protocol for distributed key generation and signing.

**Key Features:**
- Distributed keypair generation
- MuSig2 threshold signature scheme
- Two-round signing protocol
- Signature aggregation and broadcasting

**Endpoints:**
- `POST /generate` - Generate individual keypair
- `POST /aggregate-keys` - Combine public keys into aggregate key
- `POST /agg-send-step1` - First round of signing (generate nonces)
- `POST /agg-send-step2` - Second round of signing (create partial signature)
- `POST /aggregate-signatures-broadcast` - Combine signatures and broadcast transaction

### 3. Indexer (`/indexer`)
Real-time Solana blockchain indexer using Yellowstone gRPC for account monitoring.

**Key Features:**
- Real-time account balance tracking
- Transaction monitoring
- Automatic balance updates in database
- Solana Geyser plugin integration

### 4. Store (`/store`)
Shared database layer providing data persistence across all services.

**Key Features:**
- PostgreSQL with SQLx
- User management
- Balance tracking
- Quote storage
- Public key registry

## How MPC Works

### Key Generation
1. Each party generates their own keypair independently
2. Public keys are shared with all participants
3. An aggregate public key is computed using MuSig2 key aggregation
4. The aggregate public key becomes the wallet address
5. No single party ever possesses the complete private key

### Transaction Signing (Two-Round Protocol)

**Round 1:**
- Each party generates random nonces (private and public)
- Public nonces are shared with all participants
- Each party keeps their private nonces secret

**Round 2:**
- Transaction is created using the aggregate public key
- Each party computes a partial signature using:
  - Their private key share
  - Their private nonces
  - All public nonces from other parties
  - The transaction message
- Partial signatures are shared

**Aggregation:**
- All partial signatures are combined into a single valid signature
- The final transaction is verified and broadcast to Solana

## Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Solana CLI tools (for testing)
- Access to Solana RPC endpoint (devnet/testnet/mainnet)

## Database Setup

Create a PostgreSQL database and run the following schema:

```sql
-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    public_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

-- Balances table
CREATE TABLE balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    amount NUMERIC(20, 9) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Quote responses table
CREATE TABLE quote_response (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Configuration

Create a `.env` file in the project root:

```env
DATABASE_URL=postgresql://username:password@localhost/dbname
```

## Running the Services

### Start Backend API
```bash
cd backend
cargo run
# Runs on http://127.0.0.1:8083
```

### Start MPC Service
```bash
cd mpc
cargo run
# Runs on http://127.0.0.1:8081
```

### Start Indexer
```bash
cd indexer
cargo run
# Connects to Yellowstone gRPC endpoint at http://127.0.0.1:10000
```

## Usage Example

### 1. Create User Account
```bash
curl -X POST http://127.0.0.1:8083/api/v1/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepassword"
  }'
```

### 2. Generate MPC Keys (3-of-3 example)

Generate three keypairs:
```bash
# Party 1
curl -X POST http://127.0.0.1:8081/generate

# Party 2
curl -X POST http://127.0.0.1:8081/generate

# Party 3
curl -X POST http://127.0.0.1:8081/generate
```

Aggregate the public keys:
```bash
curl -X POST http://127.0.0.1:8081/aggregate-keys \
  -H "Content-Type: application/json" \
  -d '{
    "keys": ["pubkey1", "pubkey2", "pubkey3"]
  }'
```

### 3. Sign and Send Transaction

Each party performs step 1:
```bash
curl -X POST http://127.0.0.1:8081/agg-send-step1 \
  -H "Content-Type: application/json" \
  -d '{
    "keypair": "base58_private_key"
  }'
```

Each party performs step 2:
```bash
curl -X POST http://127.0.0.1:8081/agg-send-step2 \
  -H "Content-Type: application/json" \
  -d '{
    "keypair": "base58_private_key",
    "amount": 0.1,
    "to": "recipient_pubkey",
    "memo": "Test transaction",
    "recent_block_hash": "latest_blockhash",
    "keys": ["pubkey1", "pubkey2", "pubkey3"],
    "first_messages": ["msg1", "msg2", "msg3"],
    "secret_state": "secret_from_step1"
  }'
```

Aggregate and broadcast:
```bash
curl -X POST http://127.0.0.1:8081/aggregate-signatures-broadcast \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 0.1,
    "to": "recipient_pubkey",
    "memo": "Test transaction",
    "recent_block_hash": "latest_blockhash",
    "keys": ["pubkey1", "pubkey2", "pubkey3"],
    "signatures": ["sig1", "sig2", "sig3"]
  }'
```

## Security Considerations

### Advantages
- **No Single Point of Failure**: Private key is never reconstructed
- **Threshold Security**: Requires multiple parties to sign
- **Non-Custodial**: Users maintain control through key shares
- **Censorship Resistant**: No central authority can block transactions

### Best Practices
- Store private key shares in separate secure environments
- Use secure communication channels for message passing
- Implement proper key share backup and recovery mechanisms
- Rotate nonces for each signing session
- Validate all inputs before processing
- Use hardware security modules (HSMs) for production key shares

## Technology Stack

- **Rust**: Core implementation language
- **Actix-Web**: HTTP server framework
- **SQLx**: Async PostgreSQL driver
- **multi-party-eddsa**: MuSig2 cryptographic library
- **Solana SDK**: Blockchain interaction
- **Yellowstone gRPC**: Real-time blockchain indexing
- **Jupiter API**: Token swap aggregation
- **bcrypt**: Password hashing
- **JWT**: Authentication tokens

## Development

### Run Tests
```bash
cargo test --workspace
```

### Build Release
```bash
cargo build --release --workspace
```

## Limitations

- Currently configured for Solana devnet
- Requires all parties to be online for signing
- No automatic key share recovery mechanism
- Fixed threshold (all parties must participate)

## Future Enhancements

- [ ] Dynamic threshold support (t-of-n signatures)
- [ ] Key share backup and recovery system
- [ ] Mobile application integration
- [ ] Hardware wallet integration
- [ ] Mainnet production deployment
- [ ] Additional blockchain support
- [ ] Offline signing capability
- [ ] Advanced transaction batching

## Contributing

Contributions are welcome! Please follow these guidelines:
1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is provided as-is for educational and research purposes.

## Acknowledgments

- ZenGo-X for the multi-party-eddsa library
- Solana Foundation for the blockchain infrastructure
- Jupiter for DEX aggregation services

## Support

For questions or issues, please open a GitHub issue or contact the development team.
