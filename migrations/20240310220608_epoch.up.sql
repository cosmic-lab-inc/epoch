-- Add migration script here

CREATE TABLE accounts(
     hash BIGINT PRIMARY KEY ,
     key BYTEA,
     slot BIGINT NOT NULL,
     lamports BIGINT NOT NULL,
     owner BYTEA,
     executable BOOLEAN NOT NULL,
     rent_epoch BIGINT NOT NULL,
     discriminant BYTEA,
     data BYTEA
);