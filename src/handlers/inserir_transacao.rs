use std::sync::Arc;
use sql_builder::{quote, SqlBuilder};
use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::AppState;

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

    {
        app_state.traffic_observer.write().await.count += 1;
    }

    if app_state.traffic_observer.read().await.batch_activated {
        batch_insert(app_state.clone(), id_cliente, valor, payload).await
    }
    else {
        normal_insert(app_state.pg_pool.clone(), id_cliente, valor, payload).await
    }

}

async fn normal_insert(pg_pool: deadpool_postgres::Pool, id_cliente: i32, valor: i32, payload: TransacaoDTO) -> (StatusCode, std::string::String) {
    let conn = pg_pool.get().await
        .expect("error getting db conn");

    let saldo_atualizado = conn.query("CALL INSERIR_TRANSACAO($1, $2, $3, $4);", &[
        &id_cliente,
        &valor,
        &payload.tipo,
        &payload.descricao
    ]).await.expect("error running proc");

    let saldo_atualizado = saldo_atualizado.get(0).unwrap();

    match saldo_atualizado.get::<_, Option<i32>>(0) {
        Some(saldo) => (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO {
            saldo,
            limite: saldo_atualizado.get(1)
        }).unwrap()),
        None => (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}

async fn batch_insert(app_state: Arc<AppState>, id_cliente: i32, valor: i32, payload: TransacaoDTO) -> (StatusCode, std::string::String) {
    let conn = app_state.pg_pool.get().await
        .expect("error getting db conn");

    let saldo_atualizado = conn.query("SELECT INSERIR_TRANSACAO_FAST($1, $2);", &[
        &id_cliente,
        &valor
    ]).await.expect("error running proc");

    let saldo_atualizado = saldo_atualizado.get(0).unwrap();

    match saldo_atualizado.get::<_, Option<i32>>(0) {
        Some(saldo) => {
            app_state.queue.push((id_cliente, valor, payload.tipo, payload.descricao));
            return (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO {
                saldo,
                limite: saldo_atualizado.get(1)
            }).unwrap())
        },
        None => (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}

pub async fn flush_queue(app_state: Arc<AppState>) {
   let mut sql_builder = SqlBuilder::insert_into("transacoes");
   sql_builder
        .field("id_cliente")
        .field("valor")
        .field("tipo")
        .field("descricao");
    while app_state.queue.len() > 0 {
        let (id_cliente, valor, tipo, descricao) = app_state.queue.pop().await;
        sql_builder.values(&[
            &quote(id_cliente),
            &quote(valor),
            &quote(tipo),
            &quote(descricao),
        ]);
    }
    {
        let conn = app_state.pg_pool.get().await
            .expect("error getting db conn");
        _ = conn.batch_execute(&sql_builder.sql().unwrap()).await;
    }
}