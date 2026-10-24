// Flip 3D Generic Binder & Event Delegation Engine
(function () {
  function formatValue(formatStr, val) {
    if (!formatStr) return String(val);
    return formatStr.replace("{v}", val);
  }

  function syncControlFromStore(el) {
    const key = el.getAttribute("data-setting");
    const type = el.getAttribute("data-type");
    const format = el.getAttribute("data-format");
    const val = window.AppStore.get(key);
    if (val === undefined) return;

    if (type === "range") {
      el.value = val;
      const badgeId = el.getAttribute("data-badge");
      if (badgeId) {
        const badge = document.getElementById(badgeId);
        if (badge) badge.innerText = formatValue(format, val);
      }
    } else if (type === "toggle") {
      el.checked = !!val;
    } else if (type === "select") {
      el.value = String(val);
    } else if (type === "segmented") {
      el.querySelectorAll("button").forEach(btn => {
        if (btn.getAttribute("data-val") === String(val)) {
          btn.classList.add("bg-white", "text-[#134e35]", "shadow-xs", "font-semibold", "active");
          btn.classList.remove("text-[#52635a]", "font-medium");
        } else {
          btn.classList.remove("bg-white", "text-[#134e35]", "shadow-xs", "font-semibold", "active");
          btn.classList.add("text-[#52635a]", "font-medium");
        }
      });
    } else if (type === "chips") {
      renderChips(el, val);
    }
  }

  function renderChips(container, items) {
    container.innerHTML = "";
    const list = Array.isArray(items) ? items : [];
    list.forEach((item, index) => {
      const chip = document.createElement("span");
      chip.className = "inline-flex items-center space-x-1.5 px-3 py-1 rounded-full bg-[#eaf2ec] border border-[#bcd3c3] text-[12px] mono text-[#134e35]";
      chip.innerHTML = `
        <span>${item}</span>
        <button type="button" class="hover:text-red-600 text-[#546e5e]" data-action="remove_chip" data-index="${index}" title="Remove">
          <svg class="w-3 h-3" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
            <line x1="18" x2="6" y1="6" y2="18"></line>
            <line x1="6" x2="18" y1="6" y2="18"></line>
          </svg>
        </button>
      `;
      container.appendChild(chip);
    });
  }

  function syncAll() {
    document.querySelectorAll("[data-setting]").forEach(syncControlFromStore);
  }

  function init() {
    // 1. Subscribe to store changes to keep UI controls in sync
    window.AppStore.subscribe((event, data) => {
      if (event === "init" || event === "reset" || event === "saved") {
        syncAll();
      } else if (event === "change") {
        const matching = document.querySelectorAll(`[data-setting="${data.key}"]`);
        matching.forEach(syncControlFromStore);
      }
    });

    // 2. Input and Change Listeners for data-setting elements
    document.addEventListener("input", function (e) {
      const target = e.target;
      const key = target.getAttribute("data-setting");
      const type = target.getAttribute("data-type");
      const format = target.getAttribute("data-format");

      if (key && type === "range") {
        const badgeId = target.getAttribute("data-badge");
        if (badgeId) {
          const badge = document.getElementById(badgeId);
          if (badge) badge.innerText = formatValue(format, target.value);
        }
        window.AppStore.set(key, parseInt(target.value, 10), true);
      }
    });

    document.addEventListener("change", function (e) {
      const target = e.target;
      const key = target.getAttribute("data-setting");
      const type = target.getAttribute("data-type");

      if (!key) return;

      if (type === "toggle") {
        window.AppStore.set(key, target.checked, true);
      } else if (type === "select") {
        window.AppStore.set(key, target.value, true);
      } else if (type === "range") {
        window.AppStore.set(key, parseInt(target.value, 10), true);
      }
    });

    // 3. Global Click Delegation for actions
    document.addEventListener("click", function (e) {
      const actionEl = e.target.closest("[data-action]");
      if (!actionEl) return;

      const action = actionEl.getAttribute("data-action");

      switch (action) {
        case "drag_window":
          window.AppIPC.call("drag_window");
          break;
        case "minimize_window":
          window.AppIPC.call("minimize_window");
          break;
        case "close_window":
          window.AppIPC.call("close_window");
          break;
        case "switch_tab": {
          const tabId = actionEl.getAttribute("data-tab");
          if (tabId) switchTab(tabId);
          break;
        }
        case "select_segmented": {
          const container = actionEl.closest("[data-type='segmented']");
          if (container) {
            const key = container.getAttribute("data-setting");
            const val = actionEl.getAttribute("data-val");
            if (key && val) {
              container.querySelectorAll("button").forEach(b => {
                b.classList.remove("bg-white", "text-[#134e35]", "shadow-xs", "font-semibold", "active");
                b.classList.add("text-[#52635a]", "font-medium");
              });
              actionEl.classList.add("bg-white", "text-[#134e35]", "shadow-xs", "font-semibold", "active");
              actionEl.classList.remove("text-[#52635a]", "font-medium");
              window.AppStore.set(key, val, true);
            }
          }
          break;
        }
        case "reset_control": {
          const key = actionEl.getAttribute("data-setting");
          if (key) window.AppStore.resetField(key);
          break;
        }
        case "apply_settings": {
          window.AppStore.save(true);
          break;
        }
        case "restore_defaults": {
          if (confirm("Restore all settings to factory defaults?")) {
            window.AppStore.restoreDefaults();
          }
          break;
        }
        case "toggle_engine_state": {
          window.AppIPC.call("toggle_engine_pause").then(res => {
            if (res) {
              window.AppStore.handlePushEvent("engine_status", {
                is_running: res.is_active,
                is_paused: !res.is_active
              });
            }
          });
          break;
        }
        case "open_config_folder": {
          window.AppIPC.call("open_config_folder");
          break;
        }
        case "open_log_file": {
          window.AppIPC.call("open_log_file");
          break;
        }
        case "export_settings": {
          window.AppIPC.call("export_settings").then(res => {
            if (res && res.path) {
              window.AppStore.showToast("Exported configuration to " + res.path);
            }
          });
          break;
        }
        case "import_settings": {
          window.AppIPC.call("import_settings").then(res => {
            if (res && res.config) {
              window.AppStore.init({
                config: res.config,
                defaults: window.AppStore.getDefaults(),
                all_styles: window.AppStore.getAllStyles(),
                blur_status: window.AppStore.getBlurStatus(),
                is_engine_running: window.AppStore.getEngineStatus().is_running
              });
              window.AppStore.showToast("Settings imported successfully");
            }
          });
          break;
        }
        case "add_chip": {
          const inputId = actionEl.getAttribute("data-input");
          const containerId = actionEl.getAttribute("data-chips-container");
          const input = document.getElementById(inputId);
          const container = document.getElementById(containerId);
          if (input && container) {
            const val = input.value.trim().toLowerCase();
            if (val) {
              const key = container.getAttribute("data-setting");
              const current = window.AppStore.get(key) || [];
              if (!current.includes(val)) {
                const updated = [...current, val];
                window.AppStore.set(key, updated, true);
              }
              input.value = "";
            }
          }
          break;
        }
        case "remove_chip": {
          const index = parseInt(actionEl.getAttribute("data-index"), 10);
          const container = actionEl.closest("[data-type='chips']");
          if (container && !isNaN(index)) {
            const key = container.getAttribute("data-setting");
            const current = window.AppStore.get(key) || [];
            const updated = current.filter((_, i) => i !== index);
            window.AppStore.set(key, updated, true);
          }
          break;
        }
      }
    });

    // Make titlebar draggable
    const titlebar = document.getElementById("titlebar");
    if (titlebar) {
      titlebar.addEventListener("mousedown", function (e) {
        if (e.target.closest("button") || e.target.closest("select") || e.target.closest("#statusPill")) {
          return;
        }
        window.AppIPC.call("drag_window");
      });
    }
  }

  function switchTab(tabId) {
    document.querySelectorAll(".tab-btn").forEach(btn => {
      if (btn.getAttribute("data-tab") === tabId) {
        btn.classList.add("active");
      } else {
        btn.classList.remove("active");
      }
    });
    document.querySelectorAll(".tab-content").forEach(content => {
      content.classList.add("hidden");
    });
    const activeContent = document.getElementById("tab-" + tabId);
    if (activeContent) {
      activeContent.classList.remove("hidden");
    }
  }

  window.AppBind = {
    init,
    syncAll,
    switchTab
  };
})();
