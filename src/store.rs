// --------------------------------------------------
// Responsible for persistent storage of application data.
//
// This module handles:
// - Loading the database from a local JSON file
// - Saving updates back to disk safely
//
// Design choice:
// - Local-first JSON storage (no external DB)
// - Simple, hackathon-friendly, and portable
// --------------------------------------------------

use std::{fs, io, path::Path};
use crate::models::Db;

// Path to the JSON database file.
// All application state (tasks + settings) is stored here.
pub const DB_PATH: &str = "data/db.json";


// --------------------------------------------------
// Load the database from disk.
//
// Steps:
// 1. Read the JSON file as a string
// 2. Deserialize it into the Db struct
// 3. Return the in-memory Db representation
//
// Errors:
// - IO error if file is missing or unreadable
// - Deserialization error if JSON is invalid
// --------------------------------------------------
pub fn load_db() -> io::Result<Db> {
    let text = fs::read_to_string(DB_PATH)?;
    let db: Db =
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(db)
}


// --------------------------------------------------
// Save the database back to disk.
//
// Safety strategy:
// - Write to a temporary file first
// - Then atomically rename it to the real DB path
// This prevents corruption if the program crashes mid-write.
//
// Steps:
// 1. Serialize Db into pretty JSON
// 2. Ensure parent directory exists
// 3. Write to temp file
// 4. Rename temp file -> actual DB file
// --------------------------------------------------
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
