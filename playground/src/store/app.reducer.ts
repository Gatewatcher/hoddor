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
  video?: string;
  markdown: string | null;
  text: string | null;
  audio: string | null;
};

const initialState: AppState = {
  vaults: [],
  namespaces: [],
  markdown: null,
  text: null,
  audio: null,
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
      image: initialState.image,
      video: initialState.video,
    };
  }

  if (actions.setImage.match(action)) {
    return {
      ...state,
      image: action.payload,
      video: initialState.video,
      json: initialState.json,
    };
  }

  if (actions.setVideo.match(action)) {
    return {
      ...state,
      video: action.payload,
      image: initialState.image,
      json: initialState.json,
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
      video: initialState.video,
    };
  }

  if (actions.flushIdentity.match(action)) {
    return {
      ...state,
      identity: initialState.identity,
      json: initialState.json,
      image: initialState.image,
      video: initialState.video,
    };
  }

  if (actions.setMarkdown.match(action)) {
    return {
      ...state,
      markdown: action.payload,
      image: initialState.image,
      video: initialState.video,
      json: initialState.json,
      audio: initialState.audio,
      text: initialState.text,
    };
  }

  if (actions.setText.match(action)) {
    return {
      ...state,
      text: action.payload,
      image: initialState.image,
      video: initialState.video,
      json: initialState.json,
      audio: initialState.audio,
      markdown: initialState.markdown,
    };
  }

  if (actions.setAudio.match(action)) {
    return {
      ...state,
      audio: action.payload,
      image: initialState.image,
      video: initialState.video,
      json: initialState.json,
      text: initialState.text,
      markdown: initialState.markdown,
    };
  }

  return state;
};
