use std::sync::Arc;

use tokio::sync::RwLock;

use crate::storage::database::Database;

/// Request context
pub struct Context {
    pub id: usize,
    pub db: Arc<RwLock<Database>>,
}

impl Context {
    pub fn new(id: usize, db: Arc<RwLock<Database>>) -> Self {
        Self { db, id }
    }
}
