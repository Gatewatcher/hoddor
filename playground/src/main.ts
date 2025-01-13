import './style.css';
import init, { set_debug_mode, upsert_vault, read_from_vault, remove_from_vault, remove_vault, list_vaults, register, authenticate, create_credential, get_credential } from '../../hoddor/pkg/hoddor.js';
import { VaultWorker } from './vault';
import { runPerformanceTest } from './performance';

const BASE_URL = 'http://localhost:8080';
const PASSWORD = 'password123';
const STUN_SERVERS = [
  'stun:stun1.l.google.com:19302',
  'stun:stun2.l.google.com:19302',
];

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
      await remove_from_vault('default', password, namespace).catch(() => { });
    } catch (e) {
      console.warn('Failed to remove old image data:', e);
    }

    try {
      await upsert_vault('default', password, namespace, numberArray, undefined, false);
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
    const retrievedData = await read_from_vault('default', password, namespace);

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
  const chunkSize = 1024 * 1024;
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
    await vault.upsertVault('default', password, 'test_video_meta', Array.from(new TextEncoder().encode(metadata)), BigInt(5 * 60000), true);

    const firstChunk = videoFile.slice(0, chunkSize);
    const firstChunkData = await readChunk(firstChunk);
    await vault.upsertVault('default', password, 'test_video_0', Array.from(firstChunkData), BigInt(5 * 60000), true);

    while (offset < videoFile.size) {
      const chunk = videoFile.slice(offset, offset + chunkSize);
      const chunkData = await readChunk(chunk);
      await vault.upsertVault('default', password, `test_video_${offset}`, Array.from(chunkData), BigInt(5 * 60000), true);
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
    const metadataRaw = await read_from_vault('default', password, 'test_video_meta');
    const metadataText = new TextDecoder().decode(new Uint8Array(metadataRaw as number[]));
    const metadata = JSON.parse(metadataText);

    const chunkSize = 1024 * 1024;
    const chunks: Uint8Array[] = [];

    // 2) Read chunks
    for (let offset = 0; offset < metadata.size; offset += chunkSize) {
      const chunkNamespace = `test_video_${offset}`;
      const chunkData = await read_from_vault('default', password, chunkNamespace);
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
fileInput.style.display = 'none';

const importInput = document.createElement('input');
importInput.type = 'file';
importInput.accept = '*';
importInput.style.display = 'none';

const fileButton = document.createElement('button');
fileButton.textContent = 'Select Video File';
fileButton.onclick = () => fileInput.click();

const importButton = document.createElement('button');
importButton.textContent = 'Import Vault';
importButton.onclick = () => importInput.click();

fileInput.onchange = async (e) => {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (file) {
    await storeVideo(PASSWORD, file);
    await displayStoredVideo(PASSWORD);
  }
};

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

    await vault.importVault('default', uint8Array);
    alert('Vault imported successfully');
    location.reload();
  } catch (error) {
    console.error('Failed to import vault:', error);
    alert('Failed to import vault: ' + error);
  } finally {
    importInput.value = '';
  }
};

document.body.appendChild(fileButton);
document.body.appendChild(fileInput);
document.body.appendChild(importButton);
document.body.appendChild(importInput);

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

const Register = document.createElement('button');
Register.textContent = 'Register';

Register.onclick = async () => {
  const username = prompt('Enter a username');
  
  if (username) {
    await startRegistration(username);
  }
};

document.body.appendChild(Register);

const Authenticate = document.createElement('button');
Authenticate.textContent = 'Authenticate';

Authenticate.onclick = async () => {
  const username = prompt('Enter a username');
  
  if (username) {
    await startAuthentication(username);
  }
};

document.body.appendChild(Authenticate);

const exportButton = document.createElement('button');
exportButton.textContent = 'Export Vault';
exportButton.onclick = async () => {
  try {
    const vaultData = await vault.exportVault('default', PASSWORD);
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

const removeVaultButton = document.createElement('button');
removeVaultButton.textContent = 'Remove Vault';
removeVaultButton.onclick = async () => {
  try {
    await remove_vault('default', PASSWORD);
    alert('Vault removed successfully');
  } catch (error) {
    console.error('Failed to remove vault:', error);
    alert('Failed to remove vault: ' + error);
  }
};
document.body.appendChild(removeVaultButton);

const expirationTestButton = document.createElement('button');
expirationTestButton.textContent = 'Test Data Expiration';
expirationTestButton.onclick = async () => {
  const statusDiv = document.createElement('div');
  statusDiv.style.marginTop = '10px';
  document.body.appendChild(statusDiv);

  try {
    const testData = { message: "This data will expire soon!" };
    await vault.createVault("expiration_test", PASSWORD, "test_namespace", testData, 5n);
    console.log("Created data with 5 second expiration");

    const initialData = await vault.readFromVault("expiration_test", PASSWORD, "test_namespace");
    statusDiv.textContent = "Initial read successful: " + JSON.stringify(Object.fromEntries(initialData));

    // Try reading every second for 10 seconds
    for (let i = 0; i < 10; i++) {
      await new Promise(resolve => setTimeout(resolve, 1000));
      try {
        const data = await vault.readFromVault("expiration_test", PASSWORD, "test_namespace");
        statusDiv.textContent = `${i + 1}s: Data still accessible: ${JSON.stringify(Object.fromEntries(data))}`;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        statusDiv.textContent = `${i + 1}s: Data expired: ${errorMessage}`;
        console.log('Data expired:', errorMessage);
        break;
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('Expiration test failed:', errorMessage);
    statusText.textContent = `Test failed: ${errorMessage}`;
  }
};
document.body.appendChild(expirationTestButton);

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
      await upsert_vault('default', password, namespace, userData, undefined, false);
    } catch (e) {
      await upsert_vault('default', password, namespace, userData, undefined, false);
    }
    console.log('User data stored successfully');
  } catch (e) {
    console.error('Failed to store user data:', e);
    throw e;
  }
}

async function startRegistration(username: string) {
  try {
    await create_credential(username);
    
    console.log('User registered successfully');
  } catch (e) {
    console.error('Failed to register user:', e);
    // throw e;
  }
}

async function startAuthentication(username: string) {
  try {
    await get_credential();
    
    console.log('User authenticated successfully');
  } catch (e) {
    console.error('Failed to authenticate user:', e);
    // throw e;
  }
}

async function retrieveUserData(password: string): Promise<UserData | null> {
  try {
    const namespace = 'user_data_v1';
    const retrievedData = await read_from_vault('default', password, namespace);
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
    set_debug_mode(true);

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

interface TodoItem {
  id: string;
  text: string;
  completed: boolean;
  lastModified: number;
}

interface TodoList {
  items: TodoItem[];
  lastSync: number;
}

let cachedTodos: TodoList | null = null;
const READ_INTERVAL = 1000;

async function readTodoList(): Promise<TodoList> {
  try {
    const data = await read_from_vault('todos', PASSWORD, 'todo_list');
    if (data) {
      const todoData = new TextDecoder().decode(new Uint8Array(data as number[]));
      const newTodos = JSON.parse(todoData);

      if (!cachedTodos || JSON.stringify(newTodos) !== JSON.stringify(cachedTodos)) {
        console.log('New todos received:', newTodos);
        const todoContainer = document.querySelector('[data-todo-container]');
        if (todoContainer) {
          const inputContainer = todoContainer.querySelector('[data-input-container]');
          const list = todoContainer.querySelector('[data-list]');
          const waitingMessage = todoContainer.querySelector('[data-waiting-message]');
          if (inputContainer && list && waitingMessage) {
            inputContainer.style.display = 'flex';
            list.style.display = 'block';
            list.style.color = 'black';
            waitingMessage.style.display = 'none';
          }
        }

        cachedTodos = newTodos;
        const syncStatus = document.querySelector('[data-sync-status]');
        if (syncStatus) {
          updateSyncStatus(syncStatus, cachedTodos.lastSync);
        }
      }

      return newTodos;
    }
    return cachedTodos || { items: [], lastSync: Date.now() };
  } catch (e) {
    console.error('Error reading todo list:', e);
    return cachedTodos || { items: [], lastSync: Date.now() };
  }
};

async function writeTodoList(todos: TodoList) {
  try {
    const todoData = new TextEncoder().encode(JSON.stringify(todos));
    await upsert_vault('todos', PASSWORD, 'todo_list', Array.from(todoData), undefined, true);
    cachedTodos = todos;
    const syncStatus = document.querySelector('[data-sync-status]');
    if (syncStatus) {
      updateSyncStatus(syncStatus, todos.lastSync);
    }
  } catch (e) {
    console.error('Error writing todo list:', e);
  }
}

function updateSyncStatus(element: HTMLElement, lastSyncTime: number) {
  const timeSinceSync = Date.now() - lastSyncTime;
  const secondsSinceSync = Math.floor(timeSinceSync / 1000);
  element.style.color = 'black';
  element.textContent = `Last synced: ${secondsSinceSync} seconds ago`;
  element.style.backgroundColor = secondsSinceSync < 5 ? '#e8f5e9' : '#f0f0f0';
}

async function createTodoDemo() {
  const container = document.createElement('div');
  container.setAttribute('data-todo-container', '');
  container.style.margin = '20px';
  container.style.padding = '20px';
  container.style.border = '1px solid #ccc';
  container.style.borderRadius = '8px';
  container.style.maxWidth = '600px';

  const syncStatus = document.createElement('div');
  syncStatus.setAttribute('data-sync-status', '');
  syncStatus.style.marginBottom = '10px';
  syncStatus.style.padding = '8px';
  syncStatus.style.borderRadius = '4px';
  syncStatus.style.backgroundColor = '#f0f0f0';
  syncStatus.style.fontSize = '14px';
  container.appendChild(syncStatus);

  const waitingMessage = document.createElement('div');
  waitingMessage.setAttribute('data-waiting-message', '');
  waitingMessage.style.textAlign = 'center';
  waitingMessage.style.padding = '20px';
  waitingMessage.style.color = '#666';
  waitingMessage.textContent = 'Connect to a peer to start collaborating on todos';
  container.appendChild(waitingMessage);

  const inputContainer = document.createElement('div');
  inputContainer.setAttribute('data-input-container', '');
  inputContainer.style.marginBottom = '20px';
  inputContainer.style.display = 'flex';
  inputContainer.style.gap = '10px';
  inputContainer.style.display = 'none';

  const input = document.createElement('input');
  input.type = 'text';
  input.placeholder = 'Enter a new todo item';
  input.style.flexGrow = '1';
  input.style.padding = '8px';
  input.style.borderRadius = '4px';
  input.style.border = '1px solid #ccc';
  inputContainer.appendChild(input);

  const addButton = document.createElement('button');
  addButton.textContent = 'Add Todo';
  addButton.style.padding = '8px 16px';
  addButton.style.backgroundColor = '#4CAF50';
  addButton.style.color = 'white';
  addButton.style.border = 'none';
  addButton.style.borderRadius = '4px';
  addButton.style.cursor = 'pointer';
  inputContainer.appendChild(addButton);

  container.appendChild(inputContainer);

  const list = document.createElement('div');
  list.setAttribute('data-list', '');
  list.style.marginTop = '10px';
  list.style.display = 'none';
  container.appendChild(list);

  async function renderTodoList() {
    const todos = await readTodoList();
    if (!todos) return;

    const list = document.querySelector('[data-list]');
    if (!list) return;
    list.innerHTML = '';

    todos.items.forEach((todo, index) => {
      const item = document.createElement('div');
      item.style.display = 'flex';
      item.style.alignItems = 'center';
      item.style.marginBottom = '10px';
      item.style.padding = '10px';
      item.style.backgroundColor = todo.completed ? '#e8f5e9' : '#fff';
      item.style.borderRadius = '5px';
      item.style.border = '1px solid #ccc';

      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.checked = todo.completed;
      checkbox.style.marginRight = '10px';
      checkbox.addEventListener('change', async () => {
        todos.items[index].completed = checkbox.checked;
        todos.lastSync = Date.now();
        await writeTodoList(todos);
        await renderTodoList();
      });

      const text = document.createElement('span');
      text.textContent = todo.text;
      text.style.flexGrow = '1';
      if (todo.completed) {
        text.style.textDecoration = 'line-through';
        text.style.color = '#666';
      }

      const deleteBtn = document.createElement('button');
      deleteBtn.textContent = '';
      deleteBtn.style.marginLeft = '10px';
      deleteBtn.style.border = 'none';
      deleteBtn.style.background = 'none';
      deleteBtn.style.cursor = 'pointer';
      deleteBtn.addEventListener('click', async () => {
        todos.items.splice(index, 1);
        todos.lastSync = Date.now();
        await writeTodoList(todos);
        await renderTodoList();
      });

      item.appendChild(checkbox);
      item.appendChild(text);
      item.appendChild(deleteBtn);
      list.appendChild(item);
    });
  };

  const todoAddButton = container.querySelector('button');
  const todoInput = container.querySelector('input');
  if (todoAddButton && todoInput) {
    todoAddButton.addEventListener('click', async () => {
      const text = todoInput.value.trim();
      if (!text) return;

      const todos = await readTodoList();
      todos.items.push({
        id: crypto.randomUUID(),
        text,
        completed: false,
        lastModified: Date.now()
      });
      todos.lastSync = Date.now();

      await writeTodoList(todos);
      await renderTodoList();

      todoInput.value = '';
    });

    todoInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        todoAddButton.click();
      }
    });
  }

  const syncInterval = window.setInterval(renderTodoList, READ_INTERVAL);

  const cleanup = () => {
    window.clearInterval(syncInterval);
  };

  (container as any)._cleanup = cleanup;
  (container as any)._renderTodoList = renderTodoList;

  document.body.appendChild(container);

  await renderTodoList();

  return container;
}

async function main() {
  try {
    await vault.createVault('default', PASSWORD, 'test', { foo: 'bar' });
    await vault.readFromVault('default', PASSWORD, 'test');
  } catch (e) {
    console.log(e);
  }
  try {
    const namespaces = await vault.listNamespaces('default', PASSWORD);
    console.log('Available namespaces:', namespaces);

    const vaults = await list_vaults();
    console.log('Available vaults:', vaults);

  } catch (error) {
    console.error('Error:', error);
  }
  await createTodoDemo();
}

main()

async function addSyncButtons() {
  const container = document.createElement('div');
  container.style.margin = '20px';

  const statusText = document.createElement('div');
  statusText.style.marginBottom = '10px';
  container.appendChild(statusText);

  let localPeerId: string | null = null;
  const myPeerId = document.createElement('div');
  myPeerId.style.marginBottom = '10px';
  container.appendChild(myPeerId);

  const getPeerIdButton = document.createElement('button');
  getPeerIdButton.textContent = 'Get my peer ID';
  getPeerIdButton.style.marginRight = '10px';
  container.appendChild(getPeerIdButton);

  const connectButton = document.createElement('button');
  connectButton.textContent = 'Connect';
  container.appendChild(connectButton);

  const peerIdInput = document.createElement('input');
  peerIdInput.type = 'text';
  peerIdInput.placeholder = 'Enter peer ID to connect to';
  peerIdInput.style.marginLeft = '10px';
  container.appendChild(peerIdInput);

  getPeerIdButton.addEventListener('click', async () => {
    try {
      statusText.textContent = 'Getting auth token...';

      const token = await getAuthToken();
      const wsUrl = `ws://localhost:8080/ws?token=${token}`;

      const randomVaultName = `peer_${Math.random().toString(36).substring(7)}`;
      try {
        const initialData = new TextEncoder().encode(JSON.stringify({ type: 'peer' }));
        await create_vault(randomVaultName, PASSWORD, 'sync', Array.from(initialData), undefined);
      } catch (e) {
        if (!e.toString().includes('Vault already exists')) {
          throw e;
        }
      }

      statusText.textContent = 'Enabling sync...';
      localPeerId = await enable_sync(randomVaultName, PASSWORD, wsUrl, STUN_SERVERS);
      statusText.textContent = 'Sync enabled, ready to connect';

      myPeerId.textContent = `My Peer ID: ${localPeerId}`;
    } catch (e) {
      console.error('Failed to enable sync:', e);
      statusText.textContent = `Failed to enable sync: ${e}`;
    }
  });

  connectButton.addEventListener('click', async () => {
    const targetPeerId = peerIdInput.value.trim();
    if (!targetPeerId) {
      statusText.textContent = 'Please enter a peer ID';
      return;
    }
    
    try {
      statusText.textContent = 'Getting auth token...';

      const token = await getAuthToken();
      const wsUrl = `ws://localhost:8080/ws?token=${token}`;

      try {
        const initialTodos = { items: [], lastSync: Date.now() };
        const todoData = new TextEncoder().encode(JSON.stringify(initialTodos));
        await create_vault('todos', PASSWORD, 'todo_list', Array.from(todoData), undefined);
      } catch (e) {
        if (!e.toString().includes('Vault already exists')) {
          throw e;
        }
      }

      statusText.textContent = 'Enabling sync...';
      await enable_sync('todos', PASSWORD, wsUrl, STUN_SERVERS);

      statusText.textContent = 'Connecting...';
      await connect_to_peer('todos', PASSWORD, targetPeerId, wsUrl);

      await add_peer('todos', PASSWORD, targetPeerId, 'todo_list', 'contributor');

      statusText.textContent = `Connected to peer ${targetPeerId}`;

      const todos = await readTodoList();
      todos.lastSync = Date.now();
      await writeTodoList(todos);

      const todoContainer = document.querySelector('[data-todo-container]');
      if (todoContainer) {
        const renderTodoList = (todoContainer as any)._renderTodoList;
        if (renderTodoList) {
          await renderTodoList();
        }
      }
    } catch (e) {
      console.error('Failed to connect to peer:', e);
      statusText.textContent = `Failed to connect: ${e}`;
    }
  });

  document.body.appendChild(container);
}

async function getAuthToken(): Promise<string> {
  const response = await fetch(`${BASE_URL}/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' }
  });

  if (!response.ok) {
    throw new Error(`Failed to get auth token: ${response.status}`);
  }

  const { token } = await response.json();
  return token;
}

addSyncButtons();
