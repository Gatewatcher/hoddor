use crate::measure::now;
use crate::vault::{IdentitySalts, VaultMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsValue;

use std::cell::RefCell;
use std::rc::Rc;

use crate::console;
use crate::webrtc::{AccessLevel, WebRtcPeer};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultOperation {
    pub namespace: String,
    pub operation_type: OperationType,
    pub data: Option<Vec<u8>>,
    pub nonce: Option<[u8; 12]>,
    pub timestamp: u64,
    pub author: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OperationType {
    Insert,
    Delete,
    Update,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncMessage {
    pub operation: VaultOperation,
    pub vector_clock: HashMap<String, u64>,
    pub vault_name: String,
    pub vault_metadata: Option<VaultMetadata>,
    pub identity_salts: Option<IdentitySalts>,
}

pub struct SyncManager {
    pub peer_id: String,
    pub vector_clock: HashMap<String, u64>,
    pub peers: HashMap<String, Rc<RefCell<WebRtcPeer>>>,
    pub pending_operations: Vec<VaultOperation>,
}

impl SyncManager {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id: peer_id.clone(),
            vector_clock: HashMap::from([(peer_id, 0)]),
            peers: HashMap::new(),
            pending_operations: Vec::new(),
        }
    }

    pub fn add_peer(&mut self, peer: Rc<RefCell<WebRtcPeer>>) {
        let peer_id = if let Some(remote_id) = peer.borrow().remote_peer_id() {
            remote_id
        } else {
            console::error("No remote peer ID found, skipping peer addition");
            return;
        };
        console::log(&format!("Adding peer {} to sync manager", peer_id));
        self.peers.insert(peer_id.clone(), peer);
        console::log(&format!(
            "Current peers in sync manager: {:?}",
            self.peers.keys().collect::<Vec<_>>()
        ));
    }

    pub fn create_operation(
        &mut self,
        namespace: String,
        operation_type: OperationType,
        data: Option<Vec<u8>>,
        nonce: Option<[u8; 12]>,
    ) -> VaultOperation {
        VaultOperation {
            namespace,
            operation_type,
            data,
            nonce,
            timestamp: (now() / 1000.0) as u64,
            author: self.peer_id.clone(),
        }
    }

    pub fn can_apply_operation(&self, operation: &VaultOperation, peer: &WebRtcPeer) -> bool {
        match operation.operation_type {
            OperationType::Insert | OperationType::Update => {
                peer.has_permission(&operation.namespace, AccessLevel::Contributor)
            }
            OperationType::Delete => {
                peer.has_permission(&operation.namespace, AccessLevel::Administrator)
            }
        }
    }

    // Merge local + remote operations in a last‐write‐wins manner
    pub fn merge_operations(&self, mut operations: Vec<VaultOperation>) -> Vec<VaultOperation> {
        operations.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let mut merged = Vec::new();
        let mut seen_namespaces = HashSet::new();

        for op in operations.into_iter().rev() {
            if !seen_namespaces.contains(&op.namespace) {
                seen_namespaces.insert(op.namespace.clone());
                merged.push(op);
            }
        }

        merged.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        merged
    }

    pub fn create_sync_message(
        &mut self,
        vault_name: String,
        operation: VaultOperation,
        vault_metadata: Option<VaultMetadata>,
        identity_salts: Option<IdentitySalts>,
    ) -> SyncMessage {
        SyncMessage {
            operation,
            vector_clock: self.vector_clock.clone(),
            vault_name,
            vault_metadata,
            identity_salts,
        }
    }

    pub fn get_peers_mut(&mut self) -> &mut HashMap<String, Rc<RefCell<WebRtcPeer>>> {
        &mut self.peers
    }
}

// ----------------------------------------------------
// Global single-threadesd storage of all SyncManagers.
//
// We CANNOT use `lazy_static!` with a `RefCell<...>` that is not `Sync`.
// Instead, we can use `thread_local!` or `once_cell::unsync::Lazy`.
//
// For simplicity, well show `thread_local!` here:

thread_local! {
    static SYNC_MANAGERS: RefCell<HashMap<String, Rc<RefCell<SyncManager>>>>
        = RefCell::new(HashMap::new());
}

pub fn get_sync_manager(vault_name: &str) -> Result<Rc<RefCell<SyncManager>>, JsValue> {
    let result = SYNC_MANAGERS.with(|cell| {
        let mut managers = cell.borrow_mut();

        if !managers.contains_key(vault_name) {
            managers.insert(
                vault_name.to_string(),
                Rc::new(RefCell::new(SyncManager::new(vault_name.to_string()))),
            );
        }

        managers.get(vault_name).cloned()
    });

    result.ok_or_else(|| JsValue::from_str("Failed to retrieve SyncManager"))
}
