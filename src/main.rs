use std::{env, sync::{atomic::{AtomicBool, AtomicI32}, Arc}, time::Duration};

use axum::{routing::{get, post}, Router};
use deadpool::Runtime;
use handlers::inserir_transacao;
use http_body_util::Full;
use hyperlocal::{UnixClientExt, UnixConnector};
use tokio_postgres::NoTls;

mod handlers;
mod socket_client;

struct AppState {
    pg_pool: deadpool_postgres::Pool,
    queue: AppQueue,
    saldos: Vec<AtomicI32>,
    limites: Vec<i32>,
    id_transacao: AtomicI32,
    req_count: AtomicI32,
    batch_activated: AtomicBool,
    socket_client: HyperClient,
    warming_up: AtomicBool
}

type QueueEvent = (i32, i32, i32, String, String);
pub type AppQueue = deadqueue::unlimited::Queue<QueueEvent>;
type HyperClient = hyper_util::client::legacy::Client<UnixConnector, Full<hyper::body::Bytes>>;

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

    let queue = AppQueue::new();

    let mut saldos = Vec::new();
    let mut limites = Vec::new();

    let is_mem_server = std::env::var("MEM_SERVER").is_ok();
    if is_mem_server {
        for _ in 1..6 {
            saldos.push(AtomicI32::new(0));
        }
        limites.push(1000 * 100);
        limites.push(800 * 100);
        limites.push(10000 * 100);
        limites.push(100000 * 100);
        limites.push(5000 * 100);
    }

    let socket_client = HyperClient::unix();

    let app_state = Arc::new(AppState {
        pg_pool,
        queue,
        saldos,
        limites,
        id_transacao: AtomicI32::new(0),
        req_count: AtomicI32::new(0),
        batch_activated: AtomicBool::new(false),
        socket_client,
        warming_up: AtomicBool::new(true)
    });

    if !is_mem_server {
        let app_state_async = app_state.clone();
        tokio::spawn(async move {
            loop {
                {
                    let batch_activated = &app_state_async.batch_activated;
                    let req_count = app_state_async.req_count.load(std::sync::atomic::Ordering::SeqCst);
                    batch_activated.store(req_count > 3000, std::sync::atomic::Ordering::SeqCst);
                }
                inserir_transacao::flush_queue(app_state_async.clone()).await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }
    else {
        let app_state_async = app_state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let mut t = Vec::new();

            let http_client = reqwest::Client::new();

            let mount_body = || {
                format!("{{\"valor\":1,\"tipo\":\"d\",\"descricao\":\"VAF\"}}")
            };

            loop {
                if !app_state_async.warming_up.load(std::sync::atomic::Ordering::SeqCst) { break; }
                for _ in 0..75 {
                    t.push(
                        http_client.post("http://172.17.0.1:9999/clientes/1/transacoes")
                            .header("User-Agent", "W")
                            .body(mount_body())
                            .send()
                    );
                }
                futures::future::join_all(&mut t).await;
                t.clear();
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

        });
    }

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(handlers::inserir_transacao::handler))
        .route("/clientes/:id/extrato", get(handlers::extrato::handler))
        .route("/c/:i", get(handlers::saldo::consulta))
        .route("/c/:i/:v", get(handlers::saldo::movimento))
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
