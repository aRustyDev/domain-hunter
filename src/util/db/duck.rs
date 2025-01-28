use duckdb::{
    params,
    Connection,
    Result,
    Statement,
    Transaction
};
use struct_iterable::Iterable;
use std::collections::HashMap;
use dotenv::dotenv;
use std::{
    env,
    path::Path,
    fs,
    hash::{
        DefaultHasher,
        Hash,
        Hasher
    }
};

use opentelemetry::{
    Context,
    global,
    trace::{Tracer, Span}
};


#[derive(Debug, Clone)]
pub struct Domain {
    id: Option<u64>,
    pub name: String,
    pub available: bool,
    valid: Option<bool>,
    pub page_rank: Option<f64>,
    censored: Option<bool>,
}

// This is used to build dynamic "WHERE" clauses for filtering
#[derive(Iterable, Debug)]
pub struct DomainFilter {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub available: Option<bool>,
    pub valid: Option<bool>,
    pub page_rank: Option<f64>,
    pub censored: Option<bool>,
}

pub enum DomainFilterFields {
    Id,
    Name,
    Available,
    Valid,
    PageRank,
    Censored
}

#[derive(Debug)]
pub enum DuckDbType {
    InMemory,
    Persistent,
    Existing
}

#[derive(Debug)]
pub enum DuckDbImportSource {
    Csv,
    Json,
    Parquet,
    SQLite,
    PostgreSQL,
    MySQL,
    Iceberg,
    DeltaLake,
    CloudflareR2,
    AzureBlob,
    S3
}

#[derive(Debug)]
pub enum DuckDbExportFormat {
    Csv,
    Parquet
}

#[derive(Debug)]
pub enum DomainApprovedTlds {
    Com,
    Net,
    Org,
}

impl DomainFilter {
    pub fn new(obj: Option<&HashMap<DomainFilterFields, Option<String>>>) {
        let mut filter = DomainFilter {
            id: None,
            name: None,
            available: None,
            valid: None,
            page_rank: None,
            censored: None,
        };
        match obj {
            Some(hm) => {
                for (k,v) in hm.into_iter() {
                    match (k, v) {
                        (DomainFilterFields::Id, Some(val)) => {
                            filter.id = Some(val.parse::<u64>().unwrap())
                        },
                        (DomainFilterFields::Name, Some(val)) => {
                            filter.name = Some(val.clone())
                        }
                        (DomainFilterFields::Available, Some(val)) => {
                            filter.available = match val.as_str() {
                                "true" | "1" | "True" | "T" => Some(true),
                                "false" | "0" | "False" | "F" => Some(false),
                                _ => panic!("invalid value for available"),
                            };
                        }
                        (DomainFilterFields::Valid, Some(val)) => {
                            filter.valid = match val.as_str() {
                                "true" | "1" | "True" | "T" => Some(true),
                                "false" | "0" | "False" | "F" => Some(false),
                                _ => panic!("invalid value for available"),
                            };
                        }
                        (DomainFilterFields::PageRank, Some(val)) => {
                            filter.page_rank = Some(val.parse::<f64>().unwrap())
                        }
                        (DomainFilterFields::Censored, Some(val)) => {
                            filter.censored = match val.as_str() {
                                "true" | "1" | "True" | "T" => Some(true),
                                "false" | "0" | "False" | "F" => Some(false),
                                _ => panic!("invalid value for available"),
                            };
                        }
                        _ => {} // TODO: add trace/log flagging for unknown fields
                    }
                }
                filter
            }
            None => filter
        };
    }
    // Generates a WHERE clause for the filter
    #[tracing::instrument]
    fn where_clause(&self) -> Option<String> {
        let mut filters: Vec<String> = Vec::new();
        for (k,v) in self.iter() {
            match k {
                id => filters.push("id = {v}".to_string()), 
                name => {
                    let val = v.downcast_ref::<String>().unwrap().as_str();
                    let mut names: Vec<String> = Vec::new();
                    for tld in val.split(",") {
                        names.push("name LIKE '%.{tld}'".to_string())
                    }
                },
                available => filters.push("available = {v}".to_string()),
                valid => filters.push("valid = {v}".to_string()),
                censored => filters.push("censored = {v}".to_string()),
                page_rank => filters.push("page_rank >= {v}".to_string()),
            }
        };
        match filters.len() {
            0 => None,
            _ => Some("WHERE ".to_string() + filters.join(" AND ").as_str()),
        }
    }
}

impl Domain {
    pub fn new(name: &String, available: bool, page_rank: Option<f64>) -> Self {
        if name.contains(' ') {
            panic!("Domain name cannot contain spaces");
        }
        match page_rank {
            Some(page_rank) => Domain {
                id: Some(Self::calculate_hash(&name)),
                name: name.clone(),
                available: available,
                valid: None,
                page_rank: Some(page_rank),
                censored: None,
            },
            None => Domain {
                id: Some(Self::calculate_hash(&name)),
                name: name.clone(),
                available: available,
                valid: None,
                page_rank: None,
                censored: None,
            },
        }
    }

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
}

// TODO: Can this take an iterator?
#[tracing::instrument]
#[cfg(debug_assertions)]
pub fn insert_domain(tx: &Transaction, domain: &Domain) -> Result<()> {
    let mut stmt: Statement;
    stmt = tx.prepare("INSERT OR REPLACE INTO dev.domains (id, name, available, valid, page_rank, censored) VALUES (?, ?, ?, ?, ?, ?)")?;
    stmt.execute(params![
        domain.id,
        domain.name,
        domain.available,
        domain.valid,
        domain.page_rank,
        domain.censored,
    ])?;
    Ok(())
}

#[tracing::instrument]
#[cfg(not(debug_assertions))]
pub fn insert_domain(tx: &Transaction, domain: &Domain) -> Result<()> {
    let mut stmt: Statement;
    stmt = tx.prepare("INSERT OR REPLACE INTO prod.domains (id, name, available, valid, page_rank, censored) VALUES (?, ?, ?, ?, ?, ?)")?;
    stmt.execute(params![
        domain.id,
        domain.name,
        domain.available,
        domain.valid,
        domain.page_rank,
        domain.censored,
    ])?;
    Ok(())
}

#[tracing::instrument]
pub fn update_domains(conn: &mut Connection, domain: &Domain) -> Result<()> {
    let mut stmt: Statement;
    let tx = conn.transaction()?;
    stmt = tx.prepare("UPDATE id, name, available, valid, page_rank, censored INTO dev.domains VALUES (?, ?, ?, ?, ?, ?)")?;
    stmt.execute(params![
        domain.id,
        domain.name,
        domain.available,
        domain.valid,
        domain.page_rank,
        domain.censored,
    ])?;
    tx.commit()?;
    Ok(())
}

#[tracing::instrument]
#[cfg(debug_assertions)]
pub fn list_domains(conn: &Connection, filter: DomainFilter) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM prod.domains WHERE valid = true AND page_rank > 0 AND (name LIKE '%.com' OR name LIKE '%.net' OR name LIKE '%.org') AND censored = false")?;
    let mut rows = stmt.query([])?;

    let mut domains = Vec::new();
    while let Some(row) = rows.next()? {
        domains.push(row.get(0)?);
    }
    Ok(domains)
}

#[tracing::instrument]
#[cfg(not(debug_assertions))]
pub fn list_domains(conn: &Connection, filter: DomainFilter) -> Result<Vec<String>> {
    // valid = true 
    // page_rank > 0 
    // name LIKE '%.com' AND name LIKE '%.net' AND name LIKE '%.org' 
    // censored = false
    let mut stmt = conn.prepare("SELECT name FROM dev.domains ?")?;
    let mut domains = Vec::new();
    
    match filter.where_clause() {
        Some(where_clause) => {
            let mut rows = stmt.query([where_clause])?;
            while let Some(row) = rows.next()? {
                domains.push(row.get(0)?);
            }
        },
        None => {
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                domains.push(row.get(0)?);
            }
        }
    }

    Ok(domains)
}

#[tracing::instrument]
pub fn db_init(db_type: DuckDbType) -> Result<Connection, duckdb::Error> {
    match db_type {
        DuckDbType::InMemory => {
            let mut conn = Connection::open_in_memory()?;
            let tx = conn.transaction().unwrap();
            tx.execute_batch("
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
                );
                COMMENT ON TABLE dev.domains IS 'All domains from expired-domains.co';
                COMMENT ON COLUMN dev.domains.id IS 'random uuid';
                COMMENT ON COLUMN dev.domains.name IS 'domain name';
                COMMENT ON COLUMN dev.domains.available IS 'was domain available at the time of the scan';
                COMMENT ON COLUMN dev.domains.valid IS 'is domain still available';
                COMMENT ON COLUMN dev.domains.page_rank IS 'page rank score from expired-domains.co';
                COMMENT ON COLUMN dev.domains.censored IS 'did domain fail to pass the censor check (true == bad words found)';",
            ).unwrap();
            tx.commit().unwrap();
            Ok(conn)
        },
        DuckDbType::Persistent => {
            dotenv().ok();
            // let dbpath = env::var("DUCKDB_PATH").unwrap_or("./data/domain-hunter.duckdb".to_string());
            let dbpath = "./data/domain-hunter.duckdb";
            if !Path::new(&dbpath).exists() {
                match fs::create_dir_all(Path::new(&dbpath)) {
                    Ok(_) => {},
                    Err(_) => {
                        return Err(duckdb::Error::InvalidPath((&dbpath).into()));
                    }
                }
            }
            let mut conn = Connection::open(&dbpath)?;
            let tx = conn.transaction().unwrap();
            tx.execute_batch("
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
                );
                COMMENT ON TABLE dev.domains IS 'All domains from expired-domains.co';
                COMMENT ON COLUMN dev.domains.id IS 'random uuid';
                COMMENT ON COLUMN dev.domains.name IS 'domain name';
                COMMENT ON COLUMN dev.domains.available IS 'was domain available at the time of the scan';
                COMMENT ON COLUMN dev.domains.valid IS 'is domain still available';
                COMMENT ON COLUMN dev.domains.page_rank IS 'page rank score from expired-domains.co';
                COMMENT ON COLUMN dev.domains.censored IS 'did domain fail to pass the censor check (true == bad words found)';",
            ).unwrap();
            tx.commit().unwrap();
            Ok(conn)
        },
        DuckDbType::Existing => {
            dotenv().ok();
            let dbpath = env::var("DUCKDB_PATH").unwrap_or("./data/duck.db".to_string());
            let conn = Connection::open(&dbpath)?;
            Ok(conn)
        }
    }
    // conn.execute("PRAGMA journal_mode = WAL")?;
    // conn.execute("PRAGMA synchronous = NORMAL")?;
    // conn.execute("PRAGMA temp_store = MEMORY")?;
    // conn.execute("PRAGMA threads = 1")?;
    // conn.execute("PRAGMA cache_size = 10000")?;
    // conn.execute("PRAGMA locking_mode = EXCLUSIVE")?;
    // conn.execute("PRAGMA default_temporary_allocation = 10000")?;
    // conn.execute("PRAGMA default_cache_size = 10000")?;
    // conn.execute("PRAGMA wal_autocheckpoint = 1000")?;
    // conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")?;
}

#[tracing::instrument]
pub fn db_import(conn: &mut Connection, source: Option<DuckDbImportSource>) -> Result<()> {
    dotenv().ok();
    let src_directory = env::var("DUCKDB_EXPORT_TARGET_DIRECTORY").unwrap_or("./duckdb".to_string());
    let mut stmt: Statement;
    let tx = conn.transaction()?;

    match source {
        Some(DuckDbImportSource::Csv) => {
            todo!()
        },
        Some(DuckDbImportSource::Json) => {
            todo!()
        },
        Some(DuckDbImportSource::Parquet) => {
            todo!()
        },
        Some(DuckDbImportSource::SQLite) => {
            todo!()
        },
        Some(DuckDbImportSource::PostgreSQL) => {
            todo!()
        },
        Some(DuckDbImportSource::MySQL) => {
            tx.execute_batch("BEGIN;
                        INSTALL mysql;
                        LOAD mysql;",
            )?;
            Ok(())
        },
        Some(DuckDbImportSource::Iceberg) => {
            tx.execute_batch("BEGIN;
                        INSTALL iceberg;
                        LOAD iceberg;
                        UPDATE EXTENSIONS (iceberg);",
            )?;
            Ok(())
        },
        Some(DuckDbImportSource::DeltaLake) => {
            tx.execute_batch("BEGIN;
                        INSTALL delta;
                        LOAD delta;",
            )?;
            Ok(())
        },
        Some(DuckDbImportSource::CloudflareR2) => {
            todo!()
        },
        Some(DuckDbImportSource::AzureBlob) => {
            todo!()
        },
        Some(DuckDbImportSource::S3) => {
            todo!()
        },
        _ => {
            stmt = tx.prepare(r"IMPORT DATABASE '?';")?;
            match stmt.execute([src_directory]) {
                Ok(_) => {
                    tx.commit()?;
                    Ok(())
                },
                Err(e) => {
                    tx.rollback()?;
                    Err(e)
                },
            }
        }
    }
}

#[tracing::instrument]
pub fn db_export(conn: &mut Connection, format: DuckDbExportFormat) -> Result<()> {
    dotenv().ok();
    let target_directory = env::var("DUCKDB_EXPORT_TARGET_DIRECTORY").unwrap_or("./duckdb".to_string());
    let mut stmt: Statement;
    let tx = conn.transaction()?;

    match format {
        DuckDbExportFormat::Parquet => {
            stmt = tx.prepare(
              r"EXPORT DATABASE '?' (
                    FORMAT PARQUET,
                    COMPRESSION ZSTD,
                    ROW_GROUP_SIZE 100_000
                );
              ")?;
        },
        DuckDbExportFormat::Csv => {
            stmt = tx.prepare(
              r"EXPORT DATABASE '?' (
                    FORMAT CSV, 
                    DELIMITER '|'
                );")?;
        },
    }
    
    match stmt.execute([target_directory]) {
        Ok(_) => {
            tx.commit()?;
            Ok(())
        },
        Err(e) => {
            tx.rollback()?;
            Err(e)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_domain() {

        let provider = init_tracer_provider().expect("Failed to initialize tracer provider.");
        global::set_tracer_provider(provider.clone());

        // let tracer = global::tracer("domain-hunter-test");
        // let mut span = tracer.start("test_insert_domain");
        // let ctx = Context::current_with_span(parent);

        let mut conn = db_init(DuckDbType::InMemory).unwrap();
        let tx = conn.transaction().unwrap();

        // Insert a domain
        let insert = insert_domain(&tx, &Domain::new(&"test.com".to_string(), true, None));
        assert!(insert.is_ok());

        // Check if the domain was inserted
        let mut stmt = tx.prepare("SELECT name FROM dev.domains WHERE name = ?").unwrap();
        let mut rows = stmt.query(["test.com"]).unwrap();
        
        let mut names: Vec<String> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            names.push(row.get(0).unwrap());
        }
        assert_eq!(names.len(), 1);
        
        // Rollback the transaction
        tx.rollback().unwrap();
    }

    // Try to insert a duplicate domain
    #[test]
    fn test_insert_duplicate_domain() {

        let provider = init_tracer_provider().expect("Failed to initialize tracer provider.");
        global::set_tracer_provider(provider.clone());

        // let tracer = global::tracer("domain-hunter-test");
        // let mut span = tracer.start("test_insert_duplicate_domain");
        // let ctx = Context::current_with_span(parent);

        // Start a transaction
        let mut conn = db_init(DuckDbType::InMemory).unwrap();
        let tx = conn.transaction().unwrap();

        // Insert a domain
        let insert = insert_domain(&tx, &Domain::new(&"test.com".to_string(), true, None));
        assert!(insert.is_ok());
        
        // Try to insert the same domain again
        let insert = insert_domain(&tx, &Domain::new(&"test.com".to_string(), true, None));
        assert!(insert.is_ok());

        // Check if the domain was inserted more than once
        let mut stmt = tx.prepare("SELECT name FROM dev.domains WHERE name = ?").unwrap();
        let mut rows = stmt.query(["test.com"]).unwrap();
        
        let mut names: Vec<String> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            names.push(row.get(0).unwrap());
        }
        assert_eq!(names.len(), 1);
        
        // Rollback the transaction
        tx.rollback().unwrap();
    }

    // Try to insert a domain with a bad name
    #[test]
    #[should_panic]
    fn test_insert_bad_domain() {

        let provider = init_tracer_provider().expect("Failed to initialize tracer provider.");
        global::set_tracer_provider(provider.clone());

        // let tracer = global::tracer("domain-hunter-test");
        // let mut span = tracer.start("test_insert_bad_domain");
        // let ctx = Context::current_with_span(parent);

        // Start a transaction
        let mut conn = db_init(DuckDbType::InMemory).unwrap();
        let tx = conn.transaction().unwrap();

        // Insert a domain
        let insert = insert_domain(&tx, &Domain::new(&"test com".to_string(), true, None));
        
        // Rollback the transaction
        tx.rollback().unwrap();
    }

    // TODO: Try to insert a domain with a bad page rank
    // TODO: Try to insert a domain with a bad censored value
    // TODO: Try to insert a domain with a bad available value
    // TODO: Try to Load a CSV file
    // TODO: Try to Load a CSV file that doesn't exist
    // TODO: Try to export a CSV file
    // TODO: Try to export a Parquet file
    // TODO: Verify that the rollbacks work
    // TODO: Verify DuckDbType::Persistent creates a new DB

}