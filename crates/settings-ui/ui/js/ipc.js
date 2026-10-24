// Flip 3D Settings IPC Module
(function () {
  const pendingRequests = {};
  let reqCounter = 0;

  function log(level, message) {
    try {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          id: "log_" + Date.now(),
          type: "log",
          payload: { level: level, message: String(message) }
        }));
      }
    } catch (_) {}
  }

  function call(type, payload) {
    return new Promise((resolve, reject) => {
      const id = "req_" + (++reqCounter);
      pendingRequests[id] = { resolve, reject, type };
      const msg = JSON.stringify({ id, type, payload: payload || {} });
      if (window.ipc) {
        window.ipc.postMessage(msg);
      } else {
        log("warn", "window.ipc unavailable for call: " + type);
        delete pendingRequests[id];
        resolve(null);
      }
    });
  }

  function receive(jsonStr) {
    try {
      const resp = typeof jsonStr === 'string' ? JSON.parse(jsonStr) : jsonStr;
      const handler = pendingRequests[resp.id];
      if (handler) {
        delete pendingRequests[resp.id];
        if (resp.ok) {
          handler.resolve(resp.payload);
        } else {
          handler.reject(new Error(resp.error || "IPC call failed"));
        }
      } else if (resp.event) {
        // Push event from Rust (e.g. engine status changed)
        if (window.AppStore && typeof window.AppStore.handlePushEvent === 'function') {
          window.AppStore.handlePushEvent(resp.event, resp.payload);
        }
      }
    } catch (e) {
      log("error", "Error processing IPC response: " + e.message);
    }
  }

  // Intercept JS unhandled errors
  window.onerror = function (msg, url, lineNo, columnNo, error) {
    log("error", `window.onerror: ${msg} (${url}:${lineNo}:${columnNo})` + (error ? ` | ${error.stack}` : ''));
    return false;
  };

  window.onunhandledrejection = function (event) {
    log("error", "Unhandled rejection: " + (event.reason ? (event.reason.stack || event.reason) : event));
  };

  const origError = console.error;
  console.error = function (...args) {
    log("error", "console.error: " + args.map(a => typeof a === 'object' ? JSON.stringify(a) : a).join(" "));
    origError.apply(console, args);
  };

  const origWarn = console.warn;
  console.warn = function (...args) {
    log("warn", "console.warn: " + args.map(a => typeof a === 'object' ? JSON.stringify(a) : a).join(" "));
    origWarn.apply(console, args);
  };

  window.AppIPC = {
    call,
    receive,
    log
  };

  // Compatibility hook for Wry evaluate_script
  window.__app = {
    receive: receive,
    ipcCall: call,
    drag: () => call('drag_window'),
    minimize: () => call('minimize_window'),
    close: () => call('close_window')
  };
})();
