CREATE SCHEMA IF NOT EXISTS dev;

CREATE TYPE domainLanguage AS ENUM ('en', 'se', 'de', 'fr', 'es');
CREATE TYPE tld AS ENUM ('.com', '.net', '.org');

CREATE TABLE IF NOT EXISTS dev.domains (
    id          UBIGINT PRIMARY KEY,
    name        VARCHAR CHECK (NOT contains(name, ' ')),
    available   BOOLEAN DEFAULT NULL,
    valid       BOOLEAN DEFAULT NULL,
    page_rank   DECIMAL DEFAULT 0,
    censored    BOOLEAN DEFAULT NULL
    -- whoisBirth int,
    -- ArcYrs int,
    -- length int,
    -- whoisCountry VARCHAR,
    -- whoisCity VARCHAR,
    -- whoisOrg VARCHAR,
    -- whoisNet VARCHAR,
    -- whoisPostal VARCHAR,
    -- whoisStreet VARCHAR,
    -- whoisState VARCHAR,
    -- whoisPhone VARCHAR,
    -- whoisFax VARCHAR,
    -- whoisEmail VARCHAR,
    -- censored BOOLEAN,
    -- mood mood,
    -- domainType VARCHAR,
    -- domainStatus VARCHAR,
    -- domainStatusReason VARCHAR,
    -- domainStatusUpdatedAt TIMESTAMP,
    -- lastUpdatedAt TIMESTAMP,
    -- createdAt TIMESTAMP,
    -- updatedAt TIMESTAMP,
    -- droppedAt TIMESTAMP,
    -- backlinkCount int,
    -- alexaRank int,
    -- tld VARCHAR,
    -- domainLanguage domainLanguage,
);

COMMENT ON TABLE dev.domains IS 'All domains from expired-domains.co';
COMMENT ON COLUMN dev.domains.id IS 'random uuid';
COMMENT ON COLUMN dev.domains.name IS 'domain name';
COMMENT ON COLUMN dev.domains.available IS 'was domain available at the time of the scan';
COMMENT ON COLUMN dev.domains.valid IS 'is domain still available';
COMMENT ON COLUMN dev.domains.page_rank IS 'page rank score from expired-domains.co';
COMMENT ON COLUMN dev.domains.censored IS 'did domain fail to pass the censor check (true == bad words found)';

-- COMMENT ON INDEX dev.domains IS 'unique index on domain name since each domain should only occur once';

CREATE VIEW valid_domains AS SELECT name FROM dev.domains WHERE valid = true AND page_rank > 0 AND name LIKE '%.com' AND name LIKE '%.net' AND name LIKE '%.org' AND censored = false;

-- CREATE UNIQUE INDEX domains ON dev.domains (name);

