use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{CategoryId, CategoryName, HubId};

/// Canonical category record scoped to a hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub hub_id: HubId,
    pub name: CategoryName,
    pub embedding: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Data required to insert a new [`Category`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewCategory {
    pub hub_id: HubId,
    pub name: CategoryName,
    pub embedding: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
