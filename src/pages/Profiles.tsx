import { Check, Copy, Plus, Trash2 } from "lucide-react";
import { Field, SelectInput, TextArea, TextInput } from "../components/Form";
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
  const missingTargetMonitor =
    profile.targetMonitorId && !monitors.some((monitor) => monitor.id === profile.targetMonitorId)
      ? profile.targetMonitorId
      : null;

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
      enforcement: {
        ...config.enforcement,
        profileId:
          config.enforcement.profileId === profile.id ? remaining[0]?.id ?? null : config.enforcement.profileId,
      },
      profiles: remaining,
    });
    onProfileChange(remaining[0].id);
  }

  function updateProfile(next: Profile) {
    onConfigChange(patchProfile(config, profile.id, () => next));
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
      <section className="panel p-3">
        <div className="flex items-center justify-between gap-2">
          <div>
            <h1 className="section-heading">Profiles</h1>
            <div className="mt-1 text-xs text-[#71818c]">{config.profiles.length} saved</div>
          </div>
          <IconButton label="Add profile" onClick={addProfile} variant="solid">
            <Plus size={16} />
          </IconButton>
        </div>
        <div className="data-list mt-3">
          {config.profiles.map((item) => (
            <button
              key={item.id}
              className={`data-row w-full px-3 py-3 text-left transition ${
                item.id === profile.id
                  ? "bg-[#16252e] text-[#e4f6fa]"
                  : "text-zinc-300 hover:bg-[#121a20]"
              }`}
              onClick={() => onProfileChange(item.id)}
            >
              <div className="truncate text-sm font-medium">{item.name}</div>
              <div className="mt-1 text-xs text-[#71818c]">{item.apps.length} apps</div>
            </button>
          ))}
        </div>
      </section>

      <section className="panel p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <div className="eyebrow">Profile details</div>
            <h2 className="mt-1 text-lg font-semibold text-zinc-50">{profile.name}</h2>
          </div>
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
              {missingTargetMonitor && <option value={missingTargetMonitor}>Missing: {missingTargetMonitor}</option>}
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
          <div className="grid content-start gap-2 pt-[18px]">
            {config.startup.defaultProfileId === profile.id ? (
              <div className="button-secondary cursor-default border-[#42d392]/55 text-[#aef2d1]">
                <Check size={15} />
                Startup default
              </div>
            ) : (
              <button
                className="button-secondary"
                onClick={() =>
                  onConfigChange({
                    ...config,
                    startup: { ...config.startup, defaultProfileId: profile.id },
                  })
                }
              >
                <Check size={15} />
                Use at startup
              </button>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}
