// Windows View
(function () {
  function init() {
    const input = document.getElementById("newExcludedAppInput");
    if (input) {
      input.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          const addBtn = document.querySelector('[data-action="add_chip"][data-input="newExcludedAppInput"]');
          if (addBtn) addBtn.click();
        }
      });
    }
  }
  window.WindowsView = { init };
})();
