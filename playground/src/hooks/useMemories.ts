import { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';

import { graph_list_memory_nodes } from '../../../hoddor/pkg/hoddor';
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

        const decoder = new TextDecoder();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const loadedMemories: Memory[] = nodes.map((node: any) => {
          let content = '';
          try {
            if (node.encrypted_content && node.encrypted_content.length > 0) {
              content = decoder.decode(new Uint8Array(node.encrypted_content));
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
