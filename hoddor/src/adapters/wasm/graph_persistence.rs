use crate::domain::crypto;
use crate::domain::graph::{GraphEdge, GraphError, GraphNode, GraphResult};
use crate::platform::Platform;
use crate::ports::graph::GraphPort;
use crate::ports::StoragePort;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};

/// Structure pour le backup complet du graph
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphBackup {
    pub version: u32,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub created_at: u64,
}

/// Configuration de chiffrement pour les backups
pub struct EncryptionConfig {
    pub platform: Platform,
    pub recipient: String,  // Clé publique pour chiffrement
    pub identity: String,   // Clé privée pour déchiffrement
}

/// Gestionnaire de persistance pour le graph
pub struct GraphPersistence<G: GraphPort, S: StoragePort> {
    graph: G,
    storage: S,
    backup_path: String,
    encryption: Option<EncryptionConfig>,
}

impl<G: GraphPort, S: StoragePort> GraphPersistence<G, S> {
    /// Crée un nouveau gestionnaire de persistance sans chiffrement
    pub fn new(graph: G, storage: S, backup_path: String) -> Self {
        Self {
            graph,
            storage,
            backup_path,
            encryption: None,
        }
    }

    /// Crée un nouveau gestionnaire de persistance avec chiffrement Age
    pub fn new_with_encryption(
        graph: G,
        storage: S,
        backup_path: String,
        encryption: EncryptionConfig,
    ) -> Self {
        Self {
            graph,
            storage,
            backup_path,
            encryption: Some(encryption),
        }
    }

    /// Active le chiffrement pour les backups
    pub fn enable_encryption(&mut self, encryption: EncryptionConfig) {
        self.encryption = Some(encryption);
    }

    /// Désactive le chiffrement pour les backups
    pub fn disable_encryption(&mut self) {
        self.encryption = None;
    }

    /// Exporte tous les nodes d'un vault
    async fn export_nodes(&self, vault_id: &str) -> GraphResult<Vec<GraphNode>> {
        // Pour l'instant, on va devoir itérer sur tous les types de nodes possibles
        // TODO: Ajouter une méthode list_all_nodes() au GraphPort
        let mut all_nodes = Vec::new();

        // Types de nodes à exporter (basé sur NodeType dans types.rs)
        let node_types = vec![
            "memory",
            "entity",
            "event",
            "concept",
            "conversation",
            "document",
            "preference",
        ];

        for node_type in node_types {
            match self.graph.list_nodes_by_type(vault_id, node_type, None).await {
                Ok(nodes) => all_nodes.extend(nodes),
                Err(_) => continue, // Si le type n'existe pas, on continue
            }
        }

        Ok(all_nodes)
    }

    /// Exporte tous les edges d'un vault
    /// Note: Cette fonction nécessite d'itérer sur tous les nodes pour récupérer leurs edges
    async fn export_edges(&self, vault_id: &str, nodes: &[GraphNode]) -> GraphResult<Vec<GraphEdge>> {
        let mut all_edges = Vec::new();
        let mut seen_edge_ids = std::collections::HashSet::new();

        for node in nodes {
            // Récupérer les edges dans les deux directions
            match self
                .graph
                .get_edges(
                    vault_id,
                    &node.id,
                    crate::domain::graph::EdgeDirection::Both,
                )
                .await
            {
                Ok(edges) => {
                    for edge in edges {
                        // Éviter les doublons (un edge bidirectionnel sera vu 2 fois)
                        if seen_edge_ids.insert(edge.id.clone()) {
                            all_edges.push(edge);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(all_edges)
    }

    /// Sauvegarde le graph complet d'un vault dans OPFS
    pub async fn backup(&self, vault_id: &str) -> GraphResult<()> {
        // 1. Exporter tous les nodes
        let nodes = self.export_nodes(vault_id).await?;

        // 2. Exporter tous les edges
        let edges = self.export_edges(vault_id, &nodes).await?;

        // 3. Créer la structure de backup
        let backup = GraphBackup {
            version: 1,
            nodes,
            edges,
            created_at: Self::get_timestamp(),
        };

        // 4. Sérialiser en JSON
        let json = serde_json::to_string(&backup)
            .map_err(|e| GraphError::SerializationError(format!("Failed to serialize backup: {}", e)))?;

        // 5. Chiffrer si nécessaire
        let data_to_save = if let Some(ref enc_config) = self.encryption {
            // Chiffrer avec Age
            let encrypted = crypto::encrypt_for_recipients(
                &enc_config.platform,
                json.as_bytes(),
                &[&enc_config.recipient],
            )
            .await
            .map_err(|e| GraphError::Other(format!("Encryption failed: {}", e)))?;

            // Encoder en base64 pour stockage texte
            BASE64.encode(&encrypted)
        } else {
            // Pas de chiffrement
            json
        };

        // 6. Créer le répertoire si nécessaire
        if let Some(dir) = self.backup_path.rfind('/') {
            let dir_path = &self.backup_path[..dir];
            self.storage
                .create_directory(dir_path)
                .await
                .map_err(|e| GraphError::DatabaseError(format!("Failed to create backup directory: {}", e)))?;
        }

        // 7. Sauvegarder dans OPFS
        let file_extension = if self.encryption.is_some() { "age" } else { "json" };
        self.storage
            .write_file(&format!("{}/{}.{}", self.backup_path, vault_id, file_extension), &data_to_save)
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to write backup: {}", e)))?;

        Ok(())
    }

    /// Restaure le graph depuis OPFS
    pub async fn restore(&self, vault_id: &str) -> GraphResult<GraphBackup> {
        // 1. Déterminer l'extension du fichier selon le mode de chiffrement
        let file_extension = if self.encryption.is_some() { "age" } else { "json" };

        // 2. Lire le fichier depuis OPFS
        let file_content = self
            .storage
            .read_file(&format!("{}/{}.{}", self.backup_path, vault_id, file_extension))
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to read backup: {}", e)))?;

        // 3. Déchiffrer si nécessaire
        let json = if let Some(ref enc_config) = self.encryption {
            // Décoder base64
            let encrypted = BASE64.decode(&file_content)
                .map_err(|e| GraphError::Other(format!("Base64 decode failed: {}", e)))?;

            // Déchiffrer avec Age
            let decrypted = crypto::decrypt_with_identity(
                &enc_config.platform,
                &encrypted,
                &enc_config.identity,
            )
            .await
            .map_err(|e| GraphError::Other(format!("Decryption failed: {}", e)))?;

            // Convertir en String
            String::from_utf8(decrypted)
                .map_err(|e| GraphError::SerializationError(format!("UTF-8 conversion failed: {}", e)))?
        } else {
            // Pas de chiffrement
            file_content
        };

        // 4. Désérialiser
        let backup: GraphBackup = serde_json::from_str(&json)
            .map_err(|e| GraphError::SerializationError(format!("Failed to deserialize backup: {}", e)))?;

        // 3. Restaurer les nodes
        for node in &backup.nodes {
            // TODO: Il faudra une méthode pour créer un node avec un ID spécifique
            // Pour l'instant, on va ignorer les erreurs de doublons
            let _ = self
                .graph
                .create_node(
                    &node.vault_id,
                    &node.node_type,
                    node.encrypted_content.clone(),
                    node.content_hmac.clone(),
                    node.labels.clone(),
                    node.embedding.clone(),
                    node.namespace.clone(),
                )
                .await;
        }

        // 4. Restaurer les edges
        for edge in &backup.edges {
            let _ = self
                .graph
                .create_edge(
                    &edge.vault_id,
                    &edge.from_node,
                    &edge.to_node,
                    &edge.edge_type,
                    edge.properties.clone(),
                )
                .await;
        }

        Ok(backup)
    }

    /// Vérifie si un backup existe pour un vault
    pub async fn backup_exists(&self, vault_id: &str) -> bool {
        let file_extension = if self.encryption.is_some() { "age" } else { "json" };
        self.storage
            .read_file(&format!("{}/{}.{}", self.backup_path, vault_id, file_extension))
            .await
            .is_ok()
    }

    /// Supprime le backup d'un vault
    pub async fn delete_backup(&self, vault_id: &str) -> GraphResult<()> {
        let file_extension = if self.encryption.is_some() { "age" } else { "json" };
        self.storage
            .delete_file(&format!("{}/{}.{}", self.backup_path, vault_id, file_extension))
            .await
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete backup: {}", e)))?;

        Ok(())
    }

    fn get_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::wasm::{CozoGraphAdapter, OpfsStorage};
    use crate::domain::crypto;
    use crate::domain::graph::{EdgeProperties, NodeId};
    use crate::platform::Platform;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[test]
    fn test_graph_backup_structure() {
        let backup = GraphBackup {
            version: 1,
            nodes: vec![],
            edges: vec![],
            created_at: 12345,
        };

        let json = serde_json::to_string(&backup).unwrap();
        let restored: GraphBackup = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.version, 1);
        assert_eq!(restored.created_at, 12345);
    }

    #[wasm_bindgen_test]
    async fn test_backup_and_restore() {
        // Créer le graph adapter
        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();
        let persistence = GraphPersistence::new(graph, storage, "graph_backups".to_string());

        let vault_id = "test_vault_backup";

        // Créer quelques nodes
        let node1_id = persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![1, 2, 3],
                "hmac1".to_string(),
                vec!["test".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        let node2_id = persistence
            .graph
            .create_node(
                vault_id,
                "entity",
                vec![4, 5, 6],
                "hmac2".to_string(),
                vec!["test2".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        // Créer un edge
        let edge_props = EdgeProperties {
            weight: 0.8,
            bidirectional: false,
            encrypted_context: None,
            metadata: Default::default(),
        };

        persistence
            .graph
            .create_edge(vault_id, &node1_id, &node2_id, "relates_to", edge_props)
            .await
            .unwrap();

        // Backup
        persistence.backup(vault_id).await.unwrap();

        // Vérifier que le backup existe
        assert!(persistence.backup_exists(vault_id).await);

        // Restore (dans un nouveau graph)
        let graph2 = CozoGraphAdapter::new_in_memory().unwrap();
        let storage2 = OpfsStorage::new();
        let persistence2 = GraphPersistence::new(graph2, storage2, "graph_backups".to_string());

        let restored_backup = persistence2.restore(vault_id).await.unwrap();

        // Vérifier les données restaurées
        assert_eq!(restored_backup.version, 1);
        assert_eq!(restored_backup.nodes.len(), 2);
        assert_eq!(restored_backup.edges.len(), 1);

        // Nettoyer
        persistence.delete_backup(vault_id).await.unwrap();
        assert!(!persistence.backup_exists(vault_id).await);
    }

    #[wasm_bindgen_test]
    async fn test_backup_nonexistent_vault() {
        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();
        let persistence = GraphPersistence::new(graph, storage, "graph_backups".to_string());

        // Backup d'un vault vide devrait fonctionner
        let result = persistence.backup("nonexistent_vault").await;
        assert!(result.is_ok());

        // Nettoyer
        let _ = persistence.delete_backup("nonexistent_vault").await;
    }

    #[wasm_bindgen_test]
    async fn test_backup_with_multiple_edges() {
        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();
        let persistence = GraphPersistence::new(graph, storage, "graph_backups".to_string());

        let vault_id = "test_vault_multi_edges";

        // Créer 3 nodes
        let node1 = persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![1],
                "h1".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let node2 = persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![2],
                "h2".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let node3 = persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![3],
                "h3".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        // Créer plusieurs edges
        let props = EdgeProperties::default();
        persistence
            .graph
            .create_edge(vault_id, &node1, &node2, "relates_to", props.clone())
            .await
            .unwrap();
        persistence
            .graph
            .create_edge(vault_id, &node2, &node3, "relates_to", props.clone())
            .await
            .unwrap();
        persistence
            .graph
            .create_edge(vault_id, &node1, &node3, "relates_to", props)
            .await
            .unwrap();

        // Backup
        persistence.backup(vault_id).await.unwrap();

        // Restore
        let backup = persistence.restore(vault_id).await.unwrap();

        assert_eq!(backup.nodes.len(), 3);
        assert_eq!(backup.edges.len(), 3);

        // Nettoyer
        persistence.delete_backup(vault_id).await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_encrypted_backup_and_restore() {
        // Générer une identité Age pour le test
        let platform = Platform::new();
        let identity = crypto::generate_identity(&platform).unwrap();
        let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

        // Créer le graph et le storage
        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();

        // Créer la config de chiffrement
        let encryption = EncryptionConfig {
            platform: platform.clone(),
            recipient: recipient.clone(),
            identity: identity.clone(),
        };

        // Créer le gestionnaire de persistance avec chiffrement
        let persistence = GraphPersistence::new_with_encryption(
            graph,
            storage,
            "encrypted_graph_backups".to_string(),
            encryption,
        );

        let vault_id = "test_vault_encrypted";

        // Créer des nodes de test
        let node1_id = persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![1, 2, 3, 4, 5],
                "hmac_encrypted_1".to_string(),
                vec!["encrypted".to_string(), "test".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        let node2_id = persistence
            .graph
            .create_node(
                vault_id,
                "entity",
                vec![6, 7, 8, 9, 10],
                "hmac_encrypted_2".to_string(),
                vec!["sensitive".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        // Créer un edge
        let edge_props = EdgeProperties {
            weight: 0.95,
            bidirectional: true,
            encrypted_context: Some(vec![11, 12, 13]),
            metadata: Default::default(),
        };

        persistence
            .graph
            .create_edge(vault_id, &node1_id, &node2_id, "secure_link", edge_props)
            .await
            .unwrap();

        // Backup avec chiffrement
        persistence.backup(vault_id).await.unwrap();

        // Vérifier que le backup existe avec extension .age
        assert!(persistence.backup_exists(vault_id).await);

        // Vérifier que le fichier est bien chiffré (pas du JSON lisible)
        let encrypted_content = persistence
            .storage
            .read_file(&format!("encrypted_graph_backups/{}.age", vault_id))
            .await
            .unwrap();

        // Le contenu doit être en base64 (pas du JSON direct)
        assert!(!encrypted_content.starts_with("{"));
        assert!(!encrypted_content.contains("\"version\""));

        // Restaurer avec déchiffrement
        let restored_backup = persistence.restore(vault_id).await.unwrap();

        // Vérifier les données restaurées
        assert_eq!(restored_backup.version, 1);
        assert_eq!(restored_backup.nodes.len(), 2);
        assert_eq!(restored_backup.edges.len(), 1);

        // Vérifier l'intégrité des données
        let restored_node1 = restored_backup
            .nodes
            .iter()
            .find(|n| n.node_type == "memory")
            .unwrap();
        assert_eq!(restored_node1.encrypted_content, vec![1, 2, 3, 4, 5]);
        assert_eq!(restored_node1.content_hmac, "hmac_encrypted_1");

        let restored_edge = &restored_backup.edges[0];
        assert_eq!(restored_edge.edge_type, "secure_link");
        assert_eq!(restored_edge.properties.weight, 0.95);
        assert_eq!(restored_edge.properties.bidirectional, true);

        // Nettoyer
        persistence.delete_backup(vault_id).await.unwrap();
        assert!(!persistence.backup_exists(vault_id).await);
    }

    #[wasm_bindgen_test]
    async fn test_encryption_toggle() {
        // Tester l'activation/désactivation du chiffrement
        let platform = Platform::new();
        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();

        // Créer sans chiffrement
        let mut persistence = GraphPersistence::new(
            graph,
            storage,
            "toggle_graph_backups".to_string(),
        );

        let vault_id = "test_vault_toggle";

        // Créer un node
        persistence
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![1, 2, 3],
                "hmac1".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        // Backup sans chiffrement
        persistence.backup(vault_id).await.unwrap();

        // Vérifier que le fichier .json existe
        let json_content = persistence
            .storage
            .read_file(&format!("toggle_graph_backups/{}.json", vault_id))
            .await
            .unwrap();
        assert!(json_content.starts_with("{"));

        // Activer le chiffrement
        let identity = crypto::generate_identity(&platform).unwrap();
        let recipient = crypto::identity_to_public(&platform, &identity).unwrap();

        persistence.enable_encryption(EncryptionConfig {
            platform: platform.clone(),
            recipient,
            identity,
        });

        // Backup avec chiffrement (nouveau fichier .age)
        persistence.backup(vault_id).await.unwrap();

        // Vérifier que le fichier .age existe et est chiffré
        let age_content = persistence
            .storage
            .read_file(&format!("toggle_graph_backups/{}.age", vault_id))
            .await
            .unwrap();
        assert!(!age_content.starts_with("{"));

        // Désactiver le chiffrement
        persistence.disable_encryption();

        // Le fichier .json devrait toujours exister
        assert!(
            persistence
                .storage
                .read_file(&format!("toggle_graph_backups/{}.json", vault_id))
                .await
                .is_ok()
        );

        // Nettoyer les deux fichiers
        persistence
            .storage
            .delete_file(&format!("toggle_graph_backups/{}.json", vault_id))
            .await
            .unwrap();
        persistence
            .storage
            .delete_file(&format!("toggle_graph_backups/{}.age", vault_id))
            .await
            .unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_encrypted_backup_wrong_key() {
        // Tester qu'on ne peut pas déchiffrer avec la mauvaise clé
        let platform = Platform::new();

        // Générer deux identités différentes
        let identity1 = crypto::generate_identity(&platform).unwrap();
        let recipient1 = crypto::identity_to_public(&platform, &identity1).unwrap();

        let identity2 = crypto::generate_identity(&platform).unwrap();

        let graph = CozoGraphAdapter::new_in_memory().unwrap();
        let storage = OpfsStorage::new();

        // Chiffrer avec identity1
        let encryption1 = EncryptionConfig {
            platform: platform.clone(),
            recipient: recipient1,
            identity: identity1,
        };

        let persistence1 = GraphPersistence::new_with_encryption(
            graph,
            storage,
            "wrong_key_test".to_string(),
            encryption1,
        );

        let vault_id = "test_vault_wrong_key";

        // Créer un node
        persistence1
            .graph
            .create_node(
                vault_id,
                "memory",
                vec![1, 2, 3],
                "hmac".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        // Backup avec identity1
        persistence1.backup(vault_id).await.unwrap();

        // Tenter de restaurer avec identity2 (mauvaise clé)
        let graph2 = CozoGraphAdapter::new_in_memory().unwrap();
        let storage2 = OpfsStorage::new();

        let encryption2 = EncryptionConfig {
            platform: platform.clone(),
            recipient: "dummy_recipient".to_string(), // Peu importe pour le déchiffrement
            identity: identity2, // Mauvaise identité
        };

        let persistence2 = GraphPersistence::new_with_encryption(
            graph2,
            storage2,
            "wrong_key_test".to_string(),
            encryption2,
        );

        // La restauration devrait échouer
        let result = persistence2.restore(vault_id).await;
        assert!(result.is_err());

        // Nettoyer
        persistence1.delete_backup(vault_id).await.unwrap();
    }
}
