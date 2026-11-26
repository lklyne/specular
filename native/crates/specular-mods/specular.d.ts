// The `specular` module (ADR 0055): what a mod imports to reach the app.
// This file is checked in and written into the mods folder on every start.

/** The value of a state key, such as `panels/toolbar`, or null. Reading a key subscribes the mod to it: the mod renders again when it changes. */
export function state(key: string): any;
/** What the layout hands this mod (`props` on its node), or null. */
export function props(): any;
/** A value shared between mods, or null. */
export function shared(key: string): any;
/** Sets a shared value. Every mod that read it renders again. Returns whether it changed. */
export function setShared(key: string, value: any): boolean;
/** Runs a command by name, as the palette, a key or the CLI would. Throws when the command is unknown or cannot run now. */
export function run(name: string, args?: object): any;
/** Every command: `{ name, args, about, enabled }`. */
export function commands(): Array<{ name: string; args: string; about: string; enabled: boolean }>;
/** Shows a line to the person. */
export function notify(text: string, level?: "info" | "success" | "warning" | "error"): void;
