import type { ButtonHTMLAttributes, ReactNode } from "react";
import { clsx } from "clsx";

type IconButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  variant?: "solid" | "ghost" | "danger";
};

export function IconButton({ label, children, className, variant = "ghost", ...props }: IconButtonProps) {
  return (
    <button
      aria-label={label}
      title={label}
      className={clsx(
        "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-md border text-sm transition disabled:cursor-not-allowed disabled:opacity-40",
        variant === "solid" && "border-cyan-300/40 bg-cyan-300 text-zinc-950 hover:bg-cyan-200",
        variant === "ghost" && "border-zinc-700 bg-zinc-900 text-zinc-200 hover:border-zinc-500 hover:bg-zinc-800",
        variant === "danger" && "border-rose-400/40 bg-rose-500/10 text-rose-200 hover:bg-rose-500/20",
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
