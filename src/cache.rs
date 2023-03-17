use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub type Cache = Arc<Mutex<HashMap<String, String>>>;

pub fn new_cache() -> Cache {
    Arc::new(Mutex::new(HashMap::new()))
}
