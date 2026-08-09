import * as React from "react";
export const InputText = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(function InputText(
  { className, ...props },
  ref,
) {
  return <input ref={ref} className={["flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50", className].filter(Boolean).join(" ")} {...props} />;
});
InputText.displayName = "InputText";
