use core::slice::SlicePattern;
use std::{sync::Arc, time::SystemTime};

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_postgres::Row;

use crate::{socket_client::consulta_saldo, AppState};

#[derive(Serialize)]
pub struct ExtratoDTO {
    pub saldo: ExtratoSaldoDTO,
    pub ultimas_transacoes: Vec<ExtratoTransacaoDTO>
}

#[derive(Serialize)]
pub struct ExtratoSaldoDTO {
    pub total: i32,
    pub data_extrato: String,
    pub limite: i32,
}

#[derive(Serialize)]
pub struct ExtratoTransacaoDTO {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: String
}

fn parse_sys_time_as_string(system_time: SystemTime) -> String {
    DateTime::<Utc>::from(system_time).format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

impl ExtratoDTO {
    pub fn from(saldo: i32, limite: i32, extrato: Vec<Row>) -> ExtratoDTO {
        ExtratoDTO {
            saldo: ExtratoSaldoDTO {
                total: saldo,
                data_extrato: parse_sys_time_as_string(SystemTime::now()),
                limite
            },
            ultimas_transacoes: extrato.iter().map(|t| ExtratoTransacaoDTO {
                valor: t.get(0),
                tipo: t.get(1),
                descricao: t.get(2),
                realizada_em: parse_sys_time_as_string(t.get(3))
            }).collect()
        }
    }

    pub fn with_systemtime_now(me: &ExtratoDTO) -> ExtratoDTO {
        ExtratoDTO {
            saldo: ExtratoSaldoDTO {
                total: me.saldo.total,
                data_extrato: parse_sys_time_as_string(SystemTime::now()),
                limite: me.saldo.limite
            },
            ultimas_transacoes: me.ultimas_transacoes
        }
    }
}

impl ExtratoDTO {
    fn copy(me: ExtratoDTO) -> ExtratoDTO {
        ExtratoDTO {
            saldo: ExtratoSaldoDTO {
                total: me.saldo.total,
                data_extrato: String::from(me.saldo.data_extrato),
                limite: me.saldo.limite
            },
            ultimas_transacoes: me.ultimas_transacoes.iter().map(|t| ExtratoTransacaoDTO {
                valor: t.valor,
                tipo: String::from(t.tipo),
                descricao: String::from(t.descricao),
                realizada_em: String::from(t.realizada_em)
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
    
    let (saldo, limite, ultima_versao_extrato) = consulta_saldo(&app_state.socket_client, id_cliente).await;

    let versao_extrato = app_state.versao_extrato.load(std::sync::atomic::Ordering::Acquire);
    if versao_extrato == ultima_versao_extrato {
        match &app_state.extrato_cache {
            Some(corpo) => {
                return (StatusCode::OK, serde_json::to_string(&ExtratoDTO::with_systemtime_now(corpo)).unwrap());
            },
            None => ()
        }
    }

    let conn = app_state.pg_pool.get().await
        .expect("error getting db conn");

    let stmt_extrato = conn.prepare_cached("SELECT valor, tipo, descricao, realizada_em FROM transacoes WHERE id_cliente = $1 ORDER BY id DESC LIMIT 10;").await.expect("error preparing stmt (transactions)");

    let extrato = conn.query(&stmt_extrato, &[&id_cliente]).await
        .expect("error querying transactions");

    app_state.versao_extrato.store(ultima_versao_extrato, std::sync::atomic::Ordering::Relaxed);

    let extrato_ok = ExtratoDTO::from(saldo, limite, extrato);
    let extrato_json = ExtratoDTO::copy(extrato_ok);
    app_state.extrato_cache = Some(extrato_ok);

    (StatusCode::OK, serde_json::to_string(&extrato_json).unwrap())
}
