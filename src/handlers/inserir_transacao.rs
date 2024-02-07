use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

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

pub async fn handler(
    Path(id_cliente): Path<i32>,
    State(pg_pool): State<deadpool_postgres::Pool>,
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

    let conn = pg_pool.get().await
        .expect("error getting db conn");
    
    let saldo_atualizado = conn.query("CALL INSERIR_TRANSACAO($1, $2, $3, $4);", &[
        &id_cliente,
        &valor,
        &payload.tipo,
        &payload.descricao
    ]).await.expect("error running function");

    let saldo_atualizado = saldo_atualizado.get(0).unwrap();

    match saldo_atualizado.get::<_, Option<i32>>(0) {
        Some(saldo) => (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO {
            saldo,
            limite: saldo_atualizado.get(1)
        }).unwrap()),
        None => (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}