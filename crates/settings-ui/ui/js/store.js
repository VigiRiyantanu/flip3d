// Flip 3D State Store
(function () {
  let config = null;
  let defaults = null;
  let allStyles = [];
  let blurStatus = "none";
  let engineStatus = { is_running: false, is_paused: false };
  let autostart = false;
  let distribution = "installed";
  let dataPath = "";
  let isDirty = false;
  let saveDebounceTimer = null;
  const listeners = [];

  function subscribe(fn) {
    listeners.push(fn);
  }

  function emit(event, data) {
    listeners.forEach(fn => fn(event, data));
  }

  function init(initialData) {
    config = initialData.config || {};
    defaults = initialData.defaults || {};
    allStyles = initialData.all_styles || [];
    blurStatus = initialData.blur_status || "none";
    engineStatus = {
      is_running: !!initialData.is_engine_running,
      is_paused: !initialData.is_engine_running
    };
    autostart = !!initialData.autostart;
    distribution = initialData.distribution || "installed";
    dataPath = initialData.data_path || "";
    isDirty = false;
    emit("init", { config, defaults, allStyles, blurStatus, engineStatus, autostart, distribution, dataPath });
  }

  function get(key) {
    if (!config) return undefined;
    return config[key];
  }

  function getDefault(key) {
    if (!defaults) return undefined;
    return defaults[key];
  }

  function set(key, value, applyLive = true) {
    if (!config) return;
    config[key] = value;
    isDirty = true;
    emit("change", { key, value });

    clearTimeout(saveDebounceTimer);
    setSaveIndicator("Unsaved changes...");
    saveDebounceTimer = setTimeout(() => {
      save(applyLive);
    }, 150);
  }

  function save(applyLive = true) {
    clearTimeout(saveDebounceTimer);
    return window.AppIPC.call("save_config", {
      config: config,
      apply_live: applyLive
    }).then(res => {
      if (res && res.config) {
        config = res.config;
        isDirty = false;
        setSaveIndicator("Changes applied live");
        emit("saved", config);
      }
    }).catch(err => {
      setSaveIndicator("Save failed: " + err.message);
      showToast("Save error: " + err.message);
    });
  }

  function resetField(key) {
    if (!defaults) return;
    const defVal = defaults[key];
    if (defVal !== undefined) {
      set(key, JSON.parse(JSON.stringify(defVal)), true);
    }
  }

  function restoreDefaults() {
    return window.AppIPC.call("restore_defaults").then(res => {
      if (res && res.config) {
        config = res.config;
        isDirty = false;
        emit("reset", config);
        save(true);
        showToast("Settings restored to defaults");
      }
    });
  }

  function handlePushEvent(event, payload) {
    if (event === "engine_status") {
      engineStatus = payload;
      emit("engine_status", engineStatus);
    }
  }

  function setSaveIndicator(text) {
    const el = document.getElementById("saveIndicator");
    if (el) el.innerText = text;
  }

  function showToast(msg) {
    let toast = document.getElementById("appToast");
    if (!toast) {
      toast = document.createElement("div");
      toast.id = "appToast";
      toast.className = "toast hidden";
      document.body.appendChild(toast);
    }
    toast.innerText = msg;
    toast.classList.remove("hidden");
    setTimeout(() => {
      toast.classList.add("hidden");
    }, 3000);
  }

  window.AppStore = {
    init,
    get,
    getDefault,
    set,
    save,
    resetField,
    restoreDefaults,
    subscribe,
    handlePushEvent,
    showToast,
    getAllStyles: () => allStyles,
    getBlurStatus: () => blurStatus,
    getEngineStatus: () => engineStatus,
    getDistribution: () => ({ distribution, dataPath }),
    getConfig: () => config,
    getDefaults: () => defaults,
    isDirty: () => isDirty
  };
})();
