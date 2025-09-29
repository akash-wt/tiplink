use actix_web::{get, web, App, HttpServer, Responder};

mod routes;
use routes::*;
use store::Store;

use crate::authorization::AuthUser;
pub mod authorization;

    
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let s = Store::new().await;
    let store = web::Data::new(s);

    HttpServer::new(move || {
        App::new()
            .service(sign_up)
            .service(sign_in)
            .service(get_user)
            .service(quote)
            .service(swap)
            .service(send_sol)
            .service(sol_balance)
            .service(token_balance)
            .service(send_sol)
            .app_data(store.clone())
    })
    .bind("127.0.0.1:8083")?
    .run()
    .await
}
