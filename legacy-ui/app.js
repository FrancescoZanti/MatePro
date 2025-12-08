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
    currentIteration: 0,
    maxIterations: 5,
    systemPromptAdded: false,
    pendingToolCalls: [],
    isProcessing: false,
    greetingMessage: null,
    greetingShown: false,
    // AIConnect state
    backendKind: 'ollama_local',
    aiconnectFound: false,
    aiconnectServices: [],
    // Local storage state
    customSystemPrompt: {
        enabled: false,
        content: '',
    },
    currentConversationId: null,
    memoryConversations: [],
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
    aiconnectStatus: document.getElementById('aiconnect-status'),
    
    // Chat
    modelSelector: document.getElementById('model-selector'),
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
    backendIndicator: document.getElementById('backend-indicator'),
    
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
    
    // Settings Modal
    settingsBtn: document.getElementById('settings-btn'),
    settingsModal: document.getElementById('settings-modal'),
    closeSettingsModal: document.getElementById('close-settings-modal'),
    closeSettingsBtn: document.getElementById('close-settings-btn'),
    customPromptEnabled: document.getElementById('custom-prompt-enabled'),
    customPromptContent: document.getElementById('custom-prompt-content'),
    settingsStatus: document.getElementById('settings-status'),
    saveSettingsBtn: document.getElementById('save-settings-btn'),
    dataDirInfo: document.getElementById('data-dir-info'),
    dataDirPath: document.getElementById('data-dir-path'),
    
    // History Sidebar
    historyList: document.getElementById('history-list'),
    clearHistoryBtn: document.getElementById('clear-history-btn'),
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

function normalizeTextForMatch(text) {
    return text
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase();
}

function detectNewsQuery(message) {
    if (!message || typeof message !== 'string') {
        return null;
    }

    const normalized = normalizeTextForMatch(message);

    const directKeywords = [
        'notizie',
        'news',
        'ultime notizie',
        'ultima ora',
        'ultimora',
        'breaking news',
        'aggiornamenti',
        'aggiornamento',
        'novita',
        'cronaca',
        'titoli',
        'prime pagine',
    ];

    const hasDirectKeyword = directKeywords.some(keyword => normalized.includes(keyword));

    const hasTodayContext =
        normalized.includes('oggi') &&
        (normalized.includes('successo') ||
            normalized.includes('accaduto') ||
            normalized.includes('succede') ||
            normalized.includes('e successo') ||
            normalized.includes("e' successo"));

    if (!hasDirectKeyword && !hasTodayContext) {
        return null;
    }

    const query = message.trim();
    return query.length > 6 ? query : 'notizie oggi';
}

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

function decodeHtmlEntities(text) {
    const div = document.createElement('div');
    div.innerHTML = text;
    return div.textContent || '';
}

function applyInlineFormatting(text) {
    if (!text) {
        return '';
    }

    const codeSpans = [];
    let processed = text.replace(/`([^`]+)`/g, (_, code) => {
        const index = codeSpans.length;
        codeSpans.push(code);
        return `\u0000${index}\u0000`;
    });

    const escapedChars = [];
    processed = processed.replace(/\\([*_~`\\])/g, (_, char) => {
        const index = escapedChars.length;
        escapedChars.push(char);
        return `\u0002${index}\u0002`;
    });

    processed = processed.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (match, label, url) => {
        const decodedUrl = decodeHtmlEntities(url.trim());
        if (!/^(https?:|mailto:)/i.test(decodedUrl)) {
            return match;
        }
        const safeHref = escapeHtml(decodedUrl);
        const safeLabel = label.trim() ? label : decodedUrl;
        return `<a href="${safeHref}" target="_blank" rel="noopener noreferrer">${safeLabel}</a>`;
    });

    processed = processed.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    processed = processed.replace(/~~([^~]+)~~/g, '<del>$1</del>');
    processed = processed.replace(/\*([^*]+)\*/g, '<em>$1</em>');

    processed = processed.replace(/\u0002(\d+)\u0002/g, (_, index) => escapeHtml(escapedChars[Number(index)]));

    return processed.replace(/\u0000(\d+)\u0000/g, (_, index) => `<code>${codeSpans[Number(index)]}</code>`);
}

function formatMessage(content) {
    if (!content) {
        return '';
    }

    const lines = content.replace(/\r\n/g, '\n').split('\n');
    const htmlParts = [];
    let inCodeBlock = false;
    let codeLines = [];
    let codeLanguage = '';
    let currentList = null;
    let listPendingClose = false;
    let paragraphLines = [];

    const flushParagraph = () => {
        if (paragraphLines.length === 0) {
            return;
        }
        htmlParts.push(`<p>${paragraphLines.join('<br>')}</p>`);
        paragraphLines = [];
    };

    const flushList = () => {
        if (!currentList) {
            listPendingClose = false;
            return;
        }
        const items = currentList.items.join('');
        htmlParts.push(`<${currentList.type}>${items}</${currentList.type}>`);
        currentList = null;
        listPendingClose = false;
    };

    const closePendingList = () => {
        if (listPendingClose && currentList) {
            flushList();
        }
        listPendingClose = false;
    };

    const flushCode = () => {
        if (!inCodeBlock) {
            return;
        }
        const classAttr = codeLanguage ? ` class="language-${codeLanguage}"` : '';
        htmlParts.push(`<pre><code${classAttr}>${codeLines.join('\n')}</code></pre>`);
        inCodeBlock = false;
        codeLines = [];
        codeLanguage = '';
    };

    for (const rawLine of lines) {
        const line = rawLine.replace(/\t/g, '    ');

        if (line.trim().startsWith('```')) {
            if (inCodeBlock) {
                flushCode();
            } else {
                flushParagraph();
                flushList();
                inCodeBlock = true;
                const languageRaw = line.trim().slice(3).trim().toLowerCase();
                codeLanguage = languageRaw.replace(/[^a-z0-9+#._-]/gi, '');
            }
            continue;
        }

        if (inCodeBlock) {
            codeLines.push(escapeHtml(line));
            continue;
        }

        const trimmed = line.trim();

        if (trimmed.length === 0) {
            flushParagraph();
            if (currentList) {
                listPendingClose = true;
            }
            continue;
        }

        if (currentList) {
            const continuationMatch = rawLine.match(/^\s{2,}(.*)$/);
            if (continuationMatch && continuationMatch[1].trim().length > 0) {
                listPendingClose = false;
                const continuationText = applyInlineFormatting(escapeHtml(continuationMatch[1].trim()));
                if (currentList.items.length > 0) {
                    const lastIndex = currentList.items.length - 1;
                    currentList.items[lastIndex] = currentList.items[lastIndex].replace(
                        /<\/li>$/,
                        `<br>${continuationText}</li>`
                    );
                }
                continue;
            }
        }

        const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
        if (headingMatch) {
            closePendingList();
            flushParagraph();
            flushList();
            const level = headingMatch[1].length;
            const headingText = applyInlineFormatting(escapeHtml(headingMatch[2].trim()));
            htmlParts.push(`<h${level}>${headingText}</h${level}>`);
            continue;
        }

        if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
            closePendingList();
            flushParagraph();
            flushList();
            htmlParts.push('<hr>');
            continue;
        }

        if (trimmed.startsWith('>')) {
            closePendingList();
            flushParagraph();
            flushList();
            const quoteText = applyInlineFormatting(escapeHtml(trimmed.replace(/^>\s?/, '')));
            htmlParts.push(`<blockquote>${quoteText}</blockquote>`);
            continue;
        }

        const unorderedMatch = trimmed.match(/^([*+-])\s+(.*)$/);
        if (unorderedMatch) {
            flushParagraph();
            listPendingClose = false;
            const itemText = applyInlineFormatting(escapeHtml(unorderedMatch[2]));
            if (!currentList || currentList.type !== 'ul') {
                flushList();
                currentList = { type: 'ul', items: [] };
            }
            currentList.items.push(`<li>${itemText}</li>`);
            continue;
        }

        const orderedMatch = trimmed.match(/^(\d+)[.)]\s+(.*)$/);
        if (orderedMatch) {
            flushParagraph();
            listPendingClose = false;
            const itemText = applyInlineFormatting(escapeHtml(orderedMatch[2]));
            if (!currentList || currentList.type !== 'ol') {
                flushList();
                currentList = { type: 'ol', items: [] };
            }
            currentList.items.push(`<li>${itemText}</li>`);
            continue;
        }

        closePendingList();
        paragraphLines.push(applyInlineFormatting(escapeHtml(line)));
    }

    flushCode();
    flushParagraph();
    flushList();

    return htmlParts.join('');
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

function updateAiConnectStatus(found, services) {
    state.aiconnectFound = found;
    state.aiconnectServices = services;
    
    if (elements.aiconnectStatus) {
        if (found) {
            elements.aiconnectStatus.textContent = '🤖 AIConnect trovato';
            elements.aiconnectStatus.className = 'aiconnect-status found';
            elements.aiconnectStatus.classList.remove('hidden');
        } else {
            elements.aiconnectStatus.textContent = '';
            elements.aiconnectStatus.classList.add('hidden');
        }
    }
}

function updateBackendIndicator() {
    if (elements.backendIndicator) {
        if (state.backendKind === 'ai_connect') {
            elements.backendIndicator.textContent = '🤖 AIConnect';
            elements.backendIndicator.className = 'backend-indicator aiconnect';
        } else {
            elements.backendIndicator.textContent = '🦙 Ollama';
            elements.backendIndicator.className = 'backend-indicator ollama';
        }
        elements.backendIndicator.classList.remove('hidden');
    }
}

async function scanNetwork() {
    elements.scanningIndicator.classList.remove('hidden');
    elements.serverList.classList.add('hidden');
    
    try {
        // Try the new scan_services command first (AIConnect + Ollama)
        let discoveryResult = null;
        try {
            discoveryResult = await invoke('scan_services');
        } catch (e) {
            console.log('scan_services not available, falling back to scan_network');
        }
        
        if (discoveryResult) {
            // Update AIConnect status
            updateAiConnectStatus(
                discoveryResult.aiconnect_found,
                discoveryResult.aiconnect_services
            );
            
            // Update backend kind
            state.backendKind = discoveryResult.recommended_backend;
            
            // Build server list
            const servers = [];
            
            // Add AIConnect services first if found
            if (discoveryResult.aiconnect_found && discoveryResult.aiconnect_services.length > 0) {
                discoveryResult.aiconnect_services.forEach(service => {
                    const url = `http://${service.host}:${service.port}`;
                    if (!servers.includes(url)) {
                        servers.push(url);
                    }
                });
            }
            
            // Add Ollama servers
            discoveryResult.ollama_servers.forEach(server => {
                if (!servers.includes(server)) {
                    servers.push(server);
                }
            });
            
            if (servers.length > 0) {
                elements.servers.innerHTML = '';
                servers.forEach((server, index) => {
                    const isAiConnect = discoveryResult.aiconnect_services.some(
                        s => `http://${s.host}:${s.port}` === server
                    );
                    const isLocal = server.includes('localhost') || server.includes('127.0.0.1');
                    const option = document.createElement('div');
                    option.className = 'server-option';
                    
                    let icon = isLocal ? '🏠' : '🌐';
                    if (isAiConnect) {
                        icon = '🤖';
                    }
                    
                    option.textContent = `${icon} ${server}`;
                    option.dataset.url = server;
                    option.dataset.isAiconnect = isAiConnect ? 'true' : 'false';
                    
                    if (server === elements.serverUrl.value || index === 0) {
                        option.classList.add('selected');
                        if (index === 0) {
                            elements.serverUrl.value = server;
                        }
                    }
                    
                    option.addEventListener('click', () => {
                        document.querySelectorAll('.server-option').forEach(el => el.classList.remove('selected'));
                        option.classList.add('selected');
                        elements.serverUrl.value = server;
                        // Update backend kind based on selection
                        state.backendKind = option.dataset.isAiconnect === 'true' ? 'ai_connect' : 'ollama_local';
                    });
                    
                    elements.servers.appendChild(option);
                });
                
                elements.serverList.classList.remove('hidden');
            }
        } else {
            // Fallback to legacy scan_network
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
    
    // Check if connecting to AIConnect
    const isAiConnect = state.backendKind === 'ai_connect';
    
    if (isAiConnect) {
        elements.loadingText.textContent = 'Connessione ad AIConnect...';
    } else {
        elements.loadingText.textContent = 'Connessione al server...';
    }
    
    try {
        // Set backend configuration
        const config = {
            kind: isAiConnect ? 'ai_connect' : 'ollama_local',
            endpoint: url,
            auth: { none: null },
            aiconnect_service: null,
        };
        
        try {
            await invoke('set_backend_config', { config });
        } catch (e) {
            console.log('set_backend_config not available, using legacy connect');
        }
        
        await invoke('connect_to_server', { url });
        await loadModels();
        
        // Update backend indicator after successful connection
        updateBackendIndicator();
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
        await loadMemory();
        renderHistoryList();
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

const THINK_TAG_REGEX = /<think>([\s\S]*?)<\/think>/gi;

function splitVisibleContentAndReasoning(content) {
    if (!content || typeof content !== 'string') {
        return {
            visible: content || '',
            reasoningBlocks: [],
        };
    }

    const reasoningBlocks = [];
    const withoutReasoning = content.replace(THINK_TAG_REGEX, (_, inner) => {
        const trimmed = inner.trim();
        if (trimmed.length > 0) {
            reasoningBlocks.push(trimmed);
        }
        return '';
    });

    const normalized = withoutReasoning.replace(/\n{3,}/g, '\n\n').trim();

    return {
        visible: normalized,
        reasoningBlocks,
    };
}

function addMessage(role, content, timestamp = null) {
    const emptyState = elements.messages.querySelector('.empty-state');
    if (emptyState) {
        emptyState.remove();
    }
    
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${role}`;
    
    const bubble = document.createElement('div');
    bubble.className = 'message-bubble';
    let displayContent = content;
    let reasoningBlocks = [];

    if (role === 'assistant') {
        const splitResult = splitVisibleContentAndReasoning(content);
        displayContent = splitResult.visible;
        reasoningBlocks = splitResult.reasoningBlocks;
    }

    bubble.innerHTML = role === 'user' ? escapeHtml(displayContent) : formatMessage(displayContent);

    if (role === 'assistant' && reasoningBlocks.length > 0) {
        const details = document.createElement('details');
        details.className = 'think-block';

        const summary = document.createElement('summary');
        summary.textContent = 'Mostra ragionamento del modello';
        details.appendChild(summary);

        const thinkBody = document.createElement('div');
        thinkBody.className = 'think-content';
        thinkBody.innerHTML = formatMessage(reasoningBlocks.join('\n\n'));
        details.appendChild(thinkBody);

        details.addEventListener('toggle', () => {
            summary.textContent = details.open
                ? 'Nascondi ragionamento del modello'
                : 'Mostra ragionamento del modello';
        });

        bubble.appendChild(details);
    }

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
    return await invoke('get_tools_description');
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
        
        // Add custom system prompt if enabled
        if (state.customSystemPrompt.enabled && state.customSystemPrompt.content.trim()) {
            systemContent += '\n\n**ISTRUZIONI PERSONALIZZATE DELL\'UTENTE:**\n' + state.customSystemPrompt.content.trim();
        }
        
        const toolsDesc = await getToolsDescription();
        if (typeof toolsDesc === 'string' && toolsDesc.trim().length > 0) {
            systemContent += '\n\n' + toolsDesc.trim();
        }
        systemContent += '\n\n**LINEE GUIDA:**\n- Usa i tool appropriati per le richieste dell\'utente.\n- Se la risposta richiede dati aggiornati o verifiche, esegui `web_search` e integra solo fonti considerate affidabili.\n- Quando ricevi note di ricerca dal backend, trattale come riferimenti da citare in formato [Titolo](URL) indicando il dominio.\n- Riassumi con parole tue e segnala eventuali incongruenze o assenza di dati aggiornati.';
        
        state.conversation.push({ role: 'user', content: systemContent, hidden: true });
        state.conversation.push({ 
            role: 'assistant', 
            content: 'Perfetto! Sono pronto ad aiutarti.',
            hidden: true 
        });
        state.systemPromptAdded = true;
    }
    
    state.conversation.push({ role: 'user', content: fullContent, hidden: false });

    const newsQuery = detectNewsQuery(text);
    if (newsQuery) {
        const safeQuery = newsQuery.replace(/"/g, "'");
        state.conversation.push({
            role: 'user',
            content: `PROMEMORIA AGENTE: L'utente sta chiedendo notizie o eventi attuali. Esegui il tool web_search con la query "${safeQuery}" (usa max_results=5) prima di rispondere. Riassumi i risultati aggiornati in italiano e cita le fonti principali usando collegamenti Markdown [Titolo](URL) e indicando il dominio della fonte.`,
            hidden: true,
        });
    }

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
        
        const toolCalls = await invoke('parse_tool_calls', { response: response.content });

        if (toolCalls.length > 0) {
            state.pendingToolCalls = toolCalls;
            await processNextToolCall();
        } else {
            await saveCurrentConversation();
        }
        
    } catch (error) {
        removeLoadingIndicator();
        showError(`Errore: ${error}`);
        state.conversation.pop(); // Remove user message
    }
    
    state.isProcessing = false;
    updateIterationCounter();
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
        } else {
            // Save conversation when agent loop completes
            await saveCurrentConversation();
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
    if (!elements.iterationCounter) {
        return;
    }

    const shouldShow = state.isProcessing || state.currentIteration > 0;
    if (shouldShow) {
        elements.iterationCounter.textContent = `${state.currentIteration}/${state.maxIterations}`;
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

// ============ SETTINGS & CUSTOM SYSTEM PROMPT ============

async function loadSettings() {
    try {
        const prompt = await invoke('load_custom_system_prompt');
        state.customSystemPrompt = {
            enabled: prompt.enabled,
            content: prompt.content,
        };
        
        // Update UI
        if (elements.customPromptEnabled) {
            elements.customPromptEnabled.checked = prompt.enabled;
        }
        if (elements.customPromptContent) {
            elements.customPromptContent.value = prompt.content;
        }
    } catch (error) {
        console.warn('Impossibile caricare le impostazioni:', error);
    }
}

async function saveSettings() {
    const enabled = elements.customPromptEnabled?.checked || false;
    const content = elements.customPromptContent?.value || '';
    
    try {
        await invoke('save_custom_system_prompt', {
            prompt: {
                enabled,
                content,
                updated_at: new Date().toISOString(),
            }
        });
        
        state.customSystemPrompt = { enabled, content };
        
        if (elements.settingsStatus) {
            elements.settingsStatus.className = 'sql-status success';
            elements.settingsStatus.textContent = '✓ Impostazioni salvate';
            elements.settingsStatus.classList.remove('hidden');
            
            setTimeout(() => {
                elements.settingsStatus.classList.add('hidden');
            }, 2000);
        }
    } catch (error) {
        if (elements.settingsStatus) {
            elements.settingsStatus.className = 'sql-status error';
            elements.settingsStatus.textContent = `✕ Errore: ${error}`;
            elements.settingsStatus.classList.remove('hidden');
        }
    }
}

async function showSettingsModal() {
    await loadSettings();
    
    // Try to show the data directory
    try {
        const dataDir = await invoke('get_data_directory');
        if (elements.dataDirPath) {
            elements.dataDirPath.textContent = dataDir;
        }
        if (elements.dataDirInfo) {
            elements.dataDirInfo.classList.remove('hidden');
        }
    } catch (error) {
        console.warn('Impossibile ottenere la directory dati:', error);
    }
    
    if (elements.settingsModal) {
        elements.settingsModal.classList.remove('hidden');
    }
}

function hideSettingsModal() {
    if (elements.settingsModal) {
        elements.settingsModal.classList.add('hidden');
    }
    if (elements.settingsStatus) {
        elements.settingsStatus.classList.add('hidden');
    }
}

// ============ CONVERSATION HISTORY ============

async function loadMemory() {
    try {
        const memory = await invoke('load_memory');
        state.memoryConversations = memory.conversations || [];
    } catch (error) {
        console.warn('Impossibile caricare la memoria:', error);
        state.memoryConversations = [];
    }
}

async function saveCurrentConversation() {
    // Only save if there are visible messages
    const visibleMessages = state.conversation.filter(m => !m.hidden);
    if (visibleMessages.length < 2) return;
    
    // Generate a title from the first user message
    const firstUserMessage = visibleMessages.find(m => m.role === 'user');
    const title = firstUserMessage 
        ? firstUserMessage.content.substring(0, 50) + (firstUserMessage.content.length > 50 ? '...' : '')
        : 'Conversazione senza titolo';
    
    // Convert conversation to memory format
    const messages = state.conversation.map(m => ({
        role: m.role,
        content: m.content,
        hidden: m.hidden || false,
        timestamp: m.timestamp || null,
    }));
    
    try {
        if (state.currentConversationId) {
            // Update existing conversation
            await invoke('update_conversation_in_memory', {
                id: state.currentConversationId,
                messages,
            });
        } else {
            // Add new conversation
            const id = await invoke('add_conversation_to_memory', {
                title,
                messages,
                model: state.selectedModel,
            });
            state.currentConversationId = id;
        }

        await loadMemory();
        renderHistoryList();
    } catch (error) {
        console.warn('Impossibile salvare la conversazione:', error);
    }
}

async function loadConversationFromMemory(conversationId) {
    const conversation = state.memoryConversations.find(c => c.id === conversationId);
    if (!conversation) return;
    
    // Clear current chat
    state.conversation = [];
    state.systemPromptAdded = false;
    state.currentIteration = 0;
    state.pendingToolCalls = [];
    state.currentConversationId = conversationId;
    
    if (conversation.model && elements.modelSelector) {
        state.selectedModel = conversation.model;
        const hasOption = Array.from(elements.modelSelector.options || []).some(opt => opt.value === conversation.model);
        if (hasOption) {
            elements.modelSelector.value = conversation.model;
        }
    }
    
    // Load messages
    conversation.messages.forEach(m => {
        state.conversation.push({
            role: m.role,
            content: m.content,
            hidden: m.hidden || false,
            timestamp: m.timestamp || null,
        });
        
        // Mark system prompt as added if it was in the saved conversation
        if (m.hidden && m.role === 'user') {
            state.systemPromptAdded = true;
        }
    });
    
    // Render messages
    renderConversation();

    // Update sidebar highlight
    renderHistoryList();
}

function renderConversation() {
    elements.messages.innerHTML = '';
    
    const visibleMessages = state.conversation.filter(m => !m.hidden);
    
    if (visibleMessages.length === 0) {
        elements.messages.innerHTML = `
            <div class="empty-state">
                <p class="empty-title">Inizia una conversazione</p>
                <p class="empty-subtitle">Scrivi un messaggio per iniziare</p>
            </div>
        `;
        return;
    }
    
    visibleMessages.forEach(m => {
        addMessage(m.role, m.content, m.timestamp);
    });
}

async function deleteConversationFromMemory(conversationId) {
    try {
        await invoke('delete_conversation_from_memory', { id: conversationId });
        
        if (state.currentConversationId === conversationId) {
            state.currentConversationId = null;
            state.conversation = [];
            state.pendingToolCalls = [];
            state.currentIteration = 0;
            state.attachedFiles = [];
            state.messageHistoryIndex = -1;
            renderConversation();
            updateAttachedFiles();
            updateIterationCounter();
            hideError();
        }
        
        await loadMemory();
        renderHistoryList();
    } catch (error) {
        console.warn('Impossibile eliminare la conversazione:', error);
    }
}

async function clearAllConversations() {
    if (!confirm('Sei sicuro di voler eliminare tutta la cronologia delle conversazioni?')) {
        return;
    }
    
    try {
        await invoke('clear_all_conversations');
        state.memoryConversations = [];
        state.currentConversationId = null;
        state.conversation = [];
        state.pendingToolCalls = [];
        state.currentIteration = 0;
        state.attachedFiles = [];
        state.messageHistoryIndex = -1;
        renderConversation();
        updateAttachedFiles();
        updateIterationCounter();
        hideError();
        await loadMemory();
        renderHistoryList();
    } catch (error) {
        console.warn('Impossibile cancellare la cronologia:', error);
    }
}

function renderHistoryList() {
    if (!elements.historyList) return;

    if (elements.clearHistoryBtn) {
        const isEmpty = state.memoryConversations.length === 0;
        elements.clearHistoryBtn.disabled = isEmpty;
    }
    
    if (state.memoryConversations.length === 0) {
        elements.historyList.innerHTML = `
            <div class="empty-history">
                <p>Nessuna conversazione salvata</p>
                <small>Le conversazioni verranno salvate automaticamente quando le termini</small>
            </div>
        `;
        return;
    }
    
    // Sort by updated_at descending (most recent first)
    const sorted = [...state.memoryConversations].sort((a, b) => {
        return new Date(b.updated_at) - new Date(a.updated_at);
    });
    
    elements.historyList.innerHTML = sorted.map(conv => {
        const date = new Date(conv.updated_at);
        const dateStr = date.toLocaleDateString();
        const timeStr = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        const msgCount = conv.messages.filter(m => !m.hidden).length;
        const isActive = state.currentConversationId === conv.id;
        
        return `
            <div class="history-item${isActive ? ' active' : ''}" data-id="${escapeHtml(conv.id)}">
                <div class="history-item-content">
                    <div class="history-item-title">${escapeHtml(conv.title)}</div>
                    <div class="history-item-meta">
                        <span>📅 ${dateStr} ${timeStr}</span>
                        <span>💬 ${msgCount} messaggi</span>
                        ${conv.model ? `<span>🤖 ${escapeHtml(conv.model)}</span>` : ''}
                    </div>
                    ${isActive ? '<span class="history-item-status">Conversazione attiva</span>' : ''}
                </div>
                <div class="history-item-actions">
                    <button class="delete-conv-btn" data-id="${escapeHtml(conv.id)}" title="Elimina conversazione">🗑️</button>
                </div>
            </div>
        `;
    }).join('');
    
    // Add event listeners
    elements.historyList.querySelectorAll('.delete-conv-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            if (confirm('Eliminare questa conversazione?')) {
                deleteConversationFromMemory(btn.dataset.id);
            }
        });
    });
    
    // Also allow clicking the whole item to load
    elements.historyList.querySelectorAll('.history-item').forEach(item => {
        item.addEventListener('click', () => {
            loadConversationFromMemory(item.dataset.id);
        });
    });
}

// ============ NEW CHAT / DISCONNECT ============

async function newChat() {
    // Save current conversation before starting new one
    await saveCurrentConversation();
    
    state.conversation = [];
    state.attachedFiles = [];
    state.systemPromptAdded = false;
    state.currentIteration = 0;
    state.pendingToolCalls = [];
    state.messageHistoryIndex = -1;
    state.currentConversationId = null;
    
    elements.messages.innerHTML = `
        <div class="empty-state">
            <p class="empty-title">Inizia una conversazione</p>
            <p class="empty-subtitle">Scrivi un messaggio per iniziare</p>
        </div>
    `;
    
    updateAttachedFiles();
    updateIterationCounter();
    hideError();
    renderHistoryList();
}

async function disconnect() {
    // Save current conversation before disconnecting
    await saveCurrentConversation();
    
    state.conversation = [];
    state.models = [];
    state.selectedModel = null;
    state.attachedFiles = [];
    state.systemPromptAdded = false;
    state.currentIteration = 0;
    state.greetingShown = false;
    state.messageHistoryIndex = -1;
    state.currentConversationId = null;
    
    showScreen('setup-screen');
    elements.setupError.classList.add('hidden');
    renderHistoryList();
    updateAttachedFiles();
    updateIterationCounter();
    hideError();
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
    
    // Settings Modal
    if (elements.settingsBtn) {
        elements.settingsBtn.addEventListener('click', showSettingsModal);
    }
    if (elements.closeSettingsModal) {
        elements.closeSettingsModal.addEventListener('click', hideSettingsModal);
    }
    if (elements.closeSettingsBtn) {
        elements.closeSettingsBtn.addEventListener('click', hideSettingsModal);
    }
    if (elements.saveSettingsBtn) {
        elements.saveSettingsBtn.addEventListener('click', saveSettings);
    }
    
    // History Sidebar
    if (elements.clearHistoryBtn) {
        elements.clearHistoryBtn.addEventListener('click', clearAllConversations);
    }
    
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
    
    if (elements.settingsModal) {
        elements.settingsModal.addEventListener('click', (e) => {
            if (e.target === elements.settingsModal) hideSettingsModal();
        });
    }
    
}

// ============ INITIALIZATION ============

async function init() {
    initEventListeners();
    updateIterationCounter();
    await loadVersionIndicator();
    await loadGreeting();
    await loadSettings();
    await loadMemory();
    renderHistoryList();
    checkForUpdates();
    await scanNetwork();
}

// Start the app
document.addEventListener('DOMContentLoaded', init);
