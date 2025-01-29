import { Reducer, UnknownAction } from '@reduxjs/toolkit';

import { actions } from './app.actions';
import { Identity } from './models/Identity';

export type AppState = {
  vaults: string[];
  selectedVault?: string;
  namespaces: string[];
  identity?: Identity;
  json?: {};
  image?: string;
};

const initialState: AppState = {
  vaults: [],
  namespaces: [],
};

export const appReducer: Reducer<AppState, UnknownAction> = (
  state = initialState,
  action,
) => {
  if (actions.setVaults.match(action)) {
    return {
      ...state,
      vaults: action.payload,
    };
  }

  if (actions.selectVault.match(action)) {
    return {
      ...state,
      identity: initialState.identity,
      selectedVault: action.payload,
      json: initialState.json,
    };
  }

  if (actions.setNamespaces.match(action)) {
    return {
      ...state,
      namespaces: action.payload,
    };
  }

  if (actions.addIdentity.match(action)) {
    return {
      ...state,
      identity: action.payload,
    };
  }

  if (actions.setJson.match(action)) {
    return {
      ...state,
      json: action.payload,
    };
  }

  if (actions.setImage.match(action)) {
    return {
      ...state,
      image: action.payload,
    };
  }

  if (actions.deleteVault.match(action)) {
    return {
      ...state,
      vaults: initialState.vaults,
      selectedVault: initialState.selectedVault,
      namespaces: initialState.namespaces,
      identity: initialState.identity,
      json: initialState.json,
      image: initialState.image,
    };
  }

  if (actions.flushIdentity.match(action)) {
    return {
      ...state,
      identity: initialState.identity,
      json: initialState.json,
      image: initialState.image,
    };
  }

  return state;
};
