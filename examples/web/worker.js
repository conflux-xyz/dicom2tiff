// Web Worker for handling DICOM conversion
import init, { convertViaSyncAccessHandles } from '../../crates/wasm/pkg/dicom2tiff.js';

let wasmInitialized = false;

// Initialize the WASM module
async function initWasm() {
  try {
    await init();
    wasmInitialized = true;
    self.postMessage({ type: 'init', success: true });
  } catch (error) {
    console.error('Failed to initialize WASM module:', error);
    self.postMessage({ type: 'init', success: false, error: error.message });
  }
}

// Initialize WASM immediately
initWasm();

// Listen for messages from the main thread
self.addEventListener('message', async (event) => {
  const { type, fileId } = event.data;

  if (!wasmInitialized) {
    self.postMessage({ type: 'error', error: 'WASM module not initialized', fileId });
    return;
  }

  if (type === 'convertViaOpfs') {
    console.log("Converting via OPFS...");
    const { inputFileHandles, outputFileHandle } = event.data;

    if (!inputFileHandles) {
      self.postMessage({ type: 'error', error: 'No input file handles provided', fileId });
      return;
    }

    if (!outputFileHandle) {
      self.postMessage({ type: 'error', error: 'No output file handle provided', fileId });
      return;
    }

    try {
      self.postMessage({ type: 'status', status: 'Converting using OPFS API...', fileId });

      const inputSyncAccessHandles = await Promise.all(inputFileHandles.map(handle => handle.createSyncAccessHandle()));
      const outputSyncAccessHandle = await outputFileHandle.createSyncAccessHandle();

      // Convert using the OPFS API
      convertViaSyncAccessHandles(inputSyncAccessHandles, outputSyncAccessHandle);

      // Signal completion - files are already in the output directory
      self.postMessage({ 
        type: 'convertViaOpfsComplete',
        fileId,
      });

    } catch (error) {
      console.error('Conversion with Full OPFS API failed:', error);
      self.postMessage({ type: 'error', error: error.message || 'Conversion failed', fileId });
    }
  }
});