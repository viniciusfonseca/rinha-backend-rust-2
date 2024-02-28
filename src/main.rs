use std::{env, sync::Arc};

use atomic_fd::AtomicFd;
use axum::{routing::{get, post}, Router};
use alexdb_client::AlexDBClient;

mod handlers;
mod alexdb_client;
mod atomic_fd;

struct AppState {
    alexdb_client: Arc<AlexDBClient>,
    atomic_fd: scc::HashMap<usize, AtomicFd>,
    limites: Vec<i32>
}

#[tokio::main]
async fn main() {

    let atomic_fd = scc::HashMap::new();
    let limites = vec![100000, 80000, 1000000, 10000000, 500000];
    let log_size = 72;
    let is_primary = env::var("PRIMARY").is_ok();
    let alexdb_udp_port = env::var("ALEXDB_UDP_PORT").unwrap();
    let client_udp_port = env::var("UDP_PORT").unwrap();
    let alexdb_client = Arc::new(AlexDBClient::build(alexdb_udp_port, client_udp_port).await);

    for (i, limite) in limites.iter().enumerate() {
        if is_primary {
            alexdb_client.create_atomic(i + 1, -*limite, log_size).await;
        }
        atomic_fd.insert_async(i + 1, AtomicFd::new(i + 1, log_size).await).await.unwrap();
    }

    let app_state = Arc::new(AppState {
        alexdb_client,
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
