import init from '../../hoddor/pkg/hoddor.js';

export const initWasm = async () => {
  try {
    await init();
  } catch (error) {
    console.error('Failed to initialize WASM:', error);
    throw error;
  }
};
