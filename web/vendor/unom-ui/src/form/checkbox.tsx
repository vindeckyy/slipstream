import * as React from "react";
export type CheckboxValue = boolean | "indeterminate";
export type CheckboxProps = Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "onChange"> & { checked?: boolean; defaultChecked?: boolean; onCheckedChange?: (checked: CheckboxValue) => void };
export const Checkbox = React.forwardRef<HTMLButtonElement, CheckboxProps>(function Checkbox(
  { checked, defaultChecked = false, onCheckedChange, className, ...props },
  ref,
) {
  const [internal, setInternal] = React.useState(defaultChecked);
  const value = checked ?? internal;
  const toggle = () => {
    const next = !value;
    if (checked === undefined) setInternal(next);
    onCheckedChange?.(next);
  };
  return <button ref={ref} type="button" role="checkbox" aria-checked={value} onClick={toggle} className={["inline-flex size-4 shrink-0 items-center justify-center rounded-sm border border-primary ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50", value && "bg-primary text-primary-foreground", className].filter(Boolean).join(" ")} {...props}>{value ? <span aria-hidden>✓</span> : null}</button>;
});
Checkbox.displayName = "Checkbox";
