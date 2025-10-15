use std::sync::Arc;

use crate::storage::database::Database;

/// Request context
pub struct Context {
    pub id: usize,
    pub db: Arc<Database>,
}

impl Context {
    pub fn new(id: usize, db: Arc<Database>) -> Self {
        Self { db, id }
    }
}
