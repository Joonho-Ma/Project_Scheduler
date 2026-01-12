use std::{fs, io, path::Path};

use crate::models::Db;

pub const DB_PATH: &str = "data/db.json";

pub fn load_db() -> io::Result<Db> {
    let text = fs::read_to_string(DB_PATH)?;
    let db: Db =
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(db)
}

pub fn save_db(db: &Db) -> io::Result<()> {
    let tmp_path = format!("{DB_PATH}.tmp");
    let text = serde_json::to_string_pretty(db)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if let Some(parent) = Path::new(DB_PATH).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&tmp_path, text)?;
    fs::rename(&tmp_path, DB_PATH)?;
    Ok(())
}
