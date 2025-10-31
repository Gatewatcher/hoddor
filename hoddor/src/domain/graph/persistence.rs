use crate::domain::crypto;
use crate::domain::graph::{GraphBackup, GraphError, GraphResult};
use crate::platform::Platform;
use crate::ports::graph::GraphPort;
use crate::ports::StoragePort;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

const NAMESPACE_EXTENSION: &str = "hoddor";

#[derive(Clone)]
pub struct EncryptionConfig {
    pub platform: Platform,
    pub recipient: String,
    pub identity: String,
}

pub struct GraphPersistenceService<G: GraphPort, S: StoragePort> {
    graph: G,
    storage: S,
    backup_path: String,
    encryption: EncryptionConfig,
}

impl<G: GraphPort, S: StoragePort> GraphPersistenceService<G, S> {
    pub fn new(graph: G, storage: S, backup_path: String, encryption: EncryptionConfig) -> Self {
        Self {
            graph,
            storage,
            backup_path,
            encryption,
        }
    }

    pub async fn backup(&self, vault_id: &str) -> GraphResult<()> {
        let backup = self.graph.export_backup(vault_id).await?;

        let json = serde_json::to_string(&backup).map_err(|e| {
            GraphError::SerializationError(format!("Failed to serialize backup: {}", e))
        })?;

        let encrypted = crypto::encrypt_for_recipients(
            &self.encryption.platform,
            json.as_bytes(),
            &[&self.encryption.recipient],
        )
        .await
        .map_err(|e| GraphError::Other(format!("Encryption failed: {}", e)))?;

        let data_to_save = BASE64.encode(&encrypted);

        if let Some(dir) = self.backup_path.rfind('/') {
            let dir_path = &self.backup_path[..dir];
            self.storage.create_directory(dir_path).await.map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create backup directory: {}", e))
            })?;
        }

        self.storage
            .write_file(
                &format!("{}/{}.{}", self.backup_path, vault_id, NAMESPACE_EXTENSION),
                &data_to_save,
            )
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to write backup: {}", e)))?;

        Ok(())
    }

    pub async fn restore(&self, vault_id: &str) -> GraphResult<GraphBackup> {
        let file_content = self
            .storage
            .read_file(&format!(
                "{}/{}.{}",
                self.backup_path, vault_id, NAMESPACE_EXTENSION
            ))
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to read backup: {}", e)))?;

        let encrypted = BASE64
            .decode(&file_content)
            .map_err(|e| GraphError::Other(format!("Base64 decode failed: {}", e)))?;

        let decrypted = crypto::decrypt_with_identity(
            &self.encryption.platform,
            &encrypted,
            &self.encryption.identity,
        )
        .await
        .map_err(|e| GraphError::Other(format!("Decryption failed: {}", e)))?;

        let json = String::from_utf8(decrypted).map_err(|e| {
            GraphError::SerializationError(format!("UTF-8 conversion failed: {}", e))
        })?;

        let backup: GraphBackup = serde_json::from_str(&json).map_err(|e| {
            GraphError::SerializationError(format!("Failed to deserialize backup: {}", e))
        })?;

        self.graph.import_backup(&backup).await?;

        Ok(backup)
    }

    pub async fn backup_exists(&self, vault_id: &str) -> bool {
        self.storage
            .read_file(&format!(
                "{}/{}.{}",
                self.backup_path, vault_id, NAMESPACE_EXTENSION
            ))
            .await
            .is_ok()
    }

    #[cfg(test)]
    pub async fn delete_backup(&self, vault_id: &str) -> GraphResult<()> {
        self.storage
            .delete_file(&format!(
                "{}/{}.{}",
                self.backup_path, vault_id, NAMESPACE_EXTENSION
            ))
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete backup: {}", e)))?;

        Ok(())
    }
}

// #[cfg(all(test, target_arch = "wasm32"))]
// mod tests {
//     use super::*;
//     use crate::adapters::wasm::OpfsStorage;

//     #[cfg(feature = "graph")]
//     use crate::adapters::wasm::CozoGraphAdapter;

//     use crate::domain::crypto;
//     use crate::platform::Platform;
//     use wasm_bindgen_test::*;

//     wasm_bindgen_test_configure!(run_in_browser);

//     #[test]
//     fn test_graph_backup_structure() {
//         let backup = GraphBackup {
//             version: 1,
//             nodes: vec![],
//             edges: vec![],
//             created_at: 12345,
//         };

//         let json = serde_json::to_string(&backup).unwrap();
//         let restored: GraphBackup = serde_json::from_str(&json).unwrap();

//         assert_eq!(restored.version, 1);
//         assert_eq!(restored.created_at, 12345);
//     }

//     #[wasm_bindgen_test]
//     async fn test_backup_and_restore() {
//         let platform = Platform::new();
//         let identity = crypto::generate_identity(&platform).unwrap();
//         let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

//         let encryption = EncryptionConfig {
//             platform: platform.clone(),
//             recipient: recipient.clone(),
//             identity: identity.clone(),
//         };

//         let graph = CozoGraphAdapter::new();
//         let storage = OpfsStorage::new();

//         storage.create_directory("graph_backups").await.unwrap();

//         let service =
//             GraphPersistenceService::new(graph, storage, "graph_backups".to_string(), encryption);

//         let vault_id = "test_vault_backup";

//         let node1_id = service
//             .graph
//             .create_node(
//                 vault_id,
//                 "memory",
//                 vec![1, 2, 3],
//                 vec!["test".to_string()],
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let node2_id = service
//             .graph
//             .create_node(
//                 vault_id,
//                 "entity",
//                 vec![4, 5, 6],
//                 vec!["test2".to_string()],
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let edge_props = EdgeProperties {
//             weight: 0.8,
//             bidirectional: false,
//             encrypted_context: None,
//             metadata: Default::default(),
//         };

//         service
//             .graph
//             .create_edge(vault_id, &node1_id, &node2_id, "relates_to", edge_props)
//             .await
//             .unwrap();

//         service.backup(vault_id).await.unwrap();

//         assert!(service.backup_exists(vault_id).await);

//         let encryption2 = EncryptionConfig {
//             platform: platform.clone(),
//             recipient,
//             identity,
//         };

//         let graph2 = CozoGraphAdapter::new();
//         let storage2 = OpfsStorage::new();
//         let service2 = GraphPersistenceService::new(
//             graph2,
//             storage2,
//             "graph_backups".to_string(),
//             encryption2,
//         );

//         let restored_backup = service2.restore(vault_id).await.unwrap();

//         assert_eq!(restored_backup.version, 1);
//         assert_eq!(restored_backup.nodes.len(), 2);
//         assert_eq!(restored_backup.edges.len(), 1);

//         service.delete_backup(vault_id).await.unwrap();
//         assert!(!service.backup_exists(vault_id).await);
//     }

//     #[wasm_bindgen_test]
//     async fn test_backup_nonexistent_vault() {
//         let platform = Platform::new();
//         let identity = crypto::generate_identity(&platform).unwrap();
//         let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

//         let encryption = EncryptionConfig {
//             platform: platform.clone(),
//             recipient,
//             identity,
//         };

//         let graph = CozoGraphAdapter::new();
//         let storage = OpfsStorage::new();
//         storage.create_directory("graph_backups").await.unwrap();
//         let service =
//             GraphPersistenceService::new(graph, storage, "graph_backups".to_string(), encryption);

//         let result = service.backup("nonexistent_vault").await;
//         assert!(result.is_ok());

//         let _ = service.delete_backup("nonexistent_vault").await;
//     }

//     #[wasm_bindgen_test]
//     async fn test_backup_with_multiple_edges() {
//         let platform = Platform::new();
//         let identity = crypto::generate_identity(&platform).unwrap();
//         let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

//         let encryption = EncryptionConfig {
//             platform: platform.clone(),
//             recipient,
//             identity,
//         };

//         let graph = CozoGraphAdapter::new();
//         let storage = OpfsStorage::new();
//         storage.create_directory("graph_backups").await.unwrap();
//         let service =
//             GraphPersistenceService::new(graph, storage, "graph_backups".to_string(), encryption);

//         let vault_id = "test_vault_multi_edges";

//         let node1 = service
//             .graph
//             .create_node(vault_id, "memory", vec![1], vec![], None, None)
//             .await
//             .unwrap();

//         let node2 = service
//             .graph
//             .create_node(vault_id, "memory", vec![2], vec![], None, None)
//             .await
//             .unwrap();

//         let node3 = service
//             .graph
//             .create_node(vault_id, "memory", vec![3], vec![], None, None)
//             .await
//             .unwrap();

//         let props = EdgeProperties::default();
//         service
//             .graph
//             .create_edge(vault_id, &node1, &node2, "relates_to", props.clone())
//             .await
//             .unwrap();
//         service
//             .graph
//             .create_edge(vault_id, &node2, &node3, "relates_to", props.clone())
//             .await
//             .unwrap();
//         service
//             .graph
//             .create_edge(vault_id, &node1, &node3, "relates_to", props)
//             .await
//             .unwrap();

//         service.backup(vault_id).await.unwrap();

//         let backup = service.restore(vault_id).await.unwrap();

//         assert_eq!(backup.nodes.len(), 3);
//         assert_eq!(backup.edges.len(), 3);

//         service.delete_backup(vault_id).await.unwrap();
//     }

//     #[wasm_bindgen_test]
//     async fn test_encrypted_backup_and_restore() {
//         let platform = Platform::new();
//         let identity = crypto::generate_identity(&platform).unwrap();
//         let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

//         let graph = CozoGraphAdapter::new();
//         let storage = OpfsStorage::new();
//         storage
//             .create_directory("encrypted_graph_backups")
//             .await
//             .unwrap();

//         let encryption = EncryptionConfig {
//             platform: platform.clone(),
//             recipient: recipient.clone(),
//             identity: identity.clone(),
//         };

//         let service = GraphPersistenceService::new(
//             graph,
//             storage,
//             "encrypted_graph_backups".to_string(),
//             encryption,
//         );

//         let vault_id = "test_vault_encrypted";

//         let node1_id = service
//             .graph
//             .create_node(
//                 vault_id,
//                 "memory",
//                 vec![1, 2, 3, 4, 5],
//                 vec!["encrypted".to_string(), "test".to_string()],
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let node2_id = service
//             .graph
//             .create_node(
//                 vault_id,
//                 "entity",
//                 vec![6, 7, 8, 9, 10],
//                 vec!["sensitive".to_string()],
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let edge_props = EdgeProperties {
//             weight: 0.95,
//             bidirectional: true,
//             encrypted_context: Some(vec![11, 12, 13]),
//             metadata: Default::default(),
//         };

//         service
//             .graph
//             .create_edge(vault_id, &node1_id, &node2_id, "secure_link", edge_props)
//             .await
//             .unwrap();

//         service.backup(vault_id).await.unwrap();

//         assert!(service.backup_exists(vault_id).await);

//         let encrypted_content = service
//             .storage
//             .read_file(&format!("encrypted_graph_backups/{}.hoddor", vault_id))
//             .await
//             .unwrap();

//         assert!(!encrypted_content.starts_with("{"));
//         assert!(!encrypted_content.contains("\"version\""));

//         let restored_backup = service.restore(vault_id).await.unwrap();

//         assert_eq!(restored_backup.version, 1);
//         assert_eq!(restored_backup.nodes.len(), 2);
//         assert_eq!(restored_backup.edges.len(), 1);

//         let restored_node1 = restored_backup
//             .nodes
//             .iter()
//             .find(|n| n.node_type == "memory")
//             .unwrap();
//         assert_eq!(restored_node1.content, vec![1, 2, 3, 4, 5]);

//         let restored_edge = &restored_backup.edges[0];
//         assert_eq!(restored_edge.edge_type, "secure_link");
//         assert_eq!(restored_edge.properties.weight, 0.95);
//         assert_eq!(restored_edge.properties.bidirectional, true);

//         service.delete_backup(vault_id).await.unwrap();
//         assert!(!service.backup_exists(vault_id).await);
//     }

//     // Note: This test is skipped in WASM due to age library i18n limitations
//     // The age library tries to load language files when generating error messages
//     // which are not available in WASM environments
//     #[cfg_attr(target_arch = "wasm32", ignore)]
//     #[wasm_bindgen_test]
//     async fn test_encrypted_backup_wrong_key() {
//         let platform = Platform::new();

//         let identity1 = crypto::generate_identity(&platform).unwrap();
//         let recipient1 = crypto::identity_to_public(&platform, &identity1).unwrap();

//         let identity2 = crypto::generate_identity(&platform).unwrap();

//         let graph = CozoGraphAdapter::new();
//         let storage = OpfsStorage::new();
//         storage.create_directory("wrong_key_test").await.unwrap();

//         let encryption1 = EncryptionConfig {
//             platform: platform.clone(),
//             recipient: recipient1,
//             identity: identity1,
//         };

//         let service1 =
//             GraphPersistenceService::new(graph, storage, "wrong_key_test".to_string(), encryption1);

//         let vault_id = "test_vault_wrong_key";

//         service1
//             .graph
//             .create_node(vault_id, "memory", vec![1, 2, 3], vec![], None, None)
//             .await
//             .unwrap();

//         service1.backup(vault_id).await.unwrap();

//         let graph2 = CozoGraphAdapter::new();
//         let storage2 = OpfsStorage::new();

//         let encryption2 = EncryptionConfig {
//             platform: platform.clone(),
//             recipient: "dummy_recipient".to_string(),
//             identity: identity2,
//         };

//         let service2 = GraphPersistenceService::new(
//             graph2,
//             storage2,
//             "wrong_key_test".to_string(),
//             encryption2,
//         );

//         let result = service2.restore(vault_id).await;
//         assert!(result.is_err());

//         service1.delete_backup(vault_id).await.unwrap();
//     }
// }
