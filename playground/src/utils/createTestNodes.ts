/**
 * Utility to create 3 connected test memory nodes for Graph RAG testing
 */

import {
  graph_create_memory_node,
  graph_create_edge,
} from '../../../hoddor/pkg/hoddor';
import { EmbeddingService } from '../services/embedding';

export async function createTestNodes(
  vaultName: string,
  embeddingService: EmbeddingService,
): Promise<string[]> {
  const chunks = [
    'Pour installer notre application, commencez par cloner le repository Git avec la commande git clone. Ensuite, installez les dépendances avec npm install. Assurez-vous d\'avoir Node.js version 18 ou supérieure installé sur votre système.',
    'Après l\'installation mentionnée précédemment, lancez le serveur de développement avec npm run dev. L\'application sera accessible sur http://localhost:3000. Si le port est déjà utilisé, le système choisira automatiquement le port suivant disponible.',
    'Une fois le serveur démarré comme indiqué ci-dessus, vous pouvez tester l\'application en ouvrant votre navigateur. Pour la production, utilisez npm run build suivi de npm start. Les logs seront disponibles dans le dossier logs/.',
  ];

  const nodeIds: string[] = [];

  // Create 3 nodes with REAL embeddings
  for (let i = 0; i < chunks.length; i++) {
    const content = new TextEncoder().encode(chunks[i]);

    // Generate real embedding using EmbeddingService
    const { embedding } = await embeddingService.embed(chunks[i]);

    const labels = ['documentation', 'pagination', `chunk_${i + 1}`];

    const nodeId = await graph_create_memory_node(
      vaultName,
      content,
      new Float32Array(embedding),
      labels,
    );

    nodeIds.push(nodeId);
  }

  // Connect with edges
  for (let i = 0; i < nodeIds.length - 1; i++) {
    await graph_create_edge(
      vaultName,
      nodeIds[i],
      nodeIds[i + 1],
      'next_chunk',
      1.0,
      true,
    );
  }

  return nodeIds;
}
