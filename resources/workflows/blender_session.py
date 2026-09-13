"""Start the installed Blender MCP addon in a Beaver-owned empty GUI session."""

import importlib.util
import json
import os
from pathlib import Path
import sys

import bpy

request = json.loads(Path(os.environ["BEAVER_BLENDER_REQUEST"]).read_text(encoding="utf-8"))
bpy.ops.wm.read_factory_settings(use_empty=True)
if request.get("restore"):
    bpy.ops.wm.open_mainfile(filepath=request["restore"], load_ui=False, use_scripts=False)
sys.path.insert(0, str(Path(__file__).parent))
from asset_observer import Observer

observer = Observer(request["observer"])
spec = importlib.util.spec_from_file_location("blender_mcp", request["addon"])
addon = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = addon
spec.loader.exec_module(addon)


class ScopedServer(addon.BlenderMCPServer):
    def __init__(self, *args, **kwargs):
        super().__init__(host="127.0.0.1", port=request["port"])

    def execute_command(self, command):
        observer.begin_operation(str(command.get("type", "Blender command")))
        try:
            return super().execute_command(command)
        finally:
            observer.end_operation()


# Registration may auto-start the addon: bind the assigned port from its first start.
addon.BlenderMCPServer = ScopedServer
addon.register()
bpy.context.scene.blendermcp_port = request["port"]
server = getattr(bpy.types, "blendermcp_server", None)
if server is None:
    server = ScopedServer()
    bpy.types.blendermcp_server = server
if not server.running:
    server.start()
if not server.running:
    raise RuntimeError("The task-local Blender MCP listener did not start")
ready = Path(request["ready"])
temporary = ready.with_suffix(".tmp")
temporary.write_text(
    json.dumps({"pid": os.getpid(), "port": request["port"], "observerPort": observer.server.server_port}), encoding="utf-8"
)
temporary.replace(ready)
print("BEAVER_BLENDER_READY", flush=True)
