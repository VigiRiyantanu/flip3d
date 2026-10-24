// System & About View
(function () {
  function updateStatusPill(status) {
    const dot = document.getElementById("statusDot");
    const text = document.getElementById("statusText");
    const pill = document.getElementById("statusPill");
    if (!dot || !text || !pill) return;

    const isActive = status && status.is_running && !status.is_paused;
    if (isActive) {
      dot.className = "w-2 h-2 rounded-full bg-[#10b981] shadow-[0_0_8px_#10b981]";
      dot.style.boxShadow = "0 0 8px #10b981";
      text.innerText = "Active";
      text.className = "text-[12px] font-semibold text-[#134e35]";
      pill.className = "flex items-center space-x-2 px-3 py-1 rounded-full bg-[#eaf4ed] border border-[#c3dcce] shadow-xs cursor-pointer";
    } else {
      dot.className = "w-2 h-2 rounded-full bg-slate-400";
      dot.style.boxShadow = "none";
      text.innerText = "Paused";
      text.className = "text-[12px] font-medium text-slate-600";
      pill.className = "flex items-center space-x-2 px-3 py-1 rounded-full bg-slate-100 border border-slate-300 shadow-xs cursor-pointer";
    }
  }

  function updateDistroBadge(distro) {
    const badge = document.getElementById("distroBadge");
    if (!badge || !distro) return;

    if (distro.distribution === "portable") {
      badge.innerText = "Distribution: Portable";
      badge.title = `Data path: ${distro.dataPath}`;
      badge.className = "px-2.5 py-0.5 rounded-full bg-[#eef7ee] border border-[#a8d5b5] text-[#134e35] font-mono text-[11px] truncate max-w-[320px] shadow-xs";
    } else if (distro.distribution === "portable_fallback") {
      badge.innerText = "Distribution: Portable (Fallback)";
      badge.title = `Data directory was not writable. Using fallback: ${distro.dataPath}`;
      badge.className = "px-2.5 py-0.5 rounded-full bg-[#fff4e5] border border-[#ffd599] text-[#b45309] font-mono text-[11px] truncate max-w-[320px] shadow-xs";
    } else {
      badge.innerText = "Distribution: Installed";
      badge.title = `Config path: ${distro.dataPath}`;
      badge.className = "px-2.5 py-0.5 rounded-full bg-[#f0f5f1] border border-[#cdd8ce] text-[#2d4235] font-mono text-[11px] truncate max-w-[320px]";
    }
  }

  function init() {
    window.AppStore.subscribe((event, data) => {
      if (event === "init") {
        updateStatusPill(window.AppStore.getEngineStatus());
        updateDistroBadge(window.AppStore.getDistribution());
      } else if (event === "engine_status") {
        updateStatusPill(data);
      }
    });

    const autostartEl = document.getElementById("autostartToggle");
    if (autostartEl) {
      // Query autostart state from backend
      window.AppIPC.call("get_autostart").then(res => {
        if (res) autostartEl.checked = !!res.enabled;
      });

      autostartEl.addEventListener("change", () => {
        window.AppIPC.call("set_autostart", { enabled: autostartEl.checked }).then(res => {
          if (res && res.enabled !== undefined) {
            autostartEl.checked = res.enabled;
          }
        }).catch(err => {
          window.AppStore.showToast("Failed to change autostart: " + err.message);
        });
      });
    }
  }

  window.SystemView = { init };
})();
