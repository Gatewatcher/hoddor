import { configureStore } from '@reduxjs/toolkit';

import { AppState, appReducer } from './app.reducer';

export type States = {
  appState: AppState;
};

const initReduxStore = () => {
  return configureStore({
    reducer: {
      appState: appReducer,
    },
    devTools: true,
  });
};

export const reduxStore = initReduxStore();
