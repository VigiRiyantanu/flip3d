// Style View Renderer
(function () {
  const groupTitles = {
    "StacksAndFlow": "3D STACKS & FLOW",
    "ITabStages": "ITAB STAGES",
    "SpatialAndShapes": "SPATIAL & 3D SHAPES",
    "PanelsAndGrids": "PANELS, GRIDS & LISTS",
    "ClassicAndSystem": "CLASSIC & SYSTEM"
  };

  function getMiniPreview(styleId) {
    switch (styleId) {
      case "flip3d":
        return `<div class="w-14 h-10 rounded border border-[#16a34a] bg-white/90 shadow-[0_4px_12px_rgba(22,163,74,0.18)] transform -rotate-12 skew-y-6 flex items-center justify-center text-[10px] mono font-semibold text-[#14532d]">Vista</div>
                <div class="w-14 h-10 rounded border border-[#9fc1a7] bg-white/40 transform -rotate-6 skew-y-3 absolute translate-x-3 translate-y-1"></div>`;
      case "cascade":
        return `<div class="w-12 h-9 rounded border border-[#a6bdad] bg-white/60 absolute -translate-x-3 -translate-y-2"></div>
                <div class="w-12 h-9 rounded border border-[#8daaa0] bg-white/80 absolute translate-x-0 translate-y-0"></div>
                <div class="w-12 h-9 rounded border border-[#5d856d] bg-white absolute translate-x-3 translate-y-2 shadow-2xs"></div>`;
      case "coverFlow":
        return `<div class="w-10 h-10 border border-[#b2c8b7] bg-white/50 transform rotate-y-45 -skew-y-6"></div>
                <div class="w-12 h-11 border border-[#3f7253] bg-white z-10 rounded shadow-sm flex items-center justify-center">
                  <div class="w-8 h-8 rounded-sm bg-[#e8f1eb]"></div>
                </div>
                <div class="w-10 h-10 border border-[#b2c8b7] bg-white/50 transform -rotate-y-45 skew-y-6"></div>`;
      case "itab":
      case "itabTitle":
        return `<div class="w-full h-full bg-[#1a241e] rounded flex flex-col items-center justify-center p-2">
                  <div class="w-12 h-7 rounded border border-white/60 bg-white/20 mb-1"></div>
                  <div class="w-14 h-1 bg-white/60 rounded"></div>
                  <div class="w-8 h-1 bg-emerald-400/80 rounded mt-0.5"></div>
                </div>`;
      case "itabBackground":
      case "itabBackgroundTitle":
        return `<div class="w-full h-full bg-[#eaf1ec] rounded flex flex-col items-center justify-center p-2">
                  <div class="w-12 h-7 rounded border border-[#4d7e60] bg-white shadow-2xs mb-1"></div>
                  <div class="w-14 h-1 bg-[#15803d]/70 rounded"></div>
                </div>`;
      case "grid":
      case "windows11":
        return `<div class="grid grid-cols-2 gap-1.5 w-14 h-10">
                  <div class="rounded border border-[#a6bdad] bg-white/80"></div>
                  <div class="rounded border border-[#16a34a] bg-white shadow-2xs"></div>
                  <div class="rounded border border-[#a6bdad] bg-white/80"></div>
                  <div class="rounded border border-[#a6bdad] bg-white/80"></div>
                </div>`;
      default:
        return `<span class="text-[11px] mono font-semibold text-[#14532d]">${styleId}</span>`;
    }
  }

  function render(container) {
    const allStyles = window.AppStore.getAllStyles();
    const currentStyle = window.AppStore.get("style") || "flip3d";
    const winTabStyle = window.AppStore.get("win_tab_style") || "same";

    // Populate Win+Tab dropdown
    const winTabSelect = document.getElementById("win_tab_style");
    if (winTabSelect) {
      winTabSelect.innerHTML = '<option value="same">Same as Alt+Tab</option>';
      allStyles.forEach(s => {
        const opt = document.createElement("option");
        opt.value = s.id;
        opt.innerText = s.display_name;
        winTabSelect.appendChild(opt);
      });
      winTabSelect.value = winTabStyle;
    }

    // Group styles
    const groups = {};
    allStyles.forEach(s => {
      if (!groups[s.group]) groups[s.group] = [];
      groups[s.group].push(s);
    });

    container.innerHTML = "";
    for (const [groupKey, styles] of Object.entries(groups)) {
      const header = document.createElement("div");
      header.className = "flex items-center space-x-2 mb-3 mt-4";
      header.innerHTML = `
        <span class="text-[11px] font-bold uppercase tracking-wider text-[#144f33] bg-[#e3ece5] px-2.5 py-0.5 rounded-full border border-[#c6d7cb] shadow-2xs">${groupTitles[groupKey] || groupKey}</span>
        <div class="h-px flex-1 bg-black/[0.06]"></div>
      `;
      container.appendChild(header);

      const grid = document.createElement("div");
      grid.className = "grid grid-cols-3 gap-3";

      styles.forEach(s => {
        const isSelected = s.id.toLowerCase() === currentStyle.toLowerCase();
        const card = document.createElement("div");
        card.className = `card-glass p-3 rounded-[14px] cursor-pointer transition-all flex flex-col group relative ${isSelected ? "selected" : ""}`;
        card.setAttribute("data-style-id", s.id);
        card.innerHTML = `
          <div class="h-20 rounded-lg bg-[#f0f4f1] border border-[#d2ddd4] mb-2 flex items-center justify-center relative overflow-hidden group-hover:border-[#a8c2af]">
            ${getMiniPreview(s.id)}
          </div>
          <div class="font-semibold text-[#1a2e22] text-[12px] flex items-center justify-between">
            <span>${s.display_name}</span>
          </div>
          <div class="text-[11px] text-[#5e7166] mt-0.5">${s.description}</div>
        `;

        card.addEventListener("click", () => {
          container.querySelectorAll(".card-glass").forEach(c => {
            c.classList.remove("selected");
          });
          card.classList.add("selected");
          window.AppStore.set("style", s.id, true);
        });

        grid.appendChild(card);
      });

      container.appendChild(grid);
    }
  }

  window.StyleView = { render };
})();
