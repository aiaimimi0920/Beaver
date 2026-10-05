import { createTask, listTasks } from "../../../lib/bridge-db";
import { deliver } from "../../../lib/bridge-events";
// Private-Site dispatch authenticates owner or platform service access.
// This endpoint can enqueue only a fixed marker check, never executable input.
export async function GET() {
  try {
    return Response.json({ tasks: await listTasks() });
  } catch {
    return Response.json({ error: "Storage unavailable" }, { status: 503 });
  }
}
export async function POST(req: Request) {
  const origin = req.headers.get("origin");
  if (origin && origin !== new URL(req.url).origin)
    return new Response("Wrong origin", { status: 403 });
  try {
    const raw = await req.text();
    if (raw.length > 1024) return new Response("Too large", { status: 413 });
    const { request_id } = JSON.parse(raw),
      task = await createTask(request_id);
    return Response.json({ task, delivery: await deliver(task) });
  } catch (error) {
    return Response.json(
      { error: error instanceof Error ? error.message : "Request failed" },
      { status: 400 },
    );
  }
}
