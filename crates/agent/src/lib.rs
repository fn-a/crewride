use std::sync::Arc;
use adapt::AdaptState;
use datum::session::SessionStore;

pub mod chat;
pub mod tools;

use tools::ToolContext;

pub struct AgentState {
    pub adapter: Arc<AdaptState>,
    pub toolctx: ToolContext,
    pub sessions: SessionStore,
}
