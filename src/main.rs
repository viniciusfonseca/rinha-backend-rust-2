use axum::{routing::{get, post}, Router};

mod handlers;
mod schema;

#[tokio::main]
async fn main() {

    let db_client = libsql_client::Client::from_env().await
        .expect("error creating libsql client");

    schema::mount(db_client).await;

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(handlers::inserir_transacao::handler))
        .route("/clientes/:id/extrato", get(handlers::extrato::handler))
        .with_state(db_client);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:80")).await
        .expect("error while listening to port 80");
    
    axum::serve(listener, app).await
        .expect("error while serving app");

}