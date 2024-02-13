use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::AppState;

pub async fn consulta(
    Path(id_cliente): Path<usize>,
    State(app_state): State<Arc<AppState>>
) -> impl IntoResponse {
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let saldo = *app_state.saldos
        .get(id_cliente - 1).unwrap()
        .lock().unwrap();
    (StatusCode::OK, format!("{saldo},{limite}"))
}

pub async fn movimento(
    Path((id_cliente, tipo, mut valor)): Path<(usize, String, i32)>,
    State(app_state): State<Arc<AppState>>
) -> impl IntoResponse {
    if tipo.eq("d") { valor = -valor }
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let mut saldo = app_state.saldos
        .get(id_cliente).unwrap()
        .lock().unwrap();
    let saldo_atualizado = *saldo + valor;
    if saldo_atualizado < -limite {
        (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
    else {
        let _ = std::mem::replace(&mut *saldo, saldo_atualizado);
        (StatusCode::OK, format!("{saldo_atualizado},{limite}"))
    }
}