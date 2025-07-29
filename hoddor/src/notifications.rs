use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    VaultUpdate,
}

#[derive(Serialize)]
pub struct Message<T> {
    pub event: EventType,
    pub data: T,
}
