CREATE TABLE `subscriptions` (
	`id` text PRIMARY KEY NOT NULL,
	`owner` text NOT NULL,
	`callback` text NOT NULL,
	`secret` text NOT NULL,
	`expires` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `tasks` (
	`id` text PRIMARY KEY NOT NULL,
	`event_id` text NOT NULL,
	`created_at` text NOT NULL,
	`state` text DEFAULT 'pending' NOT NULL,
	`delivery` text DEFAULT 'not_sent' NOT NULL,
	`result` text,
	`completed_at` text
);
