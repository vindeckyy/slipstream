import * as React from "react";
export const defaultMaterialTheme = {};
export function MaterialProvider({ children }: { children: React.ReactNode; theme?: unknown }) { return <>{children}</>; }
