# Graph Persistence - Guide d'Utilisation

## Vue d'ensemble

Le système de persistance du graph permet de sauvegarder et restaurer automatiquement le graph CozoDB (in-memory) vers OPFS (Origin Private File System) du navigateur.

## Architecture

```
┌─────────────────┐
│  GraphPersistence│
│                 │
├─────────┬───────┤
│         │       │
▼         ▼       ▼
Graph    Storage  JSON
(CozoDB) (OPFS)   Backup
```

### Composants

- **GraphPersistence**: Couche d'orchestration pour backup/restore
- **CozoGraphAdapter**: Graph database in-memory (via GraphPort)
- **OpfsStorage**: Persistance fichiers dans le browser (via StoragePort)
- **GraphBackup**: Structure JSON contenant nodes + edges

## Utilisation de Base

### 1. Initialisation

```rust
use hoddor::adapters::wasm::{CozoGraphAdapter, OpfsStorage, GraphPersistence};

// Créer les composants
let graph = CozoGraphAdapter::new_in_memory()?;
let storage = OpfsStorage::new();

// Créer le gestionnaire de persistance
let persistence = GraphPersistence::new(
    graph,
    storage,
    "graph_backups".to_string() // Chemin OPFS pour les backups
);
```

### 2. Sauvegarder le Graph

```rust
// Backup complet d'un vault
persistence.backup("my_vault_id").await?;

// Le fichier sera sauvegardé dans:
// OPFS://graph_backups/my_vault_id.json
```

### 3. Restaurer le Graph

```rust
// Au démarrage de l'application
if persistence.backup_exists("my_vault_id").await {
    let backup = persistence.restore("my_vault_id").await?;
    println!("Restored {} nodes and {} edges",
             backup.nodes.len(),
             backup.edges.len());
}
```

### 4. Vérifier l'existence d'un Backup

```rust
if persistence.backup_exists("my_vault_id").await {
    // Le backup existe
} else {
    // Pas de backup trouvé
}
```

### 5. Supprimer un Backup

```rust
persistence.delete_backup("my_vault_id").await?;
```

### 6. Utiliser le Chiffrement Age

```rust
use hoddor::adapters::wasm::{GraphPersistence, EncryptionConfig};
use hoddor::domain::crypto;
use hoddor::platform::Platform;

// Générer une identité Age
let platform = Platform::new();
let identity = crypto::generate_identity(&platform)?;
let recipient = crypto::identity_to_public(&platform, &identity)?;

// Créer la configuration de chiffrement
let encryption = EncryptionConfig {
    platform: platform.clone(),
    recipient: recipient.clone(),
    identity: identity.clone(),
};

// Créer le gestionnaire avec chiffrement activé
let persistence = GraphPersistence::new_with_encryption(
    graph,
    storage,
    "graph_backups".to_string(),
    encryption,
);

// Tous les backups seront automatiquement chiffrés
persistence.backup("my_vault_id").await?;
// Sauvegardé dans: OPFS://graph_backups/my_vault_id.age

// La restauration déchiffre automatiquement
let backup = persistence.restore("my_vault_id").await?;
```

### 7. Activer/Désactiver le Chiffrement Dynamiquement

```rust
// Créer sans chiffrement
let mut persistence = GraphPersistence::new(graph, storage, "graph_backups".to_string());

// Activer le chiffrement plus tard
let platform = Platform::new();
let identity = crypto::generate_identity(&platform)?;
let recipient = crypto::identity_to_public(&platform, &identity)?;

persistence.enable_encryption(EncryptionConfig {
    platform,
    recipient,
    identity,
});

// Désactiver le chiffrement
persistence.disable_encryption();
```

## Format du Backup JSON

```json
{
  "version": 1,
  "created_at": 1234567890,
  "nodes": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "node_type": "memory",
      "vault_id": "my_vault",
      "labels": ["important"],
      "encrypted_content": [1, 2, 3],
      "content_hmac": "abc123...",
      "metadata": {
        "content_size": 3,
        "version": 1,
        "expires_at": null
      },
      "created_at": 1234567890,
      "updated_at": 1234567890,
      "accessed_at": 1234567890,
      "access_count": 0
    }
  ],
  "edges": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "from_node": "550e8400-e29b-41d4-a716-446655440000",
      "to_node": "770e8400-e29b-41d4-a716-446655440002",
      "edge_type": "relates_to",
      "vault_id": "my_vault",
      "properties": {
        "weight": 0.8,
        "bidirectional": false,
        "encrypted_context": null,
        "metadata": {}
      },
      "created_at": 1234567890
    }
  ]
}
```

## Patterns d'Utilisation Recommandés

### Auto-Save Périodique

```rust
use gloo_timers::future::TimeoutFuture;

async fn auto_save_loop(persistence: GraphPersistence<...>, vault_id: String) {
    loop {
        // Attendre 5 minutes
        TimeoutFuture::new(5 * 60 * 1000).await;

        // Sauvegarder
        if let Err(e) = persistence.backup(&vault_id).await {
            console_log(&format!("Auto-save failed: {}", e));
        }
    }
}
```

### Backup sur Changement

```rust
impl MyApp {
    async fn add_memory(&mut self, content: Vec<u8>) -> Result<()> {
        // 1. Créer le node dans le graph
        self.graph.create_node(
            &self.vault_id,
            "memory",
            content,
            // ... autres params
        ).await?;

        // 2. Sauvegarder immédiatement
        self.persistence.backup(&self.vault_id).await?;

        Ok(())
    }
}
```

### Restauration au Démarrage

```rust
impl MyApp {
    pub async fn init(vault_id: String) -> Result<Self> {
        let graph = CozoGraphAdapter::new_in_memory()?;
        let storage = OpfsStorage::new();
        let persistence = GraphPersistence::new(
            graph,
            storage,
            "graph_backups".to_string()
        );

        // Tenter de restaurer
        if persistence.backup_exists(&vault_id).await {
            persistence.restore(&vault_id).await?;
        }

        Ok(Self { persistence, vault_id })
    }
}
```

## Considérations de Performance

### Taille du Backup

- **JSON non compressé**: ~1KB par node/edge
- **Recommandation**: Limiter à ~10K nodes pour des performances optimales
- **Alternative**: Implémenter une compression (gzip) avant sauvegarde

### Fréquence de Sauvegarde

- **Lecture**: Très rapide (< 100ms pour 1K nodes)
- **Écriture**: Dépend de la taille (~100-500ms pour 1K nodes)
- **Recommandation**: Auto-save toutes les 5 minutes + backup manuel sur actions importantes

### Espace OPFS

- Le navigateur alloue généralement **quelques centaines de MB** par origine
- Vérifier l'espace disponible avec `navigator.storage.estimate()`

## Chiffrement Age

Le système de persistance intègre nativement le chiffrement Age pour sécuriser les backups.

### Fonctionnement

- **Avec chiffrement activé**: Les backups sont chiffrés avec Age et sauvegardés avec l'extension `.age`
- **Sans chiffrement**: Les backups sont en JSON clair avec l'extension `.json`
- **Encodage**: Les données chiffrées sont encodées en base64 pour le stockage texte dans OPFS

### Sécurité

- **Identité Age**: Clé privée utilisée pour déchiffrer les backups
- **Recipient**: Clé publique utilisée pour chiffrer les backups
- **Mauvaise clé**: La restauration échoue si l'identité ne correspond pas
- **Format Age**: Utilise le standard Age (age-encryption.org)

### Gestion des Clés

**Important**: L'identité Age (clé privée) doit être sauvegardée de manière sécurisée. Si elle est perdue, les backups chiffrés ne pourront plus être restaurés.

```rust
// Générer et sauvegarder l'identité
let identity = crypto::generate_identity(&platform)?;
// ⚠️ Sauvegarder cette identité de manière sécurisée !
// Exemple: localStorage, vault Hoddor, ou export utilisateur

// Pour réutiliser plus tard
let encryption = EncryptionConfig {
    platform,
    recipient: crypto::identity_to_public(&platform, &identity)?,
    identity, // Identité récupérée depuis le stockage sécurisé
};
```

## Prochaines Étapes

### TODO: Compression

```rust
use flate2::write::GzEncoder;

// Compresser avant sauvegarde
let compressed = compress_gzip(&json)?;
persistence.storage.write_file(path, &compressed).await?;
```

### TODO: Backup Incrémental

Au lieu de sauvegarder tout le graph à chaque fois, sauvegarder seulement les changements (delta).

## Troubleshooting

### Le backup ne se crée pas

- Vérifier que OPFS est disponible: `navigator.storage` existe
- Vérifier les permissions du navigateur
- Vérifier la console pour les erreurs JavaScript

### Le restore échoue

- Vérifier que le fichier existe dans OPFS (`.json` ou `.age`)
- Vérifier la version du backup (doit être compatible)
- Vérifier que le JSON n'est pas corrompu
- **Si chiffré**: Vérifier que la bonne identité Age est utilisée
- **Si chiffré**: Vérifier que le fichier `.age` existe (pas `.json`)

### Erreur de déchiffrement

- **"Decryption failed"**: L'identité Age utilisée ne correspond pas au backup
- **"Base64 decode failed"**: Le fichier `.age` est corrompu
- Vérifier que vous utilisez la même identité que lors du backup
- Si l'identité est perdue, le backup chiffré ne peut plus être restauré

### Performances dégradées

- Réduire la fréquence d'auto-save
- Implémenter le backup incrémental
- Compresser les backups
- Limiter le nombre de nodes/edges

## Exemples de Tests

Les tests complets sont disponibles dans `src/adapters/wasm/graph_persistence.rs`:

### Tests de base
- `test_backup_and_restore`: Backup/restore basique sans chiffrement
- `test_backup_nonexistent_vault`: Gestion des vaults vides
- `test_backup_with_multiple_edges`: Graph complexe avec plusieurs relations

### Tests de chiffrement
- `test_encrypted_backup_and_restore`: Backup/restore avec chiffrement Age
- `test_encryption_toggle`: Activation/désactivation dynamique du chiffrement
- `test_encrypted_backup_wrong_key`: Vérification qu'une mauvaise clé échoue

Pour les exécuter:

```bash
wasm-pack test --headless --chrome
```
