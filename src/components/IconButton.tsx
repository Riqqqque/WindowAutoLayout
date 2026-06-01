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
        variant === "solid" && "border-[#5db7ff]/60 bg-[#5db7ff] text-[#071019] shadow-sm shadow-[#5db7ff]/10 hover:bg-[#86caff]",
        variant === "ghost" && "border-[#2a323d] bg-[#111820] text-zinc-200 hover:border-[#455364] hover:bg-[#17202a]",
        variant === "danger" && "border-rose-400/40 bg-rose-500/10 text-rose-200 hover:border-rose-300/60 hover:bg-rose-500/20",
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
