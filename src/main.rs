use std::env;

use axum::{routing::{get, post}, Router};
use deadpool::Runtime;
use tokio_postgres::NoTls;

mod handlers;

#[tokio::main]
async fn main() {

    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some("/var/run/postgresql".into());
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
        .with_state::<()>(pg_pool);

    let hostname = env::var("HOSTNAME").unwrap();

    let sockets_dir = "/tmp/sockets";
    let socket_path = format!("{sockets_dir}/{hostname}.sock");
    match tokio::fs::remove_file(&socket_path).await {
        Err(e) => println!("warn: unable to unlink path {socket_path}: {e}"),
        _ => ()
    };

    let listener = std::os::unix::net::UnixListener::bind(&socket_path)
        .expect(format!("error listening to socket {socket_path}").as_str());
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::UnixListener::from_std(listener)
        .expect("error parsing std listener");

    axum::serve(listener, app.into_make_service()).await
        .expect("error serving app");
}