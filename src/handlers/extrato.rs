use std::{sync::Arc, time::SystemTime};

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_postgres::Row;

use crate::AppState;

#[derive(Serialize)]
struct ExtratoDTO {
    pub saldo: ExtratoSaldoDTO,
    pub ultimas_transacoes: Vec<ExtratoTransacaoDTO>
}

#[derive(Serialize)]
struct ExtratoSaldoDTO {
    pub total: i32,
    pub data_extrato: String,
    pub limite: i32,
}

#[derive(Serialize)]
struct ExtratoTransacaoDTO {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: String
}

fn parse_sys_time_as_string(system_time: SystemTime) -> String {
    DateTime::<Utc>::from(system_time).format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

impl ExtratoDTO {
    pub fn from(saldo: &Row, extrato: Vec<Row>) -> ExtratoDTO {
        ExtratoDTO {
            saldo: ExtratoSaldoDTO {
                total: saldo.get(0),
                data_extrato: parse_sys_time_as_string(saldo.get(1)),
                limite: saldo.get(2)
            },
            ultimas_transacoes: extrato.iter().map(|t| ExtratoTransacaoDTO {
                valor: t.get(0),
                tipo: t.get(1),
                descricao: t.get(2),
                realizada_em: parse_sys_time_as_string(t.get(3))
            }).collect()
        }
    }
}

pub async fn handler(
    Path(id_cliente): Path<i32>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {

    if id_cliente > 5 {
        return (StatusCode::NOT_FOUND, String::new());
    }

    let conn = app_state.pg_pool.get().await
        .expect("error getting db conn");

    let stmt_saldo = conn.prepare_cached("SELECT saldo, NOW(), limite FROM saldos_limites WHERE id_cliente = $1;").await.expect("error preparing stmt (balance)");

    let saldo_rowset = conn.query(&stmt_saldo, &[&id_cliente]).await
        .expect("error querying balance");

    let saldo = saldo_rowset.get(0)
        .expect("balance not found");

    let stmt_extrato = conn.prepare_cached("SELECT valor, tipo, descricao, realizada_em FROM transacoes WHERE id_cliente = $1 ORDER BY id DESC LIMIT 10;").await.expect("error preparing stmt (transactions)");

    let extrato = conn.query(&stmt_extrato, &[&id_cliente]).await
        .expect("error querying transactions");


    (StatusCode::OK, serde_json::to_string(&ExtratoDTO::from(saldo, extrato)).unwrap())
}