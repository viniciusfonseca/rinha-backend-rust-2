use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use libsql_client::args;
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
    State(db_client): State<libsql_client::Client>,
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

    let stmt = libsql_client::Statement::with_args("
        UPDATE saldos_limites
        SET saldo = saldo + ?
        WHERE id_cliente = ? AND saldo + ? >= - limite
        RETURNING saldo, limite
    ", &[id_cliente, valor]);
    
    let saldo_atualizado = db_client.execute(stmt).await
        .expect("error running saldo update");

    if saldo_atualizado.rows_affected == 0 {
        return (StatusCode::UNPROCESSABLE_ENTITY, String::new());
    }

    tokio::spawn(async move {
        let stmt = libsql_client::Statement::with_args("
            INSERT INTO transacoes (id_cliente, valor, tipo, descricao)
            VALUES (?, ?, ?, ?)
        ", args!(id_cliente, valor, payload.tipo, payload.descricao));
        let _ = db_client.execute(stmt).await;
    });
    let saldo_atualizado = saldo_atualizado.rows.get(0).unwrap();

    (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO {
        saldo: saldo_atualizado.try_get(0).unwrap(),
        limite: saldo_atualizado.try_get(1).unwrap()
    }).unwrap())
}