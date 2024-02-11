use std::{env, sync::Arc, time::Duration};

use axum::{routing::{get, post}, Router};
use deadpool::Runtime;
use handlers::inserir_transacao;
use tokio::sync::RwLock;
use tokio_postgres::NoTls;
use traffic_observer::TrafficObserver;

mod handlers;
mod traffic_observer;

struct AppState {
    pg_pool: deadpool_postgres::Pool,
    traffic_observer: RwLock<TrafficObserver>,
    queue: AppQueue
}

type QueueEvent = (i32, i32, String, String);
pub type AppQueue = deadqueue::unlimited::Queue<QueueEvent>;

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

    let traffic_observer = RwLock::new(TrafficObserver {
        count: 0,
        batch_activated: false
    });

    let queue = AppQueue::new();
    
    let app_state = Arc::new(AppState {
        pg_pool,
        traffic_observer,
        queue
    });

    let app_state_async = app_state.clone();
    tokio::spawn(async move {
        loop {
            let mut traffic_observer = app_state_async.traffic_observer.write().await;
            traffic_observer.batch_activated = traffic_observer.count > 40;
            traffic_observer.count = 0;
            inserir_transacao::flush_queue(app_state_async.clone()).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });


    let app = Router::new()
        .route("/clientes/:id/transacoes", post(handlers::inserir_transacao::handler))
        .route("/clientes/:id/extrato", get(handlers::extrato::handler))
        .with_state::<()>(app_state);

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
