const DEFAULT_PORT = 7331;
const ONBOARDING_TARGETS = [
  { id: "codex", label: "Codex" },
  { id: "claude-code", label: "Claude Code" },
  { id: "generic-openai", label: "Generic OpenAI" },
  { id: "generic-anthropic", label: "Generic Anthropic" },
  { id: "curl", label: "cURL / Manual" },
];

const state = {
  daemonStatus: null,
  health: null,
  providers: [],
  accounts: [],
  routes: [],
  monitor: [],
  logs: [],
  dashboardLogs: [],
  onboarding: [],
  activeTab: "dashboard",
  accountProviderFilter: "all",
  onboardingTarget: "codex",
  onboardingProvider: null,
  providerEditor: null,
  accountEditor: null,
  routeEditor: null,
  revealedSecret: "",
};
let pendingConfirmation = null;

const elements = {
  daemonChip: document.querySelector("#daemon-chip"),
  vaultState: document.querySelector("#vault-state"),
  dbPath: document.querySelector("#db-path"),
  daemonPort: document.querySelector("#daemon-port"),
  startedAt: document.querySelector("#started-at"),
  detailStatus: document.querySelector("#detail-status"),
  detailPid: document.querySelector("#detail-pid"),
  daemonLogPath: document.querySelector("#daemon-log-path"),
  daemonLastExit: document.querySelector("#daemon-last-exit"),
  daemonLastError: document.querySelector("#daemon-last-error"),
  detailsButton: document.querySelector("#details-button"),
  detailsPanel: document.querySelector("#details-panel"),
  startDaemonButton: document.querySelector("#start-daemon-button"),
  stopDaemonButton: document.querySelector("#stop-daemon-button"),
  restartDaemonButton: document.querySelector("#restart-daemon-button"),
  openDaemonLogButton: document.querySelector("#open-daemon-log-button"),
  unlockForm: document.querySelector("#unlock-form"),
  masterPassword: document.querySelector("#master-password"),
  unlockSubmit: document.querySelector("#unlock-submit"),
  vaultActionButton: document.querySelector("#vault-action-button"),
  vaultDialog: document.querySelector("#vault-dialog"),
  vaultDialogTitle: document.querySelector("#vault-dialog-title"),
  vaultDialogCopy: document.querySelector("#vault-dialog-copy"),
  closeVaultDialog: document.querySelector("#close-vault-dialog"),
  tabButtons: Array.from(document.querySelectorAll(".tab-button")),
  tabPanels: Array.from(document.querySelectorAll(".tab-panel")),
  metricsList: document.querySelector("#metrics-list"),
  routeSummaryList: document.querySelector("#route-summary-list"),
  recentActivityList: document.querySelector("#recent-activity-list"),
  dashboardHealthList: document.querySelector("#dashboard-health-list"),
  onboardingList: document.querySelector("#onboarding-list"),
  onboardingTargetTabs: document.querySelector("#onboarding-target-tabs"),
  onboardingProviderTabs: document.querySelector("#onboarding-provider-tabs"),
  refreshOnboarding: document.querySelector("#refresh-onboarding"),
  openProviderDialog: document.querySelector("#open-provider-dialog"),
  providerDialog: document.querySelector("#provider-dialog"),
  closeProviderDialog: document.querySelector("#close-provider-dialog"),
  providerForm: document.querySelector("#provider-form"),
  providerFormTitle: document.querySelector("#provider-form-title"),
  providerFormCopy: document.querySelector("#provider-form-copy"),
  providerSubmit: document.querySelector("#provider-submit"),
  providerSlug: document.querySelector("#provider-slug"),
  providerName: document.querySelector("#provider-name"),
  providerProtocol: document.querySelector("#provider-protocol"),
  providerBaseUrl: document.querySelector("#provider-base-url"),
  providerPath: document.querySelector("#provider-path"),
  providerPathDemo: document.querySelector("#provider-path-demo"),
  providerAuthHeader: document.querySelector("#provider-auth-header"),
  providerAuthPrefix: document.querySelector("#provider-auth-prefix"),
  providerEnabled: document.querySelector("#provider-enabled"),
  providersList: document.querySelector("#providers-list"),
  openAccountDialog: document.querySelector("#open-account-dialog"),
  accountDialog: document.querySelector("#account-dialog"),
  closeAccountDialog: document.querySelector("#close-account-dialog"),
  accountForm: document.querySelector("#account-form"),
  accountFormTitle: document.querySelector("#account-form-title"),
  accountFormCopy: document.querySelector("#account-form-copy"),
  accountSubmit: document.querySelector("#account-submit"),
  accountId: document.querySelector("#account-id"),
  accountProvider: document.querySelector("#account-provider"),
  accountName: document.querySelector("#account-name"),
  accountApiKey: document.querySelector("#account-api-key"),
  accountSecretTools: document.querySelector("#account-secret-tools"),
  accountSecretCopy: document.querySelector("#account-secret-copy"),
  accountSecretToggle: document.querySelector("#account-secret-toggle"),
  accountSecretPanel: document.querySelector("#account-secret-panel"),
  accountSecretPassword: document.querySelector("#account-secret-password"),
  accountSecretReveal: document.querySelector("#account-secret-reveal"),
  accountSecretValue: document.querySelector("#account-secret-value"),
  accountSecretCopyButton: document.querySelector("#account-secret-copy-button"),
  accountBaseUrl: document.querySelector("#account-base-url"),
  accountNote: document.querySelector("#account-note"),
  accountEnabled: document.querySelector("#account-enabled"),
  accountsProviderTabs: document.querySelector("#accounts-provider-tabs"),
  accountsList: document.querySelector("#accounts-list"),
  openRouteDialog: document.querySelector("#open-route-dialog"),
  routeDialog: document.querySelector("#route-dialog"),
  closeRouteDialog: document.querySelector("#close-route-dialog"),
  routeForm: document.querySelector("#route-form"),
  routeFormTitle: document.querySelector("#route-form-title"),
  routeFormCopy: document.querySelector("#route-form-copy"),
  routeProvider: document.querySelector("#route-provider"),
  routePrefix: document.querySelector("#route-prefix"),
  routeAccount: document.querySelector("#route-account"),
  routeHint: document.querySelector("#route-hint"),
  routeSubmit: document.querySelector("#route-submit"),
  routesList: document.querySelector("#routes-list"),
  refreshRoutes: document.querySelector("#refresh-routes"),
  confirmDialog: document.querySelector("#confirm-dialog"),
  closeConfirmDialog: document.querySelector("#close-confirm-dialog"),
  confirmForm: document.querySelector("#confirm-form"),
  confirmDialogTitle: document.querySelector("#confirm-dialog-title"),
  confirmDialogCopy: document.querySelector("#confirm-dialog-copy"),
  confirmCancel: document.querySelector("#confirm-cancel"),
  confirmSubmit: document.querySelector("#confirm-submit"),
  monitorFilterForm: document.querySelector("#monitor-filter-form"),
  monitorProvider: document.querySelector("#monitor-provider"),
  monitorAccount: document.querySelector("#monitor-account"),
  monitorLimit: document.querySelector("#monitor-limit"),
  monitorList: document.querySelector("#monitor-list"),
  refreshMonitor: document.querySelector("#refresh-monitor"),
  logFilterForm: document.querySelector("#log-filter-form"),
  openLogsRoot: document.querySelector("#open-logs-root"),
  logProvider: document.querySelector("#log-provider"),
  logAccount: document.querySelector("#log-account"),
  logLimit: document.querySelector("#log-limit"),
  logsList: document.querySelector("#logs-list"),
  refreshLogs: document.querySelector("#refresh-logs"),
  toastStack: document.querySelector("#toast-stack"),
  emptyTemplate: document.querySelector("#empty-template"),
};
let liveMonitorRefreshing = false;

window.addEventListener("DOMContentLoaded", async () => {
  startUiDevPolling();
  bindEvents();
  resetProviderForm();
  resetAccountForm();
  resetRouteForm();
  setActiveTab(state.activeTab);
  await refreshAll();
  window.setInterval(async () => {
    await refreshDaemonStatus(true);
    await refreshHealth();
  }, 3000);
  window.setInterval(async () => {
    if (state.activeTab !== "monitor" || liveMonitorRefreshing) {
      return;
    }
    liveMonitorRefreshing = true;
    try {
      await refreshMonitor(true);
    } finally {
      liveMonitorRefreshing = false;
    }
  }, 1000);
});

function sleep(ms) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function startUiDevPolling() {
  if (!isUiDevOrigin()) {
    return;
  }

  let currentVersion = null;
  const poll = async () => {
    try {
      const response = await fetch("/__dev__/version", { cache: "no-store" });
      if (!response.ok) {
        return;
      }
      const nextVersion = (await response.text()).trim();
      if (!nextVersion) {
        return;
      }
      if (currentVersion && currentVersion !== nextVersion) {
        window.location.reload();
        return;
      }
      currentVersion = nextVersion;
    } catch {
      // Ignore transient dev server polling failures.
    }
  };

  void poll();
  window.setInterval(poll, 900);
}

function isUiDevOrigin() {
  return (
    window.location.protocol === "http:" &&
    (window.location.hostname === "127.0.0.1" || window.location.hostname === "localhost")
  );
}

function bindEvents() {
  elements.tabButtons.forEach((button) => {
    button.addEventListener("click", () => setActiveTab(button.dataset.tab));
  });

  elements.detailsButton.addEventListener("click", (event) => {
    event.stopPropagation();
    toggleDetailsPanel();
  });

  document.addEventListener("click", (event) => {
    if (!event.target.closest(".details-anchor")) {
      closeDetailsPanel();
    }
  });

  elements.vaultDialog.addEventListener("click", (event) => {
    if (event.target === elements.vaultDialog) {
      closeVaultDialog();
    }
  });

  elements.closeVaultDialog.addEventListener("click", closeVaultDialog);
  elements.providerDialog.addEventListener("click", (event) => {
    if (event.target === elements.providerDialog) {
      closeProviderDialog();
    }
  });
  elements.accountDialog.addEventListener("click", (event) => {
    if (event.target === elements.accountDialog) {
      closeAccountDialog();
    }
  });
  elements.routeDialog.addEventListener("click", (event) => {
    if (event.target === elements.routeDialog) {
      closeRouteDialog();
    }
  });
  elements.confirmDialog.addEventListener("click", (event) => {
    if (event.target === elements.confirmDialog) {
      closeConfirmDialog();
    }
  });
  elements.confirmDialog.addEventListener("close", () => {
    pendingConfirmation = null;
  });
  elements.providerDialog.addEventListener("close", resetProviderForm);
  elements.accountDialog.addEventListener("close", resetAccountForm);
  elements.routeDialog.addEventListener("close", resetRouteForm);
  elements.closeProviderDialog.addEventListener("click", closeProviderDialog);
  elements.closeAccountDialog.addEventListener("click", closeAccountDialog);
  elements.closeRouteDialog.addEventListener("click", closeRouteDialog);
  elements.closeConfirmDialog.addEventListener("click", closeConfirmDialog);
  elements.accountSecretToggle.addEventListener("click", () => {
    if (!state.accountEditor?.id || !state.accountEditor?.hasSecret) {
      return;
    }
    const shouldOpen = elements.accountSecretPanel.hidden;
    if (!shouldOpen) {
      clearAccountSecretReveal();
      renderAccountSecretControls();
      return;
    }
    elements.accountSecretPanel.hidden = false;
    renderAccountSecretControls();
    window.setTimeout(() => elements.accountSecretPassword.focus(), 0);
  });
  elements.accountSecretReveal.addEventListener("click", async () => {
    const account = state.accountEditor;
    const masterPassword = elements.accountSecretPassword.value.trim();
    if (!account?.id) {
      notify("Open an existing account before revealing its API key.", "error");
      return;
    }
    if (!masterPassword) {
      notify("Master password cannot be empty.", "error");
      return;
    }

    const revealed = await perform(
      () =>
        api(`/admin/accounts/${account.id}/reveal`, {
          method: "POST",
          body: { masterPassword },
        }),
      "API key revealed.",
    );
    if (!revealed) {
      return;
    }

    state.revealedSecret = revealed.apiKey || "";
    elements.accountSecretPassword.value = "";
    renderAccountSecretControls();
  });
  elements.accountSecretCopyButton.addEventListener("click", async () => {
    if (!state.revealedSecret) {
      return;
    }
    await copyText(state.revealedSecret, "API key copied.");
  });
  elements.confirmCancel.addEventListener("click", closeConfirmDialog);
  elements.confirmForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const action = pendingConfirmation;
    pendingConfirmation = null;
    closeDialog(elements.confirmDialog);
    if (action) {
      await action();
    }
  });

  elements.unlockForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const masterPassword = elements.masterPassword.value.trim();
    if (!masterPassword) {
      notify("Master password cannot be empty.", "error");
      return;
    }

    const response = await perform(
      () =>
        api("/admin/unlock", {
          method: "POST",
          body: { masterPassword },
        }),
      state.health?.initialized ? "Vault available." : "Vault initialized and available.",
    );
    if (!response) {
      return;
    }

    elements.masterPassword.value = "";
    closeVaultDialog();
    await refreshAll();
  });

  elements.vaultActionButton.addEventListener("click", async () => {
    if (state.health?.unlocked) {
      const response = await perform(
        () => api("/admin/lock", { method: "POST" }),
        "Vault locked.",
      );
      if (!response) {
        return;
      }
      closeVaultDialog();
      await refreshAll();
      return;
    }

    openVaultDialog();
  });

  elements.startDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("start_daemon"),
      "Daemon started.",
      "Failed to start daemon.",
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    await refreshHealth({ attempts: 8, delayMs: 350 });
    await refreshAll();
  });

  elements.stopDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("stop_daemon"),
      "Daemon stopped.",
      "Failed to stop daemon.",
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    state.health = null;
    syncDaemonPanels();
    renderChrome();
    renderDashboard();
  });

  elements.restartDaemonButton.addEventListener("click", async () => {
    const status = await performDesktop(
      () => invokeDesktop("restart_daemon"),
      "Daemon restarted.",
      "Failed to restart daemon.",
    );
    if (!status) {
      await refreshDaemonStatus(false);
      return;
    }
    state.daemonStatus = status;
    await refreshHealth({ attempts: 8, delayMs: 350 });
    await refreshAll();
  });

  elements.openDaemonLogButton.addEventListener("click", async () => {
    await performDesktop(
      () => invokeDesktop("open_daemon_log"),
      "Daemon log opened.",
      "Failed to open daemon log.",
    );
  });

  elements.refreshOnboarding.addEventListener("click", async () => {
    await refreshOnboarding();
    notify("Onboarding guides refreshed.", "info");
  });

  elements.refreshRoutes.addEventListener("click", async () => {
    await refreshRoutes();
    renderDashboard();
    notify("Routes refreshed.", "info");
  });

  elements.refreshMonitor.addEventListener("click", async () => {
    await refreshMonitor(false);
    notify("Monitor refreshed.", "info");
  });
  elements.monitorFilterForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    await refreshMonitor(true);
  });

  elements.refreshLogs.addEventListener("click", async () => {
    await refreshLogs();
    await refreshDashboardLogs();
    notify("Logs refreshed.", "info");
  });
  elements.openLogsRoot.addEventListener("click", async () => {
    await performDesktop(
      () => invokeDesktop("open_logs_root"),
      "Log root opened.",
      "Failed to open log root.",
    );
  });

  elements.openProviderDialog.addEventListener("click", () => {
    resetProviderForm();
    openDialog(elements.providerDialog, elements.providerName);
  });
  elements.providerProtocol.addEventListener("change", () => applyProviderProtocolDefaults(true));
  elements.providerName.addEventListener("input", () => syncProviderIdentity());
  elements.providerPath.addEventListener("input", () => {
    elements.providerPath.dataset.autofill = "off";
    syncGeneratedProviderSlug();
    renderProviderPathDemo();
  });

  elements.providerForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const payload = buildProviderPayload();
    if (!payload) {
      return;
    }

    const isEditing = Boolean(state.providerEditor);
    const provider = await perform(
      () =>
        api("/admin/providers", {
          method: "POST",
          body: payload,
        }),
      isEditing ? "Provider updated." : "Provider saved.",
    );
    if (!provider) {
      return;
    }

    closeProviderDialog();
    await refreshProviders();
    await refreshAccounts();
    await refreshRoutes();
    await refreshOnboarding();
    renderDashboard();
  });

  elements.openAccountDialog.addEventListener("click", () => {
    resetAccountForm();
    openDialog(elements.accountDialog, elements.accountName);
  });

  elements.accountForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const payload = buildAccountPayload();
    if (!payload) {
      return;
    }

    const isEditing = Boolean(state.accountEditor);
    const account = await perform(
      () =>
        api("/admin/accounts", {
          method: "POST",
          body: payload,
        }),
      isEditing ? "Account updated." : "Account saved.",
    );
    if (!account) {
      return;
    }

    closeAccountDialog();
    await refreshAccounts();
    await refreshRoutes();
    renderDashboard();
  });

  elements.openRouteDialog.addEventListener("click", () => {
    resetRouteForm();
    openDialog(elements.routeDialog, elements.routeProvider);
  });

  elements.routeForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const payload = buildRoutePayload();
    if (!payload) {
      return;
    }

    const previousId = state.routeEditor?.id;
    const nextId = routeBindingId(payload.provider, payload.modelPrefix);
    const route = await perform(
      () =>
        api("/admin/routes", {
          method: "POST",
          body: payload,
        }),
      state.routeEditor ? "Route updated." : "Route saved.",
    );
    if (!route) {
      return;
    }

    if (previousId && previousId !== nextId) {
      await perform(() => api(`/admin/routes/${previousId}`, { method: "DELETE" }), null);
    }

    closeRouteDialog();
    await refreshRoutes();
    renderDashboard();
  });

  elements.routeProvider.addEventListener("change", syncRouteAccountOptions);

  elements.logFilterForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    await refreshLogs();
  });
}

async function refreshAll() {
  await refreshDaemonStatus(true);
  await refreshHealth({ attempts: 6, delayMs: 350 });
  await refreshProviders();
  await refreshAccounts();
  await refreshRoutes();
  await refreshOnboarding();
  await refreshMonitor(true);
  await refreshDashboardLogs();
  await refreshLogs();
  renderDashboard();
}

async function refreshDaemonStatus(silent = true) {
  try {
    state.daemonStatus = await invokeDesktop("daemon_status");
  } catch (error) {
    state.daemonStatus = null;
    if (!silent) {
      notify("Cannot read daemon process status from the desktop host.", "error");
    }
    console.error(error);
  }
  if (!state.health) {
    if (state.daemonStatus?.running) {
      setDaemonChip("Daemon starting", "warn");
    } else {
      setDaemonChip("Daemon offline", "bad");
    }
  }
  syncDaemonPanels();
  renderChrome();
  renderDashboard();
}

async function refreshHealth(options = {}) {
  const attempts = options.attempts || 1;
  const delayMs = options.delayMs || 0;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      state.health = await api("/health", { silent: true });
      const tone = state.health.unlocked ? "ok" : state.health.initialized ? "warn" : "idle";
      const chipText = state.health.unlocked
        ? "Daemon Online"
        : state.health.initialized
          ? "Vault Locked"
          : "Setup Required";
      setDaemonChip(chipText, tone);
      syncDaemonPanels();
      renderChrome();
      renderDashboard();
      return;
    } catch (error) {
      state.health = null;
      if (attempt < attempts - 1) {
        await sleep(delayMs);
        continue;
      }
      await refreshDaemonStatus(true);
      if (!state.daemonStatus?.running) {
        setDaemonChip("Daemon offline", "bad");
      } else {
        setDaemonChip("Daemon unreachable", "warn");
      }
      syncDaemonPanels();
      console.error(error);
    }
  }
  renderChrome();
  renderDashboard();
}

async function refreshProviders() {
  try {
    state.providers = await api("/admin/providers", { silent: true });
  } catch (error) {
    state.providers = [];
    console.error(error);
  }
  renderProviders();
  renderAccountProviderTabs();
  syncProviderOptions();
  syncMonitorProviderOptions();
  syncLogProviderOptions();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshAccounts() {
  try {
    state.accounts = await api("/admin/accounts", { silent: true });
  } catch (error) {
    state.accounts = [];
    console.error(error);
  }
  renderAccountProviderTabs();
  renderAccounts();
  syncProviderOptions();
  syncRouteAccountOptions();
  syncMonitorAccountOptions();
  syncLogAccountOptions();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshRoutes() {
  try {
    state.routes = await api("/admin/routes", { silent: true });
  } catch (error) {
    state.routes = [];
    console.error(error);
  }
  renderAccounts();
  renderRoutes();
  await refreshOnboarding();
  renderDashboard();
}

async function refreshDashboardLogs() {
  try {
    state.dashboardLogs = await fetchLogs({ limit: 100 }, true);
  } catch (error) {
    state.dashboardLogs = [];
    console.error(error);
  }
  renderDashboard();
}

async function refreshLogs() {
  try {
    state.logs = await fetchLogs(
      {
        provider: elements.logProvider.value,
        accountId: elements.logAccount.value,
        limit: elements.logLimit.value || "50",
      },
      true,
    );
  } catch (error) {
    state.logs = [];
    console.error(error);
  }
  renderLogs();
}

async function refreshMonitor(silent = true) {
  try {
    state.monitor = await fetchMonitor(
      {
        provider: elements.monitorProvider.value,
        accountId: elements.monitorAccount.value,
        limit: elements.monitorLimit.value || "60",
      },
      silent,
    );
  } catch (error) {
    state.monitor = [];
    console.error(error);
  }
  renderMonitor();
}

async function refreshOnboarding() {
  const availableTargets = availableOnboardingTargets();
  if (!availableTargets.length) {
    state.onboarding = [];
    state.onboardingProvider = null;
    renderOnboarding();
    return;
  }

  if (!availableTargets.some((target) => target.id === state.onboardingTarget)) {
    state.onboardingTarget = availableTargets[0].id;
  }

  const providers = onboardingProvidersForTarget(state.onboardingTarget);
  if (!providers.some((provider) => provider.slug === state.onboardingProvider)) {
    state.onboardingProvider = providers[0]?.slug ?? null;
  }

  const selectedProvider = providers.find((provider) => provider.slug === state.onboardingProvider);
  state.onboarding = selectedProvider ? [buildOnboardingGuide(state.onboardingTarget, selectedProvider)] : [];
  renderOnboarding();
}

async function fetchLogs(filters, silent) {
  const params = new URLSearchParams();
  if (filters.provider) {
    params.set("provider", filters.provider);
  }
  if (filters.accountId) {
    params.set("accountId", filters.accountId);
  }
  params.set("limit", String(filters.limit || 50));
  return api(`/admin/logs?${params.toString()}`, { silent });
}

async function fetchMonitor(filters, silent) {
  const params = new URLSearchParams();
  if (filters.provider) {
    params.set("provider", filters.provider);
  }
  if (filters.accountId) {
    params.set("accountId", filters.accountId);
  }
  params.set("limit", String(filters.limit || 60));
  return api(`/admin/monitor?${params.toString()}`, { silent });
}

function renderDashboard() {
  renderMetrics();
  renderRouteSummary();
  renderRecentActivity();
  renderDashboardHealth();
}

function renderMetrics() {
  const enabledProviders = state.providers.filter((provider) => provider.enabled);
  const customProviders = state.providers.filter((provider) => !provider.isBuiltin);
  const enabledAccounts = state.accounts.filter((account) => account.enabled);
  const defaultRoutes = enabledProviders.filter((provider) =>
    state.routes.some((route) => route.provider === provider.slug && !route.modelPrefix),
  );
  const metrics = [
    {
      label: "Recent Requests",
      value: String(state.dashboardLogs.length),
      note: "Last 100 logged requests",
      tone: "accent",
    },
    {
      label: "Success Rate",
      value: formatSuccessRate(state.dashboardLogs),
      note: "HTTP status below 400",
      tone: "accent",
    },
    {
      label: "Avg Latency",
      value: formatLatency(averageLatency(state.dashboardLogs)),
      note: "Across recent requests",
    },
    {
      label: "P95 Latency",
      value: formatLatency(percentileLatency(state.dashboardLogs, 0.95)),
      note: "Slow-tail indicator",
      tone: "warm",
    },
    {
      label: "Enabled Accounts",
      value: `${enabledAccounts.length}/${state.accounts.length}`,
      note: "Accounts available for routing",
    },
    {
      label: "Route Coverage",
      value: `${defaultRoutes.length}/${enabledProviders.length || 0}`,
      note: `${customProviders.length} custom provider${customProviders.length === 1 ? "" : "s"}`,
      tone: "warm",
    },
  ];

  elements.metricsList.replaceChildren(
    ...(metrics.length
      ? metrics.map((metric) => {
          const card = document.createElement("article");
          card.className = `metric-card${metric.tone ? ` ${metric.tone}` : ""}`;
          card.innerHTML = `
            <p class="metric-label">${escapeHtml(metric.label)}</p>
            <div class="metric-value">${escapeHtml(metric.value)}</div>
            <div class="metric-note">${escapeHtml(metric.note)}</div>
          `;
          return card;
        })
      : [emptyNode()]),
  );
}

function renderRouteSummary() {
  const items = state.providers.map((provider) => {
    const enabledAccounts = state.accounts.filter(
      (account) => account.provider === provider.slug && account.enabled,
    );
    const defaultRoute = state.routes.find(
      (route) => route.provider === provider.slug && !route.modelPrefix,
    );
    const defaultAccount = defaultRoute
      ? state.accounts.find((account) => account.id === defaultRoute.accountId)
      : null;
    const overrideCount = state.routes.filter(
      (route) => route.provider === provider.slug && route.modelPrefix,
    ).length;

    const item = document.createElement("article");
    item.className = "summary-item";
    item.innerHTML = `
      <div class="summary-title">
        <div>
          <h3>${escapeHtml(provider.displayName)}</h3>
          <p class="muted">${escapeHtml(provider.baseUrl)}</p>
        </div>
        <span class="pill ${provider.enabled ? "ok" : "bad"}">${provider.enabled ? "enabled" : "disabled"}</span>
      </div>
      <div class="meta-row">
        <span class="pill">${escapeHtml(provider.slug)}</span>
        <span class="pill">${escapeHtml(provider.protocol)}</span>
        <span class="pill">${escapeHtml(`/${provider.proxyPath}`)}</span>
        ${provider.isBuiltin ? '<span class="pill warm">built-in</span>' : ""}
      </div>
      <div class="meta-row">
        <span class="pill ${defaultRoute ? "ok" : "warn"}">${defaultRoute ? "default bound" : "default missing"}</span>
        <span class="pill">${escapeHtml(`${enabledAccounts.length} enabled account${enabledAccounts.length === 1 ? "" : "s"}`)}</span>
        <span class="pill">${escapeHtml(`${overrideCount} override${overrideCount === 1 ? "" : "s"}`)}</span>
      </div>
      <p class="muted">${escapeHtml(defaultAccount ? `Default route -> ${defaultAccount.name}` : "No default route bound yet.")}</p>
    `;
    return item;
  });

  elements.routeSummaryList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderRecentActivity() {
  const items = state.dashboardLogs.slice(0, 6).map((log) => {
    const provider = getProvider(log.provider);
    const account = state.accounts.find((candidate) => candidate.id === log.accountId);
    const item = document.createElement("article");
    item.className = "activity-item";
    item.innerHTML = `
      <div class="meta-row">
        <span class="pill">${escapeHtml(provider ? provider.displayName : log.provider)}</span>
        <span class="pill ${isSuccessStatus(log.statusCode) ? "ok" : "bad"}">${escapeHtml(String(log.statusCode ?? "error"))}</span>
        <span class="pill">${escapeHtml(formatLatency(log.durationMs))}</span>
        <span class="pill">${escapeHtml(formatRelativeTime(log.createdAt))}</span>
      </div>
      <strong>${escapeHtml(log.path)}</strong>
      <div class="muted">${escapeHtml(account?.name || log.accountId || "No account")} · ${escapeHtml(log.model || "model unavailable")}</div>
    `;
    return item;
  });

  elements.recentActivityList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderDashboardHealth() {
  const daemon = state.daemonStatus;
  const rows = state.health
    ? [
        ["Daemon", "Online"],
        ["PID", daemon?.pid ? String(daemon.pid) : "Unavailable"],
        [
          "Vault",
          state.health.unlocked ? "Available" : state.health.initialized ? "Locked" : "Not initialized",
        ],
        ["Database", state.health.dbPath],
        ["Started", formatDateTime(state.health.startedAt)],
        ["Port", String(state.health.port)],
        ["Launch", daemon?.launchMode || "Unavailable"],
        ["Providers", String(state.providers.length)],
      ]
    : [
        ["Daemon", daemon?.running ? "Running, health unavailable" : "Offline"],
        ["PID", daemon?.pid ? String(daemon.pid) : "Unavailable"],
        ["Vault", "Unavailable"],
        ["Database", "Unavailable"],
        ["Started", daemon?.startedAt ? formatDateTime(daemon.startedAt) : "Unavailable"],
        ["Port", String(daemon?.port || configuredDaemonPort())],
        ["Launch", daemon?.launchMode || "Unavailable"],
        ["Last Exit", daemon?.lastExit || "Unavailable"],
        ["Last Error", daemon?.lastError || "Unavailable"],
      ];

  elements.dashboardHealthList.replaceChildren(
    ...rows.map(([label, value]) => {
      const wrapper = document.createElement("div");
      wrapper.innerHTML = `
        <dt>${escapeHtml(label)}</dt>
        <dd>${escapeHtml(value)}</dd>
      `;
      return wrapper;
    }),
  );
}

function renderProviders() {
  const items = state.providers.map((provider) => {
    const item = document.createElement("article");
    item.className = "data-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(provider.displayName)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(provider.baseUrl)}</p>
        </div>
        <span class="pill ${provider.enabled ? "ok" : "bad"}">${provider.enabled ? "enabled" : "disabled"}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider.slug)}</span>
        <span class="pill">${escapeHtml(provider.protocol)}</span>
        <span class="pill">${escapeHtml(`/${provider.proxyPath}`)}</span>
        <span class="pill">${escapeHtml(provider.authHeader)}${provider.authPrefix ? `: ${escapeHtml(provider.authPrefix)}` : ""}</span>
        <span class="pill">${escapeHtml(`updated ${formatRelativeTime(provider.updatedAt)}`)}</span>
        ${provider.isBuiltin ? '<span class="pill warm">built-in</span>' : ""}
      </div>
      <div class="actions">
        <button type="button" class="ghost">Edit</button>
        ${provider.isBuiltin ? "" : '<button type="button" class="ghost">Delete</button>'}
      </div>
    `;
    const buttons = item.querySelectorAll("button");
    buttons[0].addEventListener("click", () => {
      fillProviderForm(provider);
      openDialog(elements.providerDialog, elements.providerName);
    });
    if (!provider.isBuiltin && buttons[1]) {
      buttons[1].addEventListener("click", async () => {
        requestConfirmation({
          title: `Delete Provider: ${provider.displayName}`,
          message:
            "Delete this provider definition? This only succeeds after all dependent accounts and routes are removed.",
          confirmLabel: "Delete Provider",
          onConfirm: async () => {
            const response = await perform(
              () => api(`/admin/providers/${provider.slug}`, { method: "DELETE" }),
              `Provider ${provider.displayName} deleted.`,
            );
            if (!response) {
              return;
            }
            if (state.providerEditor?.slug === provider.slug) {
              closeProviderDialog();
            }
            await refreshProviders();
            await refreshAccounts();
            await refreshRoutes();
            renderDashboard();
          },
        });
      });
    }
    return item;
  });

  elements.providersList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderAccountProviderTabs() {
  const activeFilter = normalizeAccountProviderFilter();
  const providers = state.providers.map((provider) => ({
    slug: provider.slug,
    label: provider.displayName,
    count: state.accounts.filter((account) => account.provider === provider.slug).length,
  }));

  const buttons = [
    {
      slug: "all",
      label: "All",
      count: state.accounts.length,
    },
    ...providers,
  ].map((entry) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${entry.slug === activeFilter ? " is-active" : ""}`;
    button.textContent = `${entry.label} (${entry.count})`;
    button.addEventListener("click", () => {
      if (state.accountProviderFilter === entry.slug) {
        return;
      }
      state.accountProviderFilter = entry.slug;
      renderAccountProviderTabs();
      renderAccounts();
    });
    return button;
  });

  elements.accountsProviderTabs.replaceChildren(...buttons);
}

function renderAccounts() {
  const visibleAccounts = filteredAccounts();
  const items = visibleAccounts.map((account) => {
    const provider = getProvider(account.provider);
    const defaultRoute = defaultRouteForProvider(account.provider);
    const isDefaultAccount = defaultRoute?.accountId === account.id;
    const routeCount = countRoutesForAccount(account.id);
    const upstreamSummary = account.baseUrl || provider?.baseUrl || "Uses provider base URL.";
    const noteSummary = account.note ? `${upstreamSummary} · ${account.note}` : upstreamSummary;
    const item = document.createElement("article");
    item.className = `data-item${isDefaultAccount ? " default-account-item" : ""}`;
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(account.name)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(noteSummary)}</p>
        </div>
        <span class="pill ${account.enabled ? "ok" : "bad"}">${account.enabled ? "enabled" : "disabled"}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider ? provider.displayName : account.provider)}</span>
        <span class="pill ${account.hasSecret ? "ok" : "warn"}">${account.hasSecret ? "secret stored" : "missing secret"}</span>
        <span class="pill ${account.baseUrl ? "warm" : ""}">${escapeHtml(account.baseUrl ? "account base url" : "provider base url")}</span>
        ${isDefaultAccount ? '<span class="pill ok">default</span>' : ""}
        <span class="pill">${escapeHtml(`${routeCount} route${routeCount === 1 ? "" : "s"}`)}</span>
        <span class="pill">${escapeHtml(`updated ${formatRelativeTime(account.updatedAt)}`)}</span>
      </div>
      <div class="actions">
        <button type="button" class="ghost" ${isDefaultAccount || !account.enabled ? "disabled" : ""}>${isDefaultAccount ? "Default" : "Set Default"}</button>
        <button type="button" class="ghost">Edit</button>
        <button type="button" class="ghost" ${account.enabled ? "" : "disabled"}>Disable</button>
        <button type="button" class="ghost">Delete</button>
      </div>
    `;

    const [defaultButton, editButton, disableButton, deleteButton] = item.querySelectorAll("button");
    defaultButton.addEventListener("click", async () => {
      const response = await perform(
        () =>
          api("/admin/routes", {
            method: "POST",
            body: {
              provider: account.provider,
              modelPrefix: null,
              accountId: account.id,
            },
          }),
        `${account.name} is now the default account for ${provider?.displayName || account.provider}.`,
      );
      if (!response) {
        return;
      }
      await refreshRoutes();
      renderDashboard();
    });
    editButton.addEventListener("click", () => {
      fillAccountForm(account);
      openDialog(elements.accountDialog, elements.accountName);
    });
    disableButton.addEventListener("click", async () => {
      const response = await perform(
        () => api(`/admin/accounts/${account.id}/disable`, { method: "POST" }),
        `Account ${account.name} disabled.`,
      );
      if (!response) {
        return;
      }
      await refreshAccounts();
      await refreshRoutes();
      renderDashboard();
    });
    deleteButton.addEventListener("click", async () => {
      requestConfirmation({
        title: `Delete Account: ${account.name}`,
        message:
          "Delete this account and its stored secret? Any route bindings pointing at it will also be removed.",
        confirmLabel: "Delete Account",
        onConfirm: async () => {
          const response = await perform(
            () => api(`/admin/accounts/${account.id}`, { method: "DELETE" }),
            `Account ${account.name} deleted.`,
          );
          if (!response) {
            return;
          }
          if (state.accountEditor?.id === account.id) {
            closeAccountDialog();
          }
          await refreshAccounts();
          await refreshRoutes();
          await refreshLogs();
          await refreshDashboardLogs();
          renderDashboard();
        },
      });
    });
    return item;
  });

  if (items.length) {
    elements.accountsList.replaceChildren(...items);
    return;
  }

  const activeFilter = normalizeAccountProviderFilter();
  if (activeFilter === "all") {
    elements.accountsList.replaceChildren(emptyNode());
    return;
  }

  const provider = getProvider(activeFilter);
  elements.accountsList.replaceChildren(
    emptyNode(provider ? `No accounts under ${provider.displayName} yet.` : "Nothing to show yet."),
  );
}

function renderRoutes() {
  const items = state.routes.map((route) => {
    const account = state.accounts.find((candidate) => candidate.id === route.accountId);
    const provider = getProvider(route.provider);
    const isDefaultRoute = !route.modelPrefix;
    const label = route.modelPrefix || `${provider?.displayName || route.provider} default`;
    const localUrl = providerLocalBaseUrl(provider, route.provider);

    const item = document.createElement("article");
    item.className = "data-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(label)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(
            `${route.modelPrefix ? "Override" : "Provider default"} ingress ${localUrl}`,
          )}</p>
        </div>
        <span class="pill ${route.modelPrefix ? "warm" : "ok"}">${route.modelPrefix ? "override" : "default"}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider ? provider.displayName : route.provider)}</span>
        <span class="pill">${escapeHtml(providerIngress(provider, route.provider))}</span>
        <span class="pill">${escapeHtml(account?.name || route.accountId)}</span>
        ${isDefaultRoute ? '<span class="pill ok">provider default</span>' : ""}
        <span class="pill">${escapeHtml(`updated ${formatRelativeTime(route.updatedAt)}`)}</span>
      </div>
      <div class="actions">
        <button type="button" class="ghost">Copy URL</button>
        <button type="button" class="ghost">Edit</button>
        <button type="button" class="ghost" ${isDefaultRoute ? "disabled" : ""}>Delete</button>
      </div>
    `;
    const [copyButton, editButton, deleteButton] = item.querySelectorAll("button");
    copyButton.addEventListener("click", async () => {
      await copyText(localUrl, `Copied ${providerIngress(provider, route.provider)} URL.`);
    });
    editButton.addEventListener("click", () => {
      fillRouteForm(route);
      openDialog(elements.routeDialog, elements.routeProvider);
    });
    deleteButton.addEventListener("click", async () => {
      if (isDefaultRoute) {
        return;
      }
      requestConfirmation({
        title: `Delete Route: ${label}`,
        message: "Delete this model override route?",
        confirmLabel: "Delete Route",
        onConfirm: async () => {
          const response = await perform(
            () => api(`/admin/routes/${route.id}`, { method: "DELETE" }),
            "Route deleted.",
          );
          if (!response) {
            return;
          }
          if (state.routeEditor?.id === route.id) {
            closeRouteDialog();
          }
          await refreshRoutes();
          renderDashboard();
        },
      });
    });
    return item;
  });

  elements.routesList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderMonitor() {
  const items = state.monitor.map((entry) => {
    const account = state.accounts.find((candidate) => candidate.id === entry.accountId);
    const provider = getProvider(entry.provider);
    const providerName = provider ? provider.displayName : entry.provider;
    const accountName = account?.name || entry.accountId || "routing";
    const item = document.createElement("article");
    item.className = "data-item monitor-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(`${entry.method} ${entry.path}`)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(entry.model || "model unavailable")}</p>
        </div>
        <div class="monitor-status">
          <span class="pill ${monitorPhaseTone(entry)}">${escapeHtml(monitorPhaseLabel(entry))}</span>
          <span class="pill ${monitorStatusTone(entry)}">${escapeHtml(monitorStatusLabel(entry))}</span>
          <button type="button" class="ghost monitor-copy-button">Copy</button>
        </div>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(providerName)}</span>
        <span class="pill">${escapeHtml(accountName)}</span>
        <span class="pill">${escapeHtml(entry.streamed ? "streamed" : "sync")}</span>
        <span class="pill">${escapeHtml(monitorDurationLabel(entry))}</span>
        <span class="pill">${escapeHtml(formatRelativeTime(entry.updatedAt || entry.startedAt))}</span>
      </div>
      <div class="monitor-preview-stack">
        <div class="monitor-preview-row">
          <span class="monitor-preview-label">Req</span>
          <p class="monitor-preview-text clamp-2">${escapeHtml(monitorRequestSummary(entry))}</p>
        </div>
        <div class="monitor-preview-row">
          <span class="monitor-preview-label">Res</span>
          <p class="monitor-preview-text clamp-2">${escapeHtml(monitorResponseSummary(entry))}</p>
        </div>
      </div>
    `;
    const copyButton = item.querySelector(".monitor-copy-button");
    copyButton.addEventListener("click", async () => {
      await copyText(
        buildMonitorClipboardText(entry, providerName, accountName),
        "Monitor item copied.",
      );
    });
    return item;
  });

  elements.monitorList.replaceChildren(
    ...(items.length ? items : [emptyNode("No live traffic in memory right now.")]),
  );
}

function renderLogs() {
  const items = state.logs.map((log) => {
    const account = state.accounts.find((candidate) => candidate.id === log.accountId);
    const provider = getProvider(log.provider);
    const artifactPath = logArtifactPath(log);
    const artifactRelativePath = logArtifactRelativePath(log);
    const openLabel = log.logFilePath ? "Open Day File" : "Open Folder";
    const copyLabel = log.logFilePath ? "Copy File Path" : "Copy Path";
    const item = document.createElement("article");
    item.className = "data-item log-item";
    item.innerHTML = `
      <div class="item-title">
        <div class="item-copy">
          <h3>${escapeHtml(`${log.method} ${log.path}`)}</h3>
          <p class="muted item-detail clamp-2">${escapeHtml(log.model || "model unavailable")}</p>
        </div>
        <span class="pill ${isSuccessStatus(log.statusCode) ? "ok" : "bad"}">${escapeHtml(String(log.statusCode ?? "error"))}</span>
      </div>
      <div class="data-meta">
        <span class="pill">${escapeHtml(provider ? provider.displayName : log.provider)}</span>
        <span class="pill">${escapeHtml(account?.name || log.accountId || "n/a")}</span>
        ${
          log.sessionId
            ? `<span class="pill warm" title="${escapeHtml(log.sessionId)}">${escapeHtml(formatSessionLabel(log.sessionId))}</span>`
            : ""
        }
        <span class="pill">${escapeHtml(formatLatency(log.durationMs))}</span>
        <span class="pill">${escapeHtml(log.streamed ? "streamed" : "sync")}</span>
        <span class="pill">${escapeHtml(formatRelativeTime(log.createdAt))}</span>
      </div>
      <p class="muted item-detail clamp-2">${escapeHtml(
        log.errorText || `Stored at ${artifactPath}`,
      )}</p>
      <div class="actions">
        <button type="button" class="ghost">${openLabel}</button>
        <button type="button" class="ghost">${copyLabel}</button>
      </div>
    `;
    const [openFileButton, copyPathButton] = item.querySelectorAll("button");
    openFileButton.addEventListener("click", async () => {
      await performDesktop(
        () => invokeDesktop("open_log_file", { relativePath: artifactRelativePath }),
        log.logFilePath ? "Log file opened." : "Log folder opened.",
        log.logFilePath ? "Failed to open log file." : "Failed to open log folder.",
      );
    });
    copyPathButton.addEventListener("click", async () => {
      await copyText(artifactPath, log.logFilePath ? "Log file path copied." : "Log path copied.");
    });
    return item;
  });

  elements.logsList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderOnboarding() {
  renderOnboardingTargetTabs();
  renderOnboardingProviderTabs();

  if (!state.onboarding.length) {
    const message = state.providers.length
      ? "No compatible provider is available for the selected client profile."
      : "Add a provider to generate onboarding instructions.";
    elements.onboardingList.replaceChildren(emptyNode(message));
    return;
  }

  const items = state.onboarding.map((guide) => {
    const item = document.createElement("article");
    item.className = "guide";
    item.innerHTML = `
      <div class="guide-head">
        <div>
          <p class="panel-kicker">${escapeHtml(guide.targetLabel)}</p>
          <h3>${escapeHtml(guide.title)}</h3>
          <p class="muted">${escapeHtml(guide.baseUrl)}</p>
        </div>
        <div class="guide-actions">
          <button type="button" class="ghost">Copy URL</button>
          <button type="button" class="ghost">Copy Snippet</button>
        </div>
      </div>
      <div class="guide-meta">
        ${guide.meta.map((entry) => `<span class="pill ${entry.tone || ""}">${escapeHtml(entry.label)}</span>`).join("")}
      </div>
      ${
        guide.env.length
          ? `<div class="guide-meta">
              ${guide.env
                .map(
                  (envVar) =>
                    `<span class="pill">${escapeHtml(envVar.key)}=${escapeHtml(envVar.value)}</span>`,
                )
                .join("")}
            </div>`
          : ""
      }
      <div class="guide-notes">
        ${guide.notes?.map((note) => `<p class="muted">${escapeHtml(note)}</p>`).join("") || ""}
      </div>
      <div class="guide-section">
        <p class="panel-kicker">Usage</p>
        <p class="muted">${escapeHtml(guide.summary)}</p>
      </div>
      <pre>${escapeHtml(guide.snippet)}</pre>
    `;
    const [copyUrlButton, copySnippetButton] = item.querySelectorAll(".guide-actions button");
    copyUrlButton.addEventListener("click", async () => {
      await copyText(guide.baseUrl, "Local base URL copied.");
    });
    copySnippetButton.addEventListener("click", async () => {
      await copyText(guide.snippet, "Onboarding snippet copied.");
    });
    return item;
  });

  elements.onboardingList.replaceChildren(...(items.length ? items : [emptyNode()]));
}

function renderOnboardingTargetTabs() {
  const targets = availableOnboardingTargets();
  const buttons = targets.map((target) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${target.id === state.onboardingTarget ? " is-active" : ""}`;
    button.textContent = target.label;
    button.addEventListener("click", async () => {
      if (state.onboardingTarget === target.id) {
        return;
      }
      state.onboardingTarget = target.id;
      state.onboardingProvider = null;
      await refreshOnboarding();
    });
    return button;
  });
  elements.onboardingTargetTabs.replaceChildren(...buttons);
}

function renderOnboardingProviderTabs() {
  const providers = onboardingProvidersForTarget(state.onboardingTarget);
  const buttons = providers.map((provider) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `filter-tab${provider.slug === state.onboardingProvider ? " is-active" : ""}`;
    button.textContent = provider.enabled
      ? provider.displayName
      : `${provider.displayName} (disabled)`;
    button.addEventListener("click", async () => {
      if (state.onboardingProvider === provider.slug) {
        return;
      }
      state.onboardingProvider = provider.slug;
      await refreshOnboarding();
    });
    return button;
  });
  elements.onboardingProviderTabs.replaceChildren(...buttons);
}

function availableOnboardingTargets() {
  return ONBOARDING_TARGETS.filter((target) =>
    state.providers.some((provider) => providerSupportsOnboardingTarget(provider, target.id)),
  );
}

function onboardingProvidersForTarget(targetId) {
  return state.providers.filter((provider) =>
    providerSupportsOnboardingTarget(provider, targetId),
  );
}

function providerSupportsOnboardingTarget(provider, targetId) {
  if (targetId === "curl") {
    return true;
  }
  if (targetId === "codex" || targetId === "generic-openai") {
    return provider.protocol === "openai";
  }
  if (targetId === "claude-code" || targetId === "generic-anthropic") {
    return provider.protocol === "anthropic";
  }
  return false;
}

function buildOnboardingGuide(targetId, provider) {
  const baseUrl = providerLocalBaseUrl(provider);
  const routeState = onboardingRouteState(provider.slug);
  const daemonUrl = daemonBaseUrl();
  const defaultAccountName = routeState.defaultAccount?.name || routeState.defaultRoute?.accountId;
  const defaultRouteLabel = routeState.defaultRoute
    ? `default -> ${defaultAccountName}`
    : "default missing";
  const overrideLabel = routeState.overrideCount
    ? `${routeState.overrideCount} override${routeState.overrideCount === 1 ? "" : "s"}`
    : "no overrides";
  const enabledAccountLabel = `${routeState.enabledAccounts} enabled account${
    routeState.enabledAccounts === 1 ? "" : "s"
  }`;
  const providerStatusNote = provider.enabled
    ? "This provider is enabled in the catalog."
    : "This provider is currently disabled in the catalog. Re-enable it before relying on this namespace.";
  const routeStatusNote = routeState.defaultRoute
    ? `Default traffic currently resolves to ${defaultAccountName}.`
    : "No provider default account is configured yet. Set one in Accounts or Routes before using this namespace as the catch-all path.";
  const overrideNote = routeState.overrideCount
    ? `${routeState.overrideCount} model-prefix override${
        routeState.overrideCount === 1 ? " is" : "s are"
      } active for this provider.`
    : "No model-prefix overrides are active for this provider.";
  const listenerNote = `Current local daemon address: ${daemonUrl}. This provider namespace resolves at ${baseUrl}.`;
  const credentialNote =
    "Client credentials shown here are placeholders only. LocalOpenRouter strips them and injects the real upstream secret from the selected account.";

  const meta = [
    { label: provider.protocol },
    { label: providerIngress(provider) },
    { label: provider.enabled ? "provider enabled" : "provider disabled", tone: provider.enabled ? "ok" : "bad" },
    { label: enabledAccountLabel },
    { label: defaultRouteLabel, tone: routeState.defaultRoute ? "ok" : "warn" },
    { label: overrideLabel, tone: routeState.overrideCount ? "warm" : "" },
  ];

  switch (targetId) {
    case "codex": {
      const env = openAiEnv(baseUrl);
      return {
        target: targetId,
        targetLabel: "Codex",
        title: `Codex via ${provider.displayName}`,
        baseUrl,
        meta,
        env,
        summary:
          `Use this namespace for Codex or any coding CLI that reads OpenAI-compatible base URL settings. Current target: ${baseUrl}.`,
        snippet: buildEnvSnippet(env),
        notes: [listenerNote, providerStatusNote, routeStatusNote, credentialNote],
      };
    }
    case "claude-code": {
      const env = anthropicEnv(baseUrl);
      return {
        target: targetId,
        targetLabel: "Claude Code",
        title: `Claude Code via ${provider.displayName}`,
        baseUrl,
        meta,
        env,
        summary:
          `Use this namespace for Claude Code or other Anthropic-style clients that support an alternate base URL. Current target: ${baseUrl}.`,
        snippet: buildEnvSnippet(env),
        notes: [listenerNote, providerStatusNote, routeStatusNote, credentialNote],
      };
    }
    case "generic-openai": {
      const env = openAiEnv(baseUrl);
      return {
        target: targetId,
        targetLabel: "Generic OpenAI",
        title: `OpenAI-Compatible Client via ${provider.displayName}`,
        baseUrl,
        meta,
        env,
        summary:
          `Use these settings for SDKs, tools, or scripts that speak the OpenAI API surface but are not Codex-specific. Current target: ${baseUrl}.`,
        snippet: buildEnvSnippet(env),
        notes: [listenerNote, providerStatusNote, routeStatusNote, credentialNote],
      };
    }
    case "generic-anthropic": {
      const env = anthropicEnv(baseUrl);
      return {
        target: targetId,
        targetLabel: "Generic Anthropic",
        title: `Anthropic-Compatible Client via ${provider.displayName}`,
        baseUrl,
        meta,
        env,
        summary:
          `Use these settings for SDKs or CLIs that expect Anthropic-compatible request shapes without being Claude Code itself. Current target: ${baseUrl}.`,
        snippet: buildEnvSnippet(env),
        notes: [listenerNote, providerStatusNote, routeStatusNote, credentialNote],
      };
    }
    case "curl":
    default:
      return {
        target: targetId,
        targetLabel: "cURL / Manual",
        title: `Manual HTTP via ${provider.displayName}`,
        baseUrl,
        meta,
        env: [],
        summary:
          provider.protocol === "generic"
            ? `Generic HTTP providers stay manual-only. Append the upstream-specific path and payload after this namespace. Current target: ${baseUrl}.`
            : `Use this for smoke tests, quick probes, or custom scripts against the local provider namespace. Current target: ${baseUrl}.`,
        snippet: buildCurlSnippet(provider, baseUrl),
        notes: [
          listenerNote,
          providerStatusNote,
          routeStatusNote,
          overrideNote,
          provider.protocol === "generic" ? "Generic HTTP providers do not have a Codex or Claude Code preset." : credentialNote,
        ],
      };
  }
}

function syncProviderOptions() {
  const enabledProviders = state.providers.filter((provider) => provider.enabled);
  const accountSelection = state.accountEditor?.provider || elements.accountProvider.value;
  const routeSelection = state.routeEditor?.provider || elements.routeProvider.value;
  elements.openAccountDialog.disabled = enabledProviders.length === 0;
  elements.openRouteDialog.disabled = enabledProviders.length === 0;

  const accountProviders = providerOptionsForSelect(accountSelection);
  const routeProviders = providerOptionsForSelect(routeSelection);

  if (accountProviders.length) {
    elements.accountProvider.replaceChildren(
      ...accountProviders.map((provider) =>
        optionNode(provider.slug, providerOptionLabel(provider)),
      ),
    );
    elements.accountProvider.disabled = false;
  } else {
    elements.accountProvider.replaceChildren(optionNode("", "No providers configured"));
    elements.accountProvider.disabled = true;
    elements.accountSubmit.disabled = true;
  }

  if (routeProviders.length) {
    elements.routeProvider.replaceChildren(
      ...routeProviders.map((provider) => optionNode(provider.slug, providerOptionLabel(provider))),
    );
    elements.routeProvider.disabled = false;
  } else {
    elements.routeProvider.replaceChildren(optionNode("", "No providers configured"));
    elements.routeProvider.disabled = true;
  }

  if (accountSelection && accountProviders.some((provider) => provider.slug === accountSelection)) {
    elements.accountProvider.value = accountSelection;
  } else if (accountProviders[0]) {
    elements.accountProvider.value = accountProviders[0].slug;
  }

  if (routeSelection && routeProviders.some((provider) => provider.slug === routeSelection)) {
    elements.routeProvider.value = routeSelection;
  } else if (routeProviders[0]) {
    elements.routeProvider.value = routeProviders[0].slug;
  }

  elements.accountSubmit.disabled = !accountProviders.length;
  syncRouteAccountOptions();
}

function syncRouteAccountOptions() {
  const provider = elements.routeProvider.value;
  const preferredAccountId = state.routeEditor?.provider === provider ? state.routeEditor.accountId : null;
  const candidates = routeAccountOptionsForProvider(provider, preferredAccountId);
  const current = elements.routeAccount.value;

  if (candidates.length) {
    elements.routeAccount.replaceChildren(
      ...candidates.map((account) =>
        optionNode(account.id, account.enabled ? account.name : `${account.name} (disabled)`),
      ),
    );
    if (current && candidates.some((account) => account.id === current)) {
      elements.routeAccount.value = current;
    } else if (preferredAccountId && candidates.some((account) => account.id === preferredAccountId)) {
      elements.routeAccount.value = preferredAccountId;
    } else if (candidates[0]) {
      elements.routeAccount.value = candidates[0].id;
    }

    const selectedAccount = candidates.find((account) => account.id === elements.routeAccount.value);
    elements.routeAccount.disabled = false;
    if (selectedAccount?.enabled === false) {
      elements.routeSubmit.disabled = true;
      elements.routeHint.textContent =
        "This route currently points at a disabled account. Choose an enabled account before saving.";
    } else {
      elements.routeSubmit.disabled = false;
      elements.routeHint.textContent = "Routes apply immediately to new requests.";
    }
  } else {
    elements.routeAccount.replaceChildren(optionNode("", "No enabled accounts"));
    elements.routeAccount.disabled = true;
    elements.routeSubmit.disabled = true;
    elements.routeHint.textContent =
      provider
        ? "This provider has no enabled accounts. Add or re-enable one before binding routes."
        : "Select a provider to see enabled accounts.";
  }
}

function syncProviderFilterOptions(select, allLabel) {
  const selected = select.value;
  const options = [
    optionNode("", allLabel),
    ...state.providers.map((provider) =>
      optionNode(provider.slug, `${provider.displayName} (${provider.protocol})`),
    ),
  ];
  select.replaceChildren(...options);
  if (selected) {
    select.value = selected;
  }
}

function syncAccountFilterOptions(select, allLabel) {
  const selected = select.value;
  const options = [
    optionNode("", allLabel),
    ...state.accounts.map((account) => {
      const provider = getProvider(account.provider);
      return optionNode(
        account.id,
        `${account.name} (${provider ? provider.displayName : account.provider})`,
      );
    }),
  ];
  select.replaceChildren(...options);
  if (selected) {
    select.value = selected;
  }
}

function syncMonitorProviderOptions() {
  syncProviderFilterOptions(elements.monitorProvider, "All providers");
}

function syncMonitorAccountOptions() {
  syncAccountFilterOptions(elements.monitorAccount, "All accounts");
}

function syncLogProviderOptions() {
  syncProviderFilterOptions(elements.logProvider, "All providers");
}

function syncLogAccountOptions() {
  syncAccountFilterOptions(elements.logAccount, "All accounts");
}

function resetProviderForm() {
  state.providerEditor = null;
  elements.providerFormTitle.textContent = "New Provider";
  elements.providerFormCopy.textContent =
    "Define a built-in override or register a custom upstream with its own proxy path, auth header, and protocol shape.";
  elements.providerSubmit.textContent = "Save Provider";
  elements.providerSlug.value = "";
  elements.providerProtocol.disabled = false;
  elements.providerName.value = "";
  elements.providerProtocol.value = "openai";
  elements.providerBaseUrl.value = "";
  elements.providerPath.value = "";
  elements.providerPath.dataset.autofill = "on";
  elements.providerAuthHeader.value = "";
  elements.providerAuthPrefix.value = "";
  elements.providerEnabled.checked = true;
  applyProviderProtocolDefaults(true);
  syncProviderIdentity();
}

function fillProviderForm(provider) {
  state.providerEditor = { slug: provider.slug, isBuiltin: provider.isBuiltin };
  elements.providerFormTitle.textContent = provider.isBuiltin
    ? `Tune Built-In: ${provider.displayName}`
    : `Edit Provider: ${provider.displayName}`;
  elements.providerFormCopy.textContent = provider.isBuiltin
    ? "Built-in providers keep their internal identity. You can still adjust endpoint, auth header, proxy path, and enabled state."
    : "Editing a custom provider updates the existing registry entry in place.";
  elements.providerSubmit.textContent = "Update Provider";
  elements.providerSlug.value = provider.slug;
  elements.providerProtocol.disabled = provider.isBuiltin;
  elements.providerName.value = provider.displayName;
  elements.providerProtocol.value = provider.protocol;
  elements.providerBaseUrl.value = provider.baseUrl;
  elements.providerPath.value = provider.proxyPath;
  elements.providerPath.dataset.autofill = "off";
  elements.providerAuthHeader.value = provider.authHeader;
  elements.providerAuthPrefix.value = provider.authPrefix || "";
  elements.providerEnabled.checked = provider.enabled;
  syncGeneratedProviderSlug();
  renderProviderPathDemo();
}

function buildProviderPayload() {
  const displayName = elements.providerName.value.trim();
  const slug = currentProviderSlug();
  const protocol = elements.providerProtocol.value;
  const baseUrl = elements.providerBaseUrl.value.trim();
  const proxyPath = normalizeSegment(elements.providerPath.value || slug);
  const authHeader = elements.providerAuthHeader.value.trim();
  const authPrefix = normalizeOptional(elements.providerAuthPrefix.value);

  if (!slug) {
    notify("Provider ID could not be generated from the display name.", "error");
    return null;
  }
  if (!isValidSlug(slug)) {
    notify("Provider ID may only use lowercase letters, digits, and dashes.", "error");
    return null;
  }
  if (!displayName) {
    notify("Display name is required.", "error");
    return null;
  }
  if (!proxyPath) {
    notify("Proxy path is required.", "error");
    return null;
  }
  if (!isValidSlug(proxyPath)) {
    notify("Proxy path must be one lowercase path segment with letters, digits, or dashes.", "error");
    return null;
  }
  if (!baseUrl.startsWith("http://") && !baseUrl.startsWith("https://")) {
    notify("Base URL must start with http:// or https://.", "error");
    return null;
  }
  if (!authHeader || /\s/.test(authHeader)) {
    notify("Auth header is required and cannot contain spaces.", "error");
    return null;
  }

  elements.providerSlug.value = slug;
  elements.providerPath.value = proxyPath;
  syncGeneratedProviderSlug();

  return {
    slug,
    displayName,
    protocol,
    baseUrl,
    proxyPath,
    authHeader,
    authPrefix,
    enabled: elements.providerEnabled.checked,
  };
}

function currentProviderSlug() {
  const existingSlug = elements.providerSlug.value || state.providerEditor?.slug || "";
  if (state.providerEditor) {
    return normalizeSegment(existingSlug);
  }
  return normalizeSegment(elements.providerName.value || "")
    || normalizeSegment(elements.providerPath.value || "")
    || normalizeSegment(existingSlug);
}

function syncGeneratedProviderSlug() {
  elements.providerSlug.value = currentProviderSlug();
}

function syncProviderIdentity() {
  syncGeneratedProviderSlug();
  syncProviderProxyPath();
}

function applyProviderProtocolDefaults(force) {
  const protocol = elements.providerProtocol.value;
  if (protocol === "openai") {
    if (force || !elements.providerAuthHeader.value) {
      elements.providerAuthHeader.value = "Authorization";
    }
    if (force || !elements.providerAuthPrefix.value) {
      elements.providerAuthPrefix.value = "Bearer";
    }
  } else if (protocol === "anthropic") {
    if (
      force ||
      !elements.providerAuthHeader.value ||
      elements.providerAuthHeader.value === "Authorization"
    ) {
      elements.providerAuthHeader.value = "x-api-key";
    }
    if (force || elements.providerAuthPrefix.value === "Bearer") {
      elements.providerAuthPrefix.value = "";
    }
  } else if (force) {
    elements.providerAuthHeader.value = "Authorization";
    elements.providerAuthPrefix.value = "Bearer";
  }
}

function syncProviderProxyPath() {
  if (elements.providerPath.dataset.autofill === "off") {
    renderProviderPathDemo();
    return;
  }
  elements.providerPath.value = currentProviderSlug();
  renderProviderPathDemo();
}

function resetAccountForm() {
  state.accountEditor = null;
  clearAccountSecretReveal();
  elements.accountFormTitle.textContent = "New Account";
  elements.accountFormCopy.textContent =
    "Store one encrypted credential set per account. Leave Base URL Override empty to inherit the provider upstream endpoint.";
  elements.accountSubmit.textContent = "Save Account";
  elements.accountId.value = "";
  elements.accountName.value = "";
  elements.accountApiKey.value = "";
  elements.accountBaseUrl.value = "";
  elements.accountNote.value = "";
  elements.accountEnabled.checked = true;
  const firstEnabled = state.providers.find((provider) => provider.enabled);
  if (firstEnabled) {
    elements.accountProvider.value = firstEnabled.slug;
  } else {
    elements.accountProvider.value = "";
  }
  renderAccountSecretControls();
}

function clearAccountSecretReveal() {
  state.revealedSecret = "";
  elements.accountSecretPanel.hidden = true;
  elements.accountSecretPassword.value = "";
  elements.accountSecretValue.value = "";
}

function renderAccountSecretControls() {
  const account = state.accountEditor;
  const provider = account ? getProvider(account.provider) : null;
  const visible = Boolean(account?.id && account?.hasSecret);
  elements.accountSecretTools.hidden = !visible;
  if (!visible) {
    return;
  }

  const accountLabel = account.name || "this account";
  elements.accountSecretCopy.textContent = `${
    provider?.displayName || account.provider
  } · Enter the vault password to reveal the current plaintext key for ${accountLabel}.`;
  elements.accountSecretToggle.textContent = elements.accountSecretPanel.hidden
    ? "View Stored Key"
    : "Hide Stored Key";
  elements.accountSecretValue.value = state.revealedSecret;
  elements.accountSecretCopyButton.disabled = !state.revealedSecret;
}

function fillAccountForm(account) {
  state.accountEditor = {
    id: account.id,
    provider: account.provider,
    name: account.name,
    hasSecret: account.hasSecret,
  };
  clearAccountSecretReveal();
  elements.accountFormTitle.textContent = `Edit Account: ${account.name}`;
  elements.accountFormCopy.textContent =
    "Leave the API key blank to keep the existing encrypted secret. Use View Stored Key below if you need to inspect the current key after re-entering the vault password.";
  elements.accountSubmit.textContent = "Update Account";
  elements.accountId.value = account.id;
  syncProviderOptions();
  elements.accountProvider.value = account.provider;
  elements.accountName.value = account.name;
  elements.accountApiKey.value = "";
  elements.accountBaseUrl.value = account.baseUrl || "";
  elements.accountNote.value = account.note || "";
  elements.accountEnabled.checked = account.enabled;
  renderAccountSecretControls();
}

function buildAccountPayload() {
  const provider = elements.accountProvider.value;
  const name = elements.accountName.value.trim();
  const apiKey = normalizeOptional(elements.accountApiKey.value);
  const baseUrl = normalizeOptional(elements.accountBaseUrl.value);
  const note = normalizeOptional(elements.accountNote.value);
  const isEditing = Boolean(state.accountEditor);

  if (!provider) {
    notify("Choose an enabled provider before saving an account.", "error");
    return null;
  }
  if (!name) {
    notify("Account name is required.", "error");
    return null;
  }
  if (!isEditing && !apiKey) {
    notify("New accounts require an API key.", "error");
    return null;
  }
  if (baseUrl && !baseUrl.startsWith("http://") && !baseUrl.startsWith("https://")) {
    notify("Account base URL must start with http:// or https://.", "error");
    return null;
  }

  return {
    id: elements.accountId.value || null,
    provider,
    name,
    baseUrl,
    apiKey,
    note,
    enabled: elements.accountEnabled.checked,
  };
}

function resetRouteForm() {
  state.routeEditor = null;
  elements.routeFormTitle.textContent = "New Route";
  elements.routeFormCopy.textContent =
    "Set one default account per provider, then add optional model-prefix overrides for fine-grained account selection.";
  elements.routeSubmit.textContent = "Save Route";
  elements.routePrefix.value = "";
  const firstEnabled = state.providers.find((provider) => provider.enabled);
  if (firstEnabled) {
    elements.routeProvider.value = firstEnabled.slug;
  } else {
    elements.routeProvider.value = "";
  }
  syncRouteAccountOptions();
}

function fillRouteForm(route) {
  state.routeEditor = {
    id: route.id,
    provider: route.provider,
    modelPrefix: route.modelPrefix,
    accountId: route.accountId,
  };
  elements.routeFormTitle.textContent = route.modelPrefix
    ? `Edit Route: ${route.modelPrefix}`
    : `Edit Route: ${route.provider} default`;
  elements.routeFormCopy.textContent = route.modelPrefix
    ? "Update the provider, prefix, or account binding. Changing provider or prefix will replace the previous binding."
    : "This row is the provider default account. Updating the bound account here changes the provider default used by non-matching requests.";
  elements.routeSubmit.textContent = "Update Route";
  syncProviderOptions();
  elements.routeProvider.value = route.provider;
  elements.routePrefix.value = route.modelPrefix || "";
  syncRouteAccountOptions();
  if ([...elements.routeAccount.options].some((option) => option.value === route.accountId)) {
    elements.routeAccount.value = route.accountId;
  }
}

function buildRoutePayload() {
  const provider = elements.routeProvider.value;
  const accountId = elements.routeAccount.value;
  const modelPrefix = normalizeOptional(elements.routePrefix.value);
  if (!provider) {
    notify("Choose a provider before saving a route.", "error");
    return null;
  }
  if (!accountId) {
    notify("The selected provider has no enabled accounts to bind.", "error");
    return null;
  }

  return {
    provider,
    modelPrefix,
    accountId,
  };
}

function setActiveTab(tab) {
  state.activeTab = tab;
  elements.tabButtons.forEach((button) => {
    const active = button.dataset.tab === tab;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-selected", String(active));
  });
  elements.tabPanels.forEach((panel) => {
    panel.classList.toggle("is-active", panel.dataset.panel === tab);
  });
  if (tab === "monitor") {
    void refreshMonitor(true);
  }
}

function setDaemonChip(text, tone) {
  elements.daemonChip.textContent = text;
  elements.daemonChip.dataset.tone = tone;
}

function syncDaemonPanels() {
  const daemon = state.daemonStatus;
  const health = state.health;
  const daemonRunning = Boolean(daemon?.running);

  elements.startDaemonButton.disabled = daemonRunning;
  elements.stopDaemonButton.disabled = !daemonRunning;
  elements.restartDaemonButton.disabled = false;
  elements.openDaemonLogButton.disabled = !daemon?.logFilePath;

  elements.detailPid.textContent = daemon?.pid ? String(daemon.pid) : "Unavailable";
  elements.daemonLogPath.textContent = daemon?.logFilePath || "Unavailable";
  elements.daemonLastExit.textContent = daemon?.lastExit || "Unavailable";
  elements.daemonLastError.textContent = daemon?.lastError || "Unavailable";

  if (health) {
    elements.vaultState.textContent = health.unlocked
      ? "Available"
      : health.initialized
        ? "Locked"
        : "Not initialized";
    elements.dbPath.textContent = health.dbPath;
    elements.daemonPort.textContent = String(health.port);
    elements.startedAt.textContent = formatDateTime(health.startedAt);
    elements.detailStatus.textContent = health.unlocked
      ? "Daemon online"
      : health.initialized
        ? "Vault locked"
        : "Setup required";
    renderProviderPathDemo();
    return;
  }

  elements.vaultState.textContent = daemonRunning ? "Unavailable" : "Offline";
  elements.dbPath.textContent = "Unavailable";
  elements.daemonPort.textContent = String(daemon?.port || DEFAULT_PORT);
  elements.startedAt.textContent = daemon?.startedAt
    ? formatDateTime(daemon.startedAt)
    : "Unavailable";
  elements.detailStatus.textContent = daemonRunning
    ? "Process running but health endpoint unavailable"
    : "Daemon offline";
  renderProviderPathDemo();
}

function renderChrome() {
  if (!state.health) {
    elements.vaultActionButton.disabled = true;
    elements.vaultActionButton.textContent = state.daemonStatus?.running
      ? "Daemon Starting"
      : "Daemon Offline";
    elements.vaultActionButton.dataset.mode = "waiting";
    elements.vaultDialogTitle.textContent = state.daemonStatus?.running
      ? "Daemon Starting"
      : "Daemon Offline";
    elements.vaultDialogCopy.textContent = state.daemonStatus?.running
      ? "Waiting for the local daemon health endpoint to come online."
      : "Start or restart the daemon before unlocking the vault.";
    elements.unlockSubmit.textContent = "Unlock";
    return;
  }

  elements.vaultActionButton.disabled = false;
  const initialized = Boolean(state.health?.initialized);
  const unlocked = Boolean(state.health?.unlocked);

  if (unlocked) {
    elements.vaultActionButton.textContent = "Lock Vault";
    elements.vaultActionButton.dataset.mode = "lock";
    elements.vaultDialogTitle.textContent = "Unlock Vault";
    elements.vaultDialogCopy.textContent =
      "The vault is already unlocked for this session. Use Lock Vault in the header to seal credentials again.";
    elements.unlockSubmit.textContent = "Unlock";
    return;
  }

  elements.vaultActionButton.textContent = initialized ? "Unlock Vault" : "Initialize Vault";
  elements.vaultActionButton.dataset.mode = "unlock";
  elements.vaultDialogTitle.textContent = initialized ? "Unlock Vault" : "Initialize Vault";
  elements.vaultDialogCopy.textContent = initialized
    ? "Enter the master password to unlock encrypted credentials for this local session."
    : "Choose a master password to initialize the local encrypted vault on first use.";
  elements.unlockSubmit.textContent = initialized ? "Unlock" : "Initialize";
}

function toggleDetailsPanel() {
  const hidden = elements.detailsPanel.hasAttribute("hidden");
  if (hidden) {
    elements.detailsPanel.removeAttribute("hidden");
    elements.detailsButton.setAttribute("aria-expanded", "true");
  } else {
    closeDetailsPanel();
  }
}

function closeDetailsPanel() {
  elements.detailsPanel.setAttribute("hidden", "");
  elements.detailsButton.setAttribute("aria-expanded", "false");
}

function openVaultDialog() {
  renderChrome();
  openDialog(elements.vaultDialog, elements.masterPassword);
}

function closeVaultDialog() {
  closeDialog(elements.vaultDialog);
}

function closeProviderDialog() {
  closeDialog(elements.providerDialog);
}

function closeAccountDialog() {
  closeDialog(elements.accountDialog);
}

function closeRouteDialog() {
  closeDialog(elements.routeDialog);
}

function closeConfirmDialog() {
  pendingConfirmation = null;
  closeDialog(elements.confirmDialog);
}

function openDialog(dialog, focusTarget) {
  if (typeof dialog.showModal === "function" && !dialog.open) {
    dialog.showModal();
  } else {
    dialog.setAttribute("open", "");
  }
  if (focusTarget) {
    window.setTimeout(() => focusTarget.focus(), 0);
  }
}

function closeDialog(dialog) {
  if (typeof dialog.close === "function" && dialog.open) {
    dialog.close();
  } else {
    dialog.removeAttribute("open");
    dialog.dispatchEvent(new Event("close"));
  }
}

function getProvider(slug) {
  return state.providers.find((provider) => provider.slug === slug);
}

function requestConfirmation({ title, message, confirmLabel, onConfirm }) {
  pendingConfirmation = onConfirm;
  elements.confirmDialogTitle.textContent = title;
  elements.confirmDialogCopy.textContent = message;
  elements.confirmSubmit.textContent = confirmLabel;
  elements.confirmSubmit.dataset.tone = "danger";
  openDialog(elements.confirmDialog, elements.confirmCancel);
}

function optionNode(value, label) {
  const option = document.createElement("option");
  option.value = value;
  option.textContent = label;
  return option;
}

function configuredDaemonPort() {
  const port = Number(state.health?.port || state.daemonStatus?.port || elements.daemonPort.textContent);
  return Number.isFinite(port) && port > 0 ? port : DEFAULT_PORT;
}

function daemonAddress() {
  return `127.0.0.1:${configuredDaemonPort()}`;
}

function daemonBaseUrl() {
  return `http://${daemonAddress()}`;
}

function providerIngress(provider, fallbackSlug = provider?.slug || "") {
  return `/${provider?.proxyPath || fallbackSlug}`;
}

function providerLocalBaseUrl(provider, fallbackSlug = provider?.slug || "") {
  return `${daemonBaseUrl()}${providerIngress(provider, fallbackSlug)}`;
}

function renderProviderPathDemo() {
  const path = normalizeSegment(elements.providerPath.value || "");
  elements.providerPathDemo.textContent = path
    ? `Local ingress: ${daemonBaseUrl()}/${path}`
    : `Local ingress: ${daemonBaseUrl()}/{proxy-path}`;
}

function storageRootPath() {
  const dbPath = state.health?.dbPath;
  if (!dbPath) {
    return null;
  }
  return dbPath.replace(/[/\\][^/\\]+$/, "");
}

function logArtifactRelativePath(log) {
  if (log.logFilePath) {
    return log.logFilePath;
  }
  const day = log.createdAt?.slice(0, 10) || "undated";
  return `logs/${day}/${log.id}`;
}

function logArtifactPath(log) {
  const root = storageRootPath();
  const relativePath = logArtifactRelativePath(log);
  return root ? `${root}/${relativePath}` : relativePath;
}

function formatSessionLabel(sessionId) {
  return `session ${truncateMiddle(sessionId, 20)}`;
}

function monitorPhaseLabel(entry) {
  switch (entry.phase) {
    case "routing":
      return "routing";
    case "upstream":
      return "upstream";
    case "response":
      return "response";
    case "streaming":
      return "streaming";
    case "failed":
      return "failed";
    case "completed":
    default:
      return "completed";
  }
}

function monitorPhaseTone(entry) {
  switch (entry.phase) {
    case "routing":
    case "upstream":
    case "response":
      return "warn";
    case "streaming":
      return "warm";
    case "failed":
      return "bad";
    case "completed":
    default:
      return isSuccessStatus(entry.statusCode) ? "ok" : "bad";
  }
}

function monitorStatusLabel(entry) {
  if (typeof entry.statusCode === "number") {
    return String(entry.statusCode);
  }
  return entry.phase === "failed" ? "error" : "live";
}

function monitorStatusTone(entry) {
  if (typeof entry.statusCode === "number") {
    return isSuccessStatus(entry.statusCode) ? "ok" : "bad";
  }
  return entry.phase === "failed" ? "bad" : "warm";
}

function monitorRequestSummary(entry) {
  return entry.requestPreview || "No request body preview.";
}

function monitorResponseSummary(entry) {
  if (entry.errorText) {
    return entry.errorText;
  }
  if (entry.responsePreview) {
    return entry.responsePreview;
  }
  switch (entry.phase) {
    case "routing":
      return "Resolving provider route and active account.";
    case "upstream":
      return "Forwarded upstream. Waiting for headers.";
    case "response":
      return "Receiving upstream response body.";
    case "streaming":
      return "Streaming response chunks.";
    case "failed":
      return "Request failed before a response preview was captured.";
    case "completed":
    default:
      return "Response completed with no preview payload.";
  }
}

function monitorDurationLabel(entry) {
  return typeof entry.durationMs === "number" ? formatLatency(entry.durationMs) : "live";
}

function buildMonitorClipboardText(entry, providerName, accountName) {
  return [
    `${entry.method} ${entry.path}`,
    `provider: ${providerName}`,
    `account: ${accountName}`,
    `model: ${entry.model || "model unavailable"}`,
    `phase: ${monitorPhaseLabel(entry)}`,
    `status: ${monitorStatusLabel(entry)}`,
    `duration: ${monitorDurationLabel(entry)}`,
    `mode: ${entry.streamed ? "streamed" : "sync"}`,
    `updated: ${formatDateTime(entry.updatedAt || entry.startedAt)}`,
    `request: ${monitorRequestSummary(entry)}`,
    `response: ${monitorResponseSummary(entry)}`,
  ].join("\n");
}

function countRoutesForAccount(accountId) {
  return state.routes.filter((route) => route.accountId === accountId).length;
}

function onboardingRouteState(providerSlug) {
  const providerRoutes = state.routes.filter((route) => route.provider === providerSlug);
  const defaultRoute = providerRoutes.find((route) => !route.modelPrefix) || null;
  const defaultAccount = defaultRoute
    ? state.accounts.find((account) => account.id === defaultRoute.accountId) || null
    : null;
  return {
    defaultRoute,
    defaultAccount,
    overrideCount: providerRoutes.filter((route) => route.modelPrefix).length,
    enabledAccounts: state.accounts.filter(
      (account) => account.provider === providerSlug && account.enabled,
    ).length,
  };
}

function openAiEnv(baseUrl) {
  return [
    { key: "OPENAI_BASE_URL", value: baseUrl },
    { key: "OPENAI_API_KEY", value: "localopenrouter-managed" },
  ];
}

function anthropicEnv(baseUrl) {
  return [
    { key: "ANTHROPIC_BASE_URL", value: baseUrl },
    { key: "ANTHROPIC_API_KEY", value: "localopenrouter-managed" },
  ];
}

function buildEnvSnippet(env) {
  return env.map((entry) => `export ${entry.key}="${entry.value}"`).join("\n");
}

function buildCurlSnippet(provider, baseUrl) {
  if (provider.protocol === "openai") {
    return [
      `curl "${baseUrl}/chat/completions" \\`,
      `  -H "Content-Type: application/json" \\`,
      `  -H "Authorization: Bearer localopenrouter-managed" \\`,
      `  -d '{`,
      `    "model": "gpt-4.1",`,
      `    "messages": [{"role": "user", "content": "Hello from LocalOpenRouter"}]`,
      `  }'`,
    ].join("\n");
  }

  if (provider.protocol === "anthropic") {
    return [
      `curl "${baseUrl}/messages" \\`,
      `  -H "Content-Type: application/json" \\`,
      `  -H "x-api-key: localopenrouter-managed" \\`,
      `  -H "anthropic-version: 2023-06-01" \\`,
      `  -d '{`,
      `    "model": "claude-3-7-sonnet-latest",`,
      `    "max_tokens": 256,`,
      `    "messages": [{"role": "user", "content": "Hello from LocalOpenRouter"}]`,
      `  }'`,
    ].join("\n");
  }

  return [
    `curl "${baseUrl}/<upstream-path>" \\`,
    `  -H "Content-Type: application/json" \\`,
    `  -d '{`,
    `    "replace": "with provider-specific payload"`,
    `  }'`,
  ].join("\n");
}

async function copyText(text, successMessage) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const buffer = document.createElement("textarea");
      buffer.value = text;
      buffer.setAttribute("readonly", "");
      buffer.style.position = "absolute";
      buffer.style.left = "-9999px";
      document.body.appendChild(buffer);
      buffer.select();
      document.execCommand("copy");
      buffer.remove();
    }
    notify(successMessage, "success");
  } catch (error) {
    console.error(error);
    notify("Copy failed. Clipboard access is unavailable.", "error");
  }
}

function normalizeAccountProviderFilter() {
  if (
    state.accountProviderFilter !== "all" &&
    !state.providers.some((provider) => provider.slug === state.accountProviderFilter)
  ) {
    state.accountProviderFilter = "all";
  }
  return state.accountProviderFilter;
}

function filteredAccounts() {
  const activeFilter = normalizeAccountProviderFilter();
  const accounts =
    activeFilter === "all"
      ? [...state.accounts]
      : state.accounts.filter((account) => account.provider === activeFilter);

  return accounts
    .map((account, index) => ({ account, index }))
    .sort((left, right) => {
      const leftDefault = isDefaultAccount(left.account);
      const rightDefault = isDefaultAccount(right.account);
      if (leftDefault !== rightDefault) {
        return leftDefault ? -1 : 1;
      }
      return left.index - right.index;
    })
    .map(({ account }) => account);
}

function providerOptionsForSelect(preferredSlug) {
  const candidates = [];
  const pushProvider = (provider) => {
    if (!provider || candidates.some((candidate) => candidate.slug === provider.slug)) {
      return;
    }
    candidates.push(provider);
  };

  if (preferredSlug) {
    pushProvider(state.providers.find((provider) => provider.slug === preferredSlug));
  }

  state.providers
    .filter((provider) => provider.enabled)
    .forEach((provider) => pushProvider(provider));

  return candidates;
}

function providerOptionLabel(provider) {
  return provider.enabled
    ? `${provider.displayName} (${provider.protocol})`
    : `${provider.displayName} (${provider.protocol}, disabled)`;
}

function defaultRouteForProvider(provider) {
  return state.routes.find((route) => route.provider === provider && !route.modelPrefix) || null;
}

function isDefaultAccount(account) {
  return defaultRouteForProvider(account.provider)?.accountId === account.id;
}

function routeAccountOptionsForProvider(provider, preferredAccountId) {
  const candidates = [];
  const pushAccount = (account) => {
    if (!account || candidates.some((candidate) => candidate.id === account.id)) {
      return;
    }
    candidates.push(account);
  };

  if (preferredAccountId) {
    pushAccount(
      state.accounts.find(
        (account) => account.provider === provider && account.id === preferredAccountId,
      ),
    );
  }

  state.accounts
    .filter((account) => account.provider === provider && account.enabled)
    .forEach((account) => pushAccount(account));

  return candidates;
}

async function perform(action, successMessage) {
  try {
    const response = await action();
    if (successMessage) {
      notify(successMessage, "success");
    }
    return response;
  } catch (error) {
    console.error(error);
    return null;
  }
}

async function performDesktop(action, successMessage, failureMessage) {
  try {
    const response = await action();
    if (successMessage) {
      notify(successMessage, "success");
    }
    return response;
  } catch (error) {
    console.error(error);
    notify(error?.message || failureMessage, "error");
    return null;
  }
}

async function invokeDesktop(command, args = {}) {
  const invoke = window.__TAURI__?.core?.invoke
    ? window.__TAURI__.core.invoke.bind(window.__TAURI__.core)
    : window.__TAURI_INTERNALS__?.invoke?.bind(window.__TAURI_INTERNALS__);
  if (!invoke) {
    throw new Error("Desktop integration is unavailable in this context.");
  }
  return invoke(command, args);
}

async function api(path, options = {}) {
  let response;
  try {
    response = await fetch(`${daemonBaseUrl()}${path}`, {
      method: options.method || "GET",
      headers: {
        "Content-Type": "application/json",
        ...(options.headers || {}),
      },
      body: options.body ? JSON.stringify(options.body) : undefined,
    });
  } catch (error) {
    if (!options.silent) {
      notify(`Cannot reach the local daemon on ${daemonAddress()}.`, "error");
    }
    throw error;
  }

  const text = await response.text();
  const payload = text ? safeJson(text) : null;
  if (!response.ok) {
    const rawMessage = payload?.error || `${response.status} ${response.statusText}`;
    const message = translateApiError(path, rawMessage);
    if (!options.silent || isCompatibilityError(message)) {
      notify(message, "error");
    }
    throw new Error(message);
  }
  return payload;
}

function translateApiError(path, message) {
  if (message === `resource not found: ${path}` && path.startsWith("/admin/")) {
    return `Daemon on ${daemonAddress()} is running, but it does not support ${path}. This usually means an older LocalOpenRouter daemon is still occupying the port. Stop that process and restart the desktop app.`;
  }
  return message;
}

function isCompatibilityError(message) {
  return message.includes("older LocalOpenRouter daemon");
}

function safeJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}

function notify(message, tone = "info") {
  const toast = document.createElement("div");
  toast.className = `toast ${tone}`;
  toast.textContent = message;
  elements.toastStack.appendChild(toast);
  window.setTimeout(() => {
    toast.remove();
  }, 3200);
}

function emptyNode(message = "Nothing to show yet.") {
  const node = elements.emptyTemplate.content.firstElementChild.cloneNode(true);
  node.textContent = message;
  return node;
}

function normalizeOptional(value) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function routeBindingId(provider, modelPrefix) {
  return modelPrefix ? `${provider}::${modelPrefix}` : `${provider}::*`;
}

function normalizeSegment(value) {
  return value
    .trim()
    .toLowerCase()
    .replaceAll("/", "-")
    .replaceAll("_", "-")
    .replace(/\s+/g, "-");
}

function isValidSlug(value) {
  return /^[a-z0-9-]+$/.test(value);
}

function isSuccessStatus(statusCode) {
  return typeof statusCode === "number" && statusCode < 400;
}

function averageLatency(logs) {
  if (!logs.length) {
    return null;
  }
  const total = logs.reduce((sum, log) => sum + log.durationMs, 0);
  return Math.round(total / logs.length);
}

function percentileLatency(logs, percentile) {
  if (!logs.length) {
    return null;
  }
  const sorted = logs
    .map((log) => log.durationMs)
    .sort((left, right) => left - right);
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * percentile) - 1);
  return sorted[index];
}

function formatSuccessRate(logs) {
  if (!logs.length) {
    return "--";
  }
  const successCount = logs.filter((log) => isSuccessStatus(log.statusCode)).length;
  return `${Math.round((successCount / logs.length) * 100)}%`;
}

function formatLatency(value) {
  return typeof value === "number" ? `${value} ms` : "--";
}

function formatDateTime(value) {
  if (!value) {
    return "Unavailable";
  }
  return new Date(value).toLocaleString();
}

function formatRelativeTime(value) {
  const timestamp = new Date(value).getTime();
  if (Number.isNaN(timestamp)) {
    return "unknown";
  }
  const deltaMs = Date.now() - timestamp;
  const deltaMinutes = Math.round(deltaMs / 60000);
  if (deltaMinutes < 1) {
    return "just now";
  }
  if (deltaMinutes < 60) {
    return `${deltaMinutes}m ago`;
  }
  const deltaHours = Math.round(deltaMinutes / 60);
  if (deltaHours < 24) {
    return `${deltaHours}h ago`;
  }
  const deltaDays = Math.round(deltaHours / 24);
  return `${deltaDays}d ago`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function truncateMiddle(value, maxChars) {
  if (!value || value.length <= maxChars) {
    return value;
  }
  const lead = Math.max(4, Math.floor((maxChars - 1) / 2));
  const tail = Math.max(4, maxChars - lead - 1);
  return `${value.slice(0, lead)}…${value.slice(-tail)}`;
}
