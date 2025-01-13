CREATE SCHEMA IF NOT EXISTS prod;

CREATE TABLE IF NOT EXISTS prod.domains (
    id          VARCHAR,
    name        VARCHAR,
    valid       BOOLEAN,
    page_rank   DECIMAL
);

CREATE UNIQUE INDEX IF NOT EXISTS domains ON prod.domains (name);

