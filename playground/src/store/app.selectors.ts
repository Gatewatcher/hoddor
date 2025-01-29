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

export const appSelectors = {
  getVaults,
  getSelectedVault,
  getNamespaces,
  getIdentity,
  getJson,
  getImage,
};
