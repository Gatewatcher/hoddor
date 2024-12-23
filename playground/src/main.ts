import './style.css';
import init, { set_debug_mode, upsert_vault, read_from_vault, remove_from_vault, remove_vault, list_vaults } from '../../hoddor/pkg/hoddor.js';
import { VaultWorker } from './vault';
import { runPerformanceTest } from './performance';

const PASSWORD = 'password123';
const vault = new VaultWorker();

async function fetchImageAsBytes(url: string): Promise<Uint8Array> {
  const response = await fetch(url);
  const blob = await response.blob();
  const arrayBuffer = await blob.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}

function arrayBufferToBase64(buffer: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < buffer.length; i++) {
    binary += String.fromCharCode(buffer[i]);
  }
  return btoa(binary);
}

async function storeImage(password: string, imageBytes: Uint8Array) {
  try {
    const namespace = 'image_storage_v3';
    const numberArray = Array.from(imageBytes);
    console.log('Storing image with length:', numberArray.length);

    try {
      await remove_from_vault(password, namespace).catch(() => { });
    } catch (e) {
      console.warn('Failed to remove old image data:', e);
    }

    try {
      await upsert_vault(password, namespace, numberArray);
    } catch (e) {
      console.error('Error in storeImage:', e);
      throw e;
    }
    console.log('Image stored successfully');
  } catch (e) {
    console.error('Failed to store image:', e);
    throw e;
  }
}

async function displayStoredImage(password: string) {
  try {
    const namespace = 'image_storage_v3';
    const retrievedData = await read_from_vault(password, namespace);

    const dataArray = Array.isArray(retrievedData) ? retrievedData : Array.from(retrievedData as any);
    const uint8Array = new Uint8Array(dataArray);
    console.log('Converted array length:', uint8Array.length);

    const base64String = arrayBufferToBase64(uint8Array);
    const img = document.createElement('img');
    img.src = `data:image/jpeg;base64,${base64String}`;
    document.body.appendChild(img);
    console.log('Image displayed successfully');
    return true;
  } catch (error) {
    console.error('Error reading image:', error);
    if (error.toString().includes('Namespace not found')) {
      console.log('No stored image found, will create a new one');
      return false;
    }
    throw error;
  }
}

async function storeVideo(password: string, videoFile: File) {
  const chunkSize = 1024 * 1024; // 1MB
  const reader = new FileReader();
  let offset = 0;

  const readChunk = (blob: Blob): Promise<Uint8Array> => {
    return new Promise((resolve, reject) => {
      reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
      reader.onerror = reject;
      reader.readAsArrayBuffer(blob);
    });
  };

  try {
    const metadata = JSON.stringify({
      size: videoFile.size,
      type: videoFile.type,
      chunks: Math.ceil(videoFile.size / chunkSize),
      fileName: videoFile.name,
      lastModified: videoFile.lastModified
    });
    await vault.upsertVault(password, 'test_video_meta', Array.from(new TextEncoder().encode(metadata)));

    const firstChunk = videoFile.slice(0, chunkSize);
    const firstChunkData = await readChunk(firstChunk);
    await vault.upsertVault(password, 'test_video_0', Array.from(firstChunkData));

    while (offset < videoFile.size) {
      const chunk = videoFile.slice(offset, offset + chunkSize);
      const chunkData = await readChunk(chunk);
      await vault.upsertVault(password, `test_video_${offset}`, Array.from(chunkData));
      offset += chunkSize;
      const progress = Math.round((offset / videoFile.size) * 100);
      console.log(`Upload progress: ${progress}%`);
    }

    console.log('Video stored successfully');
  } catch (e) {
    console.error('Failed to store video:', e);
    throw e;
  }
}

async function displayStoredVideo(password: string) {
  try {
    // 1) Read metadata
    const metadataRaw = await read_from_vault(password, 'test_video_meta');
    const metadataText = new TextDecoder().decode(new Uint8Array(metadataRaw as number[]));
    const metadata = JSON.parse(metadataText);

    const chunkSize = 1024 * 1024;
    const chunks: Uint8Array[] = [];

    // 2) Read chunks
    for (let offset = 0; offset < metadata.size; offset += chunkSize) {
      const chunkNamespace = `test_video_${offset}`;
      const chunkData = await read_from_vault(password, chunkNamespace);
      chunks.push(new Uint8Array(chunkData as number[]));

      const progress = Math.round((offset / metadata.size) * 100);
      console.log(`Download progress: ${progress}%`);
    }

    // 3) Combine chunks
    const fullData = new Uint8Array(metadata.size);
    let mergedOffset = 0;
    for (const chunk of chunks) {
      fullData.set(chunk, mergedOffset);
      mergedOffset += chunk.length;
    }

    // 4) Create video element
    const blob = new Blob([fullData], { type: metadata.type });
    const url = URL.createObjectURL(blob);
    const video = document.createElement('video');
    video.src = url;
    video.controls = true;
    document.body.appendChild(video);

    console.log('Video retrieved successfully');
    return true;
  } catch (error) {
    console.error('Error reading video:', error);
    if (error.toString().includes('Namespace not found')) {
      console.log('No stored video found');
      return false;
    }
    throw error;
  }
}

const fileInput = document.createElement('input');
fileInput.type = 'file';
fileInput.accept = 'video/*';
fileInput.onchange = async (e) => {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (file) {
    await storeVideo(PASSWORD, file);
    await displayStoredVideo(PASSWORD);
  }
};
document.body.appendChild(fileInput);

const perfButton = document.createElement('button');
perfButton.textContent = 'Run Performance Test';

const operationLog: { time: number; operation: string }[] = [];
const startTime = Date.now();

const logOperation = (operation: string) => {
  operationLog.push({
    time: Date.now() - startTime,
    operation
  });
};

perfButton.onclick = async () => {
  perfButton.disabled = true;
  const progressDiv = document.createElement('div');
  operationLog.length = 0;
  document.body.appendChild(progressDiv);

  // Test with 1MB, 5MB, and 10MB
  const dataSizes = [1, 5, 10];
  const results = [];

  try {
    for (const size of dataSizes) {
      progressDiv.textContent = `Testing with ${size}MB data...`;
      const result = await runPerformanceTest(10, size, (i) => {
        logOperation(`Progress (${size}MB): ${i}%`);
        progressDiv.textContent = `Testing ${size}MB data: ${i}%`;
      });
      results.push(result);
    }

    const resultsDiv = document.createElement('div');
    resultsDiv.innerHTML = `
            <h3>Large Data Performance Test Results</h3>
            ${results.map(result => `
                <h4>Test with ${result.dataSizeMb}MB data (${result.iterations} iterations)</h4>
                <pre>${JSON.stringify({
      worker: result.worker,
      direct: result.direct
    }, null, 2)}</pre>
            `).join('')}
            
            <h4>Operation Timeline</h4>
            <pre>${operationLog.map(log =>
      `[${log.time}ms] ${log.operation}`
    ).join('\n')}</pre>
        `;
    document.body.appendChild(resultsDiv);
  } finally {
    perfButton.disabled = false;
    progressDiv.remove();
  }
};

document.body.appendChild(perfButton);

const exportButton = document.createElement('button');
exportButton.textContent = 'Export Vault';
exportButton.onclick = async () => {
  try {
    const vaultData = await vault.exportVault(PASSWORD);
    const blob = new Blob([vaultData], { type: 'application/octet-stream' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'vault_backup.dat';
    a.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    console.error('Failed to export vault:', error);
    alert('Failed to export vault: ' + error);
  }
};
document.body.appendChild(exportButton);

const importInput = document.createElement('input');
importInput.type = 'file';
importInput.accept = '*'; // Accept all file types, not just .dat
importInput.style.display = 'none';

const importButton = document.createElement('button');
importButton.textContent = 'Import Vault';
importButton.onclick = () => importInput.click();

importInput.onchange = async (e) => {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;

  try {
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    console.log('Importing vault data of size:', uint8Array.length, 'bytes');
    
    if (uint8Array.length > 6) {
      const header = new TextDecoder().decode(uint8Array.slice(0, 6));
      console.log('Detected format:', header === 'VAULT1' ? 'Binary vault format' : 'Legacy format');
    }

    await vault.importVault(PASSWORD, uint8Array);
    alert('Vault imported successfully');
    location.reload();
  } catch (error) {
    console.error('Failed to import vault:', error);
    alert('Failed to import vault: ' + error);
  } finally {
    importInput.value = '';
  }
};

document.body.appendChild(importButton);
document.body.appendChild(importInput);

const removeVaultButton = document.createElement('button');
removeVaultButton.textContent = 'Remove Vault';
removeVaultButton.onclick = async () => {
  try {
    await remove_vault();
    alert('Vault removed successfully');
  } catch (error) {
    console.error('Failed to remove vault:', error);
    alert('Failed to remove vault: ' + error);
  }
};
document.body.appendChild(removeVaultButton);

const counterDiv = document.createElement('div');
counterDiv.textContent = '0';
counterDiv.style.position = 'absolute';
counterDiv.style.bottom = '20px';
counterDiv.style.left = '20px';

document.body.appendChild(counterDiv);

let counter = 0;
setInterval(() => {
  counter++;
  counterDiv.textContent = counter.toString();
}, 100);

interface UserData {
  id: string;
  name: string;
  email: string;
  preferences: {
    theme: 'light' | 'dark';
    notifications: boolean;
  };
  lastUpdated: number;
}

async function storeUserData(password: string, userData: UserData) {
  try {
    const namespace = 'user_data_v1';
    console.log('Storing user data:', userData);

    try {
      await upsert_vault(password, namespace, userData);
    } catch (e) {
      await upsert_vault(password, namespace, userData);
    }
    console.log('User data stored successfully');
  } catch (e) {
    console.error('Failed to store user data:', e);
    throw e;
  }
}

async function retrieveUserData(password: string): Promise<UserData | null> {
  try {
    const namespace = 'user_data_v1';
    const retrievedData = await read_from_vault(password, namespace);
    console.log('Retrieved user data:', retrievedData);
    return retrievedData as UserData;
  } catch (error) {
    console.error('Error reading user data:', error);
    if (error.toString().includes('Namespace not found')) {
      return null;
    }
    throw error;
  }
}

async function testUserData() {
  const password = PASSWORD;
  const testUser: UserData = {
    id: '123',
    name: 'John Doe',
    email: 'john@example.com',
    preferences: {
      theme: 'dark',
      notifications: true
    },
    lastUpdated: Date.now()
  };

  await storeUserData(password, testUser);
  await retrieveUserData(password);
}

async function run() {
  try {
    await init();
    set_debug_mode(false);

    const password = PASSWORD;

    await testUserData().catch(error => {
      console.warn('User data test failed:', error);
    });

    const imageExists = await displayStoredImage(password).catch(error => {
      console.warn('Failed to display stored image:', error);
      return false;
    });

    if (!imageExists) {
      try {
        const imageUrl = 'https://picsum.photos/200/300';
        const imageBytes = await fetchImageAsBytes(imageUrl);
        await storeImage(password, imageBytes);
        await displayStoredImage(password);
      } catch (error) {
        console.error('Failed to store/display new image:', error);
      }
    }
  } catch (error) {
    console.error('Critical operation failed:', error);
  }
}

run().catch(console.error);

async function main() {
  try {
    await vault.createVault(PASSWORD, 'test', { foo: 'bar' });

    await vault.readFromVault(PASSWORD, 'test');
    const namespaces = await vault.listNamespaces(PASSWORD);
    console.log('Available namespaces:', namespaces);

    const vaults = await list_vaults();
    console.log('Available vaults:', vaults);
    
  } catch (error) {
    console.error('Error:', error);
  }
}

main()
