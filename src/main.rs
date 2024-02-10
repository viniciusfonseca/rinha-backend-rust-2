use axum::{routing::{get, post}, Router};
use deadpool::Runtime;
use tokio_postgres::NoTls;

mod handlers;

#[tokio::main]
async fn main() {

    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some("/var/run/postgresql".to_string());
    cfg.port = Some(5432);
    cfg.dbname = Some("rinhadb".to_string());
    cfg.user = Some("root".to_string());
    cfg.password = Some("1234".to_string());
    let pool_size = 125;

    cfg.pool = deadpool_postgres::PoolConfig::new(pool_size).into();
    let pg_pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("error creating pg pool");

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(handlers::inserir_transacao::handler))
        .route("/clientes/:id/extrato", get(handlers::extrato::handler))
        .with_state(pg_pool);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:80")).await
        .expect("error while listening to port 80");
    
    axum::serve(listener, app).await
        .expect("error while serving app");

}