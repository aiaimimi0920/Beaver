import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";
export const subscriptions = sqliteTable("subscriptions", {
  id: text("id").primaryKey(),
  owner: text("owner").notNull(),
  callback: text("callback").notNull(),
  secret: text("secret").notNull(),
  expires: integer("expires").notNull(),
});
export const tasks = sqliteTable("tasks", {
  id: text("id").primaryKey(),
  eventId: text("event_id").notNull(),
  createdAt: text("created_at").notNull(),
  state: text("state").notNull().default("pending"),
  delivery: text("delivery").notNull().default("not_sent"),
  result: text("result"),
  completedAt: text("completed_at"),
});
