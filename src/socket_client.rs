use anyhow::Error;
use axum::response::IntoResponse;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

const SOCKET_PATH_BASE: &'static str = "/tmp/sockets/api03.sock";

pub async fn make_socket_request(path: String) -> String {
    let url = Uri::new(SOCKET_PATH_BASE, &path).into();

    let client: Client<UnixConnector, Full<Bytes>> = Client::unix();

    let mut response = client.get(url).await
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

pub async fn consulta_saldo(id_cliente: i32) -> (i32, i32) {

    let response = make_socket_request(format!("/c/{id_cliente}")).await;
    let split = response.split(",").collect::<Vec<&str>>();
    (
        split.get(0).unwrap().parse::<i32>().unwrap(),
        split.get(1).unwrap().parse::<i32>().unwrap()
    )
}

pub async fn movimenta_saldo(id_cliente: i32, valor: i32) -> Result<(i32, i32), anyhow::Error> {

    let response = make_socket_request(format!("/c/{id_cliente}/{valor}")).await;
    let split = response.split(",").collect::<Vec<&str>>();
    
    if split.len() == 1 {
        Err(Error::msg(""))
    }
    else {
        Ok((
            split.get(0).unwrap().parse::<i32>().unwrap(),
            split.get(1).unwrap().parse::<i32>().unwrap()
        ))
    }
}