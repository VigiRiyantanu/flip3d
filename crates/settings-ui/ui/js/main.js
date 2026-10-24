// Flip 3D Main Entry Point
(function () {
  document.addEventListener("DOMContentLoaded", () => {
    // 1. Initialize view listeners and event delegation
    window.AppBind.init();
    window.ShortcutsView.init();
    window.WindowsView.init();
    window.BackgroundView.init();
    window.SystemView.init();

    // 2. Fetch initial data from Rust backend
    window.AppIPC.call("get_initial_data").then(data => {
      if (!data) return;

      // Handle blur fallback class on <html>
      if (data.blur_status === "none" || data.blur_status === "solid") {
        document.documentElement.classList.add("no-blur");
      } else {
        document.documentElement.classList.remove("no-blur");
      }

      // Initialize Store
      window.AppStore.init(data);

      // Render Style View
      const stylesContainer = document.getElementById("stylesContainer");
      if (stylesContainer) {
        window.StyleView.render(stylesContainer);
      }

      // Default active tab: Style
      window.AppBind.switchTab("style");

      // Signal ui_ready to Rust backend so window is displayed cleanly
      window.AppIPC.call("ui_ready").catch(() => {});
    }).catch(err => {
      window.AppIPC.log("error", "Failed to load initial data: " + err.message);
      // Still signal ui_ready on error so window doesn't stay hidden forever
      window.AppIPC.call("ui_ready").catch(() => {});
    });
  });
})();
