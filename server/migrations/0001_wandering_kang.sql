CREATE TABLE `connection_tokens` (
	`token` text PRIMARY KEY NOT NULL,
	`docId` text NOT NULL,
	`scope` text NOT NULL,
	`parentGrantId` text,
	`grantToken` text,
	`userId` text,
	`expiresAt` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `docs` (
	`docId` text PRIMARY KEY NOT NULL,
	`ownerId` text NOT NULL,
	`createdAt` integer NOT NULL,
	FOREIGN KEY (`ownerId`) REFERENCES `user`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `grants` (
	`grantId` text PRIMARY KEY NOT NULL,
	`docId` text NOT NULL,
	`scope` text NOT NULL,
	`token` text NOT NULL,
	`createdBy` text NOT NULL,
	`createdAt` integer NOT NULL,
	`expiresAt` integer,
	FOREIGN KEY (`docId`) REFERENCES `docs`(`docId`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE UNIQUE INDEX `grants_token_unique` ON `grants` (`token`);