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
    Path((id_cliente, valor)): Path<(usize, i32)>,
    State(app_state): State<Arc<AppState>>
) -> impl IntoResponse {
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let mut saldo = app_state.saldos
        .get(id_cliente - 1).unwrap()
        .lock().unwrap();
    let saldo_atualizado = *saldo + valor;
    if saldo_atualizado < -limite {
        (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
    else {
        let _ = std::mem::replace(&mut *saldo, saldo_atualizado);
        let id_transacao = {
            let mut id_transacao = app_state.id_transacao.lock().unwrap();
            let novo_id_transacao = *id_transacao + 1;
            let _ = std::mem::replace(&mut *id_transacao, novo_id_transacao);
            novo_id_transacao
        };
        (StatusCode::OK, format!("{saldo_atualizado},{limite},{id_transacao}"))
    }
}