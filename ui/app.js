// MatePro - Tauri Frontend Application

const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI_PLUGIN_OPENER__;

// ============ STATE ============

const state = {
    selectedModel: null,
    models: [],
    conversation: [],
    attachedFiles: [],
    agentMode: false,
    currentIteration: 0,
    maxIterations: 5,
    systemPromptAdded: false,
    pendingToolCalls: [],
    isProcessing: false,
};

// ============ DOM ELEMENTS ============

const elements = {
    // Screens
    setupScreen: document.getElementById('setup-screen'),
    loadingScreen: document.getElementById('loading-screen'),
    chatScreen: document.getElementById('chat-screen'),
    
    // Setup
    scanningIndicator: document.getElementById('scanning-indicator'),
    serverList: document.getElementById('server-list'),
    servers: document.getElementById('servers'),
    serverUrl: document.getElementById('server-url'),
    connectBtn: document.getElementById('connect-btn'),
    rescanBtn: document.getElementById('rescan-btn'),
    setupError: document.getElementById('setup-error'),
    loadingText: document.getElementById('loading-text'),
    
    // Chat
    modelSelector: document.getElementById('model-selector'),
    agentModeToggle: document.getElementById('agent-mode-toggle'),
    iterationCounter: document.getElementById('iteration-counter'),
    sqlConfigBtn: document.getElementById('sql-config-btn'),
    newChatBtn: document.getElementById('new-chat-btn'),
    disconnectBtn: document.getElementById('disconnect-btn'),
    messages: document.getElementById('messages'),
    errorBanner: document.getElementById('error-banner'),
    errorText: document.getElementById('error-text'),
    closeError: document.getElementById('close-error'),
    attachedFilesContainer: document.getElementById('attached-files'),
    messageInput: document.getElementById('message-input'),
    attachBtn: document.getElementById('attach-btn'),
    sendBtn: document.getElementById('send-btn'),
    fileInput: document.getElementById('file-input'),
    
    // SQL Modal
    sqlModal: document.getElementById('sql-modal'),
    closeSqlModal: document.getElementById('close-sql-modal'),
    closeSqlBtn: document.getElementById('close-sql-btn'),
    sqlServer: document.getElementById('sql-server'),
    sqlDatabase: document.getElementById('sql-database'),
    sqlCredentials: document.getElementById('sql-credentials'),
    sqlUsername: document.getElementById('sql-username'),
    sqlPassword: document.getElementById('sql-password'),
    sqlStatus: document.getElementById('sql-status'),
    testSqlBtn: document.getElementById('test-sql-btn'),
    
    // Confirm Modal
    confirmModal: document.getElementById('confirm-modal'),
    confirmDetails: document.getElementById('confirm-details'),
    confirmAllow: document.getElementById('confirm-allow'),
    confirmCancel: document.getElementById('confirm-cancel'),
};

// ============ UTILITIES ============

function showScreen(screenId) {
    ['setup-screen', 'loading-screen', 'chat-screen'].forEach(id => {
        document.getElementById(id).classList.add('hidden');
    });
    document.getElementById(screenId).classList.remove('hidden');
}

function showError(message) {
    if (elements.setupError && elements.setupScreen.classList.contains('hidden') === false) {
        elements.setupError.textContent = message;
        elements.setupError.classList.remove('hidden');
    } else {
        elements.errorText.textContent = message;
        elements.errorBanner.classList.remove('hidden');
    }
}

function hideError() {
    elements.setupError?.classList.add('hidden');
    elements.errorBanner.classList.add('hidden');
}

function getTimestamp() {
    const now = new Date();
    return now.toLocaleTimeString('it-IT', { hour: '2-digit', minute: '2-digit' });
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function formatMessage(content) {
    // Basic markdown-like formatting
    let formatted = escapeHtml(content);
    
    // Code blocks
    formatted = formatted.replace(/```(\w+)?\n([\s\S]*?)```/g, '<pre><code>$2</code></pre>');
    formatted = formatted.replace(/`([^`]+)`/g, '<code>$1</code>');
    
    // Bold and italic
    formatted = formatted.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    formatted = formatted.replace(/\*([^*]+)\*/g, '<em>$1</em>');
    
    // Line breaks
    formatted = formatted.replace(/\n/g, '<br>');
    
    return formatted;
}

function scrollToBottom() {
    elements.messages.scrollTop = elements.messages.scrollHeight;
}

// ============ NETWORK SCAN ============

async function scanNetwork() {
    elements.scanningIndicator.classList.remove('hidden');
    elements.serverList.classList.add('hidden');
    
    try {
        const servers = await invoke('scan_network');
        
        if (servers.length > 0) {
            elements.servers.innerHTML = '';
            servers.forEach(server => {
                const isLocal = server.includes('localhost') || server.includes('127.0.0.1');
                const option = document.createElement('div');
                option.className = 'server-option';
                option.textContent = `${isLocal ? '🏠' : '🌐'} ${server}`;
                option.dataset.url = server;
                
                if (server === elements.serverUrl.value) {
                    option.classList.add('selected');
                }
                
                option.addEventListener('click', () => {
                    document.querySelectorAll('.server-option').forEach(el => el.classList.remove('selected'));
                    option.classList.add('selected');
                    elements.serverUrl.value = server;
                });
                
                elements.servers.appendChild(option);
            });
            
            elements.serverList.classList.remove('hidden');
            elements.serverUrl.value = servers[0];
        }
    } catch (error) {
        console.error('Scan error:', error);
    }
    
    elements.scanningIndicator.classList.add('hidden');
}

// ============ CONNECTION ============

async function connect() {
    const url = elements.serverUrl.value.trim();
    if (!url) {
        showError('Inserisci un URL valido');
        return;
    }
    
    hideError();
    elements.connectBtn.disabled = true;
    showScreen('loading-screen');
    elements.loadingText.textContent = 'Connessione al server...';
    
    try {
        await invoke('connect_to_server', { url });
        await loadModels();
    } catch (error) {
        showScreen('setup-screen');
        showError(error);
        elements.connectBtn.disabled = false;
    }
}

async function loadModels() {
    elements.loadingText.textContent = 'Caricamento modelli...';
    
    try {
        const models = await invoke('list_models');
        
        if (models.length === 0) {
            showScreen('setup-screen');
            showError('Nessun modello disponibile. Scarica un modello con "ollama pull <model>"');
            elements.connectBtn.disabled = false;
            return;
        }
        
        state.models = models;
        elements.modelSelector.innerHTML = '';
        
        models.forEach(model => {
            const option = document.createElement('option');
            option.value = model.name;
            const indicator = model.category === 'light' ? '🟢' : model.category === 'medium' ? '🟡' : '🔴';
            option.textContent = `${indicator} ${model.name} (${model.size_gb.toFixed(1)} GB)`;
            elements.modelSelector.appendChild(option);
        });
        
        state.selectedModel = models[0].name;
        showScreen('chat-screen');
        elements.connectBtn.disabled = false;
        
    } catch (error) {
        showScreen('setup-screen');
        showError(`Errore caricamento modelli: ${error}`);
        elements.connectBtn.disabled = false;
    }
}

// ============ CHAT ============

function addMessage(role, content, timestamp = null) {
    const emptyState = elements.messages.querySelector('.empty-state');
    if (emptyState) {
        emptyState.remove();
    }
    
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${role}`;
    
    const bubble = document.createElement('div');
    bubble.className = 'message-bubble';
    bubble.innerHTML = role === 'user' ? escapeHtml(content) : formatMessage(content);
    messageDiv.appendChild(bubble);
    
    if (timestamp) {
        const timeSpan = document.createElement('span');
        timeSpan.className = 'message-timestamp';
        timeSpan.textContent = timestamp;
        messageDiv.appendChild(timeSpan);
    }
    
    elements.messages.appendChild(messageDiv);
    scrollToBottom();
}

function addLoadingIndicator() {
    const indicator = document.createElement('div');
    indicator.className = 'loading-indicator';
    indicator.id = 'chat-loading';
    indicator.innerHTML = `
        <div class="spinner"></div>
        <span>Sto pensando...</span>
    `;
    elements.messages.appendChild(indicator);
    scrollToBottom();
}

function removeLoadingIndicator() {
    const indicator = document.getElementById('chat-loading');
    if (indicator) {
        indicator.remove();
    }
}

async function getToolsDescription() {
    if (state.agentMode) {
        return await invoke('get_tools_description');
    }
    return '';
}

async function sendMessage() {
    const text = elements.messageInput.value.trim();
    if (!text && state.attachedFiles.length === 0) return;
    if (state.isProcessing) return;
    
    state.isProcessing = true;
    state.currentIteration = 0;
    updateIterationCounter();
    hideError();
    
    // Build message content
    let fullContent = '';
    
    if (state.attachedFiles.length > 0) {
        fullContent += 'File allegati:\n\n';
        state.attachedFiles.forEach(file => {
            fullContent += `=== ${file.name} ===\n${file.content}\n\n`;
        });
        fullContent += '---\n\n';
    }
    
    fullContent += text;
    
    // Display message
    const displayContent = state.attachedFiles.length > 0
        ? state.attachedFiles.map(f => `📎 ${f.name}`).join('\n') + '\n\n' + text
        : text;
    
    addMessage('user', displayContent, getTimestamp());
    
    // Add to conversation
    if (!state.systemPromptAdded && state.conversation.length === 0) {
        let systemContent = `IMPORTANTE: Per questa conversazione, quando devi mostrare formule matematiche NON usare LaTeX. Usa SOLO:
• Caratteri Unicode: √ ² ³ ∫ ∑ π ∞ ≤ ≥ ≠ ± × ÷
• Notazione testuale: sqrt(), ^2, ^3, /`;
        
        if (state.agentMode) {
            const toolsDesc = await getToolsDescription();
            systemContent += '\n\n' + toolsDesc;
            systemContent += '\n\n**LINEE GUIDA:** Usa i tool appropriati per le richieste dell\'utente.';
        }
        
        state.conversation.push({ role: 'user', content: systemContent, hidden: true });
        state.conversation.push({ 
            role: 'assistant', 
            content: 'Perfetto! Sono pronto ad aiutarti.',
            hidden: true 
        });
        state.systemPromptAdded = true;
    }
    
    state.conversation.push({ role: 'user', content: fullContent, hidden: false });
    
    // Clear input
    elements.messageInput.value = '';
    state.attachedFiles = [];
    updateAttachedFiles();
    updateSendButton();
    
    // Send to backend
    await processChat();
}

async function processChat() {
    addLoadingIndicator();
    
    try {
        const response = await invoke('chat', {
            model: state.selectedModel,
            messages: state.conversation
        });
        
        removeLoadingIndicator();
        
        state.conversation.push({
            role: 'assistant',
            content: response.content,
            hidden: false
        });
        
        addMessage('assistant', response.content, response.timestamp);
        
        // Check for tool calls if agent mode is enabled
        if (state.agentMode) {
            const toolCalls = await invoke('parse_tool_calls', { response: response.content });
            
            if (toolCalls.length > 0) {
                state.pendingToolCalls = toolCalls;
                await processNextToolCall();
            }
        }
        
    } catch (error) {
        removeLoadingIndicator();
        showError(`Errore: ${error}`);
        state.conversation.pop(); // Remove user message
    }
    
    state.isProcessing = false;
}

async function processNextToolCall() {
    if (state.pendingToolCalls.length === 0) return;
    
    const toolCall = state.pendingToolCalls[0];
    
    // Check if tool is dangerous
    const isDangerous = await invoke('check_tool_dangerous', { toolName: toolCall.tool_name });
    
    if (isDangerous) {
        showConfirmModal(toolCall);
        return;
    }
    
    await executeToolCall(toolCall);
}

async function executeToolCall(toolCall) {
    state.pendingToolCalls.shift();
    
    try {
        const result = await invoke('execute_tool', { toolCall });
        
        // Show result to user
        addMessage('system', `🔧 ${result.tool_name}: ${result.success ? '✅' : '❌'}\n${result.output || result.error || ''}`, getTimestamp());
        
        // Add to conversation for context
        state.conversation.push({
            role: 'user',
            content: `**Risultato Tool:** ${result.tool_name}\n${result.output || result.error}`,
            hidden: true
        });
        
        // Handle URL results (open in browser)
        if (result.success && result.output.startsWith('URL: ')) {
            const url = result.output.replace('URL: ', '');
            try {
                await open(url);
            } catch (e) {
                console.error('Failed to open URL:', e);
            }
        }
        
        state.currentIteration++;
        updateIterationCounter();
        
        // Continue agent loop if more tool calls or iterations available
        if (state.pendingToolCalls.length > 0) {
            await processNextToolCall();
        } else if (state.currentIteration < state.maxIterations) {
            // Let the model continue
            await continueAgentLoop();
        } else {
            showError('Raggiunto limite massimo di iterazioni agentiche');
        }
        
    } catch (error) {
        showError(`Errore esecuzione tool: ${error}`);
    }
}

async function continueAgentLoop() {
    addLoadingIndicator();
    
    try {
        const response = await invoke('chat', {
            model: state.selectedModel,
            messages: state.conversation
        });
        
        removeLoadingIndicator();
        
        state.conversation.push({
            role: 'assistant',
            content: response.content,
            hidden: false
        });
        
        addMessage('assistant', response.content, response.timestamp);
        
        // Check for more tool calls
        const toolCalls = await invoke('parse_tool_calls', { response: response.content });
        
        if (toolCalls.length > 0) {
            state.pendingToolCalls = toolCalls;
            await processNextToolCall();
        }
        
    } catch (error) {
        removeLoadingIndicator();
        showError(`Errore: ${error}`);
    }
}

function showConfirmModal(toolCall) {
    elements.confirmDetails.innerHTML = `
        <strong>Tool:</strong> ${toolCall.tool_name}<br>
        <strong>Parametri:</strong><br>
        ${Object.entries(toolCall.parameters).map(([k, v]) => `  ${k}: ${JSON.stringify(v)}`).join('<br>')}
    `;
    elements.confirmModal.classList.remove('hidden');
    
    elements.confirmAllow.onclick = async () => {
        elements.confirmModal.classList.add('hidden');
        await invoke('set_allow_dangerous', { allow: true });
        await executeToolCall(toolCall);
        await invoke('set_allow_dangerous', { allow: false });
    };
    
    elements.confirmCancel.onclick = () => {
        elements.confirmModal.classList.add('hidden');
        state.pendingToolCalls = [];
        addMessage('system', '❌ Operazione annullata dall\'utente', getTimestamp());
    };
}

// ============ FILE HANDLING ============

async function attachFile() {
    elements.fileInput.click();
}

async function handleFileSelect(event) {
    const file = event.target.files[0];
    if (!file) return;
    
    try {
        const [filename, content] = await invoke('read_file', { path: file.path || file.name });
        state.attachedFiles.push({ name: filename, content });
        updateAttachedFiles();
        updateSendButton();
    } catch (error) {
        showError(`Errore lettura file: ${error}`);
    }
    
    event.target.value = '';
}

function updateAttachedFiles() {
    if (state.attachedFiles.length === 0) {
        elements.attachedFilesContainer.classList.add('hidden');
        return;
    }
    
    elements.attachedFilesContainer.classList.remove('hidden');
    elements.attachedFilesContainer.innerHTML = state.attachedFiles.map((file, index) => `
        <div class="file-chip">
            <span>📎 ${escapeHtml(file.name)}</span>
            <button class="remove-btn" data-index="${index}">✕</button>
        </div>
    `).join('');
    
    elements.attachedFilesContainer.querySelectorAll('.remove-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            state.attachedFiles.splice(parseInt(btn.dataset.index), 1);
            updateAttachedFiles();
            updateSendButton();
        });
    });
}

function updateSendButton() {
    const hasContent = elements.messageInput.value.trim() || state.attachedFiles.length > 0;
    elements.sendBtn.disabled = !hasContent || state.isProcessing;
}

function updateIterationCounter() {
    if (state.agentMode) {
        elements.iterationCounter.textContent = `(${state.currentIteration}/${state.maxIterations})`;
        elements.iterationCounter.classList.remove('hidden');
    } else {
        elements.iterationCounter.classList.add('hidden');
    }
}

// ============ SQL CONFIGURATION ============

function showSqlModal() {
    elements.sqlModal.classList.remove('hidden');
}

function hideSqlModal() {
    elements.sqlModal.classList.add('hidden');
}

function updateSqlAuth() {
    const authMethod = document.querySelector('input[name="sql-auth"]:checked').value;
    if (authMethod === 'sql') {
        elements.sqlCredentials.classList.remove('hidden');
    } else {
        elements.sqlCredentials.classList.add('hidden');
    }
}

async function testSqlConnection() {
    const server = elements.sqlServer.value.trim();
    const database = elements.sqlDatabase.value.trim();
    const authMethod = document.querySelector('input[name="sql-auth"]:checked').value;
    
    if (!server || !database) {
        elements.sqlStatus.className = 'sql-status error';
        elements.sqlStatus.textContent = 'Server e Database sono obbligatori';
        elements.sqlStatus.classList.remove('hidden');
        return;
    }
    
    elements.sqlStatus.className = 'sql-status connecting';
    elements.sqlStatus.textContent = '⟳ Connessione in corso...';
    elements.sqlStatus.classList.remove('hidden');
    elements.testSqlBtn.disabled = true;
    
    try {
        const connectionId = await invoke('sql_connect', {
            server,
            database,
            authMethod,
            username: authMethod === 'sql' ? elements.sqlUsername.value : null,
            password: authMethod === 'sql' ? elements.sqlPassword.value : null,
        });
        
        elements.sqlStatus.className = 'sql-status connected';
        elements.sqlStatus.textContent = `✓ Connesso! ID: ${connectionId}`;
        elements.sqlConfigBtn.textContent = '🗄️ SQL (✓)';
        
    } catch (error) {
        elements.sqlStatus.className = 'sql-status error';
        elements.sqlStatus.textContent = `✕ Errore: ${error}`;
    }
    
    elements.testSqlBtn.disabled = false;
}

// ============ NEW CHAT / DISCONNECT ============

function newChat() {
    state.conversation = [];
    state.attachedFiles = [];
    state.systemPromptAdded = false;
    state.currentIteration = 0;
    state.pendingToolCalls = [];
    
    elements.messages.innerHTML = `
        <div class="empty-state">
            <p class="empty-title">Inizia una conversazione</p>
            <p class="empty-subtitle">Scrivi un messaggio per iniziare</p>
        </div>
    `;
    
    updateAttachedFiles();
    updateIterationCounter();
    hideError();
}

function disconnect() {
    state.conversation = [];
    state.models = [];
    state.selectedModel = null;
    state.attachedFiles = [];
    state.systemPromptAdded = false;
    state.currentIteration = 0;
    
    showScreen('setup-screen');
    elements.setupError.classList.add('hidden');
}

// ============ EVENT LISTENERS ============

function initEventListeners() {
    // Setup
    elements.connectBtn.addEventListener('click', connect);
    elements.rescanBtn.addEventListener('click', scanNetwork);
    elements.serverUrl.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') connect();
    });
    
    // Chat
    elements.modelSelector.addEventListener('change', (e) => {
        state.selectedModel = e.target.value;
    });
    
    elements.agentModeToggle.addEventListener('change', (e) => {
        state.agentMode = e.target.checked;
        updateIterationCounter();
    });
    
    elements.sendBtn.addEventListener('click', sendMessage);
    elements.attachBtn.addEventListener('click', attachFile);
    elements.fileInput.addEventListener('change', handleFileSelect);
    
    elements.messageInput.addEventListener('input', updateSendButton);
    elements.messageInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            sendMessage();
        }
    });
    
    elements.newChatBtn.addEventListener('click', newChat);
    elements.disconnectBtn.addEventListener('click', disconnect);
    elements.closeError.addEventListener('click', hideError);
    
    // SQL Modal
    elements.sqlConfigBtn.addEventListener('click', showSqlModal);
    elements.closeSqlModal.addEventListener('click', hideSqlModal);
    elements.closeSqlBtn.addEventListener('click', hideSqlModal);
    elements.testSqlBtn.addEventListener('click', testSqlConnection);
    
    document.querySelectorAll('input[name="sql-auth"]').forEach(radio => {
        radio.addEventListener('change', updateSqlAuth);
    });
    
    // Close modals on outside click
    elements.sqlModal.addEventListener('click', (e) => {
        if (e.target === elements.sqlModal) hideSqlModal();
    });
    
    elements.confirmModal.addEventListener('click', (e) => {
        if (e.target === elements.confirmModal) {
            elements.confirmModal.classList.add('hidden');
            state.pendingToolCalls = [];
        }
    });
}

// ============ INITIALIZATION ============

async function init() {
    initEventListeners();
    await scanNetwork();
}

// Start the app
document.addEventListener('DOMContentLoaded', init);
