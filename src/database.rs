use rusqlite::{params, Connection, Result, OptionalExtension};
use std::env;
use std::fs;
use chrono::Local;
use serde::{Deserialize, Serialize};

/// Represents a translation record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub id: i64,
    pub timestamp: String,
    pub source_text: String,
    pub source_lang: String,
    pub target_text: String,
    pub target_lang: String,
}

/// Represents a translation list (like a playlist)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationList {
    pub id: i64,
    pub name: String,
    pub created_at: String, 
}

/// Represents a translation entry in a list
#[allow(dead_code)]
pub struct ListEntry {
    pub id: i64, 
    pub list_id: i64,
    pub translation_id: i64,
}

/// Main database interface
/// Implementing Clone manually since we need to clone the database connection
#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection and initialize tables if they don't exist
    pub fn new() -> Result<Self> {
        // Create config directory if it doesn't exist
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_dir = format!("{}/.config/translator-app", home_dir);
        fs::create_dir_all(&db_dir).map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;
        
        // Connect to database
        let db_path = format!("{}/translations.db", db_dir);
        let conn = Connection::open(db_path)?;
        
        // Initialize database schema
        Database::init_schema(&conn)?;
        
        Ok(Database { conn })
    }
    
    /// Create a clone by opening a new connection to the same database
    pub fn clone(&self) -> Self {
        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = format!("{}/.config/translator-app/translations.db", home_dir);
        
        let conn = Connection::open(db_path).expect("Failed to clone database connection");
        Database { conn }
    }
    
    /// Initialize the database schema if not already created
    fn init_schema(conn: &Connection) -> Result<()> {
        // Create translations table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS translations (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                source_text TEXT NOT NULL,
                source_lang TEXT NOT NULL,
                target_text TEXT NOT NULL,
                target_lang TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create lists table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS lists (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create list_entries table for many-to-many relationship
        conn.execute(
            "CREATE TABLE IF NOT EXISTS list_entries (
                id INTEGER PRIMARY KEY,
                list_id INTEGER NOT NULL,
                translation_id INTEGER NOT NULL,
                FOREIGN KEY (list_id) REFERENCES lists (id) ON DELETE CASCADE,
                FOREIGN KEY (translation_id) REFERENCES translations (id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        Ok(())
    }
    
    /// Format current timestamp in ISO format
    fn current_timestamp() -> String {
        let now = Local::now();
        now.to_rfc3339()
    }
    
    /// Add a translation to history
    pub fn add_translation(&self, source_text: &str, source_lang: &str, 
                          target_text: &str, target_lang: &str) -> Result<i64> {
        let timestamp = Database::current_timestamp();
        
        self.conn.execute(
            "INSERT INTO translations (timestamp, source_text, source_lang, target_text, target_lang)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![timestamp, source_text, source_lang, target_text, target_lang],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Get all translations (for history view)
    pub fn get_translations(&self, limit: i64) -> Result<Vec<Translation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source_text, source_lang, target_text, target_lang
             FROM translations 
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;
        
        let translations = stmt.query_map(params![limit], |row| {
            Ok(Translation {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_text: row.get(4)?,
                target_lang: row.get(5)?,
            })
        })?;
        
        let mut result = Vec::new();
        for translation in translations {
            result.push(translation?);
        }
        
        Ok(result)
    }
    
    /// Create a new translation list
    pub fn create_list(&self, name: &str) -> Result<i64> {
        let timestamp = Database::current_timestamp();
        
        self.conn.execute(
            "INSERT INTO lists (name, created_at) VALUES (?1, ?2)",
            params![name, timestamp],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Get all translation lists
    pub fn get_lists(&self) -> Result<Vec<TranslationList>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at FROM lists ORDER BY name"
        )?;
        
        let lists = stmt.query_map([], |row| {
            Ok(TranslationList {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        
        let mut result = Vec::new();
        for list in lists {
            result.push(list?);
        }
        
        Ok(result)
    }
    
    /// Add a translation to a list
    pub fn add_to_list(&self, list_id: i64, translation_id: i64) -> Result<i64> {
        // Check if entry already exists to avoid duplicates
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM list_entries WHERE list_id = ?1 AND translation_id = ?2",
            params![list_id, translation_id],
            |row| row.get(0),
        )?;
        
        if count > 0 {
            // Entry already exists, return the existing id
            let entry_id: i64 = self.conn.query_row(
                "SELECT id FROM list_entries WHERE list_id = ?1 AND translation_id = ?2",
                params![list_id, translation_id],
                |row| row.get(0),
            )?;
            
            return Ok(entry_id);
        }
        
        // Insert new entry
        self.conn.execute(
            "INSERT INTO list_entries (list_id, translation_id) VALUES (?1, ?2)",
            params![list_id, translation_id],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// Get translations in a specific list
    pub fn get_list_translations(&self, list_id: i64) -> Result<Vec<Translation>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.timestamp, t.source_text, t.source_lang, t.target_text, t.target_lang
             FROM translations t
             JOIN list_entries le ON t.id = le.translation_id
             WHERE le.list_id = ?1
             ORDER BY t.timestamp DESC"
        )?;
        
        let translations = stmt.query_map(params![list_id], |row| {
            Ok(Translation {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_text: row.get(4)?,
                target_lang: row.get(5)?,
            })
        })?;
        
        let mut result = Vec::new();
        for translation in translations {
            result.push(translation?);
        }
        
        Ok(result)
    }
    
    /// Rename a list
    #[allow(dead_code)]
    pub fn rename_list(&self, list_id: i64, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE lists SET name = ?1 WHERE id = ?2",
            params![new_name, list_id],
        )?;
        
        Ok(())
    }
    
    /// Check if a list name already exists
    pub fn list_name_exists(&self, name: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM lists WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        
        Ok(count > 0)
    }
    
    /// Delete a list and its entries
    pub fn delete_list(&self, list_id: i64) -> Result<()> {
        // Delete entries first (should cascade, but just to be safe)
        self.conn.execute(
            "DELETE FROM list_entries WHERE list_id = ?1",
            params![list_id],
        )?;
        
        // Delete the list
        self.conn.execute(
            "DELETE FROM lists WHERE id = ?1",
            params![list_id],
        )?;
        
        Ok(())
    }
    
    /// Remove a translation from a list
    #[allow(dead_code)]
    pub fn remove_from_list(&self, list_id: i64, translation_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM list_entries WHERE list_id = ?1 AND translation_id = ?2",
            params![list_id, translation_id],
        )?;
        
        Ok(())
    }
    
    /// Delete a translation from history and all lists
    pub fn delete_translation(&self, translation_id: i64) -> Result<()> {
        // Delete entries first
        self.conn.execute(
            "DELETE FROM list_entries WHERE translation_id = ?1",
            params![translation_id],
        )?;
        
        // Delete the translation
        self.conn.execute(
            "DELETE FROM translations WHERE id = ?1",
            params![translation_id],
        )?;
        
        Ok(())
    }
    
    /// Search translations by text
    pub fn search_translations(&self, query: &str) -> Result<Vec<Translation>> {
        let search_query = format!("%{}%", query);
        
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source_text, source_lang, target_text, target_lang
             FROM translations 
             WHERE source_text LIKE ?1 OR target_text LIKE ?1
             ORDER BY timestamp DESC"
        )?;
        
        let translations = stmt.query_map(params![search_query], |row| {
            Ok(Translation {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_text: row.get(4)?,
                target_lang: row.get(5)?,
            })
        })?;
        
        let mut result = Vec::new();
        for translation in translations {
            result.push(translation?);
        }
        
        Ok(result)
    }
    
    /// Export a list to Anki-compatible format
    pub fn export_list_for_anki(&self, list_id: i64) -> Result<String> {
        let translations = self.get_list_translations(list_id)?;
        
        // Format: source text;target text;source language;target language
        let mut csv = String::from("source;target;source_lang;target_lang\n");
        
        for translation in translations {
            // Escape any semicolons in the text and handle newlines
            let source_text = translation.source_text.replace(";", ",").replace("\n", " ");
            let target_text = translation.target_text.replace(";", ",").replace("\n", " ");
            
            csv.push_str(&format!(
                "{};{};{};{}\n",
                source_text, target_text, translation.source_lang, translation.target_lang
            ));
        }
        
        Ok(csv)
    }
    
    /// Get a translation by ID
    #[allow(dead_code)]
    pub fn get_translation_by_id(&self, translation_id: i64) -> Result<Option<Translation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source_text, source_lang, target_text, target_lang
             FROM translations 
             WHERE id = ?1"
        )?;
        
        let translation = stmt.query_row(params![translation_id], |row| {
            Ok(Translation {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source_text: row.get(2)?,
                source_lang: row.get(3)?,
                target_text: row.get(4)?,
                target_lang: row.get(5)?,
            })
        }).optional()?;
        
        Ok(translation)
    }
}