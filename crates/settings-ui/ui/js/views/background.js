// Background View
(function () {
  function updateBlurVisibility() {
    const bgMode = window.AppStore.get("background") || "blur";
    const blurRow = document.getElementById("blurAmountRow");
    if (blurRow) {
      blurRow.style.display = (bgMode === "blur") ? "flex" : "none";
    }
  }

  function init() {
    window.AppStore.subscribe((event, data) => {
      if (event === "init" || event === "reset" || (event === "change" && data.key === "background")) {
        updateBlurVisibility();
      }
    });
  }

  window.BackgroundView = { init };
})();
