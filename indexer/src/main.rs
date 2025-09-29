use anyhow::Result;
use std::collections::HashMap;
use store::FindUserIdViaPubkeyRequest;
use store::Store;
use store::UpdateBalanceReq;
use tokio_stream::StreamExt;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::SubscribeRequestFilterTransactions;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::SubscribeRequest;
use yellowstone_grpc_proto::prelude::SubscribeRequestFilterAccounts;

#[tokio::main]
async fn main() -> Result<()> {
    let s = Store::new().await;

    let all_pub_keys = s.get_all_pub_keys().await?;
    println!("{:?}", all_pub_keys.public_keys);

    //  connected to  local geyser plugin on testnet
    let endpoint = "http://127.0.0.1:10000";

    let mut client = GeyserGrpcClient::build_from_shared(endpoint)?
        .connect()
        .await?;

    let accounts_filter = SubscribeRequestFilterAccounts {
        account: all_pub_keys.public_keys.clone(),
        owner: vec![],
        filters: vec![],
        nonempty_txn_signature: None,
    };

    let mut accounts_map = HashMap::new();
    accounts_map.insert("my_accounts".to_string(), accounts_filter);

    let tx_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: None,
        signature: None,
        account_include: all_pub_keys.public_keys,
        account_exclude: vec![],
        account_required: vec![],
    };

    let mut tx_map = HashMap::new();
    tx_map.insert("my_txs".to_string(), tx_filter);

    let request = SubscribeRequest {
        accounts: accounts_map,
        transactions: tx_map,
        ..Default::default()
    };

    let (_sink, mut stream) = client.subscribe_with_request(Some(request)).await?;

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(update) => match update.update_oneof {
                Some(UpdateOneof::Account(acc_update)) => {
                    if let Some(account) = acc_update.account {
                        let pubkey_str = bs58::encode(&account.pubkey).into_string();
                        let sol = account.lamports as f64 / 1_000_000_000.0;

                        let req = FindUserIdViaPubkeyRequest {
                            pubkey: pubkey_str.clone(),
                        };

                        let user_id_res = s.find_user_id_via_pubkey(req).await;

                        match user_id_res {
                            Ok(user) => {
                                let req2 = UpdateBalanceReq {
                                    user_id: user.id.to_string(),
                                    new_bal: sol,
                                };

                                let _ = s.update_balance(req2).await;
                            }
                            Err(e) => {
                                eprintln!("Error finding user id: {}", e);
                            }
                        }
                        println!(" Account update: {} ({} SOL)", pubkey_str, sol);
                    }
                }

                _ => {}
            },
            Err(err) => eprintln!(" Stream error: {:?}", err),
        }
    }

    Ok(())
}
