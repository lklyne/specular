CREATE TABLE `comments` (
	`docId` text NOT NULL,
	`id` text NOT NULL,
	`body` text NOT NULL,
	`createdAt` integer NOT NULL,
	`updatedAt` integer NOT NULL,
	PRIMARY KEY(`docId`, `id`),
	FOREIGN KEY (`docId`) REFERENCES `docs`(`docId`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `versions` (
	`versionId` text PRIMARY KEY NOT NULL,
	`docId` text NOT NULL,
	`manifest` text NOT NULL,
	`createdAt` integer NOT NULL,
	FOREIGN KEY (`docId`) REFERENCES `docs`(`docId`) ON UPDATE no action ON DELETE cascade
);
