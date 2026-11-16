/// <reference types="vite/client" />

declare module "*.md" {
  import type { ComponentType, HTMLAttributes } from "react";

  export const html: string;
  export const raw: string;

  const MarkdownContent: ComponentType<HTMLAttributes<HTMLDivElement>>;
  export default MarkdownContent;
}

declare module "*.markdown" {
  export * from "*.md";
  export { default } from "*.md";
}

declare module "*.mdown" {
  export * from "*.md";
  export { default } from "*.md";
}

declare module "*.mkdn" {
  export * from "*.md";
  export { default } from "*.md";
}

declare module "*.mkd" {
  export * from "*.md";
  export { default } from "*.md";
}

declare module "*.mdx" {
  export * from "*.md";
  export { default } from "*.md";
}
