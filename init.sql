CREATE UNLOGGED TABLE transacoes (
	id SERIAL,
	id_cliente INTEGER NOT NULL,
	valor INTEGER NOT NULL,
	tipo CHAR(1) NOT NULL,
	descricao VARCHAR(10) NOT NULL,
	realizada_em TIMESTAMP NOT NULL DEFAULT NOW(),
	p INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_extrato ON transacoes (id DESC);

CREATE UNLOGGED TABLE saldos_limites (
	id_cliente SERIAL PRIMARY KEY,
	limite INTEGER NOT NULL,
	saldo INTEGER NOT NULL
);

CREATE INDEX idx_id_cliente ON saldos_limites USING HASH(id_cliente);

CREATE PROCEDURE INSERIR_TRANSACAO(
	p_id_cliente INTEGER,
	p_valor INTEGER,
	p_tipo TEXT,
	p_descricao TEXT,
	INOUT v_saldo_atualizado INTEGER DEFAULT NULL,
	INOUT v_limite INTEGER DEFAULT NULL
)
LANGUAGE plpgsql
AS $$
BEGIN
  WITH
	UPDATE_SALDO AS (
		UPDATE saldos_limites
		SET saldo = saldo + p_valor
		WHERE id_cliente = p_id_cliente AND saldo + p_valor >= - limite
		RETURNING saldo, limite
	),
	INSERTED AS (
		INSERT INTO transacoes (id_cliente, valor, tipo, descricao)
		SELECT p_id_cliente, ABS(p_valor), p_tipo, p_descricao
		FROM UPDATE_SALDO
	)
	SELECT saldo, limite
	INTO v_saldo_atualizado, v_limite
	FROM UPDATE_SALDO;
END;
$$;

CREATE PROCEDURE MOVIMENTAR_SALDO(
	p_id_cliente INTEGER,
	p_valor INTEGER,
	INOUT v_saldo_atualizado INTEGER DEFAULT NULL,
	INOUT v_limite INTEGER DEFAULT NULL
)
LANGUAGE plpgsql
AS $$
BEGIN
	UPDATE saldos_limites
	SET saldo = saldo + p_valor
	WHERE id_cliente = p_id_cliente AND saldo + p_valor >= - limite
	RETURNING saldo, limite INTO v_saldo_atualizado, v_limite;
END;
$$;

DO $$
BEGIN
  INSERT INTO saldos_limites (limite, saldo)
  VALUES
    (1000 * 100, 0),
    (800 * 100, 0),
    (10000 * 100, 0),
    (100000 * 100, 0),
	  (5000 * 100, 0);
END;
$$;
