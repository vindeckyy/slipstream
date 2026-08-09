import * as React from "react";

export type ButtonVariant = "default" | "destructive" | "outline" | "secondary" | "ghost" | "link" | "success" | "accent";
export type ButtonSize = "default" | "sm" | "lg" | "icon" | "input";
export type AnimatedButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize };

const variants: Record<ButtonVariant, string> = {
  default: "bg-primary text-primary-foreground",
  destructive: "bg-destructive text-destructive-foreground",
  outline: "border border-input bg-background text-foreground",
  secondary: "bg-secondary text-secondary-foreground",
  ghost: "bg-transparent text-foreground",
  link: "bg-transparent text-primary underline-offset-4",
  success: "bg-success text-success-foreground",
  accent: "bg-accent text-accent-foreground",
};
const sizes: Record<ButtonSize, string> = {
  default: "h-10 px-4 py-2",
  sm: "h-9 rounded-md px-3 text-sm",
  lg: "h-11 rounded-md px-8",
  icon: "size-10",
  input: "h-10 px-3",
};

export function buttonVariants(options: { variant?: ButtonVariant; size?: ButtonSize; className?: string } = {}) {
  return [
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
    variants[options.variant ?? "default"],
    sizes[options.size ?? "default"],
    options.className,
  ].filter(Boolean).join(" ");
}

export const AnimatedButton = React.forwardRef<HTMLButtonElement, AnimatedButtonProps>(function AnimatedButton(
  { className, variant, size, type = "button", ...props },
  ref,
) {
  return <button ref={ref} type={type} className={buttonVariants({ variant, size, className })} {...props} />;
});
AnimatedButton.displayName = "AnimatedButton";

export const Button = AnimatedButton;
