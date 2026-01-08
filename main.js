const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');

let mainWindow;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 900,
    height: 700,
    minWidth: 800,
    minHeight: 600,
    titleBarStyle: 'hiddenInset', // Native looking macos title bar
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false // For simple prototype. In prod use preload.
    },
    backgroundColor: '#1e1e2e'
  });

  mainWindow.loadFile('index.html');
}

app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

// IPC Handlers

// 1. Select Directory
ipcMain.handle('select-dir', async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openDirectory']
  });
  if (result.canceled) return null;
  return result.filePaths[0];
});

// Helper for formatted size
function formatBytes(bytes, decimals = 2) {
  if (!+bytes) return '0 Bytes';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

// 2. Scan Directory
ipcMain.handle('scan-dir', async (event, rootPath, targetList) => {
  // Use passed targets, or fallback to default if empty/undefined (safety)
  const targetNames = new Set(targetList && targetList.length ? targetList : []);
  const results = [];
  let totalSize = 0;

  async function getDirSize(dirPath) {
    let size = 0;
    try {
      const files = await fs.promises.readdir(dirPath, { withFileTypes: true });
      for (const file of files) {
        const filePath = path.join(dirPath, file.name);
        if (file.isDirectory()) {
          size += await getDirSize(filePath);
        } else {
          try {
            const stats = await fs.promises.stat(filePath);
            size += stats.size;
          } catch (e) {}
        }
      }
    } catch (e) {}
    return size;
  }

  async function scan(currentPath) {
    try {
      const entries = await fs.promises.readdir(currentPath, { withFileTypes: true });
      const subDirs = [];

      for (const entry of entries) {
        if (entry.isDirectory()) {
            const fullPath = path.join(currentPath, entry.name);
            
            if (targetNames.has(entry.name)) {
                // Found a target! Calc size and don't recurse inside
                const size = await getDirSize(fullPath);
                totalSize += size;
                results.push({
                    path: fullPath,
                    type: entry.name,
                    sizeBytes: size,
                    sizeFormatted: formatBytes(size)
                });
                // Send incremental update to UI if list is huge? 
                // For now, let's keep it simple and return all at end, 
                // or emitting events could make it feel snappier.
                mainWindow.webContents.send('scan-progress', { found: results.length });
            } else {
                subDirs.push(fullPath);
            }
        }
      }

      // Recurse into non-target dirs
      for (const subDir of subDirs) {
        await scan(subDir);
      }

    } catch (error) {
      console.error(`Error scanning ${currentPath}:`, error);
    }
  }

  await scan(rootPath);
  return { results, totalSize, totalSizeFormatted: formatBytes(totalSize) };
});

// 3. Delete Items
ipcMain.handle('delete-all', async (event, items) => {
    let deletedCount = 0;
    for (const item of items) {
        try {
            await fs.promises.rm(item.path, { recursive: true, force: true });
            deletedCount++;
            mainWindow.webContents.send('delete-progress', { 
                current: deletedCount, 
                total: items.length 
            });
        } catch (e) {
            console.error(e);
        }
    }
    return true;
});
