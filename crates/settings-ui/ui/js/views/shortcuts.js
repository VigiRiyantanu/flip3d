// Shortcuts View
(function () {
  function renderExtraShortcuts(container) {
    container.innerHTML = "";
    const extras = window.AppStore.get("extra_shortcuts") || [];
    const allStyles = window.AppStore.getAllStyles();

    extras.forEach((entry, index) => {
      const row = document.createElement("div");
      row.className = "flex items-center space-x-3 p-2 rounded-xl bg-white/60 border border-[#d5ded7]";
      
      let styleOptions = '<option value="same">Animation: Same as Alt+Tab or Win+Tab</option>';
      allStyles.forEach(s => {
        styleOptions += `<option value="${s.id}" ${entry.style === s.id ? "selected" : ""}>Animation: ${s.display_name}</option>`;
      });

      row.innerHTML = `
        <select class="h-8 px-2.5 rounded-lg bg-white border border-[#c9d6cc] text-[#1a2e22] text-[12px] outline-none flex-1" data-field="shortcut">
          <option value="winCtrlTab" ${entry.shortcut === "winCtrlTab" ? "selected" : ""}>Win+Ctrl+Tab stays open</option>
          <option value="ctrlAltTab" ${entry.shortcut === "ctrlAltTab" ? "selected" : ""}>Ctrl+Alt+Tab stays open</option>
        </select>
        <select class="h-8 px-2.5 rounded-lg bg-white border border-[#c9d6cc] text-[#1a2e22] text-[12px] outline-none flex-1" data-field="style">
          ${styleOptions}
        </select>
        <button type="button" class="w-8 h-8 rounded-lg bg-red-100 hover:bg-red-200 text-red-700 border border-red-200 flex items-center justify-center transition-all" title="Remove">
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path></svg>
        </button>
      `;

      const scSelect = row.querySelector('[data-field="shortcut"]');
      const stSelect = row.querySelector('[data-field="style"]');
      const delBtn = row.querySelector("button");

      scSelect.addEventListener("change", () => {
        extras[index].shortcut = scSelect.value;
        window.AppStore.set("extra_shortcuts", [...extras], true);
      });
      stSelect.addEventListener("change", () => {
        extras[index].style = stSelect.value;
        window.AppStore.set("extra_shortcuts", [...extras], true);
      });
      delBtn.addEventListener("click", () => {
        const updated = extras.filter((_, i) => i !== index);
        window.AppStore.set("extra_shortcuts", updated, true);
        renderExtraShortcuts(container);
      });

      container.appendChild(row);
    });
  }

  function init() {
    const extrasContainer = document.getElementById("extraShortcutsList");
    const addBtn = document.getElementById("addExtraShortcutBtn");

    if (extrasContainer) {
      window.AppStore.subscribe((event, data) => {
        if (event === "init" || event === "reset" || (event === "change" && data.key === "extra_shortcuts")) {
          renderExtraShortcuts(extrasContainer);
        }
      });
    }

    if (addBtn && extrasContainer) {
      addBtn.addEventListener("click", () => {
        const extras = window.AppStore.get("extra_shortcuts") || [];
        if (extras.length < 20) {
          extras.push({ shortcut: "winCtrlTab", style: "same" });
          window.AppStore.set("extra_shortcuts", [...extras], true);
          renderExtraShortcuts(extrasContainer);
        }
      });
    }
  }

  window.ShortcutsView = { init };
})();
