use actix_web::{HttpResponse, Result, web};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use store::{GetUserEmailReq, Store};
use uuid::Uuid;

use crate::authorization::AuthUser;

#[derive(Deserialize)]
pub struct QuoteRequest {
    inputMint: String,
    outputMint: String,
    inAmount: u64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct SwapInfo {
    pub ammKey: String,
    pub label: String,
    pub inputMint: String,
    pub outputMint: String,
    pub inAmount: String,
    pub outAmount: String,
    pub feeAmount: String,
    pub feeMint: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RoutePlan {
    pub swapInfo: SwapInfo,
    pub percent: u32,
    pub bps: u32,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct QuoteResponse {
    pub inputMint: String,
    pub inAmount: String,
    pub outputMint: String,
    pub outAmount: String,
    pub otherAmountThreshold: String,
    pub swapMode: String,
    pub slippageBps: u32,
    pub platformFee: Option<String>,
    pub priceImpactPct: String,
    pub routePlan: Vec<RoutePlan>,
    pub contextSlot: i64,
    pub timeTaken: f64,
    pub swapUsdValue: String,
    pub simplerRouteUsed: bool,
    pub mostReliableAmmsQuoteReport: Value,
    pub useIncurredSlippageForQuoting: Option<bool>,
    pub otherRoutePlans: Option<Value>,
    pub aggregatorVersion: Option<String>,
    pub loadedLongtailToken: bool,
}

#[derive(Deserialize)]
pub struct SwapRequest {
    id: String,
}

#[derive(Serialize)]
pub struct SwapResponse {}

#[derive(Serialize)]
pub struct BalanceResponse {
    balance: u64,
}

#[derive(Serialize)]
pub struct TokenBalanceResponse {}

#[actix_web::post("/api/v1/quote")]
pub async fn quote(
    req: web::Json<QuoteRequest>,
    store: web::Data<Store>,
) -> Result<HttpResponse, actix_web::Error> {
    let url = format!(
        "https://lite-api.jup.ag/swap/v1/quote?inputMint={}&outputMint={}&amount={}",
        req.inputMint, req.outputMint, req.inAmount
    );

    let quote: QuoteResponse = reqwest::get(url)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .json::<QuoteResponse>()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let quote_json =
        serde_json::to_value(&quote).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    match store.create_quote(quote_json).await {
        Ok(quote_id) => Ok(HttpResponse::Ok().json(quote_id)),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[actix_web::post("/api/v1/swap")]
pub async fn swap(
    req: web::Json<SwapRequest>,
    store: web::Data<Store>,
    user: AuthUser,
) -> Result<HttpResponse> {
    let response = SwapResponse {};

    let client = reqwest::Client::new();

    let quote_id =
        Uuid::parse_str(&req.id).map_err(|_| actix_web::error::ErrorBadRequest("Invalid UUID"))?;

    match store.get_quote(quote_id).await {
        Ok(quote_data) => {
            let wrapped_request = serde_json::json!({
                "userPublicKey": "CbK54Vf2FZYgwP6z533xJg34LL7qJyHig75bmfouQUsc",
                "quoteResponse": &quote_data
            });

            let request = client
                .post("https://lite-api.jup.ag/swap/v1/swap")
                .json(&wrapped_request)
                .send()
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            let res_json: Value = request
                .json()
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            println!("{:?}", res_json);

            Ok(HttpResponse::Ok().json(res_json))

            // Ok(HttpResponse::Ok().json(quote_data))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[actix_web::get("/api/v1/sol-balance")]
pub async fn sol_balance(store: web::Data<Store>, user: AuthUser) -> Result<HttpResponse> {
    let user_id = user.user_id;
    match store
        .get_user_sol_bal(GetUserEmailReq { u_id: user_id })
        .await
    {
        Ok(user_bal) => Ok(HttpResponse::Ok().json(user_bal)),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[actix_web::get("/api/v1/token-balance/{pubkey}/{mint}")]
pub async fn token_balance() -> Result<HttpResponse> {
    let response = TokenBalanceResponse {};

    Ok(HttpResponse::Ok().json(response))
}

#[actix_web::post("/api/v1/send-sol/{pubkey}")]
pub async fn send_sol() -> Result<HttpResponse> {
    let response = TokenBalanceResponse {};

    Ok(HttpResponse::Ok().json(response))
}
