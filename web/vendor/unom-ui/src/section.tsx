import * as React from "react";
export default function Section({ className, maxWidth = true, ...props }: React.HTMLAttributes<HTMLElement> & { maxWidth?: boolean }) {
  return <section className={["w-full px-4 py-6 sm:px-6 lg:px-8", maxWidth && "mx-auto max-w-7xl", className].filter(Boolean).join(" ")} {...props} />;
}
