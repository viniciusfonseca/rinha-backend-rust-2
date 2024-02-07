use std::{env, time::SystemTime};
use std::convert::From;

use axum::body::Bytes;
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Router};
use deadpool::Runtime;
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, Row};

use chrono::{DateTime, Utc};

type AppResult<T> = Result<T, Box<(dyn std::error::Error + 'static)>>;

#[tokio::main]
async fn main() -> AppResult<()> {

    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some(
        env::var("DB_HOST")
            .unwrap_or("localhost".into())
            .to_string(),
    );
    cfg.port = Some(5432);
    cfg.dbname = Some("rinhadb".to_string());
    cfg.user = Some("root".to_string());
    cfg.password = Some("1234".to_string());
    let pool_size = env::var("POOL_SIZE")
        .unwrap_or("125".to_string())
        .parse::<usize>()
        .unwrap();

        cfg.pool = deadpool_postgres::PoolConfig::new(pool_size).into();
        println!("creating postgres pool...");
        let pg_pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        println!("postgres pool succesfully created");

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(inserir_transacao))
        .route("/clientes/:id/extrato", get(extrato))
        .with_state(pg_pool);

    let http_port = env::var("HTTP_PORT").unwrap_or("80".into());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{http_port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

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

async fn inserir_transacao(
    Path(id_cliente): Path<i32>,
    State(pg_pool): State<deadpool_postgres::Pool>,
    payload: Bytes,
) -> impl IntoResponse {

    if id_cliente > 5 {
        return (StatusCode::NOT_FOUND, String::new());
    }

    let payload = match serde_json::from_slice::<TransacaoDTO>(&payload[..]) {
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

    let conn = pg_pool.get().await
        .expect("error getting db conn");
    
    let saldo_atualizado = conn.query("CALL INSERIR_TRANSACAO($1, $2, $3, $4);", &[
        &id_cliente,
        &valor,
        &payload.tipo,
        &payload.descricao
    ]).await.expect("error running function");

    let saldo_atualizado = saldo_atualizado.get(0).unwrap();
    return if let Some(saldo) = saldo_atualizado.get::<_, Option<i32>>(0) {
        (StatusCode::OK, serde_json::to_string(&TransacaoResultDTO {
            saldo,
            limite: saldo_atualizado.get(1)
        }).unwrap())
    }
    else {
        (StatusCode::UNPROCESSABLE_ENTITY, String::new())
    }
}

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

async fn extrato(
    Path(id_cliente): Path<i32>,
    State(pg_pool): State<deadpool_postgres::Pool>,
) -> impl IntoResponse {

    if id_cliente > 5 {
        return (StatusCode::NOT_FOUND, String::new());
    }

    let conn = pg_pool.get().await
        .expect("error getting db conn");

    let stmt_saldo = conn.prepare_cached("SELECT saldo, NOW(), limite FROM saldos_limites WHERE id_cliente = $1;").await
        .expect("error preparing stmt (balance)");

    let saldo_rowset = conn.query(&stmt_saldo, &[&id_cliente]).await
        .expect("error querying balance");

    let saldo = saldo_rowset.get(0)
        .expect("balance not found");

    let stmt_extrato = conn.prepare_cached("SELECT valor, tipo, descricao, realizada_em FROM transacoes WHERE id_cliente = $1 ORDER BY id DESC LIMIT 10;").await
        .expect("error preparing stmt (transactions)");

    let extrato = conn.query(&stmt_extrato, &[&id_cliente]).await
        .expect("error querying transactions");


    (StatusCode::OK, serde_json::to_string(&ExtratoDTO::from(saldo, extrato)).unwrap())
}