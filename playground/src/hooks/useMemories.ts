import { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';

import {
  graph_get_neighbors,
  graph_list_memory_nodes,
} from '../../../hoddor/pkg/hoddor';
import { appSelectors } from '../store/app.selectors';

interface Memory {
  id: string;
  content: string;
  labels: string[];
  timestamp: Date;
}

export const useMemories = () => {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const vaultName = useSelector(appSelectors.getSelectedVault);
  const refreshTrigger = useSelector(appSelectors.getMemoryRefreshTrigger);

  useEffect(() => {
    const loadMemories = async () => {
      if (!vaultName) {
        setMemories([]);
        return;
      }

      setIsLoading(true);
      try {
        const nodes = await graph_list_memory_nodes(vaultName, 100);

        console.log('Node 0:', nodes[0].id);
        const neighbors = await graph_get_neighbors('my-vault', nodes[0].id, [
          'next_chunk',
        ]);
        console.log('Neighbors of node 0:', neighbors);

        console.log(nodes);

        const decoder = new TextDecoder();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const loadedMemories: Memory[] = nodes.map((node: any) => {
          let content = '';
          try {
            if (node.content && node.content.length > 0) {
              content = decoder.decode(new Uint8Array(node.content));
            }
          } catch (error) {
            console.error('Failed to decode memory:', error);
            content = '[Unable to decode content]';
          }

          return {
            id: node.id,
            content,
            labels: node.labels || [],
            timestamp: new Date(),
          };
        });

        setMemories(loadedMemories);
      } catch (error) {
        console.error('Failed to load memories:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadMemories();
  }, [vaultName, refreshTrigger]);

  const addMemory = (memory: Memory) => {
    setMemories(prev => [memory, ...prev]);
  };

  return {
    memories,
    isLoading,
    addMemory,
  };
};
