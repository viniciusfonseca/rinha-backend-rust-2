use axum::{routing::{get, post}, Router};
use deadpool::Runtime;
use tokio_postgres::NoTls;

mod handlers;

type AppResult<T> = Result<T, Box<(dyn std::error::Error + 'static)>>;

#[tokio::main]
async fn main() -> AppResult<()> {

    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some(
        std::env::var("DB_HOST")
            .unwrap_or("localhost".into())
            .to_string(),
    );
    cfg.port = Some(5432);
    cfg.dbname = Some("rinhadb".to_string());
    cfg.user = Some("root".to_string());
    cfg.password = Some("1234".to_string());
    let pool_size = std::env::var("POOL_SIZE")
        .unwrap_or("125".to_string())
        .parse::<usize>()
        .unwrap();

    cfg.pool = deadpool_postgres::PoolConfig::new(pool_size).into();
    println!("creating postgres pool...");
    let pg_pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    println!("postgres pool succesfully created");

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(handlers::inserir_transacao::handler))
        .route("/clientes/:id/extrato", get(handlers::extrato::handler))
        .with_state(pg_pool);

    let http_port = std::env::var("HTTP_PORT").unwrap_or("80".into());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{http_port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}