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
    agentMode: true,
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
    memoryContext: '',
    memoryContextInjected: false,
    calendarEvents: [],
    calendarStatusTimeout: null,
    integrations: {
        outlook: {
            configured: false,
            connected: false,
            pending: false,
            tenant: null,
            clientId: null,
            message: null,
            expiresAt: null,
            userCode: null,
            verificationUri: null,
            interval: 5,
        },
        google: {
            configured: false,
            connected: false,
            pending: false,
            clientId: null,
            calendarId: 'primary',
            message: null,
            expiresAt: null,
            userCode: null,
            verificationUri: null,
            interval: 5,
        },
    },
    integrationPolls: {
        outlook: null,
        google: null,
    },
    integrationPrompts: {
        googleCredentials: false,
        outlookCredentials: false,
    },
    pendingIntegrationStep: null,
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
    backendIndicator: document.getElementById('backend-indicator'),
    calendarList: document.getElementById('calendar-list'),
    calendarStatus: document.getElementById('calendar-status'),
    exportCalendarBtn: document.getElementById('export-calendar-btn'),
    clearCalendarBtn: document.getElementById('clear-calendar-btn'),
    
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

// Memory context configuration to avoid overloading the prompt
const MEMORY_CONTEXT_CONFIG = {
    maxConversations: 3,
    maxMessagesPerConversation: 4,
    maxCharsPerMessage: 180,
    maxHighlights: 8,
};

const MEMORY_HIGHLIGHT_PATTERNS = [
    /\bmi piace\b/i,
    /\bmi ador[oa]\b/i,
    /\bador[oa]\b/i,
    /\bamo\b/i,
    /\bpreferisco\b/i,
    /\bnon sopporto\b/i,
    /\bodio\b/i,
    /\ballergic[oa]\b/i,
    /\bintollerante\b/i,
    /\bdevo\b/i,
    /\bho bisogno\b/i,
    /\bho\s+(una|un|il|la)\b/i,
    /\bdomani\b/i,
    /\bsettimana prossima\b/i,
    /\btra\s+\d+\s+(giorni|settimane|mesi)\b/i,
    /\bsto\s+(lavorando|preparando|studiando)\b/i,
    /\bvado\b/i,
    /\bparteciper[òo]\b/i,
    /\bho\s+una\s+(gara|maratona|mezza maratona|visita|riunione)\b/i,
];

const MONTHS_IT = {
    gennaio: 0,
    febbraio: 1,
    marzo: 2,
    aprile: 3,
    maggio: 4,
    giugno: 5,
    luglio: 6,
    agosto: 7,
    settembre: 8,
    ottobre: 9,
    novembre: 10,
    dicembre: 11,
};

const WEEK_DAYS_IT = ['domenica', 'lunedì', 'martedì', 'mercoledì', 'giovedì', 'venerdì', 'sabato'];

const RELATIVE_KEYWORDS = {
    oggi: 0,
    stasera: 0,
    stanotte: 0,
    stamattina: 0,
    domani: 1,
    dopodomani: 2,
};

// ============ UTILITIES ============

function normalizeTextForMatch(text) {
    return text
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase();
}

function sanitizeMemoryText(text) {
    if (!text || typeof text !== 'string') return '';
    return text.replace(/[\s\u0000-\u001f\u007f]+/g, ' ').trim();
}

function truncateMemoryText(text, maxLength) {
    if (!text) return '';
    if (text.length <= maxLength) return text;
    return `${text.slice(0, maxLength - 1).trimEnd()}…`;
}

function extractMemoryHighlights(conversations) {
    const highlights = [];
    const seen = new Set();

    conversations.forEach(conv => {
        (conv.messages || [])
            .filter(msg => msg && !msg.hidden && msg.role === 'user')
            .forEach(msg => {
                const cleaned = sanitizeMemoryText(msg.content || '');
                if (!cleaned) return;

                const sentences = cleaned
                    .split(/[.!?\n]+/)
                    .map(sentence => sentence.trim())
                    .filter(Boolean);

                sentences.forEach(sentence => {
                    if (sentence.length < 12) return;
                    const normalized = sentence.toLowerCase();
                    if (seen.has(normalized)) return;

                    const matchesKeyword = MEMORY_HIGHLIGHT_PATTERNS.some(pattern => pattern.test(sentence));
                    if (!matchesKeyword) return;

                    seen.add(normalized);
                    highlights.push(truncateMemoryText(sentence, 220));
                });
            });
    });

    return highlights.slice(0, MEMORY_CONTEXT_CONFIG.maxHighlights);
}

function buildMemoryContext() {
    if (!Array.isArray(state.memoryConversations) || state.memoryConversations.length === 0) {
        return '';
    }

    const conversations = state.memoryConversations
        .filter(conv => conv && conv.id !== state.currentConversationId)
        .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at))
        .slice(0, MEMORY_CONTEXT_CONFIG.maxConversations);

    const segments = [];
    const highlights = extractMemoryHighlights(conversations);

    if (highlights.length > 0) {
        segments.push('PROMEMORIA UTENTE:');
        highlights.forEach(item => {
            segments.push(`• ${item}`);
        });
        segments.push('');
    }

    conversations.forEach(conv => {
        const updatedAt = conv.updated_at ? new Date(conv.updated_at) : null;
        const dateLabel = updatedAt
            ? `${updatedAt.toLocaleDateString()} ${updatedAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
            : 'data sconosciuta';

        const title = sanitizeMemoryText(conv.title || 'Conversazione precedente');
        segments.push(`Conversazione "${title}" (${dateLabel})`);

        const visibleMessages = (conv.messages || [])
            .filter(msg => msg && !msg.hidden)
            .slice(-MEMORY_CONTEXT_CONFIG.maxMessagesPerConversation);

        visibleMessages.forEach(msg => {
            const roleLabel = msg.role === 'assistant' ? 'Assistente' : 'Utente';
            const cleanedContent = truncateMemoryText(
                sanitizeMemoryText(msg.content || ''),
                MEMORY_CONTEXT_CONFIG.maxCharsPerMessage
            );

            if (cleanedContent) {
                segments.push(`- ${roleLabel}: ${cleanedContent}`);
            }
        });

        segments.push('');
    });

    return segments.join('\n').trim();
}

function updateMemoryContext() {
    state.memoryContext = buildMemoryContext();
}

function showCalendarStatus(message, isError = false) {
    if (!elements.calendarStatus) return;

    elements.calendarStatus.textContent = message;
    elements.calendarStatus.classList.remove('hidden');
    elements.calendarStatus.classList.toggle('error', isError);

    if (state.calendarStatusTimeout) {
        clearTimeout(state.calendarStatusTimeout);
    }

    state.calendarStatusTimeout = setTimeout(() => {
        if (elements.calendarStatus) {
            elements.calendarStatus.classList.add('hidden');
        }
    }, 4000);
}

function clearOutlookPollTimer() {
    if (state.integrationPolls.outlook) {
        clearTimeout(state.integrationPolls.outlook);
        state.integrationPolls.outlook = null;
    }
}

function clearGooglePollTimer() {
    if (state.integrationPolls.google) {
        clearTimeout(state.integrationPolls.google);
        state.integrationPolls.google = null;
    }
}

function applyOutlookStatus(rawStatus = {}) {
    state.integrations.outlook = {
        configured: Boolean(rawStatus.configured),
        connected: Boolean(rawStatus.connected),
        pending: Boolean(rawStatus.pending),
        tenant: rawStatus.tenant || null,
        clientId: rawStatus.client_id || null,
        message: rawStatus.message || null,
        expiresAt: rawStatus.expires_at || null,
        userCode: rawStatus.user_code || null,
        verificationUri: rawStatus.verification_uri || null,
        interval: rawStatus.interval || 5,
    };

    if (state.integrations.outlook.pending) {
        scheduleOutlookPoll(state.integrations.outlook.interval);
    } else {
        clearOutlookPollTimer();
    }
}

function applyGoogleStatus(rawStatus = {}) {
    state.integrations.google = {
        configured: Boolean(rawStatus.configured),
        connected: Boolean(rawStatus.connected),
        pending: Boolean(rawStatus.pending),
        clientId: rawStatus.client_id || null,
        calendarId: rawStatus.calendar_id || 'primary',
        message: rawStatus.message || null,
        expiresAt: rawStatus.expires_at || null,
        userCode: rawStatus.user_code || null,
        verificationUri: rawStatus.verification_uri || null,
        interval: rawStatus.interval || 5,
    };

    if (state.integrations.google.pending) {
        scheduleGooglePoll(state.integrations.google.interval);
    } else {
        clearGooglePollTimer();
    }
}

async function refreshCalendarIntegrationsStatus(options = {}) {
    try {
        const status = await invoke('get_calendar_integrations_status');
        if (status && status.outlook) {
            applyOutlookStatus(status.outlook);
        }
        if (status && status.google) {
            applyGoogleStatus(status.google);
        }
        return status;
    } catch (error) {
        if (!options.silent) {
            console.warn('Impossibile aggiornare lo stato delle integrazioni calendario:', error);
            showCalendarStatus('Impossibile ottenere lo stato delle integrazioni', true);
        }
        return null;
    }
}

function scheduleOutlookPoll(intervalSeconds = 5) {
    clearOutlookPollTimer();
    const delay = Math.max(2000, (intervalSeconds || 5) * 1000);
    state.integrationPolls.outlook = setTimeout(() => {
        pollOutlookDeviceFlowOnce().catch(error => {
            console.warn('Polling Outlook fallito:', error);
        });
    }, delay);
}

async function pollOutlookDeviceFlowOnce() {
    try {
        const result = await invoke('poll_outlook_calendar_device_flow');
        switch (result.status) {
            case 'pending':
                scheduleOutlookPoll(result.retry_in || state.integrations.outlook.interval || 5);
                if (result.message) {
                    showCalendarStatus(result.message, false);
                }
                break;
            case 'completed':
                await refreshCalendarIntegrationsStatus({ silent: true });
                showCalendarStatus('Outlook collegato con successo');
                addAssistantResponse('Ho collegato Outlook al calendario e sincronizzerò i nuovi eventi.');
                break;
            case 'expired':
            case 'declined':
                await refreshCalendarIntegrationsStatus({ silent: true });
                showCalendarStatus(result.message || 'Connessione Outlook non completata', true);
                break;
            case 'error':
                showCalendarStatus(result.message || 'Errore durante il collegamento a Outlook', true);
                scheduleOutlookPoll(result.retry_in || state.integrations.outlook.interval || 6);
                break;
            default:
                clearOutlookPollTimer();
                break;
        }
    } catch (error) {
        showCalendarStatus('Errore durante il collegamento a Outlook', true);
        scheduleOutlookPoll((state.integrations.outlook.interval || 5) + 2);
        throw error;
    }
}

function scheduleGooglePoll(intervalSeconds = 5) {
    clearGooglePollTimer();
    const delay = Math.max(2000, (intervalSeconds || 5) * 1000);
    state.integrationPolls.google = setTimeout(() => {
        pollGoogleDeviceFlowOnce().catch(error => {
            console.warn('Polling Google Calendar fallito:', error);
        });
    }, delay);
}

async function pollGoogleDeviceFlowOnce() {
    try {
        const result = await invoke('poll_google_calendar_device_flow');
        switch (result.status) {
            case 'pending':
                scheduleGooglePoll(result.retry_in || state.integrations.google.interval || 5);
                if (result.message) {
                    showCalendarStatus(result.message, false);
                }
                break;
            case 'completed':
                await refreshCalendarIntegrationsStatus({ silent: true });
                showCalendarStatus('Google Calendar collegato con successo');
                addAssistantResponse('Ho collegato Google Calendar al calendario e sincronizzerò i nuovi eventi.');
                break;
            case 'expired':
            case 'declined':
                await refreshCalendarIntegrationsStatus({ silent: true });
                showCalendarStatus(result.message || 'Connessione Google Calendar non completata', true);
                break;
            case 'error':
                showCalendarStatus(result.message || 'Errore durante il collegamento a Google Calendar', true);
                scheduleGooglePoll(result.retry_in || state.integrations.google.interval || 6);
                break;
            default:
                clearGooglePollTimer();
                break;
        }
    } catch (error) {
        showCalendarStatus('Errore durante il collegamento a Google Calendar', true);
        scheduleGooglePoll((state.integrations.google.interval || 5) + 2);
        throw error;
    }
}

function parseOutlookCredentials(text) {
    if (!text) return null;
    const clientMatch = text.match(/client(?:\s+outlook)?(?:\s+id)?\s*[:=\-]?\s*([a-z0-9-]{6,})/i);
    if (!clientMatch) {
        return null;
    }
    const tenantMatch = text.match(/tenant\s*[:=\-]?\s*([a-z0-9.-]+)/i);
    return {
        clientId: clientMatch[1].trim(),
        tenant: tenantMatch ? tenantMatch[1].trim() : 'common',
    };
}

function parseGoogleCredentials(text) {
    if (!text) return null;
    const clientMatch = text.match(/client(?:\s+google)?(?:\s+id)?\s*[:=\-]?\s*([a-z0-9._\-]{6,})/i);
    const secretMatch = text.match(/secret(?:\s+google)?\s*[:=\-]?\s*([a-z0-9_\-]{6,})/i);
    if (!clientMatch) {
        return null;
    }
    const calendarMatch = text.match(/calendar(?:\s+id)?\s*[:=\-]?\s*([^\s]+)/i);
    return {
        clientId: clientMatch[1].trim(),
        clientSecret: secretMatch ? secretMatch[1].trim() : null,
        calendarId: calendarMatch ? calendarMatch[1].trim() : 'primary',
    };
}

function buildOutlookEventsSummary(events) {
    if (!Array.isArray(events) || events.length === 0) {
        return 'Non ho trovato eventi futuri su Outlook.';
    }

    const lines = ['Ecco i prossimi impegni presenti su Outlook:'];
    events.slice(0, 10).forEach(event => {
        const start = event.start ? new Date(event.start) : null;
        const end = event.end ? new Date(event.end) : null;
        const subject = event.subject || 'Evento';
        const timeLabel = start
            ? `${start.toLocaleDateString()} ${start.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
            : 'data sconosciuta';
        const endLabel = end
            ? ` → ${end.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
            : '';
        lines.push(`• ${subject} (${timeLabel}${endLabel})`);
    });

    return lines.join('\n');
}

function buildGoogleEventsSummary(events) {
    if (!Array.isArray(events) || events.length === 0) {
        return 'Non ho trovato eventi futuri su Google Calendar.';
    }

    const lines = ['Ecco i prossimi impegni presenti su Google Calendar:'];
    events.slice(0, 10).forEach(event => {
        const start = event.start ? new Date(event.start) : null;
        const end = event.end ? new Date(event.end) : null;
        const subject = event.subject || 'Evento';
        const timeLabel = start
            ? `${start.toLocaleDateString()} ${start.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
            : 'data sconosciuta';
        const endLabel = end
            ? ` → ${end.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
            : '';
        lines.push(`• ${subject} (${timeLabel}${endLabel})`);
    });

    return lines.join('\n');
}

async function startOutlookDeviceFlowWithPrompt() {
    try {
        const flow = await invoke('start_outlook_calendar_device_flow');
        state.pendingIntegrationStep = null;
        await refreshCalendarIntegrationsStatus({ silent: true });
        const expiresLabel = flow.expires_at
            ? `Il link scade alle ${new Date(flow.expires_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}.`
            : '';
        const link = flow.verification_uri
            ? `[Clicca qui per autorizzare il collegamento a Outlook Calendar](${flow.verification_uri})`
            : 'Non ho ricevuto un link di autorizzazione da Microsoft.';
        const codeFallback = flow.user_code
            ? `Se Microsoft ti chiede un codice, usa: \`${flow.user_code}\`.`
            : '';

        addAssistantResponse([
            link,
            expiresLabel,
            codeFallback,
            'Dopo aver completato l’autorizzazione nel browser, qui confermerò automaticamente il collegamento.'
        ].filter(Boolean).join('\n'));
        showCalendarStatus('In attesa di autorizzazione Outlook');
        scheduleOutlookPoll(flow.interval || 5);
        return true;
    } catch (error) {
        console.error('Impossibile avviare il device flow Outlook:', error);
        showCalendarStatus('Errore nell\'avvio del collegamento Outlook', true);
        addAssistantResponse('Non riesco ad avviare il collegamento Outlook. Controlla il Client ID e riprova.');
        return true;
    }
}

async function startGoogleDeviceFlowWithPrompt() {
    try {
        const flow = await invoke('start_google_calendar_device_flow');
        state.pendingIntegrationStep = null;
        await refreshCalendarIntegrationsStatus({ silent: true });
        const expiresLabel = flow.expires_at
            ? `Il link scade alle ${new Date(flow.expires_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}.`
            : '';
        const link = flow.verification_uri
            ? `[Clicca qui per autorizzare il collegamento a Google Calendar](${flow.verification_uri})`
            : 'Non ho ricevuto un link di autorizzazione da Google.';
        const codeFallback = flow.user_code
            ? `Se Google ti chiede un codice, usa: \`${flow.user_code}\`.`
            : '';

        addAssistantResponse([
            link,
            expiresLabel,
            codeFallback,
            'Dopo aver completato l’autorizzazione nel browser, qui confermerò automaticamente il collegamento.'
        ].filter(Boolean).join('\n'));
        showCalendarStatus('In attesa di autorizzazione Google Calendar');
        scheduleGooglePoll(flow.interval || 5);
        return true;
    } catch (error) {
        console.error('Impossibile avviare il device flow Google Calendar:', error);
        showCalendarStatus('Errore nell\'avvio del collegamento Google Calendar', true);
        addAssistantResponse('Non riesco ad avviare il collegamento Google Calendar. Controlla il Client ID e riprova.');
        return true;
    }
}

async function handleCalendarIntegrationCommand(text) {
    if (!text || typeof text !== 'string') {
        return false;
    }

    const normalized = normalizeTextForMatch(text);

    if (state.pendingIntegrationStep === 'outlook_credentials') {
        const parsed = parseOutlookCredentials(text);
        if (!parsed) {
            addAssistantResponse('Per collegare Outlook inviami un messaggio nel formato: "Client Outlook: <client_id> Tenant: <tenant facoltativo>".');
            return true;
        }

        try {
            await invoke('set_outlook_calendar_credentials', {
                clientId: parsed.clientId,
                tenant: parsed.tenant,
            });
            addAssistantResponse('Credenziali Outlook salvate. Avvio la procedura di collegamento...');
            return await startOutlookDeviceFlowWithPrompt();
        } catch (error) {
            console.error('Errore salvataggio credenziali Outlook:', error);
            showCalendarStatus('Credenziali Outlook non valide', true);
            addAssistantResponse('Non riesco a salvare le credenziali Outlook. Controlla Client ID e tenant.');
            return true;
        }
    }

    if (state.pendingIntegrationStep === 'google_credentials') {
        const parsed = parseGoogleCredentials(text);
        if (!parsed) {
            addAssistantResponse('Per collegare Google Calendar inviami un messaggio nel formato: "Client Google: <client_id> CalendarId: <facoltativo>".');
            return true;
        }

        try {
            await invoke('set_google_calendar_credentials', {
                clientId: parsed.clientId,
                clientSecret: parsed.clientSecret || null,
                calendarId: parsed.calendarId,
            });
            addAssistantResponse('Credenziali Google salvate. Avvio la procedura di collegamento...');
            return await startGoogleDeviceFlowWithPrompt();
        } catch (error) {
            console.error('Errore salvataggio credenziali Google:', error);
            showCalendarStatus('Credenziali Google non valide', true);
            addAssistantResponse('Non riesco a salvare le credenziali Google. Controlla il Client ID e riprova.');
            return true;
        }
    }

    const mentionsOutlook = normalized.includes('outlook');
    const mentionsGoogle = normalized.includes('google') || normalized.includes('gcal');

    if (!mentionsOutlook && !mentionsGoogle) {
        return false;
    }

    if (mentionsGoogle) {
        if (/disconnetti|scollega|rimuovi|dimentica/.test(normalized)) {
            try {
                clearGooglePollTimer();
                await invoke('disconnect_google_calendar');
                await refreshCalendarIntegrationsStatus({ silent: true });
                addAssistantResponse('Ho disconnesso Google Calendar dal calendario.');
            } catch (error) {
                console.error('Errore durante la disconnessione di Google Calendar:', error);
                showCalendarStatus('Impossibile disconnettere Google Calendar', true);
                addAssistantResponse('Non sono riuscito a disconnettere Google Calendar. Riprova più tardi.');
            }
            return true;
        }

        if (/mostra|lista|elenca|dammi|visualizza/.test(normalized) && /event/i.test(normalized)) {
            await refreshCalendarIntegrationsStatus({ silent: true });
            if (!state.integrations.google.connected) {
                addAssistantResponse('Google Calendar non risulta collegato. Puoi chiedermi "Collega Google Calendar" per avviare la procedura.');
                return true;
            }
            try {
                const events = await invoke('list_google_calendar_events', { limit: 10 });
                addAssistantResponse(buildGoogleEventsSummary(events));
            } catch (error) {
                console.error('Errore durante il recupero degli eventi Google Calendar:', error);
                showCalendarStatus('Impossibile ottenere gli eventi Google Calendar', true);
                addAssistantResponse('Non riesco a leggere gli eventi da Google Calendar in questo momento.');
            }
            return true;
        }

        if (/collega|connetti|configura|autorizza|sincronizza/.test(normalized)) {
            const status = await refreshCalendarIntegrationsStatus({ silent: true });
            if (!status || !status.google) {
                addAssistantResponse('Non riesco a verificare lo stato di Google Calendar. Riprova più tardi.');
                return true;
            }

            if (!state.integrations.google.configured) {
                state.pendingIntegrationStep = 'google_credentials';
                if (!state.integrationPrompts.googleCredentials) {
                    state.integrationPrompts.googleCredentials = true;
                    addAssistantResponse('Prima posso generare il link di autorizzazione, ma devo configurare Google Calendar (una tantum) con il Client ID. Inviami: "Client Google: <client_id> CalendarId: <facoltativo>".');
                } else {
                    addAssistantResponse('Mi serve ancora il Client ID di Google per poter generare il link di autorizzazione.');
                }
                return true;
            }

            if (state.integrations.google.connected) {
                addAssistantResponse('Google Calendar risulta già collegato. Posso comunque mostrare o sincronizzare gli eventi se necessario.');
                return true;
            }

            if (state.integrations.google.pending) {
                const link = state.integrations.google.verificationUri
                    ? `[Clicca qui per autorizzare Google Calendar](${state.integrations.google.verificationUri})`
                    : (state.integrations.google.message || 'Attendo che tu autorizzi la connessione su Google.');
                const codeFallback = state.integrations.google.userCode
                    ? `Se Google ti chiede un codice, usa: \`${state.integrations.google.userCode}\`.`
                    : '';
                addAssistantResponse([link, codeFallback].filter(Boolean).join('\n'));
                scheduleGooglePoll(state.integrations.google.interval || 5);
                return true;
            }

            return await startGoogleDeviceFlowWithPrompt();
        }

        if (/stato|verifica|controlla/.test(normalized)) {
            const status = await refreshCalendarIntegrationsStatus({ silent: true });
            if (!status || !status.google) {
                addAssistantResponse('Non riesco a ottenere lo stato attuale di Google Calendar.');
                return true;
            }

            if (state.integrations.google.connected) {
                addAssistantResponse('Google Calendar è collegato e pronto per sincronizzare il calendario.');
            } else if (state.integrations.google.pending) {
                const link = state.integrations.google.verificationUri
                    ? `[Clicca qui per autorizzare Google Calendar](${state.integrations.google.verificationUri})`
                    : 'Sto ancora attendendo l\'autorizzazione su Google.';
                const codeFallback = state.integrations.google.userCode
                    ? `Se Google ti chiede un codice, usa: \`${state.integrations.google.userCode}\`.`
                    : '';
                addAssistantResponse([link, codeFallback].filter(Boolean).join('\n'));
            } else if (state.integrations.google.configured) {
                addAssistantResponse('Ho le credenziali ma Google Calendar non è ancora autorizzato. Scrivi "Collega Google Calendar" per avviare il codice di autorizzazione.');
            } else {
                addAssistantResponse('Google Calendar non è configurato. Forniscimi il Client ID per procedere.');
            }
            return true;
        }

        if (normalized.includes('client') && (normalized.includes('google') || normalized.includes('gcal') || normalized.includes('secret'))) {
            const parsed = parseGoogleCredentials(text);
            if (!parsed) {
                addAssistantResponse('Non ho riconosciuto il formato delle credenziali. Usa: "Client Google: <client_id> CalendarId: <facoltativo>".');
                return true;
            }

            try {
                await invoke('set_google_calendar_credentials', {
                    clientId: parsed.clientId,
                    clientSecret: parsed.clientSecret || null,
                    calendarId: parsed.calendarId,
                });
                addAssistantResponse('Ho aggiornato le credenziali Google. Avvio ora il collegamento.');
                return await startGoogleDeviceFlowWithPrompt();
            } catch (error) {
                console.error('Errore aggiornamento credenziali Google:', error);
                showCalendarStatus('Credenziali Google non valide', true);
                addAssistantResponse('Le credenziali Google non sono valide. Controlla il Client ID.');
                return true;
            }
        }

        return true;
    }

    // Outlook flow (invariato)
    if (/disconnetti|scollega|rimuovi|dimentica/.test(normalized)) {
        try {
            clearOutlookPollTimer();
            await invoke('disconnect_outlook_calendar');
            await refreshCalendarIntegrationsStatus({ silent: true });
            addAssistantResponse('Ho disconnesso Outlook dal calendario.');
        } catch (error) {
            console.error('Errore durante la disconnessione di Outlook:', error);
            showCalendarStatus('Impossibile disconnettere Outlook', true);
            addAssistantResponse('Non sono riuscito a disconnettere Outlook. Riprova più tardi.');
        }
        return true;
    }

    if (/mostra|lista|elenca|dammi|visualizza/.test(normalized) && /event/i.test(normalized)) {
        await refreshCalendarIntegrationsStatus({ silent: true });
        if (!state.integrations.outlook.connected) {
            addAssistantResponse('Outlook non risulta collegato. Puoi chiedermi "Collega Outlook" per avviare la procedura.');
            return true;
        }
        try {
            const events = await invoke('list_outlook_calendar_events', { limit: 10 });
            addAssistantResponse(buildOutlookEventsSummary(events));
        } catch (error) {
            console.error('Errore durante il recupero degli eventi Outlook:', error);
            showCalendarStatus('Impossibile ottenere gli eventi Outlook', true);
            addAssistantResponse('Non riesco a leggere gli eventi da Outlook in questo momento.');
        }
        return true;
    }

    if (/collega|connetti|configura|autorizza|sincronizza/.test(normalized)) {
        const status = await refreshCalendarIntegrationsStatus({ silent: true });
        if (!status || !status.outlook) {
            addAssistantResponse('Non riesco a verificare lo stato di Outlook. Riprova più tardi.');
            return true;
        }

        if (!state.integrations.outlook.configured) {
            state.pendingIntegrationStep = 'outlook_credentials';
            addAssistantResponse('Per collegare Outlook ho bisogno del Client ID dell\'app registrata e, se necessario, del tenant. Inviami un messaggio del tipo "Client Outlook: <client_id> Tenant: <tenant>".');
            return true;
        }

        if (state.integrations.outlook.connected) {
            addAssistantResponse('Outlook risulta già collegato. Posso comunque mostrare o sincronizzare gli eventi se necessario.');
            return true;
        }

        if (state.integrations.outlook.pending) {
            const link = state.integrations.outlook.verificationUri
                ? `[Clicca qui per autorizzare Outlook Calendar](${state.integrations.outlook.verificationUri})`
                : (state.integrations.outlook.message || 'Attendo che tu autorizzi la connessione su Microsoft.');
            const codeFallback = state.integrations.outlook.userCode
                ? `Se Microsoft ti chiede un codice, usa: \`${state.integrations.outlook.userCode}\`.`
                : '';
            addAssistantResponse([link, codeFallback].filter(Boolean).join('\n'));
            scheduleOutlookPoll(state.integrations.outlook.interval || 5);
            return true;
        }

        return await startOutlookDeviceFlowWithPrompt();
    }

    if (/stato|verifica|controlla/.test(normalized)) {
        const status = await refreshCalendarIntegrationsStatus({ silent: true });
        if (!status || !status.outlook) {
            addAssistantResponse('Non riesco a ottenere lo stato attuale di Outlook.');
            return true;
        }

        if (state.integrations.outlook.connected) {
            addAssistantResponse('Outlook è collegato e pronto per sincronizzare il calendario.');
        } else if (state.integrations.outlook.pending) {
            const link = state.integrations.outlook.verificationUri
                ? `[Clicca qui per autorizzare Outlook Calendar](${state.integrations.outlook.verificationUri})`
                : 'Sto ancora attendendo l\'autorizzazione su Outlook.';
            const codeFallback = state.integrations.outlook.userCode
                ? `Se Microsoft ti chiede un codice, usa: \`${state.integrations.outlook.userCode}\`.`
                : '';
            addAssistantResponse([link, codeFallback].filter(Boolean).join('\n'));
        } else if (state.integrations.outlook.configured) {
            addAssistantResponse('Ho le credenziali ma Outlook non è ancora autorizzato. Scrivi "Collega Outlook" per avviare il codice di autorizzazione.');
        } else {
            addAssistantResponse('Outlook non è configurato. Forniscimi Client ID e tenant per procedere.');
        }
        return true;
    }

    if (normalized.includes('client') && normalized.includes('tenant')) {
        const parsed = parseOutlookCredentials(text);
        if (!parsed) {
            addAssistantResponse('Non ho riconosciuto il formato delle credenziali. Usa: "Client Outlook: <client_id> Tenant: <tenant>".');
            return true;
        }

        try {
            await invoke('set_outlook_calendar_credentials', {
                clientId: parsed.clientId,
                tenant: parsed.tenant,
            });
            addAssistantResponse('Ho aggiornato le credenziali Outlook. Avvio ora il collegamento.');
            return await startOutlookDeviceFlowWithPrompt();
        } catch (error) {
            console.error('Errore aggiornamento credenziali Outlook:', error);
            showCalendarStatus('Credenziali Outlook non valide', true);
            addAssistantResponse('Le credenziali Outlook non sono valide. Controlla Client ID e tenant.');
            return true;
        }
    }

    return false;
}

function normalizeTitleForComparison(title) {
    return (title || '')
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, ' ')
        .trim();
}

function formatEventDateRange(startIso, endIso) {
    const start = new Date(startIso);
    const end = endIso ? new Date(endIso) : new Date(start.getTime() + 60 * 60 * 1000);
    const sameDay = start.toDateString() === end.toDateString();
    const dateOptions = { day: '2-digit', month: 'short', year: 'numeric' };
    const timeOptions = { hour: '2-digit', minute: '2-digit' };

    const datePart = start.toLocaleDateString(undefined, dateOptions);
    if (sameDay) {
        const startTime = start.toLocaleTimeString(undefined, timeOptions);
        const endTime = end.toLocaleTimeString(undefined, timeOptions);
        return `${datePart} · ${startTime} - ${endTime}`;
    }

    const endDatePart = end.toLocaleDateString(undefined, dateOptions);
    return `${datePart} → ${endDatePart}`;
}

function renderCalendarList() {
    if (!elements.calendarList) return;

    const events = Array.isArray(state.calendarEvents)
        ? [...state.calendarEvents].sort((a, b) => new Date(a.start) - new Date(b.start))
        : [];

    if (elements.clearCalendarBtn) {
        elements.clearCalendarBtn.disabled = events.length === 0;
    }
    if (elements.exportCalendarBtn) {
        elements.exportCalendarBtn.disabled = events.length === 0;
    }

    if (events.length === 0) {
        elements.calendarList.innerHTML = `
            <div class="empty-calendar">
                <p>Nessun evento registrato</p>
                <small>Quando segnali un impegno, verrà aggiunto automaticamente qui</small>
            </div>
        `;
        return;
    }

    elements.calendarList.innerHTML = events
        .map(event => {
            const description = (event.description || '').trim();
            return `
                <div class="calendar-event" data-id="${event.id}">
                    <div class="calendar-event-header">
                        <div class="calendar-event-title">${escapeHtml(event.title)}</div>
                        <div class="calendar-event-date">${escapeHtml(formatEventDateRange(event.start, event.end))}</div>
                    </div>
                    ${description ? `<div class="calendar-event-details">${escapeHtml(description)}</div>` : ''}
                    <div class="calendar-event-actions">
                        <button class="calendar-event-delete" data-id="${event.id}" title="Elimina evento">Elimina</button>
                    </div>
                </div>
            `;
        })
        .join('');

    elements.calendarList.querySelectorAll('.calendar-event-delete').forEach(btn => {
        btn.addEventListener('click', async (event) => {
            event.stopPropagation();
            const id = btn.dataset.id;
            if (!id) return;
            if (!confirm('Eliminare questo evento?')) return;
            await deleteCalendarEventById(id);
        });
    });
}

async function loadCalendarEventsFromStore() {
    try {
        const events = await invoke('load_calendar_events');
        state.calendarEvents = Array.isArray(events) ? events : [];
    } catch (error) {
        console.warn('Impossibile caricare il calendario:', error);
        state.calendarEvents = [];
        showCalendarStatus('Errore durante il caricamento del calendario', true);
    }

    renderCalendarList();
}

async function deleteCalendarEventById(id) {
    try {
        await invoke('delete_calendar_event', { id });
        await loadCalendarEventsFromStore();
        showCalendarStatus('Evento eliminato');
    } catch (error) {
        console.warn('Impossibile eliminare l\'evento:', error);
        showCalendarStatus('Errore durante l\'eliminazione', true);
    }
}

async function clearAllCalendarEvents() {
    if (!confirm('Sei sicuro di voler eliminare tutti gli eventi del calendario?')) {
        return;
    }

    try {
        await invoke('clear_calendar_events');
        state.calendarEvents = [];
        renderCalendarList();
        showCalendarStatus('Calendario svuotato');
    } catch (error) {
        console.warn('Impossibile svuotare il calendario:', error);
        showCalendarStatus('Errore durante lo svuotamento', true);
    }
}

async function exportCalendarAsIcs() {
    try {
        const path = await invoke('export_calendar_to_ics');
        showCalendarStatus(`Calendario esportato in ${path}`);
    } catch (error) {
        console.warn('Impossibile esportare il calendario:', error);
        showCalendarStatus('Errore durante l\'esportazione', true);
    }
}

function getNextWeekday(baseDate, targetWeekdayIndex) {
    const date = new Date(baseDate.getTime());
    const currentIndex = date.getDay();
    let diff = targetWeekdayIndex - currentIndex;
    if (diff <= 0) {
        diff += 7;
    }
    date.setDate(date.getDate() + diff);
    return date;
}

function parseFullDate(day, monthIndex, year, reference) {
    const date = new Date(reference.getTime());
    date.setHours(9, 0, 0, 0);
    date.setMonth(monthIndex);
    date.setDate(day);
    if (typeof year === 'number') {
        const normalizedYear = year < 100 ? 2000 + year : year;
        date.setFullYear(normalizedYear);
    }

    if (typeof year !== 'number' && date < reference) {
        date.setFullYear(date.getFullYear() + 1);
    }

    if (date < new Date(reference.getTime() - 24 * 60 * 60 * 1000)) {
        return null;
    }

    return date;
}

function applyTimeHints(date, sentence) {
    if (!date) return null;

    const timeMatch = sentence.match(/(?:alle|ore|h)\s*(\d{1,2})(?:[:.](\d{2}))?/i);
    if (timeMatch) {
        const hours = Math.min(23, parseInt(timeMatch[1], 10));
        const minutes = timeMatch[2] ? Math.min(59, parseInt(timeMatch[2], 10)) : 0;
        date.setHours(hours, minutes, 0, 0);
        return date;
    }

    if (/mattin/i.test(sentence)) {
        date.setHours(9, 0, 0, 0);
    } else if (/pomeriggio/i.test(sentence)) {
        date.setHours(15, 0, 0, 0);
    } else if (/sera|stasera/i.test(sentence)) {
        date.setHours(19, 0, 0, 0);
    } else if (/notte|stanotte/i.test(sentence)) {
        date.setHours(22, 0, 0, 0);
    } else {
        date.setHours(9, 0, 0, 0);
    }

    return date;
}

function extractDateFromSentence(sentence, reference) {
    const normalized = sentence.toLowerCase();
    const accentless = normalized.normalize('NFD').replace(/[\u0300-\u036f]/g, '');
    let date = null;

    for (const [keyword, offset] of Object.entries(RELATIVE_KEYWORDS)) {
        if (accentless.includes(keyword)) {
            const candidate = new Date(reference.getTime());
            candidate.setDate(candidate.getDate() + offset);
            date = candidate;
            break;
        }
    }

    if (!date) {
        WEEK_DAYS_IT.forEach((weekday, index) => {
            if (date) return;
            const accented = weekday;
            const plain = weekday.normalize('NFD').replace(/[\u0300-\u036f]/g, '');
            if (normalized.includes(accented) || accentless.includes(plain)) {
                date = getNextWeekday(reference, index);
            }
        });
    }

    if (!date) {
        const monthRegex = /(?:il\s+)?(\d{1,2})\s+(gennaio|febbraio|marzo|aprile|maggio|giugno|luglio|agosto|settembre|ottobre|novembre|dicembre)(?:\s+(\d{2,4}))?/i;
        const match = sentence.match(monthRegex);
        if (match) {
            const day = parseInt(match[1], 10);
            const monthName = match[2].toLowerCase();
            const year = match[3] ? parseInt(match[3], 10) : undefined;
            const monthIndex = MONTHS_IT[monthName];
            if (Number.isInteger(monthIndex)) {
                date = parseFullDate(day, monthIndex, year, reference);
            }
        }
    }

    if (!date) {
        const numericRegex = /(?:il\s+)?(\d{1,2})[\/.\-](\d{1,2})(?:[\/.\-](\d{2,4}))?/;
        const match = sentence.match(numericRegex);
        if (match) {
            const day = parseInt(match[1], 10);
            const month = parseInt(match[2], 10) - 1;
            const year = match[3] ? parseInt(match[3], 10) : undefined;
            date = parseFullDate(day, month, year, reference);
        }
    }

    if (date) {
        return applyTimeHints(date, sentence);
    }

    return null;
}

function deriveEventTitle(sentence) {
    const cleaned = sentence
        .replace(/(?:io|noi)\s+/, '')
        .replace(/(?:devo|dovr[oò]|avr[oò]|ho|ci sarà|c'?è)\s+/i, '')
        .trim();
    const noTrailing = cleaned.replace(/[.?!]+$/, '').trim();
    if (noTrailing.length <= 0) {
        return 'Impegno personale';
    }
    if (noTrailing.length <= 60) {
        return noTrailing.charAt(0).toUpperCase() + noTrailing.slice(1);
    }
    return `${noTrailing.slice(0, 57).trimEnd()}…`;
}

function detectCalendarEntries(text) {
    if (!text || typeof text !== 'string') return [];

    const reference = new Date();
    const sentences = text
        .split(/[.!?\n]+/)
        .map(sentence => sentence.trim())
        .filter(Boolean);

    const triggers = /(ho|avr[oò]|devo|parteciper[oò]|partecipo|vado|ci sara|ci sarà|organizzo|ricordami|programma|prenotato|inizia|comincia|allenamento|gara|riunione)/i;

    const candidates = [];

    sentences.forEach(sentence => {
        if (!triggers.test(sentence)) return;
        const date = extractDateFromSentence(sentence, reference);
        if (!date) return;

        const start = new Date(date.getTime());
        const end = new Date(date.getTime() + 60 * 60 * 1000);

        candidates.push({
            title: deriveEventTitle(sentence),
            description: sentence,
            startIso: start.toISOString(),
            endIso: end.toISOString(),
            source: sentence,
        });
    });

    return candidates;
}

function isDuplicateCalendarEvent(candidate) {
    const candidateTitle = normalizeTitleForComparison(candidate.title);
    const candidateTime = new Date(candidate.startIso).getTime();

    return state.calendarEvents.some(event => {
        const storedTitle = normalizeTitleForComparison(event.title);
        if (storedTitle !== candidateTitle) {
            return false;
        }
        const storedTime = new Date(event.start).getTime();
        const diffMinutes = Math.abs(storedTime - candidateTime) / (60 * 1000);
        return diffMinutes <= 90;
    });
}

async function syncEventToIntegrations(eventId) {
    if (!eventId) return;
    const hasOutlook = Boolean(state.integrations.outlook?.connected);
    const hasGoogle = Boolean(state.integrations.google?.connected);
    if (!hasOutlook && !hasGoogle) return;

    try {
        await invoke('sync_calendar_event_to_integrations', { id: eventId });
    } catch (error) {
        console.warn('Impossibile sincronizzare l\'evento con le integrazioni calendario:', error);
        showCalendarStatus('Evento salvato localmente ma non sincronizzato con il calendario remoto', true);
    }
}

async function autoCaptureCalendarEvents(text) {
    if (!text || text.length < 6) return;

    const candidates = detectCalendarEntries(text);
    if (candidates.length === 0) return;

    let added = 0;

    for (const candidate of candidates) {
        if (isDuplicateCalendarEvent(candidate)) {
            continue;
        }

        try {
            const eventId = await invoke('add_calendar_event', {
                event: {
                    title: candidate.title,
                    description: candidate.description,
                    start: candidate.startIso,
                    end: candidate.endIso,
                    source_text: candidate.source,
                },
            });
            added += 1;
            await syncEventToIntegrations(eventId);
        } catch (error) {
            console.warn('Impossibile aggiungere evento al calendario:', error);
            showCalendarStatus('Errore durante il salvataggio di un evento', true);
        }
    }

    if (added > 0) {
        await loadCalendarEventsFromStore();
        showCalendarStatus(added === 1 ? 'Aggiunto 1 evento al calendario' : `Aggiunti ${added} eventi al calendario`);
    }
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

function addAssistantMessage(content) {
    if (!content) return;
    addMessage('assistant', content, getTimestamp());
}

function addAssistantResponse(content) {
    if (!content) return;
    addAssistantMessage(content);
    state.conversation.push({
        role: 'assistant',
        content,
        hidden: false,
    });
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
        
        // Add custom system prompt if enabled
        if (state.customSystemPrompt.enabled && state.customSystemPrompt.content.trim()) {
            systemContent += '\n\n**ISTRUZIONI PERSONALIZZATE DELL\'UTENTE:**\n' + state.customSystemPrompt.content.trim();
        }
        
        if (state.agentMode) {
            const toolsDesc = await getToolsDescription();
            systemContent += '\n\n' + toolsDesc;
            systemContent += '\n\n**LINEE GUIDA:**\n- Usa i tool appropriati per le richieste dell\'utente.\n- Se la risposta richiede dati aggiornati o verifiche, esegui `web_search` e integra solo fonti considerate affidabili.\n- Quando ricevi note di ricerca dal backend, trattale come riferimenti da citare in formato [Titolo](URL) indicando il dominio.\n- Riassumi con parole tue e segnala eventuali incongruenze o assenza di dati aggiornati.';
        }

        if (!state.memoryContextInjected && state.memoryContext) {
            systemContent += '\n\n**MEMORIA UTENTE STORICA:**\n' + state.memoryContext + '\nUtilizza queste note per mantenere coerenza con preferenze, tono e conoscenze condivise. Evita di ripetere le frasi letteralmente, ma rispondi tenendo conto di queste informazioni.';
            state.memoryContextInjected = true;
        }

        if (!state.memoryContextInjected) {
            state.memoryContextInjected = true;
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

    if (state.agentMode) {
        const newsQuery = detectNewsQuery(text);
        if (newsQuery) {
            const safeQuery = newsQuery.replace(/"/g, "'");
            state.conversation.push({
                role: 'user',
                content: `PROMEMORIA AGENTE: L'utente sta chiedendo notizie o eventi attuali. Esegui il tool web_search con la query "${safeQuery}" (usa max_results=5) prima di rispondere. Riassumi i risultati aggiornati in italiano e cita le fonti principali usando collegamenti Markdown [Titolo](URL) e indicando il dominio della fonte.`,
                hidden: true,
            });
        }
    }

    await saveCurrentConversation({ force: true });
    await autoCaptureCalendarEvents(text);

    const handledIntegration = await handleCalendarIntegrationCommand(text);
    if (handledIntegration) {
        await saveCurrentConversation({ force: true });
        if (text) {
            state.messageHistory.push(text);
            state.messageHistoryIndex = -1;
        }
        elements.messageInput.value = '';
        state.attachedFiles = [];
        updateAttachedFiles();
        updateSendButton();
        state.isProcessing = false;
        return;
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
        
        // Check for tool calls if agent mode is enabled
        if (state.agentMode) {
            const toolCalls = await invoke('parse_tool_calls', { response: response.content });
            
            if (toolCalls.length > 0) {
                state.pendingToolCalls = toolCalls;
                await processNextToolCall();
            } else {
                // Save conversation if no more tool calls
                await saveCurrentConversation();
            }
        } else {
            // Save conversation in non-agent mode
            await saveCurrentConversation();
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
        updateMemoryContext();
    } catch (error) {
        console.warn('Impossibile caricare la memoria:', error);
        state.memoryConversations = [];
        state.memoryContext = '';
    }
}

async function saveCurrentConversation(options = {}) {
    const { force = false } = options;

    // Only save if there are visible messages (unless forced)
    const visibleMessages = state.conversation.filter(m => !m.hidden);
    if (!force && visibleMessages.length < 2) return;
    if (force && visibleMessages.length === 0) return;
    
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
    state.memoryContextInjected = true;
    state.memoryContext = buildMemoryContext();
    
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
    state.memoryContextInjected = false;
    state.memoryContext = buildMemoryContext();
    
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
    state.memoryContextInjected = false;
    state.memoryContext = buildMemoryContext();
    
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
    if (elements.clearCalendarBtn) {
        elements.clearCalendarBtn.addEventListener('click', clearAllCalendarEvents);
    }
    if (elements.exportCalendarBtn) {
        elements.exportCalendarBtn.addEventListener('click', exportCalendarAsIcs);
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
    elements.agentModeToggle.checked = state.agentMode;
    updateIterationCounter();
    await loadVersionIndicator();
    await loadGreeting();
    await loadSettings();
    await loadMemory();
    await loadCalendarEventsFromStore();
    await refreshCalendarIntegrationsStatus({ silent: true });
    renderHistoryList();
    checkForUpdates();
    await scanNetwork();
}

// Start the app
document.addEventListener('DOMContentLoaded', init);
