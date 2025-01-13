use duckdb::{params, Connection, Result};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Domain {
    id: Option<u64>,
    pub name: String,
    pub available: bool,
    valid: Option<bool>,
    pub page_rank: Option<f64>,
    censored: Option<bool>,
}

impl Domain {
    pub fn new(name: &String, available: bool, page_rank: Option<f64>) -> Self {
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
#[cfg(debug_assertions)]
pub fn insert_domain(conn: &Connection, domain: &Domain) -> Result<()> {
    let mut stmt = conn.prepare("INSERT OR REPLACE id, name, available, valid, page_rank, censored INTO dev.domains VALUES (?, ?, ?, ?, ?, ?)")?;
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

// #[cfg(not(debug_assertions))]
// pub insert_domain(conn: &Connection, domain: &Domain) -> Result<()> {
//     let mut stmt = conn.prepare("INSERT OR REPLACE id, name, available, valid, page_rank, censored INTO prod.domains VALUES (?, ?, ?, ?, ?, ?)")?;
//     stmt.execute(params![
//         domain.id,
//         domain.name,
//         domain.available,
//         domain.valid,
//         domain.page_rank,
//         domain.censored,
//     ])?;
//     Ok(())
// }

pub fn update_domains(conn: &Connection, domain: &Domain) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE id, name, available, valid, page_rank, censored INTO dev.domains VALUES (?, ?, ?, ?, ?, ?)")?;
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

pub fn list_valid_domains(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM dev.domains WHERE valid = true AND page_rank > 0 AND name LIKE '%.com' AND name LIKE '%.net' AND name LIKE '%.org' AND censored = false")?;
    let mut rows = stmt.query([])?;

    let mut domains = Vec::new();
    while let Some(row) = rows.next()? {
        domains.push(row.get(0)?);
    }
    
    Ok(domains)
}

pub fn db_init() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
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
    Ok(conn)
}

// db_import()

// db_export()
