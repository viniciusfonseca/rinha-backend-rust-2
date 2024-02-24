use std::sync::Arc;
use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{socket_client::movimenta_saldo, AppState};

#[derive(Deserialize)]
struct TransacaoDTO {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
}

#[derive(Serialize)]
struct TransacaoResultDTO {
    pub saldo: i32,
    pub limite: i32
}

#[axum::debug_handler]
pub async fn handler(
    Path(id_cliente): Path<usize>,
    State(app_state): State<Arc<AppState>>,
    payload: Bytes,
) -> impl IntoResponse {

    if id_cliente > 5 {
        return (StatusCode::NOT_FOUND, String::new());
    }

    let payload = match serde_json::from_slice::<TransacaoDTO>(&payload) {
        Ok(p) => p,
        Err(_) => return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    };

    let descricao_len = payload.descricao.len();
    if descricao_len < 1 || descricao_len > 10 {
        return (StatusCode::UNPROCESSABLE_ENTITY, String::new());
    }

    let valor = match payload.tipo.as_str() {
        "d" => -payload.valor,
        "c" => payload.valor,
        _ => return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    };
    let limite = app_state.limites.get(id_cliente).unwrap();

    match movimenta_saldo(&app_state.socket_client, id_cliente, valor, payload.tipo, payload.descricao).await {
        Ok(saldo) =>
            (StatusCode::OK, format!("{{\"saldo\":{saldo},\"limite\":{limite}}}")),
        Err(_) => (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}