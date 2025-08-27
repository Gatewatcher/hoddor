import init from '../../dist/hoddor.js';

export const initWasm = async () => {
  try {
    await init();
  } catch (error) {
    console.error('Failed to initialize WASM:', error);
    throw error;
  }
};
