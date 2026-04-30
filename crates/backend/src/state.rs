use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use db::PgPool;
use llm::LlmClient;

#[derive(Clone)]
pub struct AppState {
    pub db:             PgPool,
    pub llm:            LlmClient,
    pub jwt_secret:     String,
    // Throttle de login por cuenta (clave: "tenant_slug:email").
    // Arc<Mutex<...>>: compartido entre handlers clonados, sin awaits dentro del lock.
    pub login_throttle: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}
