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
    if id_cliente == 0 {
        return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
    let limite = app_state.limites.get(id_cliente - 1).unwrap();
    let saldo_atualizado = {
        let mut saldo = app_state.saldos
            .get(id_cliente - 1).unwrap()
            .lock().unwrap();
        let saldo_atualizado = *saldo + valor;
        if saldo_atualizado < -limite {
            return (StatusCode::UNPROCESSABLE_ENTITY, String::new())
        }
        *saldo = saldo_atualizado;
        saldo_atualizado
    };
    let id_transacao = {
        let mut id_transacao = app_state.id_transacao.lock().unwrap();
        *id_transacao += 1;
        *id_transacao
    };
    (StatusCode::OK, format!("{saldo_atualizado},{limite},{id_transacao}"))
}