use std::{sync::Arc, time::SystemTime};

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{atomic_fd::AtomicLog, AppState};

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

pub fn parse_sys_time_as_string(system_time: SystemTime) -> String {
    DateTime::<Utc>::from(system_time).format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

impl ExtratoDTO {
    pub fn from(saldo: i32, limite: i32, extrato: Vec<AtomicLog>) -> ExtratoDTO {
        let mut ultimas_transacoes = Vec::new();
        for (_txid, valor, realizada_em, tipo, descricao) in extrato {
            ultimas_transacoes.push(ExtratoTransacaoDTO {
                valor,
                tipo,
                descricao,
                realizada_em
            });
        }
        ExtratoDTO {
            saldo: ExtratoSaldoDTO {
                total: saldo,
                data_extrato: parse_sys_time_as_string(SystemTime::now()),
                limite
            },
            ultimas_transacoes
        }
    }
}

pub async fn handler(
    Path(id_cliente): Path<usize>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {

    if id_cliente > 5 {
        return (StatusCode::NOT_FOUND, String::new());
    }

    let mut atomic_fd = app_state.atomic_fd.get_async(&id_cliente).await.unwrap();
    let atomic_fd = atomic_fd.get_mut();
    let saldo = atomic_fd.get_value().await;
    let limite = *app_state.limites.get(id_cliente).unwrap();
    let extrato = atomic_fd.get_logs(10).await;

    (StatusCode::OK, serde_json::to_string(&ExtratoDTO::from(saldo, limite, extrato)).unwrap())
}