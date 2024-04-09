CREATE TABLE accounts
(
    id         BIGINT  NOT NULL,
    key        TEXT    NOT NULL,
    slot       BIGINT  NOT NULL,
    lamports   BIGINT  NOT NULL,
    owner      TEXT    NOT NULL,
    executable BOOLEAN NOT NULL,
    rent_epoch BIGINT  NOT NULL,
    data       TEXT    NOT NULL
);

SELECT create_hypertable('accounts', 'slot', if_not_exists => TRUE);

ALTER TABLE accounts SET (
    timescaledb.compress,
    timescaledb.compress_orderby = 'slot ASC',
    timescaledb.compress_segmentby = 'id',
    timescaledb.compress_chunk_time_interval = '24 hours'
    );

SELECT add_compression_policy('accounts', compress_after => BIGINT '600000');