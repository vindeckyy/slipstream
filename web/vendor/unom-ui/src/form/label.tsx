import * as React from "react";
export const Label = React.forwardRef<HTMLLabelElement, React.LabelHTMLAttributes<HTMLLabelElement>>(function Label(
  { className, ...props },
  ref,
) {
  return <label ref={ref} className={["text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70", className].filter(Boolean).join(" ")} {...props} />;
});
Label.displayName = "Label";
