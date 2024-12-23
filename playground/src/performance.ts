import { VaultWorker } from './vault';
import init, { create_vault, read_from_vault, upsert_vault } from '../../hoddor/pkg/hoddor.js';

function multiplyLargeMatrices(size: number): number[][] {
  const matrix1 = Array(size).fill(0).map(() =>
    Array(size).fill(0).map(() => Math.random())
  );
  const matrix2 = Array(size).fill(0).map(() =>
    Array(size).fill(0).map(() => Math.random())
  );

  const result = Array(size).fill(0).map(() => Array(size).fill(0));

  for (let i = 0; i < size; i++) {
    for (let j = 0; j < size; j++) {
      for (let k = 0; k < size; k++) {
        result[i][j] += matrix1[i][k] * matrix2[k][j];
      }
    }
  }

  return result;
}

function blockThread(ms: number) {
  const start = performance.now();
  while (performance.now() - start < ms) {
    multiplyLargeMatrices(50);
  }
}

function generateLargeData(sizeInMb: number): object {
  const bytesPerEntry = 100;
  const entriesNeeded = (sizeInMb * 1024 * 1024) / bytesPerEntry;

  const data: Record<string, string> = {};
  for (let i = 0; i < entriesNeeded; i++) {
    data[`key_${i}`] = 'x'.repeat(90);
  }
  return data;
}

export async function runPerformanceTest(iterations: number = 10, dataSizeMb: number = 1, onProgress?: (count: number) => void) {
  await init();
  const results = {
    worker: { create: 0, read: 0, update: 0 },
    direct: { create: 0, read: 0, update: 0 }
  };

  const worker = new VaultWorker();
  const largeData = generateLargeData(dataSizeMb);

  for (let i = 0; i < iterations; i++) {
    onProgress?.(i);
    const namespace = `test_${i}` + Date.now().toString();

    blockThread(50);

    const createStart = performance.now();
    const vaultName = 'default-performance' + Date.now().toString();
    await worker.createVault(vaultName, 'test123', namespace, largeData);
    results.worker.create += performance.now() - createStart;

    const readStart = performance.now();
    await worker.readFromVault(vaultName, 'test123', namespace)
    results.worker.read += performance.now() - readStart;

    const updateStart = performance.now();
    await worker.upsertVault(vaultName, 'test123', namespace + Date.now().toString(), { ...largeData, updated: true });
    results.worker.update += performance.now() - updateStart;
  }

  for (let i = 0; i < iterations; i++) {
    onProgress?.(i + iterations);
    const namespace = `direct_${i}` + Date.now().toString();

    blockThread(50);

    const createStart = performance.now();
    const vaultName = 'default-performance' + Date.now().toString();

    await create_vault(vaultName, 'test123', namespace, largeData);
    results.direct.create += performance.now() - createStart;

    const readStart = performance.now();
    await read_from_vault(vaultName, 'test123', namespace);
    results.direct.read += performance.now() - readStart;

    const updateStart = performance.now();    
    await upsert_vault(vaultName, 'test123', namespace + Date.now().toString(), { ...largeData, updated: true }, undefined, false);
    results.direct.update += performance.now() - updateStart;
  }

  for (const impl of ['worker', 'direct'] as const) {
    for (const op of ['create', 'read', 'update'] as const) {
      results[impl][op] = results[impl][op] / iterations;
    }
  }

  return {
    ...results,
    dataSizeMb,
    iterations
  };
}
