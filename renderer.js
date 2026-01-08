const { ipcRenderer } = require('electron');

// Elements
const sections = {
    select: document.getElementById('section-select'),
    scanning: document.getElementById('section-scanning'),
    results: document.getElementById('section-results')
};

const btnSelect = document.getElementById('btn-select');
const btnClean = document.getElementById('btn-clean');
const btnRescan = document.getElementById('btn-rescan');
const selectedPathDisplay = document.getElementById('selected-path-display');
const scanStatusText = document.getElementById('scan-status-text');
const resultsBody = document.getElementById('results-body');

const statCount = document.getElementById('stat-count');
const statSize = document.getElementById('stat-size');

let currentResults = [];

// Navigation
function showSection(name) {
    Object.values(sections).forEach(el => {
        el.classList.add('hidden');
        el.classList.remove('active');
    });
    sections[name].classList.remove('hidden');
    sections[name].classList.add('active');
}

// 1. Select Dir
btnSelect.addEventListener('click', async () => {
    // Get enabled filters
    const targets = [];
    if(document.getElementById('chk-node_modules').checked) targets.push('node_modules');
    if(document.getElementById('chk-dist').checked) targets.push('dist');
    if(document.getElementById('chk-bin').checked) targets.push('bin');
    if(document.getElementById('chk-obj').checked) targets.push('obj');

    if (targets.length === 0) {
        alert("Please select at least one target to clean.");
        return;
    }

    const path = await ipcRenderer.invoke('select-dir');
    if (path) {
        selectedPathDisplay.textContent = path;
        startScan(path, targets);
    }
});

// 2. Scan Logic
async function startScan(path, targets) {
    showSection('scanning');
    scanStatusText.textContent = `Scanning ${path}...`;
    
    // Reset
    resultsBody.innerHTML = '';
    
    // Call Main
    const data = await ipcRenderer.invoke('scan-dir', path, targets);
    
    // Render Results
    currentResults = data.results;
    updateStats(data.results.length, data.totalSizeFormatted);
    renderTable(data.results);
    
    showSection('results');
}

// 3. Render
function updateStats(count, sizeFormatted) {
    statCount.textContent = count;
    statSize.textContent = sizeFormatted;
    
    // Disable clean button if empty
    if (count === 0) {
        btnClean.disabled = true;
        btnClean.style.opacity = 0.5;
        btnClean.style.cursor = 'not-allowed';
    } else {
        btnClean.disabled = false;
        btnClean.style.opacity = 1;
        btnClean.style.cursor = 'pointer';
    }
}

function renderTable(items) {
    let html = '';
    items.forEach(item => {
        let icon;
        if (item.type === 'node_modules') {
            icon = '<i class="fa-brands fa-node-js" style="color:#6cc24a"></i>';
        } else if (item.type === 'bin' || item.type === 'obj') {
             icon = '<i class="fa-brands fa-microsoft" style="color:#0078d4"></i>';
        } else {
            icon = '<i class="fa-solid fa-box-archive" style="color:#f9c859"></i>';
        }
            
        html += `
            <tr>
                <td width="60" style="text-align:center">${icon}</td>
                <td style="color:#a6adc8; font-family:monospace; font-size:0.8rem">${item.path}</td>
                <td width="100" style="text-align:right">${item.sizeFormatted}</td>
            </tr>
        `;
    });
    resultsBody.innerHTML = html;
}

// 4. Clean Logic
btnClean.addEventListener('click', async () => {
    if (!currentResults.length) return;
    
    const confirm = window.confirm(`Are you sure you want to delete ${currentResults.length} folders?`);
    if (confirm) {
        btnClean.innerHTML = '<i class="fa-solid fa-spinner fa-spin"></i> Cleaning...';
        btnClean.disabled = true;
        
        await ipcRenderer.invoke('delete-all', currentResults);
        
        alert("Cleanup Complete!");
        
        // Reset state
        showSection('select');
        btnClean.innerHTML = '<i class="fa-solid fa-trash-can"></i> Clean All';
    }
});

// Rescan (Back)
btnRescan.addEventListener('click', () => {
    showSection('select');
});

// Listeners for progress
ipcRenderer.on('scan-progress', (event, data) => {
    scanStatusText.textContent = `Found ${data.found} items...`;
});
