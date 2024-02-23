use std::{collections::HashMap, env, sync::Arc};

use atomic_fd::AtomicFd;
use axum::{routing::{get, post}, Router};
use http_body_util::Full;
use hyperlocal::{UnixClientExt, UnixConnector};
use socket_client::create_atomic;

mod handlers;
mod socket_client;
mod atomic_fd;

struct AppState {
    socket_client: HyperClient,
    atomic_fd: HashMap<usize, AtomicFd>,
    limites: Vec<i32>
}

type QueueEvent = (i32, i32, i32, String, String, String);
pub type AppQueue = deadqueue::unlimited::Queue<QueueEvent>;
type HyperClient = hyper_util::client::legacy::Client<UnixConnector, Full<hyper::body::Bytes>>;

#[tokio::main]
async fn main() {

    let socket_client = HyperClient::unix();
    let mut atomic_fd = HashMap::new();
    let limites = vec![100000, 80000, 1000000, 10000000, 500000];
    let log_size = 128;

    for (i, limite) in limites.iter().enumerate() {
        if env::var("PRIMARY").is_ok() {
            _ = create_atomic(&socket_client, i, *limite, log_size);
        }
        atomic_fd.insert(i, AtomicFd::new(i, log_size).await);
    }

    let app_state = Arc::new(AppState {
        socket_client,
        atomic_fd,
        limites
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
