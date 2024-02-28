use std::{num::ParseIntError, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use tokio::{net::UdpSocket, sync::oneshot::{self, Sender}};

pub struct AlexDBClient {
    callbacks: Arc<scc::HashMap<usize, Option<Sender<String>>>>,
    socket: Arc<UdpSocket>,
    counter: AtomicUsize
}

const CMD_CREATE: &str = "CREATE";
const CMD_MUTATE: &str = "MUTATE";
const CMD_GET: &str = "GET";

impl AlexDBClient {
    pub async fn build(alexdb_udp_port: String, client_udp_port: String) -> AlexDBClient {
        let socket = UdpSocket::bind(format!("0.0.0.0:{client_udp_port}")).await.unwrap();
        socket.connect(format!("0.0.0.0:{alexdb_udp_port}")).await.unwrap();
        let socket = Arc::new(socket);
        let socket_async = socket.clone();
        let callbacks: Arc<scc::HashMap<usize, Option<Sender<String>>>> = Arc::new(scc::HashMap::new());
        let callbacks_async = callbacks.clone();
        tokio::spawn(async move {
            loop {
                let mut buf = [0; 30];
                _ = socket_async.recv_from(&mut buf).await.unwrap();
                let callback_key = String::from_utf8(buf[0..9].to_vec()).unwrap().trim().parse::<usize>().unwrap();
                let data = String::from_utf8(buf[9..30].to_vec()).unwrap();
                let mut cb = callbacks_async.get_async(&callback_key).await.unwrap();
                let cb = cb.get_mut().take().unwrap();
                cb.send(data).unwrap();
            }
        });
        AlexDBClient {
            socket,
            callbacks,
            counter: AtomicUsize::new(0)
        }
    }

    pub async fn send_socket_data(&self, cmd: &str, data: String) -> String {
        let (tx, rx) = oneshot::channel();
        let callback_key = self.counter.fetch_add(1, Ordering::AcqRel) + 1;
        self.callbacks.insert_async(callback_key, Some(tx)).await.unwrap();
        let data = format!("{cmd: <9}{callback_key: <9}{data}");
        self.socket.send(data.as_bytes()).await.unwrap();
        let result = rx.await.unwrap();
        self.callbacks.remove_async(&callback_key).await.unwrap();
        result.trim_end_matches(char::from(0)).to_string()
    }

    pub async fn create_atomic(&self, id: usize, min_value: i32, log_size: usize) {
        _ = self.send_socket_data(CMD_CREATE, format!("{id: <8}{min_value: <10}{log_size: <10}")).await;
    }

    pub async fn mutate_atomic(&self, id: usize, value: i32, payload: String) -> Result<i32, ParseIntError> {
        self.send_socket_data(CMD_MUTATE, format!("{id: <8}{value: <10}{payload}")).await.trim().parse::<i32>()
    }

    pub async fn get_atomic(&self, id: usize) -> i32 {
        self.send_socket_data(CMD_GET, format!("{id: <8}")).await.parse::<i32>().unwrap()
    }
}