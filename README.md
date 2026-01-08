# Node Modules Cleaner (Electron Edition)

A beautiful, native desktop application designed to reclaim disk space by deleting `node_modules` and `dist` directories. Built with Electron for a premium, cross-platform experience.

![Node.js](https://img.shields.io/badge/node->=16.0-green.svg)
![Electron](https://img.shields.io/badge/electron-28.x-blue.svg)
![License](https://img.shields.io/badge/license-MIT-purple.svg)

## Features

- **Native Experience**: Runs as a standalone desktop app on macOS, Windows, and Linux.
- **Premium UI**: Features a modern "Glassmorphism" dark theme with smooth animations.
- **Deep Scanning**: Recursively finds `node_modules`, `dist`, `bin`, and `obj` folders.
- **Custom Filters**: Toggle which folder types to scan and delete.
- **Smart & Safe**:
  - Calculates exact folder sizes.
  - Requires explicit confirmation before deletion.
  - Shows a detailed summary of reclaimable space.

## Prerequisites

- [Node.js](https://nodejs.org/) (v16 or higher)
- npm (comes with Node.js)

## Installation

1.  Clone the repository:

    ```bash
    git clone https://github.com/yourusername/node-modules-cleaner.git
    cd node-modules-cleaner
    ```

2.  Install dependencies:
    ```bash
    npm install
    ```

## Usage

Start the application with:

```bash
npm start
```

### workflow

1.  **Select Workspace**: Click "Browse System" to choose the root folder you want to clean (e.g., your `projects` directory).
2.  **Scan**: The app will automatically scan for targets. Watch the animated progress indicator.
3.  **Review**: See exactly which folders were found and how large they are.
4.  **Clean**: Click "Clean All" to delete the folders and free up space.

## Architecture

- **Main Process** (`main.js`): Handles system scanning and file deletion using Node's `fs` module to ensure performance and avoid browser sandbox limitations.
- **Renderer** (`renderer.js`): Manages the reactive UI and communicates with the main process via IPC.
- **Styling** (`style.css`): Custom CSS variable system with flexbox/grid layouts.

## License

MIT
