// MatePro - Tauri Frontend Application

const { invoke } = window.__TAURI__.core;
const { open: openExternal } = window.__TAURI_PLUGIN_OPENER__;
const { appWindow } = window.__TAURI__.window;

// ============ STATE ============

const state = {
    selectedModel: null,
    models: [],
    conversation: [],
    messageHistory: [],
    messageHistoryIndex: -1,
    attachedFiles: [],
    agentMode: false,
    currentIteration: 0,
    maxIterations: 5,
    systemPromptAdded: false,
    pendingToolCalls: [],
    isProcessing: false,
    greetingMessage: null,
    greetingShown: false,
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
    greetingBanner: document.getElementById('greeting-banner'),
    
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
    sqlTrustCert: document.getElementById('sql-trust-cert'),
    sqlStatus: document.getElementById('sql-status'),
    testSqlBtn: document.getElementById('test-sql-btn'),
    
    // Confirm Modal
    confirmModal: document.getElementById('confirm-modal'),
    confirmDetails: document.getElementById('confirm-details'),
    confirmAllow: document.getElementById('confirm-allow'),
    confirmCancel: document.getElementById('confirm-cancel'),
    versionIndicator: document.getElementById('version-indicator'),
};

const greetingTemplates = {
    en: (name) => `Hello ${name}! Welcome to MatePro.`,
    it: (name) => `Ciao ${name}! Benvenuto su MatePro.`,
    es: (name) => `Hola ${name}! Bienvenido a MatePro.`,
    fr: (name) => `Bonjour ${name} ! Bienvenue sur MatePro.`,
    de: (name) => `Hallo ${name}! Willkommen bei MatePro.`,
    pt: (name) => `Olá ${name}! Bem-vindo ao MatePro.`,
    nl: (name) => `Hallo ${name}! Welkom bij MatePro.`,
    sv: (name) => `Hej ${name}! Välkommen till MatePro.`,
    da: (name) => `Hej ${name}! Velkommen til MatePro.`,
    fi: (name) => `Hei ${name}! Tervetuloa MateProon.`,
    pl: (name) => `Cześć ${name}! Witamy w MatePro.`,
    tr: (name) => `Merhaba ${name}! MatePro'ya hoş geldin.`,
    ro: (name) => `Salut ${name}! Bine ai venit la MatePro.`,
    cs: (name) => `Ahoj ${name}! Vítej v MatePro.`,
    sk: (name) => `Ahoj ${name}! Vitaj v MatePro.`,
    hu: (name) => `Szia ${name}! Üdv a MateProban.`,
    el: (name) => `Γεια σου ${name}! Καλώς ήρθες στο MatePro.`,
    ru: (name) => `Здравствуйте, ${name}! Добро пожаловать в MatePro.`,
    uk: (name) => `Привіт ${name}! Ласкаво просимо до MatePro.`,
    zh: (name) => `你好，${name}！欢迎使用 MatePro。`,
    ja: (name) => `こんにちは、${name}さん！MateProへようこそ。`,
    ko: (name) => `안녕하세요, ${name}님! MatePro에 오신 것을 환영합니다.`,
};

function normalizeLanguageCode(tag) {
    if (!tag || typeof tag !== 'string') {
        return null;
    }

    const normalized = tag.trim().toLowerCase();
    if (!normalized) {
        return null;
    }

    const parts = normalized.split(/[-_]/);
    return parts[0] || null;
}

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
    const locale = (typeof navigator !== 'undefined' && navigator.language) ? navigator.language : 'it-IT';
    return now.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
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

async function loadVersionIndicator() {
    if (!elements.versionIndicator) return;

    try {
        const version = await invoke('get_app_version');
        if (!version) return;

        elements.versionIndicator.textContent = `v${version}`;
        elements.versionIndicator.classList.remove('hidden');
    } catch (error) {
        console.warn('Impossibile recuperare la versione dell\'applicazione:', error);
    }
}

async function loadGreeting() {
    if (!elements.greetingBanner) {
        return;
    }

    try {
        const profile = await invoke('get_user_profile');
        const candidates = [];

        if (profile?.primary_language) {
            candidates.push(profile.primary_language);
        }

        if (typeof navigator !== 'undefined') {
            if (navigator.language) {
                candidates.push(navigator.language);
            }

            if (Array.isArray(navigator.languages)) {
                candidates.push(...navigator.languages);
            }
        }

        let languageCode = null;
        for (const candidate of candidates) {
            const normalized = normalizeLanguageCode(candidate);
            if (!normalized) {
                continue;
            }

            if (!languageCode) {
                languageCode = normalized;
            }

            if (greetingTemplates[normalized]) {
                languageCode = normalized;
                break;
            }
        }

        if (!languageCode) {
            languageCode = 'en';
        }

        const template = greetingTemplates[languageCode] || greetingTemplates.en;
        const rawName = (profile?.display_name || profile?.username || '').trim();
        const fallbackName = rawName || 'MatePro user';
        const safeName = fallbackName.length > 32 ? `${fallbackName.slice(0, 29)}...` : fallbackName;
        const message = template(safeName);

        elements.greetingBanner.textContent = message;
        elements.greetingBanner.classList.remove('hidden');

        state.greetingMessage = message;
        state.greetingShown = false;
    } catch (error) {
        console.warn('Impossibile caricare il saluto personalizzato:', error);
    }
}

// Check GitHub releases for Windows updates and prompt the user when available.
async function checkForUpdates() {
    try {
        const result = await invoke('check_for_updates');
        if (!result || !result.status) return;

        if (result.status === 'unsupported' || result.status === 'up_to_date') {
            return;
        }

        if (result.status === 'error') {
            console.warn('Update check error:', result.message);
            return;
        }

        if (result.status === 'update_available') {
            const latestVersion = result.latest_version;
            const currentVersion = result.current_version;
            const downloadUrl = result.download_url;

            if (!downloadUrl) {
                console.warn('Nessun URL di download disponibile per l\'aggiornamento.');
                return;
            }

            const confirmUpdate = window.confirm(
                `È disponibile una nuova versione (${latestVersion}).\nVersione corrente: ${currentVersion}.\nVuoi installare l'aggiornamento ora?`
            );

            if (!confirmUpdate) {
                return;
            }

            try {
                await invoke('download_and_install_update', { url: downloadUrl, version: latestVersion });
                window.alert('Installazione avviata. L\'applicazione verrà chiusa per completare l\'aggiornamento.');

                if (appWindow && typeof appWindow.close === 'function') {
                    await appWindow.close();
                } else {
                    window.close();
                }
            } catch (error) {
                showError(`Errore durante l'installazione dell'aggiornamento: ${error}`);
            }
        }
    } catch (error) {
        console.warn('Controllo aggiornamenti non riuscito:', error);
    }
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

        if (state.greetingMessage && !state.greetingShown) {
            addMessage('system', state.greetingMessage, getTimestamp());
            state.greetingShown = true;
        }
        
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

    if (text) {
        state.messageHistory.push(text);
        state.messageHistoryIndex = -1;
    }
    
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
                await openExternal(url);
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

async function addAttachmentFromPath(path) {
    if (!path) {
        showError('Percorso file non disponibile. Usa il pulsante "Allega file" per selezionare il documento.');
        return false;
    }

    if (state.attachedFiles.some(file => file.path === path)) {
        return false;
    }

    try {
        const [filename, content] = await invoke('read_file', { path });
        state.attachedFiles.push({ name: filename, content, path });
        return true;
    } catch (error) {
        showError(`Errore lettura file: ${error}`);
        return false;
    }
}

async function attachFile() {
    const dialogOpen = window.__TAURI__?.dialog?.open;

    if (typeof dialogOpen === 'function') {
        try {
            const selection = await dialogOpen({
                multiple: true,
                filters: [
                    {
                        name: 'Documenti supportati',
                        extensions: ['pdf', 'xlsx', 'xls', 'ods', 'txt', 'md', 'csv'],
                    },
                ],
            });

            if (!selection) {
                return;
            }

            let paths = [];

            if (Array.isArray(selection)) {
                paths = selection;
            } else if (typeof selection === 'string') {
                paths = [selection];
            } else if (selection && Array.isArray(selection.paths)) {
                paths = selection.paths;
            } else if (selection && selection.path) {
                paths = [selection.path];
            }

            let added = false;

            for (const path of paths) {
                const result = await addAttachmentFromPath(path);
                added = added || result;
            }

            if (added) {
                updateAttachedFiles();
                updateSendButton();
            }
        } catch (error) {
            showError(`Errore selezione file: ${error}`);
        }
        return;
    }

    elements.fileInput.click();
}

async function handleFileSelect(event) {
    const file = event.target.files[0];
    event.target.value = '';
    if (!file) return;

    const added = await addAttachmentFromPath(file.path || null);
    if (added) {
        updateAttachedFiles();
        updateSendButton();
    }
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

function applyHistoryValue(value) {
    elements.messageInput.value = value;
    const caretPos = value.length;
    if (typeof elements.messageInput.setSelectionRange === 'function') {
        try {
            elements.messageInput.setSelectionRange(caretPos, caretPos);
        } catch (error) {
            // Ignore unsupported selection operations
        }
    }
    updateSendButton();
}

function navigateMessageHistory(direction) {
    if (state.messageHistory.length === 0) {
        return false;
    }

    if (direction === 'prev') {
        if (state.messageHistoryIndex === -1) {
            state.messageHistoryIndex = state.messageHistory.length - 1;
        } else if (state.messageHistoryIndex > 0) {
            state.messageHistoryIndex -= 1;
        }
    } else if (direction === 'next') {
        if (state.messageHistoryIndex === -1) {
            return false;
        }

        if (state.messageHistoryIndex < state.messageHistory.length - 1) {
            state.messageHistoryIndex += 1;
        } else {
            state.messageHistoryIndex = -1;
            applyHistoryValue('');
            return true;
        }
    } else {
        return false;
    }

    if (state.messageHistoryIndex !== -1) {
        applyHistoryValue(state.messageHistory[state.messageHistoryIndex]);
    }

    return true;
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
            trustServerCertificate: elements.sqlTrustCert.checked,
        });
        
        elements.sqlStatus.className = 'sql-status connected';
        const tlsState = elements.sqlTrustCert.checked ? 'TLS non verificato' : 'TLS verificato';
        elements.sqlStatus.textContent = `✓ Connesso! ID: ${connectionId} (${tlsState})`;
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
    state.messageHistoryIndex = -1;
    
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
    state.greetingShown = false;
    state.messageHistoryIndex = -1;
    
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
            return;
        }

        if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && !e.shiftKey && !e.altKey && !e.metaKey && !e.ctrlKey) {
            const input = elements.messageInput;
            const selectionStart = typeof input.selectionStart === 'number' ? input.selectionStart : input.value.length;
            const selectionEnd = typeof input.selectionEnd === 'number' ? input.selectionEnd : input.value.length;

            if (selectionStart !== selectionEnd) {
                return;
            }

            const valueLength = input.value.length;
            const hasMultipleLines = input.value.includes('\n');

            if (e.key === 'ArrowUp') {
                const atStart = selectionStart === 0 && selectionEnd === 0;
                const shouldUseHistory = !hasMultipleLines || atStart;

                if (shouldUseHistory && navigateMessageHistory('prev')) {
                    e.preventDefault();
                }
            } else if (e.key === 'ArrowDown') {
                const atEnd = selectionStart === valueLength && selectionEnd === valueLength;
                const shouldUseHistory = !hasMultipleLines || atEnd;

                if (shouldUseHistory && navigateMessageHistory('next')) {
                    e.preventDefault();
                }
            }
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
    await loadVersionIndicator();
    await loadGreeting();
    checkForUpdates();
    await scanNetwork();
}

// Start the app
document.addEventListener('DOMContentLoaded', init);
