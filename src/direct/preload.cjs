const { contextBridge, ipcRenderer } = require("electron");
contextBridge.exposeInMainWorld("beaver", {
  call: (method, input) =>
    ipcRenderer.invoke("beaver-direct", { method, input }),
});
