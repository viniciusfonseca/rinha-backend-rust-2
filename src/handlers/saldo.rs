use std::sync::{atomic::Ordering, Arc};

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::AppState;

pub async fn consulta(
    Path(id_cliente): Path<usize>,
    State(app_state): State<Arc<AppState>>
) -> impl IntoResponse {
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let saldo = app_state.saldos
        .get(id_cliente - 1).unwrap()
        .load(std::sync::atomic::Ordering::Relaxed);
    (StatusCode::OK, format!("{saldo},{limite}"))
}

pub async fn movimento(
    Path((id_cliente, valor)): Path<(usize, i32)>,
    State(app_state): State<Arc<AppState>>
) -> impl IntoResponse {
    if id_cliente == 0 {
        return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let saldo_atomic = app_state.saldos.get(id_cliente - 1).unwrap();
    let saldo_atualizado = saldo_atomic.load(Ordering::Acquire) + valor;
    if saldo_atualizado < -limite {
        return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
    saldo_atomic.store(saldo_atualizado, Ordering::Relaxed);
    let id_transacao = app_state.id_transacao.fetch_add(1, Ordering::Relaxed) + 1;
    (StatusCode::OK, format!("{saldo_atualizado},{limite},{id_transacao}"))
}