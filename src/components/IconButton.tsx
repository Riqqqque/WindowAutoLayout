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
        "icon-button",
        variant === "solid" && "icon-button-solid",
        variant === "danger" && "icon-button-danger",
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
