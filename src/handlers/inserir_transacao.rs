use std::{env, sync::{atomic::Ordering, Arc}};
use deadpool_postgres::Pool;
use hyper::HeaderMap;
use sql_builder::{quote, SqlBuilder};
use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{socket_client::{atualiza_extrato, movimenta_saldo}, AppQueue, AppState, HyperClient};

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
    Path(mut id_cliente): Path<i32>,
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
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

    if let Some(user_agent) = headers.get("user-agent") {
        if user_agent.to_str().unwrap().eq("W") {
            id_cliente = 0;
        }
        else {
            app_state.req_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    match movimenta_saldo(&app_state.socket_client, id_cliente, valor).await {
        Ok((saldo, limite, id_transacao)) =>
            if app_state.batch_activated.load(Ordering::Relaxed) {
                app_state.queue.push((id_transacao, id_cliente, valor.abs(), payload.tipo, payload.descricao));
                (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO { saldo, limite }).unwrap())
            }
            else {
                let conn = app_state.pg_pool.get().await
                    .expect("error getting db conn");
                let stmt = conn.prepare_cached("
                    INSERT INTO transacoes (id, id_cliente, valor, tipo, descricao, p)
                    VALUES ($1, $2, $3, $4, $5, 1);
                ").await.expect("error preparing stmt (inserir_transacao)");
                conn.execute(&stmt, &[&id_transacao, &id_cliente, &valor.abs(), &payload.tipo, &payload.descricao]).await
                    .expect("error running insert");

                atualiza_extrato(&app_state.socket_client).await;

                (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO { saldo, limite }).unwrap())
            }
        ,
        Err(_) => (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}

pub async fn flush_queue(queue: &AppQueue, pg_pool: &Pool, socket_client: &HyperClient) {
    let mut sql = String::new();
    if queue.len() > 0 {
        let mut sql_builder = SqlBuilder::insert_into("transacoes");
        sql_builder
            .field("id")
            .field("id_cliente")
            .field("valor")
            .field("tipo")
            .field("descricao")
            .field("p");
        while queue.len() > 0 {
            let (id_transacao, id_cliente, valor, tipo, descricao) = queue.pop().await;
            sql_builder.values(&[
                &id_transacao.to_string(),
                &id_cliente.to_string(),
                &valor.to_string(),
                &quote(tipo),
                &quote(descricao),
                &0.to_string()
            ]);
        }
        sql.push_str(&sql_builder.sql().unwrap());
    }
    if env::var("UPDATER").is_ok() {
        sql.push_str("
            WITH transacoes_processadas AS (
                UPDATE transacoes
                SET p = 1
                WHERE p = 0 
                RETURNING id_cliente, valor, tipo
            ),
            saldos_pendentes AS (
                SELECT SUM(CASE WHEN tipo = 'd' THEN -valor ELSE valor END) AS valor, id_cliente
                FROM transacoes_processadas
                GROUP BY id_cliente
            )
            UPDATE saldos_limites sl
            SET saldo = saldo + saldos_pendentes.valor
            FROM saldos_pendentes
            WHERE sl.id_cliente = saldos_pendentes.id_cliente;
        ");
    }
    if sql.is_empty() { return; }
    {
        let _ = match &pg_pool.get().await {
            Ok(conn) => 
                match conn.batch_execute(&sql).await {
                    Ok(_) => {
                        atualiza_extrato(socket_client).await;
                        Ok(())
                    },
                    Err(e) => { eprintln!("error running batch: {e}"); Err(e) },
                },
            _ => Ok(())
        };
    }
}
