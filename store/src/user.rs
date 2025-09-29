use std::str::FromStr;

use crate::Store;
use bcrypt::verify;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::{BigDecimal, Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub created_at: String,
}
#[derive(Debug, Clone)]
pub struct ExistUser {
    pub id: String,
}
#[derive(Debug, Serialize)]
pub struct GetUserSolBalRes {
    pub balance: f64,
}

#[derive(Debug)]
pub struct ExistsUserRequest {
    pub email: String,
    pub password: String,
}
#[derive(Debug)]
pub struct FindUserIdViaPubkeyRequest {
    pub pubkey: String,
}
#[derive(Debug)]
pub struct UpdateBalanceReq {
    pub user_id: String,
    pub new_bal: f64,
}

#[derive(Debug)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub public_key: String,
}
#[derive(Debug, Clone)]
pub struct GetAllPubKeysRes {
    pub public_keys: Vec<String>,
}
#[derive(Debug)]
pub struct GetUserEmailReq {
    pub u_id: String,
}
#[derive(Debug, Serialize)]
pub struct GetUserEmailRes {
    pub email: String,
}

#[derive(Debug)]
pub enum UserError {
    UserExists,
    InvalidInput(String),
    DatabaseError(String),
}
#[derive(Debug)]
pub enum UserExistError {
    UserNotExists,
    InvalidInput(String),
    DatabaseError(String),
}

#[derive(Debug)]
pub enum QuoteError {
    DatabaseError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub id: Uuid,
}

impl std::fmt::Display for QuoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuoteError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}
impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::UserExists => write!(f, "User already exists"),
            UserError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            UserError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::fmt::Display for UserExistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserExistError::UserNotExists => write!(f, "User not exists"),
            UserExistError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            UserExistError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for UserError {}

impl Store {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
        // Validate email format
        if !request.email.contains('@') {
            return Err(UserError::InvalidInput("Invalid email format".to_string()));
        }

        // Validate password length
        if request.password.len() < 6 {
            return Err(UserError::InvalidInput(
                "Password must be at least 6 characters".to_string(),
            ));
        }

        // Check if user already exists
        let existing_user = sqlx::query!("SELECT id FROM users WHERE email = $1", request.email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if existing_user.is_some() {
            return Err(UserError::UserExists);
        }

        // Hash the password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::DatabaseError(format!("Password hashing failed: {}", e)))?;

        // Generate user ID and timestamp
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();

        // Insert user into database
        sqlx::query!(
            "INSERT INTO users (id, email, password,created_at, public_key) VALUES ($1, $2, $3,$4,$5)",
            user_id,
            request.email,
            password_hash,
            created_at,
            request.public_key
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Return the created user
        let user = User {
            id: user_id,
            email: request.email,
            created_at: created_at.to_rfc3339(),
        };

        Ok(user)
    }

    pub async fn exists_user(
        &self,
        request: ExistsUserRequest,
    ) -> Result<ExistUser, UserExistError> {
        let existing_user = sqlx::query!(
            "SELECT id , password FROM users WHERE email = $1",
            request.email,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserExistError::DatabaseError(e.to_string()))?;

        let user = match existing_user {
            Some(user) => user,
            None => return Err(UserExistError::UserNotExists),
        };

        let is_valid = verify(&request.password, &user.password)
            .map_err(|e| UserExistError::DatabaseError(format!("wrong password {}", e)))?;

        if !is_valid {
            return Err(UserExistError::InvalidInput("wrong password".to_string()));
        }

        Ok(ExistUser {
            id: user.id.to_string(),
        })
    }

    pub async fn create_quote(&self, request: Value) -> Result<QuoteResponse, QuoteError> {
        let id: Uuid =
            sqlx::query_scalar("INSERT INTO  quote_response (data) VALUES ( $1 )  RETURNING id ")
                .bind(Json(&request))
                .fetch_one(&self.pool)
                .await
                .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(QuoteResponse { id })
    }

    pub async fn get_quote(&self, quote_id: Uuid) -> Result<Value, QuoteError> {
        let row_data = sqlx::query!(
            r#"SELECT data as "data!: Value" FROM quote_response  WHERE id= $1"#,
            quote_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QuoteError::DatabaseError(e.to_string()))?;

        Ok(row_data.data)
    }

    pub async fn find_user_id_via_pubkey(
        &self,
        request: FindUserIdViaPubkeyRequest,
    ) -> Result<ExistUser, UserExistError> {
        let existing_user = sqlx::query!(
            "SELECT id  FROM users WHERE public_key = $1",
            request.pubkey,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserExistError::DatabaseError(e.to_string()))?;

        let user = match existing_user {
            Some(user) => user,
            None => return Err(UserExistError::UserNotExists),
        };

        Ok(ExistUser {
            id: user.id.to_string(),
        })
    }

    pub async fn update_balance(&self, request: UpdateBalanceReq) -> Result<(), UserExistError> {
        let user_id: Uuid = Uuid::parse_str(&request.user_id)
            .map_err(|e| UserExistError::DatabaseError(e.to_string()))?;

        let sol_str = format!("{:.9}", request.new_bal); // keep 9 decimal places
        let amount = BigDecimal::from_str(&sol_str)
            .map_err(|e| UserExistError::DatabaseError(e.to_string()))?;

        sqlx::query("UPDATE balances SET amount = $1 WHERE user_id = $2")
            .bind(amount)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserExistError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_all_pub_keys(&self) -> Result<GetAllPubKeysRes, UserError> {
        let rows = sqlx::query!(
            "SELECT public_key
        FROM users"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        let public_keys: Vec<String> = rows.into_iter().map(|row| row.public_key).collect();
        Ok(GetAllPubKeysRes { public_keys })
    }

    pub async fn get_user_email(
        &self,
        request: GetUserEmailReq,
    ) -> Result<GetUserEmailRes, UserExistError> {
        let u_id = Uuid::parse_str(&request.u_id)
            .map_err(|_| UserExistError::InvalidInput("Invalid UUID".to_string()))?;

        let row = sqlx::query!("SELECT email FROM users WHERE id = $1", u_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::RowNotFound = e {
                    UserExistError::UserNotExists
                } else {
                    UserExistError::DatabaseError(e.to_string())
                }
            })?;

        let user_email = row.email;

        Ok(GetUserEmailRes { email: user_email })
    }

    pub async fn get_user_sol_bal(
        &self,
        request: GetUserEmailReq,
    ) -> Result<GetUserSolBalRes, UserExistError> {
        let u_id = Uuid::parse_str(&request.u_id)
            .map_err(|_| UserExistError::InvalidInput("Invalid UUID".to_string()))?;

        let row = sqlx::query!("SELECT amount FROM balances WHERE user_id = $1", u_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if let sqlx::Error::RowNotFound = e {
                    UserExistError::UserNotExists
                } else {
                    UserExistError::DatabaseError(e.to_string())
                }
            })?;

        let user_sol_bal = row.amount.to_string().parse::<f64>().unwrap() as f64;

        Ok(GetUserSolBalRes {
            balance: user_sol_bal,
        })
    }


}
