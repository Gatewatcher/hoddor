import { States } from './app.store';

const getVaults = (state: States) => {
  return state.appState.vaults;
};

const getSelectedVault = (state: States) => {
  return state.appState.selectedVault;
};

const getNamespaces = (state: States) => {
  return state.appState.namespaces;
};

const getIdentity = (state: States) => {
  return state.appState.identity;
};

const getJson = (state: States) => {
  return state.appState.json;
};

const getImage = (state: States) => {
  return state.appState.image;
};

const getVideo = (state: States) => {
  return state.appState.video;
};

const getMarkdown = (state: States) => {
  return state.appState.markdown;
};

const getText = (state: States) => {
  return state.appState.text;
};

const getAudio = (state: States) => {
  return state.appState.audio;
};

const getSelectedModel = (state: States) => {
  return state.appState.selectedModel;
};

const getUseRAG = (state: States) => {
  return state.appState.useRAG;
};

const getServicesReady = (state: States) => {
  return state.appState.servicesReady;
};

const getMemoryRefreshTrigger = (state: States) => {
  return state.appState.memoryRefreshTrigger;
};

export const appSelectors = {
  getVaults,
  getSelectedVault,
  getNamespaces,
  getIdentity,
  getJson,
  getImage,
  getVideo,
  getMarkdown,
  getText,
  getAudio,
  getSelectedModel,
  getUseRAG,
  getServicesReady,
  getMemoryRefreshTrigger,
};
