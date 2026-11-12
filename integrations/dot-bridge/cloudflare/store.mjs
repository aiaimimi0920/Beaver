import {
  canonical,
  identifier,
  requireValue,
  validateReply,
  validateRequest,
} from "./contract.mjs";

export class ExchangeStore {
  constructor(db) {
    this.db = db;
  }
  async get(id) {
    identifier(id);
    const row = await this.db
      .prepare("SELECT * FROM exchanges WHERE id = ?")
      .bind(id)
      .first();
    requireValue(row, "Exchange not found", 404);
    return {
      request: JSON.parse(row.request),
      reply: row.reply === null ? null : JSON.parse(row.reply),
      receipt: row.receipt === null ? null : JSON.parse(row.receipt),
      createdAt: row.created_at,
    };
  }
  async publish(value) {
    validateRequest(value);
    const serialized = canonical(value);
    await this.db
      .prepare(
        "INSERT INTO exchanges (id, request) VALUES (?, ?) ON CONFLICT(id) DO NOTHING",
      )
      .bind(value.id, serialized)
      .run();
    const existing = await this.get(value.id);
    requireValue(
      canonical(existing.request) === serialized,
      "Exchange ID conflict",
      409,
    );
    return existing;
  }
  async pending() {
    const rows = await this.db
      .prepare(
        "SELECT id, created_at FROM exchanges WHERE reply IS NULL ORDER BY created_at, id LIMIT 20",
      )
      .all();
    return { exchanges: rows.results, limit: 20 };
  }
  async reply(id, value) {
    validateReply(value);
    return this.writeOnce(id, "reply", value);
  }
  async receipt(id, value) {
    requireValue(
      value && typeof value === "object" && !Array.isArray(value),
      "Expected receipt object",
    );
    const existing = await this.get(id);
    requireValue(existing.reply !== null, "Reply required before receipt", 409);
    return this.writeOnce(id, "receipt", value);
  }
  async writeOnce(id, column, value) {
    identifier(id);
    const serialized = canonical(value);
    // Column comes only from the two internal methods above, never from HTTP input.
    await this.db
      .prepare(
        `UPDATE exchanges SET ${column} = ? WHERE id = ? AND ${column} IS NULL`,
      )
      .bind(serialized, id)
      .run();
    const existing = await this.get(id);
    requireValue(
      canonical(existing[column]) === serialized,
      "Immutable message conflict",
      409,
    );
    return existing;
  }
}
