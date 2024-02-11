
pub async fn mount(db_client: libsql_client::Client) {
    db_client.execute("
        CREATE TABLE transacoes (
            id SERIAL,
            id_cliente INTEGER NOT NULL,
            valor INTEGER NOT NULL,
            tipo CHAR(1) NOT NULL,
            descricao VARCHAR(10) NOT NULL,
            realizada_em TIMESTAMP NOT NULL DEFAULT NOW()
        );
        CREATE INDEX idx_extrato ON transacoes (id DESC);
        CREATE TABLE saldos_limites (
            id_cliente SERIAL PRIMARY KEY,
            limite INTEGER NOT NULL,
            saldo INTEGER NOT NULL
        );
    ").await.unwrap();
}