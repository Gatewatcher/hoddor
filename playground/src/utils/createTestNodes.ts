import {
  graph_create_edge,
  graph_create_memory_node,
} from '../../../hoddor/pkg/hoddor';
import { EmbeddingService } from '../services/embedding';

export async function createTestNodes(
  vaultName: string,
  embeddingService: EmbeddingService,
): Promise<string[]> {
  const chunks = [
    'To install our application, start by cloning the Git repository with the git clone command. Then, install dependencies with npm install. Make sure you have Node.js version 18 or higher installed on your system.',
    'After the installation mentioned previously, launch the development server with npm run dev. The application will be accessible at http://localhost:3000. If the port is already in use, the system will automatically choose the next available port.',
    'Once the server is started as indicated above, you can test the application by opening your browser. For production, use npm run build followed by npm start. Logs will be available in the logs/ folder.',
  ];

  const nodeIds: string[] = [];

  for (let i = 0; i < chunks.length; i++) {
    const { embedding } = await embeddingService.embed(chunks[i]);

    const labels = ['documentation', 'pagination', `chunk_${i + 1}`];

    const nodeId = await graph_create_memory_node(
      vaultName,
      chunks[i],
      new Float32Array(embedding),
      labels,
    );

    nodeIds.push(nodeId);
  }

  for (let i = 0; i < nodeIds.length - 1; i++) {
    await graph_create_edge(
      vaultName,
      nodeIds[i],
      nodeIds[i + 1],
      'next_chunk',
      1.0,
    );
  }

  return nodeIds;
}
