import React, {
  ReactNode,
  createContext,
  useContext,
  useRef,
  useState,
} from 'react';

import { EmbeddingService, RAGOrchestrator, WebLLMService } from '../services';

interface ServicesContextType {
  llmService: WebLLMService | null;
  embeddingService: EmbeddingService | null;
  ragOrchestrator: RAGOrchestrator | null;
  initializeServices: (
    model: string,
    onProgress?: (progress: number) => void,
  ) => Promise<{ embeddingsReady: boolean }>;
}

const ServicesContext = createContext<ServicesContextType | undefined>(
  undefined,
);

export const useServices = () => {
  const context = useContext(ServicesContext);
  if (!context) {
    throw new Error('useServices must be used within ServicesProvider');
  }
  return context;
};

interface ServicesProviderProps {
  children: ReactNode;
}

export const ServicesProvider: React.FC<ServicesProviderProps> = ({
  children,
}) => {
  const llmServiceRef = useRef<WebLLMService | null>(null);
  const embeddingServiceRef = useRef<EmbeddingService | null>(null);
  const ragOrchestratorRef = useRef<RAGOrchestrator | null>(null);
  const [, forceUpdate] = useState({});

  const initializeServices = async (
    model: string,
    onProgress?: (progress: number) => void,
  ): Promise<{ embeddingsReady: boolean }> => {
    const llmService = new WebLLMService(model);
    await llmService.initialize(report => {
      if (onProgress) {
        onProgress(report.progress * 100);
      }
    });
    llmServiceRef.current = llmService;

    let embeddingService: EmbeddingService | null = null;
    let embeddingsReady = false;
    try {
      embeddingService = new EmbeddingService();
      await embeddingService.initialize();
      embeddingServiceRef.current = embeddingService;
      embeddingsReady = embeddingService.isReady();
    } catch (embError) {
      console.error('Embedding initialization failed:', embError);
      embeddingService = new EmbeddingService();
      embeddingServiceRef.current = embeddingService;
      embeddingsReady = false;
    }

    const ragOrchestrator = new RAGOrchestrator(llmService, embeddingService);
    ragOrchestratorRef.current = ragOrchestrator;

    forceUpdate({});

    return { embeddingsReady };
  };

  return (
    <ServicesContext.Provider
      value={{
        llmService: llmServiceRef.current,
        embeddingService: embeddingServiceRef.current,
        ragOrchestrator: ragOrchestratorRef.current,
        initializeServices,
      }}
    >
      {children}
    </ServicesContext.Provider>
  );
};
