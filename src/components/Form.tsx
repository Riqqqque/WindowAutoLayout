import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="grid gap-1.5 text-sm text-zinc-300">
      <span className="text-[11px] font-semibold uppercase tracking-normal text-[#8a94a3]">{label}</span>
      {children}
    </label>
  );
}

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`h-10 rounded-md border border-[#2a323d] bg-[#0d1117] px-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 hover:border-[#384555] focus:border-[#5db7ff] disabled:cursor-not-allowed disabled:opacity-55 ${props.className ?? ""}`}
    />
  );
}

export function NumberInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return <TextInput {...props} type="number" />;
}

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={`min-h-20 rounded-md border border-[#2a323d] bg-[#0d1117] px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 hover:border-[#384555] focus:border-[#5db7ff] disabled:cursor-not-allowed disabled:opacity-55 ${props.className ?? ""}`}
    />
  );
}

export function SelectInput(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={`h-10 rounded-md border border-[#2a323d] bg-[#0d1117] px-3 text-sm text-zinc-100 outline-none transition hover:border-[#384555] focus:border-[#5db7ff] disabled:cursor-not-allowed disabled:opacity-55 ${props.className ?? ""}`}
    />
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <label className="flex min-h-11 items-center justify-between gap-3 rounded-md border border-[#252b34] bg-[#0d1117] px-3 py-2 text-sm text-zinc-200 transition hover:border-[#34404d]">
      <span className="min-w-0 leading-snug">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="peer sr-only"
      />
      <span className="relative h-5 w-9 shrink-0 rounded-full border border-[#3a4350] bg-[#151a21] transition after:absolute after:left-0.5 after:top-0.5 after:h-4 after:w-4 after:rounded-full after:bg-[#8a94a3] after:transition peer-checked:border-[#39d98a]/60 peer-checked:bg-[#39d98a]/20 peer-checked:after:translate-x-4 peer-checked:after:bg-[#39d98a] peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-[#5db7ff]/60" />
    </label>
  );
}
