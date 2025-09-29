use crate::error::Error;
use crate::serialization::{AggMessage1, SecretAggStepOne};
use crate::serialization::{PartialSignature, Serialize as CustomSerialize};
use actix_web::web::Json;
use actix_web::{App, HttpResponse, HttpServer, web::post};
use serde::Deserialize;
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::signer::Signer;
use solana_sdk::{
    instruction::Instruction, message::Message, native_token, pubkey::Pubkey, signature::Keypair,
    system_instruction, transaction::Transaction,
};
use std::str::FromStr;
use tokio::task;

pub mod error;
pub mod serialization;
pub mod tss;

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    HttpServer::new(|| {
        App::new()
            .route("/generate", post().to(generate))
            // .route("/send-single", post().to(send_single))
            .route("/aggregate-keys", post().to(aggregate_keys))
            .route("/agg-send-step1", post().to(agg_send_step1))
            .route("/agg-send-step2", post().to(agg_send_step2))
            .route(
                "/aggregate-signatures-broadcast",
                post().to(aggregate_signatures_broadcast),
            )
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}

#[derive(Serialize)]
struct KeypairResponse {
    secret: String,
    public: String,
}
#[derive(Serialize)]
struct AggregatedKey {
    agg_ey: String,
}

#[derive(Deserialize)]
struct AggregateKeysRequest {
    keys: Vec<String>,
}
#[derive(Deserialize)]
struct AggSendStep1Request {
    keypair: String,
}
#[derive(Serialize)]
struct AggSendStep1Response {
    first_msg: String,
    secret: String,
}
#[derive(Deserialize)]
struct AggregateSignaturesBroadcast {
    amount: f64,
    to: String,
    memo: Option<String>,
    recent_block_hash: String,
    keys: Vec<String>,
    signatures: Vec<String>,
}

#[derive(Deserialize)]
pub struct AggSendStep2Request {
    keypair: String,             // private key
    amount: f64,                 // SOL amount to send
    to: String,                  // recipient pubkey as string
    memo: Option<String>,        // optional memo
    recent_block_hash: String,   // latest blockhash
    keys: Vec<String>,           // all participants' public keys
    first_messages: Vec<String>, // first messages from step 1 of other participants
    secret_state: String,        // secret state from step 1
}

#[derive(Serialize)]
pub struct AggSendStep2Response {
    partial_signature: String,
}

async fn generate() -> Result<HttpResponse, Error> {
    let keypair = Keypair::generate(&mut rand07::thread_rng());

    let response = KeypairResponse {
        secret: keypair.to_base58_string(),
        public: keypair.pubkey().to_string(),
    };

    

    Ok(HttpResponse::Ok().json(response))
}

// async fn send_single(keypair, amount, to, net, memo ) -> Result<HttpResponse, Error> {
//     let rpc_client = RpcClient::new(net.get_cluster_url().to_string());
//     let mut tx = create_unsigned_transaction(amount, &to, memo, &keypair.pubkey());
//     let recent_hash = rpc_client
//         .get_latest_blockhash()
//         .map_err(Error::RecentHashFailed)?;
//     tx.sign(&[&keypair], recent_hash);
//     let sig = rpc_client
//         .send_transaction(&tx)
//         .map_err(Error::SendTransactionFailed)?;
//     println!("Transaction ID: {}", sig);
//     rpc_client
//         .confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
//         .map_err(Error::ConfirmingTransactionFailed)?;

//     Ok(HttpResponse::Ok().body("Hello, world!"))
// }

async fn aggregate_keys(req: Json<AggregateKeysRequest>) -> Result<HttpResponse, Error> {
    let mut pubkeys = Vec::new();

    for k in &req.keys {
        let pk = Pubkey::from_str(k).map_err(|_| Error::WrongNetwork(k.clone()))?;
        pubkeys.push(pk);
    }

    let aggkey = tss::key_agg(pubkeys.clone(), None)?;
    let aggpubkey = Pubkey::new(&*aggkey.agg_public_key.to_bytes(true));

    let response = AggregatedKey {
        agg_ey: aggpubkey.to_string(),
    };
    Ok(HttpResponse::Ok().json(response))
}

async fn agg_send_step1(req: Json<AggSendStep1Request>) -> Result<HttpResponse, Error> {
    let secret_bytes = bs58::decode(&req.keypair)
        .into_vec()
        .map_err(Error::BadBase58)?;
    let keypair = Keypair::from_bytes(&secret_bytes).map_err(Error::WrongKeyPair)?;

    let (first_msg, secret) = tss::step_one(keypair);
    let response = AggSendStep1Response {
        first_msg: first_msg.serialize_bs58(), // send to all other parties
        secret: secret.serialize_bs58(), // keep this a secret, and pass it back to `agg-send-step-two
    };

    Ok(HttpResponse::Ok().json(response))
}

async fn agg_send_step2(req: Json<AggSendStep2Request>) -> Result<HttpResponse, Error> {
    // Decode sender keypair
    let secret_bytes = bs58::decode(&req.keypair)
        .into_vec()
        .map_err(Error::BadBase58)?;

    let keypair = Keypair::from_bytes(&secret_bytes).map_err(Error::WrongKeyPair)?;

    // Decode recipient pubkeys
    let to_pubkey = Pubkey::from_str(&req.to).map_err(|_| Error::WrongNetwork(req.to.clone()))?;

    // Decode participant pubkeys
    let pubkeys: Vec<Pubkey> = req
        .keys
        .iter()
        .map(|k| Pubkey::from_str(k).map_err(|_| Error::WrongNetwork(k.clone())))
        .collect::<Result<_, _>>()?;

    // Decode recent blockhash
    let recent_hash: Hash = Hash::from_str(&req.recent_block_hash)
        .map_err(|_| Error::WrongNetwork(req.recent_block_hash.clone()))?;

    // Deserialize first messages
    let first_messages: Vec<AggMessage1> = req
        .first_messages
        .iter()
        .map(|m| {
            let bytes = bs58::decode(m).into_vec().map_err(Error::BadBase58)?;
            AggMessage1::deserialize(&bytes).map_err(|e| Error::DeserializationFailed {
                error: e,
                field_name: "first_message",
            })
        })
        .collect::<Result<_, _>>()?;

    let secret_state_bytes = bs58::decode(&req.secret_state)
        .into_vec()
        .map_err(Error::BadBase58)?;

    let secret_state: SecretAggStepOne = SecretAggStepOne::deserialize(&secret_state_bytes)
        .map_err(|e| Error::DeserializationFailed {
            error: e,
            field_name: "secret_state",
        })?;

    let partial_sig = tss::step_two(
        keypair,
        req.amount,
        to_pubkey,
        req.memo.clone(),
        recent_hash,
        pubkeys,
        first_messages,
        secret_state,
    )?;

    // Return Base58 encoded partial signature
    let response = AggSendStep2Response {
        partial_signature: partial_sig.serialize_bs58(),
    };

    println!("Partial signature: {}", partial_sig.serialize_bs58());

    Ok(HttpResponse::Ok().json(response))
}

async fn aggregate_signatures_broadcast(
    req: Json<AggregateSignaturesBroadcast>,
) -> Result<HttpResponse, Error> {
    let to_pubkey = Pubkey::from_str(&req.to).map_err(|_| Error::WrongNetwork(req.to.clone()))?;

    let pubkeys: Vec<Pubkey> = req
        .keys
        .iter()
        .map(|k| Pubkey::from_str(k).map_err(|_| Error::WrongNetwork(k.clone())))
        .collect::<Result<_, _>>()?;

    let recent_hash: Hash = Hash::from_str(&req.recent_block_hash)
        .map_err(|_| Error::WrongNetwork(req.recent_block_hash.clone()))?;

    // Decode partial signatures from base58 strings
    let partial_signatures: Vec<PartialSignature> = req
        .signatures
        .iter()
        .map(|s| {
            let bytes = bs58::decode(s).into_vec().map_err(Error::BadBase58)?;
            PartialSignature::deserialize(&bytes).map_err(|e| Error::DeserializationFailed {
                error: e,
                field_name: "partial_signature",
            })
        })
        .collect::<Result<_, _>>()?;

    let result = task::spawn_blocking(move || {
        let tx = tss::sign_and_broadcast(
            req.amount,
            to_pubkey,
            req.memo.clone(),
            recent_hash,
            pubkeys,
            partial_signatures,
        )?;

        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let sig = rpc_client
            .send_transaction(&tx)
            .map_err(Error::SendTransactionFailed)?;

        println!("Transaction ID: {}", sig);

        rpc_client
            .confirm_transaction_with_spinner(&sig, &recent_hash, rpc_client.commitment())
            .map_err(Error::ConfirmingTransactionFailed)?;

        Ok(())
    })
    .await;

    match result {
        Ok(inner_result) => {
            match inner_result {
                Ok(sig_str) => {
                    Ok(HttpResponse::Ok().body(format!("Transaction confirmed: {:?}", sig_str)))
                }
                // The blocking operation failed with a known error.
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            eprintln!("The spawned task failed unexpectedly: {:?}", e);
            Err(Error::WrongNetwork(e.to_string()))
        }
    }
}

pub fn create_unsigned_transaction(
    amount: f64,
    to: &Pubkey,
    memo: Option<String>,
    payer: &Pubkey,
) -> Transaction {
    let amount = native_token::sol_to_lamports(amount);
    let transfer_ins = system_instruction::transfer(payer, to, amount);
    let msg = match memo {
        None => Message::new(&[transfer_ins], Some(payer)),
        Some(memo) => {
            let memo_ins = Instruction {
                program_id: spl_memo::id(),
                accounts: Vec::new(),
                data: memo.into_bytes(),
            };
            Message::new(&[transfer_ins, memo_ins], Some(payer))
        }
    };
    Transaction::new_unsigned(msg)
}
