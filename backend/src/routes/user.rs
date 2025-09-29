use actix_web::{HttpResponse, Result, web};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use store::{
    ExistsUserRequest, Store, UserError, UserExistError,
    user::{self, CreateUserRequest},
};

use crate::authorization::AuthUser;

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    email:String
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    message: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
}

#[actix_web::post("/api/v1/signup")]
pub async fn sign_up(
    req: web::Json<SignUpRequest>,
    store: web::Data<Store>,
) -> Result<HttpResponse> {
    //todo call mpc and get aggregate keys-sahre private key

    let create_req = CreateUserRequest {
        email: req.email.clone(),
        password: req.password.clone(),
        public_key: "sdfghjkl".to_string(),
    };

    match store.create_user(create_req).await {
        Ok(user) => {
            println!("{:?}", user);
            
            let my_claim = Claims {
                sub: user.id.to_string(),
                exp: (Utc::now() + Duration::days(30)).timestamp(),
            };

            let jwt_token = encode(
                &Header::default(),
                &my_claim,
                &EncodingKey::from_secret("gfcvjkcnkxc".as_ref()),
            )
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("JWT encode error: {}", e))
            })?;

            let res = SignupResponse {
                message: "signed up successfully".to_string(),
                token: jwt_token,
            };

            Ok(HttpResponse::Ok().json(res))
        }
        Err(UserError::UserExists) => Ok(HttpResponse::Conflict().body("User already exists")),
        Err(UserError::InvalidInput(msg)) => Ok(HttpResponse::BadRequest().body(msg)),

        Err(UserError::DatabaseError(msg)) => Ok(HttpResponse::InternalServerError().body(msg)),
    }
}

#[actix_web::post("/api/v1/signin")]
pub async fn sign_in(
    req: web::Json<SignInRequest>,
    store: web::Data<Store>,
) -> Result<HttpResponse> {
    let create_req = ExistsUserRequest {
        email: req.email.clone(),
        password: req.password.clone(),
    };

    match store.exists_user(create_req).await {
        Ok(exist_user) => {
            println!("{:?}", exist_user);

            let my_claim = Claims {
                sub: exist_user.id.to_string(),
                exp: (Utc::now() + Duration::days(30)).timestamp(),
            };
            let jwt_token = encode(
                &Header::default(),
                &my_claim,
                &EncodingKey::from_secret("gfcvjkcnkxc".as_ref()),
            )
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("JWT encode error: {}", e))
            })?;

            let res = AuthResponse { token: jwt_token };

            Ok(HttpResponse::Ok().json(res))
        }
        Err(UserExistError::UserNotExists) => Ok(HttpResponse::Unauthorized().body("user not found")),

        Err(UserExistError::DatabaseError(msg)) => {
            Ok(HttpResponse::InternalServerError().body(msg))
        }

        Err(UserExistError::InvalidInput(msg)) => Ok(HttpResponse::Unauthorized().body(msg)),
    }
}

#[actix_web::get("/api/v1/user")]
pub async fn get_user(user: AuthUser, store: web::Data<Store>) -> Result<HttpResponse> {

    let user_id = user.user_id;

    match store.get_user_email(user::GetUserEmailReq { u_id: user_id }).await {
        Ok(user) => {
            Ok(HttpResponse::Ok().json(user))
        }
        Err(_) => {
            Ok(HttpResponse::InternalServerError().body("Failed to fetch user"))
        }
    }
}

