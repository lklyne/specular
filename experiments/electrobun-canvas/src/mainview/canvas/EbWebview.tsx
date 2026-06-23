import { createElement, forwardRef } from "react";
import type { CSSProperties, ReactNode } from "react";

// Minimal, self-typed handle on Electrobun's <electrobun-webview> custom element.
// The element itself is registered by the Electrobun runtime preload; we only
// need the subset of its imperative API the spike drives. (Typing it locally
// keeps the prototype decoupled from electrobun/view's exact export surface.)
export interface WebviewElement extends HTMLElement {
  webviewId?: number;
  maskSelectors: Set<string>;
  transparent: boolean;
  passthroughEnabled: boolean;
  src: string | null;
  addMaskSelector(selector: string): void;
  removeMaskSelector(selector: string): void;
  loadURL(url: string): void;
  toggleTransparent(value?: boolean): void;
  togglePassthrough(value?: boolean): void;
  toggleHidden(value?: boolean): void;
  syncDimensions(force?: boolean): void;
  executeJavascript(js: string): void;
  on(event: string, listener: (e: CustomEvent) => void): void;
  off(event: string, listener: (e: CustomEvent) => void): void;
}

interface EbWebviewProps {
  className?: string;
  style?: CSSProperties;
  src?: string;
  html?: string;
  partition?: string;
  children?: ReactNode;
}

/**
 * React wrapper around the custom element. We go through `createElement` rather
 * than JSX so we don't have to wrestle TS's JSX.IntrinsicElements for a tag the
 * runtime owns; the ref lands on the real DOM element.
 */
export const EbWebview = forwardRef<WebviewElement, EbWebviewProps>(
  function EbWebview(props, ref) {
    return createElement("electrobun-webview", { ref, ...props });
  },
);
