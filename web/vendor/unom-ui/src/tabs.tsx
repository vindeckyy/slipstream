import * as React from "react";
type TabsContextValue = { value: string; setValue: (value: string) => void };
const TabsContext = React.createContext<TabsContextValue | null>(null);
export type TabsProps = React.HTMLAttributes<HTMLDivElement> & { value?: string; defaultValue?: string; onValueChange?: (value: string) => void };
export function Tabs({ value: controlled, defaultValue = "", onValueChange, className, ...props }: TabsProps) {
  const [internal, setInternal] = React.useState(defaultValue);
  const value = controlled ?? internal;
  const setValue = (next: string) => { if (controlled === undefined) setInternal(next); onValueChange?.(next); };
  return <TabsContext.Provider value={{ value, setValue }}><div className={["flex flex-col", className].filter(Boolean).join(" ")} {...props} /></TabsContext.Provider>;
}
export const TabsList = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(function TabsList({ className, ...props }, ref) { return <div ref={ref} role="tablist" className={["inline-flex items-center", className].filter(Boolean).join(" ")} {...props} />; });
export const TabsTrigger = React.forwardRef<HTMLButtonElement, React.ButtonHTMLAttributes<HTMLButtonElement> & { value: string }>(function TabsTrigger({ value, className, onClick, ...props }, ref) { const ctx=React.useContext(TabsContext); const active=ctx?.value===value; return <button ref={ref} type="button" role="tab" aria-selected={active} data-state={active?"active":"inactive"} className={["inline-flex items-center justify-center rounded-md px-3 py-1.5 text-sm", className].filter(Boolean).join(" ")} onClick={(event)=>{ctx?.setValue(value); onClick?.(event);}} {...props} />; });
export const TabsContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement> & { value: string }>(function TabsContent({ value, className, ...props }, ref) { const ctx=React.useContext(TabsContext); if(ctx && ctx.value!==value) return null; return <div ref={ref} role="tabpanel" className={className} {...props} />; });
