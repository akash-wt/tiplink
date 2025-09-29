use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload};
use futures_util::future::{Ready, ready};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
}

pub struct AuthUser {
    pub user_id: String,
}

impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(token) = auth_header.to_str() {
                let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

                let decode = decode::<Claims>(
                    token.trim(),
                    &DecodingKey::from_secret(b"gfcvjkcnkxc"),
                    &validation,
                );

                return match decode {
                    Ok(data) => ready(Ok(AuthUser {
                        user_id: data.claims.sub,
                    })),

                    Err(_) => ready(Err(ErrorUnauthorized("Invalid or expired token"))),
                };
            }
        }

        ready(Err(ErrorUnauthorized("Unauthorized")))
    }
}
