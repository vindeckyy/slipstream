import * as React from "react";

export type AnimatedCardProps = React.HTMLAttributes<HTMLDivElement> & { padding?: boolean; interactive?: boolean };
export const AnimatedCard = React.forwardRef<HTMLDivElement, AnimatedCardProps>(function AnimatedCard(
  { className, padding = true, interactive = false, ...props },
  ref,
) {
  const classes = [
    "rounded-xl border border-border bg-card text-card-foreground",
    padding && "p-4",
    interactive && "cursor-pointer",
    className,
  ].filter(Boolean).join(" ");
  return <div ref={ref} className={classes} {...props} />;
});
AnimatedCard.displayName = "AnimatedCard";
