import { useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import { useServices } from '../contexts/ServicesContext';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';

export const useServiceInitialization = () => {
  const [isInitializing, setIsInitializing] = useState(false);
  const [initProgress, setInitProgress] = useState(0);

  const selectedModel = useSelector(appSelectors.getSelectedModel);
  const { initializeServices, embeddingService } = useServices();
  const dispatch = useDispatch();

  const initialize = async (onSuccess?: (embeddingsReady: boolean) => void) => {
    setIsInitializing(true);
    setInitProgress(0);

    try {
      await initializeServices(selectedModel, progress => {
        setInitProgress(progress);
      });

      dispatch(actions.setServicesReady(true));

      const embeddingsReady = embeddingService?.isReady() ?? false;

      if (onSuccess) {
        onSuccess(embeddingsReady);
      }
    } catch (error) {
      console.error('Initialization failed:', error);
      throw error;
    } finally {
      setIsInitializing(false);
      setInitProgress(0);
    }
  };

  return {
    isInitializing,
    initProgress,
    initialize,
  };
};
