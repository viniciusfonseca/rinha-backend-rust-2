use anyhow::Error;
use axum::{body::Bytes, response::IntoResponse};
use http_body_util::{BodyExt, Full};
use hyper::Request;
use serde_json::json;

use crate::HyperClient;

const SOCKET_PATH_BASE: &'static str = "/tmp/sockets/alexdb.sock";

async fn make_socket_request(client: &HyperClient, request: Request<Full<Bytes>>) -> String {

    let mut response = client.request(request).await
        .expect("error getting socket response")
        .into_response();

    let mut response_body = String::new();
    while let Some(frame_result) = response.frame().await {
        let frame = frame_result.expect("error getting frame result");

        if let Some(segment) = frame.data_ref() {
            response_body.push_str(&String::from_utf8_lossy(segment.iter().as_slice()));
        }
    }

    response_body
}

pub async fn create_atomic(client: &HyperClient, id_cliente: usize, limite: i32, log_size: usize) {

    let body = json!({
        "id": id_cliente,
        "min_value": -limite,
        "log_size": log_size
    });
    let body = serde_json::to_string(&body).unwrap();
    let body = Full::new(Bytes::from(body));

    let request = Request::builder()
        .method("POST")
        .uri(format!("{SOCKET_PATH_BASE}/atomics"))
        .body(body)
        .expect("error building request (create_atomic)");

    make_socket_request(client, request).await;
}

pub async fn movimenta_saldo(
    client: &HyperClient,
    id_cliente: usize,
    valor: i32,
    tipo: String,
    descricao: String
) -> Result<i32, anyhow::Error> {

    let body = format!("{tipo},{descricao}");
    let body = Full::new(Bytes::from(body));
    let request = Request::builder()
        .method("POST")
        .uri(format!("{SOCKET_PATH_BASE}/atomics/{id_cliente}/{valor}"))
        .body(body)
        .expect("error building request (movimenta_saldo)");

    let response = make_socket_request(client, request).await;
    if response.is_empty() {
        Err(Error::msg(""))
    }
    else {
        Ok(response.parse::<i32>().unwrap())
    }
}