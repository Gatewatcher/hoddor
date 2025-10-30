import { useState } from 'react';
import { useSelector } from 'react-redux';

import { useServices } from '../contexts/ServicesContext';
import { appSelectors } from '../store/app.selectors';

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface UseChatMessagesOptions {
  enableRAG?: boolean;
}

export const useChatMessages = (options: UseChatMessagesOptions = {}) => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const { ragOrchestrator } = useServices();
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const useRAG = useSelector(appSelectors.getUseRAG);
  const useGraphRAG = useSelector(appSelectors.getUseGraphRAG);

  const sendMessage = async () => {
    if (!input.trim() || !ragOrchestrator) return;

    const userMessage: Message = {
      role: 'user',
      content: input,
      timestamp: new Date(),
    };

    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const assistantMessage: Message = {
        role: 'assistant',
        content: '',
        timestamp: new Date(),
      };

      setMessages(prev => [...prev, assistantMessage]);

      // Pass vault name if RAG is enabled and vault is selected
      const ragOptions =
        options.enableRAG && useRAG && selectedVault
          ? { vaultName: selectedVault, useGraphRAG }
          : {};

      let fullResponse = '';
      for await (const chunk of ragOrchestrator.queryStream(
        input,
        ragOptions,
      )) {
        fullResponse += chunk;
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = {
            ...assistantMessage,
            content: fullResponse,
          };
          return updated;
        });
      }
    } catch (error) {
      console.error('Chat failed:', error);
      setMessages(prev => [
        ...prev,
        {
          role: 'assistant',
          content: `Error: ${error}`,
          timestamp: new Date(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const clearMessages = () => {
    setMessages([]);
  };

  const addMessage = (message: Message) => {
    setMessages(prev => [...prev, message]);
  };

  return {
    messages,
    input,
    isLoading,
    setInput,
    sendMessage,
    clearMessages,
    addMessage,
  };
};
