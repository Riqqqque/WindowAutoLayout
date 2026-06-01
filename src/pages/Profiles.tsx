import { Copy, Plus, Trash2 } from "lucide-react";
import { Field, SelectInput, TextArea, TextInput, Toggle } from "../components/Form";
import { IconButton } from "../components/IconButton";
import { newId, patchProfile } from "../lib/helpers";
import type { MonitorInfo, Profile, WindowAutoLayoutConfig } from "../lib/types";

interface ProfilesProps {
  config: WindowAutoLayoutConfig;
  profile: Profile;
  monitors: MonitorInfo[];
  onConfigChange: (config: WindowAutoLayoutConfig) => void;
  onProfileChange: (profileId: string) => void;
}

export function ProfilesPage({ config, profile, monitors, onConfigChange, onProfileChange }: ProfilesProps) {
  function addProfile() {
    const id = newId("profile");
    onConfigChange({
      ...config,
      profiles: [
        ...config.profiles,
        {
          id,
          name: "Custom",
          description: "",
          targetMonitorId: config.global.defaultMonitorId,
          apps: [],
          startupRestore: false,
          enforceAfterRestore: false,
        },
      ],
    });
    onProfileChange(id);
  }

  function duplicateProfile() {
    const id = newId("profile");
    const copy = {
      ...profile,
      id,
      name: `${profile.name} Copy`,
      apps: profile.apps.map((app) => ({ ...app, id: newId("app") })),
    };
    onConfigChange({ ...config, profiles: [...config.profiles, copy] });
    onProfileChange(id);
  }

  function deleteProfile() {
    if (config.profiles.length <= 1) return;
    const remaining = config.profiles.filter((item) => item.id !== profile.id);
    onConfigChange({
      ...config,
      startup: {
        ...config.startup,
        defaultProfileId:
          config.startup.defaultProfileId === profile.id ? remaining[0]?.id ?? null : config.startup.defaultProfileId,
      },
      profiles: remaining,
    });
    onProfileChange(remaining[0].id);
  }

  function updateProfile(next: Profile) {
    onConfigChange(patchProfile(config, profile.id, () => next));
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[280px_1fr]">
      <section className="surface rounded-md p-3">
        <div className="flex items-center justify-between gap-2">
          <h1 className="text-lg font-semibold text-zinc-50">Profiles</h1>
          <IconButton label="Add profile" onClick={addProfile} variant="solid">
            <Plus size={16} />
          </IconButton>
        </div>
        <div className="mt-3 grid gap-2">
          {config.profiles.map((item) => (
            <button
              key={item.id}
              className={`rounded-md border px-3 py-2 text-left transition ${
                item.id === profile.id
                  ? "border-[#5db7ff]/45 bg-[#5db7ff]/10 text-[#d7edff]"
                  : "border-[#252b34] bg-[#0d1117] text-zinc-300 hover:border-[#34404d] hover:bg-[#121820]"
              }`}
              onClick={() => onProfileChange(item.id)}
            >
              <div className="truncate text-sm font-medium">{item.name}</div>
              <div className="mt-1 text-xs text-[#8a94a3]">{item.apps.length} apps</div>
            </button>
          ))}
        </div>
      </section>

      <section className="surface rounded-md p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-lg font-semibold text-zinc-50">{profile.name}</h2>
          <div className="flex gap-2">
            <IconButton label="Duplicate profile" onClick={duplicateProfile}>
              <Copy size={16} />
            </IconButton>
            <IconButton label="Delete profile" onClick={deleteProfile} disabled={config.profiles.length <= 1} variant="danger">
              <Trash2 size={16} />
            </IconButton>
          </div>
        </div>

        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <Field label="Name">
            <TextInput value={profile.name} onChange={(event) => updateProfile({ ...profile, name: event.target.value })} />
          </Field>
          <Field label="Target monitor">
            <SelectInput
              value={profile.targetMonitorId ?? ""}
              onChange={(event) => updateProfile({ ...profile, targetMonitorId: event.target.value || null })}
            >
              <option value="">Use global default</option>
              {monitors.map((monitor) => (
                <option key={monitor.id} value={monitor.id}>
                  {monitor.name} - {monitor.width}x{monitor.height}
                </option>
              ))}
            </SelectInput>
          </Field>
          <Field label="Description">
            <TextArea
              value={profile.description ?? ""}
              onChange={(event) => updateProfile({ ...profile, description: event.target.value })}
            />
          </Field>
          <div className="grid content-start gap-2">
            <Toggle
              label="Restore at startup"
              checked={profile.startupRestore}
              onChange={(checked) => updateProfile({ ...profile, startupRestore: checked })}
            />
            <Toggle
              label="Enforce after restore"
              checked={profile.enforceAfterRestore}
              onChange={(checked) => updateProfile({ ...profile, enforceAfterRestore: checked })}
            />
            <Toggle
              label="Default startup profile"
              checked={config.startup.defaultProfileId === profile.id}
              onChange={(checked) =>
                onConfigChange({
                  ...config,
                  startup: { ...config.startup, defaultProfileId: checked ? profile.id : null },
                })
              }
            />
          </div>
        </div>
      </section>
    </div>
  );
}
